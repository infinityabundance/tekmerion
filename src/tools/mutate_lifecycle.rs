//! Lifecycle mutation harness.
//!
//! Rust port of the former `tools/mutate_lifecycle.py`. Every lifecycle code
//! returned by `tekmerion::fault` is a guarded site whose correct behaviour must
//! be needed by at least one test. The harness enables the off-by-default
//! `fault-injection` feature and sets `TEKMERION_FAULT` to one code per run; a
//! run whose suite stays green means the control is not load-bearing:
//! `SURVIVED`. The parser arm's score is read from `mutation-report.json`, so
//! `mutate` must have run first. **Not on the verdict path.**
//!
//! Exit 0 only when no valid mutant survives.

use super::mutate::{classify, find, json_escape, py_float, run_capture, Outcome, Scratch};
use crate::tools::json;
use std::collections::BTreeSet;
use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::time::Duration;

const FILES: [&str; 2] = ["src/lib.rs", "src/wire.rs"];
const OPEN: &[u8] = b"fault(";
const TEST_TIMEOUT: Duration = Duration::from_secs(900);
const REPORT: &str = "lifecycle-report.json";
const PARSER_REPORT: &str = "mutation-report.json";

const DESCRIPTIONS: [(&str, &str); 5] = [
    ("L01", "status ignores pins"),
    ("L02", "classify drops the pins comparison"),
    (
        "L03",
        "status stops at the direct edge instead of the closure",
    ),
    ("L04", "status ignores attestation edges"),
    ("L05", "update forgets the superseded node"),
];

fn description(code: &str) -> &'static str {
    DESCRIPTIONS
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, d)| *d)
        .unwrap_or("lifecycle guard")
}

/// `fault("Lnn")` sites, which is what the feature switch reads.
fn codes_in(text: &str) -> Vec<String> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(p) = find(&b[i..], OPEN) {
        let at = i + p;
        let mut j = at + OPEN.len();
        while j < b.len() && b[j].is_ascii_whitespace() {
            j += 1;
        }
        if b.get(j) == Some(&b'"')
            && j + 5 <= b.len()
            && b[j + 1] == b'L'
            && b[j + 2].is_ascii_digit()
            && b[j + 3].is_ascii_digit()
            && b[j + 4] == b'"'
        {
            let mut k = j + 5;
            while k < b.len() && b[k].is_ascii_whitespace() {
                k += 1;
            }
            if b.get(k) == Some(&b')') {
                out.push(String::from_utf8_lossy(&b[j + 1..j + 4]).into_owned());
            }
        }
        i = at + OPEN.len();
    }
    out
}

fn discover_codes(root: &Path) -> io::Result<BTreeSet<String>> {
    let mut codes = BTreeSet::new();
    for rel in FILES {
        let text = fs::read_to_string(root.join(rel))?;
        codes.extend(codes_in(&text));
    }
    Ok(codes)
}

fn run_suite(root: &Path, fault: Option<&str>, log: &Path) -> io::Result<(i32, String)> {
    run_capture(
        "cargo",
        &[
            "test",
            "--offline",
            "--quiet",
            "--features",
            "fault-injection",
        ],
        root,
        &[("TEKMERION_FAULT", fault)],
        log,
        TEST_TIMEOUT,
    )
}

fn tail(s: &str, n: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= n {
        s.to_string()
    } else {
        chars[chars.len() - n..].iter().collect()
    }
}

/// The parser arm is only as good as the last `mutate` run's report.
fn parser_score(root: &Path) -> String {
    let Ok(bytes) = fs::read(root.join(PARSER_REPORT)) else {
        return "unknown".to_string();
    };
    let Ok(doc) = json::parse(&bytes) else {
        return "unknown".to_string();
    };
    let (Some(json::Json::Num(killed)), Some(json::Json::Num(valid))) =
        (doc.get("killed"), doc.get("valid_mutants"))
    else {
        return "unknown".to_string();
    };
    match doc.get("mutation_score") {
        Some(json::Json::Num(score)) => {
            format!(
                "{killed}/{valid} = {score:.3}",
                killed = *killed as u64,
                valid = *valid as u64
            )
        }
        _ => "unknown".to_string(),
    }
}

fn render_report(results: &[(String, Outcome)], parser: &str) -> String {
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
        for (i, (code, outcome)) in results.iter().enumerate() {
            if i > 0 {
                body.push(',');
            }
            body.push_str(&format!(
                "\n    \"{}\": \"{}\"",
                json_escape(code),
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
    body.push_str(&format!("  \"lifecycle_mutation_score\": {score},\n"));
    body.push_str(&format!(
        "  \"parser_mutation_score\": \"{}\"\n}}\n",
        json_escape(parser)
    ));
    body
}

pub fn run(_args: &[String]) -> i32 {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let codes = match discover_codes(&root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("ERROR cannot read lifecycle fault codes: {e}");
            return 2;
        }
    };
    if codes.is_empty() {
        eprintln!("no lifecycle fault codes found");
        return 2;
    }

    let scratch = match Scratch::new("tekmerion-lifecycle-", false) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ERROR cannot create scratch directory: {e}");
            return 2;
        }
    };
    let log = scratch.path().join("suite.log");

    // A mutant is meaningful only if the unmutated suite is green with the
    // feature on.
    let (base_code, base_output) = match run_suite(&root, None, &log) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("ERROR running the baseline suite: {e}");
            return 2;
        }
    };
    if base_code != 0 {
        eprintln!("baseline suite failed with fault-injection enabled");
        eprintln!("{}", tail(&base_output, 4000));
        return 2;
    }

    let mut results: Vec<(String, Outcome)> = Vec::new();
    for code in &codes {
        let (status, output) = match run_suite(&root, Some(code), &log) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("ERROR running the suite for {code}: {e}");
                return 2;
            }
        };
        let outcome = classify(status, &output);
        println!("{code} ({}): {}", description(code), outcome.as_str());
        let _ = io::stdout().flush();
        results.push((code.clone(), outcome));
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
    let parser = parser_score(&root);

    println!();
    println!("lifecycle sites: {}", results.len());
    println!(
        "valid mutants: {}  killed: {killed}  survived: {survived}",
        valid.len()
    );
    if !valid.is_empty() {
        println!(
            "lifecycle mutation score: {killed}/{} = {:.3}",
            valid.len(),
            killed as f64 / valid.len() as f64
        );
    }
    println!("parser mutation score: {parser}");

    let report = root.join(REPORT);
    if let Err(e) = fs::write(&report, render_report(&results, &parser)) {
        eprintln!("ERROR cannot write {}: {e}", report.display());
        return 2;
    }

    if survived == 0 {
        0
    } else {
        1
    }
}
