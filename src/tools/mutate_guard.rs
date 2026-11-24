//! Refusal-site mutation harness for the Rust held-out guard and freeze
//! closure (Ordeals O9.3 and XIII.C).
//!
//! AST-free by design: it finds every `Err(Refused::new(..))` refusal expression
//! in a target file with a balanced-paren scanner that skips string literals
//! and line comments, replaces the expression with `Ok(todo!())` (a well-typed
//! "never refuse" mutant), runs the integration test target that exercises that
//! module, and restores the file. A mutant that fails to compile is not a valid
//! mutant and is excluded from the score rather than counted as killed.
//! **Not on the verdict path.**
//!
//! Exit 0 only when every surviving mutant is a recorded equivalent (`?`-site
//! refusals live in closures and `map_err` plumbing, which this scanner does not
//! mutate) or when no site survives.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const TARGET: &str = "Err(Refused::new(";
const MUTANT: &str = "Ok(todo!())";
const BACKUP_SUFFIX: &str = ".rs.orig";

struct Job {
    file: String,
    /// The integration test target that exercises this file's refusal sites.
    test: &'static str,
}

/// Every module whose refusal sites this harness mutates.
fn jobs() -> Vec<Job> {
    vec![
        Job {
            file: "src/tools/guard.rs".to_string(),
            test: "tools_guard",
        },
        Job {
            file: "src/tools/freeze.rs".to_string(),
            test: "tools_freeze",
        },
    ]
}

// Equivalent mutants, keyed by (file, line, normalised source line). A refusal
// that is unreachable in the ported public surface is listed here with its
// reason. An edit that shifts these must be re-reviewed; the tool then fails
// rather than inheriting the exemption. Empty today: every site is reached and
// killed.
const EQUIVALENT: &[(&str, usize, &str)] = &[];

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

fn run_tests(root: &Path, test: &str) -> (bool, String) {
    let out = Command::new("cargo")
        .args(["test", "--offline", "--test", test, "--quiet"])
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

struct Tally {
    sites: usize,
    killed: usize,
    invalid: usize,
    survivors: Vec<(usize, String)>,
}

fn mutate_file(root: &Path, job: &Job) -> Option<Tally> {
    let target = root.join(&job.file);
    let backup = PathBuf::from(format!("{}{BACKUP_SUFFIX}", target.display()));
    if backup.exists() {
        eprintln!(
            "REFUSING: stale backup {} from an interrupted run; move it back first",
            backup.display()
        );
        return None;
    }
    let original = match fs::read(&target) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("ERROR cannot read {}: {e}", target.display());
            return None;
        }
    };
    let sites = find_sites(&original);
    if sites.is_empty() {
        eprintln!("no {TARGET} sites found in {}", job.file);
        return None;
    }
    fs::write(&backup, &original).ok();
    let _restore = Restore {
        path: target.clone(),
        original: original.clone(),
    };

    let mut tally = Tally {
        sites: sites.len(),
        killed: 0,
        invalid: 0,
        survivors: Vec::new(),
    };
    for (start, end, line) in &sites {
        let mut mutant = original.clone();
        mutant.splice(*start..*end, MUTANT.bytes());
        if fs::write(&target, &mutant).is_err() {
            eprintln!("ERROR cannot write {}", target.display());
            return None;
        }
        let (passed, output) = run_tests(root, job.test);
        if passed {
            let snippet = span_text(&original, *start, *end);
            println!("SURVIVED {} line {line:4}  {snippet}", job.file);
            tally.survivors.push((*line, snippet));
        } else if output.contains("error[") || output.contains("could not compile") {
            println!(
                "INVALID  {} line {line:4}  mutant did not compile",
                job.file
            );
            tally.invalid += 1;
        } else {
            tally.killed += 1;
        }
        let _ = fs::write(&target, &original);
    }
    let _ = fs::remove_file(&backup);
    Some(tally)
}

fn selected_jobs(args: &[String]) -> Option<Vec<Job>> {
    let all = jobs();
    if let Some(i) = args.iter().position(|a| a == "--file") {
        let file = args.get(i + 1)?.clone();
        let test = all
            .iter()
            .find(|job| job.file == file)
            .map(|job| job.test)
            .unwrap_or("tools_guard");
        return Some(vec![Job { file, test }]);
    }
    Some(all)
}

pub fn run(args: &[String]) -> i32 {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let jobs = match selected_jobs(args) {
        Some(jobs) if !jobs.is_empty() => jobs,
        _ => {
            eprintln!("usage: mutate_guard [--file src/tools/<module>.rs]");
            return 2;
        }
    };

    let mut total_sites = 0usize;
    let mut total_killed = 0usize;
    let mut total_invalid = 0usize;
    let mut total_survivors = 0usize;
    let mut unexplained: Vec<(String, usize, String)> = Vec::new();

    for job in &jobs {
        let tally = match mutate_file(&root, job) {
            Some(tally) => tally,
            None => return 3,
        };
        let valid = tally.sites - tally.invalid;
        println!(
            "\n{} refusal sites: {} ({} invalid)",
            job.file, tally.sites, tally.invalid
        );
        println!(
            "killed: {}  survived: {}",
            tally.killed,
            tally.survivors.len()
        );
        if valid > 0 {
            println!(
                "mutation score: {}/{valid} = {:.3}",
                tally.killed,
                tally.killed as f64 / valid as f64
            );
        }
        total_sites += tally.sites;
        total_killed += tally.killed;
        total_invalid += tally.invalid;
        total_survivors += tally.survivors.len();
        for (line, snippet) in tally.survivors {
            let file = job.file.clone();
            if EQUIVALENT
                .iter()
                .any(|(f, l, s)| *f == job.file.as_str() && *l == line && *s == snippet)
            {
                println!("EQUIVALENT {file} line {line}: {snippet}");
            } else {
                unexplained.push((file, line, snippet));
            }
        }
    }

    let valid = total_sites - total_invalid;
    println!(
        "\nall targets: {total_sites} refusal sites ({total_invalid} invalid), \
         killed {total_killed}, survived {total_survivors}"
    );
    if valid > 0 {
        println!(
            "mutation score: {total_killed}/{valid} = {:.3}",
            total_killed as f64 / valid as f64
        );
    }

    if !unexplained.is_empty() {
        println!("\nUNEXPLAINED SURVIVORS (add a test, or record an equivalence):");
        for (file, line, snippet) in unexplained {
            println!("  {file} line {line}: {snippet}");
        }
        return 1;
    }
    println!("all survivors are recorded equivalent mutants");
    0
}
