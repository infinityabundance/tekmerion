//! Refusal-site mutation harness for the Rust held-out guard (Ordeal O9.3).
//!
//! AST-free by design: it finds every `Err(Refused::new(..))` refusal expression
//! in `src/tools/guard.rs` with a balanced-paren scanner that skips string
//! literals and line comments, replaces the expression with `Ok(todo!())` (a
//! well-typed "never refuse" mutant), runs `cargo test --offline --test
//! tools_guard`, and restores the file. A mutant that fails to compile is not a
//! valid mutant and is excluded from the score rather than counted as killed.
//! **Not on the verdict path.**
//!
//! Exit 0 only when every surviving mutant is a recorded equivalent (`?`-site
//! refusals live in closures, which this scanner does not mutate) or when no
//! site survives.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const TARGET_FILE: &str = "src/tools/guard.rs";
const TARGET: &str = "Err(Refused::new(";
const MUTANT: &str = "Ok(todo!())";
const BACKUP_SUFFIX: &str = ".rs.orig";

// Equivalent mutants, keyed by (line, normalised source line). A refusal that
// is unreachable in the ported public surface is listed here with its reason.
// An edit that shifts these must be re-reviewed; the tool then fails rather
// than inheriting the exemption. Empty today: every site is reached and killed.
const EQUIVALENT: &[(usize, &str)] = &[];

struct Restore {
    path: PathBuf,
    original: Vec<u8>,
}

impl Drop for Restore {
    fn drop(&mut self) {
        let _ = fs::write(&self.path, &self.original);
    }
}

fn skip_string(src: &[u8], mut i: usize) -> usize {
    let quote = src[i];
    i += 1;
    while i < src.len() {
        if src[i] == b'\\' {
            i += 2;
            continue;
        }
        if src[i] == quote {
            return i + 1;
        }
        i += 1;
    }
    i
}

fn skip_line_comment(src: &[u8], mut i: usize) -> usize {
    while i < src.len() && src[i] != b'\n' {
        i += 1;
    }
    i
}

/// (start, end, line) of each refusal expression `Err(Refused::new(..))`.
fn find_sites(src: &[u8]) -> Vec<(usize, usize, usize)> {
    let mut out = Vec::new();
    let needle = TARGET.as_bytes();
    let mut i = 0;
    while i < src.len() {
        let p = match src[i..].windows(needle.len()).position(|w| w == needle) {
            Some(p) => i + p,
            None => break,
        };
        let start = p;
        // `TARGET` ends at the '(' of `Refused::new(`.
        let mut j = start + needle.len() - 1;
        let mut depth = 0i32;
        let mut closed = false;
        while j < src.len() {
            match src[j] {
                b'"' => {
                    j = skip_string(src, j);
                    continue;
                }
                b'/' if src.get(j + 1) == Some(&b'/') => {
                    j = skip_line_comment(src, j);
                    continue;
                }
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        j += 1;
                        closed = true;
                        break;
                    }
                }
                _ => {}
            }
            j += 1;
        }
        if !closed {
            i = start + 1;
            continue;
        }
        // `j` is just past `Refused::new(..)`. The enclosing `Err(` closes next.
        let mut k = j;
        while k < src.len()
            && (src[k] == b' ' || src[k] == b'\t' || src[k] == b'\n' || src[k] == b'\r')
        {
            k += 1;
        }
        if src.get(k) != Some(&b')') {
            i = start + 1;
            continue;
        }
        let end = k + 1;
        let line = 1 + src[..start].iter().filter(|&&c| c == b'\n').count();
        out.push((start, end, line));
        i = end;
    }
    out
}

fn span_text(src: &[u8], start: usize, end: usize) -> String {
    String::from_utf8_lossy(&src[start..end])
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn run_tests(root: &Path) -> (bool, String) {
    let out = Command::new("cargo")
        .args(["test", "--offline", "--test", "tools_guard", "--quiet"])
        .current_dir(root)
        .output();
    match out {
        Ok(o) => {
            let mut text = String::from_utf8_lossy(&o.stdout).to_string();
            text.push_str(&String::from_utf8_lossy(&o.stderr));
            (o.status.success(), text)
        }
        Err(e) => (false, format!("cannot run cargo: {e}")),
    }
}

pub fn run(args: &[String]) -> i32 {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target = args
        .iter()
        .position(|a| a == "--file")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join(TARGET_FILE));

    let backup = PathBuf::from(format!("{}{BACKUP_SUFFIX}", target.display()));
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
    let mut invalid = 0usize;
    let mut survivors: Vec<(usize, String)> = Vec::new();
    for (start, end, line) in &sites {
        let mut mutant = original.clone();
        mutant.splice(*start..*end, MUTANT.bytes());
        if fs::write(&target, &mutant).is_err() {
            eprintln!("ERROR cannot write {}", target.display());
            return 3;
        }
        let (passed, output) = run_tests(&root);
        if passed {
            let snippet = span_text(&original, *start, *end);
            println!("SURVIVED line {line:4}  {snippet}");
            survivors.push((*line, snippet));
        } else if output.contains("error[") || output.contains("could not compile") {
            println!("INVALID  line {line:4}  mutant did not compile");
            invalid += 1;
        } else {
            killed += 1;
        }
        let _ = fs::write(&target, &original);
    }
    let _ = fs::remove_file(&backup);

    let valid = sites.len() - invalid;
    println!("\nguard refusal sites: {} ({invalid} invalid)", sites.len());
    println!("killed: {killed}  survived: {}", survivors.len());
    if valid > 0 {
        println!(
            "mutation score: {killed}/{valid} = {:.3}",
            killed as f64 / valid as f64
        );
    }

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
