//! Deterministic Tseitin bit-blasting.
//!
//! Variable numbering is stable: a fresh variable is allocated in the order the
//! canonical traversal first needs it (left-to-right, least-significant bit first).
//! Repeated occurrences of the same word variable share their bits; compound terms
//! are re-encoded at each occurrence. Consequently the CNF for a fixed term is
//! byte-for-byte reproducible.
//!
//! Encoding:
//!
//! * `add` is a ripple-carry adder (XOR sum, majority carry).
//! * `sub` is `a + (~b) + 1`, the two's-complement definition.
//! * `eq` is a chain of XNORs conjoined.
//! * the top-level assertion is the negation of the proposition, so UNSAT means
//!   the proposition is valid.
//!
//! Constants are folded during gate construction; they never become unconstrained
//! variables. A full adder is only allowed to produce clauses whose operands are
//! variables, because constant operands are simplified first.

use std::collections::BTreeMap;

use crate::native::{Error, Sort, Term};
use crate::{Canon, Digest};

/// Hard bounds so a malformed certificate cannot ask for unbounded allocation.
const MAX_VARS: u32 = i32::MAX as u32;
const MAX_CLAUSES: usize = 4_000_000;

/// A conjunctive normal form formula.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cnf {
    num_vars: u32,
    clauses: Vec<Vec<i32>>,
}

impl Cnf {
    pub fn num_vars(&self) -> u32 {
        self.num_vars
    }

    pub fn clauses(&self) -> &[Vec<i32>] {
        &self.clauses
    }

    /// Canonical DIMACS. One clause per line, literals separated by single spaces,
    /// each line terminated by ` 0`, including the empty clause as `0`.
    pub fn to_dimacs(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("p cnf {} {}\n", self.num_vars, self.clauses.len()));
        for clause in &self.clauses {
            for lit in clause {
                s.push_str(&lit.to_string());
                s.push(' ');
            }
            s.push_str("0\n");
        }
        s
    }

    /// Digest of the canonical DIMACS bytes under the CNF domain.
    pub fn digest(&self) -> Digest {
        let mut c = Canon::new("tekmerion/native/cnf/v1");
        c.bytes(self.to_dimacs().as_bytes());
        c.finish()
    }

    /// Strict canonical DIMACS parser.
    pub fn parse_dimacs(bytes: &[u8]) -> Result<Self, Error> {
        let text = std::str::from_utf8(bytes)
            .map_err(|_| Error::BadDimacs("input is not UTF-8".to_string()))?;
        if !text.ends_with('\n') {
            return Err(Error::BadDimacs("missing final newline".to_string()));
        }
        if text.len() == 1 {
            return Err(Error::BadDimacs("empty input".to_string()));
        }
        let body = &text[..text.len() - 1];
        let mut lines = body.split('\n');
        let header = lines
            .next()
            .ok_or_else(|| Error::BadDimacs("missing header".to_string()))?;
        let header_rest = header.strip_prefix("p cnf ").ok_or_else(|| {
            Error::BadDimacs("header must be 'p cnf <vars> <clauses>'".to_string())
        })?;
        let mut hs = header_rest.split(' ');
        let nv = hs
            .next()
            .ok_or_else(|| Error::BadDimacs("missing variable count".to_string()))?;
        let nc = hs
            .next()
            .ok_or_else(|| Error::BadDimacs("missing clause count".to_string()))?;
        if hs.next().is_some() {
            return Err(Error::BadDimacs("extra header fields".to_string()));
        }
        let num_vars = parse_canonical_u32(nv, "variable count")?;
        let declared = parse_canonical_usize(nc, "clause count")?;
        if num_vars > MAX_VARS {
            return Err(Error::BadDimacs("too many variables".to_string()));
        }
        if declared > MAX_CLAUSES {
            return Err(Error::BadDimacs("too many clauses".to_string()));
        }

        let mut clauses = Vec::with_capacity(declared);
        for line in lines {
            if line.is_empty() {
                return Err(Error::BadDimacs("empty line".to_string()));
            }
            let mut lits = Vec::new();
            let mut terminated = false;
            for tok in line.split(' ') {
                if terminated {
                    return Err(Error::BadDimacs(
                        "tokens after clause terminator".to_string(),
                    ));
                }
                let lit = parse_canonical_i32(tok, "literal")?;
                if lit == 0 {
                    terminated = true;
                } else {
                    if lit.unsigned_abs() > num_vars {
                        return Err(Error::BadDimacs(format!(
                            "literal {lit} exceeds variable count {num_vars}"
                        )));
                    }
                    lits.push(lit);
                }
            }
            if !terminated {
                return Err(Error::BadDimacs(
                    "clause is not terminated by 0".to_string(),
                ));
            }
            if clauses.len() == declared {
                return Err(Error::BadDimacs("more clauses than declared".to_string()));
            }
            clauses.push(lits);
        }
        if clauses.len() != declared {
            return Err(Error::BadDimacs(format!(
                "declared {declared} clauses, found {}",
                clauses.len()
            )));
        }
        let cnf = Cnf { num_vars, clauses };
        // Canonical-language refinement by construction (TNPS §1.7, P1): an
        // accepted input is exactly its own canonical rendering, so no
        // non-canonical whitespace, header or clause form can slip through.
        if cnf.to_dimacs() != text {
            return Err(Error::BadDimacs(
                "input is not the canonical DIMACS rendering".to_string(),
            ));
        }
        Ok(cnf)
    }
}

fn parse_canonical_u32(s: &str, what: &str) -> Result<u32, Error> {
    if s.is_empty() || (s.len() > 1 && s.starts_with('0')) || !s.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(Error::BadDimacs(format!("bad {what} {s:?}")));
    }
    s.parse::<u32>()
        .map_err(|_| Error::BadDimacs(format!("bad {what} {s:?}")))
}

fn parse_canonical_usize(s: &str, what: &str) -> Result<usize, Error> {
    if s.is_empty() || (s.len() > 1 && s.starts_with('0')) || !s.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(Error::BadDimacs(format!("bad {what} {s:?}")));
    }
    s.parse::<usize>()
        .map_err(|_| Error::BadDimacs(format!("bad {what} {s:?}")))
}

fn parse_canonical_i32(s: &str, what: &str) -> Result<i32, Error> {
    let digits = s.strip_prefix('-').unwrap_or(s);
    if digits.is_empty()
        || (digits.len() > 1 && digits.starts_with('0'))
        || !digits.bytes().all(|b| b.is_ascii_digit())
        || (s.starts_with('-') && digits == "0")
    {
        return Err(Error::BadDimacs(format!("bad {what} {s:?}")));
    }
    s.parse::<i32>()
        .map_err(|_| Error::BadDimacs(format!("bad {what} {s:?}")))
}

/// Bit-blast a Boolean proposition and assert its negation.
///
/// The returned CNF is unsatisfiable exactly when the proposition is valid under
/// the width-modular semantics of [`Term::eval`].
pub fn bitblast(prop: &Term) -> Result<Cnf, Error> {
    prop.validate()?;
    if prop.sort() != Sort::Bool {
        return Err(Error::SortMismatch {
            expected: "Bool",
            found: prop.sort().name(),
        });
    }
    let mut bb = Blaster::new();
    let top = bb.bool_lit(prop)?;
    match top.neg() {
        Lit::Const(false) => bb.clause(vec![]),
        Lit::Const(true) => {}
        Lit::Var(v) => bb.clause(vec![v]),
    }
    Ok(Cnf {
        num_vars: bb.next - 1,
        clauses: bb.clauses,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lit {
    Var(i32),
    Const(bool),
}

impl Lit {
    fn neg(self) -> Lit {
        match self {
            Lit::Var(v) => Lit::Var(-v),
            Lit::Const(b) => Lit::Const(!b),
        }
    }

    fn require_var(self) -> i32 {
        match self {
            Lit::Var(v) => v,
            Lit::Const(_) => unreachable!("simplification leaves only variables in clauses"),
        }
    }
}

struct Blaster {
    next: u32,
    clauses: Vec<Vec<i32>>,
    vars: BTreeMap<(String, u32), Vec<Lit>>,
}

impl Blaster {
    fn new() -> Self {
        Blaster {
            next: 1,
            clauses: Vec::new(),
            vars: BTreeMap::new(),
        }
    }

    fn fresh(&mut self) -> Lit {
        assert!(self.next < MAX_VARS, "variable budget exhausted");
        let v = self.next as i32;
        self.next += 1;
        Lit::Var(v)
    }

    fn clause(&mut self, clause: Vec<i32>) {
        assert!(self.clauses.len() < MAX_CLAUSES, "clause budget exhausted");
        self.clauses.push(clause);
    }

    fn and(&mut self, a: Lit, b: Lit) -> Lit {
        if a == b {
            return a;
        }
        if a == b.neg() {
            return Lit::Const(false);
        }
        match (a, b) {
            (Lit::Const(false), _) | (_, Lit::Const(false)) => Lit::Const(false),
            (Lit::Const(true), x) | (x, Lit::Const(true)) => x,
            _ => {
                let v = self.fresh();
                let (ai, bi, vi) = (a.require_var(), b.require_var(), v.require_var());
                self.clause(vec![-ai, -bi, vi]);
                self.clause(vec![ai, -vi]);
                self.clause(vec![bi, -vi]);
                v
            }
        }
    }

    fn or(&mut self, a: Lit, b: Lit) -> Lit {
        self.and(a.neg(), b.neg()).neg()
    }

    fn xor(&mut self, a: Lit, b: Lit) -> Lit {
        if a == b {
            return Lit::Const(false);
        }
        if a == b.neg() {
            return Lit::Const(true);
        }
        match (a, b) {
            (Lit::Const(false), x) | (x, Lit::Const(false)) => x,
            (Lit::Const(true), x) | (x, Lit::Const(true)) => x.neg(),
            _ => {
                let v = self.fresh();
                let (ai, bi, vi) = (a.require_var(), b.require_var(), v.require_var());
                self.clause(vec![-ai, -bi, -vi]);
                self.clause(vec![ai, bi, -vi]);
                self.clause(vec![ai, -bi, vi]);
                self.clause(vec![-ai, bi, vi]);
                v
            }
        }
    }

    fn xnor(&mut self, a: Lit, b: Lit) -> Lit {
        self.xor(a, b).neg()
    }

    fn majority(&mut self, a: Lit, b: Lit, c: Lit) -> Lit {
        let ab = self.and(a, b);
        let axb = self.xor(a, b);
        let c_ab = self.and(c, axb);
        self.or(ab, c_ab)
    }

    /// Ripple-carry addition, least-significant bit first. Returns `(sum, carry_out)`.
    fn add_bits(&mut self, a: &[Lit], b: &[Lit], carry_in: Lit) -> (Vec<Lit>, Lit) {
        debug_assert_eq!(a.len(), b.len());
        let mut sum = Vec::with_capacity(a.len());
        let mut carry = carry_in;
        for (&x, &y) in a.iter().zip(b.iter()) {
            let xy = self.xor(x, y);
            let s = self.xor(xy, carry);
            carry = self.majority(x, y, carry);
            sum.push(s);
        }
        (sum, carry)
    }

    fn eq_bits(&mut self, a: &[Lit], b: &[Lit]) -> Lit {
        debug_assert_eq!(a.len(), b.len());
        let mut acc = Lit::Const(true);
        for (&x, &y) in a.iter().zip(b.iter()) {
            let e = self.xnor(x, y);
            acc = self.and(acc, e);
        }
        acc
    }

    fn bits(&mut self, term: &Term) -> Result<Vec<Lit>, Error> {
        match term {
            Term::Var { width, name } => {
                let key = (name.clone(), *width);
                if let Some(bits) = self.vars.get(&key) {
                    return Ok(bits.clone());
                }
                let bits: Vec<Lit> = (0..*width).map(|_| self.fresh()).collect();
                self.vars.insert(key, bits.clone());
                Ok(bits)
            }
            Term::Const { width, value } => Ok((0..*width)
                .map(|i| Lit::Const((value >> i) & 1 == 1))
                .collect()),
            Term::Add { width, left, right } => {
                let a = self.bits(left)?;
                let b = self.bits(right)?;
                if a.len() != *width as usize || b.len() != *width as usize {
                    return Err(Error::WidthMismatch {
                        left: a.len() as u32,
                        right: b.len() as u32,
                    });
                }
                Ok(self.add_bits(&a, &b, Lit::Const(false)).0)
            }
            Term::Sub { width, left, right } => {
                let a = self.bits(left)?;
                let b: Vec<Lit> = self.bits(right)?.into_iter().map(|l| l.neg()).collect();
                if a.len() != *width as usize || b.len() != *width as usize {
                    return Err(Error::WidthMismatch {
                        left: a.len() as u32,
                        right: b.len() as u32,
                    });
                }
                Ok(self.add_bits(&a, &b, Lit::Const(true)).0)
            }
            other => Err(Error::SortMismatch {
                expected: "BitVec",
                found: other.sort().name(),
            }),
        }
    }

    fn bool_lit(&mut self, term: &Term) -> Result<Lit, Error> {
        match term {
            Term::Bool(b) => Ok(Lit::Const(*b)),
            Term::Not(inner) => Ok(self.bool_lit(inner)?.neg()),
            Term::Eq { width, left, right } => {
                let a = self.bits(left)?;
                let b = self.bits(right)?;
                if a.len() != *width as usize || b.len() != *width as usize {
                    return Err(Error::WidthMismatch {
                        left: a.len() as u32,
                        right: b.len() as u32,
                    });
                }
                Ok(self.eq_bits(&a, &b))
            }
            other => Err(Error::SortMismatch {
                expected: "Bool",
                found: other.sort().name(),
            }),
        }
    }
}
