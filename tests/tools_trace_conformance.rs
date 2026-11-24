//! Tests for the Rust port of `trace_conformance.py`.
//!
//! The fixture pair is the one the Python `--self-test` used; the rest pins
//! the alignment, skip and refusal behaviour.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use tekmerion::tools::trace_conformance::{collect, compare_dirs, run};

static SEQ: AtomicU32 = AtomicU32::new(0);

struct Tmp(PathBuf);

impl Tmp {
    fn new(tag: &str) -> Tmp {
        let n = SEQ.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!(
            "tek-conformance-{}-{}-{}",
            std::process::id(),
            tag,
            n
        ));
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

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("trace/fixtures/conformance")
}

fn record(tactic: &str, num_subgoals: u32) -> String {
    format!(
        "[{{\"tactic\":\"{tactic}\",\
         \"definition_line_number\":{{\"file_path\":\"hol.ml\",\"line\":245}},\
         \"user_line_number\":{{\"file_path\":\"proof.ml\",\"line\":57}},\
         \"arg_names\":[\"thms\"],\"arg_types\":[\"thm list\"],\
         \"arg_values\":[\"[]\"],\"arg_exprs\":[\"[]\"],\
         \"goal_before\":\"|- x = x\",\
         \"goals_after\":[{{\"goal\":\"|- x = x\"}}],\
         \"num_subgoals\":{num_subgoals}}}]"
    )
}

#[test]
fn matching_fixture_pair_has_no_differences() {
    let (checked, skipped, diffs) =
        compare_dirs(&fixtures().join("stock"), &fixtures().join("patched")).unwrap();
    assert_eq!(checked, 2);
    assert_eq!(skipped, 0);
    assert!(diffs.is_empty(), "{diffs:?}");
}

#[test]
fn diverging_fixture_pair_names_num_subgoals() {
    let (checked, _skipped, diffs) =
        compare_dirs(&fixtures().join("stock"), &fixtures().join("diverged")).unwrap();
    assert_eq!(checked, 2);
    assert!(!diffs.is_empty());
    assert!(diffs.iter().any(|d| d.contains("num_subgoals")));
}

#[test]
fn identical_synthetic_directories_pass() {
    let root = Tmp::new("identical");
    for side in ["stock", "patched"] {
        fs::create_dir_all(root.path().join(side)).unwrap();
        fs::write(
            root.path().join(side).join("REWRITE_TAC.json"),
            record("REWRITE_TAC", 1),
        )
        .unwrap();
    }
    let (checked, skipped, diffs) =
        compare_dirs(&root.path().join("stock"), &root.path().join("patched")).unwrap();
    assert_eq!((checked, skipped, diffs.len()), (1, 0, 0));
}

#[test]
fn conversion_traces_are_skipped() {
    let root = Tmp::new("conv");
    for side in ["stock", "patched"] {
        let dir = root.path().join(side);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("REWRITE_TAC.json"), record("REWRITE_TAC", 1)).unwrap();
        fs::write(
            dir.join("BETA_CONV.json"),
            "[{\"conv\":\"BETA_CONV\",\"input\":\"x\"}]",
        )
        .unwrap();
    }
    let (checked, skipped, diffs) =
        compare_dirs(&root.path().join("stock"), &root.path().join("patched")).unwrap();
    assert_eq!(checked, 1);
    assert_eq!(skipped, 2);
    assert!(diffs.is_empty());
}

#[test]
fn missing_fields_are_reported() {
    let root = Tmp::new("missing-field");
    for side in ["stock", "patched"] {
        fs::create_dir_all(root.path().join(side)).unwrap();
    }
    fs::write(
        root.path().join("stock/REWRITE_TAC.json"),
        record("REWRITE_TAC", 1),
    )
    .unwrap();
    let without = record("REWRITE_TAC", 1).replace("\"arg_names\":[\"thms\"],", "");
    fs::write(root.path().join("patched/REWRITE_TAC.json"), without).unwrap();
    let (_checked, _skipped, diffs) =
        compare_dirs(&root.path().join("stock"), &root.path().join("patched")).unwrap();
    assert!(
        diffs
            .iter()
            .any(|d| d.contains("arg_names") && d.contains("missing")),
        "{diffs:?}"
    );
}

#[test]
fn duplicate_alignment_keys_are_refused() {
    let dir = Tmp::new("duplicate");
    fs::write(dir.path().join("a.json"), record("REWRITE_TAC", 1)).unwrap();
    fs::write(dir.path().join("b.json"), record("REWRITE_TAC", 2)).unwrap();
    let error = collect(dir.path()).unwrap_err();
    assert!(error.0.contains("duplicate alignment key"));
}

#[test]
fn non_tactic_records_are_refused() {
    let dir = Tmp::new("neither");
    fs::write(dir.path().join("x.json"), "[{\"other\":1}]").unwrap();
    let error = collect(dir.path()).unwrap_err();
    assert!(error.0.contains("neither a 'tactic' nor a 'conv' field"));
}

#[test]
fn invalid_json_and_non_arrays_are_refused() {
    let dir = Tmp::new("invalid");
    fs::write(dir.path().join("x.json"), "{oops").unwrap();
    assert!(collect(dir.path()).unwrap_err().0.contains("invalid JSON"));

    let dir = Tmp::new("nonarray");
    fs::write(dir.path().join("x.json"), "{\"tactic\":\"T\"}").unwrap();
    assert!(collect(dir.path())
        .unwrap_err()
        .0
        .contains("expected a JSON array of records"));
}

#[test]
fn missing_directory_and_empty_pair_are_refused() {
    let error = collect(Path::new("/definitely/not/here")).unwrap_err();
    assert!(error.0.contains("not a directory"));

    let root = Tmp::new("empty");
    fs::create_dir_all(root.path().join("stock")).unwrap();
    fs::create_dir_all(root.path().join("patched")).unwrap();
    let error = compare_dirs(&root.path().join("stock"), &root.path().join("patched")).unwrap_err();
    assert!(error.0.contains("no <tac>.json records found"));
}

#[test]
fn report_exit_codes_match_the_fixtures() {
    assert_eq!(
        run(&[
            fixtures().join("stock").display().to_string(),
            fixtures().join("patched").display().to_string(),
        ]),
        0
    );
    assert_eq!(
        run(&[
            fixtures().join("stock").display().to_string(),
            fixtures().join("diverged").display().to_string(),
        ]),
        1
    );
    assert_eq!(
        run(&["/definitely/not/here".to_string(), "/nor/here".to_string()]),
        2
    );
}

#[test]
fn cli_self_test_passes() {
    assert_eq!(run(&["--self-test".to_string()]), 0);
}
