//! # TEST ORACLE — not part of the trusted path
//!
//! This module is a deliberately naive second opinion on the logical core. It is
//! **never called by [`crate::native::check`]** and it is not used to construct a
//! [`crate::native::Theorem`]. Its only consumer is `tests/native_semantics.rs`,
//! where it is used to differentially test the signed checker.
//!
//! Independence is structural, not cosmetic:
//!
//! * the rule checks are re-implemented here rather than shared with `check.rs`;
//! * the LRAT verifier below does **not** use hints for propagation. Each added
//!   clause is checked by running unit propagation to a fixpoint over the whole
//!   live clause set. Hint annotations are still required to be well-formed
//!   references (positive, present, not deleted), because they are part of the
//!   file format, but their *order* is ignored;
//! * the DIMACS/LRAT line grammars are parsed by separate code, so a bug in
//!   `lrat.rs` does not propagate to the oracle;
//! * for tiny propositions (at most 20 free bits) the claimed proposition is
//!   evaluated under **every** assignment with a separate evaluator. A
//!   counterexample refuses the certificate even if some LRAT happens to be
//!   accepted.
//!
//! The oracle is intentionally incomplete: it declines the exhaustive semantic
//! check above 20 free bits, and it is not hardened. It is an oracle, not a
//! second product.
//!
//! `#[doc(hidden)]` keeps it out of the public documentation; the module is
//! compiled so integration tests can reach it. The test
//! `reference_oracle_is_not_on_the_trusted_path` scans the trusted modules for
//! any textual dependency on it.

#![doc(hidden)]
#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};

use crate::native::cert::{BlobKind, Certificate, Rule, Step};
use crate::native::{bitblast, Term, Value};

/// A refusal from the oracle. `code` is stable enough to assert on in tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefError {
    pub code: &'static str,
    pub detail: String,
}

fn err(code: &'static str, detail: impl Into<String>) -> RefError {
    RefError {
        code,
        detail: detail.into(),
    }
}

/// Decide acceptance with the naive oracle.
///
/// The certificate is assumed to have been built or parsed by the signed
/// front end; structural errors are re-detected where the oracle needs them.
pub fn accepts(cert: &Certificate) -> Result<(), RefError> {
    let mut proved: BTreeMap<u32, Term> = BTreeMap::new();
    let mut visiting: BTreeSet<u32> = BTreeSet::new();
    for step in cert.steps() {
        derive(cert, step.id(), &mut proved, &mut visiting)?;
    }
    if !proved.contains_key(&cert.theorem_id()) {
        return Err(err("R-OR08", "theorem step was not derived"));
    }
    Ok(())
}

fn derive(
    cert: &Certificate,
    id: u32,
    proved: &mut BTreeMap<u32, Term>,
    visiting: &mut BTreeSet<u32>,
) -> Result<(), RefError> {
    if proved.contains_key(&id) {
        return Ok(());
    }
    if !visiting.insert(id) {
        return Err(err("R-OR09", "premise cycle"));
    }
    let step = cert
        .step(id)
        .ok_or_else(|| err("R-OR10", "dangling premise"))?;
    for &p in step.premises() {
        derive(cert, p, proved, visiting)?;
    }
    let conclusion = apply_rule(cert, step, proved)?;
    proved.insert(id, conclusion);
    visiting.remove(&id);
    Ok(())
}

fn apply_rule(
    cert: &Certificate,
    step: &Step,
    proved: &BTreeMap<u32, Term>,
) -> Result<Term, RefError> {
    match step.rule() {
        Rule::Refl => match step.conclusion() {
            Term::Eq { left, right, .. } if left == right => Ok(step.conclusion().clone()),
            _ => Err(err(
                "R-OR11",
                "refl is not an equality with identical sides",
            )),
        },
        Rule::Sym => {
            let premise = premise(proved, step.premises()[0])?;
            let (a, b) =
                as_eq(premise).ok_or_else(|| err("R-OR12", "sym premise is not an equality"))?;
            let (x, y) = as_eq(step.conclusion())
                .ok_or_else(|| err("R-OR13", "sym conclusion is not an equality"))?;
            if x == b && y == a {
                Ok(step.conclusion().clone())
            } else {
                Err(err("R-OR14", "sym does not exchange the sides"))
            }
        }
        Rule::Trans => {
            let first = premise(proved, step.premises()[0])?;
            let second = premise(proved, step.premises()[1])?;
            let (a, b) =
                as_eq(first).ok_or_else(|| err("R-OR15", "trans first premise not equality"))?;
            let (c, d) =
                as_eq(second).ok_or_else(|| err("R-OR16", "trans second premise not equality"))?;
            if b != c {
                return Err(err("R-OR17", "trans premises do not share a middle"));
            }
            let (x, y) = as_eq(step.conclusion())
                .ok_or_else(|| err("R-OR18", "trans conclusion not equality"))?;
            if x == a && y == d {
                Ok(step.conclusion().clone())
            } else {
                Err(err("R-OR19", "trans conclusion is not the outer pair"))
            }
        }
        Rule::Congr => {
            let premise = premise(proved, step.premises()[0])?;
            let (a, b) =
                as_eq(premise).ok_or_else(|| err("R-OR20", "congr premise is not an equality"))?;
            let (left, right) = as_eq(step.conclusion())
                .ok_or_else(|| err("R-OR21", "congr conclusion is not an equality"))?;
            if one_operand_replaced(a, b, left, right) {
                Ok(step.conclusion().clone())
            } else {
                Err(err("R-OR22", "congr does not replace one add/sub operand"))
            }
        }
        Rule::Bitblast { cnf, lrat } => {
            let cnf_blob = cert
                .blob(*cnf, BlobKind::Cnf)
                .map_err(|e| err("R-OR23", e.to_string()))?;
            let lrat_blob = cert
                .blob(*lrat, BlobKind::Lrat)
                .map_err(|e| err("R-OR24", e.to_string()))?;
            let (num_vars, clauses) = parse_dimacs_ref(cnf_blob.bytes())?;
            let expected = bitblast(step.conclusion()).map_err(|e| err("R-OR26", e.to_string()))?;
            if num_vars != expected.num_vars() || clauses.as_slice() != expected.clauses() {
                return Err(err(
                    "R-OR27",
                    "attached CNF is not the canonical bit-blast of the conclusion",
                ));
            }
            check_lrat_naive(num_vars, &clauses, lrat_blob.bytes())?;
            // Independent semantic check for tiny widths. A false proposition is
            // refused even if the LRAT happens to be accepted.
            if let Some(witness) = falsifying_assignment(step.conclusion())? {
                return Err(err(
                    "R-OR28",
                    format!("proposition is false under assignment {witness:?}"),
                ));
            }
            Ok(step.conclusion().clone())
        }
    }
}

fn premise<'a>(proved: &'a BTreeMap<u32, Term>, id: u32) -> Result<&'a Term, RefError> {
    proved
        .get(&id)
        .ok_or_else(|| err("R-OR29", "premise was not derived"))
}

fn as_eq(term: &Term) -> Option<(&Term, &Term)> {
    match term {
        Term::Eq { left, right, .. } => Some((left, right)),
        _ => None,
    }
}

fn one_operand_replaced(a: &Term, b: &Term, left: &Term, right: &Term) -> bool {
    match (left, right) {
        (
            Term::Add {
                width: lw,
                left: ll,
                right: lr,
            },
            Term::Add {
                width: rw,
                left: rl,
                right: rr,
            },
        )
        | (
            Term::Sub {
                width: lw,
                left: ll,
                right: lr,
            },
            Term::Sub {
                width: rw,
                left: rl,
                right: rr,
            },
        ) => {
            lw == rw
                && ((ll.as_ref() == a && rl.as_ref() == b && lr == rr)
                    || (lr.as_ref() == a && rr.as_ref() == b && ll == rl))
        }
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Independent DIMACS parser (not shared with `bitblast.rs`).
// ---------------------------------------------------------------------------

const REF_MAX_VARS: u32 = i32::MAX as u32;
const REF_MAX_CLAUSES: usize = 4_000_000;

/// Parse canonical DIMACS into `(variable count, clauses)`.
pub fn parse_dimacs_ref(bytes: &[u8]) -> Result<(u32, Vec<Vec<i32>>), RefError> {
    let text = std::str::from_utf8(bytes).map_err(|_| err("R-OR63", "DIMACS is not UTF-8"))?;
    if !text.ends_with('\n') || text.len() == 1 {
        return Err(err("R-OR64", "DIMACS is empty or lacks a final newline"));
    }
    let body = &text[..text.len() - 1];
    let mut lines = body.split('\n');
    let header = lines
        .next()
        .ok_or_else(|| err("R-OR65", "missing header"))?;
    let rest = header
        .strip_prefix("p cnf ")
        .ok_or_else(|| err("R-OR66", "header is not 'p cnf <vars> <clauses>'"))?;
    let fields: Vec<&str> = rest.split(' ').collect();
    if fields.len() != 2 {
        return Err(err("R-OR67", "header must have exactly two counts"));
    }
    let num_vars = ref_u32(fields[0], "variable count")?;
    let declared = ref_usize(fields[1], "clause count")?;
    if num_vars > REF_MAX_VARS {
        return Err(err("R-OR68", "too many variables"));
    }
    if declared > REF_MAX_CLAUSES {
        return Err(err("R-OR69", "too many clauses"));
    }

    let mut clauses: Vec<Vec<i32>> = Vec::with_capacity(declared);
    for line in lines {
        if line.is_empty() {
            return Err(err("R-OR70", "empty clause line"));
        }
        let mut lits = Vec::new();
        let mut terminated = false;
        for tok in line.split(' ') {
            if terminated {
                return Err(err("R-OR71", "tokens after the clause terminator"));
            }
            let lit = ref_i32(tok)?;
            if lit == 0 {
                terminated = true;
            } else {
                if lit.unsigned_abs() > num_vars {
                    return Err(err("R-OR72", "literal exceeds the variable count"));
                }
                lits.push(lit);
            }
        }
        if !terminated {
            return Err(err("R-OR73", "clause is not terminated by 0"));
        }
        if clauses.len() == declared {
            return Err(err("R-OR74", "more clauses than declared"));
        }
        clauses.push(lits);
    }
    if clauses.len() != declared {
        return Err(err("R-OR75", "fewer clauses than declared"));
    }
    Ok((num_vars, clauses))
}

fn ref_u32(tok: &str, what: &str) -> Result<u32, RefError> {
    if !canonical_digits(tok) {
        return Err(err("R-OR76", format!("bad {what} {tok:?}")));
    }
    tok.parse::<u32>()
        .map_err(|_| err("R-OR77", format!("bad {what} {tok:?}")))
}

fn ref_usize(tok: &str, what: &str) -> Result<usize, RefError> {
    if !canonical_digits(tok) {
        return Err(err("R-OR78", format!("bad {what} {tok:?}")));
    }
    tok.parse::<usize>()
        .map_err(|_| err("R-OR79", format!("bad {what} {tok:?}")))
}

fn ref_i32(tok: &str) -> Result<i32, RefError> {
    let digits = tok.strip_prefix('-').unwrap_or(tok);
    if !canonical_digits(digits) || (tok.starts_with('-') && digits == "0") {
        return Err(err("R-OR80", format!("bad literal {tok:?}")));
    }
    tok.parse::<i32>()
        .map_err(|_| err("R-OR81", format!("bad literal {tok:?}")))
}

// ---------------------------------------------------------------------------
// Naive LRAT verifier: no hints, full-clause unit propagation to fixpoint.
// ---------------------------------------------------------------------------

/// Verify an LRAT refutation without using hint order.
pub fn check_lrat_naive(
    num_vars: u32,
    cnf_clauses: &[Vec<i32>],
    bytes: &[u8],
) -> Result<(), RefError> {
    let text =
        std::str::from_utf8(bytes).map_err(|_| err("R-OR30", "LRAT payload is not UTF-8"))?;
    if !text.ends_with('\n') || text.len() == 1 {
        return Err(err("R-OR31", "LRAT is empty or lacks a final newline"));
    }
    let body = &text[..text.len() - 1];
    if body.is_empty() {
        return Err(err("R-OR32", "LRAT has no records"));
    }

    let mut live: BTreeMap<u64, Vec<i32>> = cnf_clauses
        .iter()
        .enumerate()
        .map(|(i, c)| ((i + 1) as u64, c.clone()))
        .collect();
    let mut deleted: BTreeSet<u64> = BTreeSet::new();
    let mut max_id = cnf_clauses.len() as u64;
    let mut last_added: Option<u64> = None;
    let mut saw_empty = false;

    for (idx, line) in body.split('\n').enumerate() {
        let line_no = idx + 1;
        if line.is_empty() {
            return Err(err("R-OR33", format!("line {line_no}: empty line")));
        }
        if saw_empty {
            return Err(err(
                "R-OR34",
                format!("line {line_no}: record after the empty clause"),
            ));
        }
        if let Some(rest) = line.strip_prefix("d ") {
            let ids = parse_ids(rest, line_no)?;
            for id in ids {
                if !live.contains_key(&id) {
                    return Err(err(
                        "R-OR35",
                        format!("line {line_no}: deletion of absent clause {id}"),
                    ));
                }
                live.remove(&id);
                deleted.insert(id);
            }
            continue;
        }

        let (id_s, rest) = split_field(line, line_no)?;
        let id = parse_u64(id_s, line_no)?;
        if let Some(after) = rest.strip_prefix("d ") {
            if last_added != Some(id) {
                return Err(err(
                    "R-OR36",
                    format!("line {line_no}: deletion annotation follows the wrong id"),
                ));
            }
            let ids = parse_ids(after, line_no)?;
            for del in ids {
                if !live.contains_key(&del) {
                    return Err(err(
                        "R-OR37",
                        format!("line {line_no}: deletion of absent clause {del}"),
                    ));
                }
                live.remove(&del);
                deleted.insert(del);
            }
            continue;
        }
        if id == 0 || id <= max_id {
            return Err(err(
                "R-OR38",
                format!("line {line_no}: clause id {id} is not fresh"),
            ));
        }

        let (clause, rest) = parse_literals(rest, num_vars, line_no)?;
        let (hints, rest) = parse_hints(rest, line_no)?;
        if !rest.is_empty() {
            return Err(err(
                "R-OR39",
                format!("line {line_no}: tokens after the hint terminator"),
            ));
        }
        // Hints are format metadata here; only their references are validated.
        for hint in hints {
            match live.get(&hint) {
                Some(_) => {}
                None => {
                    return Err(err(
                        "R-OR40",
                        format!("line {line_no}: hint {hint} names no live clause"),
                    ))
                }
            }
        }

        if !rup_fixpoint(&live, num_vars, &clause) {
            return Err(err(
                "R-OR41",
                format!("line {line_no}: clause is not RUP by full propagation"),
            ));
        }

        let empty = clause.is_empty();
        live.insert(id, clause);
        max_id = id;
        last_added = Some(id);
        if empty {
            saw_empty = true;
        }
    }

    if !saw_empty {
        return Err(err(
            "R-OR42",
            "the refutation never derives the empty clause",
        ));
    }
    Ok(())
}

fn rup_fixpoint(live: &BTreeMap<u64, Vec<i32>>, num_vars: u32, clause: &[i32]) -> bool {
    let mut assign = vec![0i8; num_vars as usize + 1];
    for &lit in clause {
        assign[lit.unsigned_abs() as usize] = if lit > 0 { -1 } else { 1 };
    }
    loop {
        let mut changed = false;
        for c in live.values() {
            let mut open = 0usize;
            let mut last = 0i32;
            let mut satisfied = false;
            for &lit in c {
                match assign[lit.unsigned_abs() as usize] {
                    0 => {
                        open += 1;
                        last = lit;
                    }
                    v if (v > 0) == (lit > 0) => {
                        satisfied = true;
                        break;
                    }
                    _ => {}
                }
            }
            if satisfied {
                continue;
            }
            if open == 0 {
                return true;
            }
            if open == 1 {
                assign[last.unsigned_abs() as usize] = if last > 0 { 1 } else { -1 };
                changed = true;
            }
        }
        if !changed {
            return false;
        }
    }
}

fn parse_ids(s: &str, line: usize) -> Result<Vec<u64>, RefError> {
    let mut out = Vec::new();
    let mut rest = s;
    loop {
        let (tok, next) = split_field(rest, line)?;
        let v = parse_u64(tok, line)?;
        if v == 0 {
            if !next.is_empty() {
                return Err(err(
                    "R-OR43",
                    format!("line {line}: tokens after the deletion terminator"),
                ));
            }
            return Ok(out);
        }
        out.push(v);
        if next.is_empty() {
            return Err(err("R-OR44", format!("line {line}: truncated deletion")));
        }
        rest = next;
    }
}

fn parse_literals(s: &str, num_vars: u32, line: usize) -> Result<(Vec<i32>, &str), RefError> {
    let mut out: Vec<i32> = Vec::new();
    let mut seen: BTreeSet<i32> = BTreeSet::new();
    let mut rest = s;
    loop {
        let (tok, next) = split_field(rest, line)?;
        let v = parse_i64(tok, line)?;
        if v == 0 {
            return Ok((out, next));
        }
        if v.unsigned_abs() > num_vars as u64 {
            return Err(err(
                "R-OR45",
                format!("line {line}: literal {v} is out of range"),
            ));
        }
        let lit = v as i32;
        if !seen.insert(lit) {
            return Err(err(
                "R-OR46",
                format!("line {line}: repeated literal {lit}"),
            ));
        }
        if seen.contains(&-lit) {
            return Err(err("R-OR47", format!("line {line}: tautological clause")));
        }
        out.push(lit);
        if next.is_empty() {
            return Err(err(
                "R-OR48",
                format!("line {line}: clause is not terminated"),
            ));
        }
        rest = next;
    }
}

fn parse_hints(s: &str, line: usize) -> Result<(Vec<u64>, &str), RefError> {
    let mut out = Vec::new();
    let mut rest = s;
    loop {
        let (tok, next) = split_field(rest, line)?;
        let v = parse_i64(tok, line)?;
        if v == 0 {
            return Ok((out, next));
        }
        if v < 0 {
            return Err(err(
                "R-OR49",
                format!("line {line}: negative hint requires RAT"),
            ));
        }
        out.push(v as u64);
        if next.is_empty() {
            return Err(err(
                "R-OR50",
                format!("line {line}: hint list is not terminated"),
            ));
        }
        rest = next;
    }
}

fn split_field(s: &str, line: usize) -> Result<(&str, &str), RefError> {
    if s.is_empty() {
        return Err(err("R-OR51", format!("line {line}: missing field")));
    }
    match s.find(' ') {
        Some(0) => Err(err("R-OR52", format!("line {line}: empty field"))),
        Some(i) => Ok((&s[..i], &s[i + 1..])),
        None => Ok((s, "")),
    }
}

fn parse_u64(tok: &str, line: usize) -> Result<u64, RefError> {
    if !canonical_digits(tok) {
        return Err(err("R-OR53", format!("line {line}: bad integer {tok:?}")));
    }
    tok.parse::<u64>()
        .map_err(|_| err("R-OR54", format!("line {line}: integer overflow {tok:?}")))
}

fn parse_i64(tok: &str, line: usize) -> Result<i64, RefError> {
    let digits = tok.strip_prefix('-').unwrap_or(tok);
    if !canonical_digits(digits) || (tok.starts_with('-') && digits == "0") {
        return Err(err("R-OR55", format!("line {line}: bad integer {tok:?}")));
    }
    tok.parse::<i64>()
        .map_err(|_| err("R-OR56", format!("line {line}: integer overflow {tok:?}")))
}

fn canonical_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) && (s.len() == 1 || !s.starts_with('0'))
}

// ---------------------------------------------------------------------------
// Independent evaluator and exhaustive falsification search.
// ---------------------------------------------------------------------------

const EXHAUSTIVE_BIT_BUDGET: u32 = 20;

/// Return a falsifying assignment if the proposition is false under one, or
/// `None` if it holds or the search budget is exceeded.
pub fn falsifying_assignment(prop: &Term) -> Result<Option<BTreeMap<String, u64>>, RefError> {
    if prop.sort() != crate::native::Sort::Bool {
        return Err(err("R-OR57", "proposition is not Boolean"));
    }
    let mut vars: BTreeMap<String, u32> = BTreeMap::new();
    collect_vars(prop, &mut vars)?;
    let total: u32 = vars.values().copied().sum();
    if total > EXHAUSTIVE_BIT_BUDGET {
        return Ok(None);
    }
    let mut names: Vec<(String, u32)> = vars.into_iter().collect();
    names.sort_by(|a, b| a.0.cmp(&b.0));

    let limit: u64 = 1u64 << total;
    for code in 0..limit {
        let mut env: BTreeMap<String, u64> = BTreeMap::new();
        let mut shift = 0u32;
        for (name, width) in &names {
            let value = if *width == 0 {
                0
            } else {
                (code >> shift) & ((1u64 << *width) - 1)
            };
            env.insert(name.clone(), value);
            shift += *width;
        }
        if eval_ref(prop, &env)? != Value::Bool(true) {
            return Ok(Some(env));
        }
    }
    Ok(None)
}

fn collect_vars(term: &Term, vars: &mut BTreeMap<String, u32>) -> Result<(), RefError> {
    match term {
        Term::Bool(_) => Ok(()),
        Term::Not(inner) => collect_vars(inner, vars),
        Term::Eq { left, right, .. }
        | Term::Add { left, right, .. }
        | Term::Sub { left, right, .. } => {
            collect_vars(left, vars)?;
            collect_vars(right, vars)
        }
        Term::Var { width, name } => {
            let entry = vars.entry(name.clone()).or_insert(0);
            if *width > *entry {
                *entry = *width;
            }
            Ok(())
        }
        Term::Const { width, .. } => {
            if *width == 0 || *width > 64 {
                return Err(err("R-OR58", "constant width out of range"));
            }
            Ok(())
        }
    }
}

/// Total evaluator, written separately from `Term::eval`.
pub fn eval_ref(term: &Term, env: &BTreeMap<String, u64>) -> Result<Value, RefError> {
    match term {
        Term::Bool(b) => Ok(Value::Bool(*b)),
        Term::Not(inner) => match eval_ref(inner, env)? {
            Value::Bool(b) => Ok(Value::Bool(!b)),
            Value::Bits(_) => Err(err("R-OR59", "not applied to a word")),
        },
        Term::Eq { width, left, right } => {
            let a = eval_word(left, *width, env)?;
            let b = eval_word(right, *width, env)?;
            Ok(Value::Bool(a == b))
        }
        Term::Var { width, name } => {
            let v = env
                .get(name)
                .copied()
                .ok_or_else(|| err("R-OR60", format!("unbound variable {name}")))?;
            Ok(Value::Bits(mask(v, *width)))
        }
        Term::Const { width, value } => Ok(Value::Bits(mask(*value, *width))),
        Term::Add { width, left, right } => {
            let a = eval_word(left, *width, env)?;
            let b = eval_word(right, *width, env)?;
            Ok(Value::Bits(mask(a.wrapping_add(b), *width)))
        }
        Term::Sub { width, left, right } => {
            let a = eval_word(left, *width, env)?;
            let b = eval_word(right, *width, env)?;
            Ok(Value::Bits(mask(a.wrapping_sub(b), *width)))
        }
    }
}

fn eval_word(term: &Term, width: u32, env: &BTreeMap<String, u64>) -> Result<u64, RefError> {
    if !(1..=64).contains(&width) {
        return Err(err("R-OR61", "width out of range"));
    }
    match eval_ref(term, env)? {
        Value::Bits(v) => Ok(mask(v, width)),
        Value::Bool(_) => Err(err("R-OR62", "expected a word, found a Boolean")),
    }
}

fn mask(value: u64, width: u32) -> u64 {
    if width >= 64 {
        value
    } else {
        value & ((1u64 << width) - 1)
    }
}
