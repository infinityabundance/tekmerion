//! Refusal-site mutation harness (invariant 5).
//!
//! Rust port of the former `tools/mutate.py`. Every `refuse!(Rnn, ..)` site in
//! `src/lib.rs` and `src/wire.rs` is disabled in turn (replaced with the no-op
//! `skip_refusal!`) inside a scratch copy of the crate, and `cargo test
//! --offline` decides whether the suite notices. A mutant that fails to compile
//! is not a valid mutant and is excluded from the score rather than counted as
//! killed. **Not on the verdict path.**
//!
//! Exit 0 only when no valid mutant survives.

use std::collections::BTreeSet;
use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const FILES: [&str; 2] = ["src/lib.rs", "src/wire.rs"];
const OPEN: &[u8] = b"refuse!(";
const IGNORED_DIRS: [&str; 4] = ["target", ".git", ".bench", "research"];
const TEST_TIMEOUT: Duration = Duration::from_secs(600);
const REPORT: &str = "mutation-report.json";

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Outcome {
    Killed,
    Survived,
    Invalid,
}

impl Outcome {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Outcome::Killed => "KILLED",
            Outcome::Survived => "SURVIVED",
            Outcome::Invalid => "INVALID_MUTANT",
        }
    }
}

/// Standard mutation-testing classification: a non-zero exit with a compile
/// error means the mutant never ran, so it cannot count as killed.
pub(super) fn classify(code: i32, output: &str) -> Outcome {
    if code == 0 {
        Outcome::Survived
    } else if output.contains("error[") || output.contains("could not compile") {
        Outcome::Invalid
    } else {
        Outcome::Killed
    }
}

pub(super) fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || hay.len() < needle.len() {
        return None;
    }
    hay.windows(needle.len()).position(|w| w == needle)
}

/// Scratch directory that removes itself on drop unless it is being kept.
pub(super) struct Scratch {
    dir: PathBuf,
    keep: bool,
}

impl Scratch {
    pub(super) fn new(prefix: &str, keep: bool) -> io::Result<Scratch> {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("{prefix}{}-{nanos}", std::process::id()));
        fs::create_dir_all(&dir)?;
        Ok(Scratch { dir, keep })
    }

    pub(super) fn path(&self) -> &Path {
        &self.dir
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if !self.keep {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }
}

fn wait_with_timeout(child: &mut Child, timeout: Duration) -> io::Result<Option<ExitStatus>> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Some(status));
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Ok(None);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Run `program` with `args`, capturing stdout and stderr into `log`.
///
/// Output goes to a file rather than a pipe so that a chatty child cannot fill
/// a pipe buffer and deadlock the timeout loop.
pub(super) fn run_capture(
    program: &str,
    args: &[&str],
    cwd: &Path,
    env: &[(&str, Option<&str>)],
    log: &Path,
    timeout: Duration,
) -> io::Result<(i32, String)> {
    let out = fs::File::create(log)?;
    let err = out.try_clone()?;
    let mut cmd = Command::new(program);
    cmd.args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::from(out))
        .stderr(Stdio::from(err));
    for (key, value) in env {
        match value {
            Some(v) => cmd.env(key, v),
            None => cmd.env_remove(key),
        };
    }
    let mut child = cmd.spawn()?;
    let status = wait_with_timeout(&mut child, timeout)?;
    let text = fs::read(log)
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_default();
    match status {
        Some(s) => Ok((s.code().unwrap_or(-1), text)),
        None => Err(io::Error::new(
            io::ErrorKind::TimedOut,
            format!("timed out after {}s", timeout.as_secs()),
        )),
    }
}

/// Python's `json.dumps` renders whole floats as `1.0`, not `1`.
pub(super) fn py_float(x: f64) -> String {
    if !x.is_finite() {
        return "null".to_string();
    }
    if x == x.trunc() {
        format!("{x:.1}")
    } else {
        format!("{x}")
    }
}

pub(super) fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
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
    out
}

fn is_ignored(name: &str) -> bool {
    IGNORED_DIRS.contains(&name) || name.ends_with(".md") || name.ends_with(".log")
}

#[cfg(unix)]
fn make_symlink(target: &Path, link: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(not(unix))]
fn make_symlink(target: &Path, link: &Path) -> io::Result<()> {
    fs::copy(target, link).map(|_| ())
}

/// Copy the crate, skipping build output, version control, benchmark artefacts,
/// receipts and prose. Symlinks are copied as links so a dangling target does
/// not fail the copy.
fn copy_tree(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name();
        if is_ignored(&name.to_string_lossy()) {
            continue;
        }
        let from = entry.path();
        let to = dst.join(&name);
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            make_symlink(&fs::read_link(&from)?, &to)?;
        } else if kind.is_dir() {
            copy_tree(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

fn sites_in(text: &str) -> Vec<String> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(p) = find(&b[i..], OPEN) {
        let at = i + p;
        let mut j = at + OPEN.len();
        while j < b.len() && b[j].is_ascii_whitespace() {
            j += 1;
        }
        if j + 3 <= b.len()
            && b[j] == b'R'
            && b[j + 1].is_ascii_digit()
            && b[j + 2].is_ascii_digit()
        {
            out.push(String::from_utf8_lossy(&b[j..j + 3]).into_owned());
        }
        i = at + OPEN.len();
    }
    out
}

fn discover_sites(root: &Path) -> io::Result<BTreeSet<String>> {
    let mut sites = BTreeSet::new();
    for rel in FILES {
        let text = fs::read_to_string(root.join(rel))?;
        sites.extend(sites_in(&text));
    }
    Ok(sites)
}

/// True if `refuse!(<site>,` still occurs (whitespace allowed around the site).
fn contains_site(text: &str, site: &str) -> bool {
    let b = text.as_bytes();
    let needle = site.as_bytes();
    let mut i = 0;
    while let Some(p) = find(&b[i..], OPEN) {
        let at = i + p;
        let mut j = at + OPEN.len();
        while j < b.len() && b[j].is_ascii_whitespace() {
            j += 1;
        }
        if b[j..].starts_with(needle) {
            let mut k = j + needle.len();
            while k < b.len() && b[k].is_ascii_whitespace() {
                k += 1;
            }
            if b.get(k) == Some(&b',') {
                return true;
            }
        }
        i = at + OPEN.len();
    }
    false
}

/// Replace every `refuse!(<site>,` with `skip_refusal!(<site>,`, dropping any
/// whitespace that sat between the macro name, the site and the comma.
fn disable_site(text: &str, site: &str) -> String {
    let b = text.as_bytes();
    let needle = site.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < b.len() {
        let Some(p) = find(&b[i..], OPEN) else {
            out.push_str(&text[i..]);
            break;
        };
        let at = i + p;
        let mut j = at + OPEN.len();
        while j < b.len() && b[j].is_ascii_whitespace() {
            j += 1;
        }
        if b[j..].starts_with(needle) {
            let mut k = j + needle.len();
            while k < b.len() && b[k].is_ascii_whitespace() {
                k += 1;
            }
            if b.get(k) == Some(&b',') {
                out.push_str(&text[i..at]);
                out.push_str("skip_refusal!(");
                out.push_str(site);
                out.push(',');
                i = k + 1;
                continue;
            }
        }
        out.push_str(&text[i..at + OPEN.len()]);
        i = at + OPEN.len();
    }
    out
}

fn render_report(results: &[(String, Outcome)]) -> String {
    let valid = results
        .iter()
        .filter(|(_, o)| *o != Outcome::Invalid)
        .count();
    let killed = results
        .iter()
        .filter(|(_, o)| *o == Outcome::Killed)
        .count();
    let survived = results
        .iter()
        .filter(|(_, o)| *o == Outcome::Survived)
        .count();

    let mut body = String::from("{\n");
    if results.is_empty() {
        body.push_str("  \"sites\": {},\n");
    } else {
        body.push_str("  \"sites\": {");
        for (i, (site, outcome)) in results.iter().enumerate() {
            if i > 0 {
                body.push(',');
            }
            body.push_str(&format!(
                "\n    \"{}\": \"{}\"",
                json_escape(site),
                outcome.as_str()
            ));
        }
        body.push_str("\n  },\n");
    }
    body.push_str(&format!("  \"valid_mutants\": {valid},\n"));
    body.push_str(&format!("  \"killed\": {killed},\n"));
    body.push_str(&format!("  \"survived\": {survived},\n"));
    body.push_str(&format!(
        "  \"invalid_mutants\": {},\n",
        results.len() - valid
    ));
    let score = if valid == 0 {
        "null".to_string()
    } else {
        py_float(killed as f64 / valid as f64)
    };
    body.push_str(&format!("  \"mutation_score\": {score}\n}}\n"));
    body
}

pub fn run(args: &[String]) -> i32 {
    let keep = args.iter().any(|a| a == "--keep");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let sites = match discover_sites(&root) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ERROR cannot read refusal sites: {e}");
            return 2;
        }
    };
    if sites.is_empty() {
        eprintln!("no refusal sites found");
        return 2;
    }

    let scratch = match Scratch::new("tekmerion-mutate-", keep) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ERROR cannot create scratch directory: {e}");
            return 2;
        }
    };
    let target = scratch.path().join("target");
    let target_env = target.to_string_lossy().into_owned();

    let mut results: Vec<(String, Outcome)> = Vec::new();
    for site in &sites {
        let work = scratch.path().join(site);
        if let Err(e) = copy_tree(&root, &work) {
            eprintln!("ERROR cannot copy crate to {}: {e}", work.display());
            return 2;
        }

        for rel in FILES {
            let path = work.join(rel);
            let text = match fs::read_to_string(&path) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("ERROR cannot read {}: {e}", path.display());
                    return 2;
                }
            };
            let mutated = disable_site(&text, site);
            if mutated != text {
                if let Err(e) = fs::write(&path, &mutated) {
                    eprintln!("ERROR cannot write {}: {e}", path.display());
                    return 2;
                }
            }
        }

        let mut check = String::new();
        for rel in FILES {
            let path = work.join(rel);
            match fs::read_to_string(&path) {
                Ok(t) => check.push_str(&t),
                Err(e) => {
                    eprintln!("ERROR cannot read {}: {e}", path.display());
                    return 2;
                }
            }
        }
        if contains_site(&check, site) {
            eprintln!("site {site} was not fully disabled");
            return 2;
        }

        let log = scratch.path().join(format!("{site}.log"));
        let (code, output) = match run_capture(
            "cargo",
            &["test", "--offline", "--quiet"],
            &work,
            &[("CARGO_TARGET_DIR", Some(target_env.as_str()))],
            &log,
            TEST_TIMEOUT,
        ) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("ERROR running the suite for {site}: {e}");
                return 2;
            }
        };
        let outcome = classify(code, &output);
        println!("{site}: {}", outcome.as_str());
        let _ = io::stdout().flush();
        results.push((site.clone(), outcome));
    }

    if keep {
        println!("scratch kept at {}", scratch.path().display());
    }

    let valid: Vec<&(String, Outcome)> = results
        .iter()
        .filter(|(_, o)| *o != Outcome::Invalid)
        .collect();
    let killed = valid.iter().filter(|(_, o)| *o == Outcome::Killed).count();
    let survived = valid
        .iter()
        .filter(|(_, o)| *o == Outcome::Survived)
        .count();

    println!();
    println!("refusal sites: {}", results.len());
    println!(
        "valid mutants: {}  killed: {killed}  survived: {survived}",
        valid.len()
    );
    if !valid.is_empty() {
        println!(
            "mutation score: {killed}/{} = {:.3}",
            valid.len(),
            killed as f64 / valid.len() as f64
        );
    }

    let report = root.join(REPORT);
    if let Err(e) = fs::write(&report, render_report(&results)) {
        eprintln!("ERROR cannot write {}: {e}", report.display());
        return 2;
    }

    if survived == 0 {
        0
    } else {
        1
    }
}
