//! Compare stock and failure-instrumented TacticTrace success records.
//!
//! Rust port of the earlier Python helper, with the same CLI, the same field
//! list and the same alignment key. **Not on the verdict path.**
//!
//! Each directory holds the per-tactic files TacticTrace writes, `<tac>.json`.
//! Files whose records carry a `conv` key are conversion traces and are
//! skipped; conversion success records are a separate channel and are not
//! compared here. Records are aligned by (tactic, ordinal within file); JSON
//! key order and whitespace are normalised by parsing, but field values are
//! compared exactly.
//!
//! The shared JSON reader stores object members in sorted order, so a diff
//! diagnostic that shows an *object value* may print its keys in a different
//! order than Python's `repr`. Comparison results are unaffected.

use crate::tools::json::{self, Json};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

pub const FIELDS: &[&str] = &[
    "tactic",
    "definition_line_number",
    "user_line_number",
    "goal_before",
    "goals_after",
    "num_subgoals",
    "arg_names",
    "arg_types",
    "arg_values",
    "arg_exprs",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConformanceError(pub String);

impl fmt::Display for ConformanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

fn err<T>(message: impl Into<String>) -> Result<T, ConformanceError> {
    Err(ConformanceError(message.into()))
}

/// Return tactic records, or `None` for a conversion trace.
///
/// Conversion traces use a `conv` key and are a separate generator branch, so
/// they are skipped rather than mixed into the tactic alignment.
pub fn load_file(path: &Path) -> Result<Option<Vec<Json>>, ConformanceError> {
    let text = fs::read_to_string(path)
        .map_err(|e| ConformanceError(format!("{}: {e}", path.display())))?;
    let data = json::parse(text.as_bytes())
        .map_err(|e| ConformanceError(format!("{}: invalid JSON: {e}", path.display())))?;
    let arr = match data {
        Json::Arr(items) => items,
        _ => {
            return err(format!(
                "{}: expected a JSON array of records",
                path.display()
            ))
        }
    };
    if arr.is_empty() {
        return Ok(Some(Vec::new()));
    }
    let has_tactic = arr.iter().any(|record| record.get("tactic").is_some());
    if !has_tactic {
        if arr.iter().all(|record| record.get("conv").is_some()) {
            return Ok(None);
        }
        return err(format!(
            "{}: records have neither a 'tactic' nor a 'conv' field",
            path.display()
        ));
    }
    for (ordinal, record) in arr.iter().enumerate() {
        if record.as_obj().is_none() {
            return err(format!(
                "{}: record {ordinal} is not an object",
                path.display()
            ));
        }
        match record.get("tactic") {
            Some(Json::Str(_)) => {}
            _ => {
                return err(format!(
                    "{}: record {ordinal} has no string 'tactic' field",
                    path.display()
                ))
            }
        }
    }
    Ok(Some(arr))
}

type Aligned = BTreeMap<(String, usize), (String, Json)>;

fn json_files(directory: &Path) -> Result<Vec<PathBuf>, ConformanceError> {
    let entries = fs::read_dir(directory)
        .map_err(|e| ConformanceError(format!("{}: {e}", directory.display())))?;
    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in entries {
        let path = entry
            .map_err(|e| ConformanceError(format!("{}: {e}", directory.display())))?
            .path();
        if path.extension().map(|e| e == "json").unwrap_or(false) && path.is_file() {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

pub fn collect(directory: &Path) -> Result<(Aligned, usize), ConformanceError> {
    if !directory.is_dir() {
        return err(format!("not a directory: {}", directory.display()));
    }
    let mut aligned: Aligned = BTreeMap::new();
    let mut skipped_conversions = 0;
    for path in json_files(directory)? {
        let Some(records) = load_file(&path)? else {
            skipped_conversions += 1;
            continue;
        };
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        for (ordinal, record) in records.into_iter().enumerate() {
            let tactic = match record.get("tactic") {
                Some(Json::Str(s)) => s.clone(),
                _ => String::new(),
            };
            let key = (tactic, ordinal);
            if let Some((first, _)) = aligned.get(&key) {
                return err(format!(
                    "duplicate alignment key ('{}', {}) in {} and {}",
                    key.0,
                    key.1,
                    path.display(),
                    first
                ));
            }
            aligned.insert(key, (name.clone(), record));
        }
    }
    Ok((aligned, skipped_conversions))
}

#[derive(Clone, Copy)]
enum Val<'a> {
    Missing,
    Present(&'a Json),
    Int(usize),
}

fn py_float(x: f64) -> String {
    if !x.is_finite() {
        return if x.is_nan() {
            "nan".to_string()
        } else if x > 0.0 {
            "inf".to_string()
        } else {
            "-inf".to_string()
        };
    }
    if x.fract() == 0.0 && x.abs() < 1e16 {
        return format!("{}", x as i64);
    }
    format!("{x}")
}

fn py_str_repr(s: &str) -> String {
    let quote = if s.contains('\'') && !s.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::new();
    out.push(quote);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\x{:02x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

fn py_repr(value: &Json) -> String {
    match value {
        Json::Null => "None".to_string(),
        Json::Bool(b) => (if *b { "True" } else { "False" }).to_string(),
        Json::Num(n) => py_float(*n),
        Json::Str(s) => py_str_repr(s),
        Json::Arr(items) => {
            let parts: Vec<String> = items.iter().map(py_repr).collect();
            format!("[{}]", parts.join(", "))
        }
        Json::Obj(map) => {
            let parts: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("{}: {}", py_str_repr(k), py_repr(v)))
                .collect();
            format!("{{{}}}", parts.join(", "))
        }
    }
}

/// Ported `_short`: a repr, truncated to 160 characters.
fn short(value: Val<'_>) -> String {
    let text = match value {
        Val::Missing => "<missing>".to_string(),
        Val::Present(v) => py_repr(v),
        Val::Int(n) => n.to_string(),
    };
    if text.chars().count() > 160 {
        let mut truncated: String = text.chars().take(157).collect();
        truncated.push_str("...");
        truncated
    } else {
        text
    }
}

/// Return (field path, stock value, patched value) differences.
fn deep_diff<'a>(
    stock: &'a Json,
    patched: &'a Json,
    path: &str,
) -> Vec<(String, Val<'a>, Val<'a>)> {
    let mut diffs = Vec::new();
    match (stock, patched) {
        (Json::Obj(left), Json::Obj(right)) => {
            let keys: BTreeSet<&String> = left.keys().chain(right.keys()).collect();
            for key in keys {
                let sub = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                match (left.get(key), right.get(key)) {
                    (None, Some(r)) => diffs.push((sub, Val::Missing, Val::Present(r))),
                    (Some(l), None) => diffs.push((sub, Val::Present(l), Val::Missing)),
                    (Some(l), Some(r)) => diffs.extend(deep_diff(l, r, &sub)),
                    (None, None) => {}
                }
            }
        }
        (Json::Arr(left), Json::Arr(right)) => {
            if left.len() != right.len() {
                diffs.push((
                    format!("{path}[length]"),
                    Val::Int(left.len()),
                    Val::Int(right.len()),
                ));
            }
            for (index, (l, r)) in left.iter().zip(right.iter()).enumerate() {
                diffs.extend(deep_diff(l, r, &format!("{path}[{index}]")));
            }
        }
        _ => {
            if stock != patched {
                diffs.push((path.to_string(), Val::Present(stock), Val::Present(patched)));
            }
        }
    }
    diffs
}

/// Compare two trace directories. Returns (checked pairs, skipped conversion
/// files, difference messages).
pub fn compare_dirs(
    stock_dir: &Path,
    patched_dir: &Path,
) -> Result<(usize, usize, Vec<String>), ConformanceError> {
    let (stock, stock_conversions) = collect(stock_dir)?;
    let (patched, patched_conversions) = collect(patched_dir)?;
    if stock.is_empty() && patched.is_empty() {
        return err(format!(
            "no <tac>.json records found in {} or {}",
            stock_dir.display(),
            patched_dir.display()
        ));
    }

    let mut diffs: Vec<String> = Vec::new();
    let mut checked = 0;
    let keys: BTreeSet<&(String, usize)> = stock.keys().chain(patched.keys()).collect();

    for (tactic, ordinal) in keys {
        let Some((stock_name, stock_record)) = stock.get(&(tactic.clone(), *ordinal)) else {
            let (patched_name, _) = &patched[&(tactic.clone(), *ordinal)];
            diffs.push(format!(
                "{tactic}[{ordinal}]: unexpected in patched ({patched_name})"
            ));
            continue;
        };
        let Some((patched_name, patched_record)) = patched.get(&(tactic.clone(), *ordinal)) else {
            diffs.push(format!(
                "{tactic}[{ordinal}]: missing from patched ({stock_name})"
            ));
            continue;
        };

        checked += 1;

        for field in FIELDS {
            let left = stock_record.get(field);
            let right = patched_record.get(field);
            match (left, right) {
                (None, _) | (_, None) => diffs.push(format!(
                    "{tactic}[{ordinal}] field '{field}': stock={}, patched={}",
                    if left.is_none() { "missing" } else { "present" },
                    if right.is_none() {
                        "missing"
                    } else {
                        "present"
                    }
                )),
                (Some(l), Some(r)) => {
                    for (field_path, left_value, right_value) in deep_diff(l, r, field) {
                        diffs.push(format!(
                            "{tactic}[{ordinal}] field '{field}' ({field_path}): \
                             stock={} patched={} [{stock_name} vs {patched_name}]",
                            short(left_value),
                            short(right_value)
                        ));
                    }
                }
            }
        }
    }

    Ok((checked, stock_conversions + patched_conversions, diffs))
}

/// Print the comparison and return the CLI exit status.
pub fn report(stock_dir: &Path, patched_dir: &Path) -> i32 {
    let (checked, skipped, diffs) = match compare_dirs(stock_dir, patched_dir) {
        Ok(result) => result,
        Err(exc) => {
            eprintln!("trace_conformance: error: {exc}");
            return 2;
        }
    };

    for diff in &diffs {
        println!("DIFF {diff}");
    }
    if skipped > 0 {
        println!(
            "skipped {skipped} conversion trace file(s); conversion success \
             records are not compared"
        );
    }
    println!(
        "checked {checked} aligned record pair(s); {} difference(s)",
        diffs.len()
    );
    if !diffs.is_empty() {
        println!("CONFORMANCE FAILED");
        return 1;
    }
    println!("CONFORMANCE PASSED");
    0
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("trace/fixtures/conformance")
}

/// Ported `--self-test`: the matching and deliberately diverging fixture pairs.
pub fn self_test() -> i32 {
    let root = fixtures();
    let stock = root.join("stock");
    let patched = root.join("patched");
    let diverged = root.join("diverged");

    if !(stock.is_dir() && patched.is_dir() && diverged.is_dir()) {
        eprintln!("FAIL: missing fixture directories under {}", root.display());
        return 1;
    }

    let mut ok = true;

    println!("matching fixture pair (must pass):");
    let code = report(&stock, &patched);
    if code == 0 {
        println!("  ok: matching pair passed");
    } else {
        println!("  FAIL: matching pair returned {code}");
        ok = false;
    }

    println!("diverging fixture pair (must fail and name num_subgoals):");
    match compare_dirs(&stock, &diverged) {
        Ok((checked, _skipped, diffs)) => {
            for diff in &diffs {
                println!("  DIFF {diff}");
            }
            if diffs.is_empty() {
                println!("  FAIL: diverging pair produced no differences");
                ok = false;
            } else if !diffs.iter().any(|d| d.contains("num_subgoals")) {
                println!("  FAIL: divergence report did not name 'num_subgoals'");
                ok = false;
            } else {
                println!(
                    "  ok: detected {} difference(s) across {} pair(s), including num_subgoals",
                    diffs.len(),
                    checked
                );
            }
        }
        Err(exc) => {
            println!("  FAIL: unexpected load error: {exc}");
            return 1;
        }
    }

    println!(
        "{}",
        if ok {
            "SELF-TEST PASSED"
        } else {
            "SELF-TEST FAILED"
        }
    );
    if ok {
        0
    } else {
        1
    }
}

const USAGE: &str = "\
Usage: trace_conformance STOCK_DIR PATCHED_DIR
       trace_conformance --self-test

Compare stock and patched TacticTrace success records.";

pub fn run(args: &[String]) -> i32 {
    let mut positional: Vec<&str> = Vec::new();
    let mut self_test_flag = false;
    for arg in args {
        match arg.as_str() {
            "--self-test" => self_test_flag = true,
            "-h" | "--help" => {
                println!("{USAGE}");
                return 0;
            }
            other if other.starts_with('-') && other.len() > 1 => {
                eprintln!("trace_conformance: error: unrecognized argument: {other}");
                eprintln!("{USAGE}");
                return 2;
            }
            other => positional.push(other),
        }
    }

    if self_test_flag {
        if !positional.is_empty() {
            eprintln!("trace_conformance: error: --self-test takes no positional arguments");
            return 2;
        }
        return self_test();
    }

    if positional.len() != 2 {
        eprintln!("trace_conformance: error: STOCK_DIR and PATCHED_DIR are required unless --self-test is used");
        eprintln!("{USAGE}");
        return 2;
    }

    report(Path::new(positional[0]), Path::new(positional[1]))
}
