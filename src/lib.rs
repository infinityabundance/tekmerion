//! # tekmerion
//!
//! Content-addressed evidence objects with **typed** automatic revocation.
//!
//! ## What this crate is for
//!
//! Tekmerion's third invariant is that a proof is revocable evidence: every accepted
//! theorem is content-addressed and names the digests it depends on, and any change to
//! those dependencies revokes it automatically. The naive reading of that invariant —
//! "if a pinned object's digest changed, revoke everything that pinned it" — is sound
//! but wildly imprecise: it revokes a proof when a source file is merely reformatted,
//! and it cannot distinguish "the theorem is no longer true" from "the theorem is still
//! true but is now about different bytes".
//!
//! This crate implements the three distinctions that make revocation usable:
//!
//! * [`EdgeKind::Semantic`] — the dependent uses the dependency's *meaning*.
//!   Only a change of semantic digest revokes.
//! * [`EdgeKind::Subject`] — the dependent is *about* the dependency's exact bytes.
//!   A byte change revokes, even when the meaning is unchanged, because the theorem
//!   no longer covers the shipped artefact.
//! * [`EdgeKind::Attestation`] — the dependency merely *accepted* the dependent
//!   (e.g. the checker binary that replayed it). A byte change makes the acceptance
//!   record stale; it does **not** invalidate the theorem.
//!
//! Revocation therefore returns a [`Severity`] per affected object, in increasing order:
//! [`Severity::AttestationStale`] (cheap to repair: replay), [`Severity::SubjectChanged`]
//! (the proof no longer applies to the artefact, but its statement is unchanged), and
//! [`Severity::SemanticsChanged`] (the proof must be redone).
//!
//! ## Event delta versus effective status
//!
//! A [`Revocation`] (alias [`AffectedByEvent`]) is **the delta of one event**: what the
//! update just made worse. It is not current status. A later event can report only its own
//! small delta while an older, more severe mismatch remains unresolved. Admission must
//! therefore read [`Store::status`], which recomputes the effective severity of an object
//! from the current head map over the whole transitive pin closure.
//!
//! ## What this crate does *not* establish
//!
//! It does not decide whether any theorem is true. Revocation is a graph property of
//! *recorded* dependencies; validity is decided only by a proof kernel. If a dependency
//! is omitted from the graph, the graph cannot revoke on it, and no amount of hashing
//! fixes that. Hashing establishes identity, never validity. See `README.md`.
//!
//! ## Design invariants enforced here
//!
//! 1. Object identity is *derived* from content; it is never supplied by a caller.
//! 2. Construction is monotone: through the API, a node may only pin a declared unit at
//!    its current digest, so the graph is acyclic by construction. A parsed document's
//!    node table may contain historical pins, but a pin must still name a node declared
//!    earlier, which gives the same acyclicity guarantee.
//! 3. Every unrepresentable or ambiguous input is refused rather than coerced. Refusal
//!    sites carry stable identifiers (`R01`…`R16`) so that
//!    `tools/mutate.py` can prove each one is load-bearing.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use sha2::{Digest as _, Sha256};

pub mod native;
pub mod wire;

// ---------------------------------------------------------------------------
// Lifecycle fault injection
// ---------------------------------------------------------------------------

/// Deterministic lifecycle fault switch, used only by `tools/mutate_lifecycle.py`.
///
/// With the `fault-injection` feature disabled this is a constant `false` and does not
/// read the environment. With the feature enabled, `TEKMERION_FAULT` may name one or
/// more comma-separated lifecycle codes (`L01`…`L05`).
pub fn fault(code: &str) -> bool {
    #[cfg(feature = "fault-injection")]
    {
        std::env::var("TEKMERION_FAULT")
            .map(|v| v.split(',').any(|c| c.trim() == code))
            .unwrap_or(false)
    }
    #[cfg(not(feature = "fault-injection"))]
    {
        let _ = code;
        false
    }
}

// ---------------------------------------------------------------------------
// Refusal sites
// ---------------------------------------------------------------------------

/// Return an error from the enclosing function and tag the site for mutation testing.
///
/// `tools/mutate.py` replaces `refuse!(Rnn, ..)` with [`skip_refusal!`] one site at a
/// time and records whether the test suite notices. A site whose removal no test catches
/// is a vacuous check.
#[macro_export]
macro_rules! refuse {
    ($site:ident, $e:expr) => {{
        let _ = stringify!($site);
        return ::core::result::Result::Err($e);
    }};
}

/// No-op counterpart used only by the mutation harness.
#[macro_export]
macro_rules! skip_refusal {
    ($site:ident, $e:expr) => {{
        let _ = stringify!($site);
    }};
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// R01
    BadMagic,
    /// R02
    UnsupportedVersion(String),
    /// R03
    WrongFieldCount(String),
    /// R04
    EmptyName,
    /// R05
    BadHex,
    /// R06
    WrongDigestLength(usize),
    /// R07
    UnknownUnit(String),
    /// R08
    PinDigestNotDeclared(String),
    /// R09
    UnknownEdgeKind(String),
    /// R10
    UnknownKind(String),
    /// R11
    MissingRequiredField { unit: String, field: &'static str },
    /// R12
    NameNotCanonical(String),
    /// R13
    DuplicatePin(String),
    /// R14
    TrailingBytes,
    /// R15
    UnknownUnitOnUpdate(String),
    /// R16
    DuplicateUnit(String),
    /// R10: the first field is not a known record tag.
    UnknownRecord(String),
    /// R03: a `node` record appeared after the head map started.
    RecordAfterHead,
    /// R03: a unit has declared nodes but no head.
    MissingHead(String),
    /// Internal inconsistency (should be unreachable through the public API).
    Corrupt(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::BadMagic => write!(f, "R01: not a TEKMERION-EVIDENCE document"),
            Error::UnsupportedVersion(v) => write!(f, "R02: unsupported version line {v:?}"),
            Error::WrongFieldCount(l) => {
                write!(f, "R03: expected exactly 5 tab-separated fields in {l:?}")
            }
            Error::EmptyName => write!(f, "R04: empty unit name"),
            Error::BadHex => write!(f, "R05: non-hexadecimal digit"),
            Error::WrongDigestLength(n) => {
                write!(f, "R06: digest must be 64 hex characters, got {n}")
            }
            Error::UnknownUnit(u) => write!(f, "R07: reference to undeclared unit {u:?}"),
            Error::PinDigestNotDeclared(u) => {
                write!(f, "R08: reference to a node digest not declared for {u:?}")
            }
            Error::UnknownEdgeKind(k) => write!(f, "R09: unknown edge kind {k:?}"),
            Error::UnknownKind(k) => write!(f, "R10: unknown evidence kind {k:?}"),
            Error::MissingRequiredField { unit, field } => {
                write!(f, "R11: {unit:?} requires field {field:?}")
            }
            Error::NameNotCanonical(n) => write!(f, "R12: non-canonical unit name {n:?}"),
            Error::DuplicatePin(p) => write!(f, "R13: duplicate pin {p:?}"),
            Error::TrailingBytes => write!(
                f,
                "R14: document is not newline-terminated or has trailing data"
            ),
            Error::UnknownUnitOnUpdate(u) => write!(f, "R15: cannot update undeclared unit {u:?}"),
            Error::DuplicateUnit(u) => write!(f, "R16: unit {u:?} declared twice"),
            Error::UnknownRecord(r) => write!(f, "R10: unknown record tag {r:?}"),
            Error::RecordAfterHead => write!(f, "R03: node record after the head map"),
            Error::MissingHead(u) => write!(f, "R03: unit {u:?} has nodes but no head"),
            Error::Corrupt(w) => write!(f, "internal inconsistency: {w}"),
        }
    }
}

impl std::error::Error for Error {}

// ---------------------------------------------------------------------------
// Digests and canonical encoding
// ---------------------------------------------------------------------------

/// A SHA-256 content identity.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Digest([u8; 32]);

impl Digest {
    pub const LEN: usize = 32;

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn hex(&self) -> String {
        let mut s = String::with_capacity(64);
        for b in self.0 {
            s.push(char::from_digit((b >> 4) as u32, 16).unwrap());
            s.push(char::from_digit((b & 0xf) as u32, 16).unwrap());
        }
        s
    }

    /// Parse exactly 64 lowercase or uppercase hex characters.
    pub fn parse_hex(s: &str) -> Result<Self, Error> {
        if s.len() != 64 {
            crate::refuse!(R06, Error::WrongDigestLength(s.len()));
        }
        let bytes = s.as_bytes();
        let mut out = [0u8; 32];
        for (i, slot) in out.iter_mut().enumerate() {
            let hi = hex_val(bytes[2 * i]);
            if hi.is_none() {
                crate::refuse!(R05, Error::BadHex);
            }
            let lo = hex_val(bytes[2 * i + 1]);
            if lo.is_none() {
                crate::refuse!(R05, Error::BadHex);
            }
            *slot = (hi.unwrap_or(0) << 4) | lo.unwrap_or(0);
        }
        Ok(Digest(out))
    }

    /// A named fixture digest. Not an evidence identity: labels are for tests and for
    /// synthetic corpora, never for recorded evidence.
    pub fn of_label(label: &str) -> Self {
        let mut c = Canon::new("tekmerion/label/v1");
        c.bytes(label.as_bytes());
        c.finish()
    }
}

impl fmt::Debug for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.hex())
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.hex())
    }
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Domain-separated, length-prefixed hasher.
///
/// Every variable-length field is length-prefixed and every optional field is
/// tagged, so distinct structures cannot collide by concatenation.
pub(crate) struct Canon(Sha256);

impl Canon {
    pub(crate) fn new(domain: &str) -> Self {
        let mut h = Sha256::new();
        h.update(domain.as_bytes());
        h.update([0u8]);
        Canon(h)
    }

    pub(crate) fn tag(&mut self, b: u8) {
        self.0.update([b]);
    }

    pub(crate) fn bytes(&mut self, b: &[u8]) {
        self.0.update((b.len() as u64).to_le_bytes());
        self.0.update(b);
    }

    pub(crate) fn digest(&mut self, d: &Digest) {
        self.0.update(d.0);
    }

    pub(crate) fn opt_digest(&mut self, d: Option<&Digest>) {
        match d {
            None => self.0.update([0u8]),
            Some(x) => {
                self.0.update([1u8]);
                self.0.update(x.0);
            }
        }
    }

    pub(crate) fn finish(self) -> Digest {
        let out = self.0.finalize();
        let mut a = [0u8; 32];
        a.copy_from_slice(&out);
        Digest(a)
    }
}

// ---------------------------------------------------------------------------
// Units, kinds, edges
// ---------------------------------------------------------------------------

/// A stable logical name for an evidence object.
///
/// The unit is what survives a change; the [`Digest`] is what changes. Revocation is
/// keyed on `(unit, digest)` pairs so that a pin can say precisely which version it used.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct UnitId(String);

impl UnitId {
    pub fn new(s: &str) -> Result<Self, Error> {
        if s.is_empty() {
            crate::refuse!(R04, Error::EmptyName);
        }
        let ok = s
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-' | b'/' | b'+'));
        if !ok {
            crate::refuse!(R12, Error::NameNotCanonical(s.to_string()));
        }
        Ok(UnitId(s.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for UnitId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// What sort of evidence an object is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Spec,
    Artefact,
    IsaModel,
    Kernel,
    Toolchain,
    Certificate,
    Theorem,
}

impl Kind {
    pub(crate) fn tag(self) -> u8 {
        match self {
            Kind::Spec => 1,
            Kind::Artefact => 2,
            Kind::IsaModel => 3,
            Kind::Kernel => 4,
            Kind::Toolchain => 5,
            Kind::Certificate => 6,
            Kind::Theorem => 7,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Kind::Spec => "spec",
            Kind::Artefact => "artefact",
            Kind::IsaModel => "isamodel",
            Kind::Kernel => "kernel",
            Kind::Toolchain => "toolchain",
            Kind::Certificate => "cert",
            Kind::Theorem => "thm",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "spec" => Kind::Spec,
            "artefact" => Kind::Artefact,
            "isamodel" => Kind::IsaModel,
            "kernel" => Kind::Kernel,
            "toolchain" => Kind::Toolchain,
            "cert" => Kind::Certificate,
            "thm" => Kind::Theorem,
            _ => return None,
        })
    }

    /// Fields a well-formed object of this kind must carry.
    pub fn requires(self, field: Field) -> bool {
        matches!(
            (self, field),
            (Kind::Spec, Field::Semantic)
                | (Kind::IsaModel, Field::Semantic)
                | (Kind::Kernel, Field::Semantic)
                | (Kind::Theorem, Field::Semantic)
                | (Kind::Artefact, Field::Bytes)
                | (Kind::Toolchain, Field::Bytes)
                | (Kind::Certificate, Field::Bytes)
                | (Kind::Theorem, Field::Bytes)
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Field {
    Semantic,
    Bytes,
}

/// How a dependent uses its dependency.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EdgeKind {
    /// Uses the dependency's meaning.
    Semantic,
    /// Is *about* the dependency's exact bytes.
    Subject,
    /// Was merely accepted by the dependency.
    Attestation,
}

impl EdgeKind {
    pub(crate) fn tag(self) -> u8 {
        match self {
            EdgeKind::Semantic => 1,
            EdgeKind::Subject => 2,
            EdgeKind::Attestation => 3,
        }
    }

    pub(crate) fn sigil(self) -> char {
        match self {
            EdgeKind::Semantic => 'm',
            EdgeKind::Subject => 's',
            EdgeKind::Attestation => 'a',
        }
    }

    pub(crate) fn parse_sigil(c: char) -> Option<Self> {
        Some(match c {
            'm' => EdgeKind::Semantic,
            's' => EdgeKind::Subject,
            'a' => EdgeKind::Attestation,
            _ => return None,
        })
    }
}

/// How badly an object is affected by a change. Ordered by increasing severity.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Severity {
    /// The proof stands; only the record of who accepted it is stale. Repair by replay.
    AttestationStale = 1,
    /// The statement is unchanged, but the theorem no longer covers the shipped bytes.
    SubjectChanged = 2,
    /// The dependencies' meanings changed. The proof must be redone.
    SemanticsChanged = 3,
}

// ---------------------------------------------------------------------------
// Nodes
// ---------------------------------------------------------------------------

/// A pinned dependency: this unit, at this digest, used in this way.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Pin {
    pub unit: UnitId,
    pub digest: Digest,
    pub kind: EdgeKind,
}

impl Pin {
    pub fn new(unit: UnitId, digest: Digest, kind: EdgeKind) -> Self {
        Pin { unit, digest, kind }
    }
}

/// An immutable evidence object.
///
/// `semantic` is the digest of the object's meaning (an alpha-normalised statement, a
/// normalised logic, a normalised ISA semantics). `bytes` is the digest of the exact
/// artefact (object file, checker binary, certificate). Keeping both is what makes
/// revocation precise.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Node {
    pub kind: Kind,
    pub semantic: Option<Digest>,
    pub bytes: Option<Digest>,
    pub pins: Vec<Pin>,
}

impl Node {
    pub fn new(
        kind: Kind,
        semantic: Option<Digest>,
        bytes: Option<Digest>,
        pins: Vec<Pin>,
    ) -> Self {
        Node {
            kind,
            semantic,
            bytes,
            pins,
        }
    }

    /// The single validity path shared by the parser and by [`Store::insert`] / [`Store::update`].
    ///
    /// Field requirements are part of a kind's meaning; a caller must not be able to build a
    /// node through the public API that a document could never contain. Duplicate pins are
    /// also refused here so that the API and the wire format agree exactly.
    pub fn validate(&self, unit: &UnitId) -> Result<(), Error> {
        if self.kind.requires(Field::Semantic) && self.semantic.is_none() {
            crate::refuse!(
                R11,
                Error::MissingRequiredField {
                    unit: unit.as_str().to_string(),
                    field: "semantic",
                }
            );
        }
        if self.kind.requires(Field::Bytes) && self.bytes.is_none() {
            crate::refuse!(
                R11,
                Error::MissingRequiredField {
                    unit: unit.as_str().to_string(),
                    field: "bytes",
                }
            );
        }

        let mut seen: BTreeSet<(u8, String, Digest)> = BTreeSet::new();
        for p in &self.pins {
            let key = (p.kind.tag(), p.unit.as_str().to_string(), p.digest);
            if !seen.insert(key) {
                crate::refuse!(
                    R13,
                    Error::DuplicatePin(format!("{}:{}", p.kind.sigil(), p.unit))
                );
            }
        }
        Ok(())
    }

    /// The object's identity: derived from content, including its typed pins.
    pub fn digest(&self) -> Digest {
        let mut c = Canon::new("tekmerion/evidence/v1");
        c.tag(self.kind.tag());
        c.opt_digest(self.semantic.as_ref());
        c.opt_digest(self.bytes.as_ref());

        let mut pins: Vec<&Pin> = self.pins.iter().collect();
        pins.sort_by_key(|p| (p.kind.tag(), p.unit.clone(), p.digest));

        c.bytes(&(pins.len() as u64).to_le_bytes());
        for p in pins {
            c.tag(p.kind.tag());
            c.bytes(p.unit.as_str().as_bytes());
            c.digest(&p.digest);
        }
        c.finish()
    }
}

// ---------------------------------------------------------------------------
// Store and revocation
// ---------------------------------------------------------------------------

/// An append-only evidence store.
#[derive(Clone, Debug, Default)]
pub struct Store {
    /// Current head of each unit: the head map.
    units: BTreeMap<UnitId, Digest>,
    /// First unit that declared each node digest. Kept forever so [`Store::unit_of`] can
    /// name historical nodes; it is not a head map.
    names: BTreeMap<Digest, UnitId>,
    /// The immutable node table. Superseded nodes stay here: dependents still pin them.
    nodes: BTreeMap<Digest, Node>,
    /// Node digest -> every unit that declared it. Identity is content-only, so identical
    /// content may legitimately appear under more than one unit name.
    declared: BTreeMap<Digest, BTreeSet<UnitId>>,
    /// dependency digest -> (dependent digest, how the dependent uses it)
    rev: BTreeMap<Digest, Vec<(Digest, EdgeKind)>>,
    /// dependency unit -> (dependent digest, pinned digest, how the dependent uses it).
    ///
    /// Keyed on the unit rather than the digest so that a second change to the same
    /// unit still reaches dependents that pin an older version.
    rev_unit: BTreeMap<UnitId, Vec<(Digest, Digest, EdgeKind)>>,
}

impl Store {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn node(&self, d: &Digest) -> Option<&Node> {
        self.nodes.get(d)
    }

    pub fn unit_named(&self, u: &UnitId) -> Option<Digest> {
        self.units.get(u).copied()
    }

    /// The first unit that declared this node digest, including superseded nodes.
    pub fn unit_of(&self, d: &Digest) -> Option<&UnitId> {
        self.names.get(d)
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// The head map, in canonical (unit-name) order.
    pub fn units(&self) -> impl Iterator<Item = (&UnitId, &Digest)> {
        self.units.iter()
    }

    /// The whole immutable node table, including superseded nodes.
    pub fn nodes(&self) -> impl Iterator<Item = (&Digest, &Node)> {
        self.nodes.iter()
    }

    pub(crate) fn declared_units(&self, d: &Digest) -> Option<&BTreeSet<UnitId>> {
        self.declared.get(d)
    }

    /// Every unit name that has at least one declared node, sorted and deduplicated.
    pub(crate) fn declared_unit_names(&self) -> Vec<UnitId> {
        let mut names: Vec<UnitId> = self
            .declared
            .values()
            .flat_map(|units| units.iter().cloned())
            .collect();
        names.sort();
        names.dedup();
        names
    }

    pub(crate) fn has_head(&self, unit: &UnitId) -> bool {
        self.units.contains_key(unit)
    }

    /// Direct dependents of a digest, with the edge kind that connects them.
    pub fn dependents_of(&self, d: &Digest) -> impl Iterator<Item = (Digest, EdgeKind)> + '_ {
        self.rev.get(d).into_iter().flat_map(|v| v.iter().copied())
    }

    /// Units in an order that satisfies every pin (dependencies first).
    ///
    /// The wire format is order-sensitive on purpose, so rendering must not rely on
    /// alphabetical luck.
    pub fn topological_units(&self) -> Vec<UnitId> {
        fn visit(store: &Store, u: &UnitId, seen: &mut BTreeSet<UnitId>, out: &mut Vec<UnitId>) {
            if !seen.insert(u.clone()) {
                return;
            }
            if let Some(d) = store.units.get(u) {
                if let Some(n) = store.nodes.get(d) {
                    let mut deps: Vec<UnitId> = n.pins.iter().map(|p| p.unit.clone()).collect();
                    deps.sort();
                    deps.dedup();
                    for dep in deps {
                        visit(store, &dep, seen, out);
                    }
                }
            }
            out.push(u.clone());
        }

        let mut keys: Vec<UnitId> = self.units.keys().cloned().collect();
        keys.sort();
        let mut seen = BTreeSet::new();
        let mut out = Vec::new();
        for k in keys {
            visit(self, &k, &mut seen, &mut out);
        }
        out
    }

    /// Every node digest in an order where pins name a digest emitted earlier.
    ///
    /// This is the order the v2 node table requires. Historical nodes are included, which
    /// is exactly what makes stale pins serialisable.
    pub(crate) fn topological_nodes(&self) -> Vec<Digest> {
        fn visit(store: &Store, d: Digest, seen: &mut BTreeSet<Digest>, out: &mut Vec<Digest>) {
            if !seen.insert(d) {
                return;
            }
            if let Some(node) = store.nodes.get(&d) {
                let mut deps: Vec<Digest> = node.pins.iter().map(|p| p.digest).collect();
                deps.sort();
                deps.dedup();
                for dep in deps {
                    visit(store, dep, seen, out);
                }
            }
            out.push(d);
        }

        let mut keys: Vec<Digest> = self.nodes.keys().copied().collect();
        keys.sort();
        let mut seen = BTreeSet::new();
        let mut out = Vec::new();
        for k in keys {
            visit(self, k, &mut seen, &mut out);
        }
        out
    }

    /// API-side pin rule: a new node pins each unit at that unit's current head.
    fn check_pins(&self, node: &Node) -> Result<(), Error> {
        for p in &node.pins {
            match self.units.get(&p.unit) {
                None => crate::refuse!(R07, Error::UnknownUnit(p.unit.as_str().to_string())),
                Some(current) if *current != p.digest => crate::refuse!(
                    R08,
                    Error::PinDigestNotDeclared(p.unit.as_str().to_string())
                ),
                Some(_) => {}
            }
        }
        Ok(())
    }

    /// Wire-side pin rule: a pin names a `(unit, digest)` node declared earlier.
    fn check_declared_pins(&self, node: &Node) -> Result<(), Error> {
        for p in &node.pins {
            let unit_known = self.declared.values().any(|units| units.contains(&p.unit));
            if !unit_known {
                crate::refuse!(R07, Error::UnknownUnit(p.unit.as_str().to_string()));
            }
            match self.declared.get(&p.digest) {
                Some(units) if units.contains(&p.unit) => {}
                _ => crate::refuse!(
                    R08,
                    Error::PinDigestNotDeclared(p.unit.as_str().to_string())
                ),
            }
        }
        Ok(())
    }

    /// Declare a unit for the first time.
    pub fn insert(&mut self, unit: UnitId, node: Node) -> Result<Digest, Error> {
        if self.units.contains_key(&unit) {
            crate::refuse!(R16, Error::DuplicateUnit(unit.as_str().to_string()));
        }
        node.validate(&unit)?;
        self.check_pins(&node)?;
        let d = node.digest();
        self.record(unit.clone(), d, node);
        self.units.insert(unit, d);
        Ok(d)
    }

    /// Add an immutable node to the table. Does not touch the head map.
    fn record(&mut self, unit: UnitId, d: Digest, node: Node) {
        for p in &node.pins {
            let dependents = self.rev.entry(p.digest).or_default();
            if !dependents.contains(&(d, p.kind)) {
                dependents.push((d, p.kind));
            }
            let by_unit = self.rev_unit.entry(p.unit.clone()).or_default();
            if !by_unit.contains(&(d, p.digest, p.kind)) {
                by_unit.push((d, p.digest, p.kind));
            }
        }
        self.declared.entry(d).or_default().insert(unit.clone());
        self.names.entry(d).or_insert(unit);
        self.nodes.insert(d, node);
    }

    /// Parser constructor: validate, require pins to be declared earlier, record only.
    pub(crate) fn declare_checked(&mut self, unit: UnitId, node: Node) -> Result<Digest, Error> {
        node.validate(&unit)?;
        self.check_declared_pins(&node)?;
        let d = node.digest();
        self.record(unit, d, node);
        Ok(d)
    }

    /// Parser head-map entry: exactly one head per unit, pointing at a declared node.
    pub(crate) fn declare_head(&mut self, unit: UnitId, d: Digest) -> Result<(), Error> {
        if self.units.contains_key(&unit) {
            crate::refuse!(R16, Error::DuplicateUnit(unit.as_str().to_string()));
        }
        let unit_known = self.declared.values().any(|units| units.contains(&unit));
        if !unit_known {
            crate::refuse!(R07, Error::UnknownUnit(unit.as_str().to_string()));
        }
        match self.declared.get(&d) {
            Some(units) if units.contains(&unit) => {}
            _ => crate::refuse!(R08, Error::PinDigestNotDeclared(unit.as_str().to_string())),
        }
        self.units.insert(unit, d);
        Ok(())
    }

    /// Replace a unit's content and return the **event delta**: everything this event
    /// made worse. This is [`AffectedByEvent`], not current status; see [`Store::status`].
    ///
    /// The caller does not need to say *what* changed: old and new nodes are compared.
    /// A change is classified per edge kind (see [`EdgeKind`]), which is the whole point
    /// of typing the edges.
    pub fn update(&mut self, unit: &UnitId, node: Node) -> Result<Revocation, Error> {
        let current = self.units.get(unit).copied();
        if current.is_none() {
            crate::refuse!(R15, Error::UnknownUnitOnUpdate(unit.as_str().to_string()));
        }
        let old_digest = current.unwrap_or(Digest([0u8; 32]));
        let old = self
            .nodes
            .get(&old_digest)
            .cloned()
            .ok_or(Error::Corrupt("unit points at a missing node"))?;
        if old.digest() != old_digest {
            return Err(Error::Corrupt("stored node does not match its identity"));
        }

        node.validate(unit)?;
        self.check_pins(&node)?;
        let new_digest = node.digest();

        let mut direct: Vec<(Digest, Severity)> = Vec::new();
        if let Some(dependents) = self.rev_unit.get(unit) {
            for (dependent, pinned_digest, kind) in dependents {
                let Some(pinned) = self.nodes.get(pinned_digest) else {
                    continue;
                };
                if let Some(sev) = classify(pinned, &node, *kind) {
                    direct.push((*dependent, sev));
                }
            }
        }

        self.record(unit.clone(), new_digest, node);
        self.units.insert(unit.clone(), new_digest);

        if crate::fault("L05") && new_digest != old_digest {
            // Fault L05: forget the superseded node, so dependents' stale pins no longer
            // resolve when status is recomputed or the store is rendered.
            self.nodes.remove(&old_digest);
        }

        let affected = self.propagate(direct);
        Ok(Revocation {
            unit: unit.clone(),
            old: old_digest,
            new: new_digest,
            affected,
        })
    }

    /// Spread severity to transitively dependent objects.
    ///
    /// `AttestationStale` does not propagate: if a checker binary was rebuilt, the
    /// theorems it accepted remain valid, and so does everything built on them.
    fn propagate(&self, seeds: Vec<(Digest, Severity)>) -> BTreeMap<Digest, Severity> {
        let mut out: BTreeMap<Digest, Severity> = BTreeMap::new();
        let mut work: Vec<(Digest, Severity)> = seeds;

        while let Some((d, sev)) = work.pop() {
            if out.get(&d).is_some_and(|prev| *prev >= sev) {
                continue;
            }
            out.insert(d, sev);
            if sev == Severity::AttestationStale {
                continue;
            }
            if let Some(deps) = self.rev.get(&d) {
                for (dependent, _kind) in deps {
                    match out.get(dependent) {
                        Some(prev) if *prev >= sev => {}
                        _ => work.push((*dependent, sev)),
                    }
                }
            }
        }
        out
    }

    /// **Effective** status of an object against the current head map.
    ///
    /// Walks every pin in the transitive closure, compares the pinned node with its
    /// unit's current head, classifies the comparison through the pin's edge kind, and
    /// takes the maximum. Memoised. The graph is acyclic by construction, but a visiting
    /// set is still used so a corrupt store cannot recurse forever.
    ///
    /// `None` means no revocation is effective (or the digest is not in this store).
    /// Admission decisions must use this, never a [`Revocation`] event delta.
    pub fn status(&self, d: &Digest) -> Option<Severity> {
        let mut memo: BTreeMap<Digest, Option<Severity>> = BTreeMap::new();
        let mut visiting: BTreeSet<Digest> = BTreeSet::new();
        self.status_inner(d, &mut memo, &mut visiting)
    }

    fn status_inner(
        &self,
        d: &Digest,
        memo: &mut BTreeMap<Digest, Option<Severity>>,
        visiting: &mut BTreeSet<Digest>,
    ) -> Option<Severity> {
        if let Some(cached) = memo.get(d) {
            return *cached;
        }
        if !visiting.insert(*d) {
            return None;
        }

        let mut worst: Option<Severity> = None;
        if let Some(node) = self.nodes.get(d) {
            // Fault L01: status ignores pins, so nothing is ever effectively revoked.
            if !crate::fault("L01") {
                for p in &node.pins {
                    // Fault L04: status ignores attestation edges.
                    if p.kind == EdgeKind::Attestation && crate::fault("L04") {
                        continue;
                    }

                    if let (Some(pinned), Some(head)) = (
                        self.nodes.get(&p.digest),
                        self.units.get(&p.unit).and_then(|h| self.nodes.get(h)),
                    ) {
                        if let Some(sev) = classify(pinned, head, p.kind) {
                            worst = Some(worst.map_or(sev, |w| w.max(sev)));
                        }
                    }

                    // Fault L03: stop at the direct edge (the event delta) instead of
                    // propagating effective status through the transitive closure.
                    if !crate::fault("L03") {
                        if let Some(deeper) = self.status_inner(&p.digest, memo, visiting) {
                            worst = Some(worst.map_or(deeper, |w| w.max(deeper)));
                        }
                    }
                }
            }
        }

        visiting.remove(d);
        memo.insert(*d, worst);
        worst
    }

    /// Admission: an object is admissible only when it exists and has no effective
    /// revocation. Reads [`Store::status`] only.
    pub fn is_admissible(&self, d: &Digest) -> bool {
        self.nodes.contains_key(d) && self.status(d).is_none()
    }
}

/// The class of a change, as seen through an edge of a given kind.
///
/// This names one event. It is deliberately not called "status": a bytes-only change is
/// a subject event through a [`EdgeKind::Subject`] edge and an attestation event through
/// an [`EdgeKind::Attestation`] edge.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ChangeEvent {
    Semantic,
    Subject,
    Attestation,
}

impl ChangeEvent {
    /// Detect the change event of `old` to `new` through an edge of kind `edge`.
    pub fn detect(old: &Node, new: &Node, edge: EdgeKind) -> Option<Self> {
        // A re-pin (same semantic and bytes, different pins) changes the derivation, so
        // it is a semantic event. Fault L02 drops this comparison.
        let pins_changed = !crate::fault("L02") && !pins_equal(&old.pins, &new.pins);
        if old.kind != new.kind || old.semantic != new.semantic || pins_changed {
            return Some(ChangeEvent::Semantic);
        }
        if old.bytes != new.bytes {
            return match edge {
                EdgeKind::Semantic => None,
                EdgeKind::Subject => Some(ChangeEvent::Subject),
                EdgeKind::Attestation => Some(ChangeEvent::Attestation),
            };
        }
        None
    }

    pub fn severity(self) -> Severity {
        match self {
            ChangeEvent::Semantic => Severity::SemanticsChanged,
            ChangeEvent::Subject => Severity::SubjectChanged,
            ChangeEvent::Attestation => Severity::AttestationStale,
        }
    }
}

/// Pins are a set, so order must not matter.
fn pins_equal(a: &[Pin], b: &[Pin]) -> bool {
    fn key(p: &Pin) -> (u8, UnitId, Digest) {
        (p.kind.tag(), p.unit.clone(), p.digest)
    }
    let mut x: Vec<_> = a.iter().map(key).collect();
    let mut y: Vec<_> = b.iter().map(key).collect();
    x.sort();
    y.sort();
    x == y
}

/// Classify a change of `old` to `new` as seen through an edge of kind `k`.
fn classify(old: &Node, new: &Node, k: EdgeKind) -> Option<Severity> {
    ChangeEvent::detect(old, new, k).map(ChangeEvent::severity)
}

/// The **per-event** result of one update: what that event made worse.
///
/// This is an event delta, not current status. See [`AffectedByEvent`]. After several
/// updates, the object with the worst effective status may not appear here at all (for
/// example, an unresolved subject mismatch followed by a checker rebuild reports only the
/// rebuild's attestation delta). Admission must use [`Store::status`].
#[derive(Clone, Debug)]
pub struct Revocation {
    pub unit: UnitId,
    pub old: Digest,
    pub new: Digest,
    /// Affected object digest -> severity, for this event only.
    pub affected: BTreeMap<Digest, Severity>,
}

/// The set of objects affected by one [`ChangeEvent`].
///
/// Alias of [`Revocation`], named for what it is: an event delta. It is not effective
/// status; use [`Store::status`] for admission.
pub type AffectedByEvent = Revocation;

impl Revocation {
    pub fn is_empty(&self) -> bool {
        self.affected.is_empty()
    }

    pub fn count(&self, s: Severity) -> usize {
        self.affected.values().filter(|v| **v == s).count()
    }

    pub fn worst(&self) -> Option<Severity> {
        self.affected.values().copied().max()
    }

    /// A one-line, receipt-shaped summary. Every number here is computed, never typed.
    pub fn summary(&self) -> String {
        format!(
            "{} {} -> {}: {} affected ({} semantic, {} subject, {} attestation)",
            self.unit,
            &self.old.hex()[..12],
            &self.new.hex()[..12],
            self.affected.len(),
            self.count(Severity::SemanticsChanged),
            self.count(Severity::SubjectChanged),
            self.count(Severity::AttestationStale),
        )
    }
}

/// Precision of a revocation set against known ground truth.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Score {
    /// Objects reported as affected.
    pub affected: usize,
    /// Objects that a dependency change actually broke.
    pub broken: usize,
    /// Objects that are both affected and broken.
    pub caught: usize,
}

impl Score {
    pub fn recall_ppm(&self) -> u32 {
        if self.broken == 0 {
            return 1_000_000;
        }
        ((self.caught as u64 * 1_000_000) / self.broken as u64) as u32
    }

    pub fn precision_ppm(&self) -> u32 {
        if self.affected == 0 {
            return 1_000_000;
        }
        ((self.caught as u64 * 1_000_000) / self.affected as u64) as u32
    }
}

impl Revocation {
    /// Score this revocation against a ground-truth set of genuinely broken objects.
    ///
    /// Soundness of revocation is exactly `recall == 1.0`: every broken object must be
    /// reported. Precision is an engineering quantity — over-approximation is safe but
    /// expensive.
    pub fn score(&self, broken: &std::collections::BTreeSet<Digest>) -> Score {
        let affected: std::collections::BTreeSet<Digest> = self.affected.keys().copied().collect();
        Score {
            affected: affected.len(),
            broken: broken.len(),
            caught: affected.intersection(broken).count(),
        }
    }
}
