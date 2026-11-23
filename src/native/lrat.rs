//! LRAT parsing and checking.
//!
//! The supported fragment is linear RAT (LRAT) with **RUP** additions only:
//!
//! ```text
//! <id> <lit>* 0 <hint>* 0      addition
//! d <id>* 0                    deletion
//! <id> d <id>* 0               CaDiCaL deletion annotation
//! ```
//!
//! The third form is emitted by CaDiCaL 2.1.2 with `--lrat`: the leading id must
//! be the most recently added clause id, and the record deletes the listed
//! clauses. It is a deletion, not an addition, and is checked as such.
//!
//! An addition is accepted only if, assuming the negation of its clause, the
//! hinted clauses propagate to a conflict in the given order. Every hint must be a
//! still-present clause that is unit under the current assignment; the last hint
//! must be falsified. Hints after a conflict are refused. Negative hints denote RAT
//! and are refused, because the trusted base does not include RAT.
//!
//! Fail-closed details:
//!
//! * clause ids must be fresh and greater than every id seen so far;
//! * literals must be non-zero and within the CNF's declared variable range;
//! * duplicate literals and tautological clauses are refused;
//! * deletion of an absent clause is refused;
//! * a truncated line, a missing empty clause, or any line after the empty clause
//!   is refused.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::native::Cnf;

/// Why an LRAT refutation was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LratError {
    /// R-L01: a line is not in the grammar.
    Malformed { line: usize, detail: &'static str },
    /// R-L02: the file ends in the middle of a record or without a final newline.
    Truncated { line: usize },
    /// R-L03: a record follows the empty clause.
    TrailingGarbage { line: usize },
    /// R-L04: an integer is not canonical.
    BadInteger { line: usize, token: String },
    /// R-L05: a literal is zero in a clause, or out of the CNF range.
    LiteralOutOfRange { line: usize, lit: i64 },
    /// R-L06: a clause id is reused.
    DuplicateClauseId { line: usize, id: u64 },
    /// R-L07: a hint names no clause.
    UnknownClause { line: usize, id: u64 },
    /// R-L08: a hint names a deleted clause.
    DeletedClause { line: usize, id: u64 },
    /// R-L09: a hinted clause is satisfied, not unit, or comes after the conflict.
    BadHint {
        line: usize,
        id: i64,
        detail: &'static str,
    },
    /// R-L10: a negative hint requires RAT, which is outside the trusted base.
    RatUnsupported { line: usize },
    /// R-L11: unit propagation did not reach a conflict.
    NotRup { line: usize },
    /// R-L12: a clause repeats a literal or is tautological.
    AmbiguousClause { line: usize, detail: &'static str },
    /// R-L13: the refutation never derives the empty clause.
    NoEmptyClause,
}

impl fmt::Display for LratError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LratError::Malformed { line, detail } => {
                write!(f, "R-L01: line {line}: {detail}")
            }
            LratError::Truncated { line } => write!(f, "R-L02: truncated at line {line}"),
            LratError::TrailingGarbage { line } => {
                write!(f, "R-L03: line {line} follows the empty clause")
            }
            LratError::BadInteger { line, token } => {
                write!(f, "R-L04: line {line}: bad integer {token:?}")
            }
            LratError::LiteralOutOfRange { line, lit } => {
                write!(f, "R-L05: line {line}: literal {lit} is out of range")
            }
            LratError::DuplicateClauseId { line, id } => {
                write!(f, "R-L06: line {line}: clause id {id} is reused")
            }
            LratError::UnknownClause { line, id } => {
                write!(f, "R-L07: line {line}: unknown clause {id}")
            }
            LratError::DeletedClause { line, id } => {
                write!(f, "R-L08: line {line}: clause {id} was deleted")
            }
            LratError::BadHint { line, id, detail } => {
                write!(f, "R-L09: line {line}: hint {id} {detail}")
            }
            LratError::RatUnsupported { line } => {
                write!(
                    f,
                    "R-L10: line {line}: RAT hint is outside the trusted base"
                )
            }
            LratError::NotRup { line } => {
                write!(
                    f,
                    "R-L11: line {line}: hints do not propagate to a conflict"
                )
            }
            LratError::AmbiguousClause { line, detail } => {
                write!(f, "R-L12: line {line}: {detail}")
            }
            LratError::NoEmptyClause => write!(f, "R-L13: no empty clause was derived"),
        }
    }
}

impl std::error::Error for LratError {}

/// Measured work, for reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LratStats {
    pub additions: u64,
    pub deletions: u64,
    pub hints: u64,
    pub max_clause_id: u64,
}

/// Check an LRAT refutation of `cnf`.
///
/// `text` must be ASCII/UTF-8 LRAT with a final newline.
pub fn check(cnf: &Cnf, text: &str) -> Result<LratStats, LratError> {
    if !text.ends_with('\n') {
        return Err(LratError::Truncated { line: 1 });
    }
    if text.len() == 1 {
        return Err(LratError::Truncated { line: 1 });
    }
    let body = &text[..text.len() - 1];
    if body.is_empty() {
        return Err(LratError::Truncated { line: 1 });
    }

    let mut clauses: BTreeMap<u64, Vec<i32>> = BTreeMap::new();
    for (i, clause) in cnf.clauses().iter().enumerate() {
        clauses.insert((i + 1) as u64, clause.clone());
    }
    let mut deleted: BTreeSet<u64> = BTreeSet::new();
    let mut max_id = cnf.clauses().len() as u64;
    let mut stats = LratStats {
        max_clause_id: max_id,
        ..LratStats::default()
    };
    let mut empty_at: Option<usize> = None;
    let mut last_added: Option<u64> = None;

    for (idx, line) in body.split('\n').enumerate() {
        let line_no = idx + 1;
        if line.is_empty() {
            return Err(LratError::Malformed {
                line: line_no,
                detail: "empty line",
            });
        }
        if empty_at.is_some() {
            return Err(LratError::TrailingGarbage { line: line_no });
        }

        if let Some(rest) = line.strip_prefix("d ") {
            let ids = parse_ids(rest, line_no)?;
            for id in ids {
                if deleted.contains(&id) {
                    return Err(LratError::DeletedClause { line: line_no, id });
                }
                if !clauses.contains_key(&id) {
                    return Err(LratError::UnknownClause { line: line_no, id });
                }
                clauses.remove(&id);
                deleted.insert(id);
                stats.deletions += 1;
            }
            continue;
        }

        let (id_s, rest) = token(line, line_no)?;
        let id = parse_u64(id_s, line_no)?;

        // CaDiCaL's `--lrat` annotates a deletion with the id of the clause it
        // follows: `<id> d <deleted ids> 0`. The leading id must be the most
        // recently added clause id; it is not a new clause.
        if let Some(deleted_rest) = rest.strip_prefix("d ") {
            if last_added != Some(id) {
                return Err(LratError::Malformed {
                    line: line_no,
                    detail: "deletion annotation does not follow that clause id",
                });
            }
            let ids = parse_ids(deleted_rest, line_no)?;
            for id in ids {
                if deleted.contains(&id) {
                    return Err(LratError::DeletedClause { line: line_no, id });
                }
                if !clauses.contains_key(&id) {
                    return Err(LratError::UnknownClause { line: line_no, id });
                }
                clauses.remove(&id);
                deleted.insert(id);
                stats.deletions += 1;
            }
            continue;
        }

        if id == 0 {
            return Err(LratError::BadInteger {
                line: line_no,
                token: id.to_string(),
            });
        }
        if id <= max_id {
            return Err(LratError::DuplicateClauseId { line: line_no, id });
        }

        let (clause, rest) = parse_literals(rest, cnf.num_vars(), line_no)?;
        let (hints, rest) = parse_hints(rest, line_no)?;
        if !rest.is_empty() {
            return Err(LratError::Malformed {
                line: line_no,
                detail: "tokens after the hint terminator",
            });
        }

        check_rup(&clauses, &deleted, cnf.num_vars(), &clause, &hints, line_no)?;
        stats.additions += 1;
        stats.hints += hints.len() as u64;
        stats.max_clause_id = id;
        max_id = id;
        last_added = Some(id);

        let is_empty = clause.is_empty();
        clauses.insert(id, clause);
        if is_empty {
            empty_at = Some(line_no);
        }
    }

    if empty_at.is_none() {
        return Err(LratError::NoEmptyClause);
    }
    Ok(stats)
}

fn token(s: &str, line: usize) -> Result<(&str, &str), LratError> {
    if s.is_empty() {
        return Err(LratError::Malformed {
            line,
            detail: "missing field",
        });
    }
    match s.find(' ') {
        Some(i) => {
            if i == 0 {
                return Err(LratError::Malformed {
                    line,
                    detail: "empty field",
                });
            }
            Ok((&s[..i], &s[i + 1..]))
        }
        None => Ok((s, "")),
    }
}

fn parse_ids(s: &str, line: usize) -> Result<Vec<u64>, LratError> {
    let mut out = Vec::new();
    let mut rest = s;
    loop {
        let (tok, next) = token(rest, line)?;
        let v = parse_u64(tok, line)?;
        if v == 0 {
            if !next.is_empty() {
                return Err(LratError::Malformed {
                    line,
                    detail: "tokens after deletion terminator",
                });
            }
            return Ok(out);
        }
        out.push(v);
        rest = next;
        if rest.is_empty() {
            return Err(LratError::Truncated { line });
        }
    }
}

fn parse_literals(s: &str, num_vars: u32, line: usize) -> Result<(Vec<i32>, &str), LratError> {
    let mut out: Vec<i32> = Vec::new();
    let mut seen: BTreeSet<i32> = BTreeSet::new();
    let mut rest = s;
    loop {
        let (tok, next) = token(rest, line)?;
        let v = parse_i64(tok, line)?;
        if v == 0 {
            return Ok((out, next));
        }
        if v.unsigned_abs() > num_vars as u64 {
            return Err(LratError::LiteralOutOfRange { line, lit: v });
        }
        let lit = v as i32;
        if !seen.insert(lit) {
            return Err(LratError::AmbiguousClause {
                line,
                detail: "repeated literal",
            });
        }
        if seen.contains(&-lit) {
            return Err(LratError::AmbiguousClause {
                line,
                detail: "tautological clause",
            });
        }
        out.push(lit);
        rest = next;
        if rest.is_empty() {
            return Err(LratError::Truncated { line });
        }
    }
}

fn parse_hints(s: &str, line: usize) -> Result<(Vec<i64>, &str), LratError> {
    let mut out = Vec::new();
    let mut rest = s;
    loop {
        let (tok, next) = token(rest, line)?;
        let v = parse_i64(tok, line)?;
        if v == 0 {
            return Ok((out, next));
        }
        if v < 0 {
            return Err(LratError::RatUnsupported { line });
        }
        out.push(v);
        rest = next;
        if rest.is_empty() {
            return Err(LratError::Truncated { line });
        }
    }
}

fn check_rup(
    clauses: &BTreeMap<u64, Vec<i32>>,
    deleted: &BTreeSet<u64>,
    num_vars: u32,
    clause: &[i32],
    hints: &[i64],
    line: usize,
) -> Result<(), LratError> {
    let mut assign = vec![0i8; num_vars as usize + 1];
    for &lit in clause {
        let var = lit.unsigned_abs() as usize;
        // Assume the clause is false: positive literal -> variable false,
        // negative literal -> variable true.
        assign[var] = if lit > 0 { -1 } else { 1 };
    }

    let mut conflict = false;
    for (i, &hint) in hints.iter().enumerate() {
        if hint <= 0 {
            return Err(LratError::RatUnsupported { line });
        }
        let id = hint as u64;
        if deleted.contains(&id) {
            return Err(LratError::DeletedClause { line, id });
        }
        let hinted = clauses
            .get(&id)
            .ok_or(LratError::UnknownClause { line, id })?;

        let mut unassigned: Option<i32> = None;
        let mut open = 0usize;
        let mut satisfied = false;
        for &lit in hinted {
            let var = lit.unsigned_abs() as usize;
            match assign[var] {
                0 => {
                    open += 1;
                    unassigned = Some(lit);
                }
                v if (v > 0) == (lit > 0) => satisfied = true,
                _ => {}
            }
        }
        if satisfied {
            return Err(LratError::BadHint {
                line,
                id: hint,
                detail: "is satisfied, not unit",
            });
        }
        if open == 0 {
            conflict = true;
            if i + 1 != hints.len() {
                return Err(LratError::BadHint {
                    line,
                    id: hint,
                    detail: "conflicts before the final hint",
                });
            }
            break;
        }
        if open > 1 {
            return Err(LratError::BadHint {
                line,
                id: hint,
                detail: "is not unit",
            });
        }
        let lit = unassigned.expect("open == 1");
        let var = lit.unsigned_abs() as usize;
        assign[var] = if lit > 0 { 1 } else { -1 };
    }

    if !conflict {
        return Err(LratError::NotRup { line });
    }
    Ok(())
}

fn parse_u64(tok: &str, line: usize) -> Result<u64, LratError> {
    if !canonical_digits(tok) {
        return Err(LratError::BadInteger {
            line,
            token: tok.to_string(),
        });
    }
    tok.parse::<u64>().map_err(|_| LratError::BadInteger {
        line,
        token: tok.to_string(),
    })
}

fn parse_i64(tok: &str, line: usize) -> Result<i64, LratError> {
    let digits = tok.strip_prefix('-').unwrap_or(tok);
    if !canonical_digits(digits) || (tok.starts_with('-') && digits == "0") {
        return Err(LratError::BadInteger {
            line,
            token: tok.to_string(),
        });
    }
    tok.parse::<i64>().map_err(|_| LratError::BadInteger {
        line,
        token: tok.to_string(),
    })
}

fn canonical_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) && (s.len() == 1 || !s.starts_with('0'))
}
