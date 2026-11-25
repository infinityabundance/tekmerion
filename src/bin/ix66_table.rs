//! Regenerate the IX.66 §3.3 close table from the receipt JSON, and assert that
//! the **printed** close total equals the sum of the per-task counts.
//!
//! Why this exists (XV.0.4): `research/receipts/ix66-bridge-split.md` said "30
//! of 30 closes" in §6 while its own per-task table listed 21 closes across 22
//! CSAT attempts. The inference totals reconciled; the close-count sentence did
//! not. The original table was produced by `build66.awk`, which lives under
//! `.bench/` and is git-ignored, so it cannot be the check that every future
//! close count is backed by data. This binary is the checked-in generator: it
//! reads `research/receipts/ix66-bridge-split.json` (the receipt of record) and
//! refuses to print a total that its own rows do not add up to.
//!
//! Invariants, all asserted:
//!
//! - per task, `sat_closes == closes.len()` (one measured close entry each);
//! - per task, `csat_attempts == word_goal_sizes.len()` (one entry per attempt);
//! - the sum of the per-task `sat_closes` equals the total re-parsed from the
//!   rendered table, so the printed number is the sum and not a typed constant.
//!
//! Usage: `ix66_table [RECEIPT.json]` (default: the receipt of record).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use tekmerion::tools::json::{self, Json};

#[derive(Debug, Clone, PartialEq, Eq)]
struct TaskRow {
    task: String,
    attempts: usize,
    closes: usize,
    close_entries: usize,
    word_goals: usize,
}

fn read_u64(v: Option<&Json>, what: &str, task: &str) -> Result<usize, String> {
    match v {
        Some(Json::Num(n)) if *n >= 0.0 && n.fract() == 0.0 => Ok(*n as usize),
        _ => Err(format!("{task}: {what} is not a non-negative integer")),
    }
}

fn rows_from_receipt(text: &str) -> Result<Vec<TaskRow>, String> {
    let j = json::parse(text.as_bytes()).map_err(|e| format!("receipt JSON: {e}"))?;
    let tasks = j
        .get("tasks")
        .and_then(Json::as_array)
        .ok_or_else(|| "receipt JSON has no tasks[]".to_string())?;
    let mut out = Vec::with_capacity(tasks.len());
    for t in tasks {
        let task = t
            .get("task")
            .and_then(Json::as_str)
            .ok_or_else(|| "task entry without a name".to_string())?
            .to_string();
        let close_entries = t
            .get("closes")
            .and_then(Json::as_array)
            .ok_or_else(|| format!("{task}: no closes[]"))?
            .len();
        let word_goals = t
            .get("word_goal_sizes")
            .and_then(Json::as_array)
            .ok_or_else(|| format!("{task}: no word_goal_sizes[]"))?
            .len();
        out.push(TaskRow {
            attempts: read_u64(t.get("csat_attempts"), "csat_attempts", &task)?,
            closes: read_u64(t.get("sat_closes"), "sat_closes", &task)?,
            close_entries,
            word_goals,
            task,
        });
    }
    Ok(out)
}

/// The §3.3 close column, one row per task.
fn render(rows: &[TaskRow]) -> String {
    let mut out = String::new();
    out.push_str("| task | SAT closes / CSAT attempts |\n|---|---:|\n");
    for r in rows {
        out.push_str(&format!(
            "| `{}` | {} / {} |\n",
            r.task, r.closes, r.attempts
        ));
    }
    out
}

/// Re-read the rendered table and total the `closes / attempts` cells, so the
/// printed total is checked against the text a reader sees.
fn totals_from_table(table: &str) -> Result<(usize, usize), String> {
    let mut closes = 0usize;
    let mut attempts = 0usize;
    let mut seen = 0usize;
    for line in table.lines() {
        if !line.starts_with("| `") {
            // Skip the header and the separator row.
            continue;
        }
        let Some(cell) = line.rsplit('|').nth(1) else {
            continue;
        };
        let Some((c, a)) = cell.trim().split_once(" / ") else {
            continue;
        };
        let c: usize = c
            .parse()
            .map_err(|_| format!("close cell is not a number: {cell:?}"))?;
        let a: usize = a
            .parse()
            .map_err(|_| format!("attempt cell is not a number: {cell:?}"))?;
        closes += c;
        attempts += a;
        seen += 1;
    }
    if seen != table.lines().filter(|l| l.starts_with("| `")).count() {
        return Err("a close row did not parse".to_string());
    }
    Ok((closes, attempts))
}

fn check(rows: &[TaskRow]) -> Result<String, String> {
    for r in rows {
        if r.closes != r.close_entries {
            return Err(format!(
                "{}: sat_closes={} but the closes[] list has {} entries; the \
                 printed count would not be backed by data",
                r.task, r.closes, r.close_entries
            ));
        }
        if r.attempts != r.word_goals {
            return Err(format!(
                "{}: csat_attempts={} but word_goal_sizes[] has {} entries",
                r.task, r.attempts, r.word_goals
            ));
        }
    }
    let table = render(rows);
    let (printed_closes, printed_attempts) = totals_from_table(&table)?;
    let sum_closes: usize = rows.iter().map(|r| r.closes).sum();
    let sum_attempts: usize = rows.iter().map(|r| r.attempts).sum();
    if printed_closes != sum_closes || printed_attempts != sum_attempts {
        return Err(format!(
            "printed total {} / {} does not equal the per-task sums {} / {}",
            printed_closes, printed_attempts, sum_closes, sum_attempts
        ));
    }
    Ok(format!(
        "{table}\nCLOSES total={printed_closes} attempts={printed_attempts} \
         tasks={}\n",
        rows.len()
    ))
}

fn default_receipt() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("research/receipts/ix66-bridge-split.json")
}

fn main() -> ExitCode {
    let path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(default_receipt);
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("ix66_table: {}: {e}", path.display());
            return ExitCode::from(2);
        }
    };
    let rows = match rows_from_receipt(&text) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("ix66_table: {e}");
            return ExitCode::from(1);
        }
    };
    match check(&rows) {
        Ok(report) => {
            print!("{report}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("ix66_table: {e}");
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(task: &str, attempts: usize, closes: usize) -> TaskRow {
        TaskRow {
            task: task.to_string(),
            attempts,
            closes,
            close_entries: closes,
            word_goals: attempts,
        }
    }

    #[test]
    fn the_printed_total_is_the_sum_of_the_per_task_counts() {
        let rows = vec![row("a", 4, 4), row("b", 4, 3), row("c", 2, 2)];
        let report = check(&rows).expect("consistent rows");
        assert!(
            report.contains("CLOSES total=9 attempts=10 tasks=3"),
            "{report}"
        );
        // The total is re-parsed from the rendered table, not typed.
        let table = render(&rows);
        assert_eq!(totals_from_table(&table), Ok((9, 10)));
    }

    #[test]
    fn a_count_without_a_backing_close_entry_is_refused() {
        let mut bad = row("a", 4, 4);
        bad.close_entries = 3;
        let err = check(&[bad]).unwrap_err();
        assert!(err.contains("not be backed by data"), "{err}");
    }

    #[test]
    fn the_receipt_of_record_totals_21_closes_across_22_attempts() {
        let text = std::fs::read_to_string(default_receipt()).expect("ix66 receipt JSON");
        let rows = rows_from_receipt(&text).expect("receipt rows");
        assert_eq!(rows.len(), 6, "six measured theorems");
        let report = check(&rows).expect("the receipt is internally consistent");
        assert!(
            report.contains("CLOSES total=21 attempts=22 tasks=6"),
            "{report}"
        );
        // XV.0.4: the corrected prose must state the same numbers.
        let prose = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("research/receipts/ix66-bridge-split.md"),
        )
        .expect("ix66 receipt MD");
        assert!(
            prose.contains("21 of 21 closes across 22 CSAT attempts"),
            "the §6 sentence was not corrected to the data"
        );
        // The uncorrected *claim*, not the correction note that quotes it.
        assert!(
            !prose.contains("theorems (30 of 30"),
            "the uncorrected 30-of-30 claim is still present"
        );
        assert!(
            prose.contains("Correction 2026-11-25 (XV.0.4)"),
            "the correction note is missing"
        );
    }
}
