//! Refusal-site mutation harness for the Python held-out guard (Ordeal O9.3).
//!
//! Rust replacement for the earlier Python harness. AST-free by design: it
//! finds `raise HeldOutRefused(...)` spans with a balanced-paren scanner that
//! skips string literals, replaces each with `pass  # MUTATED`, runs the test
//! suite, and restores the file. **Not on the verdict path.**
//!
//! Exit 0 only when every surviving mutant is a recorded equivalent.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const TARGET: &str = "raise HeldOutRefused(";
const BACKUP_SUFFIX: &str = ".orig";

// Equivalent mutants, keyed by (line, normalised source line). The guard
// refuses via a downstream check in each case. An edit that shifts these must
// be re-reviewed; the tool then fails rather than inheriting the exemption.
const EQUIVALENT: &[(usize, &str)] = &[
    (271, "raise HeldOutRefused(\"empty path\")"),
    (520, "raise HeldOutRefused( f\"working tree has changes to tracked files under tools/: {first}\")"),
    (607, "raise HeldOutRefused(\"empty path\")"),
    (632, "raise HeldOutRefused(\"empty path\")"),
    (640, "raise HeldOutRefused( f\"held-out marker {marker!r} in proof path {path}\")"),
    (647, "raise HeldOutRefused(\"empty path\")"),
];

struct Restore {
    path: PathBuf,
    original: Vec<u8>,
}

impl Drop for Restore {
    fn drop(&mut self) {
        let _ = fs::write(&self.path, &self.original);
    }
}

fn skip_string(b: &[u8], mut i: usize) -> usize {
    let quote = b[i];
    i += 1;
    while i < b.len() {
        if b[i] == b'\\' {
            i += 2;
            continue;
        }
        if b[i] == quote {
            return i + 1;
        }
        i += 1;
    }
    i
}

/// (start, end, line) of each refusal statement, including a trailing `from …`.
fn find_sites(src: &[u8]) -> Vec<(usize, usize, usize)> {
    let mut out = Vec::new();
    let needle = TARGET.as_bytes();
    let mut i = 0;
    while let Some(p) = src[i..].windows(needle.len()).position(|w| w == needle) {
        let start = i + p;
        let mut j = start + needle.len() - 1; // at '('
        let mut depth = 0i32;
        while j < src.len() {
            match src[j] {
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        j += 1;
                        break;
                    }
                }
                b'"' | b'\'' => {
                    j = skip_string(src, j);
                    continue;
                }
                _ => {}
            }
            j += 1;
        }
        // Extend over `from <expr>` to the end of the line.
        let mut k = j;
        while k < src.len() && (src[k] == b' ' || src[k] == b'\t') {
            k += 1;
        }
        if src[k..].starts_with(b"from ") {
            while k < src.len() && src[k] != b'\n' {
                if src[k] == b'"' || src[k] == b'\'' {
                    k = skip_string(src, k);
                } else {
                    k += 1;
                }
            }
            j = k;
        }
        let line = 1 + src[..start].iter().filter(|&&c| c == b'\n').count();
        out.push((start, j, line));
        i = j;
    }
    out
}

fn span_text(src: &[u8], start: usize, end: usize) -> String {
    String::from_utf8_lossy(&src[start..end])
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn run_tests(root: &Path, tests: &[String]) -> bool {
    let mut cmd = Command::new("python3");
    cmd.args(["-m", "pytest", "-x", "-q"])
        .args(tests)
        .current_dir(root);
    matches!(cmd.status(), Ok(s) if s.success())
}

pub fn run(args: &[String]) -> i32 {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target = args
        .iter()
        .position(|a| a == "--file")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("tools/heldout_guard.py"));
    let tests: Vec<String> = match args.iter().position(|a| a == "--tests") {
        Some(i) => args[i + 1..]
            .iter()
            .take_while(|a| !a.starts_with("--"))
            .cloned()
            .collect(),
        None => vec![
            "tools/test_heldout_modes.py".to_string(),
            "tools/test_guard_refusals.py".to_string(),
        ],
    };

    let backup = target.with_extension(format!("py{BACKUP_SUFFIX}"));
    if backup.exists() {
        eprintln!(
            "REFUSING: stale backup {} from an interrupted run; move it back first",
            backup.display()
        );
        return 2;
    }
    let original = match fs::read(&target) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("ERROR cannot read {}: {e}", target.display());
            return 3;
        }
    };
    let sites = find_sites(&original);
    if sites.is_empty() {
        eprintln!("no {TARGET} sites found");
        return 2;
    }
    fs::write(&backup, &original).ok();
    let _restore = Restore {
        path: target.clone(),
        original: original.clone(),
    };

    let mut killed = 0usize;
    let mut survivors: Vec<(usize, String)> = Vec::new();
    for (start, end, line) in &sites {
        let mut mutant = original.clone();
        mutant.splice(*start..*end, b"pass  # MUTATED".iter().copied());
        if fs::write(&target, &mutant).is_err() {
            eprintln!("ERROR cannot write {}", target.display());
            return 3;
        }
        let passed = run_tests(&root, &tests);
        if passed {
            let snippet = span_text(&original, *start, *end);
            println!("SURVIVED line {line:4}  {snippet}");
            survivors.push((*line, snippet));
        } else {
            killed += 1;
        }
        let _ = fs::write(&target, &original);
    }
    let _ = fs::remove_file(&backup);

    println!("\nguard refusal sites: {}", sites.len());
    println!("killed: {killed}  survived: {}", survivors.len());
    println!(
        "mutation score: {killed}/{} = {:.3}",
        sites.len(),
        killed as f64 / sites.len() as f64
    );

    let mut unexplained = Vec::new();
    for (line, snippet) in &survivors {
        if EQUIVALENT.iter().any(|(l, s)| l == line && s == snippet) {
            println!("EQUIVALENT line {line}: {snippet}");
        } else {
            unexplained.push((line, snippet));
        }
    }
    if !unexplained.is_empty() {
        println!("\nUNEXPLAINED SURVIVORS (add a test, or record an equivalence):");
        for (line, snippet) in unexplained {
            println!("  line {line}: {snippet}");
        }
        return 1;
    }
    println!("all survivors are recorded equivalent mutants");
    0
}
