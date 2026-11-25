//! The Rust reference trimmer and the shipped OCaml trimmer must agree.
//!
//! `src/tools/lrat_trim.rs` is the Rust measurement-side reference;
//! `tools/csat_lib.ml` carries the trimmer that actually runs in an answer
//! (between `BEGIN CSAT_TRIM_STANDALONE` and `END CSAT_TRIM_STANDALONE`). Both
//! refuse the same inputs, and the Rust refusal's `reason()` is exactly the
//! token the OCaml side prints after `CSAT_TRIM skipped=`.
//!
//! This test runs both implementations over one case table, including the
//! refusal cases (XV.A4): a negative hint and a non-empty root, plus the
//! malformed/empty/no-addition/duplicate-id paths. For a successful trim the
//! two outputs must also be byte-identical.
//!
//! The OCaml half needs `ocaml` on `PATH`. CI installs `ocaml-nox`; when the
//! interpreter is absent this test says so and checks the Rust side alone,
//! which is still the assertion that no refusal is reported as a success.
//!
//! No `.bench` input is read and no Python is used.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use tekmerion::tools::lrat_trim::{self, TrimRefusal};

static SEQ: AtomicU32 = AtomicU32::new(0);

struct Tmp(PathBuf);

impl Tmp {
    fn new(tag: &str) -> Tmp {
        let n = SEQ.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!(
            "tek-trim-agree-{}-{}-{}",
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

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The case table, shared by both implementations. `expect` is the verdict
/// token: `"trimmed"` or the `skipped=` reason. These are the same texts the
/// OCaml selftest driver (`tests/csat_trim_selftest_driver.ml`) uses, so the
/// three artefacts exercise one table.
const CASES: &[(&str, &str, &str)] = &[
    (
        "chain",
        "1 1 0 0\n2 2 0 0\n9 -1 0 1 0\n3 3 0 0\n4 1 2 0 1 9 0\n7 0 1 2 4 0\n",
        "trimmed",
    ),
    (
        "deletions",
        "10 1 0 0\n11 2 0 0\n10 d 11 0\n12 0 10 11 0\n",
        "trimmed",
    ),
    (
        "original_order",
        "5 -2 0 7 0\n7 2 0 0\n1 1 0 0\n8 0 5 0\n",
        "trimmed",
    ),
    (
        "trailing_deletion",
        "1 1 0 0\n2 0 1 0\n3 d 1 0\n",
        "trimmed",
    ),
    (
        "cnf_clause_hint",
        "100 -3 0 1 2 0\n101 0 100 0\n",
        "trimmed",
    ),
    // The two refusals XV.A4 is about.
    (
        "negative_hint",
        "1 1 0 0\n2 2 0 0\n3 0 -1 0\n",
        "negative_hint",
    ),
    (
        "negative_hint_unreachable",
        "1 1 0 0\n2 2 0 0\n9 -1 0 -2 0\n3 0 1 0\n",
        "negative_hint",
    ),
    (
        "no_empty_clause",
        "1 1 0 0\n2 2 0 0\n4 1 2 0 1 2 0\n",
        "no_empty_clause",
    ),
    ("malformed", "1 1 0 0\nnot lrat\n2 0 1 0\n", "malformed"),
    ("empty", "", "empty_lrat"),
    ("deletion_only", "1 d 1 0\n", "no_additions"),
    (
        "duplicate_ids",
        "1 1 0 0\n1 2 0 0\n2 0 1 0\n",
        "duplicate_id",
    ),
    // Precedence: the root check precedes the duplicate-id check in OCaml.
    (
        "duplicate_ids_and_non_empty_root",
        "1 1 0 0\n1 1 0 0\n2 1 0 1 0\n",
        "no_empty_clause",
    ),
];

fn rust_verdict(text: &str) -> (String, Option<String>) {
    match lrat_trim::trim(text) {
        Ok((out, _)) => ("trimmed".to_string(), Some(out)),
        Err(e) => (e.reason().to_string(), None),
    }
}

/// The shipped OCaml trimmer, extracted verbatim from `tools/csat_lib.ml`.
fn ocaml_block() -> String {
    let src = fs::read_to_string(repo().join("tools/csat_lib.ml")).expect("tools/csat_lib.ml");
    let begin = "BEGIN CSAT_TRIM_STANDALONE";
    let end = "END CSAT_TRIM_STANDALONE";
    // Both markers sit inside a comment, so the extracted text starts on the
    // line after `BEGIN` and stops at the line that opens `END`.
    let marker = src.find(begin).expect("BEGIN marker") + begin.len();
    let start = src[marker..]
        .find('\n')
        .map(|i| marker + i + 1)
        .expect("newline after BEGIN marker");
    let stop = src[start..].find(end).expect("END marker") + start;
    let stop = src[..stop].rfind('\n').map(|i| i + 1).unwrap_or(stop);
    src[start..stop].to_string()
}

const OCAML_DRIVER: &str = r#"
(* One case: rewrite the file in place and print the verdict token. The trimmed
   text stays in the same file, which the harness reads back.) *)
let () =
  csat_trim_lrat Sys.argv.(1)
"#;

fn ocaml_available() -> bool {
    Command::new("ocaml")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Run the OCaml trimmer on one input. Returns the verdict token and the
/// resulting file text.
fn ocaml_verdict(module: &Path, dir: &Path, text: &str) -> (String, String) {
    let case = dir.join("case.lrat");
    fs::write(&case, text).expect("write case");
    let out = Command::new("ocaml")
        .arg(module)
        .arg(&case)
        .current_dir(dir)
        .output()
        .expect("run ocaml");
    assert!(
        out.status.success(),
        "ocaml failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    // The trimmer prints exactly one `CSAT_TRIM ...` line.
    let mut token = String::new();
    for line in stdout.lines() {
        if let Some(reason) = line.strip_prefix("CSAT_TRIM skipped=") {
            token = reason.trim().to_string();
        } else if line.starts_with("CSAT_TRIM before=") {
            token = "trimmed".to_string();
        }
    }
    assert!(!token.is_empty(), "no CSAT_TRIM line in {stdout:?}");
    let result = fs::read_to_string(&case).expect("read trimmed file");
    (token, result)
}

#[test]
fn the_rust_and_ocaml_trimmers_reach_the_same_verdict_on_every_case() {
    // Rust half first: it must never report a refusal case as a success.
    for (name, text, expect) in CASES {
        let (token, out) = rust_verdict(text);
        assert_eq!(token, *expect, "rust verdict for {name}");
        if *expect == "trimmed" {
            assert!(out.is_some(), "{name}: success must produce a file");
        } else {
            assert!(out.is_none(), "{name}: refusal must not produce a file");
        }
    }

    if !ocaml_available() {
        eprintln!(
            "skipping the OCaml half: `ocaml` is not on PATH \
             (CI installs ocaml-nox; the Rust half above still ran)"
        );
        return;
    }

    let tmp = Tmp::new("agree");
    let module = tmp.path().join("csat_trim_agreement.ml");
    let mut src = ocaml_block();
    src.push_str(OCAML_DRIVER);
    fs::write(&module, src).expect("write module");

    for (name, text, expect) in CASES {
        let (rust_token, rust_out) = rust_verdict(text);
        let (ocaml_token, ocaml_out) = ocaml_verdict(&module, tmp.path(), text);
        assert_eq!(
            ocaml_token, *expect,
            "ocaml verdict for {name} (the selftest table and the shipped \
             trimmer disagree)"
        );
        assert_eq!(
            (rust_token.as_str(), expect),
            (ocaml_token.as_str(), expect),
            "{name}: the two implementations disagree"
        );
        if *expect == "trimmed" {
            assert_eq!(
                rust_out.as_deref().unwrap_or(""),
                ocaml_out,
                "{name}: trimmed texts differ"
            );
        }
    }
}

#[test]
fn a_negative_hint_refusal_names_the_ocaml_reason() {
    // The reason token itself is the contract; keep it pinned here so a
    // rename cannot pass the agreement test by renaming both sides.
    let err = lrat_trim::trim("1 1 0 0\n2 0 -1 0\n").unwrap_err();
    assert_eq!(err.reason(), "negative_hint");
    let err = lrat_trim::trim("1 1 0 0\n2 1 0 1 0\n").unwrap_err();
    assert_eq!(err.reason(), "no_empty_clause");
    assert!(matches!(err, TrimRefusal::NoEmptyClause { .. }));
}
