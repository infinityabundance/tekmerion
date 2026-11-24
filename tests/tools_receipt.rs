//! Tests for the Slice D Rust ports of the remaining receipt scripts:
//! `curve_check`, `instrument`, `inline_answer`, `sealed_court`, and the
//! `dropin sweep` driver.
//!
//! The rule-level tests use synthetic inputs and need nothing outside the
//! repository. The differential tests compare against artefacts recorded from
//! the Python originals; the `inline_answer` one skips cleanly when the pinned
//! `.bench` sources are absent.

use std::fs;
use std::path::PathBuf;

use tekmerion::tools::inline_answer;
use tekmerion::tools::instrument;
use tekmerion::tools::json;
use tekmerion::tools::sealed_court;
use tekmerion::tools::{curve_check, dropin};

/// sha256 of `tools/bridge.ml` when the recorded goldens were captured. The
/// differential tests skip if the file has since moved.
const BRIDGE_DIGEST: &str = "ceb690a8a24589614886953337caaf05ab64a8c62cc50cd6500f36c413a5ea4c";

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn bridge_source() -> Option<String> {
    let path = manifest_dir().join("tools/bridge.ml");
    let src = fs::read_to_string(&path).ok()?;
    if sha256(src.as_bytes()) != BRIDGE_DIGEST {
        eprintln!("skipping: tools/bridge.ml is not at the recorded digest");
        return None;
    }
    Some(src)
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tek-receipt-{}-{}", std::process::id(), tag));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

// ---------------------------------------------------------------------------
// curve_check
// ---------------------------------------------------------------------------

#[test]
fn curve_check_strips_comments_before_reporting() {
    let clean = "(* p_256 and 123456 live only in a comment *)\n\
                 let T = prove (`!x. x + 0 = x`, REWRITE_TAC[]);;\n";
    let (text, statements, hits) = curve_check::check_text(clean);
    assert_eq!(statements, 1);
    assert_eq!(hits, 0, "{text}");
    assert!(text.contains("line 2: CLEAN"), "{text}");
    assert!(text.contains("statements=1 hits=0"), "{text}");

    let hidden = "(* outer (* inner *) still p_384 *) let T = prove (`!x. x = x`, T);;";
    assert_eq!(
        curve_check::strip_comments(hidden),
        " let T = prove (`!x. x = x`, T);;"
    );
}

#[test]
fn curve_check_hit_changes_the_exit_code() {
    let dir = scratch("curve-exit");
    let clean = dir.join("clean.ml");
    fs::write(
        &clean,
        "(* 99999 *)\nlet T = prove (`!x. x = x`, REWRITE_TAC[]);;\n",
    )
    .unwrap();
    let hit = dir.join("hit.ml");
    fs::write(
        &hit,
        "let T = prove (`!x. x MOD p_256 = x`, REWRITE_TAC[]);;\n",
    )
    .unwrap();

    let bin = env!("CARGO_BIN_EXE_curve_check");
    let ok = std::process::Command::new(bin)
        .arg(&clean)
        .output()
        .unwrap();
    assert!(ok.status.success(), "clean file should exit 0");
    let bad = std::process::Command::new(bin).arg(&hit).output().unwrap();
    assert_eq!(bad.status.code(), Some(1), "a hit should exit 1");
    assert!(String::from_utf8_lossy(&bad.stdout).contains("HIT curve:p_256"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn curve_check_reproduces_the_recorded_report() {
    let Some(src) = bridge_source() else {
        return;
    };
    let (text, statements, hits) = curve_check::check_text(&src);
    assert_eq!(statements, 71);
    assert_eq!(hits, 0);
    let expected =
        fs::read_to_string(manifest_dir().join("tests/fixtures/curve_check_bridge.txt")).unwrap();
    assert_eq!(text, expected, "curve_check report differs");
}

// ---------------------------------------------------------------------------
// instrument
// ---------------------------------------------------------------------------

#[test]
fn instrument_wraps_a_three_phrase_fixture() {
    let src = "let a = 1;;\n\nlet rec b = 2;;\nprint_string \"x\";;\n";
    let (out, phrases) = instrument::instrument_text(src, "sample.ml");
    assert_eq!(phrases, 3);
    assert!(out.contains("let a =\n  bridge_check \"a\" (fun () -> (\nlet a = 1\n   in a));;\n"));
    assert!(
        out.contains("let b =\n  bridge_check \"b\" (fun () -> (\n\nlet rec b = 2\n   in b));;\n")
    );
    assert!(out.ends_with("print_string \"x\";;\n"));
    // The header records the digest of the source, not of the output.
    assert!(out.contains(&format!("sha256(source) = {}", sha256(src.as_bytes()))));
}

#[test]
fn instrument_reproduces_the_recorded_output() {
    let Some(src) = bridge_source() else {
        return;
    };
    let (out, phrases) = instrument::instrument_text(&src, "tools/bridge.ml");
    assert_eq!(phrases, 119);
    assert_eq!(
        sha256(out.as_bytes()),
        "f4bba97fba1ba3281acd60cb8aa09275ef7f62a205d3c32254c858a5999c5b23"
    );
}

// ---------------------------------------------------------------------------
// inline_answer
// ---------------------------------------------------------------------------

#[test]
fn inline_answer_splits_phrases_and_skips_directives() {
    let src = "#use \"x\";;\nneeds \"base.ml\";;\nloadt \"y.ml\";;\n\
               let a = 1;;\n\nlet b = 2;;\nneedsome = 3;;\n";
    assert_eq!(
        inline_answer::phrases_from(src),
        vec![
            "let a = 1".to_string(),
            "let b = 2".to_string(),
            "needsome = 3".to_string(),
        ]
    );
}

#[test]
fn inline_answer_reproduces_all_four_recorded_digests() {
    let repo = manifest_dir();
    if !repo.join(".bench/hol-light/Cadical/dimacs.ml").is_file() {
        eprintln!("skipping: .bench sources are absent");
        return;
    }
    let parts = inline_answer::collect(&repo).unwrap();
    assert_eq!(parts.len(), 155);
    for (mode, digest) in [
        (
            "add-ref",
            "39fd3d64c65063410c02f0a879d505632b4d82042ac3a64f26c137426e23264f",
        ),
        (
            "sub",
            "b36a7fd0274396d72bdbe87f6cd8810888ad308bfe415e952f9bfed015c9e176",
        ),
        (
            "neg",
            "b5a674d51aeb621f11611200a7e47451b4d6ada2e434728bf60e6eb49143cb6e",
        ),
        (
            "add-p384",
            "83d90067847d411d3eee7fb7b6ca6b532d790ab262341e6ba5d248d54db15044",
        ),
    ] {
        let body = inline_answer::render(&parts, mode).unwrap();
        assert_eq!(sha256(body.as_bytes()), digest, "mode {mode} differs");
    }
}

// ---------------------------------------------------------------------------
// sealed_court
// ---------------------------------------------------------------------------

fn synthetic_problems() -> json::Json {
    let entries: Vec<String> = sealed_court::TASK_IDS
        .iter()
        .enumerate()
        .map(|(i, task)| {
            format!(
                "\"{task}\": {{\"inlined_locations\": [[\"ml_files/t{i}.ml\", {}]]}}",
                i + 1
            )
        })
        .collect();
    json::parse(format!("{{{}}}", entries.join(",")).as_bytes()).unwrap()
}

#[test]
fn sealed_court_reads_linenum_from_the_first_inlined_location() {
    let plan = sealed_court::template_plan(&synthetic_problems()).unwrap();
    assert_eq!(plan.len(), 9);
    assert_eq!(plan[0].rel, "ml_files/t0.ml");
    assert_eq!(plan[0].linenum, 1);
    assert_eq!(plan[8].linenum, 9);
}

#[test]
fn sealed_court_truncates_templates_to_the_prefix() {
    let (prefix, count) = sealed_court::prefix_bytes(b"l1\nl2\nl3\n", 2);
    assert_eq!(count, 2);
    assert_eq!(prefix.as_slice(), b"l1\nl2\n".as_slice());

    // A prefix longer than the file keeps everything, including the final
    // fragment that has no newline.
    let (all, n) = sealed_court::prefix_bytes(b"l1\nl2\nl3", 10);
    assert_eq!(n, 3);
    assert_eq!(all.as_slice(), b"l1\nl2\nl3".as_slice());

    let (none, zero) = sealed_court::prefix_bytes(b"l1\nl2\n", 0);
    assert!(none.is_empty());
    assert_eq!(zero, 0);
}

// ---------------------------------------------------------------------------
// dropin sweep
// ---------------------------------------------------------------------------

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

#[test]
fn dropin_sweep_argument_parsing() {
    assert!(dropin::parse_sweep_args(&[]).is_err());
    assert!(dropin::parse_sweep_args(&args(&["--arch", "riscv"])).is_err());
    assert!(dropin::parse_sweep_args(&args(&["--arch", "x86", "--bogus"])).is_err());
    assert!(dropin::parse_sweep_args(&args(&["--arch"])).is_err());
    // Both architectures, and the pair, are now in scope.
    assert_eq!(
        dropin::parse_sweep_args(&args(&["--arch", "arm"]))
            .unwrap()
            .arch,
        "arm"
    );
    assert_eq!(
        dropin::parse_sweep_args(&args(&["--arch", "all"]))
            .unwrap()
            .arch,
        "all"
    );

    let parsed = dropin::parse_sweep_args(&args(&[
        "--arch",
        "x86",
        "--dir",
        "/d",
        "--gen",
        "/g",
        "--hol",
        "/h",
        "--answers",
        "/a",
        "--force",
        "--timeout",
        "120",
        "--dry-run",
    ]))
    .unwrap();
    assert_eq!(parsed.arch, "x86");
    assert_eq!(parsed.dropin, PathBuf::from("/d"));
    assert_eq!(parsed.gen, PathBuf::from("/g"));
    assert_eq!(parsed.hol, PathBuf::from("/h"));
    assert_eq!(parsed.answers, Some(PathBuf::from("/a")));
    assert!(parsed.force);
    assert_eq!(parsed.timeout, 120);
    assert!(parsed.dry_run);
}
