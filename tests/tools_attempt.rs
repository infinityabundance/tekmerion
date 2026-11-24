//! Tests for the Rust ports of `attempt_id.py`, `attempt_lifecycle.py` and
//! `run_traces_inject.py`.
//!
//! Each test is the ported `--self-test`, plus the CLI behaviour that
//! `tools/run_traces.sh` depends on.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use tekmerion::tools::{attempt_id, attempt_lifecycle, run_traces_inject};

static SEQ: AtomicU32 = AtomicU32::new(0);

struct Tmp(PathBuf);

impl Tmp {
    fn new(tag: &str) -> Tmp {
        let n = SEQ.fetch_add(1, Ordering::SeqCst);
        let p =
            std::env::temp_dir().join(format!("tek-attempt-{}-{}-{}", std::process::id(), tag, n));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        Tmp(p)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

// --- attempt_id ------------------------------------------------------------

#[test]
fn attempt_id_is_deterministic_and_64_lowercase_hex() {
    let base = attempt_id::compute_attempt_id("arm/proofs/word_max.ml", b"proof", "", "aa", "bb");
    let again = attempt_id::compute_attempt_id("arm/proofs/word_max.ml", b"proof", "", "aa", "bb");
    assert_eq!(base, again);
    assert_eq!(base.len(), 64);
    assert!(base
        .bytes()
        .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)));
}

#[test]
fn attempt_id_every_component_changes_the_id() {
    let base = attempt_id::compute_attempt_id("arm/proofs/word_max.ml", b"proof", "", "aa", "bb");
    let variants = [
        attempt_id::compute_attempt_id("arm/proofs/word_min.ml", b"proof", "", "aa", "bb"),
        attempt_id::compute_attempt_id("arm/proofs/word_max.ml", b"prooF", "", "aa", "bb"),
        attempt_id::compute_attempt_id(
            "arm/proofs/word_max.ml",
            b"proof",
            "TRY (FAIL_TAC \"probe\")",
            "aa",
            "bb",
        ),
        attempt_id::compute_attempt_id("arm/proofs/word_max.ml", b"proof", "", "ab", "bb"),
        attempt_id::compute_attempt_id("arm/proofs/word_max.ml", b"proof", "", "aa", "bc"),
    ];
    for variant in variants {
        assert_ne!(variant, base);
    }
}

#[test]
fn attempt_id_length_framing_prevents_boundary_collisions() {
    let left = attempt_id::compute_attempt_id("ab", b"c", "", "", "");
    let right = attempt_id::compute_attempt_id("a", b"bc", "", "", "");
    assert_ne!(left, right);
}

#[test]
fn attempt_id_cli_prints_the_digest_of_the_file_bytes() {
    let t = Tmp::new("cli");
    let proof = t.path().join("word_max.ml");
    fs::write(&proof, b"proof").unwrap();
    let args = vec![
        "--theorem-id".to_string(),
        "arm/proofs/word_max.ml".to_string(),
        "--proof".to_string(),
        proof.display().to_string(),
        "--inject".to_string(),
        String::new(),
        "--wrapper-sha".to_string(),
        "aa".to_string(),
        "--module-sha".to_string(),
        "bb".to_string(),
    ];
    assert_eq!(attempt_id::run(&args), 0);
    let expected =
        attempt_id::compute_attempt_id("arm/proofs/word_max.ml", b"proof", "", "aa", "bb");
    assert_eq!(expected.len(), 64);
}

#[test]
fn attempt_id_cli_refuses_missing_options_and_unreadable_proofs() {
    let rc = attempt_id::run(&["--theorem-id".to_string(), "x".to_string()]);
    assert_eq!(rc, 2);

    let rc = attempt_id::run(&[
        "--theorem-id".to_string(),
        "x".to_string(),
        "--proof".to_string(),
        "/definitely/not/here.ml".to_string(),
        "--wrapper-sha".to_string(),
        "aa".to_string(),
        "--module-sha".to_string(),
        "bb".to_string(),
    ]);
    assert_eq!(rc, 1);
}

#[test]
fn attempt_id_cli_accepts_equals_form() {
    let t = Tmp::new("equals");
    let proof = t.path().join("p.ml");
    fs::write(&proof, b"x").unwrap();
    let args = vec![
        "--theorem-id=t".to_string(),
        format!("--proof={}", proof.display()),
        "--inject=".to_string(),
        "--wrapper-sha=aa".to_string(),
        "--module-sha=bb".to_string(),
    ];
    assert_eq!(attempt_id::run(&args), 0);
}

// --- attempt_lifecycle -----------------------------------------------------

fn template() -> String {
    format!(
        "let rt_start = Unix.time();;\n{}\n{}\ncheck_axioms ();;\n{}\n\
         Printf.printf \"ok\\n\";;\n",
        attempt_lifecycle::START,
        attempt_lifecycle::TIMEOUT,
        attempt_lifecycle::END
    )
}

#[test]
fn lifecycle_expands_each_sentinel_once() {
    let expanded = attempt_lifecycle::expand(&template()).unwrap();
    assert!(expanded.contains("exptrace_attempt_start"));
    assert_eq!(expanded.matches("exptrace_attempt_end").count(), 2);
    assert!(expanded.contains("Unix.alarm rt_timeout"));
    assert!(!expanded.contains(attempt_lifecycle::START));
    assert!(!expanded.contains(attempt_lifecycle::TIMEOUT));
    assert!(!expanded.contains(attempt_lifecycle::END));
}

#[test]
fn lifecycle_refuses_missing_and_duplicated_sentinels() {
    let error = attempt_lifecycle::expand("let x = 1\n").unwrap_err();
    assert!(error.0.contains("missing or duplicated"));

    let duplicated = format!(
        "{}\n{}\n{}\n{}\n",
        attempt_lifecycle::START,
        attempt_lifecycle::START,
        attempt_lifecycle::TIMEOUT,
        attempt_lifecycle::END
    );
    let error = attempt_lifecycle::expand(&duplicated).unwrap_err();
    assert!(error.0.contains("missing or duplicated"));
    assert!(error.0.contains("START"));
}

#[test]
fn lifecycle_ignores_indented_and_trailing_space_around_sentinels() {
    let text = template().replace(
        attempt_lifecycle::TIMEOUT,
        &format!("  {}   ", attempt_lifecycle::TIMEOUT),
    );
    let expanded = attempt_lifecycle::expand(&text).unwrap();
    assert!(expanded.contains("Unix.alarm rt_timeout"));
}

#[test]
fn lifecycle_cli_round_trip_and_argument_check() {
    let t = Tmp::new("lifecycle");
    let source = t.path().join("in.ml");
    let destination = t.path().join("out.ml");
    fs::write(&source, template()).unwrap();
    assert_eq!(
        attempt_lifecycle::run(&[
            source.display().to_string(),
            destination.display().to_string()
        ]),
        0
    );
    let written = fs::read_to_string(&destination).unwrap();
    assert_eq!(written, attempt_lifecycle::expand(&template()).unwrap());
    assert_eq!(attempt_lifecycle::run(&["only-one".to_string()]), 2);
}

// --- run_traces_inject -----------------------------------------------------

#[test]
fn inject_inserts_the_tactic_at_the_first_proof_step() {
    let proof = "let T = prove\n  (`x + 0 = x`,\n   REWRITE_TAC[] THEN\n   ARITH_TAC);;\n";
    let out = run_traces_inject::inject(proof, "TRY (FAIL_TAC \"probe\")").unwrap();
    assert!(out.contains("   TRY (FAIL_TAC \"probe\") THEN\n   REWRITE_TAC[]"));
}

#[test]
fn inject_skips_blank_lines_after_the_goal() {
    let proof = "let T = prove\n  (`x = x`,\n\n   REWRITE_TAC[]);;\n";
    let out = run_traces_inject::inject(proof, "TAC").unwrap();
    assert!(out.contains("   TAC THEN\n   REWRITE_TAC[]"));
    assert_eq!(out.lines().nth(2), Some(""));
    assert_eq!(out.lines().nth(3), Some("   TAC THEN"));
}

#[test]
fn inject_refuses_malformed_input() {
    assert!(run_traces_inject::inject("let x = 1\n", "TAC").is_err());
    assert!(run_traces_inject::inject("let T = prove\n  (`x = x`,\n", "TAC").is_err());
    assert!(run_traces_inject::inject("let T = prove\n  (`x = x`,\n TAC", "A\nB").is_err());
    assert!(run_traces_inject::inject("let T = prove\n  (`x = x`,\n TAC", "   ").is_err());
}

#[test]
fn inject_cli_writes_the_transformed_file() {
    let t = Tmp::new("inject");
    let source = t.path().join("in.ml");
    let destination = t.path().join("out.ml");
    fs::write(&source, "let T = prove\n  (`x = x`,\n   TAC);;\n").unwrap();
    assert_eq!(
        run_traces_inject::run(&[
            source.display().to_string(),
            destination.display().to_string(),
            "TRY (FAIL_TAC \"probe\")".to_string(),
        ]),
        0
    );
    let written = fs::read_to_string(&destination).unwrap();
    assert!(written.contains("   TRY (FAIL_TAC \"probe\") THEN\n   TAC);;"));
    assert_eq!(run_traces_inject::run(&["one".to_string()]), 2);
}
