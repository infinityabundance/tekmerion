//! Fail-closed modes for held-out evaluation targets.
//!
//! Three controls are deliberately kept separate:
//!
//! `develop` (the default)
//!     Development tooling must not read evaluation targets. Any path or task id
//!     whose curve or family is held out is refused, and the guard fails closed:
//!     a missing file, a directory, an unreadable file, a path outside the
//!     allowed root, or any unexpected error is a refusal, not an allow.
//!
//! `sealed-eval`
//!     The sealed run must be *able* to generate answers for the declared
//!     held-out task ids and their ELF objects, and still must never read their
//!     proof text. This mode requires an explicit manifest and a freeze tag. It
//!     allows exactly the task ids listed in the manifest and the objects those
//!     ids name, refuses every proof `.ml` of a sealed task, and refuses to run
//!     at all unless the tag resolves to the manifest's freeze commit, the
//!     manifest's optional `freeze_tag` agrees, and `tools/` is the frozen code.
//!
//! `spent`
//!     Development data that has already been disclosed is *spent*: answering it
//!     again is no longer a blind evaluation, but the guard still must not be
//!     disabled wholesale. `--spent --spent-manifest PATH` allows exactly the
//!     task ids recorded in that manifest and their objects, and still refuses
//!     to open any proof `.ml` for them. The manifest must state, per id, why
//!     the id is spent, and carry a `decision` citing `research/receipts/`.
//!
//! Held-out markers are matched case-insensitively against the requested path
//! and its fully resolved path, so a symlink with an innocent name does not
//! slip through. Defaults: `25519`, `secp256k1`, `p256k1`, `sm2`. Markers can
//! be added (never removed) with `--held-out`.
//!
//! Exit status: 0 allowed, 2 refused (fail closed).

use crate::tools::json::{self, Json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

/// The always-present held-out markers.
pub const DEFAULT_HELD_OUT: &[&str] = &["25519", "secp256k1", "p256k1", "sm2"];

pub const EXIT_ALLOWED: i32 = 0;
pub const EXIT_REFUSED: i32 = 2;

const DECISION_CITATION: &str = "research/receipts/";
const MANIFEST_KEYS: &[&str] = &["freeze_commit", "freeze_tag", "sealed_ids", "note"];
const SPENT_MANIFEST_KEYS: &[&str] = &["decision", "spent", "note"];

/// Raised instead of opening a held-out or otherwise unsafe path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused(pub String);

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Refused {}

impl Refused {
    fn new(msg: impl Into<String>) -> Refused {
        Refused(msg.into())
    }
}

pub type Result<T> = std::result::Result<T, Refused>;

// ---------------------------------------------------------------------------
// Path primitives
// ---------------------------------------------------------------------------

fn lexical_absolute(path: &Path) -> PathBuf {
    let mut out = if path.is_absolute() {
        PathBuf::new()
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    };
    for c in path.components() {
        match c {
            Component::RootDir => {
                out.clear();
                out.push("/");
            }
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(part) => out.push(part),
            Component::Prefix(_) => {}
        }
    }
    out
}

/// `os.path.realpath` with `strict=False`: resolve symlinks as far as the path
/// exists, then append the remaining components lexically.
pub fn realpath(path: &Path) -> PathBuf {
    let absolute = lexical_absolute(path);
    let mut probe = absolute.clone();
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    loop {
        if let Ok(canonical) = fs::canonicalize(&probe) {
            let mut out = canonical;
            for name in tail.iter().rev() {
                out.push(name);
            }
            return out;
        }
        let name = match probe.file_name() {
            Some(n) => n.to_os_string(),
            None => return absolute,
        };
        match probe.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => {
                tail.push(name);
                probe = parent.to_path_buf();
            }
            _ => return absolute,
        }
    }
}

fn has_component(path: &Path, name: &str) -> bool {
    path.components()
        .any(|c| matches!(c, Component::Normal(p) if p == name))
}

fn file_name(path: &Path) -> Option<&str> {
    path.file_name().and_then(|n| n.to_str())
}

fn grandparent_name(path: &Path) -> Option<&str> {
    path.parent().and_then(|p| p.parent()).and_then(file_name)
}

fn under_root(path: &Path, root: &Path) -> bool {
    path == root || path.starts_with(root)
}

fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Path and task-id primitives
// ---------------------------------------------------------------------------

/// A task id in the declared sealed-set format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskId {
    pub arch: String,
    pub op: String,
    pub family: String,
}

/// Return the first held-out marker in `text`, or `None`.
pub fn marker_in(text: &str, held_out: &[String]) -> Option<String> {
    let low = text.to_lowercase();
    for marker in held_out {
        if low.contains(&marker.to_lowercase()) {
            return Some(marker.clone());
        }
    }
    None
}

pub fn canonical_task_id(arch: &str, op: &str, family: &str) -> String {
    format!(
        "{arch}.bignum_{op}_{family}.BIGNUM_{}_{}_CORRECT",
        op.to_uppercase(),
        family.to_uppercase()
    )
}

/// Match a declared sealed-set id, or return `None`.
///
/// A match implies the theorem name is consistent with the operation and
/// family; this is the structural half of the manifest validation.
pub fn parse_task_id(task: &str) -> Option<TaskId> {
    let (arch, rest) = task.split_once('.')?;
    if arch != "arm" && arch != "x86" {
        return None;
    }
    if rest.matches('.').count() != 1 {
        return None;
    }
    let (base, theorem) = rest.split_once('.')?;
    let body = base.strip_prefix("bignum_")?;
    let (op, family) = body.split_once('_')?;
    if !matches!(op, "add" | "sub" | "neg") {
        return None;
    }
    if family.is_empty()
        || family.len() > 32
        || !family
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
    {
        return None;
    }
    let expected = format!(
        "BIGNUM_{}_{}_CORRECT",
        op.to_uppercase(),
        family.to_uppercase()
    );
    if theorem != expected {
        return None;
    }
    Some(TaskId {
        arch: arch.to_string(),
        op: op.to_string(),
        family: family.to_string(),
    })
}

fn parse_bignum_file(name: &str, ext: &str) -> Option<(String, String)> {
    let stem = name.strip_suffix(ext)?;
    let body = stem.strip_prefix("bignum_")?;
    let (op, family) = body.split_once('_')?;
    if !matches!(op, "add" | "sub" | "neg") {
        return None;
    }
    if family.is_empty()
        || !family
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
    {
        return None;
    }
    Some((op.to_string(), family.to_string()))
}

/// The sealed id an ELF object path would answer, or `None`.
///
/// Only `<arch>/<dir>/bignum_<op>_<family>.o` under an `arm`/`x86` tree is
/// recognised. A path under a `proofs` tree is never an object.
pub fn task_id_for_object(path: &Path) -> Option<String> {
    let real = realpath(path);
    if has_component(&real, "proofs") {
        return None;
    }
    let (op, family) = parse_bignum_file(file_name(&real)?, ".o")?;
    let arch = grandparent_name(&real)?;
    if arch != "arm" && arch != "x86" {
        return None;
    }
    Some(canonical_task_id(arch, &op, &family))
}

/// The sealed id a proof-file path would belong to, or `None`.
pub fn task_id_for_proof(path: &Path) -> Option<String> {
    let real = realpath(path);
    let (op, family) = parse_bignum_file(file_name(&real)?, ".ml")?;
    let arch = grandparent_name(&real)?;
    if arch != "arm" && arch != "x86" {
        return None;
    }
    Some(canonical_task_id(arch, &op, &family))
}

fn looks_like_proof(path: &Path) -> bool {
    let real = realpath(path);
    real.extension().and_then(|e| e.to_str()) == Some("ml") || has_component(&real, "proofs")
}

/// Stat/root/non-empty checks shared by develop and sealed allowances.
///
/// Marker checking is the caller's job; this helper only refuses paths that
/// are not readable regular non-empty files inside `root`.
pub fn check_regular(path: &Path, root: Option<&Path>) -> Result<PathBuf> {
    if path.as_os_str().is_empty() {
        return Err(Refused::new("empty path"));
    }
    let meta = fs::metadata(path)
        .map_err(|e| Refused::new(format!("cannot stat {}: {e}", path.display())))?;
    if !meta.is_file() {
        return Err(Refused::new(format!(
            "not a regular file: {}",
            path.display()
        )));
    }
    if let Some(root) = root {
        let real = realpath(path);
        let real_root = realpath(root);
        if !under_root(&real, &real_root) {
            return Err(Refused::new(format!(
                "outside allowed root {root:?}: {}",
                real.display()
            )));
        }
    }
    if meta.len() == 0 {
        return Err(Refused::new(format!("empty file: {}", path.display())));
    }
    Ok(realpath(path))
}

/// Validate a path without opening it. This is the unreduced develop-mode
/// behaviour: every marker is refused.
pub fn check_path(path: &Path, held_out: &[String], root: Option<&Path>) -> Result<PathBuf> {
    if held_out.is_empty() {
        return Err(Refused::new("empty held-out list; refusing by default"));
    }
    if path.as_os_str().is_empty() {
        return Err(Refused::new("empty path"));
    }
    for hay in [path.to_path_buf(), realpath(path), lexical_absolute(path)] {
        let text = hay.display().to_string();
        if let Some(marker) = marker_in(&text, held_out) {
            return Err(Refused::new(format!(
                "held-out marker {marker:?} in {text}"
            )));
        }
    }
    check_regular(path, root)
}

/// Validate, then read. The check always runs before the read.
pub fn open_guarded(path: &Path, held_out: &[String], root: Option<&Path>) -> Result<Vec<u8>> {
    check_path(path, held_out, root)?;
    fs::read(path).map_err(|e| Refused::new(format!("cannot read {}: {e}", path.display())))
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(char::from_digit((b >> 4) as u32, 16).unwrap());
        s.push(char::from_digit((b & 0xf) as u32, 16).unwrap());
    }
    s
}

fn count_lines(bytes: &[u8]) -> usize {
    let newlines = bytes.iter().filter(|&&b| b == b'\n').count();
    if !bytes.is_empty() && bytes.last() != Some(&b'\n') {
        newlines + 1
    } else {
        newlines
    }
}

/// Validate, open read-only, and return a digest line.
pub fn describe(path: &Path, root: Option<&Path>, policy: &GuardPolicy) -> Result<String> {
    policy.check_path(path, root)?;
    let bytes =
        fs::read(path).map_err(|e| Refused::new(format!("cannot read {}: {e}", path.display())))?;
    let mut digest = Sha256::new();
    digest.update(&bytes);
    Ok(format!(
        "allowed: {} sha256={} lines={}",
        realpath(path).display(),
        hex(&digest.finalize()),
        count_lines(&bytes)
    ))
}

// ---------------------------------------------------------------------------
// Sealed manifest and freeze verification
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedManifest {
    pub path: PathBuf,
    pub freeze_commit: String,
    pub sealed_ids: BTreeSet<String>,
    pub freeze_tag: Option<String>,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpentManifest {
    /// An allow-list of disclosed development ids, each with its reason.
    pub path: PathBuf,
    pub spent: BTreeMap<String, String>,
    pub decision: String,
    pub note: String,
}

fn read_manifest_json(path: &Path, noun: &str) -> Result<Json> {
    let bytes = fs::read(path).map_err(|e| {
        Refused::new(format!(
            "cannot parse {noun} manifest {}: {e}",
            path.display()
        ))
    })?;
    match json::parse(&bytes) {
        Ok(j) => Ok(j),
        Err(e) if e.starts_with("duplicate object key") => {
            Err(Refused::new(format!("duplicate key in {noun} manifest")))
        }
        Err(e) => Err(Refused::new(format!(
            "cannot parse {noun} manifest {}: {e}",
            path.display()
        ))),
    }
}

fn manifest_header(path: &Path, noun: &str) -> Result<()> {
    if path.as_os_str().is_empty() {
        return Err(Refused::new(format!("no {noun} manifest given")));
    }
    if is_symlink(path) {
        return Err(Refused::new(format!(
            "{noun} manifest may not be a symlink: {}",
            path.display()
        )));
    }
    let meta = fs::metadata(path).map_err(|e| {
        Refused::new(format!(
            "cannot stat {noun} manifest {}: {e}",
            path.display()
        ))
    })?;
    if !meta.is_file() {
        return Err(Refused::new(format!(
            "{noun} manifest is not a regular file: {}",
            path.display()
        )));
    }
    if meta.len() == 0 {
        return Err(Refused::new(format!(
            "{noun} manifest is empty: {}",
            path.display()
        )));
    }
    Ok(())
}

fn unknown_keys(payload: &BTreeMap<String, Json>, allowed: &[&str]) -> Vec<String> {
    payload
        .keys()
        .filter(|k| !allowed.contains(&k.as_str()))
        .cloned()
        .collect()
}

fn is_hex40(s: &str) -> bool {
    s.len() == 40
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn valid_tag(tag: &str) -> bool {
    !tag.is_empty() && !tag.chars().any(char::is_whitespace)
}

/// Load and strictly validate a sealed manifest. Fail closed on anything.
pub fn load_sealed_manifest(path: &Path) -> Result<SealedManifest> {
    manifest_header(path, "sealed")?;
    let payload = read_manifest_json(path, "sealed")?;
    let obj = payload
        .as_obj()
        .ok_or_else(|| Refused::new("sealed manifest must be a JSON object"))?;

    let unknown = unknown_keys(obj, MANIFEST_KEYS);
    if !unknown.is_empty() {
        return Err(Refused::new(format!(
            "sealed manifest has unknown keys: {unknown:?}"
        )));
    }

    let commit = match obj.get("freeze_commit") {
        Some(Json::Str(s)) if is_hex40(s) => s.clone(),
        _ => {
            return Err(Refused::new(
                "freeze_commit must be a 40-character lowercase hex sha1",
            ))
        }
    };

    let raw_ids = match obj.get("sealed_ids") {
        Some(Json::Arr(v)) if !v.is_empty() => v,
        _ => return Err(Refused::new("sealed_ids must be a non-empty JSON list")),
    };

    let mut seen = BTreeSet::new();
    for item in raw_ids {
        let id = match item.as_str() {
            Some(s) => s,
            None => {
                return Err(Refused::new(format!(
                    "sealed id is not a string: {}",
                    json_value_label(item)
                )))
            }
        };
        if parse_task_id(id).is_none() {
            return Err(Refused::new(format!(
                "sealed id is not in the declared format: {id:?}"
            )));
        }
        if !seen.insert(id.to_string()) {
            return Err(Refused::new(format!("duplicate sealed id: {id:?}")));
        }
    }

    let freeze_tag = match obj.get("freeze_tag") {
        None | Some(Json::Null) => None,
        Some(Json::Str(tag)) if valid_tag(tag) => Some(tag.clone()),
        other => {
            return Err(Refused::new(format!(
                "freeze_tag is not a valid tag name: {}",
                other.map(json_value_label).unwrap_or_default()
            )))
        }
    };

    let note = match obj.get("note") {
        None => String::new(),
        Some(Json::Str(s)) => s.clone(),
        Some(_) => return Err(Refused::new("note must be a string")),
    };

    Ok(SealedManifest {
        path: realpath(path),
        freeze_commit: commit,
        sealed_ids: seen,
        freeze_tag,
        note,
    })
}

/// Load and strictly validate a spent manifest. Fail closed on anything.
///
/// A spent allowance is a scope decision, not a secrecy one, so no freeze tag
/// is involved; the evidence that the scope decision was made is the `decision`
/// string, which must cite a receipt, and a per-id reason.
pub fn load_spent_manifest(path: &Path) -> Result<SpentManifest> {
    manifest_header(path, "spent")?;
    let payload = read_manifest_json(path, "spent")?;
    let obj = payload
        .as_obj()
        .ok_or_else(|| Refused::new("spent manifest must be a JSON object"))?;

    let unknown = unknown_keys(obj, SPENT_MANIFEST_KEYS);
    if !unknown.is_empty() {
        return Err(Refused::new(format!(
            "spent manifest has unknown keys: {unknown:?}"
        )));
    }

    let decision = match obj.get("decision") {
        Some(Json::Str(s)) if !s.trim().is_empty() => s.clone(),
        _ => return Err(Refused::new("spent manifest needs a non-empty decision")),
    };
    if !decision.contains(DECISION_CITATION) {
        return Err(Refused::new(format!(
            "spent manifest decision must cite a path under {DECISION_CITATION}"
        )));
    }

    let raw = match obj.get("spent") {
        Some(Json::Obj(m)) if !m.is_empty() => m,
        _ => return Err(Refused::new("spent must be a non-empty JSON object")),
    };

    let mut spent = BTreeMap::new();
    for (task, reason) in raw {
        if parse_task_id(task).is_none() {
            return Err(Refused::new(format!(
                "spent id is not in the declared format: {task:?}"
            )));
        }
        match reason {
            Json::Str(r) if !r.trim().is_empty() => {
                spent.insert(task.clone(), r.clone());
            }
            _ => return Err(Refused::new(format!("spent id {task:?} has no reason"))),
        }
    }

    let note = match obj.get("note") {
        None => String::new(),
        Some(Json::Str(s)) => s.clone(),
        Some(_) => return Err(Refused::new("note must be a string")),
    };

    Ok(SpentManifest {
        path: realpath(path),
        spent,
        decision,
        note,
    })
}

fn json_value_label(v: &Json) -> String {
    match v {
        Json::Null => "None".to_string(),
        Json::Bool(b) => b.to_string(),
        Json::Num(n) => n.to_string(),
        Json::Str(s) => format!("{s:?}"),
        Json::Arr(_) => "[]".to_string(),
        Json::Obj(_) => "{}".to_string(),
    }
}

fn git(repo: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .arg("--no-optional-locks")
        .args(args)
        .current_dir(repo)
        .output();
    let output = match output {
        Ok(o) => o,
        Err(e) => {
            return Err(Refused::new(format!(
                "cannot run git in {}: {e}",
                repo.display()
            )))
        }
    };
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        let first = detail.trim().lines().next().unwrap_or("").to_string();
        let shown = if first.is_empty() {
            format!("exit {}", output.status.code().unwrap_or(-1))
        } else {
            first
        };
        return Err(Refused::new(format!(
            "git {} failed in {}: {shown}",
            args.join(" "),
            repo.display()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Refuse unless the tag, the manifest commit and the tools tree agree.
pub fn verify_freeze(manifest: &SealedManifest, freeze_tag: &str, repo: &Path) -> Result<()> {
    if !valid_tag(freeze_tag) {
        return Err(Refused::new(format!(
            "--freeze-tag is not a valid tag name: {freeze_tag:?}"
        )));
    }
    if let Some(manifest_tag) = &manifest.freeze_tag {
        if manifest_tag != freeze_tag {
            return Err(Refused::new(format!(
                "manifest freeze_tag {manifest_tag:?} does not match --freeze-tag {freeze_tag:?}"
            )));
        }
    }

    let spec = format!("{freeze_tag}^{{commit}}");
    let got = git(repo, &["rev-parse", "--verify", &spec])?
        .trim()
        .to_lowercase();
    if got != manifest.freeze_commit {
        return Err(Refused::new(format!(
            "tag {freeze_tag:?} resolves to {got:?}, but the manifest records {:?}",
            manifest.freeze_commit
        )));
    }

    let dirty = git(
        repo,
        &[
            "status",
            "--porcelain",
            "--untracked-files=no",
            "--",
            "tools",
        ],
    )?
    .trim()
    .to_string();
    if !dirty.is_empty() {
        let first = dirty.lines().next().unwrap_or("");
        return Err(Refused::new(format!(
            "working tree has changes to tracked files under tools/: {first}"
        )));
    }

    // Cleanliness is not enough: the tree must *be* the frozen code. Without
    // this, a later commit could change the emitters and still be accepted
    // under an older freeze tag.
    let changed = git(
        repo,
        &[
            "diff",
            "--name-only",
            &manifest.freeze_commit,
            "--",
            "tools",
        ],
    )?
    .trim()
    .to_string();
    if !changed.is_empty() {
        let first = changed.lines().next().unwrap_or("");
        let short: String = manifest.freeze_commit.chars().take(12).collect();
        return Err(Refused::new(format!(
            "tools/ differs from the freezer commit {short}: {first}"
        )));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Policy object shared by the guard command and the emitters
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Develop,
    SealedEval,
    Spent,
}

impl Mode {
    pub fn name(self) -> &'static str {
        match self {
            Mode::Develop => "develop",
            Mode::SealedEval => "sealed-eval",
            Mode::Spent => "spent",
        }
    }

    pub fn parse(name: &str) -> Option<Mode> {
        match name {
            "develop" => Some(Mode::Develop),
            "sealed-eval" => Some(Mode::SealedEval),
            "spent" => Some(Mode::Spent),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Manifest {
    Sealed(SealedManifest),
    Spent(SpentManifest),
}

/// Mode + manifest + markers. Immutable for the life of one process.
#[derive(Debug, Clone)]
pub struct GuardPolicy {
    mode: Mode,
    manifest: Option<Manifest>,
    repo: PathBuf,
    held_out: Vec<String>,
}

impl GuardPolicy {
    pub fn new(
        mode: Mode,
        manifest: Option<Manifest>,
        repo: &Path,
        held_out: &[String],
    ) -> Result<GuardPolicy> {
        if held_out.is_empty() {
            return Err(Refused::new("empty held-out list; refusing by default"));
        }
        match (mode, &manifest) {
            (Mode::SealedEval, Some(Manifest::Sealed(_))) => {}
            (Mode::SealedEval, _) => {
                return Err(Refused::new("sealed-eval requires a sealed manifest"))
            }
            (Mode::Spent, Some(Manifest::Spent(_))) => {}
            (Mode::Spent, _) => return Err(Refused::new("spent requires a spent manifest")),
            (Mode::Develop, _) => {}
        }
        Ok(GuardPolicy {
            mode,
            manifest,
            repo: repo.to_path_buf(),
            held_out: held_out.to_vec(),
        })
    }

    /// Construct from a mode *name*, for callers that parse the mode from text.
    pub fn new_named(
        mode: &str,
        manifest: Option<Manifest>,
        repo: &Path,
        held_out: &[String],
    ) -> Result<GuardPolicy> {
        let parsed = Mode::parse(mode)
            .ok_or_else(|| Refused::new(format!("unknown guard mode: {mode:?}")))?;
        GuardPolicy::new(parsed, manifest, repo, held_out)
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn repo(&self) -> &Path {
        &self.repo
    }

    pub fn held_out(&self) -> &[String] {
        &self.held_out
    }

    pub fn sealed(&self) -> bool {
        self.mode == Mode::SealedEval
    }

    pub fn spent(&self) -> bool {
        self.mode == Mode::Spent
    }

    /// Does the mode admit any held-out marker at all?
    pub fn admitted(&self) -> bool {
        matches!(self.mode, Mode::SealedEval | Mode::Spent)
    }

    fn manifest_noun(&self) -> &'static str {
        match self.mode {
            Mode::Spent => "spent",
            _ => "sealed",
        }
    }

    fn marker(&self, text: &str) -> Option<String> {
        marker_in(text, &self.held_out)
    }

    fn listed(&self, task: Option<&str>) -> bool {
        if !self.admitted() {
            return false;
        }
        let task = match task {
            Some(t) => t,
            None => return false,
        };
        match &self.manifest {
            Some(Manifest::Sealed(m)) => m.sealed_ids.contains(task),
            Some(Manifest::Spent(m)) => m.spent.contains_key(task),
            None => false,
        }
    }

    /// Allow a task id only if it is not held out or is manifest-listed.
    pub fn check_task(&self, task: &str) -> Result<()> {
        self.check_task_opt(Some(task))
    }

    /// As [`GuardPolicy::check_task`], for callers whose id may be absent.
    pub fn check_task_opt(&self, task: Option<&str>) -> Result<()> {
        let task = match task {
            Some(t) if !t.is_empty() => t,
            other => {
                return Err(Refused::new(format!(
                    "task id is not a non-empty string: {}",
                    match other {
                        None => "None".to_string(),
                        Some(s) => format!("{s:?}"),
                    }
                )))
            }
        };
        let marker = match self.marker(task) {
            Some(m) => m,
            None => return Ok(()),
        };
        if !self.admitted() {
            return Err(Refused::new(format!(
                "held-out marker {marker:?} in task {task:?}"
            )));
        }
        if parse_task_id(task).is_none() {
            return Err(Refused::new(format!(
                "held-out task id is not in the declared task-id format: {task:?}"
            )));
        }
        if !self.listed(Some(task)) {
            return Err(Refused::new(format!(
                "held-out task id {task:?} is not in the {} manifest",
                self.manifest_noun()
            )));
        }
        Ok(())
    }

    /// Allow a held-out ELF object only when its task id is manifest-listed.
    pub fn check_object(&self, path: &Path, root: Option<&Path>) -> Result<PathBuf> {
        if path.as_os_str().is_empty() {
            return Err(Refused::new("empty path"));
        }
        let text = path.display().to_string();
        if looks_like_proof(path) {
            return Err(Refused::new(format!("not an object path: {text}")));
        }
        let marker = match self.marker(&text) {
            Some(m) => m,
            None => return check_path(path, &self.held_out, root),
        };
        if !self.admitted() {
            return Err(Refused::new(format!(
                "held-out marker {marker:?} in {text}"
            )));
        }
        let task = match task_id_for_object(path) {
            Some(t) => t,
            None => {
                return Err(Refused::new(format!(
                    "held-out path is not a declared bignum object: {text}"
                )))
            }
        };
        if !self.listed(Some(&task)) {
            return Err(Refused::new(format!(
                "object {text} belongs to {task:?}, which is not in the {} manifest",
                self.manifest_noun()
            )));
        }
        let root = match root {
            Some(r) => r,
            None => {
                return Err(Refused::new(format!(
                    "{} object allowance for {text} requires an allowed root",
                    self.manifest_noun()
                )))
            }
        };
        check_regular(path, Some(root))
    }

    /// A listed task's proof text is never readable, in any mode.
    pub fn check_proof(&self, path: &Path, root: Option<&Path>) -> Result<PathBuf> {
        if path.as_os_str().is_empty() {
            return Err(Refused::new("empty path"));
        }
        let text = path.display().to_string();
        let task = task_id_for_proof(path);
        if self.listed(task.as_deref()) {
            let shown = match &task {
                Some(t) => format!("'{t}'"),
                None => "None".to_string(),
            };
            return Err(Refused::new(format!(
                "{} proof file is never readable: {text} ({shown})",
                self.manifest_noun()
            )));
        }
        if let Some(marker) = self.marker(&text) {
            return Err(Refused::new(format!(
                "held-out marker {marker:?} in proof path {text}"
            )));
        }
        check_path(path, &self.held_out, root)
    }

    /// Generic path entry point: proofs are never objects.
    pub fn check_path(&self, path: &Path, root: Option<&Path>) -> Result<PathBuf> {
        if path.as_os_str().is_empty() {
            return Err(Refused::new("empty path"));
        }
        let text = path.display().to_string();
        if self.marker(&text).is_some() {
            if looks_like_proof(path) {
                return self.check_proof(path, root);
            }
            return self.check_object(path, root);
        }
        if looks_like_proof(path) {
            return self.check_proof(path, root);
        }
        check_path(path, &self.held_out, root)
    }
}

/// The default held-out markers as owned strings.
pub fn default_held_out() -> Vec<String> {
    DEFAULT_HELD_OUT.iter().map(|s| s.to_string()).collect()
}

/// A develop-mode policy with the default markers.
pub fn develop_policy() -> GuardPolicy {
    GuardPolicy {
        mode: Mode::Develop,
        manifest: None,
        repo: repo_root(),
        held_out: default_held_out(),
    }
}

/// A develop-mode policy over an explicit marker list.
pub fn develop_policy_with(held_out: &[String]) -> GuardPolicy {
    GuardPolicy {
        mode: Mode::Develop,
        manifest: None,
        repo: repo_root(),
        held_out: held_out.to_vec(),
    }
}

/// Load, verify and return a sealed policy; refuses on any mismatch.
pub fn sealed_policy(
    manifest_path: &Path,
    freeze_tag: &str,
    repo: &Path,
    held_out: &[String],
) -> Result<GuardPolicy> {
    let manifest = load_sealed_manifest(manifest_path)?;
    verify_freeze(&manifest, freeze_tag, repo)?;
    GuardPolicy::new(
        Mode::SealedEval,
        Some(Manifest::Sealed(manifest)),
        repo,
        held_out,
    )
}

/// Load and return a spent policy. No freeze tag is involved.
pub fn spent_policy(manifest_path: &Path, repo: &Path, held_out: &[String]) -> Result<GuardPolicy> {
    let manifest = load_spent_manifest(manifest_path)?;
    GuardPolicy::new(Mode::Spent, Some(Manifest::Spent(manifest)), repo, held_out)
}

// ---------------------------------------------------------------------------
// CLI plumbing shared by the emitters
// ---------------------------------------------------------------------------

/// The mode flags every emitter accepts.
#[derive(Debug, Clone, Default)]
pub struct ModeArgs {
    pub develop: bool,
    pub sealed_eval: bool,
    pub sealed_manifest: Option<String>,
    pub freeze_tag: Option<String>,
    pub spent: bool,
    pub spent_manifest: Option<String>,
    pub held_out: Vec<String>,
}

/// Build the policy named by parsed CLI arguments. Fail closed.
pub fn policy_from_args(args: &ModeArgs, repo: &Path) -> Result<GuardPolicy> {
    let mut held_out = default_held_out();
    held_out.extend(args.held_out.iter().cloned());

    let chosen: Vec<&str> = [
        (Mode::Develop.name(), args.develop),
        (Mode::SealedEval.name(), args.sealed_eval),
        (Mode::Spent.name(), args.spent),
    ]
    .iter()
    .filter(|(_, on)| *on)
    .map(|(name, _)| *name)
    .collect();
    if chosen.len() > 1 {
        return Err(Refused::new(format!(
            "guard modes are mutually exclusive: {}",
            chosen.join(", ")
        )));
    }

    if args.spent {
        let path = args
            .spent_manifest
            .as_deref()
            .ok_or_else(|| Refused::new("--spent requires --spent-manifest PATH"))?;
        if args.sealed_manifest.is_some() {
            return Err(Refused::new(
                "--sealed-manifest requires --sealed-eval, not --spent",
            ));
        }
        if args.freeze_tag.is_some() {
            return Err(Refused::new("--freeze-tag requires --sealed-eval"));
        }
        return spent_policy(Path::new(path), repo, &held_out);
    }

    if !args.sealed_eval {
        if args.sealed_manifest.is_some() || args.freeze_tag.is_some() {
            return Err(Refused::new(
                "--sealed-manifest and --freeze-tag require --sealed-eval",
            ));
        }
        if args.spent_manifest.is_some() {
            return Err(Refused::new("--spent-manifest requires --spent"));
        }
        return GuardPolicy::new(Mode::Develop, None, repo, &held_out);
    }
    if args.spent_manifest.is_some() {
        return Err(Refused::new(
            "--spent-manifest requires --spent, not --sealed-eval",
        ));
    }
    let manifest = args
        .sealed_manifest
        .as_deref()
        .ok_or_else(|| Refused::new("--sealed-eval requires --sealed-manifest PATH"))?;
    let tag = args
        .freeze_tag
        .as_deref()
        .ok_or_else(|| Refused::new("--sealed-eval requires --freeze-tag TAG"))?;
    sealed_policy(Path::new(manifest), tag, repo, &held_out)
}

/// The repository root used by the CLI.
///
/// The port walks up from the working directory to a directory holding both
/// `tools/` and `Cargo.toml`, so a binary built once can verify a checked-out
/// freeze the way the Python tool did; it falls back to the crate manifest
/// directory.
pub fn repo_root() -> PathBuf {
    let mut dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    loop {
        if dir.join("tools").is_dir() && dir.join("Cargo.toml").is_file() {
            return dir;
        }
        match dir.parent() {
            Some(parent) if parent != dir => dir = parent.to_path_buf(),
            _ => break,
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

/// A parsed `--flag value` / `--flag=value` argument pair.
pub fn take_value(args: &[String], i: &mut usize, flag: &str) -> Option<String> {
    let arg = &args[*i];
    if let Some(v) = arg.strip_prefix(&format!("{flag}=")) {
        return Some(v.to_string());
    }
    if arg == flag {
        if let Some(v) = args.get(*i + 1) {
            *i += 1;
            return Some(v.clone());
        }
    }
    None
}

pub fn run(argv: &[String]) -> i32 {
    let mut paths: Vec<String> = Vec::new();
    let mut root: Option<PathBuf> = None;
    let mut list = false;
    let mut mode = ModeArgs::default();
    let mut extra_markers: Vec<String> = Vec::new();

    let mut i = 0;
    while i < argv.len() {
        let arg = &argv[i];
        if let Some(v) = take_value(argv, &mut i, "--root") {
            root = Some(PathBuf::from(v));
        } else if let Some(v) = take_value(argv, &mut i, "--held-out") {
            extra_markers.push(v);
        } else if let Some(v) = take_value(argv, &mut i, "--sealed-manifest") {
            mode.sealed_manifest = Some(v);
        } else if let Some(v) = take_value(argv, &mut i, "--freeze-tag") {
            mode.freeze_tag = Some(v);
        } else if let Some(v) = take_value(argv, &mut i, "--spent-manifest") {
            mode.spent_manifest = Some(v);
        } else {
            match arg.as_str() {
                "--develop" => mode.develop = true,
                "--sealed-eval" => mode.sealed_eval = true,
                "--spent" => mode.spent = true,
                "--list" => list = true,
                other if other.starts_with("--") => {
                    eprintln!("REFUSED: unknown option {other}");
                    return EXIT_REFUSED;
                }
                other => paths.push(other.to_string()),
            }
        }
        i += 1;
    }

    let mut markers = default_held_out();
    markers.extend(extra_markers.iter().cloned());

    if list {
        for marker in &markers {
            println!("{marker}");
        }
        return EXIT_ALLOWED;
    }

    mode.held_out = extra_markers;
    let repo = repo_root();
    let policy = match policy_from_args(&mode, &repo) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("REFUSED: {e}");
            return EXIT_REFUSED;
        }
    };

    if paths.is_empty() {
        eprintln!("REFUSED: no path given (fail closed)");
        return EXIT_REFUSED;
    }

    let mut status = EXIT_ALLOWED;
    for path in &paths {
        match describe(Path::new(path), root.as_deref(), &policy) {
            Ok(line) => println!("{line}"),
            Err(e) => {
                status = EXIT_REFUSED;
                eprintln!("REFUSED: {e}");
            }
        }
    }
    status
}
