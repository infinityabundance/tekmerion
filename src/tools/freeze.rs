//! Freeze closure (XIII.C): bind the Rust producer, not just `tools/`.
//!
//! The pre-migration freeze verified only `git status/diff -- tools`, which
//! covered the Python emitters. Since the migration the producer lives in
//! `src/tools/` and `src/bin/` and is compiled into `target/release/*`, so that
//! scope does not bind what produced the proofs. This module records the
//! closure in two layers, kept separate on purpose:
//!
//! * **source** — portable. Every file under `src/**`, the cargo manifests,
//!   `rust-toolchain*` when present, every `tools/**/*.ml` and every
//!   `tools/*.sh`, one SHA-256 each, plus a canonical rollup `source_digest`
//!   over the sorted `path\0sha256\n` concatenation. A third party can
//!   recompute it from a checkout with no build and no machine state.
//! * **runtime** — machine-specific, recorded at run time. The compiled
//!   executable's digest, the `rustc` version, the frozen HOL Light pin and
//!   `hol_lib.cmxa` digest, the `cadical` path/digest/version, the OCaml
//!   bridge digests and the benchmark input digest. A binary digest is only
//!   meaningful for the build that produced it, which is exactly why it is not
//!   in the source layer and why it is never claimed to be portable.
//!
//! **Not on the verdict path.**

use crate::tools::config::{self, EffectiveConfig, RunFlags};
use crate::tools::guard::{self, is_hex40, valid_tag, Refused, Result};
use crate::tools::json::{self, Json};
use crate::tools::sha256;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Recognised `schema` value. A different schema is refused, not guessed at.
pub const SCHEMA: &str = "tekmerion-freeze-closure/1";

/// The tag created with the first Rust-closure freeze (XIII.C4).
pub const DEFAULT_TAG: &str = "freeze-rust-closure-v5";

/// Repository-relative files, outside `src/` and `tools/`, that shape the build.
const SOURCE_FILES: &[&str] = &[
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "rust-toolchain",
];

const CLOSURE_KEYS: &[&str] = &[
    "schema",
    "tag",
    "repo_commit",
    "created_at",
    "source",
    "source_digest",
    "configuration",
    "runtime",
];
const RUNTIME_KEYS: &[&str] = &[
    "executable",
    "rustc",
    "cargo_lock_sha256",
    "hol_light",
    "cadical",
    "ocaml_bridge",
    "benchmark_inputs",
];
const ENTRY_KEYS: &[&str] = &["path", "sha256"];
const HOL_KEYS: &[&str] = &["path", "pin", "hol_lib_cmxa_sha256"];
const CADICAL_KEYS: &[&str] = &["path", "sha256", "version"];
const BENCH_KEYS: &[&str] = &["path", "problems_json_sha256"];
const CONFIG_KEYS: &[&str] = &["env", "flags"];
const FLAG_KEYS: &[&str] = &["mode", "stamp", "force", "slice"];

// ---------------------------------------------------------------------------
// Shape
// ---------------------------------------------------------------------------

/// One content-addressed file in either layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HolBlock {
    pub path: String,
    pub pin: String,
    pub hol_lib_cmxa_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CadicalBlock {
    pub path: String,
    pub sha256: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkBlock {
    pub path: String,
    pub problems_json_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeBlock {
    pub executable: Entry,
    pub rustc: String,
    pub cargo_lock_sha256: String,
    pub hol_light: HolBlock,
    pub cadical: CadicalBlock,
    pub ocaml_bridge: Vec<Entry>,
    pub benchmark_inputs: BenchmarkBlock,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Closure {
    pub tag: String,
    pub repo_commit: String,
    pub created_at: u64,
    pub source: Vec<Entry>,
    pub source_digest: String,
    /// The effective execution configuration, when the manifest was produced
    /// by a build that records it (XVI.C). A manifest from before this field
    /// has `None`, verifies under its own scope, and says so loudly.
    pub configuration: Option<EffectiveConfig>,
    pub runtime: Option<RuntimeBlock>,
}

impl Closure {
    /// How many runtime digests the closure carries (0 for a source-only seal).
    pub fn runtime_entries(&self) -> usize {
        match &self.runtime {
            // executable, HOL Light library, cadical, benchmark input, plus bridges
            Some(rt) => 4 + rt.ocaml_bridge.len(),
            None => 0,
        }
    }
}

/// Where the runtime layer is read from when a manifest is generated.
///
/// Defaults describe this host's measurement environment; every path can be
/// overridden on the command line, and none of them is claimed portable.
#[derive(Debug, Clone)]
pub struct RuntimePaths {
    pub executable: PathBuf,
    pub hol_light: PathBuf,
    pub cadical: PathBuf,
    pub benchmark_inputs: PathBuf,
    pub rustc: String,
}

impl Default for RuntimePaths {
    fn default() -> RuntimePaths {
        RuntimePaths {
            executable: PathBuf::from("target/release/emit"),
            hol_light: PathBuf::from(".bench/hol-light-stamp"),
            cadical: PathBuf::from(
                "/home/one/.elan/toolchains/leanprover--lean4---v4.29.1/bin/cadical",
            ),
            benchmark_inputs: PathBuf::from(".bench/problems.json"),
            rustc: "rustc".to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// Canonical rendering
// ---------------------------------------------------------------------------

fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn render_entry(entry: &Entry) -> String {
    format!(
        "{{\"path\": {}, \"sha256\": {}}}",
        quote(&entry.path),
        quote(&entry.sha256)
    )
}

/// Hand-serialised canonical JSON. Deterministic for a given [`Closure`].
pub fn render(closure: &Closure) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!("  \"schema\": {},\n", quote(SCHEMA)));
    out.push_str(&format!("  \"tag\": {},\n", quote(&closure.tag)));
    out.push_str(&format!(
        "  \"repo_commit\": {},\n",
        quote(&closure.repo_commit)
    ));
    out.push_str(&format!("  \"created_at\": {},\n", closure.created_at));
    out.push_str("  \"source\": [\n");
    for (i, entry) in closure.source.iter().enumerate() {
        out.push_str("    ");
        out.push_str(&render_entry(entry));
        out.push_str(if i + 1 == closure.source.len() {
            "\n"
        } else {
            ",\n"
        });
    }
    out.push_str("  ],\n");
    out.push_str(&format!(
        "  \"source_digest\": {},\n",
        quote(&closure.source_digest)
    ));
    if let Some(config) = &closure.configuration {
        out.push_str(&format!(
            "  \"configuration\": {},\n",
            config::render_json(config)
        ));
    }
    match &closure.runtime {
        None => out.push_str("  \"runtime\": null\n"),
        Some(rt) => {
            out.push_str("  \"runtime\": {\n");
            out.push_str(&format!(
                "    \"executable\": {},\n",
                render_entry(&rt.executable)
            ));
            out.push_str(&format!("    \"rustc\": {},\n", quote(&rt.rustc)));
            out.push_str(&format!(
                "    \"cargo_lock_sha256\": {},\n",
                quote(&rt.cargo_lock_sha256)
            ));
            out.push_str("    \"hol_light\": {\n");
            out.push_str(&format!("      \"path\": {},\n", quote(&rt.hol_light.path)));
            out.push_str(&format!("      \"pin\": {},\n", quote(&rt.hol_light.pin)));
            out.push_str(&format!(
                "      \"hol_lib_cmxa_sha256\": {}\n",
                quote(&rt.hol_light.hol_lib_cmxa_sha256)
            ));
            out.push_str("    },\n");
            out.push_str("    \"cadical\": {\n");
            out.push_str(&format!("      \"path\": {},\n", quote(&rt.cadical.path)));
            out.push_str(&format!(
                "      \"sha256\": {},\n",
                quote(&rt.cadical.sha256)
            ));
            out.push_str(&format!(
                "      \"version\": {}\n",
                quote(&rt.cadical.version)
            ));
            out.push_str("    },\n");
            out.push_str("    \"ocaml_bridge\": [\n");
            for (i, entry) in rt.ocaml_bridge.iter().enumerate() {
                out.push_str("      ");
                out.push_str(&render_entry(entry));
                out.push_str(if i + 1 == rt.ocaml_bridge.len() {
                    "\n"
                } else {
                    ",\n"
                });
            }
            out.push_str("    ],\n");
            out.push_str("    \"benchmark_inputs\": {\n");
            out.push_str(&format!(
                "      \"path\": {},\n",
                quote(&rt.benchmark_inputs.path)
            ));
            out.push_str(&format!(
                "      \"problems_json_sha256\": {}\n",
                quote(&rt.benchmark_inputs.problems_json_sha256)
            ));
            out.push_str("    }\n");
            out.push_str("  }\n");
        }
    }
    out.push_str("}\n");
    out
}

// ---------------------------------------------------------------------------
// Source closure
// ---------------------------------------------------------------------------

/// `tools/**/*.ml` anywhere, and `tools/*.sh` only at the top level.
fn keep_tool(path: &Path) -> bool {
    match path.extension().and_then(|e| e.to_str()) {
        Some("ml") => true,
        Some("sh") => {
            path.parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                == Some("tools")
        }
        _ => false,
    }
}

/// Does a repo-relative path belong to the source closure?
pub fn is_source_path(rel: &str) -> bool {
    if rel.starts_with("src/") || SOURCE_FILES.contains(&rel) {
        return true;
    }
    match rel.strip_prefix("tools/") {
        Some(name) => name.ends_with(".ml") || (!name.contains('/') && name.ends_with(".sh")),
        None => false,
    }
}

fn walk(dir: &Path, rel: &str, keep: &dyn Fn(&Path) -> bool, out: &mut Vec<String>) -> Result<()> {
    let entries = fs::read_dir(dir)
        .map_err(|e| Refused::new(format!("cannot list {}: {e}", dir.display())))?;
    for entry in entries {
        let entry =
            entry.map_err(|e| Refused::new(format!("cannot list {}: {e}", dir.display())))?;
        let path = entry.path();
        let kind = entry
            .file_type()
            .map_err(|e| Refused::new(format!("cannot stat {}: {e}", path.display())))?;
        let name = entry.file_name().to_string_lossy().to_string();
        let child = format!("{rel}/{name}");
        if kind.is_symlink() {
            return Err(Refused::new(format!(
                "source tree contains a symlink: {child}"
            )));
        }
        if kind.is_dir() {
            walk(&path, &child, keep, out)?;
        } else if kind.is_file() && keep(&path) {
            out.push(child);
        }
    }
    Ok(())
}

fn hash_file(path: &Path) -> Result<String> {
    let bytes =
        fs::read(path).map_err(|e| Refused::new(format!("cannot read {}: {e}", path.display())))?;
    Ok(sha256::hex(&bytes))
}

/// The rollup digest over the canonical `path\0sha256\n` concatenation.
pub fn digest_of(entries: &[Entry]) -> String {
    let mut buf = Vec::new();
    for entry in entries {
        buf.extend_from_slice(entry.path.as_bytes());
        buf.push(0);
        buf.extend_from_slice(entry.sha256.as_bytes());
        buf.push(b'\n');
    }
    sha256::hex(&buf)
}

/// Every source-closure file in the working tree, sorted by path.
pub fn source_entries(root: &Path) -> Result<Vec<Entry>> {
    let src = root.join("src");
    if !src.is_dir() {
        return Err(Refused::new(format!(
            "source closure root {} is not a directory",
            src.display()
        )));
    }
    let tools = root.join("tools");
    if !tools.is_dir() {
        return Err(Refused::new(format!(
            "source closure root {} is not a directory",
            tools.display()
        )));
    }
    let mut rels = Vec::new();
    walk(&src, "src", &|_: &Path| true, &mut rels)?;
    walk(&tools, "tools", &keep_tool, &mut rels)?;
    for name in SOURCE_FILES {
        if root.join(name).is_file() {
            rels.push((*name).to_string());
        }
    }
    rels.sort();
    let mut out = Vec::with_capacity(rels.len());
    for rel in rels {
        let digest = hash_file(&root.join(&rel))?;
        out.push(Entry {
            path: rel,
            sha256: digest,
        });
    }
    Ok(out)
}

/// Compare recorded and recomputed source closures; refuse on the first
/// difference, naming the path and both digests where both are known.
fn compare_source(recorded: &[Entry], actual: &[Entry], context: &str) -> Result<()> {
    let want: BTreeMap<&str, &str> = recorded
        .iter()
        .map(|e| (e.path.as_str(), e.sha256.as_str()))
        .collect();
    let got: BTreeMap<&str, &str> = actual
        .iter()
        .map(|e| (e.path.as_str(), e.sha256.as_str()))
        .collect();
    let mut paths: BTreeSet<&str> = BTreeSet::new();
    paths.extend(want.keys().copied());
    paths.extend(got.keys().copied());
    for path in paths {
        match (want.get(path), got.get(path)) {
            (Some(w), Some(g)) if w != g => {
                return Err(Refused::new(format!(
                    "{context}: source file {path} differs: recorded {w}, actual {g}"
                )));
            }
            (Some(_), None) => {
                return Err(Refused::new(format!(
                    "{context}: source file {path} is sealed but missing from the tree"
                )));
            }
            (None, Some(_)) => {
                return Err(Refused::new(format!(
                    "{context}: source file {path} is present but not in the sealed closure"
                )));
            }
            _ => {}
        }
    }
    Ok(())
}

/// Recompute the source closure from the working tree and compare.
pub fn verify_source(root: &Path, closure: &Closure) -> Result<()> {
    let actual = source_entries(root)?;
    compare_source(&closure.source, &actual, "sealed source closure")
}

// ---------------------------------------------------------------------------
// Runtime closure
// ---------------------------------------------------------------------------

fn path_text(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn resolve(root: &Path, recorded: &str) -> PathBuf {
    let path = Path::new(recorded);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

fn first_line(text: impl AsRef<str>) -> String {
    text.as_ref()
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string()
}

fn run_capture(program: &str, args: &[&str]) -> Result<String> {
    let out = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| Refused::new(format!("cannot run {program}: {e}")))?;
    if !out.status.success() {
        return Err(Refused::new(format!(
            "{program} {} failed with exit {}",
            args.join(" "),
            out.status.code().unwrap_or(-1)
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

fn git_raw(root: &Path, args: &[&str]) -> Result<std::process::Output> {
    let out = Command::new("git")
        .arg("--no-optional-locks")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|e| Refused::new(format!("cannot run git in {}: {e}", root.display())))?;
    if !out.status.success() {
        let detail = String::from_utf8_lossy(&out.stderr).to_string();
        let first = detail.trim().lines().next().unwrap_or("").to_string();
        return Err(Refused::new(format!(
            "git {} failed in {}: {first}",
            args.join(" "),
            root.display()
        )));
    }
    Ok(out)
}

fn git_text(root: &Path, args: &[&str]) -> Result<String> {
    Ok(String::from_utf8_lossy(&git_raw(root, args)?.stdout).to_string())
}

fn git_bytes(root: &Path, args: &[&str]) -> Result<Vec<u8>> {
    Ok(git_raw(root, args)?.stdout)
}

fn bridge_entries(root: &Path) -> Result<Vec<Entry>> {
    let dir = root.join("tools");
    let entries = fs::read_dir(&dir)
        .map_err(|e| Refused::new(format!("cannot list {}: {e}", dir.display())))?;
    let mut names = Vec::new();
    for entry in entries {
        let entry =
            entry.map_err(|e| Refused::new(format!("cannot list {}: {e}", dir.display())))?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("ml") {
            names.push(entry.file_name().to_string_lossy().to_string());
        }
    }
    names.sort();
    let mut out = Vec::with_capacity(names.len());
    for name in names {
        let digest = hash_file(&dir.join(&name))?;
        out.push(Entry {
            path: format!("tools/{name}"),
            sha256: digest,
        });
    }
    Ok(out)
}

fn runtime_block(root: &Path, paths: &RuntimePaths) -> Result<RuntimeBlock> {
    let executable = path_text(&paths.executable);
    let executable_digest = hash_file(&resolve(root, &executable))?;

    let rustc = first_line(run_capture(&paths.rustc, &["--version"])?);
    let cargo_lock_sha256 = hash_file(&root.join("Cargo.lock"))?;

    let hol_path = path_text(&paths.hol_light);
    let hol_dir = resolve(root, &hol_path);
    let pin = first_line(git_text(&hol_dir, &["rev-parse", "--short", "HEAD"])?);
    let hol_lib_cmxa_sha256 = hash_file(&hol_dir.join("hol_lib.cmxa"))?;

    let cadical_path = path_text(&paths.cadical);
    let cadical_file = resolve(root, &cadical_path);
    let cadical_sha256 = hash_file(&cadical_file)?;
    let version = first_line(run_capture(&path_text(&cadical_file), &["--version"])?);

    let benchmark_path = path_text(&paths.benchmark_inputs);
    let problems_json_sha256 = hash_file(&resolve(root, &benchmark_path))?;

    Ok(RuntimeBlock {
        executable: Entry {
            path: executable,
            sha256: executable_digest,
        },
        rustc,
        cargo_lock_sha256,
        hol_light: HolBlock {
            path: hol_path,
            pin,
            hol_lib_cmxa_sha256,
        },
        cadical: CadicalBlock {
            path: cadical_path,
            sha256: cadical_sha256,
            version,
        },
        ocaml_bridge: bridge_entries(root)?,
        benchmark_inputs: BenchmarkBlock {
            path: benchmark_path,
            problems_json_sha256,
        },
    })
}

/// Recompute and compare every runtime digest. Machine-specific by design:
/// a mismatch here means "not the build that produced the proofs".
pub fn verify_runtime(root: &Path, closure: &Closure) -> Result<()> {
    let rt = match &closure.runtime {
        Some(rt) => rt,
        None => {
            return Err(Refused::new(
                "freeze closure has no runtime layer; a sealed run cannot bind the producer",
            ))
        }
    };

    let executable = resolve(root, &rt.executable.path);
    let actual = hash_file(&executable)?;
    if actual != rt.executable.sha256 {
        return Err(Refused::new(format!(
            "runtime executable digest mismatch for {}: recorded {}, actual {actual}",
            rt.executable.path, rt.executable.sha256
        )));
    }

    let actual = first_line(run_capture("rustc", &["--version"])?);
    if actual != rt.rustc {
        return Err(Refused::new(format!(
            "runtime rustc mismatch: recorded {:?}, actual {:?}",
            rt.rustc, actual
        )));
    }

    let actual = hash_file(&root.join("Cargo.lock"))?;
    if actual != rt.cargo_lock_sha256 {
        return Err(Refused::new(format!(
            "runtime Cargo.lock digest mismatch: recorded {}, actual {actual}",
            rt.cargo_lock_sha256
        )));
    }

    let hol_dir = resolve(root, &rt.hol_light.path);
    let actual = first_line(git_text(&hol_dir, &["rev-parse", "--short", "HEAD"])?);
    if actual != rt.hol_light.pin {
        return Err(Refused::new(format!(
            "runtime HOL Light pin mismatch for {}: recorded {}, actual {actual}",
            rt.hol_light.path, rt.hol_light.pin
        )));
    }
    let library = hol_dir.join("hol_lib.cmxa");
    let actual = hash_file(&library)?;
    if actual != rt.hol_light.hol_lib_cmxa_sha256 {
        return Err(Refused::new(format!(
            "runtime HOL Light hol_lib.cmxa digest mismatch for {}: recorded {}, actual {actual}",
            library.display(),
            rt.hol_light.hol_lib_cmxa_sha256
        )));
    }

    let cadical = resolve(root, &rt.cadical.path);
    let actual = hash_file(&cadical)?;
    if actual != rt.cadical.sha256 {
        return Err(Refused::new(format!(
            "runtime cadical digest mismatch for {}: recorded {}, actual {actual}",
            rt.cadical.path, rt.cadical.sha256
        )));
    }
    let actual = first_line(run_capture(&path_text(&cadical), &["--version"])?);
    if actual != rt.cadical.version {
        return Err(Refused::new(format!(
            "runtime cadical version mismatch for {}: recorded {}, actual {actual}",
            rt.cadical.path, rt.cadical.version
        )));
    }

    for entry in &rt.ocaml_bridge {
        let actual = hash_file(&root.join(&entry.path))?;
        if actual != entry.sha256 {
            return Err(Refused::new(format!(
                "runtime OCaml bridge digest mismatch for {}: recorded {}, actual {actual}",
                entry.path, entry.sha256
            )));
        }
    }

    let benchmark = resolve(root, &rt.benchmark_inputs.path);
    let actual = hash_file(&benchmark)?;
    if actual != rt.benchmark_inputs.problems_json_sha256 {
        return Err(Refused::new(format!(
            "runtime benchmark input digest mismatch for {}: recorded {}, actual {actual}",
            rt.benchmark_inputs.path, rt.benchmark_inputs.problems_json_sha256
        )));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Generation
// ---------------------------------------------------------------------------

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Build a closure manifest from the working tree, and optionally the runtime.
///
/// The configuration is not captured here: `build` is the source/runtime seal
/// used by the tests and by any caller that has no run to bind. The CLI uses
/// [`build_with_config`], which records the effective configuration (XVI.C).
pub fn build(root: &Path, tag: &str, runtime: Option<&RuntimePaths>) -> Result<Closure> {
    build_with_config(root, tag, runtime, None)
}

/// As [`build`], additionally recording the effective execution configuration.
pub fn build_with_config(
    root: &Path,
    tag: &str,
    runtime: Option<&RuntimePaths>,
    configuration: Option<EffectiveConfig>,
) -> Result<Closure> {
    if !valid_tag(tag) {
        return Err(Refused::new(format!(
            "freeze tag is not a valid tag name: {tag:?}"
        )));
    }
    let commit = git_text(root, &["rev-parse", "HEAD"])?;
    let repo_commit = commit.trim().to_lowercase();
    let source = source_entries(root)?;
    let source_digest = digest_of(&source);
    let runtime = match runtime {
        Some(paths) => Some(runtime_block(root, paths)?),
        None => None,
    };
    Ok(Closure {
        tag: tag.to_string(),
        repo_commit,
        created_at: now(),
        source,
        source_digest,
        configuration,
        runtime,
    })
}

// ---------------------------------------------------------------------------
// Strict parsing
// ---------------------------------------------------------------------------

fn as_obj<'a>(value: &'a Json, where_: &str) -> Result<&'a BTreeMap<String, Json>> {
    match value.as_obj() {
        Some(obj) => Ok(obj),
        None => Err(Refused::new(format!("{where_} must be a JSON object"))),
    }
}

fn as_arr<'a>(value: &'a Json, where_: &str) -> Result<&'a [Json]> {
    match value.as_array() {
        Some(items) => Ok(items),
        None => Err(Refused::new(format!("{where_} must be a JSON array"))),
    }
}

fn require<'a>(obj: &'a BTreeMap<String, Json>, key: &str, where_: &str) -> Result<&'a Json> {
    match obj.get(key) {
        Some(value) => Ok(value),
        None => Err(Refused::new(format!("{where_}: {key} is required"))),
    }
}

fn string_field<'a>(obj: &'a BTreeMap<String, Json>, key: &str, where_: &str) -> Result<&'a str> {
    match obj.get(key).and_then(Json::as_str) {
        Some(text) => Ok(text),
        None => Err(Refused::new(format!("{where_}: {key} must be a string"))),
    }
}

fn nonempty_field<'a>(obj: &'a BTreeMap<String, Json>, key: &str, where_: &str) -> Result<&'a str> {
    let text = string_field(obj, key, where_)?;
    if text.trim().is_empty() {
        return Err(Refused::new(format!("{where_}: {key} must not be empty")));
    }
    Ok(text)
}

fn require_hex64(digest: &str, where_: &str) -> Result<()> {
    if digest.len() == 64
        && digest
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        Ok(())
    } else {
        Err(Refused::new(format!(
            "{where_} is not a 64-character lowercase hex sha256: {digest:?}"
        )))
    }
}

fn hex_field<'a>(obj: &'a BTreeMap<String, Json>, key: &str, where_: &str) -> Result<&'a str> {
    let text = string_field(obj, key, where_)?;
    require_hex64(text, &format!("{where_}: {key}"))?;
    Ok(text)
}

fn allow_only(obj: &BTreeMap<String, Json>, allowed: &[&str], where_: &str) -> Result<()> {
    for key in obj.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(Refused::new(format!("{where_} has unknown key {key:?}")));
        }
    }
    Ok(())
}

fn clean_relative(path: &str, where_: &str) -> Result<()> {
    if path.is_empty()
        || path.starts_with('/')
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(Refused::new(format!(
            "{where_}: path is not a clean repo-relative path: {path:?}"
        )));
    }
    Ok(())
}

fn clean_any(path: &str, where_: &str) -> Result<()> {
    let parent = Path::new(path)
        .components()
        .any(|c| matches!(c, Component::ParentDir));
    if path.is_empty() || parent {
        return Err(Refused::new(format!(
            "{where_}: path is empty or contains '..': {path:?}"
        )));
    }
    Ok(())
}

fn parse_entry(value: &Json, where_: &str) -> Result<Entry> {
    let obj = as_obj(value, where_)?;
    allow_only(obj, ENTRY_KEYS, where_)?;
    let path = string_field(obj, "path", where_)?;
    clean_relative(path, where_)?;
    let sha256 = hex_field(obj, "sha256", where_)?;
    Ok(Entry {
        path: path.to_string(),
        sha256: sha256.to_string(),
    })
}

fn parse_hol(obj: &BTreeMap<String, Json>) -> Result<HolBlock> {
    allow_only(obj, HOL_KEYS, "freeze runtime hol_light")?;
    let path = nonempty_field(obj, "path", "freeze runtime hol_light")?;
    clean_any(path, "freeze runtime hol_light")?;
    let pin = nonempty_field(obj, "pin", "freeze runtime hol_light")?;
    let digest = hex_field(obj, "hol_lib_cmxa_sha256", "freeze runtime hol_light")?;
    Ok(HolBlock {
        path: path.to_string(),
        pin: pin.to_string(),
        hol_lib_cmxa_sha256: digest.to_string(),
    })
}

fn parse_cadical(obj: &BTreeMap<String, Json>) -> Result<CadicalBlock> {
    allow_only(obj, CADICAL_KEYS, "freeze runtime cadical")?;
    let path = nonempty_field(obj, "path", "freeze runtime cadical")?;
    clean_any(path, "freeze runtime cadical")?;
    let sha256 = hex_field(obj, "sha256", "freeze runtime cadical")?;
    let version = nonempty_field(obj, "version", "freeze runtime cadical")?;
    Ok(CadicalBlock {
        path: path.to_string(),
        sha256: sha256.to_string(),
        version: version.to_string(),
    })
}

fn parse_benchmark(obj: &BTreeMap<String, Json>) -> Result<BenchmarkBlock> {
    allow_only(obj, BENCH_KEYS, "freeze runtime benchmark_inputs")?;
    let path = nonempty_field(obj, "path", "freeze runtime benchmark_inputs")?;
    clean_any(path, "freeze runtime benchmark_inputs")?;
    let digest = hex_field(
        obj,
        "problems_json_sha256",
        "freeze runtime benchmark_inputs",
    )?;
    Ok(BenchmarkBlock {
        path: path.to_string(),
        problems_json_sha256: digest.to_string(),
    })
}

fn parse_configuration(raw: &Json, where_: &str) -> Result<EffectiveConfig> {
    let obj = as_obj(raw, where_)?;
    allow_only(obj, CONFIG_KEYS, where_)?;

    let env_where = format!("{where_} env");
    let env_obj = as_obj(require(obj, "env", where_)?, &env_where)?;
    let mut env = BTreeMap::new();
    for (key, value) in env_obj {
        let text = match value {
            Json::Str(text) => text.clone(),
            // `null` is accepted as a second spelling of unset so a hand-edited
            // manifest cannot miss a variable by writing the natural sentinel.
            Json::Null => config::UNSET.to_string(),
            _ => {
                return Err(Refused::new(format!(
                    "{env_where}: {key} must be a string or null"
                )))
            }
        };
        let recorded = if text == config::UNSET {
            None
        } else {
            Some(text)
        };
        env.insert(key.clone(), recorded);
    }

    let flags_where = format!("{where_} flags");
    let flags_obj = as_obj(require(obj, "flags", where_)?, &flags_where)?;
    allow_only(flags_obj, FLAG_KEYS, &flags_where)?;
    let mode = match string_field(flags_obj, "mode", &flags_where)? {
        "unspecified" => None,
        other => Some(other.to_string()),
    };
    let stamp = config::parse_tri(
        string_field(flags_obj, "stamp", &flags_where)?,
        &format!("{flags_where} stamp"),
    )?;
    let force = config::parse_tri(
        string_field(flags_obj, "force", &flags_where)?,
        &format!("{flags_where} force"),
    )?;
    let slice = match string_field(flags_obj, "slice", &flags_where)? {
        "on" => true,
        "off" => false,
        other => {
            return Err(Refused::new(format!(
                "{flags_where} slice must be on or off: {other:?}"
            )))
        }
    };

    EffectiveConfig::from_entries(
        env,
        RunFlags {
            mode,
            stamp,
            force,
            slice,
        },
    )
}

fn parse_runtime(obj: &BTreeMap<String, Json>) -> Result<RuntimeBlock> {
    allow_only(obj, RUNTIME_KEYS, "freeze runtime")?;

    let executable = parse_entry(
        require(obj, "executable", "freeze runtime")?,
        "freeze runtime executable",
    )?;
    let rustc = nonempty_field(obj, "rustc", "freeze runtime")?.to_string();
    let cargo_lock_sha256 = hex_field(obj, "cargo_lock_sha256", "freeze runtime")?.to_string();
    let hol_light = parse_hol(as_obj(
        require(obj, "hol_light", "freeze runtime")?,
        "freeze runtime hol_light",
    )?)?;
    let cadical = parse_cadical(as_obj(
        require(obj, "cadical", "freeze runtime")?,
        "freeze runtime cadical",
    )?)?;
    let raw_bridge = as_arr(
        require(obj, "ocaml_bridge", "freeze runtime")?,
        "freeze runtime ocaml_bridge",
    )?;
    let mut ocaml_bridge = Vec::with_capacity(raw_bridge.len());
    for item in raw_bridge {
        ocaml_bridge.push(parse_entry(item, "freeze runtime ocaml_bridge")?);
    }
    let benchmark_inputs = parse_benchmark(as_obj(
        require(obj, "benchmark_inputs", "freeze runtime")?,
        "freeze runtime benchmark_inputs",
    )?)?;

    Ok(RuntimeBlock {
        executable,
        rustc,
        cargo_lock_sha256,
        hol_light,
        cadical,
        ocaml_bridge,
        benchmark_inputs,
    })
}

/// Parse and strictly validate a closure manifest. Fail closed on anything.
pub fn parse(bytes: &[u8], path: &Path) -> Result<Closure> {
    let payload = json::parse(bytes)
        .map_err(|e| Refused::new(format!("cannot parse {}: {e}", path.display())))?;
    let obj = as_obj(&payload, "freeze manifest")?;
    allow_only(obj, CLOSURE_KEYS, "freeze manifest")?;

    let schema = string_field(obj, "schema", "freeze manifest")?;
    if schema != SCHEMA {
        return Err(Refused::new(format!(
            "unsupported freeze schema {schema:?} (want {SCHEMA:?})"
        )));
    }
    let tag = nonempty_field(obj, "tag", "freeze manifest")?;
    if !valid_tag(tag) {
        return Err(Refused::new(format!(
            "freeze tag is not a valid tag name: {tag:?}"
        )));
    }
    let repo_commit = string_field(obj, "repo_commit", "freeze manifest")?;
    if !is_hex40(repo_commit) {
        return Err(Refused::new(format!(
            "repo_commit is not a 40-character lowercase hex sha1: {repo_commit:?}"
        )));
    }
    let created_at = match obj.get("created_at") {
        Some(Json::Num(n)) if *n >= 0.0 && n.fract() == 0.0 && *n <= u64::MAX as f64 => *n as u64,
        _ => {
            return Err(Refused::new(
                "created_at must be a non-negative integer unix timestamp",
            ))
        }
    };

    let raw_source = as_arr(
        require(obj, "source", "freeze manifest")?,
        "freeze manifest source",
    )?;
    let mut source = Vec::with_capacity(raw_source.len());
    for item in raw_source {
        source.push(parse_entry(item, "freeze manifest source")?);
    }
    if source.is_empty() {
        return Err(Refused::new(
            "freeze manifest source closure is empty; a seal must bind something",
        ));
    }
    for pair in source.windows(2) {
        if pair[0].path.as_str() >= pair[1].path.as_str() {
            return Err(Refused::new(format!(
                "freeze manifest source is not strictly sorted by path: {:?} then {:?}",
                pair[0].path, pair[1].path
            )));
        }
    }

    let recorded = hex_field(obj, "source_digest", "freeze manifest")?;
    let computed = digest_of(&source);
    if computed != recorded {
        return Err(Refused::new(format!(
            "source_digest does not match the manifest's own source list: recorded {recorded}, computed {computed}"
        )));
    }

    let runtime = match obj.get("runtime") {
        None | Some(Json::Null) => None,
        Some(value) => Some(parse_runtime(as_obj(value, "freeze runtime")?)?),
    };

    let configuration = match obj.get("configuration") {
        None | Some(Json::Null) => None,
        Some(value) => Some(parse_configuration(value, "freeze configuration")?),
    };

    Ok(Closure {
        tag: tag.to_string(),
        repo_commit: repo_commit.to_string(),
        created_at,
        source,
        source_digest: recorded.to_string(),
        configuration,
        runtime,
    })
}

pub fn load(path: &Path) -> Result<Closure> {
    let bytes =
        fs::read(path).map_err(|e| Refused::new(format!("cannot read {}: {e}", path.display())))?;
    parse(&bytes, path)
}

/// The warning for a manifest that predates configuration binding (XVI.C).
/// `None` means the effective configuration is bound and must verify.
pub fn configuration_warning(closure: &Closure) -> Option<&'static str> {
    match &closure.configuration {
        Some(_) => None,
        None => Some(
            "WARNING: manifest records no effective configuration; the TEKMERION_* \
             variables that change the reasoning path are not bound (legacy scope).",
        ),
    }
}

// ---------------------------------------------------------------------------
// Tag binding
// ---------------------------------------------------------------------------

/// Source closure as recorded in a commit's tree, computed from git objects.
fn tree_source_entries(root: &Path, commit: &str) -> Result<Vec<Entry>> {
    let listing = git_text(root, &["ls-tree", "-r", "--name-only", commit])?;
    let mut rels: Vec<String> = listing
        .lines()
        .filter(|line| is_source_path(line))
        .map(str::to_string)
        .collect();
    rels.sort();
    let mut out = Vec::with_capacity(rels.len());
    for rel in rels {
        let bytes = git_bytes(root, &["show", &format!("{commit}:{rel}")])?;
        out.push(Entry {
            path: rel,
            sha256: sha256::hex(&bytes),
        });
    }
    Ok(out)
}

fn is_ancestor(root: &Path, commit: &str, of: &str) -> bool {
    Command::new("git")
        .arg("--no-optional-locks")
        .args(["merge-base", "--is-ancestor", commit, of])
        .current_dir(root)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Verify the tag binding and return the commit it resolves to.
///
/// A manifest cannot contain the hash of the commit that contains it: the
/// content would have to be a fixed point of SHA-1. So `repo_commit` is the
/// sealed *code* commit and the tag may sit at that commit or at a descendant
/// that only adds the manifest and receipts. Both trees' source closures must
/// equal the recorded closure, and the **complete** manifest committed at the
/// tag must be byte-identical, under canonical rendering, to the manifest
/// under verification (XVII.A4). Comparing selected digests was not enough: a
/// committed manifest that differed in the runtime block, the configuration
/// block or a file entry could still pass.
pub fn check_tag(root: &Path, closure: &Closure, manifest_rel: &str) -> Result<String> {
    let spec = format!("{}^{{commit}}", closure.tag);
    let tag_commit = git_text(root, &["rev-parse", "--verify", &spec])?
        .trim()
        .to_lowercase();

    if tag_commit != closure.repo_commit && !is_ancestor(root, &closure.repo_commit, &tag_commit) {
        return Err(Refused::new(format!(
            "tag {:?} resolves to {tag_commit}, which is neither the sealed code commit {} nor its descendant",
            closure.tag, closure.repo_commit
        )));
    }

    let sealed = tree_source_entries(root, &closure.repo_commit)?;
    compare_source(
        &closure.source,
        &sealed,
        &format!("sealed code commit {}", closure.repo_commit),
    )?;

    if tag_commit != closure.repo_commit {
        let tagged = tree_source_entries(root, &tag_commit)?;
        compare_source(
            &closure.source,
            &tagged,
            &format!("tag commit {tag_commit}"),
        )?;
    }

    let committed =
        git_bytes(root, &["show", &format!("{tag_commit}:{manifest_rel}")]).map_err(|e| {
            Refused::new(format!(
                "the tag's tree has no manifest at {manifest_rel}: {e}"
            ))
        })?;
    let at_tag = parse(&committed, Path::new(manifest_rel))?;

    if at_tag.source_digest != closure.source_digest {
        return Err(Refused::new(format!(
            "manifest committed at tag {tag_commit} records source_digest {}, but {manifest_rel} records {}",
            at_tag.source_digest, closure.source_digest
        )));
    }
    if at_tag.repo_commit != closure.repo_commit {
        return Err(Refused::new(format!(
            "manifest committed at tag {tag_commit} records repo_commit {}, but {manifest_rel} records {}",
            at_tag.repo_commit, closure.repo_commit
        )));
    }

    // Bind every remaining field through the canonical bytes. The two explicit
    // checks above keep their specific messages; anything else that differs
    // (the runtime block, the configuration block, a source entry, the tag or
    // created_at) is refused here, naming the field where the structure tells
    // us which one, else the first differing byte offset.
    let committed_bytes = render(&at_tag);
    let local_bytes = render(closure);
    if committed_bytes != local_bytes {
        let where_ = match manifest_difference(&at_tag, closure) {
            Some(field) => format!("field {field}"),
            None => format!(
                "byte offset {}",
                first_byte_offset(&committed_bytes, &local_bytes)
            ),
        };
        return Err(Refused::new(format!(
            "manifest committed at tag {tag_commit} differs from {manifest_rel} at {where_}; \
             the tag binds the complete manifest, not selected digests"
        )));
    }
    Ok(tag_commit)
}

/// The offset of the first differing byte, or the length of the shorter string
/// when one is a prefix of the other.
fn first_byte_offset(a: &str, b: &str) -> usize {
    a.bytes()
        .zip(b.bytes())
        .position(|(x, y)| x != y)
        .unwrap_or(a.len().min(b.len()))
}

fn entry_field_difference(a: &Entry, b: &Entry) -> Option<&'static str> {
    if a.path != b.path {
        Some("path")
    } else if a.sha256 != b.sha256 {
        Some("sha256")
    } else {
        None
    }
}

fn entries_difference(prefix: &str, a: &[Entry], b: &[Entry]) -> Option<String> {
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        if let Some(field) = entry_field_difference(x, y) {
            return Some(format!("{prefix}[{i}].{field}"));
        }
    }
    if a.len() != b.len() {
        Some(format!("{prefix}.len"))
    } else {
        None
    }
}

fn configuration_difference(a: &EffectiveConfig, b: &EffectiveConfig) -> Option<String> {
    for name in config::ALL_ENV_VARS {
        if a.value(name) != b.value(name) {
            return Some(format!("configuration.env.{name}"));
        }
    }
    let (fa, fb) = (a.flags(), b.flags());
    if fa.mode != fb.mode {
        return Some("configuration.flags.mode".to_string());
    }
    if fa.stamp != fb.stamp {
        return Some("configuration.flags.stamp".to_string());
    }
    if fa.force != fb.force {
        return Some("configuration.flags.force".to_string());
    }
    if fa.slice != fb.slice {
        return Some("configuration.flags.slice".to_string());
    }
    None
}

fn runtime_difference(a: &RuntimeBlock, b: &RuntimeBlock) -> Option<String> {
    if let Some(field) = entry_field_difference(&a.executable, &b.executable) {
        return Some(format!("runtime.executable.{field}"));
    }
    if a.rustc != b.rustc {
        return Some("runtime.rustc".to_string());
    }
    if a.cargo_lock_sha256 != b.cargo_lock_sha256 {
        return Some("runtime.cargo_lock_sha256".to_string());
    }
    if a.hol_light.path != b.hol_light.path {
        return Some("runtime.hol_light.path".to_string());
    }
    if a.hol_light.pin != b.hol_light.pin {
        return Some("runtime.hol_light.pin".to_string());
    }
    if a.hol_light.hol_lib_cmxa_sha256 != b.hol_light.hol_lib_cmxa_sha256 {
        return Some("runtime.hol_light.hol_lib_cmxa_sha256".to_string());
    }
    if a.cadical.path != b.cadical.path {
        return Some("runtime.cadical.path".to_string());
    }
    if a.cadical.sha256 != b.cadical.sha256 {
        return Some("runtime.cadical.sha256".to_string());
    }
    if a.cadical.version != b.cadical.version {
        return Some("runtime.cadical.version".to_string());
    }
    if let Some(field) =
        entries_difference("runtime.ocaml_bridge", &a.ocaml_bridge, &b.ocaml_bridge)
    {
        return Some(field);
    }
    if a.benchmark_inputs.path != b.benchmark_inputs.path {
        return Some("runtime.benchmark_inputs.path".to_string());
    }
    if a.benchmark_inputs.problems_json_sha256 != b.benchmark_inputs.problems_json_sha256 {
        return Some("runtime.benchmark_inputs.problems_json_sha256".to_string());
    }
    None
}

/// The first field that differs, in canonical-render order.
fn manifest_difference(a: &Closure, b: &Closure) -> Option<String> {
    if a.tag != b.tag {
        return Some("tag".to_string());
    }
    if a.repo_commit != b.repo_commit {
        return Some("repo_commit".to_string());
    }
    if a.created_at != b.created_at {
        return Some("created_at".to_string());
    }
    if let Some(field) = entries_difference("source", &a.source, &b.source) {
        return Some(field);
    }
    if a.source_digest != b.source_digest {
        return Some("source_digest".to_string());
    }
    match (&a.configuration, &b.configuration) {
        (None, None) => {}
        (Some(_), None) | (None, Some(_)) => return Some("configuration".to_string()),
        (Some(x), Some(y)) => {
            if let Some(field) = configuration_difference(x, y) {
                return Some(field);
            }
        }
    }
    match (&a.runtime, &b.runtime) {
        (None, None) => {}
        (Some(_), None) | (None, Some(_)) => return Some("runtime".to_string()),
        (Some(x), Some(y)) => {
            if let Some(field) = runtime_difference(x, y) {
                return Some(field);
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

fn usage() {
    eprintln!(
        "usage:\n  freeze manifest --out PATH [--runtime] [--tag NAME] [--repo PATH]\n\
         \x20                [--executable P] [--hol P] [--cadical P] [--benchmark P]\n\
         \x20                [--mode M] [--stamp|--no-stamp] [--force|--no-force]\n\
         \x20 freeze verify --manifest PATH [--runtime] [--repo PATH]\n\
         \x20 freeze check-tag --manifest PATH [--repo PATH]\n\
         Exit 0 allowed, 2 refused."
    );
}

fn report(result: Result<()>, what: &str) -> i32 {
    match result {
        Ok(()) => {
            println!("OK: {what}");
            guard::EXIT_ALLOWED
        }
        Err(e) => {
            eprintln!("REFUSED: {e}");
            guard::EXIT_REFUSED
        }
    }
}

fn relative_to_root(root: &Path, path: &Path) -> Result<String> {
    let root = guard::realpath(root);
    let full = guard::realpath(path);
    let rel = full.strip_prefix(&root).map_err(|_| {
        Refused::new(format!(
            "manifest {} is outside the repository {}",
            full.display(),
            root.display()
        ))
    })?;
    Ok(rel.to_string_lossy().replace('\\', "/"))
}

pub fn run(argv: &[String]) -> i32 {
    let mut repo: Option<PathBuf> = None;
    let mut out: Option<String> = None;
    let mut manifest: Option<String> = None;
    let mut tag: Option<String> = None;
    let mut runtime = false;
    let mut paths = RuntimePaths::default();
    let mut command: Option<String> = None;
    let mut mode: Option<String> = None;
    let mut stamp: Option<bool> = None;
    let mut force: Option<bool> = None;

    let mut i = 0;
    while i < argv.len() {
        let arg = &argv[i];
        if let Some(v) = guard::take_value(argv, &mut i, "--repo") {
            repo = Some(PathBuf::from(v));
        } else if let Some(v) = guard::take_value(argv, &mut i, "--out") {
            out = Some(v);
        } else if let Some(v) = guard::take_value(argv, &mut i, "--manifest") {
            manifest = Some(v);
        } else if let Some(v) = guard::take_value(argv, &mut i, "--tag") {
            tag = Some(v);
        } else if let Some(v) = guard::take_value(argv, &mut i, "--executable") {
            paths.executable = PathBuf::from(v);
        } else if let Some(v) = guard::take_value(argv, &mut i, "--hol") {
            paths.hol_light = PathBuf::from(v);
        } else if let Some(v) = guard::take_value(argv, &mut i, "--cadical") {
            paths.cadical = PathBuf::from(v);
        } else if let Some(v) = guard::take_value(argv, &mut i, "--benchmark") {
            paths.benchmark_inputs = PathBuf::from(v);
        } else if let Some(v) = guard::take_value(argv, &mut i, "--mode") {
            mode = Some(v);
        } else {
            match arg.as_str() {
                "--runtime" => runtime = true,
                "--stamp" => stamp = Some(true),
                "--no-stamp" => stamp = Some(false),
                "--force" => force = Some(true),
                "--no-force" => force = Some(false),
                "--help" | "-h" => {
                    usage();
                    return guard::EXIT_ALLOWED;
                }
                other if other.starts_with("--") => {
                    eprintln!("REFUSED: unknown option {other}");
                    return guard::EXIT_REFUSED;
                }
                other => {
                    if command.is_some() {
                        eprintln!("REFUSED: unexpected argument {other:?}");
                        return guard::EXIT_REFUSED;
                    }
                    command = Some(other.to_string());
                }
            }
        }
        i += 1;
    }

    let root = repo.unwrap_or_else(guard::repo_root);
    match command.as_deref() {
        Some("manifest") => {
            let out = match out {
                Some(out) => out,
                None => {
                    eprintln!("REFUSED: manifest needs --out PATH");
                    return guard::EXIT_REFUSED;
                }
            };
            let tag = tag.unwrap_or_else(|| DEFAULT_TAG.to_string());
            let runtime_paths = if runtime { Some(&paths) } else { None };
            let effective = EffectiveConfig::capture(RunFlags {
                mode,
                stamp,
                force,
                slice: config::slice_on(),
            });
            let closure = match build_with_config(&root, &tag, runtime_paths, Some(effective)) {
                Ok(closure) => closure,
                Err(e) => {
                    eprintln!("REFUSED: {e}");
                    return guard::EXIT_REFUSED;
                }
            };
            let target = PathBuf::from(&out);
            if let Some(parent) = target.parent() {
                if !parent.as_os_str().is_empty() {
                    let _ = fs::create_dir_all(parent);
                }
            }
            match fs::write(&target, render(&closure).as_bytes()) {
                Ok(()) => {
                    println!(
                        "wrote {out}: {} source files, source_digest {}, {} runtime digests, repo_commit {}",
                        closure.source.len(),
                        closure.source_digest,
                        closure.runtime_entries(),
                        closure.repo_commit
                    );
                    guard::EXIT_ALLOWED
                }
                Err(e) => {
                    eprintln!("REFUSED: cannot write {out}: {e}");
                    guard::EXIT_REFUSED
                }
            }
        }
        Some("verify") => {
            let manifest = match manifest {
                Some(manifest) => manifest,
                None => {
                    eprintln!("REFUSED: verify needs --manifest PATH");
                    return guard::EXIT_REFUSED;
                }
            };
            let closure = match load(Path::new(&manifest)) {
                Ok(closure) => closure,
                Err(e) => {
                    eprintln!("REFUSED: {e}");
                    return guard::EXIT_REFUSED;
                }
            };
            if let Err(e) = verify_source(&root, &closure) {
                eprintln!("REFUSED: {e}");
                return guard::EXIT_REFUSED;
            }
            println!(
                "OK: source closure verified: {} files, source_digest {}",
                closure.source.len(),
                closure.source_digest
            );
            match &closure.configuration {
                Some(config) => match config::verify_live(config) {
                    Ok(()) => println!(
                        "OK: effective configuration verified: {} TEKMERION_* variables",
                        config.env().len()
                    ),
                    Err(e) => {
                        eprintln!("REFUSED: {e}");
                        return guard::EXIT_REFUSED;
                    }
                },
                None => println!(
                    "{}",
                    configuration_warning(&closure)
                        .expect("a configuration-less closure has a warning")
                ),
            }
            if runtime {
                report(
                    verify_runtime(&root, &closure),
                    "runtime closure verified against the recorded build",
                )
            } else {
                guard::EXIT_ALLOWED
            }
        }
        Some("check-tag") => {
            let manifest = match manifest {
                Some(manifest) => manifest,
                None => {
                    eprintln!("REFUSED: check-tag needs --manifest PATH");
                    return guard::EXIT_REFUSED;
                }
            };
            let closure = match load(Path::new(&manifest)) {
                Ok(closure) => closure,
                Err(e) => {
                    eprintln!("REFUSED: {e}");
                    return guard::EXIT_REFUSED;
                }
            };
            let rel = match relative_to_root(&root, Path::new(&manifest)) {
                Ok(rel) => rel,
                Err(e) => {
                    eprintln!("REFUSED: {e}");
                    return guard::EXIT_REFUSED;
                }
            };
            match check_tag(&root, &closure, &rel) {
                Ok(commit) => {
                    println!(
                        "OK: tag {} resolves to {commit}; source_digest {} matches the manifest committed at the tag",
                        closure.tag, closure.source_digest
                    );
                    guard::EXIT_ALLOWED
                }
                Err(e) => {
                    eprintln!("REFUSED: {e}");
                    guard::EXIT_REFUSED
                }
            }
        }
        Some(other) => {
            eprintln!("REFUSED: unknown command {other:?}");
            guard::EXIT_REFUSED
        }
        None => {
            usage();
            guard::EXIT_REFUSED
        }
    }
}
