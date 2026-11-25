//! XVI.D3: the failure taxonomy is exercised by a standalone OCaml selftest.
//!
//! `tools/bridge.ml` marks the classifier and retry policy with
//! `BEGIN/END BRIDGE_FAILURE_STANDALONE`; `tests/bridge_failure_selftest.sh`
//! extracts that block verbatim and runs it with plain OCaml. The code uses only
//! the OCaml standard library, so this needs neither HOL Light nor a solver.
//!
//! The selftest asserts one case per class and — the defect XVI.D removes —
//! that `Out_of_memory` and `Stack_overflow` do **not** run the fallback to the
//! full hypothesis set. XVII.A5 adds the other direction: a generic `Failure`
//! is not mathematical and must not retry; only the explicit allow-list does.

use std::process::Command;

fn repo() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn ocaml_available() -> bool {
    Command::new("ocaml")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn bridge_failure_taxonomy_selftest_passes() {
    if !ocaml_available() {
        eprintln!(
            "skipping the OCaml half: `ocaml` is not on PATH \
             (CI installs ocaml-nox; the taxonomy text is still covered by \
             instrument/inline-answer tests)"
        );
        return;
    }
    let script = repo().join("tests/bridge_failure_selftest.sh");
    let out = Command::new("bash")
        .arg(&script)
        .output()
        .unwrap_or_else(|e| panic!("cannot run {}: {e}", script.display()));
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "{} failed\nstdout:\n{stdout}\nstderr:\n{stderr}",
        script.display()
    );
    // The decisive assertion is named, not inferred from the exit code alone.
    for expected in [
        "OK closed: no fallback",
        "OK mathematical: fallback ran",
        "OK mathematical-slice: fallback ran",
        "OK unsupported: no fallback",
        "OK resource-memory: no fallback",
        "OK resource-stack: no fallback",
        "OK resource-signal: no fallback",
        "OK internal: no fallback",
        "OK internal-generic: no fallback",
        "OK bridge_failure_taxonomy",
    ] {
        assert!(
            stdout.contains(expected),
            "missing `{expected}` in the selftest output:\n{stdout}"
        );
    }
}
