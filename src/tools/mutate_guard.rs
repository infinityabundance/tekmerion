//! Refusal-site mutation harness for the Rust held-out guard and freeze
//! closure (Ordeals O9.3 and XIII.C).
//!
//! AST-free by design: it finds every `Err(Refused::new(..))` refusal expression
//! in a target file with a balanced-paren scanner that skips string literals
//! and line comments, replaces the expression with a mutant, runs the
//! integration test target that exercises that module, and restores the file. A
//! mutant that fails to compile is not a valid mutant and is excluded from the
//! score rather than counted as killed. **Not on the verdict path.**
//!
//! Two operators are supported, selected by `--operator <name>` or the
//! `MUTATE_GUARD_OPERATOR` environment variable (`reachability` is the
//! default, so an unconfigured run is unchanged):
//!
//! * `reachability` (default) replaces the site with `Ok(todo!())`, a well-typed
//!   "never refuse" mutant that panics. Any test that merely reaches the site
//!   kills it, so the score measures reachability of the refusal, not whether a
//!   test asserts it.
//! * `accept` replaces the site with `Ok(Default::default())`, the real checker
//!   failure: the refusal is silently accepted. It is killed only by a test
//!   that asserts the refusal. Sites whose `Ok` payload has no `Default` impl do
//!   not compile and are reported `INVALID` and excluded from the score.
//!
//! Exit 0 only when every surviving mutant is a recorded equivalent (`?`-site
//! refusals live in closures and `map_err` plumbing, which this scanner does not
//! mutate) or when no site survives.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const TARGET: &str = "Err(Refused::new(";
const REACHABILITY_MUTANT: &str = "Ok(todo!())";
const ACCEPT_MUTANT: &str = "Ok(Default::default())";
const BACKUP_SUFFIX: &str = ".rs.orig";

// The selector (`MUTATE_GUARD_OPERATOR` / `--operator`) is deliberately not
// `TEKMERION_*`: `declared_binary_vars_match_the_source` in
// src/tools/config.rs asserts that every `TEKMERION_*` variable read under
// `src/` is declared in `ENV_VARS`, the freeze-bound configuration set. This
// harness is a developer tool, not on the verdict path, so its switch must not
// enter that set; declaring it there is out of scope for this change.

/// The mutation operator applied to each refusal site.
///
/// * `Reachability` (default) replaces the refusal with `Ok(todo!())`: the
///   mutant is killed by any test that merely *reaches* the site, because the
///   panic is observed even when no assertion checks the refusal.
/// * `Accept` replaces it with `Ok(Default::default())`: the mutant silently
///   accepts what should be refused. It is killed only by a test that actually
///   *asserts* the refusal, which is the property the reachability score does
///   not measure. Sites whose `Ok` payload lacks `Default` fail to compile and
///   are excluded as `INVALID`, never counted as killed.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Operator {
    Reachability,
    Accept,
}

impl Operator {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "reachability" => Ok(Operator::Reachability),
            "accept" => Ok(Operator::Accept),
            other => Err(format!(
                "unknown mutation operator {other:?}; expected reachability|accept"
            )),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Operator::Reachability => "reachability",
            Operator::Accept => "accept",
        }
    }

    fn mutant(self) -> &'static str {
        match self {
            Operator::Reachability => REACHABILITY_MUTANT,
            Operator::Accept => ACCEPT_MUTANT,
        }
    }
}

/// Resolve the operator: `--operator <name>` wins, then the environment, then
/// the `reachability` default.
fn operator_from(args: &[String]) -> Result<Operator, String> {
    if let Some(i) = args.iter().position(|a| a == "--operator") {
        let value = args
            .get(i + 1)
            .ok_or_else(|| "--operator requires a value (reachability|accept)".to_string())?;
        return Operator::parse(value);
    }
    match std::env::var("MUTATE_GUARD_OPERATOR").ok().as_deref() {
        None | Some("") => Ok(Operator::Reachability),
        Some(value) => Operator::parse(value),
    }
}

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

fn mutate_file(root: &Path, job: &Job, operator: Operator) -> Option<Tally> {
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
        mutant.splice(*start..*end, operator.mutant().bytes());
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

fn print_score(operator: Operator, killed: usize, valid: usize) {
    let value = killed as f64 / valid as f64;
    match operator {
        // Byte-identical to the historical single-operator output.
        Operator::Reachability => println!("mutation score: {killed}/{valid} = {value:.3}"),
        Operator::Accept => println!("accept mutation score: {killed}/{valid} = {value:.3}"),
    }
}

pub fn run(args: &[String]) -> i32 {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let operator = match operator_from(args) {
        Ok(op) => op,
        Err(msg) => {
            eprintln!("{msg}");
            return 2;
        }
    };
    // The accept run labels itself so the two scores can never be mistaken for
    // one another. The default reachability run keeps its original output.
    if operator != Operator::Reachability {
        println!(
            "operator: {} (mutant `{}`)",
            operator.name(),
            operator.mutant()
        );
    }
    let jobs = match selected_jobs(args) {
        Some(jobs) if !jobs.is_empty() => jobs,
        _ => {
            eprintln!("usage: mutate_guard [--file src/tools/<module>.rs] [--operator reachability|accept]");
            return 2;
        }
    };

    let mut total_sites = 0usize;
    let mut total_killed = 0usize;
    let mut total_invalid = 0usize;
    let mut total_survivors = 0usize;
    let mut unexplained: Vec<(String, usize, String)> = Vec::new();

    for job in &jobs {
        let tally = match mutate_file(&root, job, operator) {
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
            print_score(operator, tally.killed, valid);
        }
        total_sites += tally.sites;
        total_killed += tally.killed;
        total_invalid += tally.invalid;
        total_survivors += tally.survivors.len();
        for (line, snippet) in tally.survivors {
            let file = job.file.clone();
            // Equivalences were recorded against the reachability operator; an
            // accept mutant that survives is always a real gap in asserted
            // refusals, so it is never exempted.
            let equivalent = operator == Operator::Reachability
                && EQUIVALENT
                    .iter()
                    .any(|(f, l, s)| *f == job.file.as_str() && *l == line && *s == snippet);
            if equivalent {
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
        print_score(operator, total_killed, valid);
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
