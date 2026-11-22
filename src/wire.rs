//! Strict, canonical wire format for an evidence document.
//!
//! ```text
//! TEKMERION-EVIDENCE v1
//! kind<TAB>unit<TAB>semantic<TAB>bytes<TAB>pins
//! ```
//!
//! `semantic` and `bytes` are either `-` or exactly 64 hex characters. `pins` is either
//! empty or a comma-separated list of `SIGIL:unit@hex`, where the sigil is `m` (semantic),
//! `s` (subject) or `a` (attestation).
//!
//! The parser refuses anything it cannot interpret. It never guesses, never skips a field,
//! and never repairs a malformed digest. Construction order is enforced by [`Store::insert`],
//! so a document may only pin units declared above it — which makes the graph acyclic by
//! construction rather than by a separate check.

use std::collections::BTreeSet;

use crate::{Digest, EdgeKind, Error, Field, Kind, Node, Pin, Store, UnitId};

pub const MAGIC: &str = "TEKMERION-EVIDENCE v1";

/// Parse a complete document.
pub fn parse(text: &str) -> Result<Store, Error> {
    if !text.ends_with('\n') {
        crate::refuse!(R14, Error::TrailingBytes);
    }
    let body = &text[..text.len() - 1];

    let mut lines = body.split('\n');
    let head = lines.next().unwrap_or("");
    if head != MAGIC {
        if head.starts_with("TEKMERION-EVIDENCE v") {
            crate::refuse!(R02, Error::UnsupportedVersion(head.to_string()));
        }
        crate::refuse!(R01, Error::BadMagic);
    }

    let mut store = Store::new();

    for line in lines {
        if line.is_empty() {
            crate::refuse!(R03, Error::WrongFieldCount(line.to_string()));
        }

        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != 5 {
            crate::refuse!(R03, Error::WrongFieldCount(line.to_string()));
        }

        let kind_opt = Kind::parse(fields[0]);
        if kind_opt.is_none() {
            crate::refuse!(R10, Error::UnknownKind(fields[0].to_string()));
        }
        let kind = kind_opt.unwrap_or(Kind::Spec);

        let unit = UnitId::new(fields[1])?;
        let semantic = opt_digest(fields[2])?;
        let bytes = opt_digest(fields[3])?;

        if kind.requires(Field::Semantic) && semantic.is_none() {
            crate::refuse!(
                R11,
                Error::MissingRequiredField {
                    unit: unit.as_str().to_string(),
                    field: "semantic",
                }
            );
        }
        if kind.requires(Field::Bytes) && bytes.is_none() {
            crate::refuse!(
                R11,
                Error::MissingRequiredField {
                    unit: unit.as_str().to_string(),
                    field: "bytes",
                }
            );
        }

        let mut pins: Vec<Pin> = Vec::new();
        if !fields[4].is_empty() {
            for item in fields[4].split(',') {
                pins.push(parse_pin(item)?);
            }
        }

        let mut seen: BTreeSet<(u8, String, Digest)> = BTreeSet::new();
        for p in &pins {
            let key = (p.kind.tag(), p.unit.as_str().to_string(), p.digest);
            if !seen.insert(key) {
                crate::refuse!(
                    R13,
                    Error::DuplicatePin(format!("{}:{}", p.kind.sigil(), p.unit))
                );
            }
        }

        store.insert(unit, Node::new(kind, semantic, bytes, pins))?;
    }

    Ok(store)
}

fn opt_digest(field: &str) -> Result<Option<Digest>, Error> {
    if field == "-" {
        return Ok(None);
    }
    Digest::parse_hex(field).map(Some)
}

fn parse_pin(item: &str) -> Result<Pin, Error> {
    let pair = item.split_once(':');
    if pair.is_none() {
        crate::refuse!(R09, Error::UnknownEdgeKind(item.to_string()));
    }
    let (sigil, rest) = pair.unwrap_or(("", ""));

    let mut chars = sigil.chars();
    let first = chars.next();
    if first.is_none() || chars.next().is_some() {
        crate::refuse!(R09, Error::UnknownEdgeKind(sigil.to_string()));
    }
    let edge_opt = EdgeKind::parse_sigil(first.unwrap_or('?'));
    if edge_opt.is_none() {
        crate::refuse!(R09, Error::UnknownEdgeKind(sigil.to_string()));
    }
    let edge = edge_opt.unwrap_or(EdgeKind::Semantic);

    let split = rest.split_once('@');
    if split.is_none() {
        crate::refuse!(R09, Error::UnknownEdgeKind(item.to_string()));
    }
    let (unit_name, hex) = split.unwrap_or(("", ""));

    let unit = UnitId::new(unit_name)?;
    let digest = Digest::parse_hex(hex)?;
    Ok(Pin::new(unit, digest, edge))
}

/// Serialise a store, in canonical order, round-tripping the parser.
pub fn render(store: &Store) -> String {
    let mut out = String::new();
    out.push_str(MAGIC);
    out.push('\n');
    for unit in store.topological_units() {
        let Some(d) = store.unit_named(&unit) else {
            continue;
        };
        let Some(node) = store.node(&d) else { continue };
        out.push_str(node.kind.name());
        out.push('\t');
        out.push_str(unit.as_str());
        out.push('\t');
        out.push_str(&render_opt(node.semantic.as_ref()));
        out.push('\t');
        out.push_str(&render_opt(node.bytes.as_ref()));
        out.push('\t');
        let mut pins = node.pins.clone();
        pins.sort_by_key(|p| (p.kind.tag(), p.unit.clone(), p.digest));
        for (i, p) in pins.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push(p.kind.sigil());
            out.push(':');
            out.push_str(p.unit.as_str());
            out.push('@');
            out.push_str(&p.digest.hex());
        }
        out.push('\n');
    }
    out
}

fn render_opt(d: Option<&Digest>) -> String {
    match d {
        None => "-".to_string(),
        Some(x) => x.hex(),
    }
}
