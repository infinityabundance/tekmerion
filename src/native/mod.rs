//! # Tekmerion Native (TNPS Milestone A)
//!
//! A Rust-native checker for bit-vector arithmetic certificates. Nothing in this
//! module invokes HOL Light, OCaml, a SAT solver, or any external process at check
//! time. A solver may *propose* an LRAT refutation; [`check`] decides.
//!
//! ## Trusted base
//!
//! The checker's answer depends on exactly these things, and on nothing else:
//!
//! 1. **SHA-256** as implemented by the `sha2` crate, used only for content identity.
//!    Hashing never establishes truth; it only names bytes and terms.
//! 2. **The Rust compiler and runtime** executing this code. There is no FFI, no
//!    `unsafe`, no dynamic loading, and no subprocess.
//! 3. **The term grammar and hasher** in [`logic`]: seven constructors
//!    (`true`, `false`, `not`, `eq`, `var`, `const`, `add`, `sub`), fixed widths
//!    `1..=64`, and a length-prefixed, domain-separated digest.
//! 4. **The bit-blaster** in [`bitblast`]: ripple-carry addition, two's-complement
//!    subtraction, bitwise equality, and Tseitin clauses. Its encoding is tested
//!    against exhaustive evaluation at small widths.
//! 5. **The LRAT checker** in [`lrat`]: RUP only. RAT hints are refused. Clause
//!    deletion is honoured. Every addition is checked by unit propagation in hint
//!    order; the final derived clause must be empty.
//! 6. **The certificate calculus** in [`cert`] and [`check`]: exactly five rules,
//!    `refl`, `sym`, `trans`, `congr`, `bitblast(cnf_digest, lrat_digest)`.
//!
//! The trusted base does **not** include CaDiCaL, the CNF producer, or the fixture
//! generator. They only supply candidate bytes. If a candidate CNF differs in one
//! clause from the canonical bit-blast of the claimed proposition, the checker
//! refuses it ([`Error::CnfMismatch`]).
//!
//! ## What is checked, precisely
//!
//! A `bitblast` step closes an arbitrary Boolean proposition `P` by requiring
//!
//! * its attached CNF to be byte-for-byte the canonical DIMACS of `bitblast(P)`,
//! * its attached LRAT to derive the empty clause from that CNF by RUP.
//!
//! The `refl`/`sym`/`trans`/`congr` steps are ordinary equational reasoning over
//! the bit-vector terms. Every step carries its exact conclusion; a rule that does
//! not match the premises is refused rather than repaired.
//!
//! ## Bounds
//!
//! Terms are finite trees, widths are `1..=64`, CNF variables are bounded by
//! `i32::MAX`, and parser inputs are bounded by the allocation they request. There
//! is deliberately no coercion anywhere: an unknown tag, an ambiguous encoding, a
//! missing literal, a dangling reference, or a trailing byte is an error.

use std::fmt;

pub mod bitblast;
pub mod cert;
pub mod check;
pub mod logic;
pub mod lrat;

pub use bitblast::{bitblast, Cnf};
pub use cert::{blob_digest, Blob, BlobKind, Certificate, Rule, Step};
pub use check::{check, Theorem};
pub use logic::{Sort, Term, Value};
pub use lrat::{check as check_lrat, LratError, LratStats};

/// Every way the native checker can refuse input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    // ---- term grammar -----------------------------------------------------
    /// R-N01: a term tag is not in the trusted grammar.
    UnknownTermTag(String),
    /// R-N02: a node has the wrong number of children.
    WrongTermArity {
        tag: String,
        expected: usize,
        found: usize,
    },
    /// R-N03: a child has the wrong sort (Boolean where a word was required, etc.).
    SortMismatch {
        expected: &'static str,
        found: &'static str,
    },
    /// R-N04: a width outside `1..=64`.
    WidthOutOfRange(u32),
    /// R-N05: operands of a binary operator have different widths.
    WidthMismatch { left: u32, right: u32 },
    /// R-N06: a literal does not fit its declared width.
    ConstOutOfRange { width: u32, value: String },
    /// R-N07: a variable name is not canonical.
    BadVarName(String),
    /// R-N08: a numeric field is not canonical (empty, leading zero, overflow).
    BadNumber(String),
    /// R-N09: the input is not the canonical rendering of the parsed term.
    NonCanonicalTerm(String),
    /// R-N10: tokens remain after a complete term.
    TrailingTermTokens,
    /// R-N11: input ended inside a term.
    UnexpectedEof,
    /// R-N12: a free variable has no value during evaluation.
    UnboundVariable(String),

    // ---- certificate container -------------------------------------------
    /// R-N20: not a native certificate.
    BadCertificateMagic,
    /// R-N21: the certificate version is not understood.
    UnsupportedCertificateVersion(String),
    /// R-N22: the certificate ends before a required line.
    Truncated,
    /// R-N23: data follows the `end` line, or an unexpected record appears.
    TrailingGarbage,
    /// R-N24: a record tag is not part of the format.
    UnknownRecord(String),
    /// R-N25: a record is structurally malformed.
    MalformedCertificate(&'static str),
    /// R-N26: a rule name is not in the trusted set.
    UnknownRule(String),
    /// R-N27: two steps share an id.
    DuplicateStep(u32),
    /// R-N28: a premise id names no step.
    DanglingRef(u32),
    /// R-N29: the premise graph contains a cycle.
    Cycle(u32),
    /// R-N30: a rule was given the wrong number of premises.
    WrongPremiseCount {
        rule: &'static str,
        expected: usize,
        found: usize,
    },
    /// R-N31: the conclusion does not have the form the rule requires, or the
    /// premises do not entail the stated conclusion.
    RuleMismatch(&'static str),
    /// R-N32: two blobs share a digest.
    DuplicateBlob(crate::Digest),
    /// R-N33: a blob's declared digest does not name its bytes.
    BlobDigestMismatch(crate::Digest),
    /// R-N34: a blob's declared length does not match its payload.
    BlobLengthMismatch { declared: usize, actual: usize },
    /// R-N35: a `bitblast` step names a blob that is not present.
    MissingBlob(crate::Digest),
    /// R-N36: a blob is present but no step references it.
    UnknownBlob(crate::Digest),
    /// R-N37: a blob is referenced under the wrong kind (CNF vs LRAT).
    BlobKindMismatch {
        expected: &'static str,
        found: &'static str,
    },
    /// R-N38: the declared theorem step does not exist.
    MissingTheorem(u32),
    /// R-N39: the input is parseable but not the canonical rendering.
    NonCanonicalCertificate,
    /// R-N40: a hexadecimal payload is not canonical lowercase hex.
    BadPayloadHex,

    // ---- CNF and checking -------------------------------------------------
    /// R-N50: the attached CNF is not canonical DIMACS.
    BadDimacs(String),
    /// R-N51: the attached CNF is not the canonical bit-blast of the conclusion.
    CnfMismatch,
    /// R-N52: an LRAT refutation was refused.
    Lrat(LratError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::UnknownTermTag(t) => write!(f, "R-N01: unknown term tag {t:?}"),
            Error::WrongTermArity {
                tag,
                expected,
                found,
            } => write!(
                f,
                "R-N02: term {tag:?} expects {expected} children, found {found}"
            ),
            Error::SortMismatch { expected, found } => {
                write!(f, "R-N03: expected {expected}, found {found}")
            }
            Error::WidthOutOfRange(w) => write!(f, "R-N04: width {w} is outside 1..=64"),
            Error::WidthMismatch { left, right } => {
                write!(f, "R-N05: width mismatch {left} vs {right}")
            }
            Error::ConstOutOfRange { width, value } => {
                write!(f, "R-N06: literal {value} does not fit bv{width}")
            }
            Error::BadVarName(n) => write!(f, "R-N07: non-canonical variable name {n:?}"),
            Error::BadNumber(n) => write!(f, "R-N08: non-canonical number {n:?}"),
            Error::NonCanonicalTerm(t) => write!(f, "R-N09: non-canonical term {t:?}"),
            Error::TrailingTermTokens => write!(f, "R-N10: trailing tokens after term"),
            Error::UnexpectedEof => write!(f, "R-N11: input ended inside a term"),
            Error::UnboundVariable(v) => write!(f, "R-N12: unbound variable {v:?}"),
            Error::BadCertificateMagic => write!(f, "R-N20: not a native certificate"),
            Error::UnsupportedCertificateVersion(v) => {
                write!(f, "R-N21: unsupported certificate version {v:?}")
            }
            Error::Truncated => write!(f, "R-N22: certificate is truncated"),
            Error::TrailingGarbage => write!(f, "R-N23: trailing garbage"),
            Error::UnknownRecord(r) => write!(f, "R-N24: unknown record {r:?}"),
            Error::MalformedCertificate(m) => write!(f, "R-N25: malformed certificate: {m}"),
            Error::UnknownRule(r) => write!(f, "R-N26: unknown rule {r:?}"),
            Error::DuplicateStep(id) => write!(f, "R-N27: duplicate step id {id}"),
            Error::DanglingRef(id) => write!(f, "R-N28: dangling step reference {id}"),
            Error::Cycle(id) => write!(f, "R-N29: premise cycle through step {id}"),
            Error::WrongPremiseCount {
                rule,
                expected,
                found,
            } => write!(
                f,
                "R-N30: rule {rule} expects {expected} premises, found {found}"
            ),
            Error::RuleMismatch(m) => write!(f, "R-N31: {m}"),
            Error::DuplicateBlob(d) => write!(f, "R-N32: duplicate blob {d}"),
            Error::BlobDigestMismatch(d) => write!(f, "R-N33: blob digest mismatch for {d}"),
            Error::BlobLengthMismatch { declared, actual } => write!(
                f,
                "R-N34: blob length {declared} does not match payload {actual}"
            ),
            Error::MissingBlob(d) => write!(f, "R-N35: missing blob {d}"),
            Error::UnknownBlob(d) => write!(f, "R-N36: unreferenced blob {d}"),
            Error::BlobKindMismatch { expected, found } => {
                write!(
                    f,
                    "R-N37: blob kind mismatch: expected {expected}, found {found}"
                )
            }
            Error::MissingTheorem(id) => write!(f, "R-N38: theorem step {id} does not exist"),
            Error::NonCanonicalCertificate => write!(f, "R-N39: non-canonical certificate"),
            Error::BadPayloadHex => write!(f, "R-N40: payload is not canonical lowercase hex"),
            Error::BadDimacs(m) => write!(f, "R-N50: bad DIMACS: {m}"),
            Error::CnfMismatch => {
                write!(
                    f,
                    "R-N51: CNF is not the canonical bit-blast of the conclusion"
                )
            }
            Error::Lrat(e) => write!(f, "R-N52: {e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<LratError> for Error {
    fn from(e: LratError) -> Self {
        Error::Lrat(e)
    }
}
