//! Integration tests for the Rust held-out guard (`src/tools/guard.rs`).
//!
//! These replace `tools/test_heldout_modes.py` and `tools/test_guard_refusals.py`.
//! Freeze verification is exercised against a throw-away git repository so the
//! tests do not depend on the real repository's tag or working-tree state.
//! Tests that need the benchmark tree skip themselves when `.bench` is absent.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use tekmerion::tools::guard::{self, default_held_out, GuardPolicy, Manifest, Mode, ModeArgs};

static SEQ: AtomicU32 = AtomicU32::new(0);

struct Tmp(PathBuf);

impl Tmp {
    fn new(tag: &str) -> Tmp {
        let n = SEQ.fetch_add(1, Ordering::SeqCst);
        let p =
            std::env::temp_dir().join(format!("tek-guard-{}-{}-{}", std::process::id(), tag, n));
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

const FREEZE_TAG: &str = "freeze-test-v1";
const SEALED_ID: &str = "x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT";
const SPENT_ID: &str = "arm.bignum_add_p384.BIGNUM_ADD_P384_CORRECT";
const COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const DECISION: &str = "IX.53 decision record research/receipts/ix53-spent-mode.md; \
                        R4 disclosed these ids as development data in \
                        research/receipts/r4-reference-route.md";

/// The declared sealed set from research/receipts/ix51-freeze-manifest.md.
const NINE: [&str; 9] = [
    "x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT",
    "x86.bignum_sub_p25519.BIGNUM_SUB_P25519_CORRECT",
    "x86.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT",
    "x86.bignum_add_p256k1.BIGNUM_ADD_P256K1_CORRECT",
    "x86.bignum_sub_p256k1.BIGNUM_SUB_P256K1_CORRECT",
    "x86.bignum_neg_p256k1.BIGNUM_NEG_P256K1_CORRECT",
    "x86.bignum_add_sm2.BIGNUM_ADD_SM2_CORRECT",
    "x86.bignum_sub_sm2.BIGNUM_SUB_SM2_CORRECT",
    "x86.bignum_neg_sm2.BIGNUM_NEG_SM2_CORRECT",
];

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn s2n() -> PathBuf {
    repo().join(".bench/s2n-bignum")
}

fn git(repo: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .unwrap_or_else(|e| panic!("cannot run git: {e}"));
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn commit_all(repo: &Path, message: &str) {
    git(repo, &["add", "-A"]);
    git(
        repo,
        &[
            "-c",
            "user.name=infinityabundance",
            "-c",
            "user.email=255699974+infinityabundance@users.noreply.github.com",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-q",
            "-m",
            message,
        ],
    );
}

/// A clean repository whose tracked `tools/` file is unmodified.
fn init_repo(tmp: &Tmp) -> (PathBuf, String) {
    let repo = tmp.path().join("repo");
    fs::create_dir_all(repo.join("tools")).unwrap();
    fs::write(repo.join("tools/tracked.py"), "# tracked\n").unwrap();
    git(&repo, &["init", "-q"]);
    commit_all(&repo, "freeze");
    git(&repo, &["tag", FREEZE_TAG]);
    let commit = git(&repo, &["rev-parse", "HEAD"]);
    (repo, commit)
}

fn quote_ids(ids: &[&str]) -> String {
    ids.iter()
        .map(|id| format!("\"{id}\""))
        .collect::<Vec<_>>()
        .join(",")
}

fn write_sealed_manifest(
    tmp: &Tmp,
    ids: &[&str],
    commit: &str,
    name: &str,
    tag: Option<&str>,
) -> PathBuf {
    let tag_field = match tag {
        Some(t) => format!(",\"freeze_tag\":\"{t}\""),
        None => String::new(),
    };
    let body = format!(
        "{{\"freeze_commit\":\"{commit}\",\"sealed_ids\":[{}]{tag_field}}}",
        quote_ids(ids)
    );
    let path = tmp.path().join(name);
    fs::write(&path, body).unwrap();
    path
}

fn sealed_policy(tmp: &Tmp, ids: &[&str]) -> (GuardPolicy, PathBuf) {
    let (repo, commit) = init_repo(tmp);
    let path = write_sealed_manifest(tmp, ids, &commit, "sealed.json", Some(FREEZE_TAG));
    let policy =
        guard::sealed_policy(&path, FREEZE_TAG, &repo, &default_held_out()).expect("sealed policy");
    (policy, repo)
}

fn write_spent_manifest(
    tmp: &Tmp,
    ids: &[&str],
    name: &str,
    decision: &str,
    reason: &str,
) -> PathBuf {
    let entries = ids
        .iter()
        .map(|id| format!("\"{id}\":\"{reason}\""))
        .collect::<Vec<_>>()
        .join(",");
    let body = format!("{{\"decision\":\"{decision}\",\"spent\":{{{entries}}}}}");
    let path = tmp.path().join(name);
    fs::write(&path, body).unwrap();
    path
}

fn spent_policy(tmp: &Tmp, ids: &[&str]) -> GuardPolicy {
    let path = write_spent_manifest(tmp, ids, "spent.json", DECISION, "disclosed development id");
    guard::spent_policy(&path, &repo(), &default_held_out()).expect("spent policy")
}

fn seated_sealed_policy(tmp: &Tmp) -> GuardPolicy {
    let path = write_sealed_manifest(tmp, &[SEALED_ID], COMMIT, "s.json", None);
    let manifest = guard::load_sealed_manifest(&path).unwrap();
    GuardPolicy::new(
        Mode::SealedEval,
        Some(Manifest::Sealed(manifest)),
        &repo(),
        &default_held_out(),
    )
    .unwrap()
}

fn task_parts(task: &str) -> (String, String) {
    let head = task.rsplit_once('.').unwrap().0;
    let (_arch, rest) = head.split_once('.').unwrap();
    let (_bignum, tail) = rest.split_once('_').unwrap();
    let (op, family) = tail.split_once('_').unwrap();
    (op.to_string(), family.to_string())
}

fn x86_object(op: &str, family: &str) -> Option<PathBuf> {
    let root = s2n().join("x86");
    let wanted = format!("bignum_{op}_{family}.o");
    let mut hits: Vec<PathBuf> = fs::read_dir(&root)
        .ok()?
        .flatten()
        .map(|e| e.path().join(&wanted))
        .filter(|p| p.is_file())
        .collect();
    hits.sort();
    if hits.len() == 1 {
        Some(hits.remove(0))
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// 1. Develop (default) is unchanged
// ---------------------------------------------------------------------------

#[test]
fn develop_refuses_heldout_task_and_object() {
    let policy = guard::develop_policy();
    let task = "x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT";

    assert!(policy.check_task(task).is_err());

    // The refusal happens on the marker, before any stat: the path need not exist.
    let obj = s2n().join("x86/curve25519/bignum_add_p25519.o");
    assert!(policy.check_object(&obj, Some(&s2n())).is_err());

    // The emitter's default entry point still refuses it.
    assert!(tekmerion::tools::emit::build_x86("add", task, &policy, &repo()).is_err());
}

#[test]
fn develop_default_is_the_cli_default() {
    let args = ModeArgs::default();
    let policy = guard::policy_from_args(&args, &repo()).unwrap();
    assert_eq!(policy.mode(), Mode::Develop);
    assert!(policy
        .check_task("x86.bignum_neg_sm2.BIGNUM_NEG_SM2_CORRECT")
        .is_err());
}

// ---------------------------------------------------------------------------
// 2. Sealed evaluation allows exactly the manifest's nine
// ---------------------------------------------------------------------------

#[test]
fn sealed_allows_declared_nine_ids() {
    let tmp = Tmp::new("sealed-nine");
    let (policy, _repo) = sealed_policy(&tmp, &NINE);
    for task in NINE {
        assert!(policy.check_task(task).is_ok(), "task {task}");
    }
}

#[test]
fn sealed_allows_declared_nine_objects() {
    if !s2n().join("x86").is_dir() {
        eprintln!(
            "skipping: {} is absent (the s2n-bignum bench is not checked in)",
            s2n().join("x86").display()
        );
        return;
    }
    let mut objects = Vec::new();
    for task in NINE {
        let (op, family) = task_parts(task);
        match x86_object(&op, &family) {
            Some(obj) => objects.push(obj),
            None => {
                eprintln!("skipping: no unique x86 object for {op}/{family}");
                return;
            }
        }
    }
    let tmp = Tmp::new("sealed-objects");
    let (policy, _repo) = sealed_policy(&tmp, &NINE);
    for obj in objects {
        let allowed = policy
            .check_object(&obj, Some(&s2n()))
            .expect("object allowed");
        assert_eq!(allowed.file_name(), obj.file_name());
    }
}

// ---------------------------------------------------------------------------
// 3. Sealed evaluation still refuses unlisted held-out ids and objects
// ---------------------------------------------------------------------------

#[test]
fn sealed_refuses_unlisted_heldout_id_and_object() {
    let tmp = Tmp::new("sealed-unlisted");
    let eight = &NINE[..NINE.len() - 1];
    let (policy, _repo) = sealed_policy(&tmp, eight);

    assert!(policy.check_task(NINE[NINE.len() - 1]).is_err());

    // An id that is held out but was never in the declared set is refused too.
    assert!(policy
        .check_task("x86.bignum_cmul_p25519.BIGNUM_CMUL_P25519_CORRECT")
        .is_err());

    let (op, family) = task_parts(NINE[NINE.len() - 1]);
    if let Some(obj) = x86_object(&op, &family) {
        assert!(policy.check_object(&obj, Some(&s2n())).is_err());
    }
    if let Some(cmul) = x86_object("cmul", "p25519") {
        assert!(policy.check_object(&cmul, Some(&s2n())).is_err());
    }
}

// ---------------------------------------------------------------------------
// 4. Sealed evaluation never opens a sealed task's proof .ml
// ---------------------------------------------------------------------------

#[test]
fn sealed_refuses_sealed_proof_file() {
    let tmp = Tmp::new("sealed-proof");
    let (policy, _repo) = sealed_policy(&tmp, &NINE);

    let proof = s2n().join("x86/proofs/bignum_add_p25519.ml");
    assert!(policy.check_proof(&proof, Some(&s2n())).is_err());

    // The guard's generic entry point must not treat a .ml as an object.
    assert!(policy.check_path(&proof, Some(&s2n())).is_err());

    // The --mode dev proof-read path is refused for a listed task as well.
    assert!(
        tekmerion::tools::emit::modulus_definition("x86", "add", "p25519", &policy, &repo())
            .is_err()
    );
    assert!(tekmerion::tools::emit::machine_code_phrase(
        "x86",
        "add",
        "p25519",
        "p25519_mc",
        "x86/curve25519/bignum_add_p25519.o",
        &policy,
        &repo()
    )
    .is_err());

    // The ARM reference emitter has the same proof-read path.
    assert!(
        tekmerion::tools::emit::modulus_definition("arm", "add", "p25519", &policy, &repo())
            .is_err()
    );
}

// ---------------------------------------------------------------------------
// 5. Freeze verification
// ---------------------------------------------------------------------------

#[test]
fn sealed_refuses_freeze_mismatch() {
    let tmp = Tmp::new("freeze-mismatch");
    let (repo, commit) = init_repo(&tmp);
    let manifest = write_sealed_manifest(&tmp, &NINE, &commit, "sealed.json", Some(FREEZE_TAG));

    assert!(guard::sealed_policy(&manifest, "no-such-tag", &repo, &default_held_out()).is_err());

    let wrong_commit = write_sealed_manifest(
        &tmp,
        &NINE,
        &"0".repeat(40),
        "wrong-commit.json",
        Some(FREEZE_TAG),
    );
    assert!(guard::sealed_policy(&wrong_commit, FREEZE_TAG, &repo, &default_held_out()).is_err());

    let wrong_tag_field = write_sealed_manifest(
        &tmp,
        &NINE,
        &commit,
        "wrong-tag-field.json",
        Some("another-tag"),
    );
    assert!(
        guard::sealed_policy(&wrong_tag_field, FREEZE_TAG, &repo, &default_held_out()).is_err()
    );

    // A tracked file under tools/ is modified: the seal is not valid.
    fs::write(repo.join("tools/tracked.py"), "# dirty\n").unwrap();
    assert!(guard::sealed_policy(&manifest, FREEZE_TAG, &repo, &default_held_out()).is_err());
}

#[test]
fn sealed_refuses_tools_that_moved_since_the_freeze() {
    // Clean is not enough: the tree must be the frozen code.
    let tmp = Tmp::new("freeze-moved");
    let (repo, commit1) = init_repo(&tmp);
    fs::write(repo.join("tools/tracked.py"), "# later\n").unwrap();
    commit_all(&repo, "later");
    let commit2 = git(&repo, &["rev-parse", "HEAD"]);

    // Clean tree at commit2, manifest frozen at commit1: refuse.
    let moved = write_sealed_manifest(&tmp, &NINE, &commit1, "moved.json", Some(FREEZE_TAG));
    match guard::sealed_policy(&moved, FREEZE_TAG, &repo, &default_held_out()) {
        Err(e) => assert!(
            e.to_string().contains("differs from the freezer commit"),
            "unexpected message {e}"
        ),
        Ok(_) => panic!("tools/ moved since the freeze was accepted"),
    }

    // Manifest records commit2 but the tag resolves to commit1: refuse.
    let wrong = write_sealed_manifest(
        &tmp,
        &NINE,
        &commit2,
        "wrong-commit2.json",
        Some(FREEZE_TAG),
    );
    match guard::sealed_policy(&wrong, FREEZE_TAG, &repo, &default_held_out()) {
        Err(e) => assert!(
            e.to_string().contains("resolves to"),
            "unexpected message {e}"
        ),
        Ok(_) => panic!("a tag/manifest commit mismatch was accepted"),
    }
}

// ---------------------------------------------------------------------------
// 6. Manifest validation
// ---------------------------------------------------------------------------

#[test]
fn manifest_rejects_duplicate_and_malformed() {
    let tmp = Tmp::new("manifest-bad");
    let zero = "0".repeat(40);
    let cases: [(&str, String); 7] = [
        (
            "duplicate",
            format!(
                "{{\"freeze_commit\":\"{zero}\",\"sealed_ids\":[\"{SEALED_ID}\",\"{SEALED_ID}\"]}}"
            ),
        ),
        (
            "bare-prefix",
            format!("{{\"freeze_commit\":\"{zero}\",\"sealed_ids\":[\"x86.bignum_add_p25519\"]}}"),
        ),
        (
            "wildcard",
            format!(
                "{{\"freeze_commit\":\"{zero}\",\"sealed_ids\":[\"x86.bignum_add_p25519.*\"]}}"
            ),
        ),
        (
            "theorem-mismatch",
            format!(
                "{{\"freeze_commit\":\"{zero}\",\"sealed_ids\":[\"x86.bignum_add_p25519.BIGNUM_SUB_P25519_CORRECT\"]}}"
            ),
        ),
        (
            "not-a-list",
            format!("{{\"freeze_commit\":\"{zero}\",\"sealed_ids\":\"{SEALED_ID}\"}}"),
        ),
        (
            "bad-commit",
            format!("{{\"freeze_commit\":\"abc\",\"sealed_ids\":[\"{SEALED_ID}\"]}}"),
        ),
        (
            "unknown-key",
            format!("{{\"freeze_commit\":\"{zero}\",\"sealed_ids\":[\"{SEALED_ID}\"],\"allow_all\":true}}"),
        ),
    ];
    for (name, body) in cases {
        let path = tmp.path().join(format!("{name}.json"));
        fs::write(&path, body).unwrap();
        assert!(
            guard::load_sealed_manifest(&path).is_err(),
            "manifest case {name} was accepted"
        );
    }
}

#[test]
fn manifest_rejects_unparseable_json() {
    let tmp = Tmp::new("manifest-broken");
    let path = tmp.path().join("broken.json");
    fs::write(&path, "{not json").unwrap();
    assert!(guard::load_sealed_manifest(&path).is_err());
}

#[test]
fn manifest_rejects_duplicate_json_keys() {
    let tmp = Tmp::new("manifest-dup-key");
    let path = tmp.path().join("duplicate-key.json");
    let body = format!(
        "{{\"freeze_commit\":\"{}\",\"sealed_ids\":[\"{}\"],\"sealed_ids\":[\"{}\"]}}",
        "0".repeat(40),
        NINE[0],
        NINE[1]
    );
    fs::write(&path, body).unwrap();
    match guard::load_sealed_manifest(&path) {
        Err(e) => assert!(
            e.to_string().contains("duplicate key"),
            "unexpected message {e}"
        ),
        Ok(_) => panic!("duplicate JSON keys were accepted"),
    }
}

#[test]
fn manifest_duplicate_message_names_the_id() {
    let tmp = Tmp::new("manifest-dup-id");
    let path = write_sealed_manifest(&tmp, &[NINE[0], NINE[0]], &"0".repeat(40), "dup.json", None);
    match guard::load_sealed_manifest(&path) {
        Err(e) => assert!(
            e.to_string().contains("duplicate"),
            "unexpected message {e}"
        ),
        Ok(_) => panic!("duplicate sealed ids were accepted"),
    }
}

// ---------------------------------------------------------------------------
// 7. Spent mode: disclosed development ids, still no proof text
// ---------------------------------------------------------------------------

#[test]
fn spent_allows_only_listed_ids() {
    let tmp = Tmp::new("spent-ids");
    let policy = spent_policy(&tmp, &NINE[..NINE.len() - 1]);
    assert_eq!(policy.mode(), Mode::Spent);
    for task in &NINE[..NINE.len() - 1] {
        assert!(policy.check_task(task).is_ok(), "task {task}");
    }
    assert!(policy.check_task(NINE[NINE.len() - 1]).is_err());
    // A held-out id that was never declared is refused even in spent mode.
    assert!(policy
        .check_task("x86.bignum_cmul_p25519.BIGNUM_CMUL_P25519_CORRECT")
        .is_err());
}

#[test]
fn spent_allows_listed_object_and_refuses_unlisted() {
    if !s2n().join("x86").is_dir() {
        eprintln!(
            "skipping: {} is absent (the s2n-bignum bench is not checked in)",
            s2n().join("x86").display()
        );
        return;
    }
    let tmp = Tmp::new("spent-objects");
    let policy = spent_policy(&tmp, &[NINE[0]]);
    let (op, family) = task_parts(NINE[0]);
    let obj = match x86_object(&op, &family) {
        Some(o) => o,
        None => {
            eprintln!("skipping: no unique x86 object for {op}/{family}");
            return;
        }
    };
    let allowed = policy
        .check_object(&obj, Some(&s2n()))
        .expect("listed object");
    assert_eq!(allowed.file_name(), obj.file_name());

    let (op, family) = task_parts(NINE[NINE.len() - 1]);
    if let Some(other) = x86_object(&op, &family) {
        assert!(policy.check_object(&other, Some(&s2n())).is_err());
    }

    // An object allowance still requires an allowed root.
    assert!(policy.check_object(&obj, None).is_err());
}

#[test]
fn spent_refuses_listed_proof_files() {
    let tmp = Tmp::new("spent-proof");
    let policy = spent_policy(&tmp, &NINE);
    let proof = s2n().join("x86/proofs/bignum_add_p25519.ml");
    match policy.check_proof(&proof, Some(&s2n())) {
        Err(e) => assert!(e.to_string().contains("never readable"), "{e}"),
        Ok(_) => panic!("a spent proof file was readable"),
    }
    assert!(policy.check_path(&proof, Some(&s2n())).is_err());

    // The emitters' --mode dev proof-read path is refused for a spent id too.
    assert!(
        tekmerion::tools::emit::modulus_definition("x86", "add", "p25519", &policy, &repo())
            .is_err()
    );
    assert!(tekmerion::tools::emit::machine_code_phrase(
        "x86",
        "add",
        "p25519",
        "p25519_mc",
        "x86/curve25519/bignum_add_p25519.o",
        &policy,
        &repo()
    )
    .is_err());
    assert!(
        tekmerion::tools::emit::modulus_definition("arm", "add", "p25519", &policy, &repo())
            .is_err()
    );

    // A proof file is never accepted as an object in spent mode either.
    assert!(policy.check_object(&proof, Some(&s2n())).is_err());
}

#[test]
fn spent_manifest_requires_decision_and_reasons() {
    let tmp = Tmp::new("spent-bad");
    let cases: [(&str, String); 7] = [
        (
            "no-decision",
            format!("{{\"decision\":\"\",\"spent\":{{\"{SEALED_ID}\":\"r\"}}}}"),
        ),
        (
            "uncited",
            format!("{{\"decision\":\"because I said so\",\"spent\":{{\"{SEALED_ID}\":\"r\"}}}}"),
        ),
        (
            "no-reason",
            format!("{{\"decision\":\"{DECISION}\",\"spent\":{{\"{SEALED_ID}\":\"  \"}}}}"),
        ),
        (
            "bad-id",
            format!("{{\"decision\":\"{DECISION}\",\"spent\":{{\"x86.bignum_add\":\"r\"}}}}"),
        ),
        ("empty", format!("{{\"decision\":\"{DECISION}\",\"spent\":{{}}}}")),
        (
            "unknown-key",
            format!(
                "{{\"decision\":\"{DECISION}\",\"spent\":{{\"{SEALED_ID}\":\"r\"}},\"allow_all\":true}}"
            ),
        ),
        ("not-an-object", format!("[\"{SEALED_ID}\"]")),
    ];
    for (name, body) in cases {
        let path = tmp.path().join(format!("{name}.json"));
        fs::write(&path, body).unwrap();
        assert!(
            guard::load_spent_manifest(&path).is_err(),
            "spent manifest case {name} was accepted"
        );
    }

    let dup = tmp.path().join("dup-spent.json");
    fs::write(
        &dup,
        format!(
            "{{\"decision\":\"{DECISION}\",\"spent\":{{\"{SEALED_ID}\":\"a\",\"{SEALED_ID}\":\"b\"}}}}"
        ),
    )
    .unwrap();
    match guard::load_spent_manifest(&dup) {
        Err(e) => assert!(
            e.to_string().contains("duplicate key"),
            "unexpected message {e}"
        ),
        Ok(_) => panic!("duplicate spent keys were accepted"),
    }
}

#[test]
fn spent_mode_is_explicit_and_develop_is_the_default() {
    let tmp = Tmp::new("spent-args");
    let manifest = write_spent_manifest(&tmp, &NINE, "spent.json", DECISION, "why");

    let args = ModeArgs {
        spent: true,
        spent_manifest: Some(manifest.display().to_string()),
        ..ModeArgs::default()
    };
    let policy = guard::policy_from_args(&args, &repo()).unwrap();
    assert_eq!(policy.mode(), Mode::Spent);
    assert!(policy.check_task(NINE[0]).is_ok());

    // Default remains develop.
    assert_eq!(
        guard::policy_from_args(&ModeArgs::default(), &repo())
            .unwrap()
            .mode(),
        Mode::Develop
    );

    let bad = [
        ModeArgs {
            spent: true,
            ..ModeArgs::default()
        },
        ModeArgs {
            spent: true,
            spent_manifest: Some(manifest.display().to_string()),
            sealed_eval: true,
            ..ModeArgs::default()
        },
        ModeArgs {
            spent: true,
            spent_manifest: Some(manifest.display().to_string()),
            freeze_tag: Some(FREEZE_TAG.to_string()),
            ..ModeArgs::default()
        },
        ModeArgs {
            spent_manifest: Some(manifest.display().to_string()),
            ..ModeArgs::default()
        },
        ModeArgs {
            spent: true,
            spent_manifest: Some(manifest.display().to_string()),
            sealed_manifest: Some(manifest.display().to_string()),
            ..ModeArgs::default()
        },
    ];
    for (i, case) in bad.into_iter().enumerate() {
        assert!(
            guard::policy_from_args(&case, &repo()).is_err(),
            "bad argument set {i} was accepted"
        );
    }

    // The CLI entry point agrees: a spent run without its manifest is refused.
    assert_eq!(
        guard::run(&["--spent".to_string(), NINE[0].to_string()]),
        guard::EXIT_REFUSED
    );
}

#[test]
fn real_spent_manifest_lists_the_eighteen_r4_arm_ids() {
    let path = repo().join("research/receipts/spent-r4-arm.json");
    let manifest = guard::load_spent_manifest(&path).expect("real spent manifest");
    assert_eq!(manifest.spent.len(), 18);
    assert!(manifest
        .decision
        .contains("research/receipts/r4-reference-route.md"));
    for (task, reason) in &manifest.spent {
        assert!(guard::parse_task_id(task).is_some(), "task {task}");
        assert!(task.starts_with("arm.bignum_"), "task {task}");
        assert!(!reason.trim().is_empty());
    }

    // The manifest's ids are exactly the 18 declared ids in the R4 verdict CSV.
    let csv = fs::read_to_string(repo().join("research/receipts/r4-verdicts.csv")).unwrap();
    let declared: Vec<String> = csv
        .lines()
        .skip(1)
        .filter_map(|line| line.split(',').next())
        .filter(|p| guard::parse_task_id(p).is_some())
        .map(|p| p.to_string())
        .collect();
    let mut manifest_ids: Vec<String> = manifest.spent.keys().cloned().collect();
    let mut declared_ids = declared;
    manifest_ids.sort();
    declared_ids.sort();
    assert_eq!(manifest_ids, declared_ids);

    let policy = guard::spent_policy(&path, &repo(), &default_held_out()).unwrap();
    assert!(policy
        .check_task("arm.bignum_add_p256k1.BIGNUM_ADD_P256K1_CORRECT")
        .is_ok());
    assert!(policy
        .check_task("arm.bignum_add_p384.BIGNUM_ADD_P384_CORRECT")
        .is_ok());
    // The real manifest is ARM-only: an x86 held-out id is still refused.
    assert!(policy
        .check_task("x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT")
        .is_err());
    let proof = s2n().join("arm/proofs/bignum_add_p256k1.ml");
    match policy.check_proof(&proof, Some(&s2n())) {
        Err(e) => assert!(e.to_string().contains("never readable"), "{e}"),
        Ok(_) => panic!("a spent ARM proof file was readable"),
    }
}

// ---------------------------------------------------------------------------
// Refusal-site coverage (ported from tools/test_guard_refusals.py)
// ---------------------------------------------------------------------------

fn refused_contains(result: guard::Result<impl std::fmt::Debug>, needle: &str) {
    match result {
        Err(e) => assert!(
            e.to_string().contains(needle),
            "expected {needle:?}, got {e}"
        ),
        Ok(v) => panic!("expected a refusal containing {needle:?}, got Ok({v:?})"),
    }
}

#[test]
fn check_regular_refusals() {
    let tmp = Tmp::new("regular");
    refused_contains(guard::check_regular(Path::new(""), None), "empty path");
    let missing = tmp.path().join("nope");
    refused_contains(guard::check_regular(&missing, None), "cannot stat");
    refused_contains(guard::check_regular(tmp.path(), None), "not a regular file");

    let inner = tmp.path().join("inner");
    fs::create_dir_all(&inner).unwrap();
    let outside = tmp.path().join("outside.txt");
    fs::write(&outside, "x").unwrap();
    refused_contains(
        guard::check_regular(&outside, Some(&inner)),
        "outside allowed root",
    );

    let empty = tmp.path().join("empty.bin");
    fs::write(&empty, b"").unwrap();
    refused_contains(guard::check_regular(&empty, None), "empty file");
}

#[test]
fn check_path_refusals() {
    refused_contains(
        guard::check_path(Path::new("anything"), &[], None),
        "empty held-out list",
    );
    refused_contains(
        guard::check_path(Path::new(""), &default_held_out(), None),
        "empty path",
    );
    let tmp = Tmp::new("marker");
    let p = tmp.path().join("p25519.txt");
    fs::write(&p, "x").unwrap();
    refused_contains(
        guard::check_path(&p, &default_held_out(), None),
        "held-out marker",
    );
}

#[test]
fn sealed_manifest_header_refusals() {
    let tmp = Tmp::new("sealed-header");
    refused_contains(
        guard::load_sealed_manifest(Path::new("")),
        "no sealed manifest",
    );

    let real = tmp.path().join("real.json");
    fs::write(
        &real,
        format!("{{\"freeze_commit\":\"{COMMIT}\",\"sealed_ids\":[\"{SEALED_ID}\"]}}"),
    )
    .unwrap();
    let link = tmp.path().join("link.json");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    refused_contains(guard::load_sealed_manifest(&link), "may not be a symlink");

    refused_contains(
        guard::load_sealed_manifest(&tmp.path().join("nope.json")),
        "cannot stat sealed manifest",
    );
    refused_contains(
        guard::load_sealed_manifest(tmp.path()),
        "not a regular file",
    );

    let empty = tmp.path().join("empty.json");
    fs::write(&empty, b"").unwrap();
    refused_contains(
        guard::load_sealed_manifest(&empty),
        "sealed manifest is empty",
    );

    let arr = tmp.path().join("arr.json");
    fs::write(&arr, "[]").unwrap();
    refused_contains(guard::load_sealed_manifest(&arr), "must be a JSON object");

    let missing = tmp.path().join("m.json");
    fs::write(&missing, format!("{{\"freeze_commit\":\"{COMMIT}\"}}")).unwrap();
    refused_contains(
        guard::load_sealed_manifest(&missing),
        "sealed_ids must be a non-empty",
    );

    let not_string = tmp.path().join("ns.json");
    fs::write(
        &not_string,
        format!("{{\"freeze_commit\":\"{COMMIT}\",\"sealed_ids\":[123]}}"),
    )
    .unwrap();
    refused_contains(
        guard::load_sealed_manifest(&not_string),
        "sealed id is not a string",
    );

    let bad_tag = tmp.path().join("bt.json");
    fs::write(
        &bad_tag,
        format!("{{\"freeze_commit\":\"{COMMIT}\",\"sealed_ids\":[\"{SEALED_ID}\"],\"freeze_tag\":\"bad tag\"}}"),
    )
    .unwrap();
    refused_contains(
        guard::load_sealed_manifest(&bad_tag),
        "freeze_tag is not a valid",
    );

    let note = tmp.path().join("note.json");
    fs::write(
        &note,
        format!("{{\"freeze_commit\":\"{COMMIT}\",\"sealed_ids\":[\"{SEALED_ID}\"],\"note\":5}}"),
    )
    .unwrap();
    refused_contains(guard::load_sealed_manifest(&note), "note must be a string");
}

#[test]
fn spent_manifest_header_refusals() {
    let tmp = Tmp::new("spent-header");
    refused_contains(
        guard::load_spent_manifest(Path::new("")),
        "no spent manifest",
    );

    let real = tmp.path().join("real.json");
    fs::write(
        &real,
        format!("{{\"decision\":\"research/receipts/x.md\",\"spent\":{{\"{SPENT_ID}\":\"r\"}}}}"),
    )
    .unwrap();
    let link = tmp.path().join("link.json");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    refused_contains(guard::load_spent_manifest(&link), "may not be a symlink");

    refused_contains(
        guard::load_spent_manifest(&tmp.path().join("nope.json")),
        "cannot stat spent manifest",
    );
    refused_contains(guard::load_spent_manifest(tmp.path()), "not a regular file");

    let empty = tmp.path().join("empty.json");
    fs::write(&empty, b"").unwrap();
    refused_contains(
        guard::load_spent_manifest(&empty),
        "spent manifest is empty",
    );

    let arr = tmp.path().join("arr.json");
    fs::write(&arr, "[]").unwrap();
    refused_contains(guard::load_spent_manifest(&arr), "must be a JSON object");

    let broken = tmp.path().join("broken.json");
    fs::write(&broken, "{not json").unwrap();
    refused_contains(
        guard::load_spent_manifest(&broken),
        "cannot parse spent manifest",
    );

    let blank = tmp.path().join("blank.json");
    fs::write(
        &blank,
        format!("{{\"decision\":\"   \",\"spent\":{{\"{SPENT_ID}\":\"r\"}}}}"),
    )
    .unwrap();
    refused_contains(guard::load_spent_manifest(&blank), "non-empty decision");

    let note = tmp.path().join("note.json");
    fs::write(
        &note,
        format!(
            "{{\"decision\":\"research/receipts/x.md\",\"spent\":{{\"{SPENT_ID}\":\"r\"}},\"note\":5}}"
        ),
    )
    .unwrap();
    refused_contains(guard::load_spent_manifest(&note), "note must be a string");
}

#[test]
fn freeze_refusals() {
    let tmp = Tmp::new("freeze-sites");
    let path = write_sealed_manifest(&tmp, &[SEALED_ID], COMMIT, "s.json", None);
    let manifest = guard::load_sealed_manifest(&path).unwrap();

    refused_contains(
        guard::verify_freeze(&manifest, "bad tag", &repo()),
        "not a valid tag name",
    );
    refused_contains(
        guard::verify_freeze(&manifest, "t", &tmp.path().join("nope")),
        "cannot run git",
    );
    refused_contains(
        guard::verify_freeze(&manifest, "t", tmp.path()),
        "failed in",
    );
}

#[test]
fn policy_construction_refusals() {
    refused_contains(
        GuardPolicy::new_named("bogus", None, &repo(), &default_held_out()),
        "unknown guard mode",
    );
    refused_contains(
        GuardPolicy::new(Mode::Develop, None, &repo(), &[]),
        "empty held-out list",
    );
    refused_contains(
        GuardPolicy::new(Mode::SealedEval, None, &repo(), &default_held_out()),
        "sealed-eval requires a sealed",
    );
    refused_contains(
        GuardPolicy::new(Mode::Spent, None, &repo(), &default_held_out()),
        "spent requires a spent",
    );
}

#[test]
fn task_object_and_proof_refusals() {
    let tmp = Tmp::new("policy-sites");
    refused_contains(
        guard::develop_policy().check_task_opt(None),
        "not a non-empty string",
    );
    refused_contains(
        guard::develop_policy().check_task(SEALED_ID),
        "held-out marker",
    );

    let sealed = seated_sealed_policy(&tmp);
    refused_contains(sealed.check_task("25519"), "not in the declared task-id");

    refused_contains(
        guard::develop_policy().check_object(Path::new(""), None),
        "empty path",
    );
    refused_contains(
        guard::develop_policy().check_object(Path::new("arm/proofs/whatever.ml"), None),
        "not an object path",
    );
    refused_contains(
        guard::develop_policy().check_object(Path::new("arm/p25519/bignum_add_p25519.o"), None),
        "held-out marker",
    );
    refused_contains(
        sealed.check_object(Path::new("arm/p25519/notbignum_p25519.o"), None),
        "not a declared bignum object",
    );
    refused_contains(
        sealed.check_object(Path::new("x86/sm2/bignum_add_sm2.o"), Some(tmp.path())),
        "is not in the",
    );
    refused_contains(
        sealed.check_object(Path::new("x86/curve25519/bignum_add_p25519.o"), None),
        "requires an allowed root",
    );
    refused_contains(
        sealed.check_proof(Path::new("x86/proofs/bignum_add_p25519.ml"), None),
        "never readable",
    );
    refused_contains(
        guard::develop_policy().check_proof(Path::new(""), None),
        "empty path",
    );
    refused_contains(
        guard::develop_policy().check_proof(Path::new("arm/proofs/bignum_add_p25519.ml"), None),
        "held-out marker",
    );
    refused_contains(
        guard::develop_policy().check_path(Path::new(""), None),
        "empty path",
    );
}

#[test]
fn policy_from_args_refusals() {
    let args = |f: fn(&mut ModeArgs)| {
        let mut a = ModeArgs::default();
        f(&mut a);
        a
    };
    refused_contains(
        guard::policy_from_args(&args(|a| a.spent = true), &repo()),
        "--spent requires --spent-manifest",
    );
    refused_contains(
        guard::policy_from_args(
            &args(|a| a.sealed_manifest = Some("x.json".into())),
            &repo(),
        ),
        "require --sealed-eval",
    );
    refused_contains(
        guard::policy_from_args(
            &args(|a| {
                a.sealed_eval = true;
                a.sealed_manifest = Some("x.json".into());
                a.freeze_tag = Some("t".into());
                a.spent_manifest = Some("y.json".into());
            }),
            &repo(),
        ),
        "not --sealed-eval",
    );
    refused_contains(
        guard::policy_from_args(
            &args(|a| {
                a.sealed_eval = true;
                a.freeze_tag = Some("t".into());
            }),
            &repo(),
        ),
        "requires --sealed-manifest",
    );
    refused_contains(
        guard::policy_from_args(
            &args(|a| {
                a.sealed_eval = true;
                a.sealed_manifest = Some("x.json".into());
            }),
            &repo(),
        ),
        "requires --freeze-tag",
    );
}

// ---------------------------------------------------------------------------
// 5b. Freeze closure (XIII.C) — the Rust producer, not just tools/
// ---------------------------------------------------------------------------

#[test]
fn sealed_manifest_accepts_only_a_clean_freeze_closure_path() {
    let tmp = Tmp::new("closure-path");
    let for_path = |rel: &str| {
        let path = tmp.path().join("c.json");
        fs::write(
            &path,
            format!(
                "{{\"freeze_commit\":\"{COMMIT}\",\"sealed_ids\":[\"{SEALED_ID}\"],\
                 \"freeze_closure\":\"{rel}\"}}"
            ),
        )
        .unwrap();
        guard::load_sealed_manifest(&path)
    };
    for bad in ["../outside.json", "/abs.json", "./x.json", "a//b.json"] {
        refused_contains(
            for_path(bad),
            "freeze_closure is not a clean repo-relative path",
        );
    }

    let ok = for_path("research/receipts/freeze-closure-v5.json").expect("clean path");
    assert_eq!(
        ok.freeze_closure.as_deref(),
        Some("research/receipts/freeze-closure-v5.json")
    );
}

#[test]
fn freeze_closure_tag_must_match_the_freeze_tag() {
    use tekmerion::tools::freeze::{self, Closure, Entry};

    let tmp = Tmp::new("closure-tag");
    let (repo, commit) = init_repo(&tmp);
    let digest = tekmerion::tools::sha256::hex(&fs::read(repo.join("tools/tracked.py")).unwrap());
    let mut closure = Closure {
        tag: "another-tag".to_string(),
        repo_commit: commit.clone(),
        created_at: 0,
        source: vec![Entry {
            path: "tools/tracked.py".to_string(),
            sha256: digest,
        }],
        source_digest: String::new(),
        runtime: None,
    };
    closure.source_digest = freeze::digest_of(&closure.source);
    fs::write(repo.join("closure.json"), freeze::render(&closure)).unwrap();

    let path = tmp.path().join("sealed-closure.json");
    fs::write(
        &path,
        format!(
            "{{\"freeze_commit\":\"{commit}\",\"sealed_ids\":[\"{SEALED_ID}\"],\
             \"freeze_tag\":\"{FREEZE_TAG}\",\"freeze_closure\":\"closure.json\"}}"
        ),
    )
    .unwrap();
    let manifest = guard::load_sealed_manifest(&path).unwrap();
    refused_contains(
        guard::verify_freeze(&manifest, FREEZE_TAG, &repo),
        "freeze closure names tag",
    );
}
