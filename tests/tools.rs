//! Integration tests for the Rust tooling in `src/tools/`.
//!
//! These replace the Python tests that previously covered the same behaviour.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use tekmerion::tools::{challenge, script_gate};

static SEQ: AtomicU32 = AtomicU32::new(0);

struct Tmp(PathBuf);

impl Tmp {
    fn new(tag: &str) -> Tmp {
        let n = SEQ.fetch_add(1, Ordering::SeqCst);
        let p =
            std::env::temp_dir().join(format!("tek-tools-{}-{}-{}", std::process::id(), tag, n));
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

const TASK: &str = "x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT";
const STATEMENT: &str = "`!z x y m n pc. foo ==> bar`";

fn corpus(statement: &str) -> String {
    format!(
        "{{\"{TASK}\":{{\"category\":\"functional_correctness_x86\",\"query\":\"{statement}\"}}}}"
    )
}

struct Fixture {
    root: Tmp,
    problems: PathBuf,
    s2n: PathBuf,
}

fn fixture() -> Fixture {
    let root = Tmp::new("challenge");
    let s2n = root.path().join("s2n");
    fs::create_dir_all(s2n.join("x86/curve25519")).unwrap();
    fs::write(s2n.join("x86/curve25519/bignum_add_p25519.o"), b"ELF-bytes").unwrap();
    let problems = root.path().join("problems.json");
    fs::write(&problems, corpus(STATEMENT)).unwrap();
    Fixture {
        root,
        problems,
        s2n,
    }
}

fn freeze(f: &Fixture, out: &Path) {
    let args = vec![
        "--task".into(),
        TASK.into(),
        "--out".into(),
        out.display().to_string(),
        "--problems".into(),
        f.problems.display().to_string(),
        "--s2n-bignum".into(),
        f.s2n.display().to_string(),
    ];
    challenge::freeze(&args).unwrap();
}

fn judged(f: &Fixture, verdict: bool) -> PathBuf {
    let ev = f.root.path().join("eval");
    fs::create_dir_all(&ev).unwrap();
    fs::write(
        ev.join("ml_files_x86_some_template.ml"),
        format!("bench_run \"{TASK}\" ({STATEMENT}) (fun () -> TAC) 1 \"x\";\n"),
    )
    .unwrap();
    if verdict {
        fs::write(ev.join(format!("{TASK}.judge.txt")), "OK").unwrap();
    }
    ev
}

fn verify(f: &Fixture, chal: &Path, evaldir: Option<&Path>) -> i32 {
    let mut args = vec![
        "--challenge".into(),
        chal.display().to_string(),
        "--problems".into(),
        f.problems.display().to_string(),
        "--s2n-bignum".into(),
        f.s2n.display().to_string(),
    ];
    if let Some(ev) = evaldir {
        args.push("--evaldir".into());
        args.push(ev.display().to_string());
    }
    challenge::verify(&args, false).unwrap()
}

#[test]
fn challenge_round_trip_passes() {
    let f = fixture();
    let chal = f.root.path().join("c.challenge");
    freeze(&f, &chal);
    let ev = judged(&f, true);
    assert_eq!(verify(&f, &chal, Some(&ev)), 0);
}

#[test]
fn challenge_detects_tampered_statement() {
    let f = fixture();
    let chal = f.root.path().join("c.challenge");
    freeze(&f, &chal);
    let mut raw = fs::read(&chal).unwrap();
    let at = raw
        .windows(20)
        .position(|w| w == b"--- statement begin ")
        .unwrap();
    raw[at + 30] = b'X';
    fs::write(&chal, &raw).unwrap();
    assert_eq!(verify(&f, &chal, None), 1);
}

#[test]
fn challenge_detects_tampered_object() {
    let f = fixture();
    let chal = f.root.path().join("c.challenge");
    freeze(&f, &chal);
    fs::write(
        f.s2n.join("x86/curve25519/bignum_add_p25519.o"),
        b"ELF-bytez",
    )
    .unwrap();
    assert_eq!(verify(&f, &chal, None), 1);
}

#[test]
fn challenge_missing_verdict_fails() {
    let f = fixture();
    let chal = f.root.path().join("c.challenge");
    freeze(&f, &chal);
    let ev = judged(&f, false);
    assert_eq!(verify(&f, &chal, Some(&ev)), 1);
}

#[test]
fn challenge_without_evaldir_is_incomplete() {
    let f = fixture();
    let chal = f.root.path().join("c.challenge");
    freeze(&f, &chal);
    assert_eq!(verify(&f, &chal, None), 3);
}

#[test]
fn challenge_detects_corpus_change() {
    let f = fixture();
    let chal = f.root.path().join("c.challenge");
    freeze(&f, &chal);
    fs::write(&f.problems, corpus("`!z. changed`")).unwrap();
    assert_eq!(verify(&f, &chal, None), 1);
}

// --- script gate (token mode; always available) ----------------------------

fn gate(tag: &str, text: &str, extra: &[&str]) -> i32 {
    let t = Tmp::new(tag);
    let p = t.path().join("candidate.txt");
    fs::write(&p, text).unwrap();
    let mut args = vec![p.display().to_string(), "--tokens-only".to_string()];
    args.extend(extra.iter().map(|s| s.to_string()));
    script_gate::run(&args)
}

#[test]
fn gate_allows_clean_expression() {
    assert_eq!(
        gate(
            "clean",
            "(fun g -> (X_GEN_TAC `x:int` THEN REWRITE_TAC[]) g)",
            &["--expr"]
        ),
        0
    );
}

#[test]
fn gate_refuses_obj_magic() {
    assert_eq!(
        gate("obj", "(fun g -> (let x = Obj.magic g in x))", &["--expr"]),
        1
    );
}

#[test]
fn gate_refuses_sys_command() {
    assert_eq!(
        gate(
            "sys",
            "(fun g -> ignore (Sys.command \"true\"); g)",
            &["--expr"]
        ),
        1
    );
}

#[test]
fn gate_refuses_shadowed_kernel_name() {
    assert_eq!(
        gate(
            "shadow",
            "let check_axioms () = ()\nlet _ = check_axioms ()\n",
            &[]
        ),
        1
    );
}

#[test]
fn gate_refuses_external() {
    assert_eq!(
        gate("ext", "external evil : int -> int = \"evil\"\n", &[]),
        1
    );
}

#[test]
fn gate_strict_refuses_refs() {
    assert_eq!(
        gate(
            "refs",
            "(fun g -> (let r = ref 0 in (r := 1; g)))",
            &["--expr", "--profile", "strict"]
        ),
        1
    );
}

#[test]
fn gate_denylist_allows_refs() {
    assert_eq!(
        gate(
            "refs2",
            "(fun g -> (let r = ref 0 in (r := 1; g)))",
            &["--expr"]
        ),
        0
    );
}
