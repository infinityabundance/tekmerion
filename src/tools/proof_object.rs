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
//! | `IMPORT` | 0 | the whole sequent, which must match an entry in an
//! external [`CheckedTheoremStore`] (a pre-recording theorem) |
//!
//! # What is hashed
//!
//! * `statement_digest` — SHA-256 of a canonical, alpha-normalised encoding of
//!   the root conclusion. Bound variables become de Bruijn indices, so
//!   alpha-renaming cannot change it.
//! * `statement_id` — SHA-256 over the *theory digest* plus the canonical
//!   hypotheses and conclusion. It binds the statement to the theory
//!   environment, so the same term under a different axiom/definition/import
//!   set is a different statement (`G1.5`).
//! * `theory_digest` — SHA-256 of a canonical [`ValidatedTheory`] manifest.
//! * `store_digest` — SHA-256 of the canonical [`CheckedTheoremStore`] file.
//!   The theory digest includes it, so the import environment is bound too.
//! * `derivation_digest` — SHA-256 of every byte of the object before the
//!   trailer. It detects truncation and corruption, not forgery.
//! * import and axiom statement digests — SHA-256 of the same canonical
//!   encoding, so a reader can list exactly what was assumed.
//!
//! # What is deliberately not recorded
//!
//! Tactics, strategy, timing, failed branches of proof search, and the
//! derivations of imported lemmas. The last is the real trust boundary: an
//! `IMPORT` node is an assumption about the pinned library. See
//! `research/receipts/ix84-export-and-cap.md`.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::File;
use std::io::{BufWriter, Read, Write};
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
    /// An `IMPORT` names a sequent that is absent from the external store.
    ImportNotJustified {
        digest: String,
    },
    /// An `Axiom` node is not in the validated theory's authorised set.
    AxiomNotAuthorised {
        digest: String,
    },
    /// A `Definition` node is not in the validated theory's authorised set.
    DefinitionNotAuthorised {
        digest: String,
    },
    /// A definition rebinds a constant that already exists (`G1.3`).
    DefinitionNotFresh {
        name: String,
    },
    /// A type definition is not in the validated theory's extension registry.
    TypeDefinitionNotAuthorised {
        tyname: String,
    },
    /// A type definition introduces a type constructor that already exists.
    TypeNotFresh {
        name: String,
    },
    /// The checked-theorem store file is malformed.
    StoreMalformed(String),
    /// The object was checked against a different theory environment.
    TheoryDigestMismatch {
        expected: String,
        got: String,
    },
    /// The supplied store does not hash to the theory's pinned store digest.
    StoreDigestMismatch {
        expected: String,
        got: String,
    },
    /// The replayed statement identity differs from the expected one.
    StatementIdentityMismatch {
        expected: String,
        got: String,
    },
    /// Replay exceeded its node budget before finishing.
    BudgetExceeded {
        budget: u64,
        reached: u64,
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
            Error::ImportNotJustified { digest } => write!(
                f,
                "refused: IMPORT sequent {digest} is not justified by the checked-theorem store"
            ),
            Error::AxiomNotAuthorised { digest } => write!(
                f,
                "refused: axiom {digest} is not authorised by the validated theory"
            ),
            Error::DefinitionNotAuthorised { digest } => write!(
                f,
                "refused: definition {digest} is not authorised by the validated theory"
            ),
            Error::DefinitionNotFresh { name } => {
                write!(f, "refused: definition rebinds existing constant {name}")
            }
            Error::TypeDefinitionNotAuthorised { tyname } => write!(
                f,
                "refused: type definition {tyname} is not in the theory's extension registry"
            ),
            Error::TypeNotFresh { name } => {
                write!(f, "refused: type definition rebinds existing type {name}")
            }
            Error::StoreMalformed(m) => write!(f, "store: malformed: {m}"),
            Error::TheoryDigestMismatch { expected, got } => write!(
                f,
                "refused: theory digest {got} does not match the expected {expected}"
            ),
            Error::StoreDigestMismatch { expected, got } => write!(
                f,
                "refused: store digest {got} does not match the theory's pinned {expected}"
            ),
            Error::StatementIdentityMismatch { expected, got } => write!(
                f,
                "refused: replayed statement identity {got} differs from expected {expected}"
            ),
            Error::BudgetExceeded { budget, reached } => write!(
                f,
                "refused: replay reached node {reached}, over its budget of {budget}"
            ),
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
    pub statement_id: String,
    pub theory_digest: String,
    pub store_digest: String,
    pub derivation_digest: String,
    pub node_count: u64,
    pub raw_nodes: u64,
    pub bytes: u64,
    pub root_id: u64,
    pub inference_total: u64,
    pub inference_begin: u64,
    pub imports: u64,
    pub axioms: Vec<String>,
    pub import_digests: Vec<String>,
    /// Statement digests of the reachable `new_basic_definition` nodes.  The
    /// exported theory authorises exactly these (`G1.3`).
    pub definitions: Vec<String>,
    /// Reachable `new_basic_type_definition` registries (`G1.4`).
    pub type_definitions: Vec<String>,
    /// Largest single payload buffer held while streaming finalisation.  A
    /// frontier-bounded finaliser keeps this proportional to one record, not
    /// to the object (`G1.6`).
    pub peak_buffered_payload_bytes: u64,
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
    pub statement_id: String,
    pub theory_digest: String,
    pub store_digest: String,
    pub derivation_digest: String,
    pub node_count: u64,
    pub reachable: u64,
    pub raw_nodes: u64,
    pub inference_total: u64,
    pub inference_begin: u64,
    pub imports: u64,
    pub axioms: Vec<String>,
    /// Peak number of theorems held at once during replay.
    pub peak_live_nodes: u64,
    /// The node ceiling supplied to replay (the second guard, after the
    /// frontier bound, on wall time and memory).
    pub budget: u64,
    pub replay_mode: &'static str,
}

// ---------------------------------------------------------------------------
// The external environment: a checked theorem store and a validated theory
// ---------------------------------------------------------------------------

/// A store of theorems whose statements were established **outside** the
/// certificate under inspection (`G1.1`).  An `IMPORT` node is accepted only
/// if its sequent is present here; the checker uses the stored sequent, not the
/// object's payload, as the theorem.  The file is canonicalised and its digest
/// is pinned by the [`ValidatedTheory`].
#[derive(Clone, Debug)]
pub struct CheckedTheoremStore {
    entries: BTreeMap<[u8; 32], Thm>,
    digest: [u8; 32],
}

impl CheckedTheoremStore {
    pub fn empty() -> Self {
        let mut s = CheckedTheoremStore {
            entries: BTreeMap::new(),
            digest: [0u8; 32],
        };
        s.digest = sha256(&s.render());
        s
    }

    pub fn from_theorems(theorems: &[Thm]) -> Self {
        let mut s = CheckedTheoremStore {
            entries: BTreeMap::new(),
            digest: [0u8; 32],
        };
        for t in theorems {
            s.entries
                .insert(sequent_digest(&t.hyps, &t.concl), t.clone());
        }
        s.digest = sha256(&s.render());
        s
    }

    /// Parse the canonical store file written by [`write_store`].
    pub fn load(path: &Path) -> Result<Self, Error> {
        let bytes = std::fs::read(path)?;
        Self::parse(&bytes)
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        if !bytes.starts_with(STORE_MAGIC) {
            return Err(Error::StoreMalformed("bad magic".into()));
        }
        let mut c = Cursor::new(&bytes[STORE_MAGIC.len()..]);
        let n = c.u32()? as usize;
        let mut entries = BTreeMap::new();
        for _ in 0..n {
            let nh = c.u32()? as usize;
            let mut hyps = Vec::with_capacity(nh);
            for _ in 0..nh {
                hyps.push(decode_raw_term(&mut c)?);
            }
            let concl = decode_raw_term(&mut c)?;
            let d = sequent_digest(&hyps, &concl);
            if entries.insert(d, Thm { hyps, concl }).is_some() {
                return Err(Error::StoreMalformed(
                    "duplicate sequent digest with distinct content".into(),
                ));
            }
        }
        if c.remaining() != 0 {
            return Err(Error::StoreMalformed("trailing bytes".into()));
        }
        let mut s = CheckedTheoremStore {
            entries,
            digest: [0; 32],
        };
        let rendered = s.render();
        if rendered != bytes {
            return Err(Error::StoreMalformed(
                "file is not in canonical (sorted) form".into(),
            ));
        }
        s.digest = sha256(&rendered);
        Ok(s)
    }

    fn render(&self) -> Vec<u8> {
        let mut keys: Vec<&[u8; 32]> = self.entries.keys().collect();
        keys.sort();
        let mut out = Vec::new();
        out.extend_from_slice(STORE_MAGIC);
        put_u32(&mut out, keys.len() as u32);
        for k in keys {
            let t = &self.entries[k];
            put_u32(&mut out, t.hyps.len() as u32);
            for h in &t.hyps {
                put_raw_term(&mut out, h);
            }
            put_raw_term(&mut out, &t.concl);
        }
        out
    }

    pub fn digest(&self) -> [u8; 32] {
        self.digest
    }

    pub fn digest_hex(&self) -> String {
        hex32(&self.digest)
    }

    pub fn get(&self, d: &[u8; 32]) -> Option<&Thm> {
        self.entries.get(d)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// The authoritative environment supplied to the checker (`G1.2`–`G1.5`).
///
/// The object may *declare* an axiom or definition set, but only this value
/// decides what is permitted.  Its canonical rendering is hashed into every
/// statement identity, so the same term under a different theory is a
/// different statement.
#[derive(Clone, Debug, Default)]
pub struct ValidatedTheory {
    pub axioms: BTreeSet<[u8; 32]>,
    pub definitions: BTreeSet<[u8; 32]>,
    pub constants: BTreeSet<String>,
    pub types: BTreeSet<String>,
    /// Authorised type extensions: `(type, abstraction constant, representation
    /// constant)` (`G1.4`).
    pub type_definitions: BTreeSet<(String, String, String)>,
    /// Digest of the [`CheckedTheoremStore`] the theory binds.
    pub store_digest: [u8; 32],
    digest: [u8; 32],
}

impl ValidatedTheory {
    pub fn new(
        axioms: BTreeSet<[u8; 32]>,
        definitions: BTreeSet<[u8; 32]>,
        constants: BTreeSet<String>,
        types: BTreeSet<String>,
        type_definitions: BTreeSet<(String, String, String)>,
        store_digest: [u8; 32],
    ) -> Self {
        let mut t = ValidatedTheory {
            axioms,
            definitions,
            constants,
            types,
            type_definitions,
            store_digest,
            digest: [0u8; 32],
        };
        t.digest = sha256(&t.render());
        t
    }

    pub fn with_store(store: &CheckedTheoremStore) -> Self {
        Self::new(
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            store.digest(),
        )
    }

    /// Canonical manifest text; the digest is the hash of exactly these bytes.
    pub fn render(&self) -> Vec<u8> {
        let mut s = String::from("TEKMERION-THEORY-V1\n");
        s.push_str(&format!("store {}\n", hex32(&self.store_digest)));
        let mut ax: Vec<String> = self.axioms.iter().map(hex32).collect();
        ax.sort();
        for d in ax {
            s.push_str(&format!("axiom {d}\n"));
        }
        let mut df: Vec<String> = self.definitions.iter().map(hex32).collect();
        df.sort();
        for d in df {
            s.push_str(&format!("definition {d}\n"));
        }
        for c in &self.constants {
            s.push_str(&format!("constant {c}\n"));
        }
        for t in &self.types {
            s.push_str(&format!("type {t}\n"));
        }
        for (t, a, r) in &self.type_definitions {
            s.push_str(&format!("typedef {t} {a} {r}\n"));
        }
        s.into_bytes()
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        let text =
            std::str::from_utf8(bytes).map_err(|_| Error::Malformed("theory UTF-8".into()))?;
        let mut t = ValidatedTheory::default();
        let mut seen_header = false;
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if !seen_header {
                if line != "TEKMERION-THEORY-V1" {
                    return Err(Error::Malformed("theory header".into()));
                }
                seen_header = true;
                continue;
            }
            let mut it = line.splitn(2, ' ');
            let key = it.next().unwrap_or("");
            let rest = it.next().unwrap_or("").trim();
            match key {
                "store" => {
                    t.store_digest = parse_hex32(rest)?;
                }
                "axiom" => {
                    t.axioms.insert(parse_hex32(rest)?);
                }
                "definition" => {
                    t.definitions.insert(parse_hex32(rest)?);
                }
                "constant" => {
                    t.constants.insert(rest.to_string());
                }
                "type" => {
                    t.types.insert(rest.to_string());
                }
                "typedef" => {
                    let parts: Vec<&str> = rest.split_whitespace().collect();
                    if parts.len() != 3 {
                        return Err(Error::Malformed("typedef line".into()));
                    }
                    t.type_definitions.insert((
                        parts[0].to_string(),
                        parts[1].to_string(),
                        parts[2].to_string(),
                    ));
                }
                other => return Err(Error::Malformed(format!("theory key {other}"))),
            }
        }
        if !seen_header {
            return Err(Error::Malformed("empty theory".into()));
        }
        t.digest = sha256(&t.render());
        Ok(t)
    }

    pub fn digest(&self) -> [u8; 32] {
        self.digest
    }

    pub fn digest_hex(&self) -> String {
        hex32(&self.digest)
    }

    pub fn permits_axiom(&self, d: &[u8; 32]) -> bool {
        self.axioms.contains(d)
    }

    pub fn permits_definition(&self, d: &[u8; 32]) -> bool {
        self.definitions.contains(d)
    }

    pub fn permits_type_definition(&self, ty: &str, abs: &str, rep: &str) -> bool {
        self.type_definitions
            .contains(&(ty.to_string(), abs.to_string(), rep.to_string()))
    }
}

/// The statement a certificate is expected to prove, bound to the theory.
#[derive(Clone, Debug)]
pub struct Expected {
    pub name: String,
    pub theory_digest: [u8; 32],
    pub statement_id: [u8; 32],
}

impl Expected {
    pub fn compute(
        theory: &ValidatedTheory,
        name: &str,
        hyps: &[TermRef],
        concl: &TermRef,
    ) -> Self {
        Expected {
            name: name.to_string(),
            theory_digest: theory.digest(),
            statement_id: statement_identity(&theory.digest(), hyps, concl),
        }
    }

    pub fn render(&self) -> String {
        format!(
            "TEKMERION-EXPECTED-V1\nname {}\ntheory {}\nstatement {}\n",
            self.name,
            hex32(&self.theory_digest),
            hex32(&self.statement_id)
        )
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        let text =
            std::str::from_utf8(bytes).map_err(|_| Error::Malformed("expected UTF-8".into()))?;
        let mut name = None;
        let mut theory = None;
        let mut statement = None;
        for line in text.lines() {
            let mut it = line.splitn(2, ' ');
            let key = it.next().unwrap_or("");
            let rest = it.next().unwrap_or("").trim();
            match key {
                "TEKMERION-EXPECTED-V1" => {}
                "name" => name = Some(rest.to_string()),
                "theory" => theory = Some(parse_hex32(rest)?),
                "statement" => statement = Some(parse_hex32(rest)?),
                _ => {}
            }
        }
        Ok(Expected {
            name: name.ok_or_else(|| Error::Malformed("expected: no name".into()))?,
            theory_digest: theory.ok_or_else(|| Error::Malformed("expected: no theory".into()))?,
            statement_id: statement
                .ok_or_else(|| Error::Malformed("expected: no statement".into()))?,
        })
    }

    pub fn load(path: &Path) -> Result<Self, Error> {
        Self::parse(&std::fs::read(path)?)
    }
}

fn parse_hex32(s: &str) -> Result<[u8; 32], Error> {
    if s.len() != 64 {
        return Err(Error::Malformed("expected 64 hex characters".into()));
    }
    let mut out = [0u8; 32];
    for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
        let hi = (chunk[0] as char)
            .to_digit(16)
            .ok_or_else(|| Error::Malformed("bad hex".into()))?;
        let lo = (chunk[1] as char)
            .to_digit(16)
            .ok_or_else(|| Error::Malformed("bad hex".into()))?;
        out[i] = ((hi << 4) | lo) as u8;
    }
    Ok(out)
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
            // HOL Light's `rev_assocd ty i ty` matches the *second* component
            // of each pair and returns the first: pairs are (replacement, var).
            for (repl, var) in tyin {
                if **var == key {
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
        // HOL Light pairs are (value, key): match the second, return the first.
        for (value, k) in env {
            if teq(k, key) {
                return value.clone();
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

fn canon_hex(t: &TermRef) -> String {
    let mut out = Vec::new();
    canon_term(&mut out, t, &mut Vec::new());
    out.iter().map(|b| format!("{b:02x}")).collect()
}

/// Digest of a whole sequent (hypotheses and conclusion), used as the key of
/// the external theorem store.
fn sequent_digest(hyps: &[TermRef], concl: &TermRef) -> [u8; 32] {
    let mut out = Vec::new();
    out.extend_from_slice(b"HLSEQ1\n");
    put_u32(&mut out, hyps.len() as u32);
    let mut binders = Vec::new();
    for h in hyps {
        canon_term(&mut out, h, &mut binders);
    }
    canon_term(&mut out, concl, &mut binders);
    sha256(&out)
}

/// Statement identity: the term **plus the complete theory environment**
/// (`G1.5`).  Callers pass the digest of a [`ValidatedTheory`].
pub fn statement_identity(theory_digest: &[u8; 32], hyps: &[TermRef], concl: &TermRef) -> [u8; 32] {
    let mut out = Vec::new();
    out.extend_from_slice(b"HLSTMT1\n");
    out.extend_from_slice(theory_digest);
    put_u32(&mut out, hyps.len() as u32);
    let mut binders = Vec::new();
    for h in hyps {
        canon_term(&mut out, h, &mut binders);
    }
    canon_term(&mut out, concl, &mut binders);
    sha256(&out)
}

/// Raw structural term encoding, matching the recording kernel's stream.  Used
/// for store files.
fn put_raw_type(out: &mut Vec<u8>, ty: &Type) {
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
                put_raw_type(out, a);
            }
        }
    }
}

fn put_raw_term(out: &mut Vec<u8>, t: &TermRef) {
    match &**t {
        Term::Var(n, ty) => {
            put_u8(out, 0);
            put_str(out, n);
            put_raw_type(out, ty);
        }
        Term::Const(n, ty) => {
            put_u8(out, 1);
            put_str(out, n);
            put_raw_type(out, ty);
        }
        Term::Comb(a, b) => {
            put_u8(out, 2);
            put_raw_term(out, a);
            put_raw_term(out, b);
        }
        Term::Abs(v, b) => {
            put_u8(out, 3);
            put_raw_term(out, v);
            put_raw_term(out, b);
        }
    }
}

// ---------------------------------------------------------------------------
// Raw stream scan
// ---------------------------------------------------------------------------

const RAW_MAGIC: &[u8] = b"HLRAW1\n";
const STORE_MAGIC: &[u8] = b"HLSTORE1\n";
const OBJ_MAGIC: &[u8] = b"HLPROOF1\n";
const OBJ_END: &[u8] = b"HLEND1\n";
const OBJ_VERSION: u8 = 2;
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
// Finalisation: raw stream -> compact object (streamed)
// ---------------------------------------------------------------------------

/// Mark the nodes reachable from the recorded root.  Only parent ids are
/// touched, so this is a metadata walk, not a payload walk.
fn mark_reachable(scan: &RawScan) -> Result<Vec<bool>, Error> {
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

/// Decode an IMPORT payload as a full sequent (raw structural encoding).
fn decode_import(raw: &[u8]) -> Result<(Vec<TermRef>, TermRef), Error> {
    let mut c = Cursor::new(raw);
    let nh = c.u32()? as usize;
    let mut hyps = Vec::with_capacity(nh);
    for _ in 0..nh {
        hyps.push(decode_raw_term(&mut c)?);
    }
    let concl = decode_raw_term(&mut c)?;
    if c.remaining() != 0 {
        return Err(Error::Malformed("import payload trailing bytes".into()));
    }
    Ok((hyps, concl))
}

/// A writer that hashes exactly the bytes it forwards.
struct DigestWriter<W: Write> {
    inner: W,
    hasher: Sha256,
}

impl<W: Write> DigestWriter<W> {
    fn new(inner: W) -> Self {
        DigestWriter {
            inner,
            hasher: Sha256::new(),
        }
    }
    fn write_u8(&mut self, b: u8) -> Result<(), Error> {
        self.write_all(&[b])?;
        Ok(())
    }
    fn write_u32(&mut self, n: u32) -> Result<(), Error> {
        self.write_all(&n.to_be_bytes())?;
        Ok(())
    }
    fn write_u64(&mut self, n: u64) -> Result<(), Error> {
        self.write_all(&n.to_be_bytes())?;
        Ok(())
    }
    fn write_str(&mut self, s: &str) -> Result<(), Error> {
        self.write_u32(s.len() as u32)?;
        self.write_all(s.as_bytes())?;
        Ok(())
    }
    fn finish(self) -> [u8; 32] {
        let out = self.hasher.finalize();
        let mut a = [0u8; 32];
        a.copy_from_slice(&out);
        a
    }
}

impl<W: Write> Write for DigestWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.hasher.update(&buf[..n]);
        Ok(n)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

fn write_type_table<W: Write>(out: &mut DigestWriter<W>, it: &Interner) -> Result<(), Error> {
    out.write_u32(it.types.len() as u32)?;
    for k in &it.types {
        match k {
            TypeKey::Var(v) => {
                out.write_u8(0)?;
                out.write_str(v)?;
            }
            TypeKey::App(f, args) => {
                out.write_u8(1)?;
                out.write_str(f)?;
                out.write_u32(args.len() as u32)?;
                for a in args {
                    out.write_u32(*a)?;
                }
            }
        }
    }
    Ok(())
}

fn write_term_table<W: Write>(out: &mut DigestWriter<W>, it: &Interner) -> Result<(), Error> {
    out.write_u32(it.terms.len() as u32)?;
    for k in &it.terms {
        match k {
            TermKey::Var(nm, ty) => {
                out.write_u8(0)?;
                out.write_str(nm)?;
                out.write_u32(*ty)?;
            }
            TermKey::Const(nm, ty) => {
                out.write_u8(1)?;
                out.write_str(nm)?;
                out.write_u32(*ty)?;
            }
            TermKey::Comb(a, b) => {
                out.write_u8(2)?;
                out.write_u32(*a)?;
                out.write_u32(*b)?;
            }
            TermKey::Abs(a, b) => {
                out.write_u8(3)?;
                out.write_u32(*a)?;
                out.write_u32(*b)?;
            }
        }
    }
    Ok(())
}

/// Finalise a raw stream into a compact object.
///
/// The input is read sequentially; payloads are held one at a time and the
/// output is written as it is produced (`G1.6`).  The only per-node state kept
/// is the parent/offset index the scanner already needs, a reach bit, and the
/// old->new id map.
///
/// Side artefacts:
/// * `store_path` — the canonical [`CheckedTheoremStore`] built from the
///   reachable IMPORT leaves; its digest is bound into the theory.
/// * `expected_out` — the statement the kernel recorded, bound to the theory
///   digest; the replay must reconstruct exactly this (`G1.5`).
pub fn finalize(
    raw_path: &Path,
    obj_path: &Path,
    store_path: Option<&Path>,
    theory_in: Option<&ValidatedTheory>,
    expected_out: Option<&Path>,
    theory_out: Option<&Path>,
) -> Result<Summary, Error> {
    let scan = scan_raw(raw_path)?;
    let reach = mark_reachable(&scan)?;
    let n = scan.nodes.len();
    let mut it = Interner::default();
    let f = File::open(raw_path)?;
    let mut peak_buf: u64 = 0;

    // Pass 1: intern every reachable payload, collect the IMPORT sequents and
    // the Axiom statement digests.  Each buffer is dropped before the next.
    let mut import_seqs: Vec<Thm> = Vec::new();
    let mut import_set: BTreeSet<[u8; 32]> = BTreeSet::new();
    let mut axiom_digests: BTreeSet<[u8; 32]> = BTreeSet::new();
    let mut definition_digests: BTreeSet<[u8; 32]> = BTreeSet::new();
    let mut type_definitions: BTreeSet<(String, String, String)> = BTreeSet::new();
    for (id, m) in scan.nodes.iter().enumerate() {
        if !reach[id] {
            continue;
        }
        let raw = read_payload(&f, m)?;
        peak_buf = peak_buf.max(raw.len() as u64);
        let compact = compact_payload(m.rule, &raw, &mut it)?;
        peak_buf = peak_buf.max(compact.len() as u64);
        match m.rule {
            Rule::Import => {
                let (hyps, concl) = decode_import(&raw)?;
                let d = sequent_digest(&hyps, &concl);
                if import_set.insert(d) {
                    import_seqs.push(Thm { hyps, concl });
                }
            }
            Rule::Axiom => {
                // Computed exactly as the checker computes it: on the term the
                // rule produces, not on a table id.
                let t = decode_raw_term(&mut Cursor::new(&raw))?;
                axiom_digests.insert(statement_digest(&t));
            }
            Rule::Definition => {
                // The rule turns `Var(c,ty) = r` into `Const(c,ty) = r`; the
                // authorised digest is that of the latter (`G1.3`).
                let t = decode_raw_term(&mut Cursor::new(&raw))?;
                let (l, r) = dest_eq(&t)
                    .ok_or_else(|| Error::Malformed("definition is not an equation".into()))?;
                let (cname, cty) = match &*l {
                    Term::Var(n, ty) => (n.clone(), ty.clone()),
                    _ => {
                        return Err(Error::Malformed(
                            "definition left side is not a variable".into(),
                        ))
                    }
                };
                let c2 = Rc::new(Term::Const(cname, cty));
                let concl = mk_eq(c2, r)?;
                definition_digests.insert(statement_digest(&concl));
            }
            Rule::TypeDefAbs | Rule::TypeDefRep => {
                let mut c = Cursor::new(&raw);
                let _which = c.u8()?;
                type_definitions.insert((c.str()?, c.str()?, c.str()?));
            }
            _ => {}
        }
    }

    // The external store is written out and its digest pinned by the theory.
    let store = CheckedTheoremStore::from_theorems(&import_seqs);
    if let Some(p) = store_path {
        std::fs::write(p, store.render())?;
    }
    let owned_theory;
    let theory: &ValidatedTheory = match theory_in {
        Some(t) => {
            if t.store_digest != store.digest() {
                return Err(Error::StoreDigestMismatch {
                    expected: hex32(&t.store_digest),
                    got: store.digest_hex(),
                });
            }
            t
        }
        None => {
            // The environment is the one this derivation establishes: the
            // reachable axioms/definitions/type extensions it introduces, and
            // the store of pre-recording sequents it imports.
            owned_theory = ValidatedTheory::new(
                axiom_digests.clone(),
                definition_digests.clone(),
                BTreeSet::new(),
                BTreeSet::new(),
                type_definitions.clone(),
                store.digest(),
            );
            &owned_theory
        }
    };
    if let Some(p) = theory_out {
        std::fs::write(p, theory.render())?;
    }
    let theory_digest = theory.digest();

    // The kernel's own footer statement, bound to the theory environment.
    let stmt = intern_rc_term(&mut it, &scan.footer.concl)?;
    let mut hyp_ids = Vec::with_capacity(scan.footer.hyps.len());
    for h in &scan.footer.hyps {
        hyp_ids.push(intern_rc_term(&mut it, h)?);
    }
    let stmt_digest = it.digest_term(stmt);
    let statement_id = statement_identity(&theory_digest, &scan.footer.hyps, &scan.footer.concl);
    if let Some(p) = expected_out {
        let e = Expected {
            name: scan.footer.name.clone(),
            theory_digest,
            statement_id,
        };
        std::fs::write(p, e.render())?;
    }

    // Reachable nodes are emitted in ascending id order (parents precede
    // children), so the compact ids are just ranks.
    let mut newid = vec![u32::MAX; n];
    let mut emitted_count: u32 = 0;
    for (id, r) in reach.iter().enumerate() {
        if *r {
            newid[id] = emitted_count;
            emitted_count += 1;
        }
    }
    let root_new = newid[scan.footer.root as usize];
    if root_new == u32::MAX || root_new as usize + 1 != emitted_count as usize {
        return Err(Error::RootNotLast);
    }

    let raw_file = File::create(obj_path)?;
    let mut out = DigestWriter::new(BufWriter::new(raw_file));

    out.write_all(OBJ_MAGIC)?;
    out.write_u8(OBJ_VERSION)?;
    out.write_str(&scan.footer.name)?;
    out.write_u32(n as u32)?;
    out.write_u64(scan.footer.inference_total)?;
    out.write_u64(scan.footer.inference_begin)?;
    out.write_u32(root_new)?;
    out.write_all(&theory_digest)?;
    out.write_all(&statement_id)?;
    out.write_u32(stmt)?;
    out.write_u32(hyp_ids.len() as u32)?;
    for h in &hyp_ids {
        out.write_u32(*h)?;
    }
    out.write_u32(axiom_digests.len() as u32)?;
    for d in &axiom_digests {
        out.write_all(d)?;
    }
    out.write_u32(import_set.len() as u32)?;
    for d in &import_set {
        out.write_all(d)?;
    }
    out.write_u32(scan.footer.axioms.len() as u32)?;
    for (id, dg, txt) in &scan.footer.axioms {
        out.write_u32(*id)?;
        out.write_str(dg)?;
        out.write_str(txt)?;
    }
    write_type_table(&mut out, &it)?;
    write_term_table(&mut out, &it)?;

    // Node table: streamed straight out, one payload at a time.
    out.write_u32(emitted_count)?;
    for (id, m) in scan.nodes.iter().enumerate() {
        if !reach[id] {
            continue;
        }
        let raw = read_payload(&f, m)?;
        peak_buf = peak_buf.max(raw.len() as u64);
        let payload = compact_payload(m.rule, &raw, &mut it)?;
        peak_buf = peak_buf.max(payload.len() as u64);
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
        out.write_u8(m.rule as u8)?;
        out.write_u32(p0)?;
        out.write_u32(p1)?;
        out.write_u32(payload.len() as u32)?;
        out.write_all(&payload)?;
    }

    let file_digest = out.finish();
    let mut file = std::fs::OpenOptions::new().append(true).open(obj_path)?;
    file.write_all(&file_digest)?;
    file.write_all(OBJ_END)?;
    let total_bytes = std::fs::metadata(obj_path)?.len();

    Ok(Summary {
        theorem: scan.footer.name.clone(),
        statement_digest: hex32(&stmt_digest),
        statement_id: hex32(&statement_id),
        theory_digest: hex32(&theory_digest),
        store_digest: store.digest_hex(),
        derivation_digest: hex32(&file_digest),
        node_count: emitted_count as u64,
        raw_nodes: n as u64,
        bytes: total_bytes,
        root_id: root_new as u64,
        inference_total: scan.footer.inference_total,
        inference_begin: scan.footer.inference_begin,
        imports: import_set.len() as u64,
        axioms: scan
            .footer
            .axioms
            .iter()
            .map(|(_, d, _)| d.clone())
            .collect(),
        import_digests: import_set.iter().map(hex32).collect(),
        definitions: definition_digests.iter().map(hex32).collect(),
        type_definitions: type_definitions
            .iter()
            .map(|(t, a, r)| format!("{t}/{a}/{r}"))
            .collect(),
        peak_buffered_payload_bytes: peak_buf,
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
    while let Ok(tag) = r.u8() {
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
    theory_digest: [u8; 32],
    statement_id: [u8; 32],
    hyps: Vec<TermRef>,
    axiom_digests: BTreeSet<[u8; 32]>,
    import_digests: BTreeSet<[u8; 32]>,
    axioms: Vec<(u32, String, String)>,
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

/// A reader over the object file that hashes exactly the body (every byte
/// before the digest trailer).  Tables are decoded as they arrive, so the file
/// is never held in memory as a whole.
struct HashReader {
    f: std::io::BufReader<File>,
    hasher: Sha256,
    pos: u64,
    hash_limit: u64,
}

impl HashReader {
    fn open(path: &Path, hash_limit: u64) -> Result<Self, Error> {
        Ok(HashReader {
            f: std::io::BufReader::new(File::open(path)?),
            hasher: Sha256::new(),
            pos: 0,
            hash_limit,
        })
    }

    fn feed(&mut self, b: &[u8]) {
        if self.pos < self.hash_limit {
            let upto = (self.pos + b.len() as u64).min(self.hash_limit);
            let take = (upto - self.pos) as usize;
            self.hasher.update(&b[..take]);
        }
        self.pos += b.len() as u64;
    }

    fn exact(&mut self, n: usize) -> Result<Vec<u8>, Error> {
        // Never size a buffer from a declared length that the body cannot
        // contain: a hostile header must not be able to request an allocation.
        if self.pos < self.hash_limit && n as u64 > self.hash_limit - self.pos {
            return Err(Error::CountMismatch {
                what: "file length",
            });
        }
        let mut b = vec![0u8; n];
        self.f
            .read_exact(&mut b)
            .map_err(|_| Error::CountMismatch {
                what: "file length",
            })?;
        self.feed(&b);
        Ok(b)
    }

    fn u8(&mut self) -> Result<u8, Error> {
        Ok(self.exact(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, Error> {
        let b = self.exact(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn u64(&mut self) -> Result<u64, Error> {
        let b = self.exact(8)?;
        let mut a = [0u8; 8];
        a.copy_from_slice(&b);
        Ok(u64::from_be_bytes(a))
    }

    fn str(&mut self) -> Result<String, Error> {
        let n = self.u32()? as usize;
        let b = self.exact(n)?;
        String::from_utf8(b).map_err(|_| Error::Malformed("invalid UTF-8".into()))
    }

    /// Consume `n` bytes without keeping them.  Payloads can be large and the
    /// metadata pass only needs their length.
    fn skip(&mut self, n: usize) -> Result<(), Error> {
        let mut buf = [0u8; 64 * 1024];
        let mut left = n;
        while left > 0 {
            let take = left.min(buf.len());
            self.f
                .read_exact(&mut buf[..take])
                .map_err(|_| Error::CountMismatch {
                    what: "file length",
                })?;
            self.feed(&buf[..take]);
            left -= take;
        }
        Ok(())
    }

    fn digest(&self) -> [u8; 32] {
        let out = self.hasher.clone().finalize();
        let mut a = [0u8; 32];
        a.copy_from_slice(&out);
        a
    }
}

/// Everything replay needs that is not a theorem: the tables, the header, the
/// node count and, per node, the number of consumers not yet validated.
///
/// `remaining[i]` is the only per-node array.  It is one `u32` per node; the
/// theorems themselves live in the frontier and nowhere else.
struct ObjectScan {
    obj: ParsedObject,
    node_count: u32,
    /// Absolute file offset of the node table.
    nodes_offset: u64,
    remaining: Vec<u32>,
}

/// Read the header and the type/term tables, then make one metadata pass over
/// the node table to count each node's consumers.  The digest is verified over
/// exactly the body bytes seen, so a truncated or spliced file is refused
/// before any rule is applied.
fn scan_object(path: &Path) -> Result<ObjectScan, Error> {
    let file_len = std::fs::metadata(path)?.len();
    let trailer = 32u64 + OBJ_END.len() as u64;
    if file_len < OBJ_MAGIC.len() as u64 + 1 + trailer {
        return Err(Error::CountMismatch {
            what: "file length",
        });
    }
    let body_end = file_len - trailer;
    let mut c = HashReader::open(path, body_end)?;

    if c.exact(OBJ_MAGIC.len())? != OBJ_MAGIC {
        return Err(Error::BadMagic);
    }
    let version = c.u8()?;
    if version != OBJ_VERSION {
        return Err(Error::BadVersion(version));
    }
    let name = c.str()?;
    let raw_nodes = c.u32()? as u64;
    let inference_total = c.u64()?;
    let inference_begin = c.u64()?;
    let root = c.u32()?;
    let mut theory_digest = [0u8; 32];
    theory_digest.copy_from_slice(&c.exact(32)?);
    let mut statement_id = [0u8; 32];
    statement_id.copy_from_slice(&c.exact(32)?);
    let stmt_id = c.u32()?;
    let nh = c.u32()? as usize;
    if nh as u64 > body_end.saturating_sub(c.pos) / 4 {
        return Err(Error::CountMismatch {
            what: "hypothesis count",
        });
    }
    let mut hyp_ids = Vec::with_capacity(nh);
    for _ in 0..nh {
        hyp_ids.push(c.u32()?);
    }
    let na = c.u32()? as usize;
    if na as u64 > body_end.saturating_sub(c.pos) / 32 {
        return Err(Error::CountMismatch {
            what: "axiom count",
        });
    }
    let mut axiom_digests = BTreeSet::new();
    for _ in 0..na {
        let mut d = [0u8; 32];
        d.copy_from_slice(&c.exact(32)?);
        axiom_digests.insert(d);
    }
    let ni = c.u32()? as usize;
    if ni as u64 > body_end.saturating_sub(c.pos) / 32 {
        return Err(Error::CountMismatch {
            what: "import count",
        });
    }
    let mut import_digests = BTreeSet::new();
    for _ in 0..ni {
        let mut d = [0u8; 32];
        d.copy_from_slice(&c.exact(32)?);
        import_digests.insert(d);
    }
    let nreg = c.u32()? as usize;
    if nreg as u64 > body_end.saturating_sub(c.pos) / 12 {
        return Err(Error::CountMismatch {
            what: "axiom registry count",
        });
    }
    let mut axioms = Vec::with_capacity(nreg);
    for _ in 0..nreg {
        let id = c.u32()?;
        let dg = c.str()?;
        let txt = c.str()?;
        axioms.push((id, dg, txt));
    }

    // Type table.
    let nt = c.u32()? as usize;
    if nt as u64 > body_end.saturating_sub(c.pos) {
        return Err(Error::CountMismatch {
            what: "type table count",
        });
    }
    let mut types_raw: Vec<TypeKey> = Vec::with_capacity(nt);
    for _ in 0..nt {
        match c.u8()? {
            0 => types_raw.push(TypeKey::Var(c.str()?)),
            1 => {
                let f = c.str()?;
                let n = c.u32()? as usize;
                if n as u64 > body_end.saturating_sub(c.pos) / 4 {
                    return Err(Error::CountMismatch {
                        what: "type argument count",
                    });
                }
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
    if nterm as u64 > body_end.saturating_sub(c.pos) {
        return Err(Error::CountMismatch {
            what: "term table count",
        });
    }
    let mut term_keys: Vec<TermKey> = Vec::with_capacity(nterm);
    for _ in 0..nterm {
        let tag = c.u8()?;
        match tag {
            0 | 1 => {
                let n = c.str()?;
                let ty = c.u32()?;
                if ty as usize >= types.len() {
                    return Err(Error::TableReference {
                        what: "term type",
                        id: ty as u64,
                    });
                }
                if tag == 0 {
                    term_keys.push(TermKey::Var(n, ty));
                } else {
                    term_keys.push(TermKey::Const(n, ty));
                }
            }
            2 | 3 => {
                let a = c.u32()?;
                let b = c.u32()?;
                if a as usize >= term_keys.len() || b as usize >= term_keys.len() {
                    return Err(Error::TableReference {
                        what: "term child",
                        id: a.max(b) as u64,
                    });
                }
                if tag == 2 {
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
    let statement = terms[stmt_id as usize].clone();
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

    // Node table: one metadata pass.  A node record is at least 13 bytes, so
    // the declared count is bounded by the bytes actually present before any
    // per-node allocation is made.
    let nn = c.u32()? as usize;
    let nodes_offset = c.pos;
    let node_bytes = body_end.saturating_sub(nodes_offset);
    if nn as u64 > node_bytes / 13 {
        return Err(Error::CountMismatch {
            what: "node count against file size",
        });
    }
    let mut remaining = vec![0u32; nn];
    for i in 0..nn {
        let rule = Rule::from_u8(c.u8()?)?;
        let p0 = c.u32()?;
        let p1 = c.u32()?;
        let plen = c.u32()? as usize;
        let mut declared = 0usize;
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
            remaining[p as usize] = remaining[p as usize]
                .checked_add(1)
                .ok_or_else(|| Error::Malformed("consumer count overflow".into()))?;
        }
        if declared != rule.parents_needed() {
            return Err(Error::RuleFailure {
                node: i as u64,
                rule: rule.name(),
                why: format!("{declared} parents, expected {}", rule.parents_needed()),
            });
        }
        c.skip(plen)?;
    }
    if c.pos != body_end {
        return Err(Error::CountMismatch { what: "node table" });
    }
    let computed = c.digest();
    let stored = c.exact(32)?;
    if stored[..] != computed[..] {
        return Err(Error::DigestMismatch);
    }
    if c.exact(OBJ_END.len())? != OBJ_END {
        return Err(Error::BadMagic);
    }
    if root as usize >= nn {
        return Err(Error::ParentOutOfRange {
            node: root as u64,
            parent: root as u64,
        });
    }
    let mut file_digest = [0u8; 32];
    file_digest.copy_from_slice(&stored);

    Ok(ObjectScan {
        obj: ParsedObject {
            name,
            raw_nodes,
            inference_total,
            inference_begin,
            root,
            statement,
            theory_digest,
            statement_id,
            hyps,
            axiom_digests,
            import_digests,
            axioms,
            types,
            terms,
            file_digest,
        },
        node_count: nn as u32,
        nodes_offset,
        remaining,
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

/// How the replayer holds theorems while it walks the node table.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ReplayMode {
    /// Keep a theorem only while it has an unvalidated consumer.  A node's
    /// theorem is dropped as soon as the last node that names it as a parent
    /// has been validated, so retained theorems are O(frontier), not O(nodes).
    Frontier,
    /// Keep one theorem per node (the pre-IX.85 scheme).  Retained so the
    /// frontier bound can be measured against it; not used by `validate`.
    Retained,
}

impl ReplayMode {
    pub fn name(self) -> &'static str {
        match self {
            ReplayMode::Frontier => "frontier",
            ReplayMode::Retained => "retained",
        }
    }
}

/// Work state that outlives individual rule applications.
struct ReplayContext {
    axiom_seen: BTreeSet<[u8; 32]>,
    import_seen: BTreeSet<[u8; 32]>,
    constants: BTreeSet<String>,
    types: BTreeSet<String>,
    type_halves: BTreeMap<(String, String, String), (bool, bool)>,
}

/// A forward-only reader over the object's node table.
struct NodeReader {
    f: std::io::BufReader<File>,
    pos: u64,
}

impl NodeReader {
    fn open(path: &Path, offset: u64) -> Result<Self, Error> {
        use std::io::Seek;
        let mut f = File::open(path)?;
        f.seek(std::io::SeekFrom::Start(offset))?;
        Ok(NodeReader {
            f: std::io::BufReader::new(f),
            pos: offset,
        })
    }

    fn exact(&mut self, n: usize) -> Result<Vec<u8>, Error> {
        let mut b = vec![0u8; n];
        self.f
            .read_exact(&mut b)
            .map_err(|_| Error::CountMismatch { what: "node table" })?;
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

    /// Read one node record.  The payload buffer is the only per-node
    /// allocation and it is dropped when the node's theorem is produced.
    fn node(&mut self) -> Result<ObjNode, Error> {
        let rule = Rule::from_u8(self.u8()?)?;
        let p0 = self.u32()?;
        let p1 = self.u32()?;
        let plen = self.u32()? as usize;
        let payload = self.exact(plen)?;
        Ok(ObjNode {
            rule,
            p0,
            p1,
            payload,
        })
    }
}

fn show_type(t: &Type) -> String {
    match t {
        Type::Var(v) => v.clone(),
        Type::App(f, args) if args.is_empty() => f.clone(),
        Type::App(f, args) => format!(
            "{} {f}",
            args.iter().map(show_type).collect::<Vec<_>>().join(" ")
        ),
    }
}

fn show_term(t: &TermRef) -> String {
    match &**t {
        Term::Var(n, ty) => format!("{n}:{}", show_type(ty)),
        Term::Const(n, _) => n.clone(),
        Term::Comb(f, x) => format!("({} {})", show_term(f), show_term(x)),
        Term::Abs(v, b) => format!("(\\{}. {})", show_term(v), show_term(b)),
    }
}

fn record_assumptions(n: &ObjNode, th: &Thm, ctx: &mut ReplayContext) {
    match n.rule {
        Rule::Axiom => {
            ctx.axiom_seen.insert(statement_digest(&th.concl));
        }
        Rule::Import => {
            ctx.import_seen.insert(sequent_digest(&th.hyps, &th.concl));
        }
        _ => {}
    }
}

/// Validate an object by independently replaying it.
///
/// The environment is supplied from outside the certificate:
/// * `theory` decides which axioms and definitions are authorised and which
///   constants/types already exist (`G1.2`–`G1.4`);
/// * `imports` decides which pre-recording sequents may be imported (`G1.1`);
/// * `expected` names the statement, bound to the theory (`G1.5`);
/// * `budget` is a hard node ceiling, so replay is resource-bounded.
///
/// Replay is frontier-bounded (see [`ReplayMode::Frontier`]).
///
/// Refusals:
/// * truncation / corruption — trailer digest or declared counts disagree;
/// * splicing — a node does not follow from its parents, or an import is not in
///   the store;
/// * wrong conclusion — the replayed statement identity differs from
///   `expected`;
/// * environment — the object names a different theory, or the supplied store
///   does not hash to the theory's pinned store.
pub fn validate(
    path: &Path,
    expected: &Expected,
    theory: &ValidatedTheory,
    imports: &CheckedTheoremStore,
    budget: u64,
) -> Result<Validation, Error> {
    validate_with(
        path,
        expected,
        theory,
        imports,
        budget,
        ReplayMode::Frontier,
    )
}

/// [`validate`] with an explicit retention strategy.
#[allow(clippy::too_many_arguments)]
pub fn validate_with(
    path: &Path,
    expected: &Expected,
    theory: &ValidatedTheory,
    imports: &CheckedTheoremStore,
    budget: u64,
    mode: ReplayMode,
) -> Result<Validation, Error> {
    let scan = scan_object(path)?;

    // The environment must be self-consistent before it is used.
    if theory.store_digest != imports.digest() {
        return Err(Error::StoreDigestMismatch {
            expected: hex32(&theory.store_digest),
            got: imports.digest_hex(),
        });
    }

    let mut ctx = ReplayContext {
        axiom_seen: BTreeSet::new(),
        import_seen: BTreeSet::new(),
        constants: theory.constants.clone(),
        types: theory.types.clone(),
        type_halves: BTreeMap::new(),
    };
    let (root, peak_live) = match mode {
        ReplayMode::Frontier => replay_frontier(path, &scan, theory, imports, budget, &mut ctx)?,
        ReplayMode::Retained => replay_retained(path, &scan, theory, imports, budget, &mut ctx)?,
    };
    finish_validation(
        &scan, &root, &ctx, theory, imports, expected, budget, peak_live, mode,
    )
}

/// The frontier replay: validate in ascending id order, hold a theorem only
/// while it still has a consumer, and release it on the consumer's validation.
fn replay_frontier(
    path: &Path,
    scan: &ObjectScan,
    theory: &ValidatedTheory,
    imports: &CheckedTheoremStore,
    budget: u64,
    ctx: &mut ReplayContext,
) -> Result<(Thm, u64), Error> {
    let p = &scan.obj;
    let mut reader = NodeReader::open(path, scan.nodes_offset)?;
    let mut remaining = scan.remaining.clone();
    let mut frontier: HashMap<u32, Thm> = HashMap::new();
    let mut root_thm: Option<Thm> = None;
    let mut peak_live: u64 = 0;

    for i in 0..scan.node_count {
        if i as u64 >= budget {
            return Err(Error::BudgetExceeded {
                budget,
                reached: i as u64,
            });
        }
        let n = reader.node()?;
        let mut parents: Vec<Thm> = Vec::with_capacity(2);
        for pid in [n.p0, n.p1] {
            if pid == ABSENT {
                continue;
            }
            let th = frontier.get(&pid).ok_or(Error::ParentOutOfRange {
                node: i as u64,
                parent: pid as u64,
            })?;
            parents.push(th.clone());
        }
        let th = match apply_rule_checked(
            i as u64,
            &n,
            p,
            &parents,
            theory,
            imports,
            &mut ctx.constants,
            &mut ctx.types,
            &mut ctx.type_halves,
        ) {
            Ok(th) => th,
            Err(e) => {
                if std::env::var_os("TEKMERION_PROOF_DEBUG").is_some() {
                    eprintln!(
                        "DEBUG node={i} rule={} p0={} p1={} plen={} err={e}",
                        n.rule.name(),
                        n.p0,
                        n.p1,
                        n.payload.len()
                    );
                    for (k, th) in parents.iter().enumerate() {
                        eprintln!(
                            "DEBUG parent[{k}] hyps={} concl={}",
                            th.hyps.len(),
                            canon_hex(&th.concl)
                        );
                        for h in th.hyps.iter().take(4) {
                            eprintln!("DEBUG   hyp {}", canon_hex(h));
                        }
                    }
                    if n.rule == Rule::Abs {
                        if let Ok(v) = decode_term_id(&mut Cursor::new(&n.payload), &p.terms) {
                            eprintln!("DEBUG abs var {}", show_term(&v));
                        }
                    }
                    if let Ok(mut dbg) = NodeReader::open(path, scan.nodes_offset) {
                        let start = i.saturating_sub(4);
                        for j in 0..=i {
                            if let Ok(nn) = dbg.node() {
                                if j >= start {
                                    eprintln!(
                                        "DEBUG rule[{j}]={} p0={} p1={} plen={}",
                                        nn.rule.name(),
                                        nn.p0,
                                        nn.p1,
                                        nn.payload.len()
                                    );
                                }
                            }
                        }
                    }
                }
                return Err(e);
            }
        };
        drop(parents);
        if std::env::var_os("TEKMERION_PROOF_DEBUG").is_some()
            && (i <= 22 || (355..=363).contains(&i))
        {
            eprintln!(
                "TRACE node={i} rule={} hyps={} concl={}",
                n.rule.name(),
                th.hyps.len(),
                show_term(&th.concl)
            );
            for h in th.hyps.iter().take(3) {
                eprintln!("TRACE   hyp {}", show_term(h));
            }
        }
        record_assumptions(&n, &th, ctx);

        for pid in [n.p0, n.p1] {
            if pid == ABSENT {
                continue;
            }
            let left = remaining[pid as usize];
            if left == 0 {
                return Err(Error::Malformed(format!(
                    "node {i}: parent {pid} has no unvalidated consumer to release"
                )));
            }
            remaining[pid as usize] = left - 1;
            if left == 1 {
                frontier.remove(&pid);
            }
        }

        if i == p.root {
            // The root is checked against the recorded statement below, so its
            // theorem is kept aside rather than in the frontier.
            root_thm = Some(th.clone());
        }
        // A node with no consumers that is not the root is unreachable from
        // the conclusion: it was validated for well-formedness and can be
        // dropped at once.
        if i != p.root && remaining[i as usize] == 0 {
            continue;
        }
        frontier.insert(i, th);
        peak_live = peak_live.max(frontier.len() as u64);
    }

    let root = root_thm.ok_or_else(|| Error::Malformed("root node was not validated".into()))?;
    Ok((root, peak_live))
}

/// The pre-IX.85 replay: one theorem per node.  Present so the bound can be
/// measured on the same object with the same rules.
fn replay_retained(
    path: &Path,
    scan: &ObjectScan,
    theory: &ValidatedTheory,
    imports: &CheckedTheoremStore,
    budget: u64,
    ctx: &mut ReplayContext,
) -> Result<(Thm, u64), Error> {
    let p = &scan.obj;
    let mut reader = NodeReader::open(path, scan.nodes_offset)?;
    let mut thms: Vec<Thm> = Vec::new();

    for i in 0..scan.node_count {
        if i as u64 >= budget {
            return Err(Error::BudgetExceeded {
                budget,
                reached: i as u64,
            });
        }
        let n = reader.node()?;
        let mut parents: Vec<Thm> = Vec::new();
        for pid in [n.p0, n.p1] {
            if pid == ABSENT {
                continue;
            }
            let th = thms.get(pid as usize).ok_or(Error::ParentOutOfRange {
                node: i as u64,
                parent: pid as u64,
            })?;
            parents.push(th.clone());
        }
        let th = apply_rule_checked(
            i as u64,
            &n,
            p,
            &parents,
            theory,
            imports,
            &mut ctx.constants,
            &mut ctx.types,
            &mut ctx.type_halves,
        )?;
        record_assumptions(&n, &th, ctx);
        thms.push(th);
    }

    let root = thms
        .get(p.root as usize)
        .cloned()
        .ok_or_else(|| Error::Malformed("root node was not validated".into()))?;
    Ok((root, thms.len() as u64))
}

/// Checks that need the complete replay: complete type extensions, the
/// claimed axiom/import sets, a closed root, the object's environment, and the
/// statement identity.
#[allow(clippy::too_many_arguments)]
fn finish_validation(
    scan: &ObjectScan,
    root: &Thm,
    ctx: &ReplayContext,
    theory: &ValidatedTheory,
    imports: &CheckedTheoremStore,
    expected: &Expected,
    budget: u64,
    peak_live: u64,
    mode: ReplayMode,
) -> Result<Validation, Error> {
    let p = &scan.obj;

    // Every type extension must be complete: both the abs/rep and rep/abs
    // halves, under the same triple (`G1.4`).
    for (k, (abs, rep)) in &ctx.type_halves {
        if !*abs || !*rep {
            return Err(Error::RuleFailure {
                node: p.root as u64,
                rule: "new_basic_type_definition",
                why: format!(
                    "incomplete extension registry for {} ({} / {})",
                    k.0, k.1, k.2
                ),
            });
        }
    }

    if ctx.axiom_seen != p.axiom_digests {
        return Err(Error::AssumptionSetMismatch { what: "axiom" });
    }
    if ctx.import_seen != p.import_digests {
        return Err(Error::AssumptionSetMismatch { what: "import" });
    }

    if !root.hyps.is_empty() {
        return Err(Error::RootNotClosed(root.hyps.len()));
    }
    if p.hyps.len() != root.hyps.len() {
        return Err(Error::CountMismatch {
            what: "root hypotheses",
        });
    }
    if !alpha_eq(&root.concl, &p.statement) {
        return Err(Error::ConclusionMismatch {
            expected: "<object header statement>".into(),
            got: statement_digest(&root.concl)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
        });
    }

    // The object must have been checked against the environment it names.
    if p.theory_digest != theory.digest() {
        return Err(Error::TheoryDigestMismatch {
            expected: theory.digest_hex(),
            got: hex32(&p.theory_digest),
        });
    }

    let got_id = statement_identity(&theory.digest(), &root.hyps, &root.concl);
    if got_id != expected.statement_id {
        return Err(Error::StatementIdentityMismatch {
            expected: hex32(&expected.statement_id),
            got: hex32(&got_id),
        });
    }
    if p.statement_id != expected.statement_id {
        return Err(Error::StatementIdentityMismatch {
            expected: hex32(&expected.statement_id),
            got: hex32(&p.statement_id),
        });
    }

    Ok(Validation {
        theorem: p.name.clone(),
        statement_digest: hex32(&statement_digest(&root.concl)),
        statement_id: hex32(&got_id),
        theory_digest: theory.digest_hex(),
        store_digest: imports.digest_hex(),
        derivation_digest: hex32(&p.file_digest),
        node_count: scan.node_count as u64,
        reachable: scan.node_count as u64,
        raw_nodes: p.raw_nodes,
        inference_total: p.inference_total,
        inference_begin: p.inference_begin,
        imports: p.import_digests.len() as u64,
        axioms: p.axioms.iter().map(|(_, d, _)| d.clone()).collect(),
        peak_live_nodes: peak_live,
        budget,
        replay_mode: mode.name(),
    })
}

/// One primitive rule application, checked against the external environment.
#[allow(clippy::too_many_arguments)]
fn apply_rule_checked(
    node: u64,
    n: &ObjNode,
    p: &ParsedObject,
    parents: &[Thm],
    theory: &ValidatedTheory,
    imports: &CheckedTheoremStore,
    constants: &mut BTreeSet<String>,
    types: &mut BTreeSet<String>,
    type_halves: &mut BTreeMap<(String, String, String), (bool, bool)>,
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
                return Err(fail(format!(
                    "EQ_MP: premise does not match (lhs {} vs premise {})",
                    canon_hex(&l),
                    canon_hex(&b.concl)
                )));
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
            let d = statement_digest(&t);
            if !theory.permits_axiom(&d) {
                return Err(Error::AxiomNotAuthorised { digest: hex32(&d) });
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
            let c2 = Rc::new(Term::Const(cname.clone(), cty));
            concl = mk_eq(c2, r).map_err(|e| fail(e.to_string()))?;
            // G1.3: freshness, decided by the authoritative environment plus the
            // definitions already accepted in this derivation.
            if constants.contains(&cname) {
                return Err(Error::DefinitionNotFresh { name: cname });
            }
            let d = statement_digest(&concl);
            if !theory.permits_definition(&d) {
                return Err(Error::DefinitionNotAuthorised { digest: hex32(&d) });
            }
            constants.insert(cname);
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
            let abs = Rc::new(Term::Const(absname.clone(), absty));
            let rep = Rc::new(Term::Const(repname.clone(), repty));
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
            // G1.4: the extension must be in the registry, and every name fresh.
            if types.contains(&tyname)
                || constants.contains(&absname)
                || constants.contains(&repname)
            {
                return Err(Error::TypeNotFresh { name: tyname });
            }
            if !theory.permits_type_definition(&tyname, &absname, &repname) {
                return Err(Error::TypeDefinitionNotAuthorised { tyname });
            }
            types.insert(tyname.clone());
            constants.insert(absname.clone());
            constants.insert(repname.clone());
            let e = type_halves
                .entry((tyname, absname, repname))
                .or_insert((false, false));
            if rule == Rule::TypeDefAbs {
                e.0 = true;
            } else {
                e.1 = true;
            }
        }
        Rule::Import => {
            let nh = c.u32()? as usize;
            for _ in 0..nh {
                hyps.push(decode_term_id(&mut c, &p.terms)?);
            }
            concl = decode_term_id(&mut c, &p.terms)?;
            // G1.1: the sequent must be justified by the external store; the
            // result is the *stored* theorem, never one rebuilt from the object.
            let d = sequent_digest(&hyps, &concl);
            let stored = imports
                .get(&d)
                .ok_or_else(|| Error::ImportNotJustified { digest: hex32(&d) })?;
            if stored.hyps.len() != hyps.len()
                || !stored.hyps.iter().zip(&hyps).all(|(a, b)| alpha_eq(a, b))
                || !alpha_eq(&stored.concl, &concl)
            {
                return Err(fail("IMPORT payload differs from the checked store".into()));
            }
            return Ok(stored.clone());
        }
    }
    if c.remaining() != 0 {
        return Err(fail(format!("{} trailing payload bytes", c.remaining())));
    }
    Ok(Thm { hyps, concl })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static CTR: AtomicUsize = AtomicUsize::new(0);

    fn tmpdir() -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!(
            "tekmerion-po-{}-{}",
            std::process::id(),
            CTR.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn bv(n: &str) -> TermRef {
        Rc::new(Term::Var(n.to_string(), bool_ty()))
    }

    fn bc(n: &str) -> TermRef {
        Rc::new(Term::Const(n.to_string(), bool_ty()))
    }

    fn obj_with_terms(terms: Vec<TermRef>) -> ParsedObject {
        ParsedObject {
            name: "test".into(),
            raw_nodes: 0,
            inference_total: 0,
            inference_begin: 0,
            root: 0,
            statement: terms[0].clone(),
            theory_digest: [0; 32],
            statement_id: [0; 32],
            hyps: vec![],
            axiom_digests: BTreeSet::new(),
            import_digests: BTreeSet::new(),
            axioms: vec![],
            types: vec![Type::App("bool".into(), vec![])],
            terms,
            file_digest: [0; 32],
        }
    }

    fn leaf(rule: Rule, payload: Vec<u8>) -> ObjNode {
        ObjNode {
            rule,
            p0: ABSENT,
            p1: ABSENT,
            payload,
        }
    }

    fn run_leaf(
        obj: &ParsedObject,
        n: &ObjNode,
        theory: &ValidatedTheory,
        store: &CheckedTheoremStore,
    ) -> Result<Thm, Error> {
        apply_rule_checked(
            0,
            n,
            obj,
            &[],
            theory,
            store,
            &mut theory.constants.clone(),
            &mut theory.types.clone(),
            &mut BTreeMap::new(),
        )
    }

    // ---- raw stream construction -------------------------------------------

    fn raw_node(rule: Rule, p0: u32, p1: u32, payload: &[u8]) -> Vec<u8> {
        let mut v = vec![1u8];
        put_u32(&mut v, (13 + payload.len()) as u32);
        v.push(rule as u8);
        put_u32(&mut v, p0);
        put_u32(&mut v, p1);
        put_u32(&mut v, payload.len() as u32);
        v.extend_from_slice(payload);
        v
    }

    #[allow(clippy::too_many_arguments)]
    fn raw_footer(
        name: &str,
        root: u32,
        hyps: &[TermRef],
        concl: &TermRef,
        total: u32,
        begin: u32,
        nodes: u32,
        axioms: &[(u32, String, String)],
    ) -> Vec<u8> {
        let mut b = Vec::new();
        put_str(&mut b, name);
        put_u32(&mut b, root);
        put_u32(&mut b, hyps.len() as u32);
        for h in hyps {
            put_raw_term(&mut b, h);
        }
        put_raw_term(&mut b, concl);
        put_u32(&mut b, total);
        put_u32(&mut b, begin);
        put_u32(&mut b, nodes);
        put_u32(&mut b, axioms.len() as u32);
        for (id, dg, txt) in axioms {
            put_u32(&mut b, *id);
            put_str(&mut b, dg);
            put_str(&mut b, txt);
        }
        let mut v = vec![2u8];
        put_u32(&mut v, b.len() as u32);
        v.extend_from_slice(&b);
        v
    }

    fn term_payload(t: &TermRef) -> Vec<u8> {
        let mut v = Vec::new();
        put_raw_term(&mut v, t);
        v
    }

    struct Fixture {
        summary: Summary,
        object: std::path::PathBuf,
        store: std::path::PathBuf,
        expected: std::path::PathBuf,
    }

    fn finalize_stream(raw: &[u8]) -> Fixture {
        let d = tmpdir();
        let raw_path = d.join("stream.raw");
        let object = d.join("obj.hlpo");
        let store = d.join("store.hlst");
        let expected = d.join("expected.txt");
        std::fs::write(&raw_path, raw).unwrap();
        let summary = finalize(
            &raw_path,
            &object,
            Some(&store),
            None,
            Some(&expected),
            None,
        )
        .unwrap();
        Fixture {
            summary,
            object,
            store,
            expected,
        }
    }

    fn check(f: &Fixture) -> Result<Validation, Error> {
        check_mode(f, ReplayMode::Frontier)
    }

    fn check_mode(f: &Fixture, mode: ReplayMode) -> Result<Validation, Error> {
        let store = CheckedTheoremStore::load(&f.store)?;
        let theory = ValidatedTheory::with_store(&store);
        let expected = Expected::load(&f.expected)?;
        validate_with(&f.object, &expected, &theory, &store, 1_000_000, mode)
    }

    /// `n`-node chain: node 0 is `REFL x`; node `i` is `TRANS(i - 1, 0)`.
    /// Node 0 therefore has `n - 1` consumers and stays live for the whole
    /// replay, while every other node is consumed exactly once.
    fn chain_raw(n: u32, x: &TermRef) -> Vec<u8> {
        let mut raw = Vec::new();
        raw.extend_from_slice(RAW_MAGIC);
        raw.extend_from_slice(&raw_node(Rule::Refl, ABSENT, ABSENT, &term_payload(x)));
        for i in 1..n {
            raw.extend_from_slice(&raw_node(Rule::Trans, i - 1, 0, &[]));
        }
        let concl = mk_eq(x.clone(), x.clone()).unwrap();
        raw.extend_from_slice(&raw_footer("CHAIN", n - 1, &[], &concl, n, 0, n, &[]));
        raw
    }

    fn simple_raw(name: &str, term: &TermRef) -> Vec<u8> {
        let concl = mk_eq(term.clone(), term.clone()).unwrap();
        let mut raw = Vec::new();
        raw.extend_from_slice(RAW_MAGIC);
        raw.extend_from_slice(&raw_node(Rule::Refl, ABSENT, ABSENT, &term_payload(term)));
        raw.extend_from_slice(&raw_footer(name, 0, &[], &concl, 1, 0, 1, &[]));
        raw
    }

    // ---- the six defects ---------------------------------------------------

    #[test]
    fn g1_1_import_requires_external_store() {
        let x = bv("x");
        let obj = obj_with_terms(vec![x.clone()]);
        let mut payload = Vec::new();
        put_u32(&mut payload, 1); // one hypothesis
        put_u32(&mut payload, 0); // hypothesis = term 0
        put_u32(&mut payload, 0); // conclusion = term 0
        let node = leaf(Rule::Import, payload);
        let theory = ValidatedTheory::default();

        let empty = CheckedTheoremStore::empty();
        let err = run_leaf(&obj, &node, &theory, &empty).unwrap_err();
        assert!(matches!(err, Error::ImportNotJustified { .. }), "{err}");

        let good = CheckedTheoremStore::from_theorems(&[Thm {
            hyps: vec![x.clone()],
            concl: x,
        }]);
        assert!(run_leaf(&obj, &node, &theory, &good).is_ok());
    }

    #[test]
    fn g1_2_axiom_requires_theory_authorisation() {
        let p = bv("p");
        let obj = obj_with_terms(vec![p.clone()]);
        let mut payload = Vec::new();
        put_u32(&mut payload, 0);
        let node = leaf(Rule::Axiom, payload);
        let store = CheckedTheoremStore::empty();

        let empty = ValidatedTheory::default();
        let err = run_leaf(&obj, &node, &empty, &store).unwrap_err();
        assert!(matches!(err, Error::AxiomNotAuthorised { .. }), "{err}");

        let mut axioms = BTreeSet::new();
        axioms.insert(statement_digest(&p));
        let theory = ValidatedTheory::new(
            axioms,
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            store.digest(),
        );
        assert!(run_leaf(&obj, &node, &theory, &store).is_ok());
    }

    #[test]
    fn g1_3_definition_rebinding_is_refused() {
        let lhs = Rc::new(Term::Var("foo".into(), bool_ty()));
        let definition = mk_eq(lhs, bc("base")).unwrap();
        let obj = obj_with_terms(vec![definition]);
        let node = leaf(Rule::Definition, {
            let mut v = Vec::new();
            put_u32(&mut v, 0);
            v
        });
        let store = CheckedTheoremStore::empty();
        let result = mk_eq(bc("foo"), bc("base")).unwrap();
        let mut defs = BTreeSet::new();
        defs.insert(statement_digest(&result));

        // Authorised and fresh: accepted.
        let theory = ValidatedTheory::new(
            BTreeSet::new(),
            defs.clone(),
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            store.digest(),
        );
        assert!(run_leaf(&obj, &node, &theory, &store).is_ok());

        // Authorised but the name already exists: refused.
        let mut constants = BTreeSet::new();
        constants.insert("foo".to_string());
        let rebinding = ValidatedTheory::new(
            BTreeSet::new(),
            defs,
            constants,
            BTreeSet::new(),
            BTreeSet::new(),
            store.digest(),
        );
        let err = run_leaf(&obj, &node, &rebinding, &store).unwrap_err();
        assert!(matches!(err, Error::DefinitionNotFresh { .. }), "{err}");
    }

    #[test]
    fn g1_4_type_definition_requires_registry() {
        let pfun = Rc::new(Term::Const("P".into(), fun_ty(&bool_ty(), &bool_ty())));
        let x = bv("x");
        let parent = Thm {
            hyps: vec![],
            concl: Rc::new(Term::Comb(pfun, x)),
        };
        let obj = obj_with_terms(vec![parent.concl.clone()]);
        let mut payload = Vec::new();
        put_str(&mut payload, "ty");
        put_str(&mut payload, "abs");
        put_str(&mut payload, "rep");
        let node = leaf(Rule::TypeDefAbs, payload);
        let store = CheckedTheoremStore::empty();

        let empty = ValidatedTheory::default();
        let err = apply_rule_checked(
            0,
            &node,
            &obj,
            std::slice::from_ref(&parent),
            &empty,
            &store,
            &mut BTreeSet::new(),
            &mut BTreeSet::new(),
            &mut BTreeMap::new(),
        )
        .unwrap_err();
        assert!(
            matches!(err, Error::TypeDefinitionNotAuthorised { .. }),
            "{err}"
        );

        let mut td = BTreeSet::new();
        td.insert(("ty".to_string(), "abs".to_string(), "rep".to_string()));
        let theory = ValidatedTheory::new(
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            td,
            store.digest(),
        );
        let ok = apply_rule_checked(
            0,
            &node,
            &obj,
            &[parent],
            &theory,
            &store,
            &mut BTreeSet::new(),
            &mut BTreeSet::new(),
            &mut BTreeMap::new(),
        );
        assert!(ok.is_ok(), "{:?}", ok.err());
    }

    #[test]
    fn g1_7_finalize_authorises_the_definitions_it_reaches() {
        // The checker authorises a definition through the theory (`G1.3`).  A
        // derivation that introduces a constant therefore exports a theory
        // that lists exactly that definition, and a theory without it refuses
        // the replay.
        let cty = bool_ty();
        let lhs = Rc::new(Term::Var("foo".into(), cty.clone()));
        let body = bc("base");
        let definition = mk_eq(lhs, body.clone()).unwrap();
        let mut raw = Vec::new();
        raw.extend_from_slice(RAW_MAGIC);
        raw.extend_from_slice(&raw_node(
            Rule::Definition,
            ABSENT,
            ABSENT,
            &term_payload(&definition),
        ));
        let concl = mk_eq(Rc::new(Term::Const("foo".into(), cty)), body).unwrap();
        raw.extend_from_slice(&raw_footer("DEF", 0, &[], &concl, 1, 0, 1, &[]));

        let d = tmpdir();
        let raw_path = d.join("def.raw");
        let object = d.join("def.hlpo");
        let store_path = d.join("def.hlst");
        let expected_path = d.join("def.expected");
        let theory_path = d.join("def.theory");
        std::fs::write(&raw_path, &raw).unwrap();
        let summary = finalize(
            &raw_path,
            &object,
            Some(&store_path),
            None,
            Some(&expected_path),
            Some(&theory_path),
        )
        .unwrap();
        assert_eq!(summary.definitions.len(), 1);

        let store = CheckedTheoremStore::load(&store_path).unwrap();
        let expected = Expected::load(&expected_path).unwrap();
        let exported = ValidatedTheory::parse(&std::fs::read(&theory_path).unwrap()).unwrap();
        assert!(validate(&object, &expected, &exported, &store, 1_000_000).is_ok());

        let bare = ValidatedTheory::with_store(&store);
        let err = validate(&object, &expected, &bare, &store, 1_000_000).unwrap_err();
        assert!(
            matches!(err, Error::DefinitionNotAuthorised { .. }),
            "{err}"
        );
        eprintln!("UNAUTHORISED DEFINITION REFUSED: {err}");
    }

    #[test]
    fn g1_5_statement_identity_binds_the_theory() {
        let x = bv("x");
        let store = CheckedTheoremStore::empty();
        let t1 = ValidatedTheory::with_store(&store);
        let mut ax = BTreeSet::new();
        ax.insert(statement_digest(&x));
        let t2 = ValidatedTheory::new(
            ax,
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            store.digest(),
        );
        let s1 = statement_identity(&t1.digest(), &[], &x);
        let s2 = statement_identity(&t2.digest(), &[], &x);
        assert_ne!(s1, s2, "the same term under two theories must differ");
    }

    #[test]
    fn g1_6_finalize_buffers_one_payload_not_the_object() {
        // A 20,000-node chain: every node is reachable, so the object is much
        // larger than any single payload buffer.
        let n = 20_000u32;
        let x = bv("x");
        let mut raw = Vec::new();
        raw.extend_from_slice(RAW_MAGIC);
        let mut payload = Vec::new();
        put_raw_term(&mut payload, &x);
        raw.extend_from_slice(&raw_node(Rule::Refl, ABSENT, ABSENT, &payload));
        for i in 1..n {
            raw.extend_from_slice(&raw_node(Rule::Trans, i - 1, 0, &[]));
        }
        let concl = mk_eq(x.clone(), x).unwrap();
        raw.extend_from_slice(&raw_footer("CHAIN", n - 1, &[], &concl, n, 0, n, &[]));
        let f = finalize_stream(&raw);
        assert_eq!(f.summary.node_count, n as u64);
        assert!(
            f.summary.bytes > 200_000,
            "object is {} bytes",
            f.summary.bytes
        );
        assert!(
            f.summary.peak_buffered_payload_bytes < 4096,
            "peak payload buffer {}",
            f.summary.peak_buffered_payload_bytes
        );
    }

    // ---- frontier-bounded replay ------------------------------------------

    #[test]
    fn frontier_releases_theorems_on_their_last_consumer() {
        // 20,000 nodes, but at most two theorems are ever retained: node 0
        // (consumed by every link) and the current link.
        let n = 20_000u32;
        let x = bv("x");
        let f = finalize_stream(&chain_raw(n, &x));
        assert_eq!(f.summary.node_count, n as u64);

        let frontier = check_mode(&f, ReplayMode::Frontier).unwrap();
        let retained = check_mode(&f, ReplayMode::Retained).unwrap();
        assert_eq!(frontier.statement_id, retained.statement_id);
        assert_eq!(frontier.node_count, n as u64);
        assert_eq!(frontier.replay_mode, "frontier");
        assert_eq!(frontier.budget, 1_000_000);
        assert!(
            frontier.peak_live_nodes <= 3,
            "frontier peak live {}",
            frontier.peak_live_nodes
        );
        // The same object under the old scheme retains every node.
        assert_eq!(retained.peak_live_nodes, n as u64);
        eprintln!(
            "FRONTIER PEAK {} of {} nodes (retained: {})",
            frontier.peak_live_nodes, frontier.node_count, retained.peak_live_nodes
        );
    }

    #[test]
    fn frontier_and_retained_agree_on_a_shared_parent() {
        // Two rewrites of the same leaf feed one TRANS: the leaf is named
        // twice by the same node, so its consumer count is two and it is only
        // released after the second decrement.
        let x = bv("x");
        let mut raw = Vec::new();
        raw.extend_from_slice(RAW_MAGIC);
        raw.extend_from_slice(&raw_node(Rule::Refl, ABSENT, ABSENT, &term_payload(&x)));
        raw.extend_from_slice(&raw_node(Rule::Trans, 0, 0, &[]));
        let concl = mk_eq(x.clone(), x).unwrap();
        raw.extend_from_slice(&raw_footer("SELF", 1, &[], &concl, 2, 0, 2, &[]));
        let f = finalize_stream(&raw);
        let frontier = check_mode(&f, ReplayMode::Frontier).unwrap();
        let retained = check_mode(&f, ReplayMode::Retained).unwrap();
        assert_eq!(frontier.statement_id, retained.statement_id);
        assert_eq!(frontier.node_count, 2);
    }

    #[test]
    fn budget_still_refuses_before_replay_finishes() {
        let x = bv("x");
        let f = finalize_stream(&chain_raw(1_000, &x));
        let store = CheckedTheoremStore::load(&f.store).unwrap();
        let theory = ValidatedTheory::with_store(&store);
        let expected = Expected::load(&f.expected).unwrap();
        let err = validate_with(
            &f.object,
            &expected,
            &theory,
            &store,
            10,
            ReplayMode::Frontier,
        )
        .unwrap_err();
        assert!(
            matches!(
                err,
                Error::BudgetExceeded {
                    budget: 10,
                    reached: 10
                }
            ),
            "{err}"
        );
        eprintln!("BUDGET REFUSED: {err}");
    }

    // ---- the four refusals, end to end ------------------------------------

    #[test]
    fn control_valid_object_replays() {
        let x = bv("x");
        let f = finalize_stream(&simple_raw("REFL", &x));
        let v = check(&f).unwrap();
        assert_eq!(v.node_count, 1);
        assert_eq!(v.statement_id, f.summary.statement_id);
    }

    #[test]
    fn control_truncated_object_is_refused() {
        let x = bv("x");
        let f = finalize_stream(&simple_raw("REFL", &x));
        let bytes = std::fs::read(&f.object).unwrap();
        std::fs::write(&f.object, &bytes[..bytes.len() - 10]).unwrap();
        let err = check(&f).unwrap_err();
        assert!(
            matches!(
                err,
                Error::DigestMismatch | Error::CountMismatch { .. } | Error::BadMagic
            ),
            "{err}"
        );
        eprintln!("TRUNCATED REFUSED: {err}");
    }

    #[test]
    fn control_spliced_object_is_refused() {
        // Node 2 is TRANS(REFL x, REFL y): the middle terms differ, so the
        // second node does not follow from its parents.  Finalisation accepts
        // the stream; replay refuses it.
        let x = bv("x");
        let y = bv("y");
        let mut raw = Vec::new();
        raw.extend_from_slice(RAW_MAGIC);
        raw.extend_from_slice(&raw_node(Rule::Refl, ABSENT, ABSENT, &term_payload(&x)));
        raw.extend_from_slice(&raw_node(Rule::Refl, ABSENT, ABSENT, &term_payload(&y)));
        raw.extend_from_slice(&raw_node(Rule::Trans, 0, 1, &[]));
        let concl = mk_eq(x.clone(), x).unwrap();
        raw.extend_from_slice(&raw_footer("SPLICED", 2, &[], &concl, 3, 0, 3, &[]));
        let f = finalize_stream(&raw);
        let err = check(&f).unwrap_err();
        assert!(matches!(err, Error::RuleFailure { .. }), "{err}");
        eprintln!("SPLICED REFUSED: {err}");
    }

    #[test]
    fn control_wrong_conclusion_is_refused() {
        let x = bv("x");
        let y = bv("y");
        let f = finalize_stream(&simple_raw("REFL", &x));
        let store = CheckedTheoremStore::load(&f.store).unwrap();
        let theory = ValidatedTheory::with_store(&store);
        let wrong = Expected::compute(&theory, "REFL", &[], &y);
        let err = validate(&f.object, &wrong, &theory, &store, 1_000_000).unwrap_err();
        assert!(
            matches!(err, Error::StatementIdentityMismatch { .. }),
            "{err}"
        );
        eprintln!("WRONG CONCLUSION REFUSED: {err}");
    }

    #[test]
    fn control_unjustified_import_is_refused() {
        // The object imports x; the environment supplied to the checker has an
        // empty store, so the import is not justified.
        let x = bv("x");
        let mut raw = Vec::new();
        raw.extend_from_slice(RAW_MAGIC);
        let mut payload = Vec::new();
        put_u32(&mut payload, 1);
        put_raw_term(&mut payload, &x);
        put_raw_term(&mut payload, &x);
        raw.extend_from_slice(&raw_node(Rule::Import, ABSENT, ABSENT, &payload));
        raw.extend_from_slice(&raw_footer(
            "IMPORT",
            0,
            std::slice::from_ref(&x),
            &x,
            1,
            0,
            1,
            &[],
        ));
        let f = finalize_stream(&raw);
        let empties = CheckedTheoremStore::empty();
        let theory = ValidatedTheory::with_store(&empties);
        let expected = Expected::load(&f.expected).unwrap();
        let err = validate(&f.object, &expected, &theory, &empties, 1_000_000).unwrap_err();
        assert!(matches!(err, Error::ImportNotJustified { .. }), "{err}");
        eprintln!("UNJUSTIFIED IMPORT REFUSED: {err}");
    }

    #[test]
    fn control_truncated_raw_stream_is_refused() {
        let x = bv("x");
        let raw = simple_raw("REFL", &x);
        let d = tmpdir();
        let p = d.join("t.raw");
        std::fs::write(&p, &raw[..raw.len() / 2]).unwrap();
        let err = finalize(&p, &d.join("o"), None, None, None, None).unwrap_err();
        assert!(
            matches!(err, Error::RawTruncated | Error::RawShortRecord),
            "{err}"
        );
    }

    #[test]
    fn substitution_pair_order_matches_hol_light() {
        // HOL's `rev_assocd` matches the second component of each pair and
        // returns the first, so an INST_TYPE pair is (replacement, variable).
        let a = Rc::new(Type::Var("A".into()));
        let num = Rc::new(Type::App("num".into(), vec![]));
        let got = type_subst(&[(num.clone(), a)], &Type::Var("A".into()));
        assert_eq!(got, Type::App("num".into(), vec![]));
    }

    #[test]
    fn control_unauthorised_axiom_is_refused() {
        let p = bv("p");
        let mut raw = Vec::new();
        raw.extend_from_slice(RAW_MAGIC);
        raw.extend_from_slice(&raw_node(Rule::Axiom, ABSENT, ABSENT, &term_payload(&p)));
        raw.extend_from_slice(&raw_footer("AXIOM", 0, &[], &p, 1, 0, 1, &[]));
        let f = finalize_stream(&raw);
        let err = check(&f).unwrap_err();
        assert!(matches!(err, Error::AxiomNotAuthorised { .. }), "{err}");
        eprintln!("UNAUTHORISED AXIOM REFUSED: {err}");
    }
}
