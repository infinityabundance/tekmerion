//! Minimal typed term language for native bit-vector arithmetic.
//!
//! Sorts are `Bool` and `BitVec(w)` for `1 <= w <= 64`. The only Boolean
//! constructors are `true`, `false`, `not`, and word equality; the only word
//! constructors are variables, literals, addition, and subtraction. Addition and
//! subtraction are modulo `2^w`, which is the machine semantics the checker
//! claims. That is stated, not assumed: [`Term::eval`] implements the same
//! semantics and the bit-blaster is tested against it.
//!
//! There is no implicit coercion. A term is either well sorted or refused.

#![allow(clippy::should_implement_trait)]

use std::collections::BTreeMap;

use crate::native::Error;
use crate::{Canon, Digest};

/// The sort of a well-typed term.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sort {
    Bool,
    BitVec(u32),
}

impl Sort {
    pub fn name(self) -> &'static str {
        match self {
            Sort::Bool => "Bool",
            Sort::BitVec(_) => "BitVec",
        }
    }
}

/// The value of a closed term under an assignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Value {
    Bool(bool),
    Bits(u64),
}

/// A typed term. Every variant other than [`Term::Bool`] has an associated sort.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Term {
    Bool(bool),
    Not(Box<Term>),
    Eq {
        width: u32,
        left: Box<Term>,
        right: Box<Term>,
    },
    Var {
        width: u32,
        name: String,
    },
    Const {
        width: u32,
        value: u64,
    },
    Add {
        width: u32,
        left: Box<Term>,
        right: Box<Term>,
    },
    Sub {
        width: u32,
        left: Box<Term>,
        right: Box<Term>,
    },
}

fn check_width(width: u32) -> Result<(), Error> {
    if width == 0 || width > 64 {
        return Err(Error::WidthOutOfRange(width));
    }
    Ok(())
}

fn canonical_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    let Some(first) = bytes.first() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || *first == b'_') {
        return false;
    }
    bytes[1..]
        .iter()
        .all(|b| b.is_ascii_alphanumeric() || *b == b'_')
}

impl Term {
    pub fn var(width: u32, name: &str) -> Result<Self, Error> {
        check_width(width)?;
        if !canonical_name(name) {
            return Err(Error::BadVarName(name.to_string()));
        }
        Ok(Term::Var {
            width,
            name: name.to_string(),
        })
    }

    pub fn constant(width: u32, value: u64) -> Result<Self, Error> {
        check_width(width)?;
        let fits = width == 64 || value < (1u64 << width);
        if !fits {
            return Err(Error::ConstOutOfRange {
                width,
                value: value.to_string(),
            });
        }
        Ok(Term::Const { width, value })
    }

    pub fn add(left: Term, right: Term) -> Result<Self, Error> {
        let width = binary_width(&left, &right)?;
        Ok(Term::Add {
            width,
            left: Box::new(left),
            right: Box::new(right),
        })
    }

    pub fn sub(left: Term, right: Term) -> Result<Self, Error> {
        let width = binary_width(&left, &right)?;
        Ok(Term::Sub {
            width,
            left: Box::new(left),
            right: Box::new(right),
        })
    }

    pub fn eq(left: Term, right: Term) -> Result<Self, Error> {
        let width = binary_width(&left, &right)?;
        Ok(Term::Eq {
            width,
            left: Box::new(left),
            right: Box::new(right),
        })
    }

    pub fn not(inner: Term) -> Result<Self, Error> {
        match inner.sort() {
            Sort::Bool => Ok(Term::Not(Box::new(inner))),
            s => Err(Error::SortMismatch {
                expected: "Bool",
                found: s.name(),
            }),
        }
    }

    /// The sort of this term. Constructors guarantee that children have the right
    /// sorts, so this is a shallow inspection rather than a validation pass.
    pub fn sort(&self) -> Sort {
        match self {
            Term::Bool(_) | Term::Not(_) | Term::Eq { .. } => Sort::Bool,
            Term::Var { width, .. }
            | Term::Const { width, .. }
            | Term::Add { width, .. }
            | Term::Sub { width, .. } => Sort::BitVec(*width),
        }
    }

    /// Recursive well-sortedness check.
    ///
    /// The constructors already enforce this, but the enum's variants are public
    /// for pattern matching. Every trust boundary (`bitblast`, `Certificate::new`)
    /// calls this, so a hand-built ill-typed term is refused rather than allowed to
    /// reach the encoder.
    pub fn validate(&self) -> Result<(), Error> {
        match self {
            Term::Bool(_) => Ok(()),
            Term::Not(inner) => {
                inner.validate()?;
                if inner.sort() != Sort::Bool {
                    return Err(Error::SortMismatch {
                        expected: "Bool",
                        found: inner.sort().name(),
                    });
                }
                Ok(())
            }
            Term::Var { width, name } => {
                check_width(*width)?;
                if !canonical_name(name) {
                    return Err(Error::BadVarName(name.clone()));
                }
                Ok(())
            }
            Term::Const { width, value } => {
                check_width(*width)?;
                if *width < 64 && *value >= (1u64 << *width) {
                    return Err(Error::ConstOutOfRange {
                        width: *width,
                        value: value.to_string(),
                    });
                }
                Ok(())
            }
            Term::Eq { width, left, right }
            | Term::Add { width, left, right }
            | Term::Sub { width, left, right } => {
                check_width(*width)?;
                left.validate()?;
                right.validate()?;
                let lw = left.width().ok_or(Error::SortMismatch {
                    expected: "BitVec",
                    found: "Bool",
                })?;
                let rw = right.width().ok_or(Error::SortMismatch {
                    expected: "BitVec",
                    found: "Bool",
                })?;
                if lw != rw {
                    return Err(Error::WidthMismatch {
                        left: lw,
                        right: rw,
                    });
                }
                if lw != *width {
                    return Err(Error::WidthMismatch {
                        left: *width,
                        right: lw,
                    });
                }
                Ok(())
            }
        }
    }

    pub fn width(&self) -> Option<u32> {
        match self.sort() {
            Sort::BitVec(w) => Some(w),
            Sort::Bool => None,
        }
    }

    /// Canonical rendering. Parsing is strict: a string is accepted only if it is
    /// exactly this rendering.
    pub fn render(&self) -> String {
        match self {
            Term::Bool(true) => "true".to_string(),
            Term::Bool(false) => "false".to_string(),
            Term::Not(t) => format!("(not {})", t.render()),
            Term::Eq { width, left, right } => {
                format!("(eq bv{} {} {})", width, left.render(), right.render())
            }
            Term::Var { width, name } => format!("(var bv{} {})", width, name),
            Term::Const { width, value } => format!("(const bv{} {})", width, value),
            Term::Add { width, left, right } => {
                format!("(add bv{} {} {})", width, left.render(), right.render())
            }
            Term::Sub { width, left, right } => {
                format!("(sub bv{} {} {})", width, left.render(), right.render())
            }
        }
    }

    /// Content identity. The encoding is domain-separated, length-prefixed for
    /// names, fixed-width for widths and literals, and tagged per constructor.
    pub fn digest(&self) -> Digest {
        let mut c = Canon::new("tekmerion/native/term/v1");
        self.hash_into(&mut c);
        c.finish()
    }

    fn hash_into(&self, c: &mut Canon) {
        match self {
            Term::Bool(b) => {
                c.tag(0);
                c.tag(u8::from(*b));
            }
            Term::Not(t) => {
                c.tag(1);
                c.digest(&t.digest());
            }
            Term::Eq { width, left, right } => {
                c.tag(2);
                c.bytes(&width.to_le_bytes());
                c.digest(&left.digest());
                c.digest(&right.digest());
            }
            Term::Var { width, name } => {
                c.tag(3);
                c.bytes(&width.to_le_bytes());
                c.bytes(name.as_bytes());
            }
            Term::Const { width, value } => {
                c.tag(4);
                c.bytes(&width.to_le_bytes());
                c.bytes(&value.to_le_bytes());
            }
            Term::Add { width, left, right } => {
                c.tag(5);
                c.bytes(&width.to_le_bytes());
                c.digest(&left.digest());
                c.digest(&right.digest());
            }
            Term::Sub { width, left, right } => {
                c.tag(6);
                c.bytes(&width.to_le_bytes());
                c.digest(&left.digest());
                c.digest(&right.digest());
            }
        }
    }

    /// Evaluate under a total assignment of the free variables. Values are masked
    /// to the declared width; addition and subtraction wrap modulo `2^width`.
    pub fn eval(&self, env: &BTreeMap<String, u64>) -> Result<Value, Error> {
        self.validate()?;
        self.eval_inner(env)
    }

    fn eval_inner(&self, env: &BTreeMap<String, u64>) -> Result<Value, Error> {
        match self {
            Term::Bool(b) => Ok(Value::Bool(*b)),
            Term::Not(t) => match t.eval_inner(env)? {
                Value::Bool(b) => Ok(Value::Bool(!b)),
                v => Err(Error::SortMismatch {
                    expected: "Bool",
                    found: sort_name_of_value(v),
                }),
            },
            Term::Eq { width, left, right } => {
                let a = left.eval_bits(*width, env)?;
                let b = right.eval_bits(*width, env)?;
                Ok(Value::Bool(a == b))
            }
            Term::Var { width, name } => {
                let v = env
                    .get(name)
                    .copied()
                    .ok_or_else(|| Error::UnboundVariable(name.clone()))?;
                Ok(Value::Bits(mask(v, *width)))
            }
            Term::Const { width, value } => Ok(Value::Bits(mask(*value, *width))),
            Term::Add { width, left, right } => {
                let a = left.eval_bits(*width, env)?;
                let b = right.eval_bits(*width, env)?;
                Ok(Value::Bits(mask(a.wrapping_add(b), *width)))
            }
            Term::Sub { width, left, right } => {
                let a = left.eval_bits(*width, env)?;
                let b = right.eval_bits(*width, env)?;
                Ok(Value::Bits(mask(a.wrapping_sub(b), *width)))
            }
        }
    }

    fn eval_bits(&self, width: u32, env: &BTreeMap<String, u64>) -> Result<u64, Error> {
        match self.eval_inner(env)? {
            Value::Bits(v) => Ok(mask(v, width)),
            Value::Bool(_) => Err(Error::SortMismatch {
                expected: "BitVec",
                found: "Bool",
            }),
        }
    }

    /// Strict canonical parse.
    pub fn parse(input: &str) -> Result<Self, Error> {
        let mut p = Parser::new(input);
        let term = p.term()?;
        p.skip_space();
        if p.pos != p.bytes.len() {
            return Err(Error::TrailingTermTokens);
        }
        if term.render() != input {
            return Err(Error::NonCanonicalTerm(input.to_string()));
        }
        Ok(term)
    }
}

fn mask(value: u64, width: u32) -> u64 {
    if width == 64 {
        value
    } else {
        value & ((1u64 << width) - 1)
    }
}

fn sort_name_of_value(v: Value) -> &'static str {
    match v {
        Value::Bool(_) => "Bool",
        Value::Bits(_) => "BitVec",
    }
}

fn binary_width(left: &Term, right: &Term) -> Result<u32, Error> {
    let lw = match left.sort() {
        Sort::BitVec(w) => w,
        s => {
            return Err(Error::SortMismatch {
                expected: "BitVec",
                found: s.name(),
            })
        }
    };
    let rw = match right.sort() {
        Sort::BitVec(w) => w,
        s => {
            return Err(Error::SortMismatch {
                expected: "BitVec",
                found: s.name(),
            })
        }
    };
    if lw != rw {
        return Err(Error::WidthMismatch {
            left: lw,
            right: rw,
        });
    }
    Ok(lw)
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(s: &'a str) -> Self {
        Parser {
            bytes: s.as_bytes(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn skip_space(&mut self) {
        while self.peek() == Some(b' ') {
            self.pos += 1;
        }
    }

    fn eat(&mut self, b: u8) -> bool {
        if self.peek() == Some(b) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, b: u8) -> Result<(), Error> {
        self.skip_space();
        if self.eat(b) {
            Ok(())
        } else {
            Err(Error::UnexpectedEof)
        }
    }

    fn atom(&mut self) -> Result<&'a str, Error> {
        self.skip_space();
        let start = self.pos;
        while let Some(b) = self.peek() {
            if b == b' ' || b == b'(' || b == b')' {
                break;
            }
            self.pos += 1;
        }
        if self.pos == start {
            return Err(Error::UnexpectedEof);
        }
        std::str::from_utf8(&self.bytes[start..self.pos]).map_err(|_| {
            Error::NonCanonicalTerm(
                String::from_utf8_lossy(&self.bytes[start..self.pos]).into_owned(),
            )
        })
    }

    fn width(&mut self) -> Result<u32, Error> {
        let atom = self.atom()?;
        let digits = atom
            .strip_prefix("bv")
            .ok_or_else(|| Error::BadNumber(atom.into()))?;
        let width = parse_decimal_u32(digits)?;
        check_width(width)?;
        Ok(width)
    }

    fn term(&mut self) -> Result<Term, Error> {
        self.skip_space();
        if !self.eat(b'(') {
            let atom = self.atom()?;
            return match atom {
                "true" => Ok(Term::Bool(true)),
                "false" => Ok(Term::Bool(false)),
                other => Err(Error::UnknownTermTag(other.to_string())),
            };
        }
        let tag = self.atom()?.to_string();
        let out = match tag.as_str() {
            "not" => {
                let inner = self.term()?;
                self.expect(b')')?;
                Term::not(inner)?
            }
            "eq" => {
                let width = self.width()?;
                let left = self.term()?;
                let right = self.term()?;
                self.expect(b')')?;
                check_width(width)?;
                Term::eq(left, right)?
            }
            "add" => {
                let width = self.width()?;
                let left = self.term()?;
                let right = self.term()?;
                self.expect(b')')?;
                check_width(width)?;
                Term::add(left, right)?
            }
            "sub" => {
                let width = self.width()?;
                let left = self.term()?;
                let right = self.term()?;
                self.expect(b')')?;
                check_width(width)?;
                Term::sub(left, right)?
            }
            "var" => {
                let width = self.width()?;
                let name = self.atom()?;
                self.expect(b')')?;
                Term::var(width, name)?
            }
            "const" => {
                let width = self.width()?;
                let atom = self.atom()?;
                let value = parse_decimal_u64(atom)?;
                self.expect(b')')?;
                Term::constant(width, value)?
            }
            other => return Err(Error::UnknownTermTag(other.to_string())),
        };
        Ok(out)
    }
}

fn parse_decimal_u32(s: &str) -> Result<u32, Error> {
    if s.is_empty() || (s.len() > 1 && s.starts_with('0')) {
        return Err(Error::BadNumber(s.to_string()));
    }
    if !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::BadNumber(s.to_string()));
    }
    s.parse::<u32>()
        .map_err(|_| Error::BadNumber(s.to_string()))
}

fn parse_decimal_u64(s: &str) -> Result<u64, Error> {
    if s.is_empty() || (s.len() > 1 && s.starts_with('0')) {
        return Err(Error::BadNumber(s.to_string()));
    }
    if !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::BadNumber(s.to_string()));
    }
    s.parse::<u64>()
        .map_err(|_| Error::BadNumber(s.to_string()))
}

/// Domain-separated digest of a raw payload. Used for CNF and LRAT blobs.
pub(crate) fn digest_bytes(bytes: &[u8]) -> Digest {
    let mut c = Canon::new("tekmerion/native/blob/v1");
    c.bytes(bytes);
    c.finish()
}

/// Reject non-canonical hex payloads (uppercase, odd length, bad digits).
pub(crate) fn decode_hex(s: &str) -> Result<Vec<u8>, Error> {
    let bytes = s.as_bytes();
    if bytes.len() % 2 != 0 {
        return Err(Error::BadPayloadHex);
    }
    let mut out = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks_exact(2) {
        let hi = hex_digit(pair[0])?;
        let lo = hex_digit(pair[1])?;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

fn hex_digit(b: u8) -> Result<u8, Error> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        _ => Err(Error::BadPayloadHex),
    }
}

pub(crate) fn encode_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(char::from_digit((b >> 4) as u32, 16).unwrap());
        s.push(char::from_digit((b & 0xf) as u32, 16).unwrap());
    }
    s
}
