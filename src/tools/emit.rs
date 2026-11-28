//! Reference-route answer emitters for ARM and x86 add/sub/neg tasks.
//!
//! The route simulated here is:
//!
//!   * digitize the inputs and run the machine code (the accumulate set is
//!     derived from the instruction stream by [`crate::tools::derive`]; a
//!     routine with no carry chain at all is run with plain `ARM_STEPS_TAC`),
//!   * expand the written bignum into a digit sum,
//!   * close with `BRIDGE_{ADD,SUB,NEG}_REF_TAC`.
//!
//! No flag *meaning* is derived and no curve arithmetic appears in the close.
//! The machine code, the query, the state names and the modulus name are read
//! from the benchmark inputs; nothing is copied from a human proof except the
//! simulation/expansion idiom, which is labelled `DEVELOPMENT_ONLY`.
//!
//! Guard modes come from [`crate::tools::guard`]. `--develop` (default) refuses
//! held-out task ids, objects and proof reads; `--sealed-eval` with an explicit
//! manifest and freeze tag allows exactly the listed ids and objects and still
//! refuses their proof text, including in `--mode dev`; `--spent` with
//! `--spent-manifest` re-answers already-disclosed development ids.
//!
//! The x86 sibling differs in two derived ways: the step list is disassembled in
//! Intel syntax and trimmed past the four-byte `ENDBR64` prefix, and the entry
//! offset is read from the query's `read RIP s = word (pc + k)` so the `PUSH`
//! prologue is not simulated. The simulation ends at the last instruction that
//! writes memory. Neither route interprets the carry flag, which matters on x86
//! because its CF is a *borrow* (see [`crate::tools::derive::x86_cf_meaning`]).

use crate::tools::derive;
use crate::tools::disasm;
use crate::tools::guard::{self, GuardPolicy, Refused};
use crate::tools::json::{self, Json};
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Either a guard refusal or an ordinary emitter error. The Python tools turned
/// both into a `SystemExit` with a message, so the CLI maps both to exit 1.
#[derive(Debug)]
pub enum EmitError {
    Refused(Refused),
    Other(String),
}

impl fmt::Display for EmitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EmitError::Refused(r) => write!(f, "refused: {r}"),
            EmitError::Other(m) => f.write_str(m),
        }
    }
}

impl From<Refused> for EmitError {
    fn from(r: Refused) -> EmitError {
        EmitError::Refused(r)
    }
}

impl From<String> for EmitError {
    fn from(m: String) -> EmitError {
        EmitError::Other(m)
    }
}

type Result<T> = std::result::Result<T, EmitError>;

fn other<T>(msg: impl Into<String>) -> Result<T> {
    Err(EmitError::Other(msg.into()))
}

// Transcribed fragments, named so they cannot hide in the emitted answer.
pub const DEVELOPMENT_ONLY: &[(&str, &str)] = &[
    (
        "simulation.accsteps",
        "The single ARM_ACCSTEPS_TAC block over the whole routine is the human \
         proofs' simulation idiom; the accumulate set itself is derived.",
    ),
    (
        "post.digit_expansion",
        "BIGNUM_EXPAND_CONV / BIGNUM_LEXPAND_CONV with the LAND_CONV/RAND_CONV \
         traversal and DISCARD_STATE_TAC are the human expansion idiom.",
    ),
    (
        "post.bitval_rewrites",
        "RULE_ASSUM_TAC(REWRITE_RULE[REAL_BITVAL_NOT; ADD_CLAUSES; \
         VAL_WORD_BITVAL]) is copied from the human add closes.",
    ),
];

/// The x86 labels, which travel with every emitted x86 answer.
pub const X86_DEVELOPMENT_ONLY: &[(&str, &str)] = &[
    (
        "simulation.accsteps",
        "The single X86_ACCSTEPS_TAC block over the whole routine is the x86 \
         human proofs' simulation idiom; the accumulate set itself is derived. \
         x86 NEG is dropped from the requested set because ACCUMULATE_ARITH_TAC \
         has no word_neg pattern (requesting it is a no-op).",
    ),
    (
        "simulation.entry",
        "Skipping the PUSH prologue follows the query's `read RIP s = word \
         (pc + k)`; that the harness means an executable entry (rather than a \
         non-executed prefix) is taken from the x86 proof style.",
    ),
    (
        "post.neg_flag_rewrite",
        "RULE_ASSUM_TAC(REWRITE_RULE[WORD_NEG_EQ_0; WORD_BITVAL_EQ_0]) is \
         copied from the x86 human closes. It is emitted only when the routine \
         contains NEG: NEG's CF is `~(x = 0)`, and without this rewrite the \
         bit-valued form `bitval(~(word_neg(word(bitval c)) = word 0))` is left \
         free by BITBLAST, making the residual SAT instance satisfiable (measured \
         on four of the six declared tasks).",
    ),
    (
        "post.zero_test_fold",
        "BRIDGE_NEG_REF_ZERO_TAC folds the machine's OR zero test and the \
         reference function's joined-word zero test to the same limb-wise \
         conjunction before the case split (BRIDGE_ZERO_TEST_TAC, built from \
         WORD_OR_EQ_0 and WORD_JOIN_EQ_0, both proved in tools/bridge.ml). The \
         fold is generic; what is transcribed is the decision to use it for \
         routines containing NEG. Measured: BITBLAST alone does not prove \
         `word_join a b = word 0 ==> word_or a b = word 0'.",
    ),
    (
        "post.digit_expansion",
        "BIGNUM_EXPAND_CONV / BIGNUM_LEXPAND_CONV with the LAND_CONV/RAND_CONV \
         traversal and DISCARD_STATE_TAC are the human expansion idiom.",
    ),
    (
        "post.bitval_rewrites",
        "RULE_ASSUM_TAC(REWRITE_RULE[REAL_BITVAL_NOT; ADD_CLAUSES; \
         VAL_WORD_BITVAL]) is copied from the human add closes.",
    ),
];

// ---------------------------------------------------------------------------
// Specification parsing
// ---------------------------------------------------------------------------

/// One `bignum_from_memory (ptr,n) s = value` precondition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputRef {
    pub pointer: String,
    pub value: String,
    pub digits: usize,
}

impl InputRef {
    pub fn digit_prefix(&self) -> String {
        format!("{}_", self.value)
    }
}

/// The postcondition equality, classified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Conclusion {
    NatMod {
        left: String,
        right: String,
        sum: String,
        modulus: String,
    },
    NatModSelf {
        minuend: String,
        subtrahend: String,
        modulus: String,
    },
    IntRem {
        minuend: String,
        subtrahend: String,
        modulus: String,
    },
}

impl Conclusion {
    pub fn modulus(&self) -> &str {
        match self {
            Conclusion::NatMod { modulus, .. }
            | Conclusion::NatModSelf { modulus, .. }
            | Conclusion::IntRem { modulus, .. } => modulus,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Conclusion::NatMod { .. } => "nat_mod",
            Conclusion::NatModSelf { .. } => "nat_mod_self",
            Conclusion::IntRem { .. } => "int_rem",
        }
    }
}

/// The specification extracted from a benchmark query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spec {
    pub variables: Vec<String>,
    pub inputs: Vec<InputRef>,
    pub result: String,
    pub digits: usize,
    pub limb_bits: usize,
    pub bits: usize,
    pub code_bytes: usize,
    pub guard: Vec<String>,
    pub conclusion: Conclusion,
    pub var_types: BTreeMap<String, String>,
}

struct Cursor<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> Cursor<'a> {
    fn at(s: &'a [u8], i: usize) -> Cursor<'a> {
        Cursor { s, i }
    }

    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }

    fn bump(&mut self) {
        self.i += 1;
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b) if b.is_ascii_whitespace()) {
            self.i += 1;
        }
    }

    fn ws_span(&mut self) -> usize {
        let start = self.i;
        self.skip_ws();
        self.i - start
    }

    fn eat(&mut self, want: u8) -> Option<()> {
        if self.peek() == Some(want) {
            self.i += 1;
            Some(())
        } else {
            None
        }
    }

    fn eat_lit(&mut self, lit: &str) -> Option<()> {
        if self.s[self.i..].starts_with(lit.as_bytes()) {
            self.i += lit.len();
            Some(())
        } else {
            None
        }
    }

    /// `\w+`
    fn word(&mut self) -> Option<String> {
        let start = self.i;
        while let Some(b) = self.peek() {
            if b.is_ascii_alphanumeric() || b == b'_' {
                self.i += 1;
            } else {
                break;
            }
        }
        if self.i == start {
            return None;
        }
        Some(
            std::str::from_utf8(&self.s[start..self.i])
                .ok()?
                .to_string(),
        )
    }

    /// `[A-Za-z_]\w*`
    fn word_alpha(&mut self) -> Option<String> {
        match self.peek() {
            Some(b) if b.is_ascii_alphabetic() || b == b'_' => self.word(),
            _ => None,
        }
    }

    /// `\d+`
    fn digits(&mut self) -> Option<String> {
        let start = self.i;
        while matches!(self.peek(), Some(b) if b.is_ascii_digit()) {
            self.i += 1;
        }
        if self.i == start {
            return None;
        }
        Some(
            std::str::from_utf8(&self.s[start..self.i])
                .ok()?
                .to_string(),
        )
    }

    /// `[0-9a-fA-F]+`
    fn hex_digits(&mut self) -> Option<String> {
        let start = self.i;
        while matches!(self.peek(), Some(b) if b.is_ascii_hexdigit()) {
            self.i += 1;
        }
        if self.i == start {
            return None;
        }
        Some(
            std::str::from_utf8(&self.s[start..self.i])
                .ok()?
                .to_string(),
        )
    }
}

fn find_from(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if from > hay.len() {
        return None;
    }
    hay[from..]
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| p + from)
}

fn unquote(query: &str) -> &str {
    let mut q = query.trim();
    if let Some(rest) = q.strip_prefix('`') {
        q = rest;
    }
    if let Some(rest) = q.strip_suffix('`') {
        q = rest;
    }
    q
}

/// Extract the variables, input references, guard and conclusion.
pub fn parse_spec(query: &str) -> std::result::Result<Spec, String> {
    let q = unquote(query);

    // `!\s*([A-Za-z0-9_ ]+?)\s*\.`
    let rest = q.trim_start();
    let rest = rest
        .strip_prefix('!')
        .ok_or_else(|| "query does not start with a universal quantifier".to_string())?;
    let rest = rest.trim_start();
    let run_bytes = rest
        .bytes()
        .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_' || *b == b' ')
        .count();
    if run_bytes == 0 {
        return Err("query does not start with a universal quantifier".to_string());
    }
    let group = rest[..run_bytes].trim_end();
    let after = rest[run_bytes..].trim_start_matches(|c: char| c.is_whitespace());
    if !after.starts_with('.') {
        return Err("query does not start with a universal quantifier".to_string());
    }
    let variables: Vec<String> = group.split_whitespace().map(|s| s.to_string()).collect();

    // bignum_from_memory (ptr,N) s = value, terminated by /\ or )
    let mut refs: Vec<InputRef> = Vec::new();
    let mut search = 0;
    while let Some(at) = find_from(q.as_bytes(), b"bignum_from_memory", search) {
        match parse_ref_at(q.as_bytes(), at) {
            Some((pointer, ndigits, value, end)) => {
                if variables.contains(&value) {
                    refs.push(InputRef {
                        pointer,
                        value,
                        digits: ndigits,
                    });
                }
                search = end;
            }
            None => search = at + 1,
        }
    }

    let no = parse_nonoverlapping(q)
        .ok_or_else(|| "cannot find nonoverlapping shape in query".to_string())?;

    let guard_text = find_guard(q).ok_or_else(|| "cannot find postcondition guard".to_string())?;
    let guard: Vec<String> = guard_text
        .split("/\\")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|p| p.to_string())
        .collect();

    let conclusion = parse_conclusion(q, &no.result_ptr)?;

    let mut var_types: BTreeMap<String, String> = BTreeMap::new();
    let mut pointers: Vec<&str> = refs.iter().map(|r| r.pointer.as_str()).collect();
    pointers.push(&no.result_ptr);
    for v in &variables {
        let ty = if pointers.contains(&v.as_str()) {
            "int64"
        } else {
            "num"
        };
        var_types.insert(v.clone(), ty.to_string());
    }

    let digits = no.digits;
    let limb_bits = no.limb_factor * 8;
    Ok(Spec {
        variables,
        inputs: refs,
        result: no.result_ptr,
        digits,
        limb_bits,
        bits: digits * limb_bits,
        code_bytes: no.code_bytes,
        guard,
        conclusion,
        var_types,
    })
}

fn parse_ref_at(s: &[u8], start: usize) -> Option<(String, usize, String, usize)> {
    let mut c = Cursor::at(s, start + "bignum_from_memory".len());
    c.skip_ws();
    c.eat(b'(')?;
    c.skip_ws();
    let pointer = c.word()?;
    c.skip_ws();
    c.eat(b',')?;
    c.skip_ws();
    let ndigits = c.digits()?;
    c.skip_ws();
    c.eat(b')')?;
    c.skip_ws();
    c.eat(b's')?;
    c.skip_ws();
    c.eat(b'=')?;
    c.skip_ws();
    let value = c.word_alpha()?;
    c.skip_ws();
    let lookahead_ok =
        (c.peek() == Some(b'/') && s.get(c.i + 1) == Some(&b'\\')) || c.peek() == Some(b')');
    if !lookahead_ok {
        return None;
    }
    Some((pointer, ndigits.parse().ok()?, value, c.i))
}

struct NonOverlapping {
    code_bytes: usize,
    result_ptr: String,
    limb_factor: usize,
    digits: usize,
}

fn parse_nonoverlapping(q: &str) -> Option<NonOverlapping> {
    let s = q.as_bytes();
    let mut search = 0;
    while let Some(at) = find_from(s, b"nonoverlapping", search) {
        let mut c = Cursor::at(s, at + "nonoverlapping".len());
        if let Some(parsed) = parse_nonoverlapping_at(&mut c) {
            return Some(parsed);
        }
        search = at + 1;
    }
    None
}

fn parse_nonoverlapping_at(c: &mut Cursor<'_>) -> Option<NonOverlapping> {
    c.skip_ws();
    c.eat(b'(')?;
    c.skip_ws();
    c.eat_lit("word")?;
    if c.ws_span() == 0 {
        return None;
    }
    c.eat_lit("pc")?;
    c.skip_ws();
    c.eat(b',')?;
    c.skip_ws();
    c.eat_lit("0x")?;
    let code = c.hex_digits()?;
    c.skip_ws();
    c.eat(b')')?;
    c.skip_ws();
    c.eat(b'(')?;
    c.skip_ws();
    let result_ptr = c.word()?;
    c.skip_ws();
    c.eat(b',')?;
    c.skip_ws();
    let limb_factor = c.digits()?;
    c.skip_ws();
    c.eat(b'*')?;
    c.skip_ws();
    let digits = c.digits()?;
    c.skip_ws();
    c.eat(b')')?;
    Some(NonOverlapping {
        code_bytes: usize::from_str_radix(&code, 16).ok()?,
        result_ptr,
        limb_factor: limb_factor.parse().ok()?,
        digits: digits.parse().ok()?,
    })
}

fn contains_p_identifier(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut search = 0;
    while let Some(at) = find_from(bytes, b"p_", search) {
        if let Some(next) = bytes.get(at + 2) {
            if next.is_ascii_alphanumeric() || *next == b'_' {
                return true;
            }
        }
        search = at + 1;
    }
    false
}

/// `\(([^()]*?p_[A-Za-z0-9_]+[^()]*?)\s*==>\s*(?:&\(|\bbignum_from_memory)`
fn find_guard(q: &str) -> Option<String> {
    let bytes = q.as_bytes();
    let mut search = 0;
    while let Some(rel) = q[search..].find("==>") {
        let arrow = search + rel;
        let mut after = Cursor::at(bytes, arrow + 3);
        after.skip_ws();
        let alt = (bytes.get(after.i) == Some(&b'&') && bytes.get(after.i + 1) == Some(&b'('))
            || bytes[after.i.min(bytes.len())..].starts_with(b"bignum_from_memory");
        if alt {
            if let Some(lp) = q[..arrow].rfind('(') {
                let content = &q[lp + 1..arrow];
                if !content.contains('(') && !content.contains(')') {
                    let trimmed = content.trim_end();
                    if contains_p_identifier(trimmed) {
                        return Some(trimmed.to_string());
                    }
                }
            }
        }
        search = arrow + 3;
    }
    None
}

fn parse_conclusion(q: &str, result_ptr: &str) -> std::result::Result<Conclusion, String> {
    if let Some(concl) = find_nat(q, result_ptr) {
        return Ok(concl);
    }
    if let Some(concl) = find_int_rem(q, result_ptr) {
        return Ok(concl);
    }
    Err("unrecognised postcondition equality shape".to_string())
}

fn parse_anchor_tail(c: &mut Cursor<'_>, result_ptr: &str) -> Option<()> {
    // `\(\s*<ptr>\s*,\s*\d+\s*\)\s*s`
    c.skip_ws();
    c.eat(b'(')?;
    c.skip_ws();
    c.eat_lit(result_ptr)?;
    c.skip_ws();
    c.eat(b',')?;
    c.skip_ws();
    c.digits()?;
    c.skip_ws();
    c.eat(b')')?;
    c.skip_ws();
    c.eat(b's')?;
    Some(())
}

fn find_nat(q: &str, result_ptr: &str) -> Option<Conclusion> {
    let s = q.as_bytes();
    let mut search = 0;
    while let Some(at) = find_from(s, b"bignum_from_memory", search) {
        if let Some(concl) = parse_nat_at(s, at, result_ptr) {
            return Some(concl);
        }
        search = at + 1;
    }
    None
}

fn parse_nat_at(s: &[u8], start: usize, result_ptr: &str) -> Option<Conclusion> {
    let mut c = Cursor::at(s, start + "bignum_from_memory".len());
    parse_anchor_tail(&mut c, result_ptr)?;
    c.skip_ws();
    c.eat(b'=')?;
    c.skip_ws();
    c.eat(b'(')?;
    let inner_start = c.i;
    let mut j = c.i;
    while j < s.len() {
        if s[j] == b')' {
            let mut m = Cursor::at(s, j + 1);
            m.skip_ws();
            if m.eat_lit("MOD").is_some() {
                m.skip_ws();
                if let Some(modulus) = m.word() {
                    let inner = std::str::from_utf8(&s[inner_start..j]).ok()?.trim();
                    let parsed = classify_nat_inner(inner, modulus)?;
                    return Some(parsed);
                }
            }
        }
        j += 1;
    }
    None
}

fn classify_nat_inner(inner: &str, modulus: String) -> Option<Conclusion> {
    let s = inner.as_bytes();
    let mut c = Cursor::at(s, 0);
    let first = c.word()?;
    c.skip_ws();
    if c.eat(b'-').is_some() {
        c.skip_ws();
        let second = c.word()?;
        if c.i != s.len() {
            return None;
        }
        return Some(Conclusion::NatModSelf {
            minuend: first,
            subtrahend: second,
            modulus,
        });
    }
    if c.eat(b'+').is_some() {
        c.skip_ws();
        let second = c.word()?;
        if c.i != s.len() {
            return None;
        }
        return Some(Conclusion::NatMod {
            left: first,
            right: second,
            sum: inner.to_string(),
            modulus,
        });
    }
    None
}

fn prefix_is_amp_paren(s: &[u8], start: usize) -> bool {
    // `&\s*\(\s*` immediately before the anchor.
    let mut i = start;
    while i > 0 && s[i - 1].is_ascii_whitespace() {
        i -= 1;
    }
    if i == 0 || s[i - 1] != b'(' {
        return false;
    }
    i -= 1;
    while i > 0 && s[i - 1].is_ascii_whitespace() {
        i -= 1;
    }
    i > 0 && s[i - 1] == b'&'
}

fn find_int_rem(q: &str, result_ptr: &str) -> Option<Conclusion> {
    let s = q.as_bytes();
    let mut search = 0;
    while let Some(at) = find_from(s, b"bignum_from_memory", search) {
        if prefix_is_amp_paren(s, at) {
            if let Some(concl) = parse_int_rem_at(s, at, result_ptr) {
                return Some(concl);
            }
        }
        search = at + 1;
    }
    None
}

fn parse_int_rem_at(s: &[u8], start: usize, result_ptr: &str) -> Option<Conclusion> {
    let mut c = Cursor::at(s, start + "bignum_from_memory".len());
    parse_anchor_tail(&mut c, result_ptr)?;
    c.skip_ws();
    c.eat(b')')?;
    c.skip_ws();
    c.eat(b'=')?;
    c.skip_ws();
    c.eat(b'(')?;
    c.skip_ws();
    c.eat(b'&')?;
    c.skip_ws();
    let minuend = c.word()?;
    c.skip_ws();
    c.eat(b'-')?;
    c.skip_ws();
    c.eat(b'&')?;
    c.skip_ws();
    let subtrahend = c.word()?;
    c.skip_ws();
    c.eat(b')')?;
    c.skip_ws();
    c.eat_lit("rem")?;
    c.skip_ws();
    c.eat(b'&')?;
    c.skip_ws();
    let modulus = c.word()?;
    Some(Conclusion::IntRem {
        minuend,
        subtrahend,
        modulus,
    })
}

// ---------------------------------------------------------------------------
// Tactics
// ---------------------------------------------------------------------------

/// The `MAP_EVERY X_GEN_TAC ... BIGNUM_DIGITIZE_TAC` block shared by both arches.
pub fn preamble(spec: &Spec) -> String {
    let types: Vec<String> = spec
        .variables
        .iter()
        .map(|v| {
            let ty = spec.var_types.get(v).map(String::as_str).unwrap_or("num");
            format!("`{v}:{ty}`")
        })
        .collect();
    let bytes_per = spec.bits / spec.digits.max(1) / 8;
    let shape = format!("{bytes_per} * {}", spec.digits);
    let mut lines = vec![
        format!("MAP_EVERY X_GEN_TAC [{}]", types.join("; ")),
        "REWRITE_TAC[C_ARGUMENTS; C_RETURN; SOME_FLAGS; NONOVERLAPPING_CLAUSES]".to_string(),
        "DISCH_THEN(REPEAT_TCL CONJUNCTS_THEN ASSUME_TAC)".to_string(),
        "REWRITE_TAC[BIGNUM_FROM_MEMORY_BYTES] THEN ENSURES_INIT_TAC \"s0\"".to_string(),
    ];
    for inp in &spec.inputs {
        lines.push(format!(
            "BIGNUM_DIGITIZE_TAC \"{}\" `read (memory :> bytes ({},{shape})) s0`",
            inp.digit_prefix(),
            inp.pointer
        ));
    }
    lines.join(" THEN\n")
}

/// The close text for one operation.
pub fn post_text(op: &str, state: &str, modulus: &str) -> String {
    post_text_for(None, false, op, state, modulus)
}

/// IX.70: the algebraic close (recovered from the IX.67 WIP) is **opt-in**, not
/// a default gate.  The WIP hard-wired one task to `BRIDGE_ADD_REF_ALG_TAC` at
/// emission; measured, that close does not finish (the residual is a linear
/// integer system HOL Light's real-linear `INT_ARITH` cannot discharge -- see
/// `research/receipts/ix70-algebraic-close.md`), so hard-wiring it would make
/// the theorem unprovable in the shipped tree.  Setting
/// `TEKMERION_ALG_CLOSE=<task id>` selects the entry point for that one task and
/// is visible in the emitted answer; unset, every task emits exactly the tactic
/// it had and its inference count is unchanged.
///
/// IX.82 added the weighted-sum aggregate for `arm.bignum_add_p25519`.
/// IX.83 generalises it to subtraction and negation.  Every task listed here
/// has a measured, solver-free close in `research/receipts/ix83-generality.md`;
/// a task not listed emits its SAT entry point even with the gate set.
///
/// `x86.bignum_add_p25519`, `arm.bignum_add_p256` and `arm.bignum_add_p384`
/// have the same generic entry point (the `ref_entry` mapping does not name a
/// task), but they are **not** listed: the x86 add drives the linear prover
/// past the 12 GB cap, p256's accumulator presentation emits no aggregate, and
/// p384 was not attempted.  Shipping them would make the gated answer
/// unprovable, so the opt-in set stays limited to routes that close.
pub const ALGEBRAIC_CLOSE_TASKS: &[&str] = &[
    "arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT",
    "arm.bignum_sub_p25519.BIGNUM_SUB_P25519_CORRECT",
    "arm.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT",
];

/// The entry point a task's reference close emits.  `op` selects the family of
/// instructions (and therefore the reduction lemma); the gate selects the
/// finish.  Pure, so the mapping is unit-testable without the environment.
fn ref_entry(task: Option<&str>, enabled: bool, op: &str) -> &'static str {
    let gated = matches!(task, Some(t) if enabled && ALGEBRAIC_CLOSE_TASKS.contains(&t));
    match (op, gated) {
        ("add", true) => "BRIDGE_ADD_REF_ALG_TAC",
        ("sub", true) => "BRIDGE_SUB_REF_ALG_TAC",
        ("neg", true) => "BRIDGE_NEG_REF_ALG_TAC",
        ("add", false) => "BRIDGE_ADD_REF_TAC",
        ("sub", false) => "BRIDGE_SUB_REF_TAC",
        ("neg", false) => "BRIDGE_NEG_REF_TAC",
        _ => "BRIDGE_ADD_REF_TAC",
    }
}

/// `post_text` for a named task: the only difference is which entry point
/// the emitted chain calls (see `ref_entry`).
pub fn post_text_for(
    task: Option<&str>,
    enabled: bool,
    op: &str,
    state: &str,
    modulus: &str,
) -> String {
    match op {
        "add" => format!(
            "ENSURES_FINAL_STATE_TAC THEN ASM_REWRITE_TAC[] THEN STRIP_TAC THEN\n\
             RULE_ASSUM_TAC(REWRITE_RULE [REAL_BITVAL_NOT; ADD_CLAUSES; VAL_WORD_BITVAL]) THEN\n\
             CONV_TAC SYM_CONV THEN CONV_TAC(RAND_CONV BIGNUM_LEXPAND_CONV) THEN\n\
             ASM_REWRITE_TAC[] THEN DISCARD_STATE_TAC \"{state}\" THEN\n\
             {} (CONV_TAC (ONCE_DEPTH_CONV BIGNUM_OF_WORDLIST_CONV)) {modulus}",
            ref_entry(task, enabled, "add")
        ),
        "sub" => format!(
            "ENSURES_FINAL_STATE_TAC THEN ASM_REWRITE_TAC[] THEN STRIP_TAC THEN\n\
             CONV_TAC(LAND_CONV(RAND_CONV BIGNUM_EXPAND_CONV)) THEN\n\
             ASM_REWRITE_TAC[] THEN DISCARD_STATE_TAC \"{state}\" THEN\n\
             {} (CONV_TAC (ONCE_DEPTH_CONV BIGNUM_OF_WORDLIST_CONV)) {modulus}",
            ref_entry(task, enabled, "sub")
        ),
        "neg" => format!(
            "ENSURES_FINAL_STATE_TAC THEN ASM_REWRITE_TAC[] THEN STRIP_TAC THEN\n\
             CONV_TAC(LAND_CONV BIGNUM_EXPAND_CONV) THEN\n\
             ASM_REWRITE_TAC[] THEN DISCARD_STATE_TAC \"{state}\" THEN\n\
             {} (CONV_TAC (ONCE_DEPTH_CONV BIGNUM_OF_WORDLIST_CONV)) {modulus}",
            ref_entry(task, enabled, "neg")
        ),
        other => format!("{other}: unknown operation"),
    }
}

/// `TEKMERION_SIM_BACKEND` selects the ARM simulation backend the emitted answer
/// calls (XXXI.B3).  The default, `tekmerion`, calls the Tekmerion-owned
/// `TEKMERION_ARM_ACCSTEPS_TAC`; `upstream` calls the shared s2n-bignum
/// `ARM_ACCSTEPS_TAC`.  The two answers differ only by that call name (and by
/// whether the Tekmerion tactic definition is inlined), which is what the
/// three-baseline procedure (`XXXI.F1`) requires without editing fixtures.
pub const SIM_BACKEND_ENV: &str = "TEKMERION_SIM_BACKEND";

/// Which ARM simulation backend an emitted answer uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimBackend {
    /// The Tekmerion-owned re-implementation (`TEKMERION_ARM_ACCSTEPS_TAC`).
    Tekmerion,
    /// The shared s2n-bignum framework (`ARM_ACCSTEPS_TAC`).
    Upstream,
}

impl SimBackend {
    /// The spelling accepted by [`SIM_BACKEND_ENV`].
    pub fn name(self) -> &'static str {
        match self {
            SimBackend::Tekmerion => "tekmerion",
            SimBackend::Upstream => "upstream",
        }
    }

    /// The tactic name the emitted answer calls.
    fn accsteps(self) -> &'static str {
        match self {
            SimBackend::Tekmerion => TEKMERION_ARM_SIMULATION,
            SimBackend::Upstream => "ARM_ACCSTEPS_TAC",
        }
    }
}

/// Resolve [`SIM_BACKEND_ENV`].  An unset or empty variable is the default
/// (`tekmerion`); an unrecognised value is an error, never a silent fallback.
fn simulation_backend() -> Result<SimBackend> {
    match std::env::var(SIM_BACKEND_ENV) {
        Err(_) => Ok(SimBackend::Tekmerion),
        Ok(v) if v.is_empty() || v == "tekmerion" => Ok(SimBackend::Tekmerion),
        Ok(v) if v == "upstream" => Ok(SimBackend::Upstream),
        Ok(v) => other(format!(
            "{SIM_BACKEND_ENV} must be \"tekmerion\" or \"upstream\", got {v:?}"
        )),
    }
}

fn arm_simulation(
    backend: SimBackend,
    exec_name: &str,
    accumulate: &[usize],
    last: usize,
) -> String {
    if accumulate.is_empty() {
        format!("ARM_STEPS_TAC {exec_name} (1--{last})")
    } else {
        let nums: Vec<String> = accumulate.iter().map(|n| n.to_string()).collect();
        format!(
            "{} {exec_name} [{}] (1--{last})",
            backend.accsteps(),
            nums.join(";")
        )
    }
}

fn x86_simulation(exec_name: &str, accumulate: &[usize], last: usize) -> String {
    if accumulate.is_empty() {
        format!("X86_STEPS_TAC {exec_name} (1--{last})")
    } else {
        let nums: Vec<String> = accumulate.iter().map(|n| n.to_string()).collect();
        format!(
            "X86_ACCSTEPS_TAC {exec_name} [{}] (1--{last})",
            nums.join(";")
        )
    }
}

const NEG_CF_REWRITE: &str = "RULE_ASSUM_TAC(REWRITE_RULE[WORD_NEG_EQ_0; WORD_BITVAL_EQ_0])";

/// The x86 close: the NEG-derived CF is `~(x = 0)`, so the two eq-0 facts are
/// rewritten before the bridge and a NEG routine uses the ZERO-test variant.
pub fn post_simulation_x86(
    task: Option<&str>,
    enabled: bool,
    op: &str,
    state: &str,
    modulus: &str,
    uses_neg: bool,
) -> String {
    let mut text = post_text_for(task, enabled, op, state, modulus);
    if !uses_neg {
        return text;
    }
    if op == "neg" {
        text = text.replace("BRIDGE_NEG_REF_TAC", "BRIDGE_NEG_REF_ZERO_TAC");
    }
    let mut lines: Vec<String> = text.split('\n').map(str::to_string).collect();
    let insert_at = lines.len() - 1;
    lines.insert(insert_at, format!("{NEG_CF_REWRITE} THEN"));
    lines.join("\n")
}

// ---------------------------------------------------------------------------
// Phase markers (IX.65; measurement mode only)
// ---------------------------------------------------------------------------
//
// `--mode phase` is the normal answer with `csat_mark` boundaries inserted
// between the emitter's own blocks.  The block boundaries are the lines of the
// emitted chain (the emitter writes them with ` THEN\n`), plus the
// `ENSURES_INIT_TAC` that shares a line with the memory rewrite.  Nothing here
// is on the `--mode answer` path, and `strip_phase_marks` restores the normal
// tactic byte-for-byte (unit-tested), so a measurement run cannot change what
// is proved or how.

/// The class a block is charged to.  `Accumulator` is deliberately absent: the
/// only accumulator reasoning in the emitted chain is inside upstream's single
/// `*_ACCSTEPS_TAC`, which cannot be split without editing upstream's tactic, so
/// it is reported with `Play` and the limit is stated in the receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseClass {
    Play,
    Rewrite,
    Finalisation,
    Bridge,
}

impl PhaseClass {
    pub fn as_str(self) -> &'static str {
        match self {
            PhaseClass::Play => "play",
            PhaseClass::Rewrite => "rewrite",
            PhaseClass::Finalisation => "finalisation",
            PhaseClass::Bridge => "bridge",
        }
    }
}

/// Classification is by the tactic name on the emitted line.  It is mechanical
/// and total: every line matches exactly one arm.
pub fn phase_class(line: &str) -> PhaseClass {
    if line.contains("ACCSTEPS_TAC") || line.contains("STEPS_TAC") {
        PhaseClass::Play
    } else if line.contains("BRIDGE_") {
        PhaseClass::Bridge
    } else if line.contains("ENSURES_INIT_TAC")
        || line.contains("ENSURES_FINAL_STATE_TAC")
        || line.contains("DISCARD_STATE_TAC")
    {
        PhaseClass::Finalisation
    } else {
        PhaseClass::Rewrite
    }
}

const PHASE_PREFIX: &str = "(csat_mark_reader := inference_count;\ncsat_mark_enabled := true;\n";
const INIT_MARK: &str = "csat_mark \"init finalisation\" THEN ";

/// The measurement prefix, including the task id so that the library's
/// `SLICE ...` lines (XV.B) name the task they belong to. The id is emitted as
/// an OCaml string literal; task ids are ASCII, so `{:?}` is exact.
fn phase_prefix(task: &str) -> String {
    format!("{PHASE_PREFIX}bridge_slice_task := {task:?};\n")
}

/// The measurement answer's tactic.  Labels are `<line> <class>`.  There is no
/// trailing marker: the last block is closed by `BRIDGE_*_TAC`, which solves the
/// goal, so a tactic after it would never run.  The receipt reconciles the last
/// block as `STAMP_INF_AFTER - last_mark_count`.
pub fn phase_tactic(tactic: &str, task: &str) -> String {
    let mut out = String::with_capacity(tactic.len() + 4096);
    out.push_str(&phase_prefix(task));
    let lines: Vec<&str> = tactic.split('\n').collect();
    for (i, line) in lines.iter().enumerate() {
        out.push_str(&format!(
            "csat_mark \"{:02} {}\" THEN ",
            i,
            phase_class(line).as_str()
        ));
        if let Some(pos) = line.find(" THEN ENSURES_INIT_TAC") {
            let split = pos + " THEN ".len();
            out.push_str(&line[..split]);
            out.push_str(INIT_MARK);
            out.push_str(&line[split..]);
        } else {
            out.push_str(line);
        }
        if i + 1 < lines.len() {
            out.push('\n');
        }
    }
    out.push(')');
    out
}

/// Remove every phase marker, restoring the normal tactic.  Refuses (returns
/// `None`) when the text does not have the shape `phase_tactic` emits, so a
/// silent partial strip is impossible.
pub fn strip_phase_marks(phase: &str) -> Option<String> {
    let body = phase.strip_prefix(PHASE_PREFIX)?;
    let body = body.strip_prefix("bridge_slice_task := ")?;
    let body = body.split_once(";\n")?.1;
    let body = body.strip_suffix(')')?;
    let mut out = Vec::new();
    for line in body.split('\n') {
        let line = strip_one_mark(line)?;
        out.push(line.replace(INIT_MARK, ""));
    }
    Some(out.join("\n"))
}

fn strip_one_mark(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("csat_mark \"")?;
    let end = rest.find("\" THEN ")?;
    Some(&rest[end + "\" THEN ".len()..])
}

// ---------------------------------------------------------------------------
// IX.66: bridge-block measurement instrumentation (phase mode only)
// ---------------------------------------------------------------------------
//
// IX.65 bracketed the block only at the emitter's own lines, which left
// `BRIDGE_*_REF_TAC` as one indivisible 67.85% of the measured work.  The
// boundaries inside it are inside library functions and cannot be bracketed
// from the call site, so the functions themselves are instrumented -- but only
// in the *copy* that `--mode phase` inlines.  `--mode answer` still inlines the
// tracked files verbatim, so a normal answer is byte-identical to IX.65's.
//
// Each edit is a `(plain, marked)` pair.  `instrument` requires the plain text
// to occur exactly once (so an anchor that drifts fails loudly rather than
// silently measuring nothing), and `deinstrument` reverses the edits in reverse
// order.  `ix66_markers_are_additive_only` proves the round trip on both files:
// the phase copy differs from the tracked file by the marker phrases alone.

struct MarkerEdit {
    plain: &'static str,
    marked: &'static str,
}

/// The four per-phase boundaries in `CSAT_PROVE` / `CSAT_REFUTE_GEN`:
/// `bridge:prove-entry` is where `BITBLAST_THEN` hands over (so
/// `pre-bitblast -> prove-entry` is the bit-blasting), `bridge:cnf-built` is
/// where the clausification ends, and `PHASE_CNF` reports the DIMACS size the
/// solver is given.  The replay brackets are IX.65's, unchanged.
const CSAT_LIB_EDITS: &[MarkerEdit] = &[
    MarkerEdit {
        plain: "    ALL_TAC g;;\n",
        marked: r#"    ALL_TAC g;;

(* IX.66 measurement-only boundaries.  Inserted into the --mode phase copy of
   this library; absent from the tracked file, so a normal answer is unchanged. *)
let rec csat_term_size tm =
  (match tm with
   | Comb(s,t) -> 1 + csat_term_size s + csat_term_size t
   | Abs(_,b) -> 1 + csat_term_size b
   | _ -> 1);;
let csat_mark_at label =
  (if !csat_mark_enabled then
     Printf.printf "PHASE_MARK %s %d\n%!" label (!csat_mark_reader ()));;
let csat_mark_size label tm =
  (if !csat_mark_enabled then
     Printf.printf "PHASE_GOAL %s %d\n%!" label (csat_term_size tm));;
let csat_mark_goal (label : string) : tactic =
  fun (asl,w) ->
    (csat_mark_size label w; ALL_TAC (asl,w));;
"#,
    },
    MarkerEdit {
        plain: "  fun tm ->\n    if is_neg tm && is_strictcnf (rand tm) then",
        marked: "  fun tm ->\n    csat_mark_at \"bridge:prove-entry\";\n    csat_mark_size \"bridge:blast\" tm;\n    if is_neg tm && is_strictcnf (rand tm) then",
    },
    MarkerEdit {
        plain: "      let cth = CSAT_REFUTE_GEN (Array.get rar) ntdcnf in",
        marked: "      csat_mark_at \"bridge:cnf-built\";\n      let cth = CSAT_REFUTE_GEN (Array.get rar) ntdcnf in",
    },
    MarkerEdit {
        plain: "  let _ = dimacs_of_hol cnfname tm in",
        marked: "  let _ = dimacs_of_hol cnfname tm in\n  let _ = if !csat_mark_enabled then\n            Printf.printf \"PHASE_CNF %d %d %d\\n%!\"\n              (List.length (atoms tm)) (count_conjuncts 1 tm) (csat_term_size tm) in",
    },
];

/// The boundaries inside `bridge.ml`: the entry to each reference close, the
/// goal handed to a SAT close, and the point where that close's proved theorem
/// comes back.  `bridge:pre-bitblast -> bridge:prove-entry` is `BITBLAST_THEN`'s
/// own rewriting; `bridge:post-replay -> bridge:sat-done` is the assembly after
/// the kernel replay.
const BRIDGE_EDITS: &[MarkerEdit] = &[
    MarkerEdit {
        plain: "let BRIDGE_ADD_REF_TAC expand pth : tactic =\n  ELIM_DIGIT_EQS_TAC THEN BRIDGE_REWRITE_MODULUS pth THEN expand THEN",
        marked: "let BRIDGE_ADD_REF_TAC expand pth : tactic =\n  csat_mark_goal \"bridge:goal\" THEN\n  ELIM_DIGIT_EQS_TAC THEN BRIDGE_REWRITE_MODULUS pth THEN expand THEN",
    },
    MarkerEdit {
        plain: "let bridge_sub_ref_core tag finish expand pth : tactic =\n  ELIM_DIGIT_EQS_TAC THEN BRIDGE_REWRITE_MODULUS pth THEN expand THEN",
        marked: "let bridge_sub_ref_core tag finish expand pth : tactic =\n  csat_mark_goal \"bridge:goal\" THEN\n  ELIM_DIGIT_EQS_TAC THEN BRIDGE_REWRITE_MODULUS pth THEN expand THEN",
    },
    MarkerEdit {
        plain: "let bridge_neg_ref_core tag finish expand pth : tactic =\n  ELIM_DIGIT_EQS_TAC THEN BRIDGE_REWRITE_MODULUS pth THEN expand THEN",
        marked: "let bridge_neg_ref_core tag finish expand pth : tactic =\n  csat_mark_goal \"bridge:goal\" THEN\n  ELIM_DIGIT_EQS_TAC THEN BRIDGE_REWRITE_MODULUS pth THEN expand THEN",
    },
    MarkerEdit {
        plain: "let BRIDGE_NEG_REF_ZERO_TAC expand pth : tactic =\n  ELIM_DIGIT_EQS_TAC THEN BRIDGE_REWRITE_MODULUS pth THEN expand THEN",
        marked: "let BRIDGE_NEG_REF_ZERO_TAC expand pth : tactic =\n  csat_mark_goal \"bridge:goal\" THEN\n  ELIM_DIGIT_EQS_TAC THEN BRIDGE_REWRITE_MODULUS pth THEN expand THEN",
    },
    MarkerEdit {
        plain: "let BRIDGE_ADD_REF_ALG_TAC expand pth : tactic =\n  ELIM_DIGIT_EQS_TAC THEN BRIDGE_REWRITE_MODULUS pth THEN expand THEN",
        marked: "let BRIDGE_ADD_REF_ALG_TAC expand pth : tactic =\n  csat_mark_goal \"bridge:goal\" THEN\n  ELIM_DIGIT_EQS_TAC THEN BRIDGE_REWRITE_MODULUS pth THEN expand THEN",
    },
    MarkerEdit {
        plain: "  fun g ->\n    (at \"canon-zero\" BRIDGE_CANON_ZERO_SELECT_TAC THEN\n     at \"normalise\" BRIDGE_ALG_NORM_TAC THEN",
        marked: "  fun g ->\n    csat_mark_at \"bridge:algebra\";\n    (at \"canon-zero\" BRIDGE_CANON_ZERO_SELECT_TAC THEN\n     at \"normalise\" BRIDGE_ALG_NORM_TAC THEN",
    },
    MarkerEdit {
        plain: "     at \"oracle\" BRIDGE_ORACLE_TAC) g;;",
        marked: "     at \"oracle\" (fun g -> csat_mark_at \"bridge:algebra-arith\"; BRIDGE_ORACLE_TAC g)) g;;",
    },
    MarkerEdit {
        plain: "let BRIDGE_CSAT_TAC : tactic =\n  fun (asl,w) ->\n    let hths = map snd asl in",
        marked: "let BRIDGE_CSAT_TAC : tactic =\n  fun (asl,w) ->\n    csat_mark_at \"bridge:pre-bitblast\";\n    csat_mark_size \"bridge:word-goal\" w;\n    let hths = map snd asl in",
    },
    MarkerEdit {
        plain: "    ACCEPT_TAC th' (asl,w);;\n\n(* everything that precedes the SAT call, on the goal *)",
        marked: "    csat_mark_at \"bridge:sat-done\";\n    ACCEPT_TAC th' (asl,w);;\n\n(* everything that precedes the SAT call, on the goal *)",
    },
    // IX.86: measurement-only boundaries inside the oracle close, so the
    // aggregate route can be split into gauss-jordan, aggregate construction,
    // hypothesis flattening, the aggregate fact proof, close assembly, the
    // linear close itself, and the return to the next branch.  Inert in the
    // normal answer; removed again by the reverse table.
    MarkerEdit {
        plain: "      let aggregates = bridge_aggregates eqs in",
        marked: "      csat_mark_at \"bridge:pre-agg\";\n      let aggregates = bridge_aggregates eqs in\n      csat_mark_at \"bridge:post-agg\";",
    },
    MarkerEdit {
        plain: "      let all_hyps = List.map (fun (_,th) -> bridge_flat_th th) (bridge_int_hyps asl) in",
        marked: "      let all_hyps = List.map (fun (_,th) -> bridge_flat_th th) (bridge_int_hyps asl) in\n      csat_mark_at \"bridge:post-flat\";",
    },
    MarkerEdit {
        plain: "      let agg_facts = bridge_aggregate_facts aggregates all_hyps in",
        marked: "      let agg_facts = bridge_aggregate_facts aggregates all_hyps in\n      csat_mark_at \"bridge:post-facts\";",
    },
    MarkerEdit {
        plain: "  let side_hyps =\n    List.filter\n      (fun th -> conjunctive th &&\n                 (not (is_int_eq th) || not (bridge_int_eq_has_carry th)))\n      all_hyps in",
        marked: "  let side_hyps =\n    List.filter\n      (fun th -> conjunctive th &&\n                 (not (is_int_eq th) || not (bridge_int_eq_has_carry th)))\n      all_hyps in\n  (if !csat_mark_enabled then\n     Printf.printf \"FACT_PREM hyps=%d size=%d aggs=%d\\n%!\"\n       (List.length side_hyps)\n       (List.fold_left (fun a th -> a + bridge_term_size (concl th)) 0 side_hyps)\n       (List.length aggregates));",
    },
    // XXVI.C7 / IX.92 §3: the phase output must bracket the *whole* close, not
    // only the full-set fallback.  `bridge:close-enter` is emitted once, right
    // after the close set is fixed and before the propositional attempt, and
    // `bridge:close-exit` once on every branch that returns a closed theorem
    // (prop, range, slim, full-set).  The legacy `bridge:pre-close` /
    // `bridge:post-close` marks stay: `instr_gen.pl` anchors the per-tactic
    // `G:INT_ARITH:close-*` markers on the `bridge:pre-close` line and the
    // full-set `prove` line, so those two must keep their exact text.
    MarkerEdit {
        plain: "      bridge_slice_note \"ORACLE close ants=%d size=%d\"\n        (List.length ants)\n        (List.fold_left (fun a th -> a + bridge_term_size (concl th)) 0 ants);",
        marked: "      bridge_slice_note \"ORACLE close ants=%d size=%d\"\n        (List.length ants)\n        (List.fold_left (fun a th -> a + bridge_term_size (concl th)) 0 ants);\n      csat_mark_at \"bridge:close-enter\";",
    },
    MarkerEdit {
        plain: "          bridge_slice_note \"ORACLE close prop-refutation ants=%d\"\n            (List.length ants);\n          ACCEPT_TAC closed (asl,w)",
        marked: "          bridge_slice_note \"ORACLE close prop-refutation ants=%d\"\n            (List.length ants);\n          csat_mark_at \"bridge:close-exit\";\n          ACCEPT_TAC closed (asl,w)",
    },
    MarkerEdit {
        plain: "          bridge_slice_note \"ORACLE close range-refutation ants=%d\"\n            (List.length ants);\n          ACCEPT_TAC closed (asl,w)",
        marked: "          bridge_slice_note \"ORACLE close range-refutation ants=%d\"\n            (List.length ants);\n          csat_mark_at \"bridge:close-exit\";\n          ACCEPT_TAC closed (asl,w)",
    },
    MarkerEdit {
        plain: "      match slim_closed with\n      | Some closed -> ACCEPT_TAC closed (asl,w)",
        marked: "      match slim_closed with\n      | Some closed ->\n          csat_mark_at \"bridge:close-exit\";\n          ACCEPT_TAC closed (asl,w)",
    },
    MarkerEdit {
        plain: "      let th = prove(bridge_mk_imps ants w', INT_ARITH_TAC) in",
        marked: "      csat_mark_at \"bridge:pre-close\";\n      let th = prove(bridge_mk_imps ants w', INT_ARITH_TAC) in",
    },
    MarkerEdit {
        plain: "      bridge_slice_note \"ORACLE close ok\";\n      ACCEPT_TAC closed (asl,w)",
        marked: "      csat_mark_at \"bridge:post-close\";\n      bridge_slice_note \"ORACLE close ok\";\n      csat_mark_at \"bridge:close-exit\";\n      ACCEPT_TAC closed (asl,w)",
    },
];

fn apply_marker_edits(
    src: &str,
    edits: &[MarkerEdit],
    which: &str,
    forward: bool,
) -> Result<String> {
    let mut out = src.to_string();
    let ordered: Vec<&MarkerEdit> = if forward {
        edits.iter().collect()
    } else {
        edits.iter().rev().collect()
    };
    for e in ordered {
        let (from, to) = if forward {
            (e.plain, e.marked)
        } else {
            (e.marked, e.plain)
        };
        let n = out.matches(from).count();
        if n != 1 {
            return other(format!(
                "{which}: IX.66 marker anchor occurs {n} times, expected 1: {from:?}"
            ));
        }
        out = out.replacen(from, to, 1);
    }
    Ok(out)
}

/// The phase-mode copy of `tools/csat_lib.ml`.
pub fn instrument_csat_lib_markers(src: &str) -> Result<String> {
    apply_marker_edits(src, CSAT_LIB_EDITS, "tools/csat_lib.ml", true)
}

/// The phase-mode copy of `tools/bridge.ml`.
pub fn instrument_bridge_markers(src: &str) -> Result<String> {
    apply_marker_edits(src, BRIDGE_EDITS, "tools/bridge.ml", true)
}

/// Remove the IX.66 marker phrases again (the proof that nothing else changed).
pub fn deinstrument_csat_lib(src: &str) -> Result<String> {
    apply_marker_edits(src, CSAT_LIB_EDITS, "tools/csat_lib.ml", false)
}

pub fn deinstrument_bridge(src: &str) -> Result<String> {
    apply_marker_edits(src, BRIDGE_EDITS, "tools/bridge.ml", false)
}

// ---------------------------------------------------------------------------
// Answer assembly (the judged artefact)
// ---------------------------------------------------------------------------

fn is_skipped(chunk: &str) -> bool {
    if chunk.starts_with('#') {
        return true;
    }
    for word in ["needs", "loadt"] {
        if let Some(rest) = chunk.strip_prefix(word) {
            match rest.chars().next() {
                None => return true,
                Some(c) if !(c.is_alphanumeric() || c == '_') => return true,
                _ => {}
            }
        }
    }
    false
}

/// Split an OCaml script on `;;[ \t]*(?:\n|$)`, as the Python tool did.
fn split_chunks(src: &str) -> Vec<&str> {
    let bytes = src.as_bytes();
    let mut out = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b';' && bytes[i + 1] == b';' {
            let mut j = i + 2;
            while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t') {
                j += 1;
            }
            if j == bytes.len() {
                out.push(&src[start..i]);
                start = bytes.len();
                break;
            }
            if bytes[j] == b'\n' {
                out.push(&src[start..i]);
                start = j + 1;
                i = start;
                continue;
            }
            i += 2;
            continue;
        }
        i += 1;
    }
    if start <= src.len() {
        out.push(&src[start..]);
    }
    out
}

fn phrases_from(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    for chunk in split_chunks(src) {
        let c = chunk.trim();
        if c.is_empty() || is_skipped(c) {
            continue;
        }
        out.push(c.to_string());
    }
    out
}

pub const ANSWER_SOURCES: &[&str] = &[
    ".bench/hol-light/Cadical/dimacs.ml",
    ".bench/hol-light/Cadical/cnf.ml",
    ".bench/hol-light/Cadical/ldrup.ml",
    "tools/csat_lib.ml",
    "tools/bridge.ml",
];

/// The name of the ARM simulation tactic the generated proofs call (XXX.G3).
/// It is Tekmerion-owned: the shared s2n-bignum framework is untouched, and
/// nothing but an emitted answer refers to this name.
pub const TEKMERION_ARM_SIMULATION: &str = "TEKMERION_ARM_ACCSTEPS_TAC";

/// The Tekmerion-owned ARM simulation tactic, inlined *only* into generated
/// ARM answers (the emitter selects it in [`arm_simulation`]).  It is a copy of
/// `arm/proofs/arm.ml:ARM_STEP_TAC` and the tactics above it, with two
/// substitutions.
///
/// First, `ASSUMPTION_STATE_UPDATE_TAC` becomes
/// `TEKMERION_ASSUMPTION_STATE_UPDATE_TAC`, which skips (does not attempt to
/// carry) a hypothesis about a component the step itself overwrites. The
/// shipped tactic only ever fails on such a hypothesis and drops it; the
/// `STATE_UPDATE_NEW_RULE` fact for the same component supersedes it. Every
/// other hypothesis is still carried by the unchanged `STATE_UPDATE_RULE`, so
/// the resulting assumption set is identical — only the wasted failing
/// attempts are removed. Rationale and measurement:
/// `research/receipts/xxx-state-propagation-feasibility.md` §3.3.
///
/// Second, `MAYCHANGE_STATE_UPDATE_TAC` becomes
/// `TEKMERION_MAYCHANGE_STATE_UPDATE_TAC`, the one-shot frame update of
/// XXX.G6 (`research/receipts/xxx-state-propagation-feasibility.md` §3.2,
/// §4.1): the step's write set is read out of band, the frame is canonicalised
/// once per step instead of once per write, membership is decided by comparing
/// components instead of by commuting, and the frame is returned to the
/// shipped `MAYCHANGE` form at the end of the step. It produces the same frame
/// term after every step.
///
/// It adds no step, rule or axiom: it derives the same theorems through fewer
/// kernel calls.
pub const TEKMERION_ARM_TACTIC: &str = r#"(* ========================================================================= *)
(* TEKMERION-OWNED ARM simulation tactic (XXX.G3).                           *)
(*                                                                           *)
(* NOT part of the shared s2n-bignum framework.  Inlined only into generated *)
(* Tekmerion answers; the upstream `ARM_ACCSTEPS_TAC`, `ARM_STEP_TAC` and    *)
(* `ASSUMPTION_STATE_UPDATE_TAC` are untouched.  The step body is a copy of  *)
(* `arm/proofs/arm.ml:ARM_STEP_TAC` with one substitution: the assumption    *)
(* update skips a hypothesis about a component this step overwrites.         *)
(* Such a hypothesis cannot be carried (the shipped tactic only ever fails   *)
(* on it and drops it) and the STATE_UPDATE_NEW_RULE fact for the same       *)
(* component supersedes it, so the resulting assumption set is identical.    *)
(* See research/receipts/xxx-state-propagation-feasibility.md section 3.3.   *)
(* ========================================================================= *)

(* `TRY` under another name.  The offline OCaml parse gate lower-cases HOL   *)
(* value names but must keep `TRY` (its lower-case spelling is a keyword); an *)
(* upper-case value then cannot be an operand of `o` in stock OCaml syntax.  *)
(* This alias has the same value and a name the gate can lower, so the        *)
(* framework expression below is copied verbatim from arm.ml.                *)
let TEKMERION_TRY = TRY;;

(* The write spine of the step: `write c1 y1 (... (write cn yn s))`. *)
let TEKMERION_WRITTEN_COMPONENTS uth =
  let rec go acc tm =
    match tm with
      Comb(Comb(Comb(Const("write",_),c),_),t) -> go (c::acc) t
    | _ -> acc in
  go [] (lhand(concl uth));;

(* The components `c` with a subterm `read c s` in `tm`. *)
let TEKMERION_READS_OF_STATE s tm =
  let rec go acc tm =
    match tm with
      Comb(a,b) ->
        let acc' =
          (match tm with
             Comb(Comb(Const("read",_),c),st) when aconv st s -> c::acc
           | _ -> acc) in
        go (go acc' a) b
    | Abs(_,b) -> go acc b
    | _ -> acc in
  go [] tm;;

let TEKMERION_SUPERSEDED uth s thi =
  let ws = TEKMERION_WRITTEN_COMPONENTS uth in
  let rs = TEKMERION_READS_OF_STATE s (concl thi) in
  exists (fun r -> exists (aconv r) ws) rs;;

(* components.ml:ASSUMPTION_STATE_UPDATE_TAC plus the supersede filter. *)
let TEKMERION_ASSUMPTION_STATE_UPDATE_TAC =
  let rec memorable tm =
    match tm with
      Var(_,_) -> false
    | Abs(_,b) -> memorable b
    | Const(n,_) -> n = "memory"
    | Comb(s,t) -> memorable s || memorable t in
  fun gl -> (DISCH_THEN(fun uth ->
   let utm = lhand(concl uth)
   and thl = map snd (fst gl) in
   let s = repeat rand utm in
   let keep thi = not (TEKMERION_SUPERSEDED uth s thi) in
   let newthms =
     if memorable utm then
       let thy,thn = partition (vfree_in s o concl) thl in
       let cxt = (NONOVERLAPPING_DRIVERS thn,FILTER_CANONIZE_ASSUMPTIONS thn) in
       mapfilter (fun thi -> STATE_UPDATE_RULE cxt uth thi) (filter keep thy)
     else
       let thy = filter (fun thi -> vfree_in s (concl thi) && keep thi) thl in
       mapfilter (fun thi -> STATE_UPDATE_RULE ([],[]) uth thi) thy in
   MP_TAC uth THEN MAP_EVERY ASSUME_TAC newthms)) gl;;

(* ------------------------------------------------------------------------- *)
(* One-shot MAYCHANGE frame update (XXX.G6).                                  *)
(*                                                                           *)
(* The shipped MAYCHANGE_STATE_UPDATE_TAC extends the frame one write at a   *)
(* time and, for every write, canonicalises the whole accumulated prefix and  *)
(* then commutes the new component back into it.  Measured on the IX.103     *)
(* `neg` answer that is 196,493 inferences for 61 writes, of which ~76,000    *)
(* are spent commuting a component that is not in the frame at all: the       *)
(* commutation is thrown away and the component appended.                    *)
(*                                                                           *)
(* This rule instead reads the step's write spine as a list first (out of     *)
(* band), keeps the frame in canonical ASSIGNS form for the whole step (one   *)
(* canonicalisation of the incoming frame, one of each new entry), decides    *)
(* membership by comparing components instead of by commuting, and converts   *)
(* back to the shipped MAYCHANGE form once at the end of the step.  A         *)
(* component already in the frame costs one move-back (which the shipped      *)
(* rule also pays); a component that is not in it costs only the append.      *)
(* Every step is a kernel proof built from the framework's own rules; the     *)
(* frame term after every step is the term the shipped rule produces.  See   *)
(* research/receipts/xxx-state-propagation-feasibility.md section 3.2.        *)
(* ------------------------------------------------------------------------- *)

(* Components of a frame term, in chain order.  A node is `MAYCHANGE [c]`,   *)
(* `ASSIGNS c`, or the identity `(=)` (the canonical form of MAYCHANGE []).   *)
let TEKMERION_NODE_COMPONENT tm =
  match tm with
    Comb(Const("ASSIGNS",_),c) -> Some c
  | Comb(Const("MAYCHANGE",_),lst) ->
      (match lst with
         Comb(Comb(Const("CONS",_),c),_) -> Some c
       | _ -> None)
  | Const("=",_) -> None
  | _ -> failwith "TEKMERION_NODE_COMPONENT";;

(* The chain as a list of nodes, for either association. *)
let rec TEKMERION_FRAME_NODES tm =
  match tm with
    Comb(Comb(Const(",,",_),l),r) ->
      TEKMERION_FRAME_NODES l @ TEKMERION_FRAME_NODES r
  | _ -> [tm];;

let TEKMERION_FRAME_COMPONENTS tm =
  let rec go acc = function
    | [] -> rev acc
    | n::rest ->
        (match TEKMERION_NODE_COMPONENT n with
           Some c -> go (c::acc) rest
         | None -> go acc rest) in
  go [] (TEKMERION_FRAME_NODES tm);;

(* Component-indexed resolution of the swap lemmas: the framework's own cache *)
(* is keyed on constant components only, so every swap of e.g.                *)
(* `memory :> bignum(z,4)` with a register is re-proved from scratch.  A       *)
(* swap theorem is a function of the pair alone and is proved by the kernel    *)
(* once; here it is also remembered once.                                     *)
let tekmerion_swap_cache = ref ([]: (term * thm) list);;
let TEKMERION_SWAP_CONV tm =
  try assoc tm !tekmerion_swap_cache
  with Failure _ ->
    let th = ASSIGNS_SWAP_CONV tm in
    tekmerion_swap_cache := (tm,th) :: !tekmerion_swap_cache;
    th;;

(* relational.ml:ASSIGNS_MOVE_BACK_CONV with the swap conversion passed in. *)
let rec TEKMERION_MOVE_BACK swcv acv tm =
  try let th0 = GEN_REWRITE_CONV I [GSYM SEQ_ASSOC] tm in
      let ltm,rtm = dest_comb(rand(concl th0)) in
      let (p,th1) = TEKMERION_MOVE_BACK swcv acv rtm in
      let th2 = TRANS th0 (AP_TERM ltm th1) in
      if p <> 0 then (p,th2) else
      let th3 = GEN_REWRITE_RULE RAND_CONV [SEQ_ASSOC] th2 in
      (try 1,CONV_RULE (RAND_CONV (LAND_CONV acv)) th3
       with Failure _ -> try
         let th4 = CONV_RULE (RAND_CONV (LAND_CONV swcv)) th3 in
         0,GEN_REWRITE_RULE RAND_CONV [GSYM SEQ_ASSOC] th4
       with Failure _ -> -1,th2)
  with Failure _ -> try 1,acv tm
  with Failure _ -> try 0,swcv tm
  with Failure _ -> -1,REFL tm;;

(* `(=) ,, r = r`, the rewrite MAYCHANGE_LID performs for the empty starter. *)
let TEKMERION_SEQ_ID_CONV tm =
  try PURE_REWRITE_CONV [SEQ_ID] tm with Failure _ -> REFL tm;;

(* Rebuild the MAYCHANGE-form frame from its node list. *)
let rec TEKMERION_FRAME_CHAIN sq = function
  | [n] -> n
  | n::rest -> mk_comb(mk_comb(sq,n),TEKMERION_FRAME_CHAIN sq rest)
  | [] -> failwith "TEKMERION_FRAME_CHAIN";;

let TEKMERION_MAYCHANGE_UPDATE_RULE uth th =
  let spine = lhand(concl uth) in
  let rec is_write tm =
    match tm with
      Comb(Comb(Comb(Const("write",_),_),_),_) -> true
    | _ -> false in
  if not (is_write spine) then
    (* no write in this step: the frame is carried unchanged *)
    EQ_MP (AP_TERM (rator(concl th)) uth) th
  else
  let frame = rator(rator(concl th)) in
  let nodes0 = List.filter
    (fun n -> TEKMERION_NODE_COMPONENT n <> None)
    (TEKMERION_FRAME_NODES frame) in
  (* canonicalise the incoming frame once, for the whole write list *)
  let lth = MAYCHANGE_CANON_CONV frame in
  let acur0 = rand(concl lth) in
  let comps0 = TEKMERION_FRAME_COMPONENTS frame in
  if not (TEKMERION_FRAME_COMPONENTS acur0 = comps0) then
    failwith "TEKMERION_MAYCHANGE_UPDATE_RULE: frame is not canonicalisable";
  let anodes0 = List.filter
    (fun n -> TEKMERION_NODE_COMPONENT n <> None)
    (TEKMERION_FRAME_NODES acur0) in
  (* thread (theorem, canonical frame, its nodes, the shipped-form nodes, the
     component list, the sequencing relation, the step certificate) *)
  let rec step st tm =
    match tm with
      Comb(Comb(Comb(Const("write",_),c),y),s) ->
        let (th_cur,acur,anodes,nodes,comps,sqo) = step st s in
        let th1 = ISPECL [c;y] (MATCH_MP MAYCHANGE_WRITE th_cur) in
        let pred = rator(rator(concl th1)) in
        let sq = match sqo with Some q -> q | None -> rator(rator pred) in
        let item = rand pred in
        let ithm = MAYCHANGE_CANON_CONV item in
        let ac = rand(concl ithm) in
        let cmp = match ac with
                    Comb(Const("ASSIGNS",_),cc) -> cc
                  | _ -> failwith "TEKMERION_MAYCHANGE_UPDATE_RULE: item" in
        (* (acur ,, MAYCHANGE [c]) = (acur ,, ASSIGNS c) *)
        let pth = MK_COMB(AP_TERM sq (REFL acur),ithm) in
        let pair = mk_comb(mk_comb(sq,acur),ac) in
        (* the same element sequence in canonical (right-associative) form *)
        let appended () =
          let rea =
            GEN_REWRITE_CONV TOP_DEPTH_CONV [GSYM SEQ_ASSOC] pair in
          (TRANS pth rea, rand(concl rea), anodes @ [ac], nodes @ [item],
           comps @ [cmp]) in
        let res =
          if anodes = [] then
            (* first component: the identity is dropped, as MAYCHANGE_LID does *)
            (TRANS pth (TEKMERION_SEQ_ID_CONV pair),
             ac, [ac], [item], [cmp])
          else if exists (fun e -> aconv e cmp) comps then
            (* already allowed: absorb it; the frame term does not change *)
            let n,eth = TEKMERION_MOVE_BACK TEKMERION_SWAP_CONV
                          ASSIGNS_IDEMPOT_CONV pair in
            if n = 1 && aconv (rand(concl eth)) acur then
              (TRANS pth eth, acur, anodes, nodes, comps)
            else appended ()
          else appended () in
        let (eq,acur',anodes',nodes',comps') = res in
        (CONV_RULE(RATOR_CONV(RATOR_CONV (K eq))) th1,
         acur',anodes',nodes',comps',Some sq)
    | _ -> st in
  let inf = (CONV_RULE(RATOR_CONV(RATOR_CONV (K lth))) th,
             acur0,anodes0,nodes0,comps0,None) in
  let (th_cur,acur,_,nodes,_,sqo) = step inf spine in
  let sq = match sqo with
             Some q -> q
           | None -> failwith "TEKMERION_MAYCHANGE_UPDATE_RULE: no write" in
  (* back to the shipped MAYCHANGE form, one conversion for the whole step *)
  let mth = MAYCHANGE_CANON_CONV (TEKMERION_FRAME_CHAIN sq nodes) in
  if not (rand(concl mth) = acur) then
    failwith "TEKMERION_MAYCHANGE_UPDATE_RULE: frame bookkeeping";
  let th' = CONV_RULE(RATOR_CONV(RATOR_CONV (K (SYM mth)))) th_cur in
  EQ_MP (AP_TERM (rator(concl th')) uth) th';;

let TEKMERION_MAYCHANGE_STATE_UPDATE_TAC =
  DISCH_THEN(fun uth ->
    MP_TAC uth THEN
    FIRST_X_ASSUM(ASSUME_TAC o TEKMERION_MAYCHANGE_UPDATE_RULE uth o
           check(maychange_term o concl)));;

(* arm.ml:ARM_STEP_TAC with the substitutions above. *)
let TEKMERION_ARM_STEP_TAC (mc_length_th,decode_ths) (subths:thm list) sname
      (store_inst_term_to: term ref option)
      (strip_component_tac: thm_tactic) =
  ARM_BASIC_STEP_TAC decode_ths sname store_inst_term_to THEN
  NONSELFMODIFYING_STATE_UPDATE_TAC
    (MATCH_MP aligned_bytes_loaded_update mc_length_th) THEN
  MAP_EVERY (TEKMERION_TRY o NONSELFMODIFYING_STATE_UPDATE_TAC o
    MATCH_MP aligned_bytes_loaded_update o CONJUNCT1) subths THEN
  TEKMERION_ASSUMPTION_STATE_UPDATE_TAC THEN
  TEKMERION_MAYCHANGE_STATE_UPDATE_TAC THEN
  DISCH_THEN(fun th ->
    let thl = STATE_UPDATE_NEW_RULE th in
    if thl = [] then ALL_TAC else
    MP_TAC(end_itlist CONJ thl) THEN
    ASSEMBLER_SIMPLIFY_TAC THEN
    strip_component_tac th);;

let TEKMERION_ARM_VERBOSE_STEP_TAC (exth1,exth2) sname g =
  TEKMERION_ARM_STEP_TAC (exth1,exth2) [] sname None (K STRIP_TAC) g;;

let TEKMERION_ARM_SINGLE_STEP_TAC th s =
  time (TEKMERION_ARM_VERBOSE_STEP_TAC th s) THEN
  DISCARD_OLDSTATE_TAC s THEN
  CLARIFY_TAC;;

let TEKMERION_ARM_XACCSTEP_TAC th excs aflag s =
  TEKMERION_ARM_SINGLE_STEP_TAC th s THEN
  (if aflag then TRY(ACCUMULATEX_ARITH_TAC excs s THEN CLARIFY_TAC)
   else ALL_TAC);;

let TEKMERION_ARM_XACCSTEPS_TAC th excs anums snums =
  MAP_EVERY
   (fun n -> TEKMERION_ARM_XACCSTEP_TAC th excs (mem n anums) ("s"^string_of_int n))
   snums;;

let TEKMERION_ARM_ACCSTEPS_TAC th anums snums =
  TEKMERION_ARM_XACCSTEPS_TAC th [`SP`] anums snums;;
"#;

/// Does this emitted tactic call the Tekmerion-owned ARM simulation?
fn tekmerion_arm_requested(tactic: &str) -> bool {
    tactic.contains(TEKMERION_ARM_SIMULATION)
}

/// The judged artefact: the tactic inlined after the CSAT and bridge libraries.
pub fn inlined_answer(tactic: &str, repo: &Path) -> Result<String> {
    inlined_answer_with(tactic, repo, false)
}

/// The phase-mode artefact (IX.66): the same answer, but `csat_lib.ml` and
/// `bridge.ml` come from instrumented *copies*.  `--mode answer` is untouched,
/// so the shipped answer is byte-identical to IX.65's.
pub fn inlined_answer_measured(tactic: &str, repo: &Path) -> Result<String> {
    inlined_answer_with(tactic, repo, true)
}

fn inlined_answer_with(tactic: &str, repo: &Path, measured: bool) -> Result<String> {
    let mut sources: Vec<(String, String)> = Vec::new();
    for rel in ANSWER_SOURCES {
        let path = repo.join(rel);
        let src = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let src = if !measured {
            src
        } else if *rel == "tools/csat_lib.ml" {
            instrument_csat_lib_markers(&src)?
        } else if *rel == "tools/bridge.ml" {
            instrument_bridge_markers(&src)?
        } else {
            src
        };
        sources.push((rel.to_string(), src));
    }
    // XXX.G3: the Tekmerion-owned ARM simulation tactic is inlined only for an
    // answer that calls it, so an x86 answer (which cannot resolve ARM names)
    // and every answer that does not use it are byte-identical to before.
    if tekmerion_arm_requested(tactic) {
        sources.push((
            "tekmerion/arm_supersede.ml".to_string(),
            TEKMERION_ARM_TACTIC.to_string(),
        ));
    }
    let refs: Vec<(&str, &str)> = sources
        .iter()
        .map(|(rel, src)| (rel.as_str(), src.as_str()))
        .collect();
    Ok(inlined_answer_from(tactic, &refs))
}

/// Assemble an answer from explicit library texts, in inline order.  `inlined_answer`
/// and `inlined_answer_measured` are this function over the tracked and the
/// instrumented texts respectively; exposing it lets the phase-mode contract
/// test rebuild the normal answer from the *deinstrumented* phase copy.
pub fn inlined_answer_from(tactic: &str, sources: &[(&str, &str)]) -> String {
    let mut parts: Vec<String> = Vec::new();
    for (_, src) in sources {
        parts.extend(phrases_from(src));
    }
    let joined = parts
        .iter()
        .map(|p| format!("{p}\nin"))
        .collect::<Vec<_>>()
        .join("\n");
    let wrapped = format!(
        "(fun g ->\n  try\n    ({}) g\n  with e ->\n    Printf.printf \"ANSWER_EXCEPTION: %s\\n%!\" (Printexc.to_string e);\n    raise e)\n",
        tactic.trim()
    );
    format!("(\n{joined}\n{wrapped})\n")
}

// ---------------------------------------------------------------------------
// Benchmark inputs
// ---------------------------------------------------------------------------

fn bench(repo: &Path) -> PathBuf {
    repo.join(".bench")
}

/// The benchmark tree (`<repo>/.bench/s2n-bignum`), exposed so the fixture
/// recorder and the parity tests locate objects exactly as the emitter does.
pub fn s2n_root(repo: &Path) -> PathBuf {
    bench(repo).join("s2n-bignum")
}

fn s2n(repo: &Path) -> PathBuf {
    s2n_root(repo)
}

/// Environment variable naming an alternate benchmark corpus (`problems.json`).
///
/// Resolution, in order:
///
/// 1. `<repo>/.bench/problems.json` when it exists — a live corpus always wins,
///    so a committed fixture can never silently shadow the pin;
/// 2. `TEKMERION_PROBLEMS` when it is set and non-empty — this is how a
///    bench-free checkout (CI) reaches `tests/fixtures/problems.json`;
/// 3. `<repo>/.bench/problems.json` otherwise, so a caller with neither sees an
///    error naming the historical default path.
///
/// With the variable unset, resolution is exactly the pre-XVI.A behaviour.
/// XVI.C1 must bind this variable into the freeze configuration once that work
/// lands; see `research/receipts/ix72-offline-inputs.md`.
pub const PROBLEMS_ENV: &str = "TEKMERION_PROBLEMS";

fn problems_path(repo: &Path) -> PathBuf {
    let live = bench(repo).join("problems.json");
    if live.is_file() {
        return live;
    }
    match std::env::var_os(PROBLEMS_ENV) {
        Some(value) if !value.is_empty() => PathBuf::from(value),
        _ => live,
    }
}

fn load_problems(repo: &Path) -> Result<Json> {
    let path = problems_path(repo);
    let bytes = fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    json::parse(&bytes).map_err(|e| EmitError::Other(format!("corpus: {e}")))
}

fn problem<'a>(problems: &'a Json, task: &str) -> Result<&'a Json> {
    problems
        .get(task)
        .ok_or_else(|| EmitError::Other(format!("unknown task id {task:?}")))
}

fn query_of<'a>(problems: &'a Json, task: &str) -> Result<&'a str> {
    problem(problems, task)?
        .get("query")
        .and_then(Json::as_str)
        .ok_or_else(|| EmitError::Other(format!("{task:?} has no query")))
}

fn theorem_of<'a>(problems: &'a Json, task: &str) -> Result<&'a str> {
    problem(problems, task)?
        .get("json")
        .and_then(|j| j.get("theorem_name"))
        .and_then(Json::as_str)
        .ok_or_else(|| EmitError::Other(format!("{task:?} has no theorem_name")))
}

fn exec_name(theorem: &str) -> String {
    format!(
        "{}_EXEC",
        theorem.strip_suffix("_CORRECT").unwrap_or(theorem)
    )
}

/// The family from an `arm.bignum_<op>_<family>.` task id.
fn arm_family(task: &str) -> Option<String> {
    let rest = task.strip_prefix("arm.")?;
    let body = rest.strip_prefix("bignum_")?;
    let (op, tail) = body.split_once('_')?;
    if !matches!(op, "add" | "sub" | "neg") {
        return None;
    }
    let (family, after) = tail.split_once('.')?;
    if after.contains('.') || family.is_empty() {
        return None;
    }
    if !family
        .bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
    {
        return None;
    }
    Some(family.to_string())
}

/// The family from an `x86.bignum_<op>_<family>.` task id, checking the op.
fn x86_family(op: &str, task: &str) -> Option<String> {
    let rest = task.strip_prefix("x86.")?;
    let body = rest.strip_prefix("bignum_")?;
    let (task_op, tail) = body.split_once('_')?;
    if task_op != op {
        return None;
    }
    let (family, after) = tail.split_once('.')?;
    if after.contains('.') || family.is_empty() {
        return None;
    }
    if !family
        .bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
    {
        return None;
    }
    Some(family.to_string())
}

/// Locate the unique `<repo>/.bench/s2n-bignum/<arch>/*/bignum_<op>_<family>.o`.
///
/// Public so the fixture recorder and the corpus-agreement test use the same
/// search as the emitter rather than a second copy of it.
pub fn object_for(
    arch: &str,
    op: &str,
    family: &str,
    repo: &Path,
) -> std::result::Result<PathBuf, EmitError> {
    let root = s2n(repo).join(arch);
    let wanted = format!("bignum_{op}_{family}.o");
    let mut matches: Vec<PathBuf> = Vec::new();
    let entries =
        fs::read_dir(&root).map_err(|e| EmitError::Other(format!("{}: {e}", root.display())))?;
    for entry in entries.flatten() {
        let dir = entry.path();
        if dir.is_dir() {
            let candidate = dir.join(&wanted);
            if candidate.is_file() {
                matches.push(candidate);
            }
        }
    }
    matches.sort();
    if matches.len() != 1 {
        return Err(EmitError::Other(format!(
            "expected one {arch} object for {op}/{family}, found {matches:?}"
        )));
    }
    let found = matches.remove(0);
    Ok(found)
}

fn relative_to(base: &Path, path: &Path) -> String {
    match path.strip_prefix(base) {
        Ok(rel) => rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join("/"),
        Err(_) => path.display().to_string(),
    }
}

// ---------------------------------------------------------------------------
// ARM
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ArmInfo {
    pub task: String,
    pub op: String,
    pub query: String,
    pub spec: Spec,
    pub accumulate: Vec<usize>,
    pub exec_name: String,
    pub objrel: String,
    pub family: String,
    pub modulus: String,
    pub tactic: String,
    pub last: usize,
}

/// `TEKMERION_ALG_CLOSE=<task id>` opts the named task into the algebraic
/// close (IX.70).  Unset, the SAT path is emitted, so the shipped tree proves
/// exactly what it proved before.
fn algebraic_close_requested(task: &str) -> bool {
    std::env::var("TEKMERION_ALG_CLOSE")
        .map(|v| v == task)
        .unwrap_or(false)
}

pub fn build_arm(op: &str, task: &str, policy: &GuardPolicy, repo: &Path) -> Result<ArmInfo> {
    policy.check_task(task)?;
    let problems = load_problems(repo)?;
    let query = query_of(&problems, task)?.to_string();
    let theorem = theorem_of(&problems, task)?.to_string();
    let exec_name = exec_name(&theorem);

    let family = arm_family(task)
        .ok_or_else(|| EmitError::Other(format!("not an ARM bignum task id: {task:?}")))?;
    let obj = object_for("arm", op, &family, repo)?;
    policy.check_object(&obj, Some(&s2n(repo)))?;

    let steps = disasm::disassemble(&obj, None, None)?;
    let spec = parse_spec(&query)?;
    let accumulate = derive::derive_accumulate(&steps);
    let last = steps
        .len()
        .checked_sub(1)
        .ok_or_else(|| EmitError::Other(format!("no instructions in {}", obj.display())))?;
    let modulus = spec.conclusion.modulus().to_string();
    let backend = simulation_backend()?;
    let tactic = [
        format!("{} THEN", preamble(&spec)),
        format!(
            "{} THEN",
            arm_simulation(backend, &exec_name, &accumulate, last)
        ),
        post_text_for(
            Some(task),
            algebraic_close_requested(task),
            op,
            &format!("s{last}"),
            &modulus,
        ),
    ]
    .join("\n");
    Ok(ArmInfo {
        task: task.to_string(),
        op: op.to_string(),
        query,
        spec,
        accumulate,
        exec_name,
        objrel: relative_to(&s2n(repo), &obj),
        family,
        modulus,
        tactic,
        last,
    })
}

/// The `let p_<family> = new_definition ...` phrase, for `--mode dev` only.
pub fn modulus_definition(
    arch: &str,
    op: &str,
    family: &str,
    policy: &GuardPolicy,
    repo: &Path,
) -> Result<String> {
    let proof = s2n(repo)
        .join(arch)
        .join("proofs")
        .join(format!("bignum_{op}_{family}.ml"));
    policy.check_proof(&proof, Some(&s2n(repo)))?;
    let src = fs::read_to_string(&proof)
        .map_err(|e| EmitError::Other(format!("{}: {e}", proof.display())))?;
    for line in src.split('\n') {
        if let Some(found) = match_modulus_definition(line) {
            return Ok(found);
        }
    }
    other(format!(
        "no modulus definition found in {}",
        proof.display()
    ))
}

fn match_modulus_definition(line: &str) -> Option<String> {
    let rest = line.strip_prefix("let ")?;
    let name_body = rest.strip_prefix("p_")?;
    let name_len = name_body
        .bytes()
        .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
        .count();
    if name_len == 0 {
        return None;
    }
    let after_name = &rest[2 + name_len..];
    let after_kw = after_name.trim_start_matches(|c: char| c.is_whitespace());
    let after_kw = after_kw.strip_prefix("= new_definition")?;
    let after_kw = after_kw.trim_start_matches(|c: char| c.is_whitespace());
    let after_tick = after_kw.strip_prefix('`')?;
    let close = after_tick.find('`')?;
    let after_close = &after_tick[close + 1..];
    if !after_close.starts_with(";;") {
        return None;
    }
    let prefix_len = line.len() - after_close.len() + 2;
    Some(line[..prefix_len].to_string())
}

/// The `define_assert_from_elf ... [words]` phrase, for `--mode dev` only.
pub fn machine_code_phrase(
    arch: &str,
    op: &str,
    family: &str,
    mcname: &str,
    objrel: &str,
    policy: &GuardPolicy,
    repo: &Path,
) -> Result<String> {
    let proof = s2n(repo)
        .join(arch)
        .join("proofs")
        .join(format!("bignum_{op}_{family}.ml"));
    policy.check_proof(&proof, Some(&s2n(repo)))?;
    let src = fs::read_to_string(&proof)
        .map_err(|e| EmitError::Other(format!("{}: {e}", proof.display())))?;
    let words = extract_elf_words(&src).ok_or_else(|| {
        EmitError::Other(format!(
            "no define_assert_from_elf block in {}",
            proof.display()
        ))
    })?;
    Ok(format!(
        "let {mcname} = define_assert_from_elf \"{mcname}\" \"{objrel}\"[{words}];;"
    ))
}

fn extract_elf_words(src: &str) -> Option<String> {
    let bytes = src.as_bytes();
    let at = find_from(bytes, b"define_assert_from_elf", 0)?;
    let mut c = Cursor::at(bytes, at + "define_assert_from_elf".len());
    c.skip_ws();
    c.eat(b'"')?;
    c.i += take_until(bytes, c.i, b'"')?;
    c.eat(b'"')?;
    c.skip_ws();
    c.eat(b'"')?;
    c.i += take_until(bytes, c.i, b'"')?;
    c.eat(b'"')?;
    c.skip_ws();
    c.eat(b'[')?;
    let end_rel = find_from(bytes, b"];;", c.i)?;
    Some(std::str::from_utf8(&bytes[c.i..end_rel]).ok()?.to_string())
}

fn take_until(bytes: &[u8], from: usize, needle: u8) -> Option<usize> {
    bytes[from..].iter().position(|b| *b == needle)
}

fn mc_name(exec: &str) -> String {
    let base = exec.to_lowercase();
    let base = base.strip_suffix("_exec").unwrap_or(&base);
    format!("{base}_mc")
}

/// `--mode dev` harness for the ARM route (scratch process only).
pub fn dev_script_arm(info: &ArmInfo, policy: &GuardPolicy, repo: &Path) -> Result<String> {
    let mcname = mc_name(&info.exec_name);
    let moddef = modulus_definition("arm", &info.op, &info.family, policy, repo)?;
    let mcphrase = machine_code_phrase(
        "arm",
        &info.op,
        &info.family,
        &mcname,
        &info.objrel,
        policy,
        repo,
    )?;
    let lines = [
        format!("(* DEVELOPMENT_ONLY dev harness for {}.", info.task),
        "   Not a receipt: the judged artefact is the inlined answer. *)".to_string(),
        "#load \"unix.cma\";;".to_string(),
        "needs \"arm/proofs/base.ml\";;".to_string(),
        "needs \"Cadical/make.ml\";;".to_string(),
        format!("loadt \"{}/tools/csat_lib.ml\";;", repo.display()),
        format!("loadt \"{}/tools/bridge.ml\";;", repo.display()),
        TEKMERION_ARM_TACTIC.to_string(),
        moddef,
        mcphrase,
        format!("let {} = ARM_MK_EXEC_RULE {};;", info.exec_name, mcname),
        format!("let ref_goal = {};;", info.query),
        "let ref_t0 = Unix.gettimeofday ();;".to_string(),
        format!("let ref_th = time prove (ref_goal,\n{});;", info.tactic),
        "Printf.printf \"REF_DEV %s ok=%b seconds=%.3f axioms=%d\\n%!\"".to_string(),
        format!("  \"{}\" (concl ref_th = ref_goal)", info.task),
        "  (Unix.gettimeofday () -. ref_t0) (List.length (axioms ()));;".to_string(),
    ];
    Ok(format!("{}\n", lines.join("\n")))
}

// ---------------------------------------------------------------------------
// x86
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct X86Info {
    pub task: String,
    pub op: String,
    pub query: String,
    pub spec: Spec,
    pub accumulate: Vec<usize>,
    pub exec_name: String,
    pub objrel: String,
    pub family: String,
    pub modulus: String,
    pub tactic: String,
    pub last: usize,
    pub entry: i64,
    pub end: i64,
    pub uses_neg: bool,
}

fn find_rip(text: &str) -> Vec<i64> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut search = 0;
    while let Some(at) = find_from(bytes, b"read", search) {
        if let Some((value, end)) = parse_rip_at(bytes, at) {
            out.push(value);
            search = end;
        } else {
            search = at + 1;
        }
    }
    out
}

fn parse_rip_at(s: &[u8], start: usize) -> Option<(i64, usize)> {
    // read\s+RIP\s+s\s*=\s*word\s*\(?\s*pc\s*(?:\+\s*(0x[0-9a-fA-F]+))?\s*\)?
    let mut c = Cursor::at(s, start + "read".len());
    if c.ws_span() == 0 {
        return None;
    }
    c.eat_lit("RIP")?;
    if c.ws_span() == 0 {
        return None;
    }
    c.eat_lit("s")?;
    c.skip_ws();
    c.eat(b'=')?;
    c.skip_ws();
    c.eat_lit("word")?;
    c.skip_ws();
    if c.peek() == Some(b'(') {
        c.bump();
    }
    c.skip_ws();
    c.eat_lit("pc")?;
    c.skip_ws();
    let mut value = 0i64;
    if c.peek() == Some(b'+') {
        c.bump();
        c.skip_ws();
        c.eat_lit("0x")?;
        let hex = c.hex_digits()?;
        value = i64::from_str_radix(&hex, 16).ok()?;
    }
    c.skip_ws();
    if c.peek() == Some(b')') {
        c.bump();
    }
    Some((value, c.i))
}

/// The precondition entry and postcondition target, as trimmed offsets.
pub fn entry_and_end(query: &str) -> std::result::Result<(i64, i64), String> {
    let found = find_rip(query);
    if found.len() != 2 {
        return Err(format!(
            "expected exactly two `read RIP s = word (pc + k)` clauses, found {}",
            found.len()
        ));
    }
    Ok((found[0], found[1]))
}

/// Derived x86 accumulate list, minus the mnemonic ACCUMULATE cannot unfold.
pub fn accumulate_set(steps: &[derive::Step]) -> Vec<usize> {
    let derived = derive::derive_accumulate_with(steps, &derive::x86_semantics());
    derived
        .into_iter()
        .filter(|&n| steps.get(n - 1).map(|s| s.mnemonic.as_str()) != Some("neg"))
        .collect()
}

/// Index of the last instruction that writes memory, with a consistency check.
///
/// The postcondition's target address must be the address of the instruction
/// right after that store. If it is not, the disassembly and the query do not
/// describe the same code and no step range can be trusted.
pub fn last_store(steps: &[derive::Step], end: i64) -> std::result::Result<usize, String> {
    let stores: Vec<usize> = steps
        .iter()
        .filter(|s| s.operands.first().map(|o| o.contains('[')).unwrap_or(false))
        .map(|s| s.index)
        .collect();
    let last = match stores.last() {
        Some(n) => *n,
        None => return Err("no memory-writing instruction found in the routine".to_string()),
    };
    if last >= steps.len() {
        return Err("last store is the final instruction; no trailing address".to_string());
    }
    let next_address = steps[last].address;
    if next_address != end {
        return Err(format!(
            "disassembly/query mismatch: after the last store (step {last}) the next instruction is at {next_address}, but the query's postcondition RIP is {end}"
        ));
    }
    Ok(last)
}

pub fn build_x86(op: &str, task: &str, policy: &GuardPolicy, repo: &Path) -> Result<X86Info> {
    policy.check_task(task)?;
    let problems = load_problems(repo)?;
    let query = query_of(&problems, task)?.to_string();
    let theorem = theorem_of(&problems, task)?.to_string();
    let exec_name = exec_name(&theorem);

    let family = x86_family(op, task)
        .ok_or_else(|| EmitError::Other(format!("not an x86 bignum {op} task id: {task:?}")))?;
    let obj = object_for("x86", op, &family, repo)?;
    policy.check_object(&obj, Some(&s2n(repo)))?;

    let (entry, end) = entry_and_end(&query)?;
    let full_steps = disasm::disassemble(&obj, None, Some("x86"))?;
    let steps = disasm::enter_at(full_steps, entry)?;
    let spec = parse_spec(&query)?;
    let accumulate = accumulate_set(&steps);
    let last = last_store(&steps, end)?;
    let uses_neg = steps.iter().any(|s| s.mnemonic == "neg");
    let modulus = spec.conclusion.modulus().to_string();
    let tactic = [
        format!("{} THEN", preamble(&spec)),
        format!("{} THEN", x86_simulation(&exec_name, &accumulate, last)),
        post_simulation_x86(
            Some(task),
            algebraic_close_requested(task),
            op,
            &format!("s{last}"),
            &modulus,
            uses_neg,
        ),
    ]
    .join("\n");
    Ok(X86Info {
        task: task.to_string(),
        op: op.to_string(),
        query,
        spec,
        accumulate,
        exec_name,
        objrel: relative_to(&s2n(repo), &obj),
        family,
        modulus,
        tactic,
        last,
        entry,
        end,
        uses_neg,
    })
}

/// `--mode dev` harness for the x86 route (scratch process only).
pub fn dev_script_x86(info: &X86Info, policy: &GuardPolicy, repo: &Path) -> Result<String> {
    let mcname = mc_name(&info.exec_name);
    let tmcname = format!("{}_tmc", mcname.strip_suffix("_mc").unwrap_or(&mcname));
    let moddef = modulus_definition("x86", &info.op, &info.family, policy, repo)?;
    let mcphrase = machine_code_phrase(
        "x86",
        &info.op,
        &info.family,
        &mcname,
        &info.objrel,
        policy,
        repo,
    )?;
    let lines = [
        format!("(* DEVELOPMENT_ONLY dev harness for {}.", info.task),
        "   Not a receipt: the judged artefact is the inlined answer.".to_string(),
        "   Run from .bench/s2n-bignum (define_assert_from_elf resolves the".to_string(),
        "   object relative to that directory). *)".to_string(),
        "#load \"unix.cma\";;".to_string(),
        "needs \"x86/proofs/base.ml\";;".to_string(),
        "needs \"Cadical/make.ml\";;".to_string(),
        format!("loadt \"{}/tools/csat_lib.ml\";;", repo.display()),
        format!("loadt \"{}/tools/bridge.ml\";;", repo.display()),
        moddef,
        mcphrase,
        format!("let {tmcname} = define_trimmed \"{tmcname}\" {mcname};;"),
        format!(
            "let {} = X86_MK_CORE_EXEC_RULE {};;",
            info.exec_name, tmcname
        ),
        format!("let ref_goal = {};;", info.query),
        "let ref_t0 = Unix.gettimeofday ();;".to_string(),
        format!("let ref_th = time prove (ref_goal,\n{});;", info.tactic),
        "Printf.printf \"REF_DEV %s ok=%b seconds=%.3f axioms=%d\\n%!\"".to_string(),
        format!("  \"{}\" (concl ref_th = ref_goal)", info.task),
        "  (Unix.gettimeofday () -. ref_t0) (List.length (axioms ()));;".to_string(),
    ];
    Ok(format!("{}\n", lines.join("\n")))
}

// ---------------------------------------------------------------------------
// Measurement: the certificate-size probe (XIII.A, step 1)
// ---------------------------------------------------------------------------
//
// This route measures one thing: how many SAT closes a proof performs and how
// large the LRAT texts are. It runs the ordinary tactic against an instrumented
// *copy* of `tools/csat_lib.ml` (the copy defines `csat_refute_hook` and calls
// it while the temporary files exist) and copies each `(CNF, LRAT)` pair out
// before they are deleted, checking that the clause list derived from the CNF
// term is the list `read_dimacs` reads back. The tracked library is untouched.
//
// The certified check path was **not** built. The measurement (4 closes,
// 65,657,625 bytes of LRAT; 22,765,098 bytes even lemma-trimmed) exceeds the
// declared 8 MB design gate, so in-answer certificates are not the design.
// See `research/receipts/ix61-certificates-as-data.md`.

/// The first `linenum` lines (newlines kept) of a task's inlined template.
pub fn template_prefix(repo: &Path, task: &str) -> Result<String> {
    let problems = load_problems(repo)?;
    let first = problem(&problems, task)?
        .get("inlined_locations")
        .and_then(Json::as_array)
        .and_then(|a| a.first())
        .and_then(Json::as_array)
        .ok_or_else(|| EmitError::Other(format!("{task:?} has no inlined_locations[0]")))?;
    let rel = first
        .first()
        .and_then(Json::as_str)
        .ok_or_else(|| EmitError::Other(format!("{task:?} inlined_locations[0][0]")))?;
    let linenum = match first.get(1) {
        Some(Json::Num(n)) if *n >= 0.0 && n.fract() == 0.0 => *n as usize,
        _ => {
            return other(format!(
                "{task:?} inlined_locations[0][1] is not an integer"
            ))
        }
    };
    let path = bench(repo).join(rel);
    let src = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut out = String::new();
    for (count, line) in src.split_inclusive('\n').enumerate() {
        if count >= linenum {
            break;
        }
        out.push_str(line);
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
    Ok(out)
}

/// Instrument a *copy* of `tools/csat_lib.ml` for the measurement run: define a
/// hook and call it from `CSAT_REFUTE_GEN` while the temporary CNF/LRAT files
/// still exist. The tracked library is left untouched, because adding the hook
/// to it would change every emitted answer and break the byte-identity
/// regression baseline for the reference answers.
fn instrument_csat_lib(src: &str) -> Result<String> {
    let anchor_hook = "let csat_debug = ref false;;";
    let anchor_call = "let res = LRAT_REFUTE_GEN amap cnfname lratname in";
    let with_hook = format!(
        "{anchor_hook}\n\
         let csat_refute_hook : (term -> string -> string -> unit) ref =\n\
           ref (fun _ _ _ -> ());;"
    );
    let with_call = format!(
        "let _ = !csat_refute_hook tm cnfname lratname in\n\
         {anchor_call}"
    );
    if !src.contains(anchor_hook) || !src.contains(anchor_call) {
        return other("tools/csat_lib.ml: measurement anchors not found");
    }
    Ok(src
        .replacen(anchor_hook, &with_hook, 1)
        .replacen(anchor_call, &with_call, 1))
}

/// The measurement script: template prefix, the CSAT/bridge sources inlined as
/// one nested `let ... in` chain (the harness compiles a script as a single
/// unit, so a runtime `loadt` would not supply names), the unchanged tactic,
/// and the hook that dumps each certificate.
pub fn cert_producer_source(info: &ArmInfo, repo: &Path, work: &Path) -> Result<String> {
    let work = fs::canonicalize(work).map_err(|e| format!("{}: {e}", work.display()))?;
    let sources = [
        ".bench/hol-light/Cadical/dimacs.ml",
        ".bench/hol-light/Cadical/cnf.ml",
        ".bench/hol-light/Cadical/ldrup.ml",
        "tools/csat_lib.ml",
        "tools/bridge.ml",
    ];
    let mut parts: Vec<String> = Vec::new();
    for rel in sources {
        let path = repo.join(rel);
        let src = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let src = if rel == "tools/csat_lib.ml" {
            instrument_csat_lib(&src)?
        } else {
            src
        };
        parts.extend(crate::tools::inline_answer::phrases_from(&src));
    }
    if tekmerion_arm_requested(&info.tactic) {
        parts.extend(crate::tools::inline_answer::phrases_from(
            TEKMERION_ARM_TACTIC,
        ));
    }
    let mut s = template_prefix(repo, &info.task)?;
    s.push_str("\n(* ==== C61 measurement run: untrusted, solver and I/O allowed ==== *)\n(\n");
    for phrase in parts {
        s.push_str(&phrase);
        s.push_str("\nin\n");
    }
    s.push_str(&format!(
        r#"let tek_cert_dir = "{}" in
let tek_slurp f =
  let ic = open_in_bin f in
  let n = in_channel_length ic in
  let s = really_input_string ic n in
  close_in ic; s in
let tek_closes = ref 0 in
let tek_dump tm cnffile lratfile =
  incr tek_closes;
  let cnf = tek_slurp cnffile and lrat = tek_slurp lratfile in
  let key = Digest.to_hex (Digest.string (string_of_term tm)) in
  let prb = read_dimacs cnffile in
  let ats = atoms tm in
  let _,vmap =
    rev_itlist (fun a (n,f) -> (n+1,(a |-> n) f)) ats (1,undefined) in
  let derived = clauses_of_term (apply vmap) tm in
  Printf.printf
    "CERTCLOSE n=%d key=%s cnf_bytes=%d lrat_bytes=%d nvars=%d nclauses=%d clauses_equal=%b\n%!"
    !tek_closes key (String.length cnf) (String.length lrat)
    (rev_itlist (rev_itlist (max o abs)) prb 0) (List.length prb)
    (derived = prb);
  let oc = open_out_bin (Filename.concat tek_cert_dir (key ^ ".cnf")) in
  output_string oc cnf; close_out oc;
  let oc = open_out_bin (Filename.concat tek_cert_dir (key ^ ".lrat")) in
  output_string oc lrat; close_out oc in
let _ = csat_refute_hook := tek_dump in
let tek_t0 = Unix.gettimeofday () in
let tek_th = prove({}, ({})) in
Printf.printf "PRODUCER task={} ok=%b hyps=%d closes=%d seconds=%.3f\n%!"
  (concl tek_th = {}) (List.length (hyp tek_th)) !tek_closes
  (Unix.gettimeofday () -. tek_t0);
 tek_th
)
;;
"#,
        work.display(),
        info.query,
        info.tactic,
        info.task,
        info.query,
    ));
    Ok(s)
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

struct EmitArgs {
    task: String,
    mode: String,
    out: String,
    accumulate_out: Option<String>,
    work: Option<String>,
    guard: guard::ModeArgs,
}

fn parse_emit_args(argv: &[String]) -> std::result::Result<EmitArgs, String> {
    let mut task = None;
    let mut mode = "answer".to_string();
    let mut out = None;
    let mut accumulate_out = None;
    let mut work = None;
    let mut guard_args = guard::ModeArgs::default();
    let mut i = 0;
    while i < argv.len() {
        if let Some(v) = guard::take_value(argv, &mut i, "--task") {
            task = Some(v);
        } else if let Some(v) = guard::take_value(argv, &mut i, "--mode") {
            mode = v;
        } else if let Some(v) = guard::take_value(argv, &mut i, "--out") {
            out = Some(v);
        } else if let Some(v) = guard::take_value(argv, &mut i, "--accumulate-out") {
            accumulate_out = Some(v);
        } else if let Some(v) = guard::take_value(argv, &mut i, "--work") {
            work = Some(v);
        } else if let Some(v) = guard::take_value(argv, &mut i, "--sealed-manifest") {
            guard_args.sealed_manifest = Some(v);
        } else if let Some(v) = guard::take_value(argv, &mut i, "--freeze-tag") {
            guard_args.freeze_tag = Some(v);
        } else if let Some(v) = guard::take_value(argv, &mut i, "--spent-manifest") {
            guard_args.spent_manifest = Some(v);
        } else if let Some(v) = guard::take_value(argv, &mut i, "--held-out") {
            guard_args.held_out.push(v);
        } else {
            match argv[i].as_str() {
                "--develop" => guard_args.develop = true,
                "--sealed-eval" => guard_args.sealed_eval = true,
                "--spent" => guard_args.spent = true,
                other => return Err(format!("unrecognised argument {other:?}")),
            }
        }
        i += 1;
    }
    if !matches!(mode.as_str(), "answer" | "dev" | "cert-producer" | "phase") {
        return Err(format!("invalid --mode {mode:?}"));
    }
    Ok(EmitArgs {
        task: task.ok_or("--task is required")?,
        mode,
        out: out.ok_or("--out is required")?,
        accumulate_out,
        work,
        guard: guard_args,
    })
}

fn write_outputs(
    out: &str,
    text: &str,
    accumulate_out: Option<&str>,
    accumulate: &[usize],
) -> std::result::Result<(), String> {
    let path = Path::new(out);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
    }
    fs::write(path, text).map_err(|e| format!("{out}: {e}"))?;
    if let Some(dest) = accumulate_out {
        let body: String = accumulate
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        fs::write(dest, format!("{body}\n")).map_err(|e| format!("{dest}: {e}"))?;
    }
    Ok(())
}

fn op_of(task: &str) -> std::result::Result<String, String> {
    let at = find_from(task.as_bytes(), b"bignum_", 0)
        .ok_or_else(|| format!("not a bignum add/sub/neg task: {task:?}"))?;
    let rest = &task[at + "bignum_".len()..];
    for op in ["add", "sub", "neg"] {
        if rest.starts_with(op) && rest[op.len()..].starts_with('_') {
            return Ok(op.to_string());
        }
    }
    Err(format!("not a bignum add/sub/neg task: {task:?}"))
}

/// ARM emitter CLI. Returns the process exit status.
pub fn run_arm(argv: &[String]) -> i32 {
    let args = match parse_emit_args(argv) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("ERROR {e}");
            return 2;
        }
    };
    let repo = guard::repo_root();
    match emit_arm(&args, &repo) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

fn emit_arm(args: &EmitArgs, repo: &Path) -> Result<()> {
    let policy = guard::policy_from_args(&args.guard, repo)?;
    let op = op_of(&args.task).map_err(EmitError::Other)?;
    let info = build_arm(&op, &args.task, &policy, repo)?;
    if args.mode == "cert-producer" {
        let work = args
            .work
            .as_deref()
            .ok_or_else(|| EmitError::Other("--work is required".into()))?;
        let work_path = Path::new(work);
        fs::create_dir_all(work_path).map_err(|e| format!("{work}: {e}"))?;
        let text = cert_producer_source(&info, repo, work_path)?;
        let path = Path::new(&args.out);
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
            }
        }
        fs::write(path, &text).map_err(|e| format!("{}: {e}", args.out))?;
        println!(
            "wrote {} mode=cert-producer task={} work={} bytes={}",
            args.out,
            args.task,
            work,
            text.len()
        );
        return Ok(());
    }
    let text = if args.mode == "answer" {
        inlined_answer(&info.tactic, repo)?
    } else if args.mode == "phase" {
        inlined_answer_measured(&phase_tactic(&info.tactic, &args.task), repo)?
    } else {
        dev_script_arm(&info, &policy, repo)?
    };
    write_outputs(
        &args.out,
        &text,
        args.accumulate_out.as_deref(),
        &info.accumulate,
    )
    .map_err(EmitError::Other)?;
    println!(
        "wrote {} mode={} task={} accumulate={:?} bytes={}",
        args.out,
        args.mode,
        args.task,
        info.accumulate,
        text.len()
    );
    Ok(())
}

/// x86 emitter CLI. Returns the process exit status.
pub fn run_x86(argv: &[String]) -> i32 {
    let args = match parse_emit_args(argv) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("ERROR {e}");
            return 2;
        }
    };
    let repo = guard::repo_root();
    match emit_x86(&args, &repo) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

fn emit_x86(args: &EmitArgs, repo: &Path) -> Result<()> {
    let policy = guard::policy_from_args(&args.guard, repo)?;
    let op = op_of(&args.task).map_err(EmitError::Other)?;
    let info = build_x86(&op, &args.task, &policy, repo)?;
    let text = if args.mode == "answer" {
        inlined_answer(&info.tactic, repo)?
    } else if args.mode == "phase" {
        inlined_answer_measured(&phase_tactic(&info.tactic, &args.task), repo)?
    } else {
        dev_script_x86(&info, &policy, repo)?
    };
    write_outputs(
        &args.out,
        &text,
        args.accumulate_out.as_deref(),
        &info.accumulate,
    )
    .map_err(EmitError::Other)?;
    println!(
        "wrote {} mode={} task={} entry={} last={} neg={} accumulate={:?} bytes={}",
        args.out,
        args.mode,
        args.task,
        info.entry,
        info.last,
        info.uses_neg,
        info.accumulate,
        text.len()
    );
    Ok(())
}

#[cfg(test)]
mod backend_tests {
    use super::*;

    #[test]
    fn arm_simulation_names_the_selected_backend() {
        let accumulate = [3usize, 4];
        assert_eq!(
            arm_simulation(SimBackend::Tekmerion, "EXEC", &accumulate, 4),
            "TEKMERION_ARM_ACCSTEPS_TAC EXEC [3;4] (1--4)"
        );
        assert_eq!(
            arm_simulation(SimBackend::Upstream, "EXEC", &accumulate, 4),
            "ARM_ACCSTEPS_TAC EXEC [3;4] (1--4)"
        );
    }

    #[test]
    fn an_empty_accumulate_set_is_backend_independent() {
        // No carry chain: both backends use the plain `ARM_STEPS_TAC`, so the two
        // emitted answers are byte-identical.
        assert_eq!(
            arm_simulation(SimBackend::Tekmerion, "EXEC", &[], 7),
            "ARM_STEPS_TAC EXEC (1--7)"
        );
        assert_eq!(
            arm_simulation(SimBackend::Upstream, "EXEC", &[], 7),
            "ARM_STEPS_TAC EXEC (1--7)"
        );
    }

    #[test]
    fn backend_names_are_the_documented_spellings() {
        assert_eq!(SimBackend::Tekmerion.name(), "tekmerion");
        assert_eq!(SimBackend::Upstream.name(), "upstream");
    }
}
