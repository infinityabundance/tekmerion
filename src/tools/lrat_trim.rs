//! Backward-closure trimming for caDiCaL text LRAT proofs.
//!
//! An LRAT file emitted by `cadical --no-binary --lrat` is a sequence of
//! *additions*
//!
//! ```text
//! <id> <lit> ... 0 <hint> ... 0
//! ```
//!
//! and *deletions*
//!
//! ```text
//! <id> d <id> ... 0
//! ```
//!
//! The last addition asserts the empty clause. `LRAT_PROVE_GEN` in HOL Light
//! (`Cadical/ldrup.ml`) replays every addition as a primitive inference and
//! leaves the deletion lines as bookkeeping. Most additions are not reachable
//! backwards from the empty clause, so they are replayed for nothing.
//!
//! This tool keeps exactly the additions reachable from the empty clause
//! through their positive RUP hints, in the original file order. Deletion
//! lines are dropped: keeping a clause cannot break a later step, so the
//! trimmed file replays with the same kernel code and no extra bookkeeping.
//! Every kept addition still has all of its hinted ancestors, which is the
//! property `lrat-trim` guarantees for RUP-only proofs.
//!
//! **Refusals agree with the OCaml trimmer.** `tools/csat_lib.ml` refuses a
//! file, leaving it untouched, when any hint is negative (RAT) or when the root
//! addition does not assert the empty clause, naming the reason
//! `skipped=negative_hint` / `skipped=no_empty_clause`. This module returns the
//! same reasons as data ([`TrimRefusal::reason`]) and never reports success on
//! either input; `tests/csat_trim_agreement.rs` runs both implementations on
//! the same texts and requires the same verdict. The checks are applied in the
//! OCaml order (malformed, empty, no additions, non-empty root, negative hint,
//! duplicate ids) so that a file which fails two of them reports the same
//! reason on both sides.
//!
//! **Not on the verdict path.** The trimmed proof is a measurement artefact:
//! it is replayed by the same kernel, and the kernel's theorem is compared
//! with the untrimmed replay's theorem by the measuring harness.

use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::Path;

/// Why a trim was refused. The `reason` token is the same string the OCaml
/// trimmer prints after `CSAT_TRIM skipped=`, so the two implementations can be
/// compared token for token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrimRefusal {
    /// A line that is neither an addition nor a deletion (`skipped=malformed`).
    Malformed { line: usize, text: String },
    /// No non-blank lines at all (`skipped=empty_lrat`).
    EmptyLrat,
    /// No addition lines (`skipped=no_additions`).
    NoAdditions,
    /// The last addition asserts a non-empty clause (`skipped=no_empty_clause`).
    NoEmptyClause { root: i64 },
    /// At least one hint is negative, i.e. a RAT step this closure does not
    /// follow (`skipped=negative_hint`).
    NegativeHint { count: usize },
    /// A clause id is defined twice (`skipped=duplicate_id`).
    DuplicateIds { count: usize },
    /// Reading or writing the file failed (`skipped=io_error` /
    /// `skipped=write_error`).
    Io { detail: String },
}

impl TrimRefusal {
    /// The machine-readable reason, identical to the OCaml `skipped=` token.
    pub fn reason(&self) -> &'static str {
        match self {
            TrimRefusal::Malformed { .. } => "malformed",
            TrimRefusal::EmptyLrat => "empty_lrat",
            TrimRefusal::NoAdditions => "no_additions",
            TrimRefusal::NoEmptyClause { .. } => "no_empty_clause",
            TrimRefusal::NegativeHint { .. } => "negative_hint",
            TrimRefusal::DuplicateIds { .. } => "duplicate_id",
            TrimRefusal::Io { .. } => "io_error",
        }
    }
}

impl fmt::Display for TrimRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrimRefusal::Malformed { line, text } => write!(
                f,
                "malformed: line {line} is not an LRAT addition or deletion: {text:?}"
            ),
            TrimRefusal::EmptyLrat => write!(f, "empty_lrat: the LRAT text is empty"),
            TrimRefusal::NoAdditions => {
                write!(f, "no_additions: the LRAT text has no addition lines")
            }
            TrimRefusal::NoEmptyClause { root } => write!(
                f,
                "no_empty_clause: the last addition (id {root}) does not assert \
                 the empty clause, so the root of the proof is not a refutation"
            ),
            TrimRefusal::NegativeHint { count } => write!(
                f,
                "negative_hint: {count} negative (RAT) hint(s) present; this trim \
                 follows only positive RUP hints, so the closure would be \
                 incomplete"
            ),
            TrimRefusal::DuplicateIds { count } => write!(
                f,
                "duplicate_id: the LRAT text redefines {count} clause id(s), so \
                 the hints are ambiguous"
            ),
            TrimRefusal::Io { detail } => write!(f, "io_error: {detail}"),
        }
    }
}

/// One parsed LRAT line. `raw` is the original line without its newline.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Line<'a> {
    raw: &'a str,
    id: i64,
    /// `Some(hints)` for an addition (empty when the line has no hints),
    /// `None` for a deletion.
    hints: Option<Vec<i64>>,
}

/// What a trim pass did. All byte counts are of the text as written, with one
/// `\n` per kept line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrimStats {
    /// Non-empty lines read.
    pub lines: usize,
    /// Addition lines read.
    pub additions: usize,
    /// Deletion lines read.
    pub deletions: usize,
    /// Addition lines kept.
    pub kept: usize,
    /// Bytes of the input (excluding the final newline convention is uniform:
    /// every line is counted with one newline).
    pub full_bytes: usize,
    /// Bytes of the output.
    pub trimmed_bytes: usize,
    /// Negative hints seen (RAT steps). A successful trim always leaves this
    /// at 0: any negative hint is a [`TrimRefusal::NegativeHint`], never a
    /// statistic of a "successful" trim.
    pub negative_hints: usize,
    /// Whether the root (last addition) asserts the empty clause. Always true
    /// on a successful trim; a non-empty root is refused.
    pub root_is_empty: bool,
}

impl TrimStats {
    /// `full_bytes / trimmed_bytes`, or `None` when nothing was kept.
    pub fn ratio(&self) -> Option<f64> {
        if self.trimmed_bytes == 0 {
            None
        } else {
            Some(self.full_bytes as f64 / self.trimmed_bytes as f64)
        }
    }
}

impl fmt::Display for TrimStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "lines={} additions={} deletions={} kept={} full_bytes={} trimmed_bytes={} \
             negative_hints={} root_empty={}",
            self.lines,
            self.additions,
            self.deletions,
            self.kept,
            self.full_bytes,
            self.trimmed_bytes,
            self.negative_hints,
            self.root_is_empty
        )
    }
}

/// Parse one non-empty line. Returns `None` for a line this format does not
/// recognise, so the caller can refuse instead of silently dropping data.
fn parse_line(line: &str) -> Option<Line<'_>> {
    let toks: Vec<&str> = line.split_whitespace().collect();
    if toks.len() < 2 {
        return None;
    }
    let id: i64 = toks[0].parse().ok()?;
    // OCaml refuses id 0 on both line kinds, so a zero id is malformed here too.
    if id == 0 {
        return None;
    }
    if toks[1] == "d" {
        // `<id> d <id> ... 0`
        for t in &toks[2..] {
            let v: i64 = t.parse().ok()?;
            if v < 0 {
                return None;
            }
        }
        return Some(Line {
            raw: line,
            id,
            hints: None,
        });
    }
    // `<id> <lit> ... 0 <hint> ... 0`
    let first_zero = toks.iter().position(|t| *t == "0")?;
    if first_zero == 0 {
        return None; // an id with no literal terminator
    }
    for t in &toks[1..first_zero] {
        let _: i64 = t.parse().ok()?;
    }
    let mut hints = Vec::new();
    for t in &toks[first_zero + 1..] {
        let v: i64 = t.parse().ok()?;
        if v != 0 {
            hints.push(v);
        }
    }
    Some(Line {
        raw: line,
        id,
        hints: Some(hints),
    })
}

/// Trim an LRAT text. Returns the trimmed text and the pass statistics.
///
/// The root is the last addition line in file order; for caDiCaL that line is
/// the empty-clause assertion. Deletions are never emitted.
///
/// Refuses rather than repairs: a negative (RAT) hint, a root that is not the
/// empty clause, an ambiguous id, or a malformed line is a [`TrimRefusal`], in
/// the same order and with the same reason token as `tools/csat_lib.ml`.
pub fn trim(text: &str) -> Result<(String, TrimStats), TrimRefusal> {
    let mut parsed: Vec<Line<'_>> = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let line = parse_line(line).ok_or_else(|| TrimRefusal::Malformed {
            line: index + 1,
            text: truncate(line, 80),
        })?;
        parsed.push(line);
    }
    if parsed.is_empty() {
        return Err(TrimRefusal::EmptyLrat);
    }

    let additions: Vec<usize> = parsed
        .iter()
        .enumerate()
        .filter(|(_, l)| l.hints.is_some())
        .map(|(i, _)| i)
        .collect();
    if additions.is_empty() {
        return Err(TrimRefusal::NoAdditions);
    }

    let root = *additions.last().expect("non-empty");
    let root_is_empty = literal_count(parsed[root].raw) == 0;
    if !root_is_empty {
        return Err(TrimRefusal::NoEmptyClause {
            root: parsed[root].id,
        });
    }

    // Conservative in exactly the same way as the OCaml trimmer: **any**
    // negative hint in the file refuses the trim, reachable or not, because a
    // RAT step is not followed and the backward closure would be incomplete.
    let negative_hints = parsed
        .iter()
        .filter_map(|l| l.hints.as_ref())
        .flatten()
        .filter(|h| **h < 0)
        .count();
    if negative_hints > 0 {
        return Err(TrimRefusal::NegativeHint {
            count: negative_hints,
        });
    }

    let mut index: HashMap<i64, usize> = HashMap::with_capacity(additions.len());
    let mut duplicate_ids = 0usize;
    for &pos in &additions {
        if index.insert(parsed[pos].id, pos).is_some() {
            duplicate_ids += 1;
        }
    }
    if duplicate_ids > 0 {
        return Err(TrimRefusal::DuplicateIds {
            count: duplicate_ids,
        });
    }

    // Backward closure from the empty-clause root over positive hints only.
    let mut keep: Vec<bool> = vec![false; parsed.len()];
    let mut stack = vec![root];
    while let Some(pos) = stack.pop() {
        if keep[pos] {
            continue;
        }
        keep[pos] = true;
        if let Some(hints) = &parsed[pos].hints {
            for h in hints {
                if *h > 0 {
                    if let Some(&j) = index.get(h) {
                        if !keep[j] {
                            stack.push(j);
                        }
                    }
                }
            }
        }
    }

    let mut out = String::with_capacity(text.len() / 2);
    let mut kept = 0usize;
    let mut full_bytes = 0usize;
    for (pos, line) in parsed.iter().enumerate() {
        full_bytes += line.raw.len() + 1;
        if keep[pos] {
            out.push_str(line.raw);
            out.push('\n');
            kept += 1;
        }
    }
    let stats = TrimStats {
        lines: parsed.len(),
        additions: additions.len(),
        deletions: parsed.len() - additions.len(),
        kept,
        full_bytes,
        trimmed_bytes: out.len(),
        negative_hints,
        root_is_empty,
    };
    Ok((out, stats))
}

/// Number of literals before the first `0` on an addition line.
fn literal_count(raw: &str) -> usize {
    raw.split_whitespace()
        .skip(1)
        .take_while(|t| *t != "0")
        .count()
}

fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        format!("{}...", &s[..n])
    }
}

/// Read `input`, write the trimmed proof to `output`, return the statistics.
pub fn trim_file(input: &Path, output: &Path) -> Result<TrimStats, TrimRefusal> {
    let text = fs::read_to_string(input).map_err(|e| TrimRefusal::Io {
        detail: format!("{}: {e}", input.display()),
    })?;
    let (trimmed, stats) = trim(&text)?;
    fs::write(output, trimmed).map_err(|e| TrimRefusal::Io {
        detail: format!("{}: {e}", output.display()),
    })?;
    Ok(stats)
}

const USAGE: &str = "\
Usage: lrat_trim INPUT.lrat OUTPUT.lrat

Keep only the LRAT additions reachable backwards from the empty clause through
positive RUP hints, in the original order, and drop every deletion line.";

/// CLI entry point. Exit 0 on success, 2 on usage error, 1 on failure.
pub fn run(args: &[String]) -> i32 {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("{USAGE}");
        return 0;
    }
    if args.len() != 2 {
        eprintln!("lrat_trim: expected 2 arguments, got {}", args.len());
        eprintln!("{USAGE}");
        return 2;
    }
    let input = Path::new(&args[0]);
    let output = Path::new(&args[1]);
    match trim_file(input, output) {
        Ok(stats) => {
            println!(
                "LRAT_TRIM in={} out={} {}",
                input.display(),
                output.display(),
                stats
            );
            0
        }
        Err(e) => {
            eprintln!("lrat_trim: {e}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kept_lines(text: &str) -> Vec<String> {
        let (out, _) = trim(text).expect("trim");
        out.lines().map(|l| l.to_string()).collect()
    }

    #[test]
    fn keeps_the_chain_and_drops_unused_additions() {
        let text = "\
1 1 0 0
2 2 0 0
9 -1 0 1 0
3 3 0 0
4 1 2 0 1 9 0
7 0 1 2 4 0
";
        // 3 is never hinted, so it is dropped. 9 is hinted by 4.
        assert_eq!(
            kept_lines(text),
            vec![
                "1 1 0 0",
                "2 2 0 0",
                "9 -1 0 1 0",
                "4 1 2 0 1 9 0",
                "7 0 1 2 4 0"
            ]
        );
    }

    #[test]
    fn deletion_lines_are_dropped_including_deletions_of_kept_clauses() {
        let text = "\
10 1 0 0
11 2 0 0
10 d 11 0
12 0 10 11 0
";
        assert_eq!(
            kept_lines(text),
            vec!["10 1 0 0", "11 2 0 0", "12 0 10 11 0"]
        );
    }

    #[test]
    fn order_is_the_original_file_order() {
        let text = "\
5 -2 0 7 0
7 2 0 0
1 1 0 0
8 0 5 0
";
        // Root 8 hints 5; 5 hints 7. 1 is unreachable.
        assert_eq!(kept_lines(text), vec!["5 -2 0 7 0", "7 2 0 0", "8 0 5 0"]);
    }

    #[test]
    fn a_negative_hint_is_refused_with_the_ocaml_reason() {
        // RAT steps are not followed, so a negative hint makes the closure
        // incomplete. The OCaml trimmer prints `skipped=negative_hint` and
        // leaves the file untouched; this refuses with the same token.
        let text = "\
1 1 0 0
2 2 0 0
3 0 -1 0
";
        let err = trim(text).unwrap_err();
        assert_eq!(err, TrimRefusal::NegativeHint { count: 1 });
        assert_eq!(err.reason(), "negative_hint");
        assert!(err.to_string().starts_with("negative_hint: "), "{err}");
    }

    #[test]
    fn a_negative_hint_anywhere_refuses_even_if_unreachable() {
        // The refusal is over the whole file, exactly as the OCaml trimmer
        // scans every hint: an unreachable RAT step still means this tool
        // cannot claim a complete RUP closure.
        let text = "\
1 1 0 0
2 2 0 0
9 -1 0 -2 0
3 0 1 0
";
        let err = trim(text).unwrap_err();
        assert_eq!(err.reason(), "negative_hint");
    }

    #[test]
    fn a_non_empty_root_is_refused_with_the_ocaml_reason() {
        let text = "\
1 1 0 0
2 2 0 0
4 1 2 0 1 2 0
";
        let err = trim(text).unwrap_err();
        assert_eq!(err, TrimRefusal::NoEmptyClause { root: 4 });
        assert_eq!(err.reason(), "no_empty_clause");
        assert!(err.to_string().starts_with("no_empty_clause: "), "{err}");
    }

    #[test]
    fn refusals_carry_distinct_named_reasons() {
        let cases: Vec<(&str, &str)> = vec![
            ("not lrat\n", "malformed"),
            ("", "empty_lrat"),
            ("1 d 1 0\n", "no_additions"),
            ("1 1 0 0\n2 1 0 1 0\n", "no_empty_clause"),
            ("1 1 0 0\n2 0 -1 0\n", "negative_hint"),
            ("1 1 0 0\n1 2 0 0\n2 0 1 0\n", "duplicate_id"),
        ];
        for (text, reason) in cases {
            let err = trim(text).unwrap_err();
            assert_eq!(err.reason(), reason, "{text:?} -> {err}");
        }
        // Malformed beats empty-root: the parse happens first, as in OCaml.
        let err = trim("0 1 0 0\n2 1 0 1 0\n").unwrap_err();
        assert_eq!(err.reason(), "malformed");
        // The root check beats the duplicate-id check, as in OCaml.
        let err = trim("1 1 0 0\n1 1 0 0\n2 1 0 1 0\n").unwrap_err();
        assert_eq!(err.reason(), "no_empty_clause");
    }

    #[test]
    fn the_root_is_the_last_addition_not_a_trailing_deletion() {
        let text = "\
1 1 0 0
2 0 1 0
3 d 1 0
";
        let (out, stats) = trim(text).expect("trim");
        assert!(stats.root_is_empty);
        assert_eq!(stats.kept, 2);
        assert_eq!(out, "1 1 0 0\n2 0 1 0\n");
    }

    #[test]
    fn stats_count_bytes_and_deletions() {
        let text = "1 1 0 0\n1 d 1 0\n2 0 1 0\n";
        let (out, stats) = trim(text).expect("trim");
        assert_eq!(stats.lines, 3);
        assert_eq!(stats.additions, 2);
        assert_eq!(stats.deletions, 1);
        assert_eq!(stats.kept, 2);
        assert_eq!(stats.full_bytes, text.len());
        assert_eq!(stats.trimmed_bytes, out.len());
        assert!(stats.root_is_empty);
        // A successful trim always checked both refusals and found neither.
        assert_eq!(stats.negative_hints, 0);
    }

    #[test]
    fn duplicate_ids_are_refused() {
        let text = "1 1 0 0\n1 2 0 0\n2 0 1 0\n";
        assert!(trim(text).is_err());
    }

    #[test]
    fn malformed_lines_are_refused() {
        assert!(trim("1 1 0 0\nnot lrat\n2 0 1 0\n").is_err());
        assert!(trim("").is_err());
        assert!(trim("1 d 1 0\n").is_err());
    }

    #[test]
    fn a_hint_to_a_cnf_clause_is_not_an_addition() {
        // Ids 1 and 2 are CNF clauses: they have no addition lines here.
        let text = "\
100 -3 0 1 2 0
101 0 100 0
";
        let (out, stats) = trim(text).expect("trim");
        assert_eq!(stats.kept, 2);
        assert_eq!(out, "100 -3 0 1 2 0\n101 0 100 0\n");
    }

    #[test]
    fn cli_rejects_wrong_argument_counts() {
        assert_eq!(run(&[]), 2);
        assert_eq!(run(&["a".into(), "b".into(), "c".into()]), 2);
        assert_eq!(run(&["--help".into()]), 0);
    }
}
