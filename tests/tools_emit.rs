//! Byte-identity tests for the Rust emitters.
//!
//! The reference answers are tracked under `tests/fixtures/answers/`. For every
//! task the Rust emitter in `--mode answer` must reproduce the file byte for
//! byte, under the same guard mode and manifest. The emitter's *inputs* are also
//! tracked (`tests/fixtures/problems.json` and `tests/fixtures/bench/`), so the
//! suite runs for real in a bench-free checkout: it materialises those inputs
//! into a temporary `.bench` and points the corpus at the fixture through
//! `TEKMERION_PROBLEMS` (XVI.A; `research/receipts/ix72-offline-inputs.md`).
//!
//! Missing fixtures are failures, not skips (XVI.A2). The only early returns
//! left are for genuinely machine-specific tools (`git`, `ocaml`) and they print
//! why.
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

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Once;

use tekmerion::tools::corpus;
use tekmerion::tools::emit;
use tekmerion::tools::guard::{self, default_held_out, GuardPolicy, Manifest, Mode};
use tekmerion::tools::json;

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

/// Tracked reference answers. These are the only reference the suite accepts:
/// `.bench/dropin-gen` protects just this machine, while the fixtures travel
/// with the tree. They are re-recorded deliberately when the emitted library
/// text changes (IX.63 added the LRAT trimmer, which every answer inlines) — see
/// `research/receipts/ix68-ci-and-integrity.md`.
fn answers_fixture(arch: &str) -> PathBuf {
    repo().join("tests/fixtures/answers").join(arch)
}

/// `tests/fixtures/problems.json`, the committed 27-entry corpus excerpt.
fn problems_fixture() -> PathBuf {
    corpus::problems_fixture(&repo())
}

/// Fail, naming the fixture, when the committed corpus excerpt is absent.
/// XVI.A2: a missing input is a failure, never a silent skip.
fn require_problems_fixture() -> PathBuf {
    let path = problems_fixture();
    assert!(
        path.is_file(),
        "missing input fixture {}: regenerate it with `cargo run --release \
         --offline --bin record_problems_fixture -- --repo {}` (see \
         research/receipts/ix72-offline-inputs.md)",
        path.display(),
        repo().display()
    );
    path
}

/// Point the emitter at the committed corpus for the rest of this process.
///
/// `TEKMERION_PROBLEMS` is a *fallback*: a repo-local `.bench/problems.json`
/// always wins, so this cannot shadow the live pin, and the synthetic-corpus
/// tests (which write their own `problems.json`) are unaffected. It is set once
/// because the value is the same for every test that needs it.
fn use_fixture_corpus() {
    let fixture = require_problems_fixture();
    static ONCE: Once = Once::new();
    ONCE.call_once(|| std::env::set_var(emit::PROBLEMS_ENV, fixture));
}

/// Materialise `tests/fixtures/bench/` into a temporary `.bench`, then add the
/// two tracked library texts under `tools/`. The result is a repository root the
/// emitter can read without the live 20 GB bench; `problems.json` is supplied by
/// `TEKMERION_PROBLEMS` instead, so the override is genuinely load-bearing.
fn fixture_bench(tag: &str) -> Tmp {
    let fixtures = repo().join("tests/fixtures");
    let tmp = Tmp::new(tag);
    corpus::materialise_bench_inputs(&fixtures, tmp.path()).unwrap_or_else(|e| {
        panic!(
            "missing input fixture {}/bench: {e}; regenerate it with \
             `cargo run --release --offline --bin record_problems_fixture` \
             (see research/receipts/ix72-offline-inputs.md)",
            fixtures.display()
        )
    });
    let tools = tmp.path().join("tools");
    fs::create_dir_all(&tools).unwrap();
    for name in ["csat_lib.ml", "bridge.ml"] {
        let from = repo().join("tools").join(name);
        fs::copy(&from, tools.join(name)).unwrap_or_else(|e| panic!("{}: {e}", from.display()));
    }
    tmp
}

/// Set `TEKMERION_RECORD_ANSWERS=1` to (re-)record the fixtures instead of
/// asserting. Recording is a deliberate act, recorded in a receipt.
fn recording_answers() -> bool {
    std::env::var_os("TEKMERION_RECORD_ANSWERS").is_some()
}

/// The tracked fixture answer. A missing file is a failure naming the path, not
/// a fallback to the git-ignored `.bench` copy (XVI.A2).
fn reference_answer(arch: &str, base: &str) -> PathBuf {
    let path = answers_fixture(arch).join(format!("{base}.answer.txt"));
    assert!(
        path.is_file(),
        "missing tracked reference answer {}: re-record with \
         TEKMERION_RECORD_ANSWERS=1 (see research/receipts/ix68-ci-and-integrity.md)",
        path.display()
    );
    path
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

const X86_TASKS: [&str; 9] = corpus::X86_TASKS;

const ARM_TASKS: [&str; 18] = corpus::ARM_TASKS;

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
    use_fixture_corpus();
    let bench = fixture_bench("x86-answers");
    let policy = sealed_policy();
    let mut count = 0;
    for task in X86_TASKS {
        let parsed = guard::parse_task_id(task).expect("declared task id");
        let info = emit::build_x86(&parsed.op, task, &policy, bench.path())
            .unwrap_or_else(|e| panic!("{task}: {e}"));
        let text = emit::inlined_answer(&info.tactic, bench.path()).expect("answer");
        let base = format!("bignum_{}_{}", parsed.op, parsed.family);
        if recording_answers() {
            let dir = answers_fixture("x86");
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join(format!("{base}.answer.txt")), text.as_bytes()).unwrap();
            count += 1;
            continue;
        }
        let path = reference_answer("x86", &base);
        let want = fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_identical(task, text.as_bytes(), &want);
        count += 1;
    }
    assert_eq!(count, 9);
}

#[test]
fn arm_answers_are_byte_identical() {
    use_fixture_corpus();
    let bench = fixture_bench("arm-answers");
    let policy = spent_policy();
    let mut count = 0;
    for task in ARM_TASKS {
        let parsed = guard::parse_task_id(task).expect("declared task id");
        let info = emit::build_arm(&parsed.op, task, &policy, bench.path())
            .unwrap_or_else(|e| panic!("{task}: {e}"));
        let text = emit::inlined_answer(&info.tactic, bench.path()).expect("answer");
        let base = format!("bignum_{}_{}", parsed.op, parsed.family);
        if recording_answers() {
            let dir = answers_fixture("arm");
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join(format!("{base}.answer.txt")), text.as_bytes()).unwrap();
            count += 1;
            continue;
        }
        let path = reference_answer("arm", &base);
        let want = fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_identical(task, text.as_bytes(), &want);
        count += 1;
    }
    assert_eq!(count, 18);
}

#[test]
fn arm_dev_mode_refuses_a_spent_proof_file() {
    use_fixture_corpus();
    let bench = fixture_bench("arm-dev");
    let policy = spent_policy();
    let task = "arm.bignum_add_p256.BIGNUM_ADD_P256_CORRECT";
    let info = emit::build_arm("add", task, &policy, bench.path()).expect("build");
    match emit::dev_script_arm(&info, &policy, bench.path()) {
        Err(e) => assert!(
            e.to_string().contains("never readable"),
            "unexpected dev refusal: {e}"
        ),
        Ok(_) => panic!("spent mode produced a dev script from the proof text"),
    }
}

#[test]
fn x86_dev_mode_refuses_a_sealed_proof_file() {
    use_fixture_corpus();
    let bench = fixture_bench("x86-dev");
    let policy = sealed_policy();
    let task = "x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT";
    let info = emit::build_x86("add", task, &policy, bench.path()).expect("build");
    match emit::dev_script_x86(&info, &policy, bench.path()) {
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
    use_fixture_corpus();
    if Command::new("git").arg("--version").output().is_err() {
        eprintln!("skipping: `git` is not on PATH; the freeze verifier shells out to it");
        return;
    }
    let bench = fixture_bench("freeze-bench");
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
    std::os::unix::fs::symlink(bench.path().join(".bench"), root.join(".bench")).unwrap();

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
    let want = fs::read(reference_answer("x86", "bignum_add_p25519")).unwrap();
    assert_identical(task, text.as_bytes(), &want);
}

/// XVI.A1: committed inputs must not rot away from the pin. The fixture must
/// always hold exactly the 27 covered entries; when the live bench is present,
/// each entry must be **byte-identical** to its committed copy, and so must the
/// object and Cadical fixtures. When the bench is absent those comparisons print
/// why they could not run — the live pin is genuinely unavailable on that
/// machine — but the fixture's own shape is still asserted.
#[test]
fn fixture_inputs_match_the_live_bench_when_present() {
    let fixture = require_problems_fixture();
    let fixture_bytes = fs::read(&fixture).expect("fixture corpus");
    let fixture_members = json::top_level_members(&fixture_bytes).expect("parse fixture corpus");
    assert_eq!(
        fixture_members.len(),
        corpus::tasks().count(),
        "the fixture must hold exactly the covered tasks"
    );
    let fixture_keys: BTreeSet<&str> = fixture_members
        .iter()
        .map(|(key, _)| key.as_str())
        .collect();
    let expected_keys: BTreeSet<&str> = corpus::tasks().collect();
    assert_eq!(
        fixture_keys, expected_keys,
        "fixture key set differs from the covered task set"
    );

    let live = repo().join(".bench/problems.json");
    if live.is_file() {
        let live_bytes = fs::read(&live).expect("live corpus");
        let live_members = json::top_level_members(&live_bytes).expect("parse live corpus");
        let live_by_key: BTreeMap<&str, &[u8]> = live_members
            .iter()
            .map(|(key, value)| (key.as_str(), *value))
            .collect();
        for (key, value) in &fixture_members {
            let want = live_by_key
                .get(key.as_str())
                .unwrap_or_else(|| panic!("live corpus has no entry for {key}"));
            assert_identical(key, value, want);
        }
    } else {
        eprintln!(
            "skipping live-corpus agreement: {} is absent (bench-free tree)",
            live.display()
        );
    }

    let bench = corpus::bench_fixture(&repo());
    let live_s2n = repo().join(".bench/s2n-bignum");
    if live_s2n.is_dir() {
        compare_tree(&bench.join("s2n-bignum"), &live_s2n, Path::new(""));
    } else {
        eprintln!(
            "skipping object-fixture agreement: {} is absent (bench-free tree)",
            live_s2n.display()
        );
    }

    let live_cadical = repo().join(".bench/hol-light/Cadical");
    if live_cadical.is_dir() {
        compare_tree(
            &bench.join("hol-light/Cadical"),
            &live_cadical,
            Path::new(""),
        );
    } else {
        eprintln!(
            "skipping Cadical-fixture agreement: {} is absent (bench-free tree)",
            live_cadical.display()
        );
    }
}

/// Every file under `fixture` must exist under `live` with identical bytes.
fn compare_tree(fixture: &Path, live: &Path, rel: &Path) {
    for entry in fs::read_dir(fixture).unwrap_or_else(|e| panic!("{}: {e}", fixture.display())) {
        let entry = entry.unwrap();
        let path = entry.path();
        let child = rel.join(entry.file_name());
        if path.is_dir() {
            compare_tree(&path, live, &child);
            continue;
        }
        let other = live.join(&child);
        let got = fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let want = fs::read(&other).unwrap_or_else(|e| panic!("{}: {e}", other.display()));
        assert_eq!(
            got,
            want,
            "fixture {} is not byte-identical to {}",
            path.display(),
            other.display()
        );
    }
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
    let phase = emit::phase_tactic(tactic, "test.task");
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
    use_fixture_corpus();
    let bench = fixture_bench("phase");
    let policy = spent_policy();
    let task = "arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT";
    let parsed = guard::parse_task_id(task).expect("declared task id");
    let info = emit::build_arm(&parsed.op, task, &policy, bench.path()).expect("build");
    let normal = emit::inlined_answer(&info.tactic, bench.path()).expect("normal answer");
    let phase =
        emit::inlined_answer_measured(&emit::phase_tactic(&info.tactic, task), bench.path())
            .expect("phase answer");
    assert_ne!(normal, phase);
    assert_eq!(
        answer_tactic(&phase),
        emit::phase_tactic(&info.tactic, task)
    );
    let stripped = emit::strip_phase_marks(&answer_tactic(&phase)).expect("strip");
    assert_eq!(stripped, info.tactic);
    assert_eq!(answer_tactic(&normal), info.tactic);
    // IX.66 refines the IX.65 contract. IX.65 kept the two libraries
    // byte-identical; IX.66's phase copy carries the boundary markers inside the
    // library functions, so the libraries differ, and the difference must be the
    // marker phrases alone.
    assert!(normal.len() < phase.len());
}

/// IX.69 (XV.B) + XVI.D: the slice is tried before the full set, and only a
/// **mathematical** failure falls back. Resource exhaustion is classified,
/// reported and re-raised; it must not retry the larger full set. The measured
/// controls live in `research/receipts/ix69-slicing.md`; the taxonomy in
/// `research/receipts/ix73-failure-taxonomy-and-cap-scope.md`. This pins the
/// code shape so the fallback cannot be removed or widened without a failure.
/// It also pins that the phase prefix carries the task id the `SLICE` lines
/// name.
#[test]
fn ix69_slice_falls_back_only_on_a_mathematical_failure() {
    let src = fs::read_to_string(repo().join("tools/bridge.ml")).expect("tools/bridge.ml");
    // The tactic: the sliced attempt is passed before the full-set fallback.
    let body = src
        .split("let BRIDGE_CSAT_TAC : tactic =")
        .nth(1)
        .expect("BRIDGE_CSAT_TAC");
    let slice = body
        .find("~attempt:(fun () -> close kept)")
        .expect("the sliced close");
    let retry = body
        .find("~fallback:(fun () -> close hths)")
        .expect("the full-set retry");
    assert!(slice < retry, "the slice must be attempted first");
    // The policy: only the mathematical branch reports and then retries.
    let policy = src
        .split("let bridge_slice_retry")
        .nth(1)
        .expect("bridge_slice_retry");
    let report = policy
        .find("reason=mathematical")
        .expect("the mathematical fallback report");
    let fallback = policy.find("fallback ()").expect("the fallback call");
    assert!(
        report < fallback,
        "the full set must be retried only after the fallback report"
    );
    assert_eq!(
        src.matches("SLICE fallback=%s reason=mathematical kept=%d/%d")
            .count(),
        1
    );
    assert_eq!(src.matches("CSAT unsupported=%s kept=%d/%d").count(), 1);
    assert_eq!(src.matches("CSAT capped=%s limit=%s kept=%d/%d").count(), 1);
    assert_eq!(src.matches("CSAT internal=%s").count(), 1);
    assert_eq!(src.matches("ACCEPT_TAC th' (asl,w)").count(), 1);
    // The two resource exceptions that must never reach the fallback.  IX.75
    // replaces the polymorphic-variant representation, which the camlp5r
    // preprocessor cannot read, with a portable string-tagged triple.
    assert!(src.contains("| Out_of_memory -> (\"resource\", \"memory\""));
    assert!(src.contains("| Stack_overflow -> (\"resource\", \"stack\""));

    let tactic = "BRIDGE_ADD_REF_TAC EXPAND P";
    let task = "arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT";
    let phase = emit::phase_tactic(tactic, task);
    assert!(phase.contains(&format!("bridge_slice_task := {task:?};")));
    assert_eq!(emit::strip_phase_marks(&phase).as_deref(), Some(tactic));
    // A phase tactic without the task binding is refused, not silently stripped.
    let old_shape = "(csat_mark_reader := inference_count;\ncsat_mark_enabled := true;\ns0)";
    assert_eq!(emit::strip_phase_marks(old_shape), None);
}

/// IX.66: the instrumentation is additive only, and the normal answer is
/// recoverable from the phase copy. Three halves, one property:
///
/// 1. `deinstrument (instrument src) == src` on **committed synthetic
///    fixtures** (`tests/fixtures/emit/`), and assembling the answer from the
///    deinstrumented phase copy reproduces the normal answer byte for byte.
///    No `.bench` input is needed (XV.A1): the property is a text round trip on
///    the two library texts, not a property of the HOL Light sources.
/// 2. The same round trip on the **tracked** `tools/csat_lib.ml` and
///    `tools/bridge.ml`, which are in git and therefore always present. This is
///    what fails if an anchor drifts in the real library.
/// 3. The **real inlined set**, which includes the Cadical sources. It reads
///    the live `.bench/hol-light/Cadical/` when present and the committed
///    `tests/fixtures/bench/hol-light/Cadical/` copy otherwise, so it runs
///    offline rather than skipping (the two are pinned together by the
///    live-agreement test).
///
/// `--mode answer` never calls the instrumenter at all.
#[test]
fn ix66_markers_are_additive_and_the_normal_answer_is_recovered() {
    let tactic = "MAP_EVERY X_GEN_TAC [`z:int64`] THEN\nBRIDGE_ADD_REF_TAC EXPAND P";

    // 1. The synthetic fixtures: always runnable, `.bench` or not.
    let fixture = |name: &str| -> String {
        let path = repo().join("tests/fixtures/emit").join(name);
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
    };
    let plain: Vec<(String, String)> = vec![
        ("cadical_stub.ml".to_string(), fixture("cadical_stub.ml")),
        ("csat_lib_min.ml".to_string(), fixture("csat_lib_min.ml")),
        ("bridge_min.ml".to_string(), fixture("bridge_min.ml")),
    ];
    let phase = instrument_sources(&plain);
    let measured = emit::inlined_answer_from(tactic, &as_refs(&phase));
    let normal = emit::inlined_answer_from(tactic, &as_refs(&plain));
    assert!(
        normal.len() < measured.len(),
        "the fixture phase copy carries no markers"
    );
    let n_normal = normal.matches("PHASE_").count();
    let n_measured = measured.matches("PHASE_").count();
    assert!(
        n_measured > n_normal,
        "the fixture phase copy carries no markers: {n_measured} vs {n_normal}"
    );
    // Deinstrumenting the phase copy and re-assembling restores the normal
    // answer byte for byte: nothing but markers is in the measured text.
    assert_eq!(deinstrument_answer(tactic, &phase), normal);

    // 2. The tracked library texts (in git, never in `.bench`).
    let tracked: Vec<(String, String)> = emit::ANSWER_SOURCES
        .iter()
        .filter(|rel| rel.starts_with("tools/"))
        .map(|rel| (rel.to_string(), read_repo_file(rel)))
        .collect();
    let tracked_phase = instrument_sources(&tracked);
    assert_ne!(tracked_phase, tracked, "no tracked library was marked");
    assert_eq!(
        deinstrument_answer(tactic, &tracked_phase),
        emit::inlined_answer_from(tactic, &as_refs(&tracked))
    );

    // 3. The real inlined set (the Cadical sources plus the tracked
    //    libraries). The live bench is used when present; otherwise the
    //    committed inputs under `tests/fixtures/bench` stand in — they are
    //    pinned to the live bytes by
    //    `fixture_inputs_match_the_live_bench_when_present`.
    let cadical = repo().join(".bench/hol-light/Cadical");
    let stand_in = if cadical.is_dir() {
        None
    } else {
        Some(fixture_bench("ix66-real"))
    };
    let root = stand_in
        .as_ref()
        .map_or_else(repo, |tmp| tmp.path().to_path_buf());
    let real: Vec<(String, String)> = emit::ANSWER_SOURCES
        .iter()
        .map(|rel| (rel.to_string(), read_source(&root, rel)))
        .collect();
    let real_phase = instrument_sources(&real);
    let measured = emit::inlined_answer_from(tactic, &as_refs(&real_phase));
    let normal = emit::inlined_answer_from(tactic, &as_refs(&real));
    assert!(normal.len() < measured.len());
    assert_eq!(deinstrument_answer(tactic, &real_phase), normal);
}

/// Read a repository-relative source file.
fn read_repo_file(rel: &str) -> String {
    read_source(&repo(), rel)
}

/// Read a source file relative to an arbitrary repository root (the live tree or
/// a materialised fixture bench).
fn read_source(root: &Path, rel: &str) -> String {
    let path = root.join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Which marker table applies to a source, by file name stem: the tracked
/// `tools/csat_lib.ml` / `tools/bridge.ml` and the `*_min.ml` fixtures.
fn library_kind(rel: &str) -> Option<&'static str> {
    let name = rel.rsplit('/').next().unwrap_or(rel);
    if name.starts_with("csat_lib") {
        Some("csat_lib")
    } else if name.starts_with("bridge") {
        Some("bridge")
    } else {
        None
    }
}

/// Apply the emitter's marker table to whichever sources it names.
fn instrument_sources(sources: &[(String, String)]) -> Vec<(String, String)> {
    sources
        .iter()
        .map(|(rel, src)| {
            let marked = match library_kind(rel) {
                Some("csat_lib") => round_trip("csat_lib", &emit::instrument_csat_lib_markers, src),
                Some("bridge") => round_trip("bridge", &emit::instrument_bridge_markers, src),
                _ => src.clone(),
            };
            (rel.clone(), marked)
        })
        .collect()
}

/// Apply one instrumenter and prove `deinstrument ∘ instrument = id`, returning
/// the marked (phase) text.
fn round_trip(
    which: &str,
    inst: &dyn Fn(&str) -> Result<String, emit::EmitError>,
    src: &str,
) -> String {
    let deinst: fn(&str) -> Result<String, emit::EmitError> = match which {
        "csat_lib" => emit::deinstrument_csat_lib,
        "bridge" => emit::deinstrument_bridge,
        other => panic!("unknown library {other}"),
    };
    let marked = inst(src).unwrap_or_else(|e| panic!("{which}: {e}"));
    assert_ne!(marked, src, "{which}: instrumentation did nothing");
    let back = deinst(&marked).unwrap_or_else(|e| panic!("{which}: {e}"));
    assert_eq!(&back, src, "{which}: instrumentation is not additive only");
    marked
}

/// Deinstrument every library in a phase source list and assemble the answer.
fn deinstrument_answer(tactic: &str, sources: &[(String, String)]) -> String {
    let tracked: Vec<(String, String)> = sources
        .iter()
        .map(|(rel, src)| {
            let back = match library_kind(rel) {
                Some("csat_lib") => emit::deinstrument_csat_lib(src),
                Some("bridge") => emit::deinstrument_bridge(src),
                _ => Ok(src.clone()),
            };
            (rel.clone(), back.unwrap_or_else(|e| panic!("{rel}: {e}")))
        })
        .collect();
    emit::inlined_answer_from(tactic, &as_refs(&tracked))
}

fn as_refs(v: &[(String, String)]) -> Vec<(&str, &str)> {
    v.iter().map(|(r, t)| (r.as_str(), t.as_str())).collect()
}

/// IX.70: the algebraic close is opt-in and gated to exactly one task. This is
/// a pure string mapping (the driver's `TEKMERION_ALG_CLOSE` read is separate),
/// so it does not depend on process environment or test ordering.
#[test]
fn ix70_algebraic_close_is_opt_in_and_one_task_only() {
    let task = "arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT";
    let sat = emit::post_text_for(Some(task), false, "add", "s4", "p");
    assert!(sat.contains("BRIDGE_ADD_REF_TAC "));
    assert!(!sat.contains("BRIDGE_ADD_REF_ALG_TAC"));
    let alg = emit::post_text_for(Some(task), true, "add", "s4", "p");
    assert!(alg.contains("BRIDGE_ADD_REF_ALG_TAC "));
    assert!(!alg.contains("BRIDGE_ADD_REF_TAC "));
    // The switch cannot select the algebraic close for any other task.
    let other = "x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT";
    let o = emit::post_text_for(Some(other), true, "add", "s4", "p");
    assert!(o.contains("BRIDGE_ADD_REF_TAC "));
    assert!(!o.contains("BRIDGE_ADD_REF_ALG_TAC"));
    // Unknown/None callers always get the SAT path.
    let none = emit::post_text_for(None, true, "add", "s4", "p");
    assert!(none.contains("BRIDGE_ADD_REF_TAC "));
}
