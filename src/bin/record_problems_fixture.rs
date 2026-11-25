//! Record the committed input fixtures for the emitter parity suite (XVI.A).
//!
//! The suite in `tests/tools_emit.rs` needs the 27 corpus entries, the matching
//! ELF objects and the three Cadical sources. `.bench/` is git-ignored, so none
//! of them exist in a CI checkout. This binary extracts exactly those inputs from
//! a live bench and writes them under `tests/fixtures/`:
//!
//! - `tests/fixtures/problems.json` — the 27 top-level entries, **byte for
//!   byte** as they appear in the live corpus (raw value slices, so the live
//!   agreement test can compare bytes rather than re-serialised values);
//! - `tests/fixtures/bench/s2n-bignum/<arch>/<dir>/bignum_<op>_<family>.o`;
//! - `tests/fixtures/bench/hol-light/Cadical/{dimacs,cnf,ldrup}.ml`.
//!
//! Recording is deliberate and audited: every written file's size and SHA-256 is
//! printed, and the tests refuse to run when the files are absent.
//!
//! Usage:
//!
//! ```text
//! cargo run --release --offline --bin record_problems_fixture -- [--repo DIR] [--fixtures DIR]
//! ```
//!
//! Defaults: `--repo` is the repository root (found from the working directory),
//! `--fixtures` is `<repo>/tests/fixtures`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use tekmerion::tools::{corpus, emit, guard, json, sha256};

fn main() {
    std::process::exit(match record() {
        Ok(()) => 0,
        Err(message) => {
            eprintln!("record_problems_fixture: {message}");
            1
        }
    });
}

fn record() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let repo = match arg_value(&args, "--repo")? {
        Some(path) => PathBuf::from(path),
        None => guard::repo_root(),
    };
    let fixtures = match arg_value(&args, "--fixtures")? {
        Some(path) => PathBuf::from(path),
        None => repo.join("tests/fixtures"),
    };
    reject_unknown(&args)?;

    let problems = write_problems(&repo, &fixtures)?;
    let objects = write_objects(&repo, &fixtures)?;
    let cadical = write_cadical(&repo, &fixtures)?;

    let total = problems
        .bytes
        .saturating_add(objects.iter().map(|w| w.bytes).sum::<u64>())
        .saturating_add(cadical.iter().map(|w| w.bytes).sum::<u64>());
    println!(
        "recorded {} files, {} bytes (problems.json {} bytes, {} object files, {} Cadical sources)",
        objects.len() + cadical.len() + 1,
        total,
        problems.bytes,
        objects.len(),
        cadical.len()
    );
    Ok(())
}

struct Written {
    bytes: u64,
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<Written, String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    let digest = sha256::hex(bytes);
    println!("{}\t{}\t{}", path.display(), bytes.len(), digest);
    Ok(Written {
        bytes: bytes.len() as u64,
    })
}

fn write_problems(repo: &Path, fixtures: &Path) -> Result<Written, String> {
    let live = repo.join(".bench/problems.json");
    let bytes = fs::read(&live).map_err(|e| format!("{}: {e}", live.display()))?;
    let members =
        json::top_level_members(&bytes).map_err(|e| format!("{}: {e}", live.display()))?;
    let mut by_key: BTreeMap<String, &[u8]> = BTreeMap::new();
    for (key, raw) in &members {
        by_key.insert(key.clone(), raw);
    }

    let mut out = String::from("{\n");
    for (index, task) in corpus::tasks().enumerate() {
        let raw = by_key
            .get(task)
            .ok_or_else(|| format!("{}: no top-level entry for {task}", live.display()))?;
        let raw =
            std::str::from_utf8(raw).map_err(|e| format!("{}: {task}: {e}", live.display()))?;
        if index > 0 {
            out.push_str(",\n");
        }
        out.push_str("  ");
        out.push_str(&json_quote(task));
        out.push_str(": ");
        out.push_str(raw);
    }
    out.push_str("\n}\n");
    write_file(&corpus::problems_fixture_in(fixtures), out.as_bytes())
}

fn write_objects(repo: &Path, fixtures: &Path) -> Result<Vec<Written>, String> {
    let s2n = emit::s2n_root(repo);
    let dest_root = fixtures.join("bench/s2n-bignum");
    let mut written = Vec::new();
    for task in corpus::tasks() {
        let parsed = guard::parse_task_id(task)
            .ok_or_else(|| format!("not a declared task id: {task:?}"))?;
        let object = emit::object_for(&parsed.arch, &parsed.op, &parsed.family, repo)
            .map_err(|e| format!("{task}: {e}"))?;
        let rel = object
            .strip_prefix(&s2n)
            .map_err(|_| format!("{} is not under {}", object.display(), s2n.display()))?;
        let bytes = fs::read(&object).map_err(|e| format!("{}: {e}", object.display()))?;
        written.push(write_file(&dest_root.join(rel), &bytes)?);
    }
    Ok(written)
}

fn write_cadical(repo: &Path, fixtures: &Path) -> Result<Vec<Written>, String> {
    let src = repo.join(".bench/hol-light/Cadical");
    let dst = fixtures.join("bench/hol-light/Cadical");
    let mut written = Vec::new();
    for name in corpus::CADICAL_SOURCES {
        let path = src.join(name);
        let bytes = fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        written.push(write_file(&dst.join(name), &bytes)?);
    }
    Ok(written)
}

/// JSON-quote a string. Task ids are plain ASCII, but the recorder should not
/// depend on that if the task set ever grows.
fn json_quote(text: &str) -> String {
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

fn arg_value(args: &[String], name: &str) -> Result<Option<String>, String> {
    let mut found: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        if args[i] == name {
            let value = args
                .get(i + 1)
                .ok_or_else(|| format!("{name} needs a value"))?;
            if found.is_some() {
                return Err(format!("{name} given more than once"));
            }
            found = Some(value.clone());
            i += 2;
        } else {
            i += 1;
        }
    }
    Ok(found)
}

fn reject_unknown(args: &[String]) -> Result<(), String> {
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--repo" | "--fixtures" => i += 2,
            "--help" | "-h" => {
                println!("usage: record_problems_fixture [--repo DIR] [--fixtures DIR]");
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument {other:?}")),
        }
    }
    Ok(())
}
