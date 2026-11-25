//! HOL Light proof objects: kernel-step DAGs, export and independent replay.
//!
//! **Not on the native verdict path.**
//!
//! The recording kernel (`tools/hol-light/proof-object.diff`) appends one node
//! per primitive inference to a *raw* stream while a proof runs. This module is
//! the reader/finalizer/validator for that stream and for the compact objects
//! it produces.
//!
//! # Representation
//!
//! A node is `(rule, parent ids, payload)`. A node deliberately does **not**
//! store the theorem it produced: the result is recomputed by the checker from
//! the parent sequents and the payload. That is what makes "spliced nodes" and
//! "wrong conclusion" refusals meaningful rather than checksum checks — any
//! change to a reachable node either violates a rule or changes the root.
//!
//! Payload by rule:
//!
//! | rule | parents | payload |
//! |------|---------|---------|
//! | `REFL` | 0 | the term `t` (result `|- t = t`) |
//! | `TRANS` | 2 | none |
//! | `MK_COMB` | 2 | none |
//! | `ABS` | 1 | the bound variable |
//! | `BETA` | 0 | the redex `(\v. b) v` |
//! | `ASSUME` | 0 | the proposition |
//! | `EQ_MP` | 2 | none |
//! | `DEDUCT_ANTISYM_RULE` | 2 | none |
//! | `INST_TYPE` | 1 | the type substitution |
//! | `INST` | 1 | the term substitution |
//! | `new_axiom` | 0 | the axiom term |
//! | `new_basic_definition` | 0 | the defining equation |
//! | `new_basic_type_definition` | 1 | the 3 names; two nodes (abs/rep and rep/abs) |
//! | `IMPORT` | 0 | the whole sequent (a pre-recording theorem) |
//!
//! # What is hashed
//!
//! * `statement_digest` — SHA-256 of a canonical, alpha-normalised encoding of
//!   the root conclusion. Bound variables become de Bruijn indices, so
//!   alpha-renaming cannot change it.
//! * `derivation_digest` — SHA-256 of every byte of the object before the
//!   trailer. It detects truncation and corruption, not forgery.
//! * import and axiom statement digests — SHA-256 of the same canonical
//!   encoding, so a reader can list exactly what was assumed.
//!
//! # What is deliberately not recorded
//!
//! Tactics, strategy, timing, failed branches of proof search, and the
//! derivations of imported lemmas. The last is the real trust boundary: an
//! `IMPORT` node is an assumption about the pinned library. See the receipt
//! `research/receipts/ix80-proof-objects.md`.

use std::collections::{BTreeSet, HashMap};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::rc::Rc;

use sha2::{Digest as _, Sha256};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// One primitive rule plus the synthetic import and type-definition variants.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Rule {
    Refl = 0,
    Trans = 1,
    MkComb = 2,
    Abs = 3,
    Beta = 4,
    Assume = 5,
    EqMp = 6,
    DeductAntisym = 7,
    InstType = 8,
    Inst = 9,
    Axiom = 10,
    Definition = 11,
    TypeDefAbs = 12,
    TypeDefRep = 13,
    Import = 14,
}

impl Rule {
    fn from_u8(b: u8) -> Result<Self, Error> {
        Ok(match b {
            0 => Rule::Refl,
            1 => Rule::Trans,
            2 => Rule::MkComb,
            3 => Rule::Abs,
            4 => Rule::Beta,
            5 => Rule::Assume,
            6 => Rule::EqMp,
            7 => Rule::DeductAntisym,
            8 => Rule::InstType,
            9 => Rule::Inst,
            10 => Rule::Axiom,
            11 => Rule::Definition,
            12 => Rule::TypeDefAbs,
            13 => Rule::TypeDefRep,
            14 => Rule::Import,
            _ => return Err(Error::UnknownRule(b)),
        })
    }

    fn parents_needed(self) -> usize {
        match self {
            Rule::Refl
            | Rule::Beta
            | Rule::Assume
            | Rule::Axiom
            | Rule::Definition
            | Rule::Import => 0,
            Rule::Abs | Rule::InstType | Rule::Inst | Rule::TypeDefAbs | Rule::TypeDefRep => 1,
            Rule::Trans | Rule::MkComb | Rule::EqMp | Rule::DeductAntisym => 2,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Rule::Refl => "REFL",
            Rule::Trans => "TRANS",
            Rule::MkComb => "MK_COMB",
            Rule::Abs => "ABS",
            Rule::Beta => "BETA",
            Rule::Assume => "ASSUME",
            Rule::EqMp => "EQ_MP",
            Rule::DeductAntisym => "DEDUCT_ANTISYM_RULE",
            Rule::InstType => "INST_TYPE",
            Rule::Inst => "INST",
            Rule::Axiom => "new_axiom",
            Rule::Definition => "new_basic_definition",
            Rule::TypeDefAbs => "new_basic_type_definition/abs",
            Rule::TypeDefRep => "new_basic_type_definition/rep",
            Rule::Import => "IMPORT",
        }
    }
}

#[derive(Clone, Debug)]
pub enum Error {
    /// Raw stream does not start with `HLRAW1\n`.
    RawMagic,
    /// Raw stream ended before its footer.
    RawTruncated,
    /// A raw record declares a length it does not contain.
    RawShortRecord,
    /// Object magic/version is wrong.
    BadMagic,
    BadVersion(u8),
    /// The trailer digest does not match the body (truncation or corruption).
    DigestMismatch,
    /// Declared counts disagree with the bytes present.
    CountMismatch {
        what: &'static str,
    },
    /// A parent id is outside the node table or not earlier than its child.
    ParentOutOfRange {
        node: u64,
        parent: u64,
    },
    /// The root is not the last node (parents must precede children).
    RootNotLast,
    UnknownRule(u8),
    /// A rule application failed: the parents do not licence the payload.
    /// This is the splice refusal.
    RuleFailure {
        node: u64,
        rule: &'static str,
        why: String,
    },
    /// The replayed conclusion differs from the recorded statement.
    ConclusionMismatch {
        expected: String,
        got: String,
    },
    /// The root theorem has hypotheses; it is not a closed theorem.
    RootNotClosed(usize),
    /// Header's statement term does not hash to the header's statement digest.
    StatementSelfInconsistent,
    /// A term/type table reference is out of range.
    TableReference {
        what: &'static str,
        id: u64,
    },
    /// The object's declared axiom/import sets disagree with its nodes.
    AssumptionSetMismatch {
        what: &'static str,
    },
    /// I/O failure, with a rendered message.
    Io(String),
    /// A malformed field (UTF-8, name, etc.).
    Malformed(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::RawMagic => write!(f, "raw stream: bad magic"),
            Error::RawTruncated => write!(f, "raw stream: truncated before footer"),
            Error::RawShortRecord => write!(f, "raw stream: record shorter than its length"),
            Error::BadMagic => write!(f, "object: bad magic"),
            Error::BadVersion(v) => write!(f, "object: unsupported version {v}"),
            Error::DigestMismatch => write!(
                f,
                "object: trailer digest mismatch (truncated or corrupted)"
            ),
            Error::CountMismatch { what } => {
                write!(f, "object: declared {what} count does not match")
            }
            Error::ParentOutOfRange { node, parent } => {
                write!(
                    f,
                    "object: node {node} names parent {parent} out of range or not earlier"
                )
            }
            Error::RootNotLast => write!(f, "object: root is not the last node"),
            Error::UnknownRule(r) => write!(f, "object: unknown rule tag {r}"),
            Error::RuleFailure { node, rule, why } => {
                write!(
                    f,
                    "refused: node {node} ({rule}) does not follow from its parents: {why}"
                )
            }
            Error::ConclusionMismatch { expected, got } => write!(
                f,
                "refused: replayed conclusion {got} differs from recorded statement {expected}"
            ),
            Error::RootNotClosed(n) => write!(f, "refused: root theorem has {n} hypotheses"),
            Error::StatementSelfInconsistent => {
                write!(
                    f,
                    "object: stored statement does not match statement digest"
                )
            }
            Error::TableReference { what, id } => {
                write!(f, "object: {what} reference {id} out of range")
            }
            Error::AssumptionSetMismatch { what } => {
                write!(f, "object: declared {what} set disagrees with the nodes")
            }
            Error::Io(m) => write!(f, "io: {m}"),
            Error::Malformed(m) => write!(f, "malformed: {m}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e.to_string())
    }
}

/// What the finalizer reports about one exported object.
#[derive(Clone, Debug)]
pub struct Summary {
    pub theorem: String,
    pub statement_digest: String,
    pub derivation_digest: String,
    pub node_count: u64,
    pub bytes: u64,
    pub root_id: u64,
    pub inference_total: u64,
    pub inference_begin: u64,
    pub imports: u64,
    pub axioms: Vec<String>,
    pub import_digests: Vec<String>,
}

impl Summary {
    /// Primitive inferences spent while recording was active.
    pub fn inference_delta(&self) -> u64 {
        self.inference_total.saturating_sub(self.inference_begin)
    }
}

/// What the validator reports when it accepts an object.
#[derive(Clone, Debug)]
pub struct Validation {
    pub theorem: String,
    pub statement_digest: String,
    pub derivation_digest: String,
    pub node_count: u64,
    pub raw_nodes: u64,
    pub inference_total: u64,
    pub inference_begin: u64,
    pub imports: u64,
    pub axioms: Vec<String>,
}

// ---------------------------------------------------------------------------
// Byte helpers
// ---------------------------------------------------------------------------

fn hex32(d: &[u8; 32]) -> String {
    let mut s = String::with_capacity(64);
    for b in d {
        s.push(char::from_digit((b >> 4) as u32, 16).unwrap());
        s.push(char::from_digit((b & 0xf) as u32, 16).unwrap());
    }
    s
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(bytes);
    let out = h.finalize();
    let mut a = [0u8; 32];
    a.copy_from_slice(&out);
    a
}

fn put_u8(v: &mut Vec<u8>, b: u8) {
    v.push(b);
}

fn put_u32(v: &mut Vec<u8>, n: u32) {
    v.extend_from_slice(&n.to_be_bytes());
}

fn put_u64(v: &mut Vec<u8>, n: u64) {
    v.extend_from_slice(&n.to_be_bytes());
}

fn put_str(v: &mut Vec<u8>, s: &str) {
    put_u32(v, s.len() as u32);
    v.extend_from_slice(s.as_bytes());
}

struct Cursor<'a> {
    b: &'a [u8],
    p: usize,
}

impl<'a> Cursor<'a> {
    fn new(b: &'a [u8]) -> Self {
        Cursor { b, p: 0 }
    }
    fn remaining(&self) -> usize {
        self.b.len().saturating_sub(self.p)
    }
    fn u8(&mut self) -> Result<u8, Error> {
        if self.p >= self.b.len() {
            return Err(Error::CountMismatch { what: "byte" });
        }
        let v = self.b[self.p];
        self.p += 1;
        Ok(v)
    }
    fn u32(&mut self) -> Result<u32, Error> {
        if self.remaining() < 4 {
            return Err(Error::CountMismatch { what: "u32" });
        }
        let mut a = [0u8; 4];
        a.copy_from_slice(&self.b[self.p..self.p + 4]);
        self.p += 4;
        Ok(u32::from_be_bytes(a))
    }
    fn u64(&mut self) -> Result<u64, Error> {
        if self.remaining() < 8 {
            return Err(Error::CountMismatch { what: "u64" });
        }
        let mut a = [0u8; 8];
        a.copy_from_slice(&self.b[self.p..self.p + 8]);
        self.p += 8;
        Ok(u64::from_be_bytes(a))
    }
    fn str(&mut self) -> Result<String, Error> {
        let n = self.u32()? as usize;
        if self.remaining() < n {
            return Err(Error::CountMismatch { what: "string" });
        }
        let s = std::str::from_utf8(&self.b[self.p..self.p + n])
            .map_err(|_| Error::Malformed("invalid UTF-8".into()))?
            .to_string();
        self.p += n;
        Ok(s)
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        if self.remaining() < n {
            return Err(Error::CountMismatch { what: "payload" });
        }
        let s = &self.b[self.p..self.p + n];
        self.p += n;
        Ok(s)
    }
}

// ---------------------------------------------------------------------------
// Terms and types (name-carrying, shared via Rc)
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Type {
    Var(String),
    App(String, Vec<Type>),
}

pub type Ty = Rc<Type>;
pub type TermRef = Rc<Term>;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Term {
    Var(String, Ty),
    Const(String, Ty),
    Comb(TermRef, TermRef),
    Abs(TermRef, TermRef),
}

fn bool_ty() -> Ty {
    Rc::new(Type::App("bool".into(), vec![]))
}

fn fun_ty(a: &Type, b: &Type) -> Ty {
    Rc::new(Type::App("fun".into(), vec![a.clone(), b.clone()]))
}

fn type_of(t: &TermRef) -> Result<Ty, Error> {
    Ok(match &**t {
        Term::Var(_, ty) | Term::Const(_, ty) => ty.clone(),
        Term::Comb(s, _) => match type_of(s)?.as_ref() {
            Type::App(f, args) if f == "fun" && args.len() == 2 => Rc::new(args[1].clone()),
            _ => return Err(Error::Malformed("Comb applied to non-function".into())),
        },
        Term::Abs(v, b) => fun_ty(type_of(v)?.as_ref(), type_of(b)?.as_ref()),
    })
}

fn mk_comb_checked(f: TermRef, a: TermRef) -> Result<TermRef, Error> {
    match type_of(&f)?.as_ref() {
        Type::App(n, args)
            if n == "fun" && args.len() == 2 && args[0] == *type_of(&a)?.as_ref() =>
        {
            Ok(Rc::new(Term::Comb(f, a)))
        }
        _ => Err(Error::Malformed("mk_comb: types do not agree".into())),
    }
}

fn mk_eq(l: TermRef, r: TermRef) -> Result<TermRef, Error> {
    let ty = type_of(&l)?;
    let cod = fun_ty(ty.as_ref(), bool_ty().as_ref());
    let eq_ty = fun_ty(ty.as_ref(), cod.as_ref());
    let eq = Rc::new(Term::Const("=".into(), eq_ty));
    let inner = Rc::new(Term::Comb(eq, l));
    Ok(Rc::new(Term::Comb(inner, r)))
}

fn dest_eq(t: &TermRef) -> Option<(TermRef, TermRef)> {
    match &**t {
        Term::Comb(f, r) => match &**f {
            Term::Comb(g, l) => match &**g {
                Term::Const(n, _) if n == "=" => Some((l.clone(), r.clone())),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}

fn dest_comb(t: &TermRef) -> Option<(TermRef, TermRef)> {
    match &**t {
        Term::Comb(f, a) => Some((f.clone(), a.clone())),
        _ => None,
    }
}

fn dest_var(t: &TermRef) -> Option<(String, Ty)> {
    match &**t {
        Term::Var(n, ty) => Some((n.clone(), ty.clone())),
        _ => None,
    }
}

fn teq(a: &TermRef, b: &TermRef) -> bool {
    Rc::ptr_eq(a, b) || a == b
}

/// Alpha-equality, matching HOL Light's `alphaorder ... = 0`.
fn alpha_eq(a: &TermRef, b: &TermRef) -> bool {
    fn ordav(env: &[(TermRef, TermRef)], x1: &TermRef, x2: &TermRef) -> bool {
        for (t1, t2) in env {
            if teq(x1, t1) {
                return teq(x2, t2);
            } else if teq(x2, t2) {
                return false;
            }
        }
        teq(x1, x2)
    }
    fn orda(env: &mut Vec<(TermRef, TermRef)>, a: &TermRef, b: &TermRef) -> bool {
        match (&**a, &**b) {
            (Term::Var(_, _), Term::Var(_, _)) => ordav(env, a, b),
            (Term::Const(n1, t1), Term::Const(n2, t2)) => n1 == n2 && t1 == t2,
            (Term::Comb(s1, t1), Term::Comb(s2, t2)) => orda(env, s1, s2) && orda(env, t1, t2),
            (Term::Abs(x1, t1), Term::Abs(x2, t2)) => match (&**x1, &**x2) {
                (Term::Var(_, ty1), Term::Var(_, ty2)) if ty1 == ty2 => {
                    env.push((x1.clone(), x2.clone()));
                    let r = orda(env, t1, t2);
                    env.pop();
                    r
                }
                _ => false,
            },
            _ => false,
        }
    }
    orda(&mut Vec::new(), a, b)
}

fn vfree_in(v: &TermRef, t: &TermRef) -> bool {
    match &**t {
        Term::Abs(bv, bod) => !teq(bv, v) && vfree_in(v, bod),
        Term::Comb(s, x) => vfree_in(v, s) || vfree_in(v, x),
        _ => teq(t, v),
    }
}

fn frees(t: &TermRef) -> Vec<TermRef> {
    fn go(t: &TermRef, acc: &mut Vec<TermRef>) {
        match &**t {
            Term::Var(_, _) => {
                if !acc.iter().any(|u| teq(u, t)) {
                    acc.push(t.clone());
                }
            }
            Term::Const(_, _) => {}
            Term::Comb(s, x) => {
                go(s, acc);
                go(x, acc);
            }
            Term::Abs(bv, bod) => {
                let mut inner = Vec::new();
                go(bod, &mut inner);
                for v in inner {
                    if !teq(&v, bv) && !acc.iter().any(|u| teq(u, &v)) {
                        acc.push(v);
                    }
                }
            }
        }
    }
    let mut acc = Vec::new();
    go(t, &mut acc);
    acc
}

fn freesin(acc: &[TermRef], t: &TermRef) -> bool {
    match &**t {
        Term::Var(_, _) => acc.iter().any(|u| teq(u, t)),
        Term::Const(_, _) => true,
        Term::Abs(bv, bod) => {
            let mut a = acc.to_vec();
            a.push(bv.clone());
            freesin(&a, bod)
        }
        Term::Comb(s, x) => freesin(acc, s) && freesin(acc, x),
    }
}

fn variant(avoid: &[TermRef], v: &TermRef) -> Result<TermRef, Error> {
    let (mut name, ty) =
        dest_var(v).ok_or_else(|| Error::Malformed("variant: not a variable".into()))?;
    let mut cur = v.clone();
    while avoid.iter().any(|a| vfree_in(&cur, a)) {
        name.push('\'');
        cur = Rc::new(Term::Var(name.clone(), ty.clone()));
    }
    Ok(cur)
}

/// `vsubst` over `(replacement, variable)` pairs, head-first (HOL `rev_assocd`).
fn vsubst(ilist: &[(TermRef, TermRef)], tm: &TermRef) -> Result<TermRef, Error> {
    match &**tm {
        Term::Var(_, _) => {
            for (repl, var) in ilist {
                if teq(var, tm) {
                    return Ok(repl.clone());
                }
            }
            Ok(tm.clone())
        }
        Term::Const(_, _) => Ok(tm.clone()),
        Term::Comb(s, t) => {
            let s2 = vsubst(ilist, s)?;
            let t2 = vsubst(ilist, t)?;
            if teq(&s2, s) && teq(&t2, t) {
                Ok(tm.clone())
            } else {
                Ok(Rc::new(Term::Comb(s2, t2)))
            }
        }
        Term::Abs(v, s) => {
            let ilist2: Vec<(TermRef, TermRef)> =
                ilist.iter().filter(|(_, x)| !teq(x, v)).cloned().collect();
            if ilist2.is_empty() {
                return Ok(tm.clone());
            }
            let s2 = vsubst(&ilist2, s)?;
            if teq(&s2, s) {
                return Ok(tm.clone());
            }
            let captures = ilist2.iter().any(|(t, x)| vfree_in(v, t) && vfree_in(x, s));
            if captures {
                let v2 = variant(std::slice::from_ref(&s2), v)?;
                let mut ilist3 = Vec::with_capacity(ilist2.len() + 1);
                ilist3.push((v2.clone(), v.clone()));
                ilist3.extend(ilist2);
                let body = vsubst(&ilist3, s)?;
                Ok(Rc::new(Term::Abs(v2, body)))
            } else {
                Ok(Rc::new(Term::Abs(v.clone(), s2)))
            }
        }
    }
}

fn type_subst(tyin: &[(Ty, Ty)], ty: &Type) -> Type {
    match ty {
        Type::Var(v) => {
            let key = Type::Var(v.clone());
            for (k, repl) in tyin {
                if **k == key {
                    return (**repl).clone();
                }
            }
            Type::Var(v.clone())
        }
        Type::App(f, args) => Type::App(
            f.clone(),
            args.iter().map(|a| type_subst(tyin, a)).collect(),
        ),
    }
}

/// A type-instantiation clash, carrying the offending term, or a fatal error.
enum InstError {
    Clash(TermRef),
    Fatal(Error),
}

impl From<Error> for InstError {
    fn from(e: Error) -> Self {
        InstError::Fatal(e)
    }
}

/// Type instantiation, ported from HOL Light's `inst` (including capture
/// renaming).
fn inst(env: &[(TermRef, TermRef)], tyin: &[(Ty, Ty)], tm: &TermRef) -> Result<TermRef, InstError> {
    fn rev_assocd(key: &TermRef, env: &[(TermRef, TermRef)], default: &TermRef) -> TermRef {
        for (a, b) in env {
            if teq(a, key) {
                return b.clone();
            }
        }
        default.clone()
    }
    match &**tm {
        Term::Var(n, ty) => {
            let ty2 = Rc::new(type_subst(tyin, ty));
            let tm2: TermRef = if *ty2 == **ty {
                tm.clone()
            } else {
                Rc::new(Term::Var(n.clone(), ty2))
            };
            let d = rev_assocd(&tm2, env, tm);
            if teq(&d, tm) {
                Ok(tm2)
            } else {
                Err(InstError::Clash(tm2))
            }
        }
        Term::Const(c, ty) => {
            let ty2 = Rc::new(type_subst(tyin, ty));
            if *ty2 == **ty {
                Ok(tm.clone())
            } else {
                Ok(Rc::new(Term::Const(c.clone(), ty2)))
            }
        }
        Term::Comb(f, x) => {
            let f2 = inst(env, tyin, f)?;
            let x2 = inst(env, tyin, x)?;
            if teq(&f2, f) && teq(&x2, x) {
                Ok(tm.clone())
            } else {
                Ok(Rc::new(Term::Comb(f2, x2)))
            }
        }
        Term::Abs(y, t) => {
            let y2 = inst(&[], tyin, y)?;
            let mut env2 = Vec::with_capacity(env.len() + 1);
            env2.push((y.clone(), y2.clone()));
            env2.extend(env.iter().cloned());
            match inst(&env2, tyin, t) {
                Ok(t2) => {
                    if teq(&y2, y) && teq(&t2, t) {
                        Ok(tm.clone())
                    } else {
                        Ok(Rc::new(Term::Abs(y2, t2)))
                    }
                }
                Err(InstError::Fatal(e)) => Err(InstError::Fatal(e)),
                Err(InstError::Clash(w2)) => {
                    if !teq(&w2, &y2) {
                        Err(InstError::Clash(w2))
                    } else {
                        let mut ifrees = Vec::new();
                        for f in frees(t) {
                            ifrees.push(inst(&[], tyin, &f)?);
                        }
                        let y3 = variant(&ifrees, &y2)?;
                        let (n3, _) = dest_var(&y3).ok_or_else(|| {
                            InstError::Fatal(Error::Malformed("inst: binder not a variable".into()))
                        })?;
                        let (_, ty_y) = dest_var(y).ok_or_else(|| {
                            InstError::Fatal(Error::Malformed("inst: binder not a variable".into()))
                        })?;
                        let z = Rc::new(Term::Var(n3, ty_y));
                        let body =
                            Rc::new(Term::Abs(z.clone(), vsubst(&[(z.clone(), y.clone())], t)?));
                        inst(env, tyin, &body)
                    }
                }
            }
        }
    }
}

/// `inst`, with a clash rendered as a rule failure string.
fn inst_checked(
    env: &[(TermRef, TermRef)],
    tyin: &[(Ty, Ty)],
    tm: &TermRef,
) -> Result<TermRef, String> {
    match inst(env, tyin, tm) {
        Ok(t) => Ok(t),
        Err(InstError::Clash(_)) => Err("INST_TYPE: type instantiation clash".into()),
        Err(InstError::Fatal(e)) => Err(e.to_string()),
    }
}

fn term_union(a: &[TermRef], b: &[TermRef]) -> Vec<TermRef> {
    let mut out: Vec<TermRef> = Vec::new();
    for t in a.iter().chain(b.iter()) {
        if !out.iter().any(|u| alpha_eq(t, u)) {
            out.push(t.clone());
        }
    }
    out
}

fn term_remove(t: &TermRef, l: &[TermRef]) -> Vec<TermRef> {
    l.iter().filter(|u| !alpha_eq(t, u)).cloned().collect()
}

fn term_image(f: &[(TermRef, TermRef)], l: &[TermRef]) -> Result<Vec<TermRef>, Error> {
    let mut out: Vec<TermRef> = Vec::new();
    for h in l {
        let h2 = vsubst(f, h)?;
        if !out.iter().any(|u| alpha_eq(&h2, u)) {
            out.push(h2);
        }
    }
    Ok(out)
}

fn type_vars_in_term(t: &TermRef) -> BTreeSet<String> {
    fn tyvars(ty: &Type, out: &mut BTreeSet<String>) {
        match ty {
            Type::Var(v) => {
                out.insert(v.clone());
            }
            Type::App(_, args) => {
                for a in args {
                    tyvars(a, out);
                }
            }
        }
    }
    fn go(t: &TermRef, out: &mut BTreeSet<String>) {
        match &**t {
            Term::Var(_, ty) | Term::Const(_, ty) => tyvars(ty, out),
            Term::Comb(s, x) => {
                go(s, out);
                go(x, out);
            }
            Term::Abs(v, b) => {
                go(v, out);
                go(b, out);
            }
        }
    }
    let mut out = BTreeSet::new();
    go(t, &mut out);
    out
}

#[derive(Clone, Debug)]
pub struct Thm {
    pub hyps: Vec<TermRef>,
    pub concl: TermRef,
}

// ---------------------------------------------------------------------------
// Canonical (alpha-normalised) encoding
// ---------------------------------------------------------------------------

fn canon_type(out: &mut Vec<u8>, ty: &Type) {
    match ty {
        Type::Var(v) => {
            put_u8(out, 0);
            put_str(out, v);
        }
        Type::App(f, args) => {
            put_u8(out, 1);
            put_str(out, f);
            put_u32(out, args.len() as u32);
            for a in args {
                canon_type(out, a);
            }
        }
    }
}

fn canon_term(out: &mut Vec<u8>, t: &TermRef, binders: &mut Vec<(String, Ty)>) {
    match &**t {
        Term::Var(n, ty) => {
            // A variable is bound iff the innermost binder with this name has
            // the same type; emit a de Bruijn index then.
            if let Some(idx) = binders
                .iter()
                .rev()
                .position(|(bn, bt)| bn == n && bt.as_ref() == ty.as_ref())
            {
                put_u8(out, 4);
                put_u32(out, idx as u32);
            } else {
                put_u8(out, 0);
                put_str(out, n);
                canon_type(out, ty);
            }
        }
        Term::Const(n, ty) => {
            put_u8(out, 1);
            put_str(out, n);
            canon_type(out, ty);
        }
        Term::Comb(a, b) => {
            put_u8(out, 2);
            canon_term(out, a, binders);
            canon_term(out, b, binders);
        }
        Term::Abs(v, bod) => match &**v {
            Term::Var(n, ty) => {
                put_u8(out, 5);
                canon_type(out, ty);
                binders.push((n.clone(), ty.clone()));
                canon_term(out, bod, binders);
                binders.pop();
            }
            _ => {
                put_u8(out, 6);
                canon_term(out, v, binders);
                canon_term(out, bod, binders);
            }
        },
    }
}

fn statement_digest(concl: &TermRef) -> [u8; 32] {
    let mut out = Vec::new();
    canon_term(&mut out, concl, &mut Vec::new());
    sha256(&out)
}

// ---------------------------------------------------------------------------
// Raw stream scan
// ---------------------------------------------------------------------------

const RAW_MAGIC: &[u8] = b"HLRAW1\n";
const OBJ_MAGIC: &[u8] = b"HLPROOF1\n";
const OBJ_END: &[u8] = b"HLEND1\n";
const OBJ_VERSION: u8 = 1;
const ABSENT: u32 = 0xFFFF_FFFF;

#[derive(Clone)]
struct NodeMeta {
    rule: Rule,
    p0: u32,
    p1: u32,
    /// Absolute file offset of the payload.
    payload_off: u64,
    payload_len: u32,
}

struct RawScan {
    nodes: Vec<NodeMeta>,
    footer: RawFooter,
}

struct RawFooter {
    name: String,
    root: u32,
    hyps: Vec<TermRef>,
    concl: TermRef,
    inference_total: u64,
    inference_begin: u64,
    axioms: Vec<(u32, String, String)>,
}

struct StreamReader {
    f: std::io::BufReader<File>,
    pos: u64,
}

impl StreamReader {
    fn open(path: &Path) -> Result<Self, Error> {
        Ok(StreamReader {
            f: std::io::BufReader::new(File::open(path)?),
            pos: 0,
        })
    }
    fn exact(&mut self, n: usize) -> Result<Vec<u8>, Error> {
        let mut b = vec![0u8; n];
        self.f.read_exact(&mut b).map_err(|_| Error::RawTruncated)?;
        self.pos += n as u64;
        Ok(b)
    }
    fn u8(&mut self) -> Result<u8, Error> {
        Ok(self.exact(1)?[0])
    }
    fn u32(&mut self) -> Result<u32, Error> {
        let b = self.exact(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn skip(&mut self, n: usize) -> Result<(), Error> {
        let b = self.exact(n)?;
        let _ = b;
        Ok(())
    }
}

/// Recursive decode of a raw term into an `Rc<Term>`. Used for the footer only;
/// node payloads are interned directly.
fn decode_raw_term(c: &mut Cursor<'_>) -> Result<TermRef, Error> {
    match c.u8()? {
        0 => {
            let n = c.str()?;
            let ty = decode_raw_type(c)?;
            Ok(Rc::new(Term::Var(n, Rc::new(ty))))
        }
        1 => {
            let n = c.str()?;
            let ty = decode_raw_type(c)?;
            Ok(Rc::new(Term::Const(n, Rc::new(ty))))
        }
        2 => {
            let f = decode_raw_term(c)?;
            let x = decode_raw_term(c)?;
            Ok(Rc::new(Term::Comb(f, x)))
        }
        3 => {
            let v = decode_raw_term(c)?;
            let b = decode_raw_term(c)?;
            Ok(Rc::new(Term::Abs(v, b)))
        }
        t => Err(Error::Malformed(format!("term tag {t}"))),
    }
}

fn decode_raw_type(c: &mut Cursor<'_>) -> Result<Type, Error> {
    match c.u8()? {
        0 => Ok(Type::Var(c.str()?)),
        1 => {
            let f = c.str()?;
            let n = c.u32()? as usize;
            let mut args = Vec::with_capacity(n);
            for _ in 0..n {
                args.push(decode_raw_type(c)?);
            }
            Ok(Type::App(f, args))
        }
        t => Err(Error::Malformed(format!("type tag {t}"))),
    }
}

fn scan_raw(path: &Path) -> Result<RawScan, Error> {
    let mut r = StreamReader::open(path)?;
    let magic = r.exact(RAW_MAGIC.len())?;
    if magic != RAW_MAGIC {
        return Err(Error::RawMagic);
    }
    let mut nodes = Vec::new();
    loop {
        let tag = r.u8()?;
        let len = r.u32()? as usize;
        let body_off = r.pos;
        match tag {
            1 => {
                // rule(1) p0(4) p1(4) plen(4) payload
                if len < 13 {
                    return Err(Error::RawShortRecord);
                }
                let rule = Rule::from_u8(r.u8()?)?;
                let p0 = r.u32()?;
                let p1 = r.u32()?;
                let plen = r.u32()? as usize;
                if 13 + plen != len {
                    return Err(Error::RawShortRecord);
                }
                let payload_off = body_off + 13;
                r.skip(plen)?;
                nodes.push(NodeMeta {
                    rule,
                    p0,
                    p1,
                    payload_off,
                    payload_len: plen as u32,
                });
            }
            2 => {
                let body = r.exact(len)?;
                let mut c = Cursor::new(&body);
                let name = c.str()?;
                let root = c.u32()?;
                let nh = c.u32()? as usize;
                let mut hyps = Vec::with_capacity(nh);
                for _ in 0..nh {
                    hyps.push(decode_raw_term(&mut c)?);
                }
                let concl = decode_raw_term(&mut c)?;
                let inference_total = c.u32()? as u64;
                let inference_begin = c.u32()? as u64;
                let node_count_declared = c.u32()? as u64;
                let na = c.u32()? as usize;
                let mut axioms = Vec::with_capacity(na);
                for _ in 0..na {
                    let id = c.u32()?;
                    let dg = c.str()?;
                    let txt = c.str()?;
                    axioms.push((id, dg, txt));
                }
                if c.remaining() != 0 {
                    return Err(Error::Malformed("footer trailing bytes".into()));
                }
                if node_count_declared != nodes.len() as u64 {
                    return Err(Error::CountMismatch { what: "raw node" });
                }
                let footer = RawFooter {
                    name,
                    root,
                    hyps,
                    concl,
                    inference_total,
                    inference_begin,
                    axioms,
                };
                return Ok(RawScan { nodes, footer });
            }
            t => return Err(Error::Malformed(format!("raw record tag {t}"))),
        }
    }
}

fn read_node_prefix(f: &File, off: u64) -> Result<(Rule, u32, u32), Error> {
    let mut b = [0u8; 13];
    use std::os::unix::fs::FileExt;
    f.read_exact_at(&mut b, off)?;
    let rule = Rule::from_u8(b[0])?;
    let p0 = u32::from_be_bytes([b[1], b[2], b[3], b[4]]);
    let p1 = u32::from_be_bytes([b[5], b[6], b[7], b[8]]);
    Ok((rule, p0, p1))
}

fn read_payload(f: &File, m: &NodeMeta) -> Result<Vec<u8>, Error> {
    use std::os::unix::fs::FileExt;
    let mut b = vec![0u8; m.payload_len as usize];
    f.read_exact_at(&mut b, m.payload_off)?;
    Ok(b)
}

// ---------------------------------------------------------------------------
// Interning
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
enum TypeKey {
    Var(String),
    App(String, Vec<u32>),
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
enum TermKey {
    Var(String, u32),
    Const(String, u32),
    Comb(u32, u32),
    Abs(u32, u32),
}

#[derive(Default)]
struct Interner {
    types: Vec<TypeKey>,
    type_map: HashMap<TypeKey, u32>,
    terms: Vec<TermKey>,
    term_map: HashMap<TermKey, u32>,
}

impl Interner {
    fn intern_type(&mut self, k: TypeKey) -> u32 {
        if let Some(&id) = self.type_map.get(&k) {
            return id;
        }
        let id = self.types.len() as u32;
        self.types.push(k.clone());
        self.type_map.insert(k, id);
        id
    }
    fn intern_term(&mut self, k: TermKey) -> u32 {
        if let Some(&id) = self.term_map.get(&k) {
            return id;
        }
        let id = self.terms.len() as u32;
        self.terms.push(k.clone());
        self.term_map.insert(k, id);
        id
    }

    fn decode_type(&mut self, c: &mut Cursor<'_>) -> Result<u32, Error> {
        match c.u8()? {
            0 => Ok(self.intern_type(TypeKey::Var(c.str()?))),
            1 => {
                let f = c.str()?;
                let n = c.u32()? as usize;
                let mut args = Vec::with_capacity(n);
                for _ in 0..n {
                    args.push(self.decode_type(c)?);
                }
                Ok(self.intern_type(TypeKey::App(f, args)))
            }
            t => Err(Error::Malformed(format!("type tag {t}"))),
        }
    }

    fn decode_term(&mut self, c: &mut Cursor<'_>) -> Result<u32, Error> {
        match c.u8()? {
            0 => {
                let n = c.str()?;
                let ty = self.decode_type(c)?;
                Ok(self.intern_term(TermKey::Var(n, ty)))
            }
            1 => {
                let n = c.str()?;
                let ty = self.decode_type(c)?;
                Ok(self.intern_term(TermKey::Const(n, ty)))
            }
            2 => {
                let f = self.decode_term(c)?;
                let x = self.decode_term(c)?;
                Ok(self.intern_term(TermKey::Comb(f, x)))
            }
            3 => {
                let v = self.decode_term(c)?;
                let b = self.decode_term(c)?;
                Ok(self.intern_term(TermKey::Abs(v, b)))
            }
            t => Err(Error::Malformed(format!("term tag {t}"))),
        }
    }

    fn type_eq(&self, a: u32, b: u32) -> bool {
        if a == b {
            return true;
        }
        match (&self.types[a as usize], &self.types[b as usize]) {
            (TypeKey::Var(x), TypeKey::Var(y)) => x == y,
            (TypeKey::App(f, xs), TypeKey::App(g, ys)) => {
                f == g
                    && xs.len() == ys.len()
                    && xs.iter().zip(ys).all(|(x, y)| self.type_eq(*x, *y))
            }
            _ => false,
        }
    }

    fn canon_type(&self, out: &mut Vec<u8>, id: u32) {
        match &self.types[id as usize] {
            TypeKey::Var(v) => {
                put_u8(out, 0);
                put_str(out, v);
            }
            TypeKey::App(f, args) => {
                put_u8(out, 1);
                put_str(out, f);
                put_u32(out, args.len() as u32);
                for a in args {
                    self.canon_type(out, *a);
                }
            }
        }
    }

    fn canon_term(&self, out: &mut Vec<u8>, id: u32, binders: &mut Vec<(String, u32)>) {
        match &self.terms[id as usize] {
            TermKey::Var(n, ty) => {
                if let Some(idx) = binders
                    .iter()
                    .rev()
                    .position(|(bn, bt)| bn == n && self.type_eq(*bt, *ty))
                {
                    put_u8(out, 4);
                    put_u32(out, idx as u32);
                } else {
                    put_u8(out, 0);
                    put_str(out, n);
                    self.canon_type(out, *ty);
                }
            }
            TermKey::Const(n, ty) => {
                put_u8(out, 1);
                put_str(out, n);
                self.canon_type(out, *ty);
            }
            TermKey::Comb(a, b) => {
                put_u8(out, 2);
                self.canon_term(out, *a, binders);
                self.canon_term(out, *b, binders);
            }
            TermKey::Abs(v, b) => match &self.terms[*v as usize] {
                TermKey::Var(n, ty) => {
                    put_u8(out, 5);
                    self.canon_type(out, *ty);
                    binders.push((n.clone(), *ty));
                    self.canon_term(out, *b, binders);
                    binders.pop();
                }
                _ => {
                    put_u8(out, 6);
                    self.canon_term(out, *v, binders);
                    self.canon_term(out, *b, binders);
                }
            },
        }
    }

    fn digest_term(&self, id: u32) -> [u8; 32] {
        let mut out = Vec::new();
        self.canon_term(&mut out, id, &mut Vec::new());
        sha256(&out)
    }
}

// ---------------------------------------------------------------------------
// Finalisation: raw stream -> compact object
// ---------------------------------------------------------------------------

fn mark_reachable(scan: &RawScan, path: &Path) -> Result<Vec<bool>, Error> {
    let n = scan.nodes.len();
    if n == 0 {
        return Err(Error::Malformed("no nodes".into()));
    }
    let root = scan.footer.root as usize;
    if root >= n {
        return Err(Error::ParentOutOfRange {
            node: root as u64,
            parent: root as u64,
        });
    }
    let f = File::open(path)?;
    let mut reach = vec![false; n];
    let mut stack = vec![root as u32];
    reach[root] = true;
    while let Some(id) = stack.pop() {
        let m = &scan.nodes[id as usize];
        for p in [m.p0, m.p1] {
            if p == ABSENT {
                continue;
            }
            if p as usize >= n || p >= id {
                return Err(Error::ParentOutOfRange {
                    node: id as u64,
                    parent: p as u64,
                });
            }
            if !reach[p as usize] {
                reach[p as usize] = true;
                stack.push(p);
            }
        }
        // Touch the prefix so a truncated record is detected here too.
        let _ = read_node_prefix(&f, scan.nodes[id as usize].payload_off - 13)?;
    }
    Ok(reach)
}

/// Encode a raw payload into the compact (term-id) form.
fn compact_payload(rule: Rule, raw: &[u8], it: &mut Interner) -> Result<Vec<u8>, Error> {
    let mut c = Cursor::new(raw);
    let mut out = Vec::new();
    match rule {
        Rule::Refl | Rule::Abs | Rule::Beta | Rule::Assume | Rule::Axiom | Rule::Definition => {
            let t = it.decode_term(&mut c)?;
            put_u32(&mut out, t);
        }
        Rule::Trans | Rule::MkComb | Rule::EqMp | Rule::DeductAntisym => {}
        Rule::InstType => {
            let n = c.u32()? as usize;
            put_u32(&mut out, n as u32);
            for _ in 0..n {
                let a = it.decode_type(&mut c)?;
                let b = it.decode_type(&mut c)?;
                put_u32(&mut out, a);
                put_u32(&mut out, b);
            }
        }
        Rule::Inst => {
            let n = c.u32()? as usize;
            put_u32(&mut out, n as u32);
            for _ in 0..n {
                let a = it.decode_term(&mut c)?;
                let b = it.decode_term(&mut c)?;
                put_u32(&mut out, a);
                put_u32(&mut out, b);
            }
        }
        Rule::TypeDefAbs | Rule::TypeDefRep => {
            let _which = c.u8()?;
            let tyname = c.str()?;
            let absname = c.str()?;
            let repname = c.str()?;
            put_str(&mut out, &tyname);
            put_str(&mut out, &absname);
            put_str(&mut out, &repname);
        }
        Rule::Import => {
            let nh = c.u32()? as usize;
            put_u32(&mut out, nh as u32);
            for _ in 0..nh {
                let t = it.decode_term(&mut c)?;
                put_u32(&mut out, t);
            }
            let concl = it.decode_term(&mut c)?;
            put_u32(&mut out, concl);
        }
    }
    if c.remaining() != 0 {
        return Err(Error::Malformed(format!(
            "{} payload has {} trailing bytes",
            rule.name(),
            c.remaining()
        )));
    }
    Ok(out)
}

/// Finalise a raw stream into a compact object and return its summary.
pub fn finalize(raw_path: &Path, obj_path: &Path) -> Result<Summary, Error> {
    let scan = scan_raw(raw_path)?;
    let reach = mark_reachable(&scan, raw_path)?;
    let n = scan.nodes.len() as u64;

    let mut it = Interner::default();
    let f = File::open(raw_path)?;

    // Statement and hypotheses (interned once).
    let stmt = intern_rc_term(&mut it, &scan.footer.concl)?;
    let mut hyp_ids = Vec::with_capacity(scan.footer.hyps.len());
    for h in &scan.footer.hyps {
        hyp_ids.push(intern_rc_term(&mut it, h)?);
    }

    // Emit reachable nodes in ascending id order (topological: parents < id),
    // remapping parent ids.
    let mut newid = vec![u32::MAX; scan.nodes.len()];
    let mut emitted: Vec<(Rule, u32, u32, Vec<u8>)> = Vec::new();
    for (id, m) in scan.nodes.iter().enumerate() {
        if !reach[id] {
            continue;
        }
        let raw = read_payload(&f, m)?;
        let payload = compact_payload(m.rule, &raw, &mut it)?;
        let p0 = if m.p0 == ABSENT {
            ABSENT
        } else {
            newid[m.p0 as usize]
        };
        let p1 = if m.p1 == ABSENT {
            ABSENT
        } else {
            newid[m.p1 as usize]
        };
        if (m.p0 != ABSENT && p0 == u32::MAX) || (m.p1 != ABSENT && p1 == u32::MAX) {
            return Err(Error::ParentOutOfRange {
                node: id as u64,
                parent: m.p0 as u64,
            });
        }
        newid[id] = emitted.len() as u32;
        emitted.push((m.rule, p0, p1, payload));
    }
    let root_new = newid[scan.footer.root as usize];
    if root_new == u32::MAX || root_new as usize + 1 != emitted.len() {
        return Err(Error::RootNotLast);
    }

    // Import and axiom statement digests (sorted, deduplicated).
    let mut import_digests: BTreeSet<[u8; 32]> = BTreeSet::new();
    let mut axiom_digests: BTreeSet<[u8; 32]> = BTreeSet::new();
    for (rule, _p0, _p1, payload) in &emitted {
        match rule {
            Rule::Import => {
                let mut c = Cursor::new(payload);
                let nh = c.u32()? as usize;
                for _ in 0..nh {
                    let _ = c.u32()?;
                }
                let concl = c.u32()?;
                import_digests.insert(it.digest_term(concl));
            }
            Rule::Axiom => {
                let mut c = Cursor::new(payload);
                let concl = c.u32()?;
                axiom_digests.insert(it.digest_term(concl));
            }
            _ => {}
        }
    }

    // Serialise.
    let stmt_digest = it.digest_term(stmt);
    let mut out = Vec::new();
    out.extend_from_slice(OBJ_MAGIC);
    out.push(OBJ_VERSION);
    put_str(&mut out, &scan.footer.name);
    put_u32(&mut out, n as u32); // raw nodes scanned (for the record)
    put_u64(&mut out, scan.footer.inference_total);
    put_u64(&mut out, scan.footer.inference_begin);
    put_u32(&mut out, root_new);
    put_u32(&mut out, stmt);
    out.extend_from_slice(&stmt_digest);
    put_u32(&mut out, hyp_ids.len() as u32);
    for h in &hyp_ids {
        put_u32(&mut out, *h);
    }
    put_u32(&mut out, axiom_digests.len() as u32);
    for d in &axiom_digests {
        out.extend_from_slice(d);
    }
    put_u32(&mut out, import_digests.len() as u32);
    for d in &import_digests {
        out.extend_from_slice(d);
    }
    // Axiom registry (human-readable; not part of any cryptographic binding
    // beyond the file digest).
    put_u32(&mut out, scan.footer.axioms.len() as u32);
    for (id, dg, txt) in &scan.footer.axioms {
        put_u32(&mut out, *id);
        put_str(&mut out, dg);
        put_str(&mut out, txt);
    }

    // Type table.
    put_u32(&mut out, it.types.len() as u32);
    for k in &it.types {
        match k {
            TypeKey::Var(v) => {
                put_u8(&mut out, 0);
                put_str(&mut out, v);
            }
            TypeKey::App(f, args) => {
                put_u8(&mut out, 1);
                put_str(&mut out, f);
                put_u32(&mut out, args.len() as u32);
                for a in args {
                    put_u32(&mut out, *a);
                }
            }
        }
    }
    // Term table.
    put_u32(&mut out, it.terms.len() as u32);
    for k in &it.terms {
        match k {
            TermKey::Var(n, ty) => {
                put_u8(&mut out, 0);
                put_str(&mut out, n);
                put_u32(&mut out, *ty);
            }
            TermKey::Const(n, ty) => {
                put_u8(&mut out, 1);
                put_str(&mut out, n);
                put_u32(&mut out, *ty);
            }
            TermKey::Comb(a, b) => {
                put_u8(&mut out, 2);
                put_u32(&mut out, *a);
                put_u32(&mut out, *b);
            }
            TermKey::Abs(a, b) => {
                put_u8(&mut out, 3);
                put_u32(&mut out, *a);
                put_u32(&mut out, *b);
            }
        }
    }
    // Node table.
    put_u32(&mut out, emitted.len() as u32);
    for (rule, p0, p1, payload) in &emitted {
        put_u8(&mut out, *rule as u8);
        put_u32(&mut out, *p0);
        put_u32(&mut out, *p1);
        put_u32(&mut out, payload.len() as u32);
        out.extend_from_slice(payload);
    }

    let file_digest = sha256(&out);
    out.extend_from_slice(&file_digest);
    out.extend_from_slice(OBJ_END);

    std::fs::write(obj_path, &out)?;

    Ok(Summary {
        theorem: scan.footer.name.clone(),
        statement_digest: hex32(&stmt_digest),
        derivation_digest: hex32(&file_digest),
        node_count: emitted.len() as u64,
        bytes: out.len() as u64,
        root_id: root_new as u64,
        inference_total: scan.footer.inference_total,
        inference_begin: scan.footer.inference_begin,
        imports: import_digests.len() as u64,
        axioms: scan
            .footer
            .axioms
            .iter()
            .map(|(_, d, _)| d.clone())
            .collect(),
        import_digests: import_digests.iter().map(hex32).collect(),
    })
}

/// Intern an `Rc<Term>` into the interner (bottom-up).
fn intern_rc_term(it: &mut Interner, t: &TermRef) -> Result<u32, Error> {
    fn intern_ty(it: &mut Interner, ty: &Type) -> u32 {
        match ty {
            Type::Var(v) => it.intern_type(TypeKey::Var(v.clone())),
            Type::App(f, args) => {
                let ids: Vec<u32> = args.iter().map(|a| intern_ty(it, a)).collect();
                it.intern_type(TypeKey::App(f.clone(), ids))
            }
        }
    }
    Ok(match &**t {
        Term::Var(n, ty) => {
            let ty = intern_ty(it, ty);
            it.intern_term(TermKey::Var(n.clone(), ty))
        }
        Term::Const(n, ty) => {
            let ty = intern_ty(it, ty);
            it.intern_term(TermKey::Const(n.clone(), ty))
        }
        Term::Comb(a, b) => {
            let a = intern_rc_term(it, a)?;
            let b = intern_rc_term(it, b)?;
            it.intern_term(TermKey::Comb(a, b))
        }
        Term::Abs(v, b) => {
            let v = intern_rc_term(it, v)?;
            let b = intern_rc_term(it, b)?;
            it.intern_term(TermKey::Abs(v, b))
        }
    })
}

// ---------------------------------------------------------------------------
// Partial stream statistics
// ---------------------------------------------------------------------------

/// Statistics over a (possibly truncated) raw stream. Used to size a recording
/// run without needing it to finish.
#[derive(Clone, Debug)]
pub struct StreamStats {
    pub nodes: u64,
    pub bytes: u64,
    pub payload_bytes: u64,
    pub max_payload: u32,
    pub finished: bool,
}

pub fn scan_stream(path: &Path) -> Result<StreamStats, Error> {
    let mut r = StreamReader::open(path)?;
    let magic = r.exact(RAW_MAGIC.len())?;
    if magic != RAW_MAGIC {
        return Err(Error::RawMagic);
    }
    let mut nodes = 0u64;
    let mut payload_bytes = 0u64;
    let mut max_payload = 0u32;
    let mut finished = false;
    loop {
        let tag = match r.u8() {
            Ok(t) => t,
            Err(_) => break,
        };
        let len = match r.u32() {
            Ok(v) => v as usize,
            Err(_) => break,
        };
        match tag {
            1 => {
                if len < 13 {
                    break;
                }
                let _rule = r.u8()?;
                let _p0 = r.u32()?;
                let _p1 = r.u32()?;
                let plen = r.u32()?;
                if 13 + plen as usize != len {
                    break;
                }
                if r.skip(plen as usize).is_err() {
                    break;
                }
                nodes += 1;
                payload_bytes += plen as u64;
                max_payload = max_payload.max(plen);
            }
            2 => {
                finished = true;
                break;
            }
            _ => break,
        }
    }
    let bytes = std::fs::metadata(path)?.len();
    Ok(StreamStats {
        nodes,
        bytes,
        payload_bytes,
        max_payload,
        finished,
    })
}

// ---------------------------------------------------------------------------
// Object parsing and validation
// ---------------------------------------------------------------------------

struct ParsedObject {
    name: String,
    raw_nodes: u64,
    inference_total: u64,
    inference_begin: u64,
    root: u32,
    statement: TermRef,
    statement_digest: [u8; 32],
    hyps: Vec<TermRef>,
    axiom_digests: BTreeSet<[u8; 32]>,
    import_digests: BTreeSet<[u8; 32]>,
    axioms: Vec<(u32, String, String)>,
    nodes: Vec<ObjNode>,
    types: Vec<Type>,
    terms: Vec<TermRef>,
    file_digest: [u8; 32],
}

struct ObjNode {
    rule: Rule,
    p0: u32,
    p1: u32,
    payload: Vec<u8>,
}

fn parse_object(path: &Path) -> Result<ParsedObject, Error> {
    let bytes = std::fs::read(path)?;
    if bytes.len() < OBJ_MAGIC.len() + 1 + 32 + OBJ_END.len() {
        return Err(Error::CountMismatch {
            what: "file length",
        });
    }
    if &bytes[..OBJ_MAGIC.len()] != OBJ_MAGIC {
        return Err(Error::BadMagic);
    }
    let body_end = bytes.len() - 32 - OBJ_END.len();
    if &bytes[body_end + 32..] != OBJ_END {
        return Err(Error::BadMagic);
    }
    let stored = &bytes[body_end..body_end + 32];
    let computed = sha256(&bytes[..body_end]);
    if stored != computed {
        return Err(Error::DigestMismatch);
    }
    let mut file_digest = [0u8; 32];
    file_digest.copy_from_slice(stored);

    let body = &bytes[..body_end];
    let mut c = Cursor::new(body);
    c.take(OBJ_MAGIC.len())?;
    let version = c.u8()?;
    if version != OBJ_VERSION {
        return Err(Error::BadVersion(version));
    }
    let name = c.str()?;
    let raw_nodes = c.u32()? as u64;
    let inference_total = c.u64()?;
    let inference_begin = c.u64()?;
    let root = c.u32()?;
    let stmt_id = c.u32()?;
    let mut statement_digest = [0u8; 32];
    statement_digest.copy_from_slice(c.take(32)?);
    let nh = c.u32()? as usize;
    let mut hyp_ids = Vec::with_capacity(nh);
    for _ in 0..nh {
        hyp_ids.push(c.u32()?);
    }
    let na = c.u32()? as usize;
    let mut axiom_digests = BTreeSet::new();
    for _ in 0..na {
        let mut d = [0u8; 32];
        d.copy_from_slice(c.take(32)?);
        axiom_digests.insert(d);
    }
    let ni = c.u32()? as usize;
    let mut import_digests = BTreeSet::new();
    for _ in 0..ni {
        let mut d = [0u8; 32];
        d.copy_from_slice(c.take(32)?);
        import_digests.insert(d);
    }
    let nreg = c.u32()? as usize;
    let mut axioms = Vec::with_capacity(nreg);
    for _ in 0..nreg {
        let id = c.u32()?;
        let dg = c.str()?;
        let txt = c.str()?;
        axioms.push((id, dg, txt));
    }

    // Type table.
    let nt = c.u32()? as usize;
    let mut types_raw: Vec<TypeKey> = Vec::with_capacity(nt);
    for _ in 0..nt {
        match c.u8()? {
            0 => types_raw.push(TypeKey::Var(c.str()?)),
            1 => {
                let f = c.str()?;
                let n = c.u32()? as usize;
                let mut args = Vec::with_capacity(n);
                for _ in 0..n {
                    let id = c.u32()?;
                    if id as usize >= types_raw.len() {
                        return Err(Error::TableReference {
                            what: "type",
                            id: id as u64,
                        });
                    }
                    args.push(id);
                }
                types_raw.push(TypeKey::App(f, args));
            }
            t => return Err(Error::Malformed(format!("type table tag {t}"))),
        }
    }
    let mut types: Vec<Type> = Vec::with_capacity(nt);
    for k in &types_raw {
        let ty = match k {
            TypeKey::Var(v) => Type::Var(v.clone()),
            TypeKey::App(f, args) => Type::App(
                f.clone(),
                args.iter().map(|i| types[*i as usize].clone()).collect(),
            ),
        };
        types.push(ty);
    }

    // Term table.
    let nterm = c.u32()? as usize;
    let mut term_keys: Vec<TermKey> = Vec::with_capacity(nterm);
    for _ in 0..nterm {
        match c.u8()? {
            0 => {
                let n = c.str()?;
                let ty = c.u32()?;
                if ty as usize >= types.len() {
                    return Err(Error::TableReference {
                        what: "term type",
                        id: ty as u64,
                    });
                }
                term_keys.push(TermKey::Var(n, ty));
            }
            1 => {
                let n = c.str()?;
                let ty = c.u32()?;
                if ty as usize >= types.len() {
                    return Err(Error::TableReference {
                        what: "term type",
                        id: ty as u64,
                    });
                }
                term_keys.push(TermKey::Const(n, ty));
            }
            2 | 3 => {
                let op = body[c.p - 1];
                let a = c.u32()?;
                let b = c.u32()?;
                if a as usize >= term_keys.len() || b as usize >= term_keys.len() {
                    return Err(Error::TableReference {
                        what: "term child",
                        id: a.max(b) as u64,
                    });
                }
                if op == 2 {
                    term_keys.push(TermKey::Comb(a, b));
                } else {
                    term_keys.push(TermKey::Abs(a, b));
                }
            }
            t => return Err(Error::Malformed(format!("term table tag {t}"))),
        }
    }
    let mut terms: Vec<TermRef> = Vec::with_capacity(nterm);
    for k in &term_keys {
        let t: TermRef = match k {
            TermKey::Var(n, ty) => {
                Rc::new(Term::Var(n.clone(), Rc::new(types[*ty as usize].clone())))
            }
            TermKey::Const(n, ty) => {
                Rc::new(Term::Const(n.clone(), Rc::new(types[*ty as usize].clone())))
            }
            TermKey::Comb(a, b) => Rc::new(Term::Comb(
                terms[*a as usize].clone(),
                terms[*b as usize].clone(),
            )),
            TermKey::Abs(a, b) => Rc::new(Term::Abs(
                terms[*a as usize].clone(),
                terms[*b as usize].clone(),
            )),
        };
        terms.push(t);
    }
    if stmt_id as usize >= terms.len() {
        return Err(Error::TableReference {
            what: "statement term",
            id: stmt_id as u64,
        });
    }
    let mut hyps = Vec::with_capacity(hyp_ids.len());
    for h in hyp_ids {
        if h as usize >= terms.len() {
            return Err(Error::TableReference {
                what: "hypothesis term",
                id: h as u64,
            });
        }
        hyps.push(terms[h as usize].clone());
    }

    // Node table.
    let nn = c.u32()? as usize;
    let mut nodes = Vec::with_capacity(nn);
    for i in 0..nn {
        let rule = Rule::from_u8(c.u8()?)?;
        let p0 = c.u32()?;
        let p1 = c.u32()?;
        let plen = c.u32()? as usize;
        let payload = c.take(plen)?.to_vec();
        // Parents must be earlier nodes (or absent).
        let mut declared = 0;
        for p in [p0, p1] {
            if p == ABSENT {
                continue;
            }
            declared += 1;
            if p as usize >= i {
                return Err(Error::ParentOutOfRange {
                    node: i as u64,
                    parent: p as u64,
                });
            }
        }
        if declared != rule.parents_needed() {
            return Err(Error::RuleFailure {
                node: i as u64,
                rule: rule.name(),
                why: format!("{declared} parents, expected {}", rule.parents_needed()),
            });
        }
        nodes.push(ObjNode {
            rule,
            p0,
            p1,
            payload,
        });
    }
    if c.remaining() != 0 {
        return Err(Error::CountMismatch { what: "node table" });
    }
    if root as usize >= nodes.len() {
        return Err(Error::ParentOutOfRange {
            node: root as u64,
            parent: root as u64,
        });
    }

    Ok(ParsedObject {
        name,
        raw_nodes,
        inference_total,
        inference_begin,
        root,
        statement: terms[stmt_id as usize].clone(),
        statement_digest,
        hyps,
        axiom_digests,
        import_digests,
        axioms,
        nodes,
        types,
        terms,
        file_digest,
    })
}

fn decode_term_id(c: &mut Cursor<'_>, terms: &[TermRef]) -> Result<TermRef, Error> {
    let id = c.u32()?;
    terms
        .get(id as usize)
        .cloned()
        .ok_or(Error::TableReference {
            what: "payload term",
            id: id as u64,
        })
}

fn decode_type_id(c: &mut Cursor<'_>, types: &[Type]) -> Result<Ty, Error> {
    let id = c.u32()?;
    types
        .get(id as usize)
        .map(|t| Rc::new(t.clone()))
        .ok_or(Error::TableReference {
            what: "payload type",
            id: id as u64,
        })
}

/// Validate an object by independently replaying it.
///
/// Refusals:
/// * truncation / corruption — trailer digest or declared counts disagree;
/// * splicing — a node does not follow from its parents, or the replayed root
///   conclusion differs from the recorded statement;
/// * wrong conclusion — `ConclusionMismatch`.
pub fn validate(path: &Path) -> Result<Validation, Error> {
    let p = parse_object(path)?;

    // Header self-consistency: the stored statement hashes to the stored digest.
    if statement_digest(&p.statement) != p.statement_digest {
        return Err(Error::StatementSelfInconsistent);
    }

    let mut thms: Vec<Thm> = Vec::with_capacity(p.nodes.len());
    let mut axiom_seen: BTreeSet<[u8; 32]> = BTreeSet::new();
    let mut import_seen: BTreeSet<[u8; 32]> = BTreeSet::new();
    for (i, n) in p.nodes.iter().enumerate() {
        // Resolve the parents into concrete theorems.
        let mut parents: Vec<Thm> = Vec::new();
        for pid in [n.p0, n.p1] {
            if pid == ABSENT {
                continue;
            }
            let th = thms
                .get(pid as usize)
                .ok_or(Error::ParentOutOfRange {
                    node: i as u64,
                    parent: pid as u64,
                })?
                .clone();
            parents.push(th);
        }
        let th = apply_rule_checked(i as u64, n, &p, &parents)?;
        match n.rule {
            Rule::Axiom => {
                axiom_seen.insert(statement_digest(&th.concl));
            }
            Rule::Import => {
                import_seen.insert(statement_digest(&th.concl));
            }
            _ => {}
        }
        thms.push(th);
    }

    if axiom_seen != p.axiom_digests {
        return Err(Error::AssumptionSetMismatch { what: "axiom" });
    }
    if import_seen != p.import_digests {
        return Err(Error::AssumptionSetMismatch { what: "import" });
    }

    let root = &thms[p.root as usize];
    if !root.hyps.is_empty() {
        return Err(Error::RootNotClosed(root.hyps.len()));
    }
    if p.hyps.len() != root.hyps.len() {
        return Err(Error::CountMismatch {
            what: "root hypotheses",
        });
    }
    let got = statement_digest(&root.concl);
    if got != p.statement_digest {
        return Err(Error::ConclusionMismatch {
            expected: hex32(&p.statement_digest),
            got: hex32(&got),
        });
    }

    Ok(Validation {
        theorem: p.name,
        statement_digest: hex32(&p.statement_digest),
        derivation_digest: hex32(&p.file_digest),
        node_count: p.nodes.len() as u64,
        raw_nodes: p.raw_nodes,
        inference_total: p.inference_total,
        inference_begin: p.inference_begin,
        imports: p.import_digests.len() as u64,
        axioms: p.axioms.iter().map(|(_, d, _)| d.clone()).collect(),
    })
}

/// Like [`apply_rule`] but taking already-resolved parents, so the checker does
/// not need shared mutable access to the theorem vector.
fn apply_rule_checked(
    node: u64,
    n: &ObjNode,
    p: &ParsedObject,
    parents: &[Thm],
) -> Result<Thm, Error> {
    let rule = n.rule;
    let fail = |why: String| Error::RuleFailure {
        node,
        rule: rule.name(),
        why,
    };
    let mut c = Cursor::new(&n.payload);
    let mut hyps = Vec::new();
    let concl: TermRef;
    match rule {
        Rule::Refl => {
            let t = decode_term_id(&mut c, &p.terms)?;
            concl = mk_eq(t.clone(), t).map_err(|e| fail(e.to_string()))?;
        }
        Rule::Trans => {
            let a = &parents[0];
            let b = &parents[1];
            let (l, m1) =
                dest_eq(&a.concl).ok_or_else(|| fail("parent 0 is not an equation".into()))?;
            let (m2, r) =
                dest_eq(&b.concl).ok_or_else(|| fail("parent 1 is not an equation".into()))?;
            if !alpha_eq(&m1, &m2) {
                return Err(fail("middle terms differ".into()));
            }
            let _ = l; // l is re-read below via the equality constant
            let eql = match &*a.concl {
                Term::Comb(f, _) => f.clone(),
                _ => unreachable!(),
            };
            concl = Rc::new(Term::Comb(eql, r));
            hyps = term_union(&a.hyps, &b.hyps);
        }
        Rule::MkComb => {
            let a = &parents[0];
            let b = &parents[1];
            let (l1, r1) =
                dest_eq(&a.concl).ok_or_else(|| fail("parent 0 is not an equation".into()))?;
            let (l2, r2) =
                dest_eq(&b.concl).ok_or_else(|| fail("parent 1 is not an equation".into()))?;
            let ty = match &*type_of(&r1)? {
                Type::App(f, args) if f == "fun" && args.len() == 2 => args[0].clone(),
                _ => return Err(fail("MK_COMB: range is not a function".into())),
            };
            if ty != *type_of(&r2)? {
                return Err(fail("MK_COMB: types do not agree".into()));
            }
            let l = Rc::new(Term::Comb(l1, l2));
            let r = Rc::new(Term::Comb(r1, r2));
            concl = mk_eq(l, r).map_err(|e| fail(e.to_string()))?;
            hyps = term_union(&a.hyps, &b.hyps);
        }
        Rule::Abs => {
            let a = &parents[0];
            let v = decode_term_id(&mut c, &p.terms)?;
            if dest_var(&v).is_none() {
                return Err(fail("ABS: not a variable".into()));
            }
            let (l, r) = dest_eq(&a.concl).ok_or_else(|| fail("ABS: not an equation".into()))?;
            if a.hyps.iter().any(|h| vfree_in(&v, h)) {
                return Err(fail("ABS: variable free in hypotheses".into()));
            }
            let l2 = Rc::new(Term::Abs(v.clone(), l));
            let r2 = Rc::new(Term::Abs(v, r));
            concl = mk_eq(l2, r2).map_err(|e| fail(e.to_string()))?;
            hyps = a.hyps.clone();
        }
        Rule::Beta => {
            let t = decode_term_id(&mut c, &p.terms)?;
            let (v, bod, arg) = match &*t {
                Term::Comb(f, arg) => match &**f {
                    Term::Abs(v, bod) => (v.clone(), bod.clone(), arg.clone()),
                    _ => return Err(fail("BETA: not a redex".into())),
                },
                _ => return Err(fail("BETA: not a redex".into())),
            };
            if !teq(&arg, &v) {
                return Err(fail("BETA: argument is not the bound variable".into()));
            }
            concl = mk_eq(t.clone(), bod).map_err(|e| fail(e.to_string()))?;
        }
        Rule::Assume => {
            let t = decode_term_id(&mut c, &p.terms)?;
            if *type_of(&t)? != Type::App("bool".into(), vec![]) {
                return Err(fail("ASSUME: not a proposition".into()));
            }
            hyps = vec![t.clone()];
            concl = t;
        }
        Rule::EqMp => {
            let a = &parents[0];
            let b = &parents[1];
            let (l, r) = dest_eq(&a.concl).ok_or_else(|| fail("EQ_MP: not an equation".into()))?;
            if !alpha_eq(&l, &b.concl) {
                return Err(fail("EQ_MP: premise does not match".into()));
            }
            concl = r;
            hyps = term_union(&a.hyps, &b.hyps);
        }
        Rule::DeductAntisym => {
            let a = &parents[0];
            let b = &parents[1];
            let asl1 = term_remove(&b.concl, &a.hyps);
            let asl2 = term_remove(&a.concl, &b.hyps);
            hyps = term_union(&asl1, &asl2);
            concl = mk_eq(a.concl.clone(), b.concl.clone()).map_err(|e| fail(e.to_string()))?;
        }
        Rule::InstType => {
            let a = &parents[0];
            let k = c.u32()? as usize;
            let mut tyin = Vec::with_capacity(k);
            for _ in 0..k {
                let x = decode_type_id(&mut c, &p.types)?;
                let y = decode_type_id(&mut c, &p.types)?;
                tyin.push((x, y));
            }
            let mut hs = Vec::new();
            for h in &a.hyps {
                let h2 = inst_checked(&[], &tyin, h).map_err(fail)?;
                if !hs.iter().any(|u| alpha_eq(&h2, u)) {
                    hs.push(h2);
                }
            }
            hyps = hs;
            concl = inst_checked(&[], &tyin, &a.concl).map_err(fail)?;
        }
        Rule::Inst => {
            let a = &parents[0];
            let k = c.u32()? as usize;
            let mut sub = Vec::with_capacity(k);
            for _ in 0..k {
                let t = decode_term_id(&mut c, &p.terms)?;
                let v = decode_term_id(&mut c, &p.terms)?;
                sub.push((t, v));
            }
            hyps = term_image(&sub, &a.hyps).map_err(|e| fail(e.to_string()))?;
            concl = vsubst(&sub, &a.concl).map_err(|e| fail(e.to_string()))?;
        }
        Rule::Axiom => {
            let t = decode_term_id(&mut c, &p.terms)?;
            if *type_of(&t)? != Type::App("bool".into(), vec![]) {
                return Err(fail("new_axiom: not a proposition".into()));
            }
            concl = t;
        }
        Rule::Definition => {
            let t = decode_term_id(&mut c, &p.terms)?;
            let (l, r) = dest_eq(&t).ok_or_else(|| fail("definition: not an equation".into()))?;
            let (cname, cty) = match &*l {
                Term::Var(n, ty) => (n.clone(), ty.clone()),
                _ => return Err(fail("definition: left side is not a variable".into())),
            };
            if !freesin(&[], &r) {
                return Err(fail("definition: right side is not closed".into()));
            }
            let lhs_vars: BTreeSet<String> = {
                let mut s = BTreeSet::new();
                fn tv(ty: &Type, s: &mut BTreeSet<String>) {
                    match ty {
                        Type::Var(v) => {
                            s.insert(v.clone());
                        }
                        Type::App(_, a) => a.iter().for_each(|x| tv(x, s)),
                    }
                }
                tv(&cty, &mut s);
                s
            };
            if !type_vars_in_term(&r).is_subset(&lhs_vars) {
                return Err(fail("definition: type variables not reflected".into()));
            }
            let c2 = Rc::new(Term::Const(cname, cty));
            concl = mk_eq(c2, r).map_err(|e| fail(e.to_string()))?;
        }
        Rule::TypeDefAbs | Rule::TypeDefRep => {
            let a = &parents[0];
            if !a.hyps.is_empty() {
                return Err(fail("type definition: assumptions in theorem".into()));
            }
            let tyname = c.str()?;
            let absname = c.str()?;
            let repname = c.str()?;
            let (bigp, x) = dest_comb(&a.concl)
                .ok_or_else(|| fail("type definition: not a combination".into()))?;
            if !freesin(&[], &bigp) {
                return Err(fail("type definition: predicate is not closed".into()));
            }
            let tyvars: Vec<Type> = type_vars_in_term(&bigp)
                .into_iter()
                .map(Type::Var)
                .collect();
            let aty = Type::App(tyname.clone(), tyvars.clone());
            let rty = (*type_of(&x)?).clone();
            let absty = fun_ty(&rty, &aty);
            let repty = fun_ty(&aty, &rty);
            let abs = Rc::new(Term::Const(absname, absty));
            let rep = Rc::new(Term::Const(repname, repty));
            let a_var = Rc::new(Term::Var("a".into(), Rc::new(aty)));
            let r_var = Rc::new(Term::Var("r".into(), Rc::new(rty)));
            if rule == Rule::TypeDefAbs {
                let rep_a = mk_comb_checked(rep, a_var.clone()).map_err(|e| fail(e.to_string()))?;
                let lhs = mk_comb_checked(abs, rep_a).map_err(|e| fail(e.to_string()))?;
                concl = mk_eq(lhs, a_var).map_err(|e| fail(e.to_string()))?;
            } else {
                let rep_r = mk_comb_checked(rep, r_var.clone()).map_err(|e| fail(e.to_string()))?;
                let abs_rep_r = mk_comb_checked(abs, rep_r).map_err(|e| fail(e.to_string()))?;
                let inner = mk_eq(abs_rep_r, r_var.clone()).map_err(|e| fail(e.to_string()))?;
                let p_r = mk_comb_checked(bigp, r_var).map_err(|e| fail(e.to_string()))?;
                concl = mk_eq(p_r, inner).map_err(|e| fail(e.to_string()))?;
            }
        }
        Rule::Import => {
            let nh = c.u32()? as usize;
            for _ in 0..nh {
                hyps.push(decode_term_id(&mut c, &p.terms)?);
            }
            concl = decode_term_id(&mut c, &p.terms)?;
        }
    }
    if c.remaining() != 0 {
        return Err(fail(format!("{} trailing payload bytes", c.remaining())));
    }
    Ok(Thm { hyps, concl })
}
