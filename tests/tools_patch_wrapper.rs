//! Tests for the Rust port of `patch_wrapper.py`.
//!
//! The golden fixture and the refusal cases are the ones the Python
//! `--self-test` used, so the port is pinned to the same bytes.

use std::fs;
use std::path::{Path, PathBuf};

use tekmerion::tools::patch_wrapper::patch_text;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("trace/fixtures")
}

fn before() -> String {
    fs::read_to_string(fixtures().join("kernel_wrapper_before.ml")).unwrap()
}

fn after() -> String {
    fs::read_to_string(fixtures().join("kernel_wrapper_after.ml")).unwrap()
}

#[test]
fn golden_fixture_is_patched_exactly() {
    let (got, stats) = patch_text(&before()).unwrap();
    assert_eq!(got, after());
    assert!(stats.patched > 0);
    assert!(stats.conv_patched > 0);
    assert_eq!(stats.already, 0);
    assert_eq!(stats.conv_already, 0);
}

#[test]
fn patching_is_idempotent() {
    let (once, stats) = patch_text(&before()).unwrap();
    let (twice, stats2) = patch_text(&once).unwrap();
    assert_eq!(twice, after());
    assert_eq!(stats2.patched, 0);
    assert_eq!(stats2.already, stats.patched);
    assert_eq!(stats2.conv_patched, 0);
    assert_eq!(stats2.conv_already, stats.conv_patched);
}

#[test]
fn conversion_only_input_is_patched() {
    let conv_only = concat!(
        "  let thm_output = BETA_CONV tm in\n",
        "  exptrace_add_conv \"BETA_CONV\" {\n",
        "    input = tm;\n",
        "  } (fun () -> {});\n",
    );
    let (out, stats) = patch_text(conv_only).unwrap();
    assert_eq!(stats.conv_patched, 1);
    assert!(out
        .contains("exptrace_guard_conv \"BETA_CONV\" (fun () -> tm) (fun () -> BETA_CONV tm) in"));
}

fn refusal(text: &str, needle: &str) {
    let error = patch_text(text).unwrap_err();
    assert!(
        error.0.contains(needle),
        "expected {needle:?} in {:?}",
        error.0
    );
}

#[test]
fn malformed_shapes_are_refused() {
    refusal(
        concat!(
            "    let (trace_ga_inst,trace_goals_after,trace_justf) = ",
            "FOO (trace_asl,trace_g) in\n",
        ),
        "cannot find the tactic name",
    );
    refusal(
        "  let thm_output = BETA_CONV tm in\n",
        "cannot find the conversion name",
    );
    refusal(
        concat!(
            "  let thm_output = BETA_CONV tm in\n",
            "  exptrace_add_conv \"BETA_CONV\" {\n",
            "  } (fun () -> {});\n",
        ),
        "cannot find the conversion input term",
    );
    refusal("let x = 1\n", "no tactic or conversion wrapper matched");
}

#[test]
fn already_patched_without_a_guard_is_refused() {
    refusal(
        concat!(
            "    let (trace_ga_inst,trace_goals_after,trace_justf) =\n",
            "      BOGUS (fun () -> foo) in\n",
        ),
        "already-patched application",
    );
    refusal(
        concat!("  let thm_output =\n", "    BOGUS in\n"),
        "already-patched conversion",
    );
}

#[test]
fn cli_writes_the_patched_file_and_reports_counts() {
    let destination = std::env::temp_dir().join(format!(
        "tek-patch-wrapper-{}-{}.ml",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let rc = tekmerion::tools::patch_wrapper::run(&[
        fixtures()
            .join("kernel_wrapper_before.ml")
            .display()
            .to_string(),
        destination.display().to_string(),
    ]);
    assert_eq!(rc, 0);
    assert_eq!(fs::read_to_string(&destination).unwrap(), after());
    let _ = fs::remove_file(&destination);
}

#[test]
fn cli_self_test_passes() {
    assert_eq!(
        tekmerion::tools::patch_wrapper::run(&["--self-test".to_string()]),
        0
    );
}
