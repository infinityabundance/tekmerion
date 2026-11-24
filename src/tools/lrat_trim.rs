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
//! **Not on the verdict path.** The trimmed proof is a measurement artefact:
//! it is replayed by the same kernel, and the kernel's theorem is compared
//! with the untrimmed replay's theorem by the measuring harness.

use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::Path;

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
    /// Negative hints seen (RAT steps). This tool follows only positive RUP
    /// hints; a non-zero count means the closure may be incomplete and the
    /// caller must not trust the trimmed file.
    pub negative_hints: usize,
    /// Whether the root (last addition) asserts the empty clause.
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
pub fn trim(text: &str) -> Result<(String, TrimStats), String> {
    let mut parsed: Vec<Line<'_>> = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let line = parse_line(line).ok_or_else(|| {
            format!(
                "line {}: not an LRAT addition or deletion: {:?}",
                index + 1,
                truncate(line, 80)
            )
        })?;
        parsed.push(line);
    }
    if parsed.is_empty() {
        return Err("empty LRAT text".to_string());
    }

    let additions: Vec<usize> = parsed
        .iter()
        .enumerate()
        .filter(|(_, l)| l.hints.is_some())
        .map(|(i, _)| i)
        .collect();
    if additions.is_empty() {
        return Err("LRAT text has no addition lines".to_string());
    }

    let mut index: HashMap<i64, usize> = HashMap::with_capacity(additions.len());
    let mut duplicate_ids = 0usize;
    for &pos in &additions {
        if index.insert(parsed[pos].id, pos).is_some() {
            duplicate_ids += 1;
        }
    }
    if duplicate_ids > 0 {
        return Err(format!(
            "LRAT text redefines {duplicate_ids} clause id(s); refusing to trim"
        ));
    }

    let root = *additions.last().expect("non-empty");
    let root_is_empty = parsed[root]
        .hints
        .as_ref()
        .is_some_and(|_| literal_count(parsed[root].raw) == 0);

    let mut negative_hints = 0usize;
    let mut keep: Vec<bool> = vec![false; parsed.len()];
    let mut stack = vec![root];
    while let Some(pos) = stack.pop() {
        if keep[pos] {
            continue;
        }
        keep[pos] = true;
        if let Some(hints) = &parsed[pos].hints {
            for h in hints {
                if *h <= 0 {
                    negative_hints += 1;
                    continue;
                }
                if let Some(&j) = index.get(h) {
                    if !keep[j] {
                        stack.push(j);
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
pub fn trim_file(input: &Path, output: &Path) -> Result<TrimStats, String> {
    let text = fs::read_to_string(input).map_err(|e| format!("{}: {e}", input.display()))?;
    let (trimmed, stats) = trim(&text)?;
    fs::write(output, trimmed).map_err(|e| format!("{}: {e}", output.display()))?;
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
    fn negative_hints_are_counted_not_followed() {
        let text = "\
1 1 0 0
2 2 0 0
3 0 -1 0
";
        let (out, stats) = trim(text).expect("trim");
        assert_eq!(stats.negative_hints, 1);
        assert_eq!(out, "3 0 -1 0\n");
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
