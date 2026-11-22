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
//! 2. Construction is monotone: a node may only pin an already-declared unit at its
//!    current digest, so the graph is acyclic by construction.
//! 3. Every unrepresentable or ambiguous input is refused rather than coerced. Refusal
//!    sites carry stable identifiers (`R01`…`R16`) so that
//!    `tools/mutate.py` can prove each one is load-bearing.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use sha2::{Digest as _, Sha256};

pub mod wire;

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
    PinDigestNotCurrent(String),
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
            Error::PinDigestNotCurrent(u) => {
                write!(f, "R08: pin on {u:?} is not at its current digest")
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
    units: BTreeMap<UnitId, Digest>,
    names: BTreeMap<Digest, UnitId>,
    nodes: BTreeMap<Digest, Node>,
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

    pub fn unit_of(&self, d: &Digest) -> Option<&UnitId> {
        self.names.get(d)
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
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

    fn check_pins(&self, node: &Node) -> Result<(), Error> {
        for p in &node.pins {
            match self.units.get(&p.unit) {
                None => crate::refuse!(R07, Error::UnknownUnit(p.unit.as_str().to_string())),
                Some(current) if *current != p.digest => {
                    crate::refuse!(R08, Error::PinDigestNotCurrent(p.unit.as_str().to_string()))
                }
                Some(_) => {}
            }
        }
        Ok(())
    }

    /// Declare a unit for the first time.
    pub fn insert(&mut self, unit: UnitId, node: Node) -> Result<Digest, Error> {
        if self.units.contains_key(&unit) {
            crate::refuse!(R16, Error::DuplicateUnit(unit.as_str().to_string()));
        }
        self.check_pins(&node)?;
        let d = node.digest();
        self.apply(unit, d, node);
        Ok(d)
    }

    fn apply(&mut self, unit: UnitId, d: Digest, node: Node) {
        for p in &node.pins {
            self.rev.entry(p.digest).or_default().push((d, p.kind));
            self.rev_unit
                .entry(p.unit.clone())
                .or_default()
                .push((d, p.digest, p.kind));
        }
        self.names.insert(d, unit.clone());
        self.units.insert(unit, d);
        self.nodes.insert(d, node);
    }

    /// Replace a unit's content and return everything whose status changed.
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

        self.names.remove(&old_digest);
        self.apply(unit.clone(), new_digest, node);

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
}

/// Classify a change of `old` to `new` as seen through an edge of kind `k`.
fn classify(old: &Node, new: &Node, k: EdgeKind) -> Option<Severity> {
    if old.kind != new.kind || old.semantic != new.semantic {
        return Some(Severity::SemanticsChanged);
    }
    if old.bytes != new.bytes {
        return match k {
            EdgeKind::Semantic => None,
            EdgeKind::Subject => Some(Severity::SubjectChanged),
            EdgeKind::Attestation => Some(Severity::AttestationStale),
        };
    }
    None
}

/// The result of one change.
#[derive(Clone, Debug)]
pub struct Revocation {
    pub unit: UnitId,
    pub old: Digest,
    pub new: Digest,
    /// Affected object digest -> severity. Contains only objects whose recorded status changed.
    pub affected: BTreeMap<Digest, Severity>,
}

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
