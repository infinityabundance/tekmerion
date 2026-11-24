//! Integration tests for the freeze closure (`src/tools/freeze.rs`, XIII.C).
//!
//! Every test works on a throw-away git repository; nothing here touches the
//! real tree's tag or `.bench/`. The runtime layer is exercised against fake
//! binaries under the fixture, so no solver or HOL Light build is required.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use tekmerion::tools::freeze::{self, Closure, Entry, RuntimePaths};
use tekmerion::tools::guard::Refused;

static SEQ: AtomicU32 = AtomicU32::new(0);

const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
const TAG: &str = "freeze-fixture-v5";

struct Tmp(PathBuf);

impl Tmp {
    fn new(tag: &str) -> Tmp {
        let n = SEQ.fetch_add(1, Ordering::SeqCst);
        let p =
            std::env::temp_dir().join(format!("tek-freeze-{}-{}-{}", std::process::id(), tag, n));
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

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap_or_else(|e| panic!("cannot run git: {e}"));
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn commit(dir: &Path, message: &str) {
    git(dir, &["add", "-A"]);
    git(
        dir,
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

fn tag(dir: &Path, name: &str, commit: &str) {
    git(
        dir,
        &[
            "-c",
            "user.name=infinityabundance",
            "-c",
            "user.email=255699974+infinityabundance@users.noreply.github.com",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "tag.gpgsign=false",
            "tag",
            "-f",
            "-a",
            "-m",
            "freeze closure fixture",
            name,
            commit,
        ],
    );
}

fn set_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).unwrap();
}

struct Fixture {
    tmp: Tmp,
    repo: PathBuf,
}

impl Fixture {
    fn new(tag_name: &str) -> Fixture {
        let tmp = Tmp::new("repo");
        let repo = tmp.path().join("repo");
        for dir in ["src/tools", "tools/hol_tests", "runtime", "receipts"] {
            fs::create_dir_all(repo.join(dir)).unwrap();
        }
        fs::write(repo.join("src/main.rs"), "fn main() {}\n").unwrap();
        fs::write(repo.join("src/tools/emit.rs"), "pub fn emit() {}\n").unwrap();
        fs::write(repo.join("Cargo.toml"), "[package]\nname = \"fixture\"\n").unwrap();
        fs::write(repo.join("Cargo.lock"), "# lock\n").unwrap();
        fs::write(repo.join("tools/bridge.ml"), "let x = 1\n").unwrap();
        fs::write(repo.join("tools/hol_tests/t.ml"), "let y = 2\n").unwrap();
        fs::write(repo.join("tools/selftest.sh"), "#!/bin/sh\n").unwrap();
        fs::write(repo.join("tools/run_traces.sh"), "#!/bin/sh\n").unwrap();
        fs::write(repo.join("tools/not_closure.py"), "# excluded\n").unwrap();
        fs::write(repo.join("runtime/emit"), b"\x7fELF not really\n").unwrap();
        fs::write(repo.join("runtime/problems.json"), "{}\n").unwrap();
        fs::write(repo.join("runtime/cadical"), "#!/bin/sh\necho 2.1.2\n").unwrap();
        set_executable(&repo.join("runtime/cadical"));

        // The HOL Light copy is its own git repository so the pin is readable.
        let hol = tmp.path().join("hol");
        fs::create_dir_all(&hol).unwrap();
        fs::write(hol.join("hol_lib.cmxa"), b"cmxa fixture\n").unwrap();
        git(&hol, &["init", "-q"]);
        commit(&hol, "hol");

        git(&repo, &["init", "-q"]);
        commit(&repo, "freeze");
        let commit = git(&repo, &["rev-parse", "HEAD"]);
        tag(&repo, tag_name, &commit);
        Fixture { tmp, repo }
    }

    fn runtime_paths(&self) -> RuntimePaths {
        RuntimePaths {
            executable: PathBuf::from("runtime/emit"),
            hol_light: self.tmp.path().join("hol"),
            cadical: self.repo.join("runtime/cadical"),
            benchmark_inputs: PathBuf::from("runtime/problems.json"),
            rustc: "rustc".to_string(),
        }
    }

    fn closure(&self, tag_name: &str) -> Closure {
        freeze::build(&self.repo, tag_name, Some(&self.runtime_paths())).expect("closure")
    }
}

fn expect_refused<T>(result: Result<T, Refused>, needle: &str) {
    match result {
        Err(e) => assert!(
            e.to_string().contains(needle),
            "unexpected refusal message: {e} (wanted {needle:?})"
        ),
        Ok(_) => panic!("expected a refusal containing {needle:?}"),
    }
}

fn hex(fill: char) -> String {
    std::iter::repeat(fill).take(64).collect()
}

fn entry_json(path: &str, digest: &str) -> String {
    format!("{{\"path\":\"{path}\",\"sha256\":\"{digest}\"}}")
}

/// A minimal, self-consistent source-only manifest.
fn doc_with(source_json: &str, digest: &str, extra: &str) -> String {
    format!(
        "{{\"schema\":\"{}\",\"tag\":\"{TAG}\",\"repo_commit\":\"{COMMIT}\",\"created_at\":7,\
         \"source\":{source_json},\"source_digest\":\"{digest}\",\"runtime\":null{extra}}}",
        freeze::SCHEMA
    )
}

fn base_doc() -> String {
    let source = [("src/a.rs", hex('a')), ("src/b.rs", hex('b'))];
    let entries: Vec<Entry> = source
        .iter()
        .map(|(path, digest)| Entry {
            path: (*path).to_string(),
            sha256: (*digest).to_string(),
        })
        .collect();
    let source_json = source
        .iter()
        .map(|(path, digest)| entry_json(path, digest))
        .collect::<Vec<_>>()
        .join(",");
    doc_with(
        &format!("[{source_json}]"),
        &freeze::digest_of(&entries),
        "",
    )
}

// ---------------------------------------------------------------------------
// 1. The source closure is what a fresh checkout can recompute
// ---------------------------------------------------------------------------

#[test]
fn source_closure_covers_the_declared_roots() {
    let fixture = Fixture::new(TAG);
    let closure = freeze::build(&fixture.repo, TAG, None).expect("source closure");

    let paths: Vec<&str> = closure.source.iter().map(|e| e.path.as_str()).collect();
    for wanted in [
        "src/main.rs",
        "src/tools/emit.rs",
        "Cargo.toml",
        "Cargo.lock",
        "tools/bridge.ml",
        "tools/hol_tests/t.ml",
        "tools/selftest.sh",
        "tools/run_traces.sh",
    ] {
        assert!(paths.contains(&wanted), "closure is missing {wanted}");
    }
    assert!(!paths.contains(&"tools/not_closure.py"));
    assert_eq!(paths.len(), 8, "unexpected closure members: {paths:?}");
    let mut sorted = paths.clone();
    sorted.sort();
    assert_eq!(paths, sorted, "source entries must be sorted");

    assert_eq!(closure.source_digest, freeze::digest_of(&closure.source));
    assert_eq!(
        closure.repo_commit,
        git(&fixture.repo, &["rev-parse", "HEAD"])
    );
    assert!(closure.runtime.is_none());

    // Verification from the tree agrees.
    freeze::verify_source(&fixture.repo, &closure).expect("source verifies");

    // The rendering round-trips through the strict parser.
    let text = freeze::render(&closure);
    let parsed = freeze::parse(text.as_bytes(), Path::new("m.json")).expect("parse");
    assert_eq!(parsed, closure);
    assert_eq!(freeze::render(&parsed), text);
}

#[test]
fn source_refusals() {
    let fixture = Fixture::new(TAG);
    let closure = freeze::build(&fixture.repo, TAG, None).expect("source closure");

    // A modified file: the first differing path is named with both digests.
    let emit = fixture.repo.join("src/tools/emit.rs");
    fs::write(&emit, "pub fn emit() { /* one byte */ }\n").unwrap();
    match freeze::verify_source(&fixture.repo, &closure) {
        Err(e) => {
            let text = e.to_string();
            assert!(text.contains("src/tools/emit.rs"), "message: {text}");
            assert!(text.contains("differs"), "message: {text}");
            let sealed = closure
                .source
                .iter()
                .find(|entry| entry.path == "src/tools/emit.rs")
                .unwrap();
            assert!(text.contains(&sealed.sha256), "message: {text}");
        }
        Ok(()) => panic!("a modified emitter source was accepted"),
    }
    fs::write(&emit, "pub fn emit() {}\n").unwrap();
    freeze::verify_source(&fixture.repo, &closure).expect("restored");

    // A sealed file that is missing from the tree.
    let main = fixture.repo.join("src/main.rs");
    let saved = fs::read(&main).unwrap();
    fs::remove_file(&main).unwrap();
    expect_refused(
        freeze::verify_source(&fixture.repo, &closure),
        "src/main.rs is sealed but missing",
    );
    fs::write(&main, &saved).unwrap();

    // A file present in the tree but absent from the seal.
    fs::write(fixture.repo.join("src/sneaked_in.rs"), "// new\n").unwrap();
    expect_refused(
        freeze::verify_source(&fixture.repo, &closure),
        "src/sneaked_in.rs is present but not in the sealed closure",
    );
    fs::remove_file(fixture.repo.join("src/sneaked_in.rs")).unwrap();
    freeze::verify_source(&fixture.repo, &closure).expect("restored again");
}

#[test]
fn source_walk_refusals() {
    let tmp = Tmp::new("walk");
    fs::create_dir_all(tmp.path().join("tools")).unwrap();
    expect_refused(freeze::source_entries(tmp.path()), "src is not a directory");

    let tools_missing = Tmp::new("walk-tools");
    fs::create_dir_all(tools_missing.path().join("src")).unwrap();
    expect_refused(
        freeze::source_entries(tools_missing.path()),
        "tools is not a directory",
    );

    let fixture = Fixture::new(TAG);
    std::os::unix::fs::symlink(
        fixture.repo.join("src/main.rs"),
        fixture.repo.join("src/linked.rs"),
    )
    .unwrap();
    expect_refused(freeze::source_entries(&fixture.repo), "contains a symlink");
}

// ---------------------------------------------------------------------------
// 2. The runtime layer is machine-specific and checked item by item
// ---------------------------------------------------------------------------

#[test]
fn runtime_closure_verifies_and_refuses_every_difference() {
    let fixture = Fixture::new(TAG);
    let closure = fixture.closure(TAG);
    let rt = closure.runtime.clone().expect("runtime layer");

    assert_eq!(rt.executable.path, "runtime/emit");
    assert_eq!(rt.cadical.version, "2.1.2");
    assert_eq!(rt.ocaml_bridge.len(), 1);
    assert_eq!(rt.ocaml_bridge[0].path, "tools/bridge.ml");
    assert!(rt.rustc.starts_with("rustc "));
    freeze::verify_runtime(&fixture.repo, &closure).expect("runtime verifies");
    assert_eq!(closure.runtime_entries(), 5);

    let mutate = |f: &dyn Fn(&mut tekmerion::tools::freeze::RuntimeBlock)| {
        let mut bad = closure.clone();
        f(bad.runtime.as_mut().unwrap());
        bad
    };

    expect_refused(
        freeze::verify_runtime(&fixture.repo, &mutate(&|r| r.executable.sha256 = hex('0'))),
        "runtime executable digest mismatch for runtime/emit",
    );
    expect_refused(
        freeze::verify_runtime(
            &fixture.repo,
            &mutate(&|r| r.executable.path = "runtime/absent".into()),
        ),
        "cannot read",
    );
    expect_refused(
        freeze::verify_runtime(&fixture.repo, &mutate(&|r| r.rustc = "rustc 0.0.0".into())),
        "runtime rustc mismatch",
    );
    expect_refused(
        freeze::verify_runtime(&fixture.repo, &mutate(&|r| r.cargo_lock_sha256 = hex('0'))),
        "runtime Cargo.lock digest mismatch",
    );
    expect_refused(
        freeze::verify_runtime(
            &fixture.repo,
            &mutate(&|r| r.hol_light.pin = "0000000".into()),
        ),
        "runtime HOL Light pin mismatch",
    );
    expect_refused(
        freeze::verify_runtime(
            &fixture.repo,
            &mutate(&|r| r.hol_light.hol_lib_cmxa_sha256 = hex('0')),
        ),
        "hol_lib.cmxa digest mismatch",
    );
    expect_refused(
        freeze::verify_runtime(&fixture.repo, &mutate(&|r| r.cadical.sha256 = hex('0'))),
        "runtime cadical digest mismatch",
    );
    expect_refused(
        freeze::verify_runtime(
            &fixture.repo,
            &mutate(&|r| r.cadical.version = "9.9.9".into()),
        ),
        "runtime cadical version mismatch",
    );
    expect_refused(
        freeze::verify_runtime(
            &fixture.repo,
            &mutate(&|r| r.ocaml_bridge[0].sha256 = hex('0')),
        ),
        "runtime OCaml bridge digest mismatch for tools/bridge.ml",
    );
    expect_refused(
        freeze::verify_runtime(
            &fixture.repo,
            &mutate(&|r| r.benchmark_inputs.problems_json_sha256 = hex('0')),
        ),
        "runtime benchmark input digest mismatch",
    );

    let mut source_only = closure.clone();
    source_only.runtime = None;
    expect_refused(
        freeze::verify_runtime(&fixture.repo, &source_only),
        "no runtime layer",
    );
}

#[test]
fn build_refusals() {
    let fixture = Fixture::new(TAG);
    expect_refused(
        freeze::build(&fixture.repo, "bad tag", None),
        "not a valid tag name",
    );

    // A directory that is not a git repository cannot name a sealed commit.
    let empty = Tmp::new("not-a-repo");
    expect_refused(
        freeze::build(empty.path(), TAG, None),
        "git rev-parse HEAD failed",
    );

    // A cadical that cannot report its version is a refusal, not a guess.
    let cadical = fixture.repo.join("runtime/cadical");
    fs::write(&cadical, "#!/bin/sh\nexit 1\n").unwrap();
    set_executable(&cadical);
    expect_refused(
        freeze::build(&fixture.repo, TAG, Some(&fixture.runtime_paths())),
        "failed with exit",
    );
}

// ---------------------------------------------------------------------------
// 3. Strict parsing
// ---------------------------------------------------------------------------

#[test]
fn parse_round_trip_and_refusals() {
    let base = base_doc();
    let parsed = freeze::parse(base.as_bytes(), Path::new("m.json")).expect("base manifest");
    assert_eq!(parsed.source.len(), 2);
    assert_eq!(parsed.created_at, 7);
    assert_eq!(parsed.tag, TAG);
    assert!(freeze::render(&parsed).contains(&parsed.source_digest));

    expect_refused(
        freeze::parse(b"[]", Path::new("m.json")),
        "must be a JSON object",
    );
    expect_refused(
        freeze::parse(
            base.replace(freeze::SCHEMA, "tekmerion-freeze-closure/999")
                .as_bytes(),
            Path::new("m.json"),
        ),
        "unsupported freeze schema",
    );
    expect_refused(
        freeze::parse(
            base.replace(&format!("\"tag\":\"{TAG}\""), "\"tag\":\"bad tag\"")
                .as_bytes(),
            Path::new("m.json"),
        ),
        "not a valid tag name",
    );
    expect_refused(
        freeze::parse(
            base.replace(COMMIT, "not-a-commit").as_bytes(),
            Path::new("m.json"),
        ),
        "repo_commit is not a 40-character",
    );
    expect_refused(
        freeze::parse(
            base.replace("\"created_at\":7", "\"created_at\":1.5")
                .as_bytes(),
            Path::new("m.json"),
        ),
        "created_at must be a non-negative integer",
    );
    expect_refused(
        freeze::parse(
            base.replace(&format!("\"tag\":\"{TAG}\""), "\"tag\":5")
                .as_bytes(),
            Path::new("m.json"),
        ),
        "tag must be a string",
    );
    expect_refused(
        freeze::parse(
            base.replace(&format!("\"tag\":\"{TAG}\""), "\"tag\":\"\"")
                .as_bytes(),
            Path::new("m.json"),
        ),
        "tag must not be empty",
    );

    // `source` missing, then not an array.
    let without_source = format!(
        "{{\"schema\":\"{}\",\"tag\":\"{TAG}\",\"repo_commit\":\"{COMMIT}\",\"created_at\":7,\
         \"source_digest\":\"{}\",\"runtime\":null}}",
        freeze::SCHEMA,
        hex('a')
    );
    expect_refused(
        freeze::parse(without_source.as_bytes(), Path::new("m.json")),
        "source is required",
    );
    let source_object = doc_with("{}", &hex('a'), "");
    expect_refused(
        freeze::parse(source_object.as_bytes(), Path::new("m.json")),
        "must be a JSON array",
    );

    // Unknown key, bad path, bad digest.
    let unknown = doc_with(
        &format!("[{}]", entry_json("src/a.rs", &hex('a'))),
        &freeze::digest_of(&[Entry {
            path: "src/a.rs".into(),
            sha256: hex('a'),
        }]),
        ",\"bogus\":1",
    );
    expect_refused(
        freeze::parse(unknown.as_bytes(), Path::new("m.json")),
        "unknown key",
    );
    let traversal = doc_with(
        &format!("[{}]", entry_json("../src/a.rs", &hex('a'))),
        &hex('a'),
        "",
    );
    expect_refused(
        freeze::parse(traversal.as_bytes(), Path::new("m.json")),
        "not a clean repo-relative path",
    );
    expect_refused(
        freeze::parse(
            base.replace(
                &format!(
                    "\"source_digest\":\"{}\"",
                    freeze::digest_of(&[
                        Entry {
                            path: "src/a.rs".into(),
                            sha256: hex('a')
                        },
                        Entry {
                            path: "src/b.rs".into(),
                            sha256: hex('b')
                        },
                    ])
                ),
                "\"source_digest\":\"zz\"",
            )
            .as_bytes(),
            Path::new("m.json"),
        ),
        "is not a 64-character lowercase hex sha256",
    );

    // Empty and unsorted source lists.
    expect_refused(
        freeze::parse(
            doc_with("[]", &freeze::digest_of(&[]), "").as_bytes(),
            Path::new("m.json"),
        ),
        "source closure is empty",
    );
    let unsorted = [
        Entry {
            path: "src/b.rs".into(),
            sha256: hex('b'),
        },
        Entry {
            path: "src/a.rs".into(),
            sha256: hex('a'),
        },
    ];
    let unsorted_json = unsorted
        .iter()
        .map(|e| entry_json(&e.path, &e.sha256))
        .collect::<Vec<_>>()
        .join(",");
    expect_refused(
        freeze::parse(
            doc_with(
                &format!("[{unsorted_json}]"),
                &freeze::digest_of(&unsorted),
                "",
            )
            .as_bytes(),
            Path::new("m.json"),
        ),
        "not strictly sorted",
    );

    // A digest that does not describe the list it is stored with.
    expect_refused(
        freeze::parse(
            doc_with(
                &format!("[{}]", entry_json("src/a.rs", &hex('a'))),
                &hex('0'),
                "",
            )
            .as_bytes(),
            Path::new("m.json"),
        ),
        "does not match the manifest's own source list",
    );

    // A runtime that is present but malformed, and a runtime path that escapes.
    expect_refused(
        freeze::parse(
            base.replace("\"runtime\":null", "\"runtime\":\"x\"")
                .as_bytes(),
            Path::new("m.json"),
        ),
        "freeze runtime must be a JSON object",
    );

    let mut with_runtime = base.replace(
        "\"runtime\":null",
        &format!(
            "\"runtime\":{{\"executable\":{},\"rustc\":\"rustc 1.0.0\",\"cargo_lock_sha256\":\"{}\",\
             \"hol_light\":{{\"path\":\"../hol\",\"pin\":\"abc1234\",\"hol_lib_cmxa_sha256\":\"{}\"}},\
             \"cadical\":{{\"path\":\"cadical\",\"sha256\":\"{}\",\"version\":\"2.1.2\"}},\
             \"ocaml_bridge\":[],\"benchmark_inputs\":{{\"path\":\"problems.json\",\"problems_json_sha256\":\"{}\"}}}}",
            entry_json("target/release/emit", &hex('e')),
            hex('c'),
            hex('d'),
            hex('f'),
            hex('1'),
        ),
    );
    expect_refused(
        freeze::parse(with_runtime.as_bytes(), Path::new("m.json")),
        "path is empty or contains '..'",
    );
    with_runtime = with_runtime.replace("../hol", "hol");
    let parsed = freeze::parse(with_runtime.as_bytes(), Path::new("m.json")).expect("runtime");
    assert_eq!(parsed.runtime_entries(), 4);
}

// ---------------------------------------------------------------------------
// 4. Tag binding: the tag's tree is the sealed tree, not a later one
// ---------------------------------------------------------------------------

fn commit_manifest(fixture: &Fixture, path: &str, text: &str) -> String {
    let full = fixture.repo.join(path);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&full, text).unwrap();
    commit(&fixture.repo, "manifest");
    let head = git(&fixture.repo, &["rev-parse", "HEAD"]);
    tag(&fixture.repo, TAG, &head);
    head
}

#[test]
fn check_tag_binds_the_sealed_tree() {
    let fixture = Fixture::new(TAG);
    let closure = fixture.closure(TAG);
    let sealed = closure.repo_commit.clone();
    let text = freeze::render(&closure);
    let tag_commit = commit_manifest(&fixture, "receipts/closure.json", &text);

    assert_eq!(
        freeze::check_tag(&fixture.repo, &closure, "receipts/closure.json").unwrap(),
        tag_commit
    );
    assert_ne!(
        sealed, tag_commit,
        "fixture must exercise the ancestor path"
    );

    // A clean later commit with a modified emitter must be refused even though
    // the committed manifest is untouched.
    fs::write(
        fixture.repo.join("src/tools/emit.rs"),
        "pub fn emit() { /* modified */ }\n",
    )
    .unwrap();
    commit(&fixture.repo, "modified emitter");
    let moved = git(&fixture.repo, &["rev-parse", "HEAD"]);
    tag(&fixture.repo, TAG, &moved);
    match freeze::check_tag(&fixture.repo, &closure, "receipts/closure.json") {
        Err(e) => {
            let text = e.to_string();
            assert!(text.contains("src/tools/emit.rs"), "message: {text}");
            assert!(text.contains("differs"), "message: {text}");
        }
        Ok(_) => panic!("a clean commit with a modified emitter was accepted"),
    }
}

#[test]
fn check_tag_refusals() {
    // A manifest committed at the tag with a different source_digest.
    let fixture = Fixture::new(TAG);
    let closure = fixture.closure(TAG);
    let other = [
        Entry {
            path: "src/a.rs".into(),
            sha256: hex('a'),
        },
        Entry {
            path: "src/b.rs".into(),
            sha256: hex('b'),
        },
    ];
    let other_json = other
        .iter()
        .map(|e| entry_json(&e.path, &e.sha256))
        .collect::<Vec<_>>()
        .join(",");
    let differing = doc_with(&format!("[{other_json}]"), &freeze::digest_of(&other), "");
    let tag_commit = commit_manifest(&fixture, "receipts/closure.json", &differing);
    expect_refused(
        freeze::check_tag(&fixture.repo, &closure, "receipts/closure.json"),
        "records source_digest",
    );
    assert!(
        freeze::check_tag(&fixture.repo, &closure, "receipts/closure.json")
            .unwrap_err()
            .to_string()
            .contains(&tag_commit)
    );

    // A manifest committed at the tag with a different repo_commit.
    let fixture = Fixture::new(TAG);
    let closure = fixture.closure(TAG);
    let text = freeze::render(&closure).replace(&closure.repo_commit, &"1".repeat(40));
    commit_manifest(&fixture, "receipts/closure.json", &text);
    expect_refused(
        freeze::check_tag(&fixture.repo, &closure, "receipts/closure.json"),
        "records repo_commit",
    );

    // A tag that is neither the sealed commit nor its descendant.
    let fixture = Fixture::new(TAG);
    let closure = fixture.closure(TAG);
    let text = freeze::render(&closure);
    commit_manifest(&fixture, "receipts/closure.json", &text);
    let tree = git(&fixture.repo, &["rev-parse", "HEAD^{tree}"]);
    let orphan = git(
        &fixture.repo,
        &[
            "-c",
            "user.name=infinityabundance",
            "-c",
            "user.email=255699974+infinityabundance@users.noreply.github.com",
            "-c",
            "commit.gpgsign=false",
            "commit-tree",
            &tree,
            "-m",
            "orphan",
        ],
    );
    tag(&fixture.repo, TAG, &orphan);
    expect_refused(
        freeze::check_tag(&fixture.repo, &closure, "receipts/closure.json"),
        "neither the sealed code commit",
    );

    // The tag's tree has no manifest at the recorded path.
    let fixture = Fixture::new(TAG);
    let closure = fixture.closure(TAG);
    expect_refused(
        freeze::check_tag(&fixture.repo, &closure, "receipts/absent.json"),
        "no manifest at",
    );

    // A tag that does not resolve.
    let fixture = Fixture::new(TAG);
    let mut closure = fixture.closure(TAG);
    closure.tag = "no-such-tag".into();
    expect_refused(
        freeze::check_tag(&fixture.repo, &closure, "receipts/closure.json"),
        "git rev-parse",
    );
}

// ---------------------------------------------------------------------------
// 5. The guard enforces the closure on a sealed run
// ---------------------------------------------------------------------------

#[test]
fn guard_enforces_the_closure_on_a_sealed_run() {
    use tekmerion::tools::guard;

    let fixture = Fixture::new(TAG);
    let closure = fixture.closure(TAG);
    let code_commit = closure.repo_commit.clone();
    assert_eq!(code_commit, git(&fixture.repo, &["rev-parse", "HEAD"]));

    let text = freeze::render(&closure);
    let tag_commit = commit_manifest(&fixture, "receipts/closure.json", &text);
    assert_ne!(code_commit, tag_commit);

    let manifest_path = fixture.tmp.path().join("sealed.json");
    fs::write(
        &manifest_path,
        format!(
            "{{\"freeze_commit\":\"{tag_commit}\",\"freeze_tag\":\"{TAG}\",\
             \"sealed_ids\":[\"x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT\"],\
             \"freeze_closure\":\"receipts/closure.json\"}}"
        ),
    )
    .unwrap();
    let manifest = guard::load_sealed_manifest(&manifest_path).expect("sealed manifest");
    guard::verify_freeze(&manifest, TAG, &fixture.repo)
        .expect("a closure-carrying freeze must verify");

    // A modified emitter in the working tree is refused by the same path.
    let emit = fixture.repo.join("src/tools/emit.rs");
    fs::write(&emit, "pub fn emit() { /* dirty */ }\n").unwrap();
    match guard::verify_freeze(&manifest, TAG, &fixture.repo) {
        Err(e) => assert!(
            e.to_string().contains("src/tools/emit.rs"),
            "unexpected message {e}"
        ),
        Ok(()) => panic!("a modified emitter was accepted under the closure"),
    }
}
