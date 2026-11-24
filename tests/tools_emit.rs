//! Byte-identity tests for the Rust emitters.
//!
//! The reference answers under `.bench/dropin-gen/answers{,-arm}` were produced
//! by the Python emitters at the pinned inputs. For every task the Rust emitter
//! in `--mode answer` must reproduce the file byte for byte, under the same
//! guard mode and manifest. The tests skip themselves when `.bench` is absent.
//!
//! IX.63 boundary: `tools/csat_lib.ml` gained the backward-closure LRAT trim, so
//! the inlined answer text changed. The `.bench` reference answers were
//! regenerated with that library text; these tests therefore pin emitter
//! consistency with the post-IX.63 answers, not byte identity with the Python
//! originals. That older baseline is retired and recorded in
//! `research/receipts/slice-b-rust-emitters.md` ("IX.63 retirement"). The
//! theorems are unchanged (IX.63 T4).
//!
//! IX.65 boundary: `tools/csat_lib.ml` gained the guarded `csat_mark` phrases
//! used only by `--mode phase`. The normal tactic is untouched, which the
//! phase-contract tests below prove by stripping every marker and comparing to
//! the answer the same binary emits in `--mode answer`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use tekmerion::tools::emit;
use tekmerion::tools::guard::{self, default_held_out, GuardPolicy, Manifest, Mode};

static SEQ: AtomicU32 = AtomicU32::new(0);

struct Tmp(PathBuf);

impl Tmp {
    fn new(tag: &str) -> Tmp {
        let n = SEQ.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!("tek-emit-{}-{}-{}", std::process::id(), tag, n));
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

fn answers_x86() -> PathBuf {
    repo().join(".bench/dropin-gen/answers")
}

fn answers_arm() -> PathBuf {
    repo().join(".bench/dropin-gen/answers-arm")
}

fn sealed_manifest_path() -> PathBuf {
    repo().join("research/receipts/ix55-sealed-manifest.json")
}

fn spent_manifest_path() -> PathBuf {
    repo().join("research/receipts/spent-r4-arm.json")
}

fn sealed_policy() -> GuardPolicy {
    let manifest = guard::load_sealed_manifest(&sealed_manifest_path()).expect("sealed manifest");
    GuardPolicy::new(
        Mode::SealedEval,
        Some(Manifest::Sealed(manifest)),
        &repo(),
        &default_held_out(),
    )
    .expect("sealed policy")
}

fn spent_policy() -> GuardPolicy {
    let manifest = guard::load_spent_manifest(&spent_manifest_path()).expect("spent manifest");
    GuardPolicy::new(
        Mode::Spent,
        Some(Manifest::Spent(manifest)),
        &repo(),
        &default_held_out(),
    )
    .expect("spent policy")
}

const X86_TASKS: [&str; 9] = [
    "x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT",
    "x86.bignum_add_p256k1.BIGNUM_ADD_P256K1_CORRECT",
    "x86.bignum_add_sm2.BIGNUM_ADD_SM2_CORRECT",
    "x86.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT",
    "x86.bignum_neg_p256k1.BIGNUM_NEG_P256K1_CORRECT",
    "x86.bignum_neg_sm2.BIGNUM_NEG_SM2_CORRECT",
    "x86.bignum_sub_p25519.BIGNUM_SUB_P25519_CORRECT",
    "x86.bignum_sub_p256k1.BIGNUM_SUB_P256K1_CORRECT",
    "x86.bignum_sub_sm2.BIGNUM_SUB_SM2_CORRECT",
];

const ARM_TASKS: [&str; 18] = [
    "arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT",
    "arm.bignum_add_p256.BIGNUM_ADD_P256_CORRECT",
    "arm.bignum_add_p256k1.BIGNUM_ADD_P256K1_CORRECT",
    "arm.bignum_add_p384.BIGNUM_ADD_P384_CORRECT",
    "arm.bignum_add_p521.BIGNUM_ADD_P521_CORRECT",
    "arm.bignum_add_sm2.BIGNUM_ADD_SM2_CORRECT",
    "arm.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT",
    "arm.bignum_neg_p256.BIGNUM_NEG_P256_CORRECT",
    "arm.bignum_neg_p256k1.BIGNUM_NEG_P256K1_CORRECT",
    "arm.bignum_neg_p384.BIGNUM_NEG_P384_CORRECT",
    "arm.bignum_neg_p521.BIGNUM_NEG_P521_CORRECT",
    "arm.bignum_neg_sm2.BIGNUM_NEG_SM2_CORRECT",
    "arm.bignum_sub_p25519.BIGNUM_SUB_P25519_CORRECT",
    "arm.bignum_sub_p256.BIGNUM_SUB_P256_CORRECT",
    "arm.bignum_sub_p256k1.BIGNUM_SUB_P256K1_CORRECT",
    "arm.bignum_sub_p384.BIGNUM_SUB_P384_CORRECT",
    "arm.bignum_sub_p521.BIGNUM_SUB_P521_CORRECT",
    "arm.bignum_sub_sm2.BIGNUM_SUB_SM2_CORRECT",
];

/// Assert byte equality, reporting the first differing offset.
fn assert_identical(task: &str, got: &[u8], want: &[u8]) {
    if got == want {
        return;
    }
    let at = got
        .iter()
        .zip(want.iter())
        .position(|(a, b)| a != b)
        .unwrap_or_else(|| got.len().min(want.len()));
    let from = at.saturating_sub(40);
    let to_got = (at + 40).min(got.len());
    let to_want = (at + 40).min(want.len());
    panic!(
        "{task}: first difference at byte {at} (got {} bytes, want {} bytes)\n  got:  {:?}\n  want: {:?}",
        got.len(),
        want.len(),
        String::from_utf8_lossy(&got[from..to_got]),
        String::from_utf8_lossy(&want[from..to_want]),
    );
}

#[test]
fn x86_answers_are_byte_identical() {
    if !answers_x86().is_dir() {
        return;
    }
    let policy = sealed_policy();
    let mut count = 0;
    for task in X86_TASKS {
        let parsed = guard::parse_task_id(task).expect("declared task id");
        let info = emit::build_x86(&parsed.op, task, &policy, &repo())
            .unwrap_or_else(|e| panic!("{task}: {e}"));
        let text = emit::inlined_answer(&info.tactic, &repo()).expect("answer");
        let path = answers_x86().join(format!("bignum_{}_{}.answer.txt", parsed.op, parsed.family));
        let want = fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_identical(task, text.as_bytes(), &want);
        count += 1;
    }
    assert_eq!(count, 9);
}

#[test]
fn arm_answers_are_byte_identical() {
    if !answers_arm().is_dir() {
        return;
    }
    let policy = spent_policy();
    let mut count = 0;
    for task in ARM_TASKS {
        let parsed = guard::parse_task_id(task).expect("declared task id");
        let info = emit::build_arm(&parsed.op, task, &policy, &repo())
            .unwrap_or_else(|e| panic!("{task}: {e}"));
        let text = emit::inlined_answer(&info.tactic, &repo()).expect("answer");
        let path = answers_arm().join(format!("bignum_{}_{}.answer.txt", parsed.op, parsed.family));
        let want = fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_identical(task, text.as_bytes(), &want);
        count += 1;
    }
    assert_eq!(count, 18);
}

#[test]
fn arm_dev_mode_refuses_a_spent_proof_file() {
    if !answers_arm().is_dir() {
        return;
    }
    let policy = spent_policy();
    let task = "arm.bignum_add_p256.BIGNUM_ADD_P256_CORRECT";
    let info = emit::build_arm("add", task, &policy, &repo()).expect("build");
    match emit::dev_script_arm(&info, &policy, &repo()) {
        Err(e) => assert!(
            e.to_string().contains("never readable"),
            "unexpected dev refusal: {e}"
        ),
        Ok(_) => panic!("spent mode produced a dev script from the proof text"),
    }
}

#[test]
fn x86_dev_mode_refuses_a_sealed_proof_file() {
    if !answers_x86().is_dir() {
        return;
    }
    let policy = sealed_policy();
    let task = "x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT";
    let info = emit::build_x86("add", task, &policy, &repo()).expect("build");
    match emit::dev_script_x86(&info, &policy, &repo()) {
        Err(e) => assert!(
            e.to_string().contains("never readable"),
            "unexpected dev refusal: {e}"
        ),
        Ok(_) => panic!("sealed mode produced a dev script from the proof text"),
    }
}

/// Freeze verification end to end: a throw-away repository at a tag, holding a
/// copy of `tools/` and a symlink to the benchmark tree, must reproduce an
/// answer byte for byte.
#[test]
fn sealed_freeze_verification_end_to_end() {
    if !answers_x86().is_dir() {
        return;
    }
    if Command::new("git").arg("--version").output().is_err() {
        return;
    }
    let tmp = Tmp::new("freeze");
    let root = tmp.path();
    fs::create_dir_all(root.join("tools")).unwrap();
    for name in ["csat_lib.ml", "bridge.ml"] {
        fs::copy(
            repo().join("tools").join(name),
            root.join("tools").join(name),
        )
        .unwrap();
    }
    std::os::unix::fs::symlink(repo().join(".bench"), root.join(".bench")).unwrap();

    let git = |args: &[&str]| {
        let out = Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .unwrap_or_else(|e| panic!("cannot run git: {e}"));
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };
    git(&["init", "-q"]);
    git(&["add", "tools/csat_lib.ml", "tools/bridge.ml"]);
    git(&[
        "-c",
        "user.name=infinityabundance",
        "-c",
        "user.email=255699974+infinityabundance@users.noreply.github.com",
        "-c",
        "commit.gpgsign=false",
        "commit",
        "-q",
        "-m",
        "freeze",
    ]);
    git(&["tag", "freeze-rust-test"]);
    let commit = git(&["rev-parse", "HEAD"]);

    let task = X86_TASKS[0];
    let manifest_path = root.join("sealed.json");
    fs::write(
        &manifest_path,
        format!(
            "{{\"freeze_commit\":\"{commit}\",\"freeze_tag\":\"freeze-rust-test\",\"sealed_ids\":[\"{task}\"]}}"
        ),
    )
    .unwrap();

    let policy = guard::sealed_policy(
        &manifest_path,
        "freeze-rust-test",
        root,
        &default_held_out(),
    )
    .expect("frozen tree must be accepted");
    let info = emit::build_x86("add", task, &policy, root).expect("build");
    let text = emit::inlined_answer(&info.tactic, root).expect("answer");
    let want = fs::read(answers_x86().join("bignum_add_p25519.answer.txt")).unwrap();
    assert_identical(task, text.as_bytes(), &want);
}

// ---------------------------------------------------------------------------
// The certificate-size probe's template reader (XIII.A step 1).
// ---------------------------------------------------------------------------

/// A synthetic repository with only the two files `template_prefix` reads.
fn template_repo(tag: &str, problems: &str, template: &str) -> Tmp {
    let tmp = Tmp::new(tag);
    fs::create_dir_all(tmp.path().join(".bench")).unwrap();
    fs::write(tmp.path().join(".bench/problems.json"), problems).unwrap();
    fs::write(tmp.path().join(".bench/t.ml"), template).unwrap();
    tmp
}

#[test]
fn template_prefix_takes_exactly_the_harness_line_count() {
    let repo = template_repo(
        "tpl",
        r#"{"t":{"inlined_locations":[["t.ml",2]]}}"#,
        "a\nb\nc\n",
    );
    let got = emit::template_prefix(repo.path(), "t").expect("prefix");
    assert_eq!(got, "a\nb\n");
}

#[test]
fn template_prefix_keeps_everything_when_linenum_exceeds_the_file() {
    let repo = template_repo(
        "tpl-long",
        r#"{"t":{"inlined_locations":[["t.ml",9]]}}"#,
        "a\nb\nc",
    );
    let got = emit::template_prefix(repo.path(), "t").expect("prefix");
    assert_eq!(got, "a\nb\nc\n");
}

#[test]
fn template_prefix_refuses_a_task_without_locations() {
    let repo = template_repo("tpl-missing", r#"{"t":{}}"#, "");
    assert!(emit::template_prefix(repo.path(), "t").is_err());
}

// ---------------------------------------------------------------------------
// Phase markers (IX.65): measurement mode is additive only.
// ---------------------------------------------------------------------------

/// The tactic text of an answer: everything between the `(fun g ->` wrapper's
/// `(` and its `) g`.
fn answer_tactic(answer: &str) -> String {
    let start = answer
        .find("  try\n    (")
        .unwrap_or_else(|| panic!("no answer wrapper in {answer:?}"))
        + "  try\n    (".len();
    let end = answer
        .rfind(") g\n  with e ->")
        .unwrap_or_else(|| panic!("no answer wrapper end"));
    answer[start..end].to_string()
}

#[test]
fn phase_markers_are_removable_and_the_class_is_mechanical() {
    let tactic = "MAP_EVERY X_GEN_TAC [`z:int64`] THEN\n\
                  REWRITE_TAC[C_ARGUMENTS] THEN\n\
                  REWRITE_TAC[BIGNUM_FROM_MEMORY_BYTES] THEN ENSURES_INIT_TAC \"s0\" THEN\n\
                  ARM_ACCSTEPS_TAC EXEC [3;4] (1--4) THEN\n\
                  ENSURES_FINAL_STATE_TAC THEN ASM_REWRITE_TAC[] THEN STRIP_TAC THEN\n\
                  ASM_REWRITE_TAC[] THEN DISCARD_STATE_TAC \"s4\" THEN\n\
                  BRIDGE_ADD_REF_TAC (CONV_TAC (ONCE_DEPTH_CONV BIGNUM_OF_WORDLIST_CONV)) p";
    let phase = emit::phase_tactic(tactic);
    assert_eq!(emit::strip_phase_marks(&phase).as_deref(), Some(tactic));
    assert_eq!(
        emit::phase_class("MAP_EVERY X_GEN_TAC [`z:int64`]"),
        emit::PhaseClass::Rewrite
    );
    assert_eq!(
        emit::phase_class("REWRITE_TAC[X] THEN ENSURES_INIT_TAC \"s0\""),
        emit::PhaseClass::Finalisation
    );
    assert_eq!(
        emit::phase_class("ARM_ACCSTEPS_TAC EXEC [3;4] (1--4)"),
        emit::PhaseClass::Play
    );
    assert_eq!(
        emit::phase_class("BRIDGE_ADD_REF_TAC x p"),
        emit::PhaseClass::Bridge
    );
    assert_eq!(
        emit::phase_class("ENSURES_FINAL_STATE_TAC THEN DISCARD_STATE_TAC \"s4\""),
        emit::PhaseClass::Finalisation
    );
}

#[test]
fn phase_mode_is_the_normal_emitter_plus_marks() {
    if !answers_arm().is_dir() {
        return;
    }
    let policy = spent_policy();
    let task = "arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT";
    let parsed = guard::parse_task_id(task).expect("declared task id");
    let info = emit::build_arm(&parsed.op, task, &policy, &repo()).expect("build");
    let normal = emit::inlined_answer(&info.tactic, &repo()).expect("normal answer");
    let phase =
        emit::inlined_answer(&emit::phase_tactic(&info.tactic), &repo()).expect("phase answer");
    assert_ne!(normal, phase);
    assert_eq!(answer_tactic(&phase), emit::phase_tactic(&info.tactic));
    let stripped = emit::strip_phase_marks(&answer_tactic(&phase)).expect("strip");
    assert_eq!(stripped, info.tactic);
    assert_eq!(answer_tactic(&normal), info.tactic);
    // The library text is the same on both paths: only the tactic differs.
    let lib_end = normal.find("(fun g ->").unwrap();
    assert_eq!(&normal[..lib_end], &phase[..lib_end]);
}
