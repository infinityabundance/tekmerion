//! Strict canonical certificate container.
//!
//! ```text
//! tekmerion-native-cert v1
//! theorem <step-id>
//! step <id> refl - <prop>
//! step <id> sym <premise> <prop>
//! step <id> trans <premise> <premise> <prop>
//! step <id> congr <premise> <prop>
//! step <id> bitblast <cnf-digest> <lrat-digest> <prop>
//! blob cnf <digest> <len> <lowercase-hex>
//! blob lrat <digest> <len> <lowercase-hex>
//! end
//! ```
//!
//! `theorem` names the step whose conclusion is the theorem. References may point
//! forward; cycles are therefore possible and are rejected by
//! [`Certificate::new`]. Steps are rendered in ascending id order and blobs in
//! `(kind, digest)` order; parsing accepts only that canonical rendering.
//!
//! A blob's digest is always recomputed. The declared digest and length are
//! checked, never trusted.

use std::collections::{BTreeMap, BTreeSet};

use crate::native::logic::{decode_hex, digest_bytes, encode_hex};
use crate::native::Error;
use crate::native::Term;
use crate::Digest;

pub const MAGIC: &str = "tekmerion-native-cert v1";

/// The kind of a payload blob.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BlobKind {
    Cnf,
    Lrat,
}

impl BlobKind {
    pub fn name(self) -> &'static str {
        match self {
            BlobKind::Cnf => "cnf",
            BlobKind::Lrat => "lrat",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "cnf" => Some(BlobKind::Cnf),
            "lrat" => Some(BlobKind::Lrat),
            _ => None,
        }
    }
}

/// A payload whose content identity is derived from its bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Blob {
    kind: BlobKind,
    digest: Digest,
    bytes: Vec<u8>,
}

impl Blob {
    pub fn new(kind: BlobKind, bytes: Vec<u8>) -> Self {
        let digest = digest_bytes(&bytes);
        Blob {
            kind,
            digest,
            bytes,
        }
    }

    pub fn kind(&self) -> BlobKind {
        self.kind
    }

    pub fn digest(&self) -> Digest {
        self.digest
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Digest used for a certificate payload. This is the identity a step names.
pub fn blob_digest(bytes: &[u8]) -> Digest {
    digest_bytes(bytes)
}

/// The trusted rule set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rule {
    /// `|- t = t`.
    Refl,
    /// `|- a = b` implies `|- b = a`.
    Sym,
    /// `|- a = b` and `|- b = c` imply `|- a = c`.
    Trans,
    /// `|- a = b` implies `|- f(..,a,..) = f(..,b,..)` for one `add`/`sub` position.
    Congr,
    /// Close any Boolean proposition by a checked bit-blast and LRAT refutation.
    Bitblast { cnf: Digest, lrat: Digest },
}

impl Rule {
    pub fn name(&self) -> &'static str {
        match self {
            Rule::Refl => "refl",
            Rule::Sym => "sym",
            Rule::Trans => "trans",
            Rule::Congr => "congr",
            Rule::Bitblast { .. } => "bitblast",
        }
    }

    pub fn arity(&self) -> usize {
        match self {
            Rule::Refl | Rule::Bitblast { .. } => 0,
            Rule::Sym | Rule::Congr => 1,
            Rule::Trans => 2,
        }
    }
}

/// One derivation step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    id: u32,
    rule: Rule,
    premises: Vec<u32>,
    conclusion: Term,
}

impl Step {
    pub fn refl(id: u32, conclusion: Term) -> Self {
        Step {
            id,
            rule: Rule::Refl,
            premises: Vec::new(),
            conclusion,
        }
    }

    pub fn sym(id: u32, premise: u32, conclusion: Term) -> Self {
        Step {
            id,
            rule: Rule::Sym,
            premises: vec![premise],
            conclusion,
        }
    }

    pub fn trans(id: u32, first: u32, second: u32, conclusion: Term) -> Self {
        Step {
            id,
            rule: Rule::Trans,
            premises: vec![first, second],
            conclusion,
        }
    }

    pub fn congr(id: u32, premise: u32, conclusion: Term) -> Self {
        Step {
            id,
            rule: Rule::Congr,
            premises: vec![premise],
            conclusion,
        }
    }

    pub fn bitblast(id: u32, cnf: Digest, lrat: Digest, conclusion: Term) -> Self {
        Step {
            id,
            rule: Rule::Bitblast { cnf, lrat },
            premises: Vec::new(),
            conclusion,
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn rule(&self) -> &Rule {
        &self.rule
    }

    pub fn premises(&self) -> &[u32] {
        &self.premises
    }

    pub fn conclusion(&self) -> &Term {
        &self.conclusion
    }

    fn render(&self) -> String {
        let mut s = format!("step {} {}", self.id, self.rule.name());
        match &self.rule {
            Rule::Refl => s.push_str(" -"),
            Rule::Sym => s.push_str(&format!(" {}", self.premises[0])),
            Rule::Trans => s.push_str(&format!(" {} {}", self.premises[0], self.premises[1])),
            Rule::Congr => s.push_str(&format!(" {}", self.premises[0])),
            Rule::Bitblast { cnf, lrat } => s.push_str(&format!(" {} {}", cnf.hex(), lrat.hex())),
        }
        s.push(' ');
        s.push_str(&self.conclusion.render());
        s
    }
}

/// A parsed and validated certificate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Certificate {
    theorem: u32,
    steps: Vec<Step>,
    blobs: Vec<Blob>,
}

impl Certificate {
    /// Construct from parts, validating structure, acyclicity, and blob references.
    pub fn new(theorem: u32, mut steps: Vec<Step>, mut blobs: Vec<Blob>) -> Result<Self, Error> {
        steps.sort_by_key(|s| s.id);
        blobs.sort_by_key(|b| (b.kind, b.digest));

        for pair in steps.windows(2) {
            if pair[0].id == pair[1].id {
                return Err(Error::DuplicateStep(pair[0].id));
            }
        }
        let mut seen_blobs: BTreeSet<Digest> = BTreeSet::new();
        for blob in &blobs {
            if !seen_blobs.insert(blob.digest) {
                return Err(Error::DuplicateBlob(blob.digest));
            }
        }
        for blob in &blobs {
            if blob.digest != digest_bytes(&blob.bytes) {
                return Err(Error::BlobDigestMismatch(blob.digest));
            }
        }

        let cert = Certificate {
            theorem,
            steps,
            blobs,
        };
        cert.validate()?;
        Ok(cert)
    }

    pub fn theorem_id(&self) -> u32 {
        self.theorem
    }

    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    pub fn blobs(&self) -> &[Blob] {
        &self.blobs
    }

    pub fn step(&self, id: u32) -> Option<&Step> {
        self.steps.iter().find(|s| s.id == id)
    }

    pub fn blob(&self, digest: Digest, kind: BlobKind) -> Result<&Blob, Error> {
        let blob = self
            .blobs
            .iter()
            .find(|b| b.digest == digest)
            .ok_or(Error::MissingBlob(digest))?;
        if blob.kind != kind {
            return Err(Error::BlobKindMismatch {
                expected: kind.name(),
                found: blob.kind.name(),
            });
        }
        Ok(blob)
    }

    /// Structural validation: ids, references, arities, rule shapes, cycles, and
    /// blob presence. Does not run unit propagation.
    fn validate(&self) -> Result<(), Error> {
        let ids: BTreeSet<u32> = self.steps.iter().map(|s| s.id).collect();
        if !ids.contains(&self.theorem) {
            return Err(Error::MissingTheorem(self.theorem));
        }
        for step in &self.steps {
            if step.premises.len() != step.rule.arity() {
                return Err(Error::WrongPremiseCount {
                    rule: step.rule.name(),
                    expected: step.rule.arity(),
                    found: step.premises.len(),
                });
            }
            for &p in &step.premises {
                if !ids.contains(&p) {
                    return Err(Error::DanglingRef(p));
                }
            }
            step.conclusion.validate()?;
            if step.conclusion.sort() != crate::native::Sort::Bool {
                return Err(Error::RuleMismatch("a step conclusion must be Boolean"));
            }
            match &step.rule {
                Rule::Refl => match &step.conclusion {
                    Term::Eq { left, right, .. } if left == right => {}
                    _ => {
                        return Err(Error::RuleMismatch(
                            "refl requires an equality with identical sides",
                        ))
                    }
                },
                Rule::Sym | Rule::Trans | Rule::Congr => {
                    if !matches!(step.conclusion, Term::Eq { .. }) {
                        return Err(Error::RuleMismatch(
                            "this rule requires an equality conclusion",
                        ));
                    }
                }
                Rule::Bitblast { .. } => {}
            }
        }

        // Kahn's algorithm: deterministic topological order and cycle detection.
        let mut indegree: BTreeMap<u32, usize> = BTreeMap::new();
        let mut dependents: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for step in &self.steps {
            indegree.insert(step.id, step.premises.len());
            for &p in &step.premises {
                dependents.entry(p).or_default().push(step.id);
            }
        }
        let mut ready: BTreeSet<u32> = indegree
            .iter()
            .filter(|(_, &d)| d == 0)
            .map(|(&id, _)| id)
            .collect();
        let mut seen = 0usize;
        while let Some(&id) = ready.iter().next() {
            ready.remove(&id);
            seen += 1;
            if let Some(ds) = dependents.get(&id) {
                for &d in ds {
                    let entry = indegree.get_mut(&d).expect("dependent exists");
                    *entry -= 1;
                    if *entry == 0 {
                        ready.insert(d);
                    }
                }
            }
        }
        if seen != self.steps.len() {
            let stuck = indegree
                .iter()
                .find(|(_, &d)| d > 0)
                .map(|(&id, _)| id)
                .unwrap_or(self.theorem);
            return Err(Error::Cycle(stuck));
        }

        // Blob references must be present, used, and used under the right kind.
        let mut referenced: BTreeMap<Digest, BlobKind> = BTreeMap::new();
        for step in &self.steps {
            if let Rule::Bitblast { cnf, lrat } = &step.rule {
                if let Some(existing) = referenced.insert(*cnf, BlobKind::Cnf) {
                    if existing != BlobKind::Cnf {
                        return Err(Error::BlobKindMismatch {
                            expected: "cnf",
                            found: "lrat",
                        });
                    }
                }
                if let Some(existing) = referenced.insert(*lrat, BlobKind::Lrat) {
                    if existing != BlobKind::Lrat {
                        return Err(Error::BlobKindMismatch {
                            expected: "lrat",
                            found: "cnf",
                        });
                    }
                }
            }
        }
        for (digest, kind) in &referenced {
            let blob = self
                .blobs
                .iter()
                .find(|b| b.digest == *digest)
                .ok_or(Error::MissingBlob(*digest))?;
            if blob.kind != *kind {
                return Err(Error::BlobKindMismatch {
                    expected: kind.name(),
                    found: blob.kind.name(),
                });
            }
        }
        for blob in &self.blobs {
            if !referenced.contains_key(&blob.digest) {
                return Err(Error::UnknownBlob(blob.digest));
            }
        }
        Ok(())
    }

    /// A deterministic topological order of all steps.
    pub(crate) fn topological_order(&self) -> Result<Vec<u32>, Error> {
        self.validate()?;
        let mut indegree: BTreeMap<u32, usize> = BTreeMap::new();
        let mut dependents: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for step in &self.steps {
            indegree.insert(step.id, step.premises.len());
            for &p in &step.premises {
                dependents.entry(p).or_default().push(step.id);
            }
        }
        let mut ready: BTreeSet<u32> = indegree
            .iter()
            .filter(|(_, &d)| d == 0)
            .map(|(&id, _)| id)
            .collect();
        let mut order = Vec::with_capacity(self.steps.len());
        while let Some(&id) = ready.iter().next() {
            ready.remove(&id);
            order.push(id);
            if let Some(ds) = dependents.get(&id) {
                for &d in ds {
                    let entry = indegree.get_mut(&d).expect("dependent exists");
                    *entry -= 1;
                    if *entry == 0 {
                        ready.insert(d);
                    }
                }
            }
        }
        Ok(order)
    }

    /// Canonical rendering.
    pub fn render(&self) -> String {
        let mut s = String::new();
        s.push_str(MAGIC);
        s.push('\n');
        s.push_str(&format!("theorem {}\n", self.theorem));
        for step in &self.steps {
            s.push_str(&step.render());
            s.push('\n');
        }
        for blob in &self.blobs {
            s.push_str(&format!(
                "blob {} {} {} {}\n",
                blob.kind.name(),
                blob.digest.hex(),
                blob.bytes.len(),
                encode_hex(&blob.bytes)
            ));
        }
        s.push_str("end\n");
        s
    }

    /// Strict canonical parser.
    pub fn parse(input: &str) -> Result<Self, Error> {
        if !input.ends_with('\n') {
            return Err(Error::Truncated);
        }
        if input.len() == 1 {
            return Err(Error::Truncated);
        }
        let body = &input[..input.len() - 1];
        let lines: Vec<&str> = body.split('\n').collect();
        let magic = *lines.first().ok_or(Error::Truncated)?;
        if magic != MAGIC {
            return Err(Error::BadCertificateMagic);
        }
        if lines.len() < 2 {
            return Err(Error::Truncated);
        }
        let theorem_line = lines[1];
        let rest = theorem_line
            .strip_prefix("theorem ")
            .ok_or(Error::MalformedCertificate("expected theorem line"))?;
        let theorem = parse_u32(rest)?;

        let mut steps = Vec::new();
        let mut blobs = Vec::new();
        let mut ended = false;
        for line in &lines[2..] {
            if *line == "end" {
                ended = true;
                break;
            }
            if let Some(rest) = line.strip_prefix("step ") {
                steps.push(parse_step(rest)?);
            } else if let Some(rest) = line.strip_prefix("blob ") {
                blobs.push(parse_blob(rest)?);
            } else {
                return Err(Error::UnknownRecord((*line).to_string()));
            }
        }
        if !ended {
            return Err(Error::Truncated);
        }
        if lines.last().map(|l| *l != "end").unwrap_or(true) {
            return Err(Error::TrailingGarbage);
        }
        let end_index = lines.iter().position(|l| *l == "end").unwrap_or(0);
        if end_index + 1 != lines.len() {
            return Err(Error::TrailingGarbage);
        }

        let cert = Certificate::new(theorem, steps, blobs)?;
        if cert.render() != input {
            return Err(Error::NonCanonicalCertificate);
        }
        Ok(cert)
    }
}

fn parse_step(s: &str) -> Result<Step, Error> {
    let (id_s, rest) = take_field(s)?;
    let id = parse_u32(id_s)?;
    let (rule_s, rest) = take_field(rest)?;
    match rule_s {
        "refl" => {
            let (p, rest) = take_field(rest)?;
            if p != "-" {
                return Err(Error::MalformedCertificate(
                    "refl takes '-' where premises would go",
                ));
            }
            let conclusion = Term::parse(rest)?;
            Ok(Step::refl(id, conclusion))
        }
        "sym" => {
            let (p, rest) = take_field(rest)?;
            let premise = parse_u32(p)?;
            let conclusion = Term::parse(rest)?;
            Ok(Step::sym(id, premise, conclusion))
        }
        "trans" => {
            let (p1, rest) = take_field(rest)?;
            let (p2, rest) = take_field(rest)?;
            let conclusion = Term::parse(rest)?;
            Ok(Step::trans(id, parse_u32(p1)?, parse_u32(p2)?, conclusion))
        }
        "congr" => {
            let (p, rest) = take_field(rest)?;
            let premise = parse_u32(p)?;
            let conclusion = Term::parse(rest)?;
            Ok(Step::congr(id, premise, conclusion))
        }
        "bitblast" => {
            let (cnf_s, rest) = take_field(rest)?;
            let (lrat_s, rest) = take_field(rest)?;
            let cnf = Digest::parse_hex(cnf_s)
                .map_err(|_| Error::MalformedCertificate("bad CNF digest"))?;
            let lrat = Digest::parse_hex(lrat_s)
                .map_err(|_| Error::MalformedCertificate("bad LRAT digest"))?;
            let conclusion = Term::parse(rest)?;
            Ok(Step::bitblast(id, cnf, lrat, conclusion))
        }
        other => Err(Error::UnknownRule(other.to_string())),
    }
}

fn parse_blob(s: &str) -> Result<Blob, Error> {
    let (kind_s, rest) = take_field(s)?;
    let kind = BlobKind::parse(kind_s).ok_or(Error::UnknownRecord(kind_s.to_string()))?;
    let (digest_s, rest) = take_field(rest)?;
    let digest =
        Digest::parse_hex(digest_s).map_err(|_| Error::MalformedCertificate("bad blob digest"))?;
    let (len_s, hex) = take_field(rest)?;
    let declared = parse_usize(len_s)?;
    let bytes = decode_hex(hex)?;
    if bytes.len() != declared {
        return Err(Error::BlobLengthMismatch {
            declared,
            actual: bytes.len(),
        });
    }
    if digest_bytes(&bytes) != digest {
        return Err(Error::BlobDigestMismatch(digest));
    }
    Ok(Blob {
        kind,
        digest,
        bytes,
    })
}

fn take_field(s: &str) -> Result<(&str, &str), Error> {
    if s.is_empty() {
        return Err(Error::MalformedCertificate("missing field"));
    }
    match s.find(' ') {
        Some(i) => {
            if i == 0 {
                return Err(Error::MalformedCertificate("empty field"));
            }
            Ok((&s[..i], &s[i + 1..]))
        }
        None => Ok((s, "")),
    }
}

fn parse_u32(s: &str) -> Result<u32, Error> {
    if !canonical_digits(s) {
        return Err(Error::BadNumber(s.to_string()));
    }
    s.parse::<u32>()
        .map_err(|_| Error::BadNumber(s.to_string()))
}

fn parse_usize(s: &str) -> Result<usize, Error> {
    if !canonical_digits(s) {
        return Err(Error::BadNumber(s.to_string()));
    }
    s.parse::<usize>()
        .map_err(|_| Error::BadNumber(s.to_string()))
}

fn canonical_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) && (s.len() == 1 || !s.starts_with('0'))
}
