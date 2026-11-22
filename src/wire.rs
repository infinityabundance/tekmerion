//! Strict, canonical wire format for an evidence document.
//!
//! ```text
//! TEKMERION-EVIDENCE v2
//! node<TAB>kind<TAB>unit<TAB>semantic<TAB>bytes<TAB>pins
//! head<TAB>unit<TAB>hex-digest
//! ```
//!
//! The document is an **immutable node table plus a head map**. A `node` line declares one
//! immutable node; a unit may have several (its history). The node's identity is derived
//! from its content, so the line does not carry its own digest. A `head` line names the
//! current node of a unit.
//!
//! `semantic` and `bytes` are either `-` or exactly 64 hex characters. `pins` is either
//! empty or a comma-separated list of `SIGIL:unit@hex`, where the sigil is `m` (semantic),
//! `s` (subject) or `a` (attestation).
//!
//! Pins must reference a `(unit, digest)` node declared **earlier**, and heads come after
//! every node. This makes serialising a store with stale pins possible — the historical
//! node the pin names is still in the table — while keeping the graph acyclic by
//! construction rather than by a separate check.
//!
//! The parser refuses anything it cannot interpret. It never guesses, never skips a field,
//! and never repairs a malformed digest.

use crate::{Digest, EdgeKind, Error, Kind, Node, Pin, Store, UnitId};

pub const MAGIC: &str = "TEKMERION-EVIDENCE v2";

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
    let mut heads_started = false;

    for line in lines {
        if line.is_empty() {
            crate::refuse!(R03, Error::WrongFieldCount(line.to_string()));
        }

        let fields: Vec<&str> = line.split('\t').collect();
        match fields[0] {
            "node" => {
                if heads_started {
                    crate::refuse!(R03, Error::RecordAfterHead);
                }
                if fields.len() != 6 {
                    crate::refuse!(R03, Error::WrongFieldCount(line.to_string()));
                }

                let kind_opt = Kind::parse(fields[1]);
                if kind_opt.is_none() {
                    crate::refuse!(R10, Error::UnknownKind(fields[1].to_string()));
                }
                let kind = kind_opt.unwrap_or(Kind::Spec);

                let unit = UnitId::new(fields[2])?;
                let semantic = opt_digest(fields[3])?;
                let bytes = opt_digest(fields[4])?;
                let pins = parse_pins(fields[5])?;

                // One validation path, shared with Store::insert/update.
                store.declare_checked(unit, Node::new(kind, semantic, bytes, pins))?;
            }
            "head" => {
                heads_started = true;
                if fields.len() != 3 {
                    crate::refuse!(R03, Error::WrongFieldCount(line.to_string()));
                }
                let unit = UnitId::new(fields[1])?;
                let digest = Digest::parse_hex(fields[2])?;
                store.declare_head(unit, digest)?;
            }
            tag => {
                crate::refuse!(R10, Error::UnknownRecord(tag.to_string()));
            }
        }
    }

    // A node table without a complete head map could make a stale pin look clean: there
    // would be no current version to compare it with. Require exactly one head per unit.
    for unit in store.declared_unit_names() {
        if !store.has_head(&unit) {
            crate::refuse!(R03, Error::MissingHead(unit.as_str().to_string()));
        }
    }

    Ok(store)
}

fn opt_digest(field: &str) -> Result<Option<Digest>, Error> {
    if field == "-" {
        return Ok(None);
    }
    Digest::parse_hex(field).map(Some)
}

fn parse_pins(field: &str) -> Result<Vec<Pin>, Error> {
    let mut pins = Vec::new();
    if !field.is_empty() {
        for item in field.split(',') {
            pins.push(parse_pin(item)?);
        }
    }
    Ok(pins)
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

/// Serialise a store: every immutable node in topological order, then the head map.
///
/// Superseded nodes are emitted too, which is what lets dependents keep their stale pins.
pub fn render(store: &Store) -> String {
    let mut out = String::new();
    out.push_str(MAGIC);
    out.push('\n');

    for d in store.topological_nodes() {
        let Some(node) = store.node(&d) else {
            continue;
        };
        let Some(units) = store.declared_units(&d) else {
            continue;
        };
        for unit in units {
            out.push_str("node\t");
            out.push_str(node.kind.name());
            out.push('\t');
            out.push_str(unit.as_str());
            out.push('\t');
            out.push_str(&render_opt(node.semantic.as_ref()));
            out.push('\t');
            out.push_str(&render_opt(node.bytes.as_ref()));
            out.push('\t');
            out.push_str(&render_pins(&node.pins));
            out.push('\n');
        }
    }

    for (unit, d) in store.units() {
        out.push_str("head\t");
        out.push_str(unit.as_str());
        out.push('\t');
        out.push_str(&d.hex());
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

fn render_pins(pins: &[Pin]) -> String {
    let mut pins: Vec<&Pin> = pins.iter().collect();
    pins.sort_by_key(|p| (p.kind.tag(), p.unit.clone(), p.digest));
    let mut out = String::new();
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
    out
}
