//! One test per refusal site, plus checks that the public API and the parser agree.
//!
//! `tools/mutate.py` disables each site in turn; every disabling must be noticed here.
//! A site whose removal no test detects is a vacuous check and must be fixed or deleted.

use tekmerion::wire;
use tekmerion::{Digest, EdgeKind, Error, Kind, Node, Pin, Store, UnitId};

fn hx(s: &str) -> String {
    Digest::of_label(s).hex()
}

fn unit(s: &str) -> UnitId {
    UnitId::new(s).unwrap()
}

fn spec_node() -> Node {
    Node::new(
        Kind::Spec,
        Some(Digest::of_label("spec-sem")),
        Some(Digest::of_label("spec-src")),
        vec![],
    )
}

fn kernel_node() -> Node {
    Node::new(
        Kind::Kernel,
        Some(Digest::of_label("logic")),
        Some(Digest::of_label("kbin")),
        vec![],
    )
}

fn theorem_node(sd: Digest, kd: Digest) -> Node {
    Node::new(
        Kind::Theorem,
        Some(Digest::of_label("thm-stmt")),
        Some(Digest::of_label("thm-cert")),
        vec![
            Pin::new(unit("spec/redc"), sd, EdgeKind::Semantic),
            Pin::new(unit("hol/kernel"), kd, EdgeKind::Attestation),
        ],
    )
}

/// A minimal valid v2 document: immutable nodes in topological order, then heads.
fn valid_doc() -> String {
    let sd = spec_node().digest();
    let kd = kernel_node().digest();
    let td = theorem_node(sd, kd).digest();
    format!(
        "{magic}\n\
         node\tspec\tspec/redc\t{sem}\t{src}\t\n\
         node\tkernel\thol/kernel\t{kl}\t{kb}\t\n\
         node\tthm\tredc/correct\t{ts}\t{tc}\tm:spec/redc@{sd},a:hol/kernel@{kd}\n\
         head\tspec/redc\t{sd}\n\
         head\thol/kernel\t{kd}\n\
         head\tredc/correct\t{td}\n",
        magic = wire::MAGIC,
        sem = hx("spec-sem"),
        src = hx("spec-src"),
        kl = hx("logic"),
        kb = hx("kbin"),
        ts = hx("thm-stmt"),
        tc = hx("thm-cert"),
        sd = sd.hex(),
        kd = kd.hex(),
        td = td.hex(),
    )
}

fn expect(text: &str, pred: fn(&Error) -> bool) {
    match wire::parse(text) {
        Err(e) if pred(&e) => {}
        Err(e) => panic!("wrong refusal: {e}"),
        Ok(_) => panic!("document was accepted but should have been refused"),
    }
}

fn node_line(unit: &UnitId, node: &Node) -> String {
    let opt = |d: Option<Digest>| match d {
        Some(x) => x.hex(),
        None => "-".to_string(),
    };
    let sigil = |k: EdgeKind| match k {
        EdgeKind::Semantic => 'm',
        EdgeKind::Subject => 's',
        EdgeKind::Attestation => 'a',
    };
    let mut pins: Vec<String> = node
        .pins
        .iter()
        .map(|p| format!("{}:{}@{}", sigil(p.kind), p.unit, p.digest.hex()))
        .collect();
    pins.sort();
    format!(
        "node\t{}\t{}\t{}\t{}\t{}",
        node.kind.name(),
        unit.as_str(),
        opt(node.semantic),
        opt(node.bytes),
        pins.join(",")
    )
}

#[test]
fn accepts_valid_document() {
    let s = wire::parse(&valid_doc()).expect("valid document");
    assert_eq!(s.len(), 3);
    assert_eq!(s.units().count(), 3);
}

#[test]
fn r01_bad_magic() {
    let doc = valid_doc().replacen(wire::MAGIC, "SOMETHING-ELSE v1", 1);
    expect(&doc, |e| matches!(e, Error::BadMagic));
}

#[test]
fn r02_unsupported_version() {
    let doc = valid_doc().replacen(wire::MAGIC, "TEKMERION-EVIDENCE v1", 1);
    expect(&doc, |e| matches!(e, Error::UnsupportedVersion(_)));
}

#[test]
fn r03_wrong_field_count() {
    // Drop the (empty) pin column from the first node line.
    let doc = valid_doc().replacen("\t\n", "\n", 1);
    expect(&doc, |e| matches!(e, Error::WrongFieldCount(_)));
}

#[test]
fn r03_blank_line() {
    let text = valid_doc();
    let mut lines: Vec<&str> = text.split('\n').collect();
    lines.insert(1, "");
    expect(&lines.join("\n"), |e| {
        matches!(e, Error::WrongFieldCount(_))
    });
}

#[test]
fn r03_node_after_head_map() {
    let doc = format!(
        "{}node\tspec\tspec/redc\t{}\t{}\t\n",
        valid_doc(),
        hx("other-sem"),
        hx("other-src"),
    );
    expect(&doc, |e| matches!(e, Error::RecordAfterHead));
}

#[test]
fn r03_node_without_head() {
    let doc = format!(
        "{}\nnode\tspec\tspec/redc\t{}\t{}\t\n",
        wire::MAGIC,
        hx("spec-sem"),
        hx("spec-src"),
    );
    expect(
        &doc,
        |e| matches!(e, Error::MissingHead(u) if u.as_str() == "spec/redc"),
    );
}

#[test]
fn r04_empty_unit_name() {
    let doc = valid_doc().replacen("node\tspec\tspec/redc", "node\tspec\t", 1);
    expect(&doc, |e| matches!(e, Error::EmptyName));
}

#[test]
fn r05_bad_hex() {
    let doc = valid_doc().replacen(&hx("spec-src"), &"g".repeat(64), 1);
    expect(&doc, |e| matches!(e, Error::BadHex));
}

#[test]
fn r06_wrong_digest_length() {
    let doc = valid_doc().replacen(&hx("spec-src"), &"a".repeat(63), 1);
    expect(&doc, |e| matches!(e, Error::WrongDigestLength(63)));
}

#[test]
fn r07_unknown_unit_pin() {
    let doc = valid_doc().replacen("m:spec/redc@", "m:spec/undeclared@", 1);
    expect(
        &doc,
        |e| matches!(e, Error::UnknownUnit(u) if u.as_str() == "spec/undeclared"),
    );
}

#[test]
fn r07_unknown_unit_head() {
    let doc = valid_doc().replacen("head\tspec/redc", "head\tspec/nowhere", 1);
    expect(
        &doc,
        |e| matches!(e, Error::UnknownUnit(u) if u.as_str() == "spec/nowhere"),
    );
}

#[test]
fn r08_pin_digest_not_declared() {
    let sd = spec_node().digest();
    let doc = valid_doc().replacen(
        &format!("m:spec/redc@{}", sd.hex()),
        &format!("m:spec/redc@{}", hx("stale")),
        1,
    );
    expect(&doc, |e| matches!(e, Error::PinDigestNotDeclared(_)));
}

#[test]
fn r08_head_digest_not_declared_for_unit() {
    let sd = spec_node().digest();
    let doc = valid_doc().replacen(
        &format!("head\tspec/redc\t{}", sd.hex()),
        &format!("head\tspec/redc\t{}", kernel_node().digest().hex()),
        1,
    );
    expect(&doc, |e| matches!(e, Error::PinDigestNotDeclared(_)));
}

#[test]
fn r09_unknown_edge_kind() {
    // Unknown sigil.
    let doc = valid_doc().replacen("m:spec/", "x:spec/", 1);
    expect(&doc, |e| matches!(e, Error::UnknownEdgeKind(_)));

    // Missing separator.
    let doc = valid_doc().replacen("m:spec/", "m spec/", 1);
    expect(&doc, |e| matches!(e, Error::UnknownEdgeKind(_)));

    // Missing `@`.
    let doc = valid_doc().replacen("@", " ", 1);
    expect(&doc, |e| matches!(e, Error::UnknownEdgeKind(_)));
}

#[test]
fn r10_unknown_kind() {
    let doc = valid_doc().replacen("node\tspec\tspec/redc", "node\ttheorem\tspec/redc", 1);
    expect(&doc, |e| matches!(e, Error::UnknownKind(_)));
}

#[test]
fn r10_unknown_record_tag() {
    let doc = valid_doc().replacen("node\tspec\tspec/redc", "unit\tspec\tspec/redc", 1);
    expect(&doc, |e| matches!(e, Error::UnknownRecord(_)));
}

#[test]
fn r11_missing_required_field() {
    let doc = valid_doc().replacen(&hx("spec-sem"), "-", 1);
    expect(&doc, |e| {
        matches!(
            e,
            Error::MissingRequiredField {
                field: "semantic",
                ..
            }
        )
    });

    let doc = format!("{}\nnode\tartefact\tobj/x\t-\t-\t\n", wire::MAGIC);
    expect(&doc, |e| {
        matches!(e, Error::MissingRequiredField { field: "bytes", .. })
    });
}

#[test]
fn r12_non_canonical_name() {
    let doc = valid_doc().replacen("spec/redc", "spec:redc", 1);
    expect(&doc, |e| matches!(e, Error::NameNotCanonical(_)));
}

#[test]
fn r13_duplicate_pin() {
    let sd = spec_node().digest().hex();
    let doc = valid_doc().replacen(
        &format!(
            "m:spec/redc@{sd},a:hol/kernel@{}",
            kernel_node().digest().hex()
        ),
        &format!("m:spec/redc@{sd},m:spec/redc@{sd}"),
        1,
    );
    expect(&doc, |e| matches!(e, Error::DuplicatePin(_)));
}

#[test]
fn r14_missing_final_newline() {
    let mut doc = valid_doc();
    doc.pop();
    expect(&doc, |e| matches!(e, Error::TrailingBytes));
}

#[test]
fn r15_is_reachable_through_the_api() {
    let mut s = Store::new();
    let missing = UnitId::new("never/declared").unwrap();
    let node = Node::new(Kind::Spec, Some(Digest::of_label("m")), None, vec![]);
    match s.update(&missing, node) {
        Err(Error::UnknownUnitOnUpdate(u)) if u.as_str() == "never/declared" => {}
        Err(e) => panic!("expected R15, got {e}"),
        Ok(r) => panic!("expected R15, got {}", r.summary()),
    }
}

#[test]
fn r16_duplicate_head() {
    let sd = spec_node().digest().hex();
    let doc = format!("{}head\tspec/redc\t{sd}\n", valid_doc());
    expect(&doc, |e| matches!(e, Error::DuplicateUnit(_)));
}

#[test]
fn all_edge_sigils_are_accepted() {
    for (sigil, expected) in [
        ('m', EdgeKind::Semantic),
        ('s', EdgeKind::Subject),
        ('a', EdgeKind::Attestation),
    ] {
        let spec = Node::new(
            Kind::Spec,
            Some(Digest::of_label("spec-sem")),
            Some(Digest::of_label("spec-src")),
            vec![],
        );
        let sd = spec.digest();
        let thm = Node::new(
            Kind::Theorem,
            Some(Digest::of_label("thm-stmt")),
            Some(Digest::of_label("thm-cert")),
            vec![Pin::new(unit("spec/redc"), sd, expected)],
        );
        let td = thm.digest();

        let doc = format!(
            "{}\nnode\tspec\tspec/redc\t{}\t{}\t\nnode\tthm\tt\t{}\t{}\t{}:spec/redc@{}\nhead\tspec/redc\t{}\nhead\tt\t{}\n",
            wire::MAGIC,
            hx("spec-sem"),
            hx("spec-src"),
            hx("thm-stmt"),
            hx("thm-cert"),
            sigil,
            sd.hex(),
            sd.hex(),
            td.hex(),
        );
        let parsed = wire::parse(&doc).expect("sigil accepted");
        let d = parsed.unit_named(&unit("t")).unwrap();
        assert_eq!(parsed.node(&d).unwrap().pins[0].kind, expected);
    }
}

/// B2: the public API must refuse exactly what the parser refuses.
#[test]
fn api_refuses_what_parser_refuses() {
    let dup = Pin::new(unit("w"), Digest::of_label("w"), EdgeKind::Semantic);
    let cases = [
        Node::new(Kind::Theorem, None, None, vec![]),
        Node::new(Kind::Spec, None, Some(Digest::of_label("bytes")), vec![]),
        Node::new(Kind::Artefact, Some(Digest::of_label("sem")), None, vec![]),
        Node::new(
            Kind::Theorem,
            Some(Digest::of_label("st")),
            Some(Digest::of_label("ct")),
            vec![dup.clone(), dup],
        ),
    ];

    for node in cases {
        let u = unit("t");
        let mut store = Store::new();
        let api = store
            .insert(u.clone(), node.clone())
            .expect_err("API accepted an invalid node");

        let doc = format!("{}\n{}\n", wire::MAGIC, node_line(&u, &node));
        let wire_err = wire::parse(&doc).expect_err("parser accepted an invalid node");
        assert_eq!(api, wire_err, "API and parser disagree for {node:?}");
    }
}
