//! Recount functional-correctness (FC) theorems in s2n-bignum.
//!
//! Rust port of the earlier Python helper, with the same CLI, the same
//! benchmark classification rule and the same report. **Not on the verdict
//! path.**
//!
//! Reads a pinned s2n-bignum checkout directly from source (no HOL Light, no
//! build) and applies the benchmark collector's own classification rule:
//!
//! * the goal argument must be a backtick-delimited term, else the theorem is
//!   dropped ("the goal is evaluated at runtime");
//! * a name containing `_EQUIV` or `SUBROUTINE_SAFE` is dropped ("not a proof
//!   related to functional correctness");
//! * a name containing `SUBROUTINE_CORRECT` is dropped ("a top-level
//!   subroutine that just uses the _CORRECT of the core part");
//! * otherwise, `_CORRECT` in the name and the standalone token `ensures` in
//!   the goal make it FC (arm/x86 by top-level directory).
//!
//! Definitions used here, stated because they are not fixed by the bench:
//!
//! * **proof length** — non-blank, non-comment lines in the *tactic* part of
//!   the `prove (...)` / `prove_by_refinement (...)` argument list, i.e.
//!   everything after the goal term. Comments and string bodies are masked
//!   before counting.
//! * **loop theorem** — the tactic part mentions `ENSURES_WHILE*`.
//! * **family** — operation token parsed from the theorem name, falling back
//!   to the file stem, longest keyword first.
//!
//! The measured commit is always printed; a number cannot be read off without
//! its revision.

use crate::tools::json::{self, Json};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

// ---------------------------------------------------------------------------
// Benchmark rule
// ---------------------------------------------------------------------------

/// Exact port of the collector's `contains_anykw`.
fn contains_anykw(keywords: &[&str], text: &str) -> bool {
    for keyword in keywords {
        if let Some(base) = keyword.strip_suffix('*') {
            if text.contains(&format!(" {base}")) {
                return true;
            }
            continue;
        }
        if text.contains(&format!(" {keyword}"))
            || text.contains(&format!("{keyword} "))
            || text.contains(&format!("({keyword}"))
            || text.contains(&format!("{keyword})"))
        {
            return true;
        }
    }
    false
}

/// Return (drop?, reason) mirroring the collector's `categorize` for a
/// goal-only view.
pub fn bench_classify(thm_name: &str, goal: &str) -> (bool, &'static str) {
    let goal = goal.trim();
    let first = goal.find('`');
    let last = goal.rfind('`');
    let (Some(first), Some(last)) = (first, last) else {
        return (true, "the goal is evaluated at runtime");
    };
    if first == last {
        return (true, "the goal is evaluated at runtime");
    }
    let goal = if first != 0 || last != goal.len() - 1 {
        &goal[first..=last]
    } else {
        goal
    };

    if thm_name.contains("_EQUIV") || thm_name.contains("SUBROUTINE_SAFE") {
        return (true, "not a proof related to functional correctness");
    }
    if thm_name.contains("SUBROUTINE_CORRECT") {
        return (
            true,
            "a top-level subroutine that just uses the _CORRECT of the core part",
        );
    }

    let folded = format!(" {} ", goal.replace('\n', " "));
    if thm_name.contains("_CORRECT") && contains_anykw(&["ensures"], &folded) {
        return (false, "functional_correctness");
    }
    (false, "non_fc")
}

// ---------------------------------------------------------------------------
// Source scanning
// ---------------------------------------------------------------------------

/// Blank out OCaml comments and string bodies, preserving offsets.
///
/// Newlines are kept so line numbers stay aligned with the original file.
/// Handles nested `(* ... *)` and backslash escapes in strings.
pub fn mask_source(src: &[u8]) -> Vec<u8> {
    let mut out = src.to_vec();
    let n = src.len();
    let mut i = 0;
    let mut mode = 0u8; // 0 normal, 1 comment, 2 string
    let mut depth = 0usize;
    while i < n {
        let c = src[i];
        match mode {
            0 => {
                if src[i..].starts_with(b"(*") {
                    mode = 1;
                    depth = 1;
                    out[i] = b' ';
                    if i + 1 < n {
                        out[i + 1] = b' ';
                    }
                    i += 2;
                } else if c == b'"' {
                    mode = 2;
                    out[i] = b' ';
                    i += 1;
                } else {
                    i += 1;
                }
            }
            1 => {
                if src[i..].starts_with(b"(*") {
                    depth += 1;
                    out[i] = b' ';
                    if i + 1 < n {
                        out[i + 1] = b' ';
                    }
                    i += 2;
                } else if src[i..].starts_with(b"*)") {
                    depth -= 1;
                    out[i] = b' ';
                    if i + 1 < n {
                        out[i + 1] = b' ';
                    }
                    i += 2;
                    if depth == 0 {
                        mode = 0;
                    }
                } else {
                    if c != b'\n' {
                        out[i] = b' ';
                    }
                    i += 1;
                }
            }
            _ => {
                if c != b'\n' {
                    out[i] = b' ';
                }
                if c == b'\\' && i + 1 < n {
                    if src[i + 1] != b'\n' {
                        out[i + 1] = b' ';
                    }
                    i += 2;
                    continue;
                }
                if c == b'"' {
                    mode = 0;
                }
                i += 1;
            }
        }
    }
    out
}

fn match_paren(masked: &[u8], open_idx: usize) -> Option<usize> {
    let mut depth = 0i64;
    for (offset, &ch) in masked[open_idx..].iter().enumerate() {
        if ch == b'(' {
            depth += 1;
        } else if ch == b')' {
            depth -= 1;
            if depth == 0 {
                return Some(open_idx + offset);
            }
        }
    }
    None
}

fn find_byte(hay: &[u8], needle: u8, from: usize, to: usize) -> Option<usize> {
    if from >= to || to > hay.len() {
        return None;
    }
    hay[from..to]
        .iter()
        .position(|&b| b == needle)
        .map(|p| from + p)
}

fn count_content_lines(masked: &[u8], start: usize, end: usize) -> usize {
    if end <= start {
        return 0;
    }
    masked[start..end]
        .split(|&b| b == b'\n')
        .filter(|line| line.iter().any(|b| !b.is_ascii_whitespace()))
        .count()
}

/// `^let[ \t]+([A-Za-z][A-Za-z0-9_']*)[ \t]*=`, anchored at a line start.
fn match_decl(masked: &[u8], start: usize) -> Option<(String, usize)> {
    let n = masked.len();
    if !masked[start..].starts_with(b"let") {
        return None;
    }
    let mut i = start + 3;
    if i >= n || (masked[i] != b' ' && masked[i] != b'\t') {
        return None;
    }
    while i < n && (masked[i] == b' ' || masked[i] == b'\t') {
        i += 1;
    }
    if i >= n || !masked[i].is_ascii_alphabetic() {
        return None;
    }
    let name_start = i;
    i += 1;
    while i < n && (masked[i].is_ascii_alphanumeric() || masked[i] == b'_' || masked[i] == b'\'') {
        i += 1;
    }
    let name = String::from_utf8_lossy(&masked[name_start..i]).into_owned();
    while i < n && (masked[i] == b' ' || masked[i] == b'\t') {
        i += 1;
    }
    if i >= n || masked[i] != b'=' {
        return None;
    }
    Some((name, i + 1))
}

fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

fn skip_space(masked: &[u8], mut i: usize) -> usize {
    while i < masked.len() && is_space(masked[i]) {
        i += 1;
    }
    i
}

/// `\s*(?:time\s+)?(prove_by_refinement|prove)\s*\(`, anchored at `start`.
/// Returns the index of the opening parenthesis.
fn match_apply(masked: &[u8], start: usize) -> Option<usize> {
    let mut i = skip_space(masked, start);
    if masked[i..].starts_with(b"time") {
        let after = i + 4;
        if after < masked.len() && is_space(masked[after]) {
            i = skip_space(masked, after);
        }
    }
    if masked[i..].starts_with(b"prove_by_refinement") {
        i += b"prove_by_refinement".len();
    } else if masked[i..].starts_with(b"prove") {
        i += b"prove".len();
    } else {
        return None;
    }
    i = skip_space(masked, i);
    if i < masked.len() && masked[i] == b'(' {
        Some(i)
    } else {
        None
    }
}

#[derive(Clone)]
struct Row {
    file: String,
    arch: &'static str,
    name: String,
    goal: String,
    tactic_lines: usize,
    span_lines: usize,
    is_loop: bool,
}

/// One row per top-level `let NAME = prove ...` declaration.
fn scan_file(path: &Path, relpath: &str, arch: &'static str) -> Result<Vec<Row>, String> {
    let src = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let masked = mask_source(&src);
    let mut found = Vec::new();
    let mut i = 0;
    while i < masked.len() {
        if i == 0 || masked[i - 1] == b'\n' {
            if let Some((name, m_end)) = match_decl(&masked, i) {
                if let Some(open_idx) = match_apply(&masked, m_end) {
                    if let Some(close_idx) = match_paren(&masked, open_idx) {
                        let bt1 = find_byte(&masked, b'`', open_idx + 1, close_idx);
                        let mut goal_text = String::new();
                        let mut tactic_start = open_idx + 1;
                        match bt1 {
                            Some(bt1) => {
                                if let Some(bt2) = find_byte(&masked, b'`', bt1 + 1, close_idx) {
                                    goal_text =
                                        String::from_utf8_lossy(&src[bt1..bt2 + 1]).into_owned();
                                    tactic_start = bt2 + 1;
                                }
                            }
                            None => {
                                goal_text =
                                    String::from_utf8_lossy(&masked[open_idx + 1..close_idx])
                                        .into_owned();
                            }
                        }
                        found.push(Row {
                            file: relpath.to_string(),
                            arch,
                            name,
                            goal: goal_text,
                            tactic_lines: count_content_lines(&masked, tactic_start, close_idx),
                            span_lines: count_content_lines(&masked, open_idx, close_idx),
                            is_loop: masked[tactic_start..close_idx]
                                .windows(b"ENSURES_WHILE".len())
                                .any(|w| w == b"ENSURES_WHILE"),
                        });
                    }
                }
                i = m_end;
                continue;
            }
        }
        i += 1;
    }
    Ok(found)
}

fn git_head(repo: &Path) -> String {
    match Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["rev-parse", "HEAD"])
        .output()
    {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        Ok(out) => format!("<unavailable: git exited {}>", out.status),
        Err(e) => format!("<unavailable: {e}>"),
    }
}

fn walk(dir: &Path, repo: &Path, arch: &'static str, rows: &mut Vec<Row>) -> Result<(), String> {
    if !dir.is_dir() {
        return Ok(());
    }
    let mut dirs: Vec<PathBuf> = Vec::new();
    let mut files: Vec<PathBuf> = Vec::new();
    let entries = fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("{}: {e}", dir.display()))?;
        let path = entry.path();
        if path.is_dir() {
            dirs.push(path);
        } else {
            files.push(path);
        }
    }
    files.sort();
    for file in files {
        if file.extension().map(|e| e == "ml").unwrap_or(false) {
            let rel = file
                .strip_prefix(repo)
                .unwrap_or(&file)
                .to_string_lossy()
                .replace('\\', "/");
            rows.extend(scan_file(&file, &rel, arch)?);
        }
    }
    for sub in dirs {
        walk(&sub, repo, arch, rows)?;
    }
    Ok(())
}

fn scan_checkout(repo: &Path) -> Result<Vec<Row>, String> {
    if !repo.join("arm").join("proofs").is_dir() {
        return Err(format!(
            "not an s2n-bignum checkout (no arm/proofs): {}",
            repo.display()
        ));
    }
    let mut rows = Vec::new();
    for arch in ["arm", "x86"] {
        walk(&repo.join(arch).join("proofs"), repo, arch, &mut rows)?;
    }
    Ok(rows)
}

// ---------------------------------------------------------------------------
// Families
// ---------------------------------------------------------------------------

/// Longest-first so 'montmul' wins over 'mul' and 'modsub' resolves explicitly.
const FAMILY_PATTERNS: &[&str] = &[
    "montmul",
    "montsqr",
    "amontmul",
    "amontsqr",
    "amontredc",
    "emontredc",
    "montinv",
    "tomont",
    "demont",
    "deamont",
    "modinv",
    "modsub",
    "modadd",
    "moddouble",
    "optneg",
    "optadd",
    "optsub",
    "cmnegadd",
    "cmadd",
    "cmul",
    "cmod",
    "cdiv",
    "madd",
    "invsqrt",
    "triple",
    "double",
    "half",
    "sqrt",
    "inv",
    "neg",
    "add",
    "sub",
    "mod",
    "mul",
    "sqr",
];

/// Operation token: theorem name first, then file stem.
fn family_of(name: &str, file: &str, _arch: &str) -> Option<&'static str> {
    let stem = {
        let base = Path::new(file)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if base.len() > 3 {
            base[..base.len() - 3].to_string()
        } else {
            String::new()
        }
    };
    let mut candidates = vec![name.to_lowercase()];
    if stem.to_lowercase() != name.to_lowercase() {
        candidates.push(stem.to_lowercase());
    }
    for raw in candidates {
        let tokens: Vec<&str> = raw
            .split(|c: char| !c.is_ascii_lowercase() && !c.is_ascii_digit())
            .filter(|t| !t.is_empty())
            .collect();
        for &pattern in FAMILY_PATTERNS {
            if raw.contains(pattern) && tokens.iter().any(|tok| tok.contains(pattern)) {
                return Some(pattern);
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Statistics
// ---------------------------------------------------------------------------

const BANDS: &[(usize, usize)] = &[
    (0, 24),
    (25, 49),
    (50, 99),
    (100, 199),
    (200, 299),
    (300, 599),
    (600, usize::MAX),
];

fn band_label(lo: usize, hi: usize) -> String {
    if hi == usize::MAX {
        format!("{lo}+")
    } else {
        format!("{lo}-{hi}")
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Num {
    Int(i64),
    Float(f64),
}

impl fmt::Display for Num {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Num::Int(n) => write!(f, "{n}"),
            Num::Float(x) => f.write_str(&fmt_float(*x)),
        }
    }
}

/// Python's `str(float)`: shortest round-trip, always with a decimal point.
fn fmt_float(x: f64) -> String {
    if !x.is_finite() {
        return if x.is_nan() {
            "nan".to_string()
        } else if x > 0.0 {
            "inf".to_string()
        } else {
            "-inf".to_string()
        };
    }
    let s = format!("{x}");
    if s.contains('.') || s.contains('e') || s.contains('E') {
        s
    } else {
        format!("{s}.0")
    }
}

/// Python's `round(x, 2)` (ties to even).
fn py_round2(x: f64) -> f64 {
    let y = x * 100.0;
    let floor = y.floor();
    let delta = y - floor;
    let rounded = if delta > 0.5 {
        floor + 1.0
    } else if delta < 0.5 || (floor as i64) % 2 == 0 {
        floor
    } else {
        floor + 1.0
    };
    rounded / 100.0
}

#[derive(Clone)]
struct Summary {
    n: usize,
    median: Option<Num>,
    q1: Option<Num>,
    q3: Option<Num>,
    min: Option<usize>,
    max: Option<usize>,
    mean: Option<f64>,
    bands: Vec<(String, usize)>,
}

fn all_bands() -> Vec<(String, usize)> {
    BANDS
        .iter()
        .map(|(lo, hi)| (band_label(*lo, *hi), 0))
        .collect()
}

/// `statistics.quantiles(values, n=4, method="inclusive")`, first and third.
fn inclusive_quartiles(vals: &[usize]) -> (f64, f64) {
    let n = 4usize;
    let m = vals.len() - 1;
    let mut result = [0.0f64; 3];
    for (index, slot) in result.iter_mut().enumerate() {
        let i = index + 1;
        let j = i * m / n;
        let delta = i * m - j * n;
        *slot = (vals[j] * (n - delta) + vals[j + 1] * delta) as f64 / n as f64;
    }
    (result[0], result[2])
}

fn summarize(values: &[usize]) -> Summary {
    if values.is_empty() {
        return Summary {
            n: 0,
            median: None,
            q1: None,
            q3: None,
            min: None,
            max: None,
            mean: None,
            bands: all_bands(),
        };
    }
    let mut vals = values.to_vec();
    vals.sort_unstable();

    let (median, q1, q3) = if vals.len() > 1 {
        let (q1, q3) = inclusive_quartiles(&vals);
        let mid = vals.len() / 2;
        let median = if vals.len() % 2 == 1 {
            Num::Int(vals[mid] as i64)
        } else {
            Num::Float((vals[mid - 1] + vals[mid]) as f64 / 2.0)
        };
        (median, Num::Float(q1), Num::Float(q3))
    } else {
        (
            Num::Int(vals[0] as i64),
            Num::Int(vals[0] as i64),
            Num::Int(vals[0] as i64),
        )
    };

    let mut bands = all_bands();
    for value in &vals {
        for (index, (lo, hi)) in BANDS.iter().enumerate() {
            if *lo <= *value && *value <= *hi {
                bands[index].1 += 1;
                break;
            }
        }
    }

    let sum: usize = vals.iter().sum();
    Summary {
        n: vals.len(),
        median: Some(median),
        q1: Some(q1),
        q3: Some(q3),
        min: Some(vals[0]),
        max: Some(vals[vals.len() - 1]),
        mean: Some(py_round2(sum as f64 / vals.len() as f64)),
        bands,
    }
}

// ---------------------------------------------------------------------------
// Report
// ---------------------------------------------------------------------------

fn bump(counts: &mut Vec<(String, usize)>, key: &str) {
    if let Some(entry) = counts.iter_mut().find(|(k, _)| k == key) {
        entry.1 += 1;
    } else {
        counts.push((key.to_string(), 1));
    }
}

fn most_common(mut counts: Vec<(String, usize)>) -> Vec<(String, usize)> {
    counts.sort_by_key(|a| std::cmp::Reverse(a.1));
    counts
}

fn normalise_goal(goal: &str) -> String {
    goal.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn arch_relative(path: &str) -> String {
    let normalised = path.replace('\\', "/");
    let parts: Vec<&str> = normalised.split('/').collect();
    for (i, part) in parts.iter().enumerate() {
        if (*part == "arm" || *part == "x86") && i + 1 < parts.len() {
            return parts[i + 1..].join("/");
        }
    }
    parts.last().copied().unwrap_or("").to_string()
}

fn arch_of_category(category: &str) -> String {
    category
        .rsplit_once('_')
        .map(|(_, arch)| arch.to_string())
        .unwrap_or_else(|| category.to_string())
}

fn abspath(path: &str) -> String {
    let given = Path::new(path);
    let joined = if given.is_absolute() {
        given.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(given))
            .unwrap_or_else(|_| given.to_path_buf())
    };
    let mut out = PathBuf::new();
    for component in joined.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out.to_string_lossy().into_owned()
}

struct Corpus {
    path: String,
    corpus_total_problems: usize,
    corpus_categories: BTreeMap<String, usize>,
    corpus_fc_total: usize,
    corpus_fc_by_arch: BTreeMap<String, usize>,
    direct_fc_total: usize,
    direct_fc_by_arch: BTreeMap<String, usize>,
    matched: usize,
    only_direct: usize,
    only_corpus: usize,
    only_direct_examples: Vec<(String, String, String)>,
    only_corpus_examples: Vec<(String, String, String)>,
}

struct Direct {
    fc_total: usize,
    fc_by_arch: BTreeMap<String, usize>,
    fc_unique_goal: usize,
    fc_duplicate_goal_extra: usize,
    straight_line: usize,
    loop_count: usize,
    straight_line_share_pct: Option<f64>,
    loop_share_pct: Option<f64>,
    equiv_named: usize,
    subroutine_safe_named: usize,
    ensures2_goal: usize,
    reason_counts: Vec<(String, usize)>,
    named_correct_decls: usize,
    subroutine_correct_decls: usize,
    length_tactic: Summary,
    length_span: Summary,
    length_by_arch: Vec<(&'static str, Summary)>,
    shape_by_arch: Vec<(&'static str, usize, usize)>,
    length_by_shape: Vec<(&'static str, Summary)>,
    long_tail_n: usize,
    long_tail_loop: usize,
    long_tail_straight: usize,
    long_tail_loop_share_pct: Option<f64>,
    families_classified: usize,
    families_unclassified: usize,
    families_by_family: Vec<(String, usize)>,
    families_by_arch: BTreeMap<String, BTreeMap<String, usize>>,
}

struct Report {
    repo: String,
    commit: String,
    direct: Direct,
    dropped_named: Vec<(String, usize)>,
    all_files_scanned: usize,
    all_declarations: usize,
    corpus: Option<Corpus>,
}

fn compare_corpus(path: &Path, fc: &[Row], report: &Direct) -> Result<Corpus, String> {
    let bytes = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let data = json::parse(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    let problems = match data {
        Json::Obj(map) => map,
        _ => return Err(format!("{}: expected a JSON object", path.display())),
    };

    let mut cats: BTreeMap<String, usize> = BTreeMap::new();
    let mut corpus_fc: BTreeSet<(String, String, String)> = BTreeSet::new();
    let mut corpus_fc_arch: BTreeMap<String, usize> = BTreeMap::new();
    for value in problems.values() {
        let category = value
            .get("category")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("{}: problem has no string category", path.display()))?;
        *cats.entry(category.to_string()).or_insert(0) += 1;

        if category.starts_with("functional") {
            let filename = value
                .get("json")
                .and_then(|v| v.get("filename"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("{}: problem has no json.filename", path.display()))?;
            let legacy = value
                .get("legacy_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("{}: problem has no legacy_id", path.display()))?;
            let theorem = legacy.rsplit('.').next().unwrap_or(legacy);
            corpus_fc.insert((
                arch_of_category(category),
                arch_relative(filename),
                theorem.to_string(),
            ));
        }
        match category {
            "functional_correctness_arm" => {
                *corpus_fc_arch.entry("arm".to_string()).or_insert(0) += 1
            }
            "functional_correctness_x86" => {
                *corpus_fc_arch.entry("x86".to_string()).or_insert(0) += 1
            }
            _ => {}
        }
    }

    let mine: BTreeSet<(String, String, String)> = fc
        .iter()
        .map(|r| (r.arch.to_string(), arch_relative(&r.file), r.name.clone()))
        .collect();
    let only_direct: Vec<(String, String, String)> = mine.difference(&corpus_fc).cloned().collect();
    let only_corpus: Vec<(String, String, String)> = corpus_fc.difference(&mine).cloned().collect();

    Ok(Corpus {
        path: abspath(&path.to_string_lossy()),
        corpus_total_problems: problems.len(),
        corpus_categories: cats,
        corpus_fc_total: corpus_fc_arch.values().sum(),
        corpus_fc_by_arch: corpus_fc_arch,
        direct_fc_total: report.fc_total,
        direct_fc_by_arch: report.fc_by_arch.clone(),
        matched: mine.intersection(&corpus_fc).count(),
        only_direct: only_direct.len(),
        only_corpus: only_corpus.len(),
        only_direct_examples: only_direct.into_iter().take(20).collect(),
        only_corpus_examples: only_corpus.into_iter().take(20).collect(),
    })
}

fn build_report(repo: &Path, corpus_path: Option<&Path>) -> Result<Report, String> {
    let rows = scan_checkout(repo)?;
    let commit = git_head(repo);

    let mut fc: Vec<Row> = Vec::new();
    let mut equiv_named = 0;
    let mut safe_names = 0;
    let mut ensures2_goals = 0;
    let mut reason_counts: Vec<(String, usize)> = Vec::new();
    let mut dropped_named: Vec<(String, usize)> = Vec::new();
    for row in &rows {
        if row.name.contains("_EQUIV") {
            equiv_named += 1;
        }
        if row.name.contains("SUBROUTINE_SAFE") {
            safe_names += 1;
        }
        if row.goal.contains("ensures2") {
            ensures2_goals += 1;
        }
        let (drop, reason) = bench_classify(&row.name, &row.goal);
        if drop {
            bump(&mut reason_counts, reason);
            if row.name.contains("_CORRECT") || row.name.contains("SUBROUTINE") {
                bump(&mut dropped_named, reason);
            }
        } else if reason == "functional_correctness" {
            fc.push(row.clone());
        }
    }

    let mut fc_by_arch: BTreeMap<String, usize> = BTreeMap::new();
    for row in &fc {
        *fc_by_arch.entry(row.arch.to_string()).or_insert(0) += 1;
    }
    let mut unique_goals: BTreeSet<String> = BTreeSet::new();
    for row in &fc {
        unique_goals.insert(normalise_goal(&row.goal));
    }

    let straight: Vec<&Row> = fc.iter().filter(|r| !r.is_loop).collect();
    let loops: Vec<&Row> = fc.iter().filter(|r| r.is_loop).collect();

    let length_tactic = summarize(&fc.iter().map(|r| r.tactic_lines).collect::<Vec<_>>());
    let length_span = summarize(&fc.iter().map(|r| r.span_lines).collect::<Vec<_>>());
    let length_by_arch = vec![
        (
            "arm",
            summarize(
                &fc.iter()
                    .filter(|r| r.arch == "arm")
                    .map(|r| r.tactic_lines)
                    .collect::<Vec<_>>(),
            ),
        ),
        (
            "x86",
            summarize(
                &fc.iter()
                    .filter(|r| r.arch == "x86")
                    .map(|r| r.tactic_lines)
                    .collect::<Vec<_>>(),
            ),
        ),
    ];
    let shape_by_arch = vec![
        (
            "arm",
            fc.iter().filter(|r| r.arch == "arm" && !r.is_loop).count(),
            fc.iter().filter(|r| r.arch == "arm" && r.is_loop).count(),
        ),
        (
            "x86",
            fc.iter().filter(|r| r.arch == "x86" && !r.is_loop).count(),
            fc.iter().filter(|r| r.arch == "x86" && r.is_loop).count(),
        ),
    ];
    let length_by_shape = vec![
        (
            "straight_line",
            summarize(&straight.iter().map(|r| r.tactic_lines).collect::<Vec<_>>()),
        ),
        (
            "loop",
            summarize(&loops.iter().map(|r| r.tactic_lines).collect::<Vec<_>>()),
        ),
    ];

    let mut families: Vec<(String, usize)> = Vec::new();
    let mut fam_by_arch: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    let mut unclassified = 0;
    for row in &straight {
        match family_of(&row.name, &row.file, row.arch) {
            Some(family) => {
                bump(&mut families, family);
                *fam_by_arch
                    .entry(family.to_string())
                    .or_default()
                    .entry(row.arch.to_string())
                    .or_insert(0) += 1;
            }
            None => unclassified += 1,
        }
    }
    let families_by_family = most_common(families.clone());

    let long_tail: Vec<&Row> = fc.iter().filter(|r| r.tactic_lines >= 300).collect();
    let long_tail_loop = long_tail.iter().filter(|r| r.is_loop).count();

    let direct = Direct {
        fc_total: fc.len(),
        fc_by_arch,
        fc_unique_goal: unique_goals.len(),
        fc_duplicate_goal_extra: fc.len() - unique_goals.len(),
        straight_line: straight.len(),
        loop_count: loops.len(),
        straight_line_share_pct: percent(straight.len(), fc.len()),
        loop_share_pct: percent(loops.len(), fc.len()),
        equiv_named,
        subroutine_safe_named: safe_names,
        ensures2_goal: ensures2_goals,
        reason_counts: most_common(reason_counts),
        named_correct_decls: rows.iter().filter(|r| r.name.contains("_CORRECT")).count(),
        subroutine_correct_decls: rows
            .iter()
            .filter(|r| r.name.contains("SUBROUTINE_CORRECT"))
            .count(),
        length_tactic,
        length_span,
        length_by_arch,
        shape_by_arch,
        length_by_shape,
        long_tail_n: long_tail.len(),
        long_tail_loop,
        long_tail_straight: long_tail.len() - long_tail_loop,
        long_tail_loop_share_pct: percent(long_tail_loop, long_tail.len()),
        families_classified: families.iter().map(|(_, n)| *n).sum(),
        families_unclassified: unclassified,
        families_by_family,
        families_by_arch: fam_by_arch,
    };

    let report = Report {
        repo: abspath(&repo.to_string_lossy()),
        commit,
        direct,
        dropped_named: most_common(dropped_named),
        all_files_scanned: rows.iter().map(|r| &r.file).collect::<BTreeSet<_>>().len(),
        all_declarations: rows.len(),
        corpus: None,
    };

    match corpus_path {
        Some(path) => {
            let corpus = compare_corpus(path, &fc, &report.direct)?;
            Ok(Report {
                corpus: Some(corpus),
                ..report
            })
        }
        None => Ok(report),
    }
}

fn percent(part: usize, whole: usize) -> Option<f64> {
    if whole == 0 {
        None
    } else {
        Some(py_round2(100.0 * part as f64 / whole as f64))
    }
}

// ---------------------------------------------------------------------------
// Human output
// ---------------------------------------------------------------------------

fn bands_text(bands: &[(String, usize)]) -> String {
    bands
        .iter()
        .map(|(k, v)| format!("{k}:{v}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn dict_text(map: &BTreeMap<String, usize>) -> String {
    let parts: Vec<String> = map
        .iter()
        .map(|(k, v)| format!("{}: {}", py_str(k), v))
        .collect();
    format!("{{{}}}", parts.join(", "))
}

fn py_str(s: &str) -> String {
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
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

fn tuple_text(tuple: &(String, String, String)) -> String {
    format!(
        "({}, {}, {})",
        py_str(&tuple.0),
        py_str(&tuple.1),
        py_str(&tuple.2)
    )
}

fn pct(value: Option<f64>) -> String {
    match value {
        Some(x) => fmt_float(x),
        None => "None".to_string(),
    }
}

fn report_text(rep: &Report) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let d = &rep.direct;
    let _ = writeln!(out, "s2n-bignum commit: {}", rep.commit);
    let _ = writeln!(out, "checkout:          {}", rep.repo);
    let _ = writeln!(out, "files scanned:     {}", rep.all_files_scanned);
    let _ = writeln!(out, "top-level decls:   {}", rep.all_declarations);
    let _ = writeln!(out);
    let _ = writeln!(out, "== 1. FC theorem count (bench rule) ==");
    let _ = writeln!(out, "  total FC: {}", d.fc_total);
    for (arch, n) in &d.fc_by_arch {
        let _ = writeln!(out, "    {arch:<4} {n}");
    }
    let _ = writeln!(
        out,
        "  unique goals: {}  (declarations sharing a goal: {})",
        d.fc_unique_goal, d.fc_duplicate_goal_extra
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "== 2. shape ==");
    let _ = writeln!(
        out,
        "  straight-line: {} ({}%)",
        d.straight_line,
        pct(d.straight_line_share_pct)
    );
    let _ = writeln!(
        out,
        "  loop:          {} ({}%)",
        d.loop_count,
        pct(d.loop_share_pct)
    );
    let _ = writeln!(out, "  equivalence (name has _EQUIV): {}", d.equiv_named);
    let _ = writeln!(
        out,
        "  SUBROUTINE_SAFE names:          {}",
        d.subroutine_safe_named
    );
    let _ = writeln!(out, "  goals mentioning ensures2:     {}", d.ensures2_goal);
    let mut long_tail = format!(
        "  proofs >= 300 lines: {}  loop {} / straight {}",
        d.long_tail_n, d.long_tail_loop, d.long_tail_straight
    );
    if d.long_tail_n != 0 {
        long_tail.push_str(&format!(
            "  (loop share {}%)",
            pct(d.long_tail_loop_share_pct)
        ));
    }
    let _ = writeln!(out, "{long_tail}");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "== 3. proof length (non-blank, non-comment tactic lines) =="
    );
    for (label, s) in [
        ("tactic-only", &d.length_tactic),
        ("whole prove span", &d.length_span),
    ] {
        if s.n > 0 {
            let _ = writeln!(
                out,
                "  {label:<16} n={} median={} q1={} q3={} min={} max={}",
                s.n,
                s.median.unwrap_or(Num::Int(0)),
                s.q1.unwrap_or(Num::Int(0)),
                s.q3.unwrap_or(Num::Int(0)),
                s.min.unwrap_or(0),
                s.max.unwrap_or(0)
            );
            let _ = writeln!(out, "    bands: {}", bands_text(&s.bands));
        }
    }
    for (arch, s) in &d.length_by_arch {
        if s.n > 0 {
            let _ = writeln!(
                out,
                "  {arch} tactic-only  n={} median={} q1={} q3={}",
                s.n,
                s.median.unwrap_or(Num::Int(0)),
                s.q1.unwrap_or(Num::Int(0)),
                s.q3.unwrap_or(Num::Int(0))
            );
        }
    }
    let _ = writeln!(out, "  by shape (tactic-only):");
    for (shape, s) in &d.length_by_shape {
        if s.n > 0 {
            let _ = writeln!(
                out,
                "    {shape:<12} n={} median={} q1={} q3={}",
                s.n,
                s.median.unwrap_or(Num::Int(0)),
                s.q1.unwrap_or(Num::Int(0)),
                s.q3.unwrap_or(Num::Int(0))
            );
            let _ = writeln!(out, "      bands: {}", bands_text(&s.bands));
        }
    }
    let _ = writeln!(
        out,
        "  shape by arch: {}",
        d.shape_by_arch
            .iter()
            .map(|(arch, straight_n, loop_n)| format!(
                "{arch} straight {straight_n} / loop {loop_n}"
            ))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "== 4. families (straight-line FC only) ==");
    let _ = writeln!(
        out,
        "  classified: {}   unclassified: {}",
        d.families_classified, d.families_unclassified
    );
    for (name, n) in &d.families_by_family {
        let arch = d.families_by_arch.get(name);
        let arm = arch.and_then(|m| m.get("arm")).copied().unwrap_or(0);
        let x86 = arch.and_then(|m| m.get("x86")).copied().unwrap_or(0);
        let _ = writeln!(out, "    {name:<10} {n:>4}  (arm {arm}, x86 {x86})");
    }
    let listed = [
        "mod", "montmul", "montsqr", "add", "sub", "neg", "double", "half", "triple", "tomont",
        "demont",
    ];
    let subtotal: usize = listed
        .iter()
        .map(|key| {
            d.families_by_family
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, n)| *n)
                .unwrap_or(0)
        })
        .sum();
    let _ = writeln!(
        out,
        "  subtotal over the vocabulary the brief lists ({}): {subtotal}",
        listed.join(", ")
    );
    let _ = writeln!(out);
    if !rep.dropped_named.is_empty() {
        let _ = writeln!(out, "== named theorems dropped by the rule ==");
        for (reason, n) in &rep.dropped_named {
            let _ = writeln!(out, "  {n:>4}  {reason}");
        }
        let _ = writeln!(out);
    }
    let _ = writeln!(out, "== drop decomposition (all top-level declarations) ==");
    for (reason, n) in &d.reason_counts {
        let _ = writeln!(out, "  {n:>5}  {reason}");
    }
    let _ = writeln!(
        out,
        "  {:>5}  declarations whose name contains _CORRECT",
        d.named_correct_decls
    );
    let _ = writeln!(
        out,
        "  {:>5}  ... of which SUBROUTINE_CORRECT",
        d.subroutine_correct_decls
    );
    let _ = writeln!(out);
    if let Some(c) = &rep.corpus {
        let _ = writeln!(out, "== 5. cross-check against collected corpus ==");
        let _ = writeln!(out, "  corpus: {}", c.path);
        let _ = writeln!(
            out,
            "  problems total: {}  categories: {}",
            c.corpus_total_problems,
            dict_text(&c.corpus_categories)
        );
        let _ = writeln!(
            out,
            "  corpus FC: {} {}",
            c.corpus_fc_total,
            dict_text(&c.corpus_fc_by_arch)
        );
        let _ = writeln!(
            out,
            "  direct FC: {} {}",
            c.direct_fc_total,
            dict_text(&c.direct_fc_by_arch)
        );
        let _ = writeln!(out, "  matched (file, theorem): {}", c.matched);
        let _ = writeln!(
            out,
            "  only direct: {}   only corpus: {}",
            c.only_direct, c.only_corpus
        );
        if !c.only_direct_examples.is_empty() {
            let _ = writeln!(out, "    only-direct examples:");
            for tuple in c.only_direct_examples.iter().take(10) {
                let _ = writeln!(out, "      {}", tuple_text(tuple));
            }
        }
        if !c.only_corpus_examples.is_empty() {
            let _ = writeln!(out, "    only-corpus examples:");
            for tuple in c.only_corpus_examples.iter().take(10) {
                let _ = writeln!(out, "      {}", tuple_text(tuple));
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// JSON output
// ---------------------------------------------------------------------------

enum J {
    Null,
    Int(i64),
    Float(f64),
    Str(String),
    Arr(Vec<J>),
    Obj(BTreeMap<String, J>),
}

fn obj(entries: Vec<(&str, J)>) -> J {
    J::Obj(
        entries
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
    )
}

fn num_j(n: Option<Num>) -> J {
    match n {
        None => J::Null,
        Some(Num::Int(x)) => J::Int(x),
        Some(Num::Float(x)) => J::Float(x),
    }
}

fn usize_j(n: usize) -> J {
    J::Int(n as i64)
}

fn opt_usize_j(n: Option<usize>) -> J {
    match n {
        Some(x) => J::Int(x as i64),
        None => J::Null,
    }
}

fn summary_j(s: &Summary) -> J {
    let bands = J::Obj(
        s.bands
            .iter()
            .map(|(k, v)| (k.clone(), J::Int(*v as i64)))
            .collect(),
    );
    obj(vec![
        ("n", usize_j(s.n)),
        ("median", num_j(s.median)),
        ("q1", num_j(s.q1)),
        ("q3", num_j(s.q3)),
        ("min", opt_usize_j(s.min)),
        ("max", opt_usize_j(s.max)),
        (
            "mean",
            match s.mean {
                Some(x) => J::Float(x),
                None => J::Null,
            },
        ),
        ("bands", bands),
    ])
}

fn map_j(map: &BTreeMap<String, usize>) -> J {
    J::Obj(
        map.iter()
            .map(|(k, v)| (k.clone(), J::Int(*v as i64)))
            .collect(),
    )
}

fn pairs_j(pairs: &[(String, usize)]) -> J {
    J::Obj(
        pairs
            .iter()
            .map(|(k, v)| (k.clone(), J::Int(*v as i64)))
            .collect(),
    )
}

fn report_j(rep: &Report) -> J {
    let d = &rep.direct;
    let families = obj(vec![
        ("classified", usize_j(d.families_classified)),
        ("unclassified", usize_j(d.families_unclassified)),
        ("by_family", pairs_j(&d.families_by_family)),
        (
            "by_family_arch",
            J::Obj(
                d.families_by_arch
                    .iter()
                    .map(|(k, v)| (k.clone(), map_j(v)))
                    .collect(),
            ),
        ),
    ]);
    let long_tail = obj(vec![
        ("n", usize_j(d.long_tail_n)),
        ("loop", usize_j(d.long_tail_loop)),
        ("straight_line", usize_j(d.long_tail_straight)),
        (
            "loop_share_pct",
            match d.long_tail_loop_share_pct {
                Some(x) => J::Float(x),
                None => J::Null,
            },
        ),
    ]);
    let direct = obj(vec![
        ("fc_total", usize_j(d.fc_total)),
        ("fc_by_arch", map_j(&d.fc_by_arch)),
        ("fc_unique_goal", usize_j(d.fc_unique_goal)),
        (
            "fc_duplicate_goal_extra",
            usize_j(d.fc_duplicate_goal_extra),
        ),
        ("straight_line", usize_j(d.straight_line)),
        ("loop", usize_j(d.loop_count)),
        (
            "straight_line_share_pct",
            match d.straight_line_share_pct {
                Some(x) => J::Float(x),
                None => J::Null,
            },
        ),
        (
            "loop_share_pct",
            match d.loop_share_pct {
                Some(x) => J::Float(x),
                None => J::Null,
            },
        ),
        ("equiv_named", usize_j(d.equiv_named)),
        ("subroutine_safe_named", usize_j(d.subroutine_safe_named)),
        ("ensures2_goal", usize_j(d.ensures2_goal)),
        ("reason_counts", pairs_j(&d.reason_counts)),
        ("named_correct_decls", usize_j(d.named_correct_decls)),
        (
            "subroutine_correct_decls",
            usize_j(d.subroutine_correct_decls),
        ),
        ("length_tactic", summary_j(&d.length_tactic)),
        ("length_span", summary_j(&d.length_span)),
        (
            "length_by_arch",
            J::Obj(
                d.length_by_arch
                    .iter()
                    .map(|(k, v)| (k.to_string(), summary_j(v)))
                    .collect(),
            ),
        ),
        (
            "shape_by_arch",
            J::Obj(
                d.shape_by_arch
                    .iter()
                    .map(|(arch, straight_n, loop_n)| {
                        (
                            arch.to_string(),
                            obj(vec![
                                ("straight_line", usize_j(*straight_n)),
                                ("loop", usize_j(*loop_n)),
                            ]),
                        )
                    })
                    .collect(),
            ),
        ),
        (
            "length_by_shape",
            J::Obj(
                d.length_by_shape
                    .iter()
                    .map(|(k, v)| (k.to_string(), summary_j(v)))
                    .collect(),
            ),
        ),
        ("long_tail_300", long_tail),
        ("families", families),
    ]);

    let mut top = vec![
        ("repo", J::Str(rep.repo.clone())),
        ("commit", J::Str(rep.commit.clone())),
        ("direct", direct),
        ("dropped_named", pairs_j(&rep.dropped_named)),
        ("all_files_scanned", usize_j(rep.all_files_scanned)),
        ("all_declarations", usize_j(rep.all_declarations)),
    ];

    if let Some(c) = &rep.corpus {
        let examples = |items: &[(String, String, String)]| -> J {
            J::Arr(
                items
                    .iter()
                    .map(|(a, b, n)| {
                        J::Arr(vec![
                            J::Str(a.clone()),
                            J::Str(b.clone()),
                            J::Str(n.clone()),
                        ])
                    })
                    .collect(),
            )
        };
        top.push((
            "corpus",
            obj(vec![
                ("path", J::Str(c.path.clone())),
                ("corpus_total_problems", usize_j(c.corpus_total_problems)),
                ("corpus_categories", map_j(&c.corpus_categories)),
                ("corpus_fc_total", usize_j(c.corpus_fc_total)),
                ("corpus_fc_by_arch", map_j(&c.corpus_fc_by_arch)),
                ("direct_fc_total", usize_j(c.direct_fc_total)),
                ("direct_fc_by_arch", map_j(&c.direct_fc_by_arch)),
                ("matched", usize_j(c.matched)),
                ("only_direct", usize_j(c.only_direct)),
                ("only_corpus", usize_j(c.only_corpus)),
                ("only_direct_examples", examples(&c.only_direct_examples)),
                ("only_corpus_examples", examples(&c.only_corpus_examples)),
            ]),
        ));
    }

    obj(top)
}

fn json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c if c.is_ascii() => out.push(c),
            c => {
                let mut units = [0u16; 2];
                for unit in c.encode_utf16(&mut units) {
                    out.push_str(&format!("\\u{unit:04x}"));
                }
            }
        }
    }
    out.push('"');
    out
}

fn indent(out: &mut String, level: usize) {
    for _ in 0..level {
        out.push_str("  ");
    }
}

fn dump(value: &J, out: &mut String, level: usize) {
    match value {
        J::Null => out.push_str("null"),
        J::Int(n) => out.push_str(&n.to_string()),
        J::Float(x) => out.push_str(&fmt_float(*x)),
        J::Str(s) => out.push_str(&json_string(s)),
        J::Arr(items) => {
            if items.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push_str("[\n");
            for (index, item) in items.iter().enumerate() {
                indent(out, level + 1);
                dump(item, out, level + 1);
                if index + 1 < items.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            indent(out, level);
            out.push(']');
        }
        J::Obj(map) => {
            if map.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push_str("{\n");
            for (index, (key, item)) in map.iter().enumerate() {
                indent(out, level + 1);
                out.push_str(&json_string(key));
                out.push_str(": ");
                dump(item, out, level + 1);
                if index + 1 < map.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            indent(out, level);
            out.push('}');
        }
    }
}

fn dump_json(value: &J) -> String {
    let mut out = String::new();
    dump(value, &mut out, 0);
    out
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

const USAGE: &str = "\
Usage: fcdist CHECKOUT [--corpus PATH] [--json]

Recount s2n-bignum FC theorems with the benchmark's rule.";

pub fn run(args: &[String]) -> i32 {
    let mut positional: Vec<String> = Vec::new();
    let mut corpus: Option<String> = None;
    let mut json_out = false;

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        let (name, inline) = match arg.split_once('=') {
            Some((n, v)) if n.starts_with("--") => (n, Some(v.to_string())),
            _ => (arg.as_str(), None),
        };
        match name {
            "--json" if inline.is_none() => json_out = true,
            "-h" | "--help" if inline.is_none() => {
                println!("{USAGE}");
                return 0;
            }
            "--corpus" => {
                let value = match inline {
                    Some(v) => v,
                    None => {
                        i += 1;
                        match args.get(i) {
                            Some(v) => v.clone(),
                            None => {
                                eprintln!("fcdist: error: --corpus needs a value");
                                return 2;
                            }
                        }
                    }
                };
                corpus = Some(value);
            }
            other if other.starts_with('-') && other.len() > 1 => {
                eprintln!("fcdist: error: unrecognized argument: {other}");
                eprintln!("{USAGE}");
                return 2;
            }
            _ => positional.push(arg.clone()),
        }
        i += 1;
    }

    if positional.len() != 1 {
        eprintln!("fcdist: error: exactly one checkout path is required");
        eprintln!("{USAGE}");
        return 2;
    }

    let checkout = Path::new(&positional[0]);
    let corpus_path = corpus.as_ref().map(|s| Path::new(s.as_str()));
    let report = match build_report(checkout, corpus_path) {
        Ok(report) => report,
        Err(message) => {
            eprintln!("{message}");
            return 1;
        }
    };

    if json_out {
        println!("{}", dump_json(&report_j(&report)));
    } else {
        print!("{}", report_text(&report));
    }
    0
}

/// Build and render the report without printing it. Used by the CLI and by the
/// differential test against the recorded Python output.
pub fn render(checkout: &Path, corpus: Option<&Path>, json_out: bool) -> Result<String, String> {
    let report = build_report(checkout, corpus)?;
    Ok(if json_out {
        format!("{}\n", dump_json(&report_j(&report)))
    } else {
        report_text(&report)
    })
}
