//! One test per refusal site.
//!
//! `tools/mutate.py` disables each site in turn; every disabling must be noticed here.
//! A site whose removal no test detects is a vacuous check and must be fixed or deleted.

use tekmerion::wire;
use tekmerion::{Digest, EdgeKind, Error, Kind, Node, Store, UnitId};

fn hx(s: &str) -> String {
    Digest::of_label(s).hex()
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

/// A minimal valid document. Pins point at the current digests of the units above them.
fn valid_doc() -> String {
    format!(
        "{magic}\n\
         spec\tspec/redc\t{sem}\t{src}\t\n\
         kernel\thol/kernel\t{kl}\t{kb}\t\n\
         thm\tredc/correct\t{ts}\t{tc}\tm:spec/redc@{sd},a:hol/kernel@{kd}\n",
        magic = wire::MAGIC,
        sem = hx("spec-sem"),
        src = hx("spec-src"),
        kl = hx("logic"),
        kb = hx("kbin"),
        ts = hx("thm-stmt"),
        tc = hx("thm-cert"),
        sd = spec_node().digest().hex(),
        kd = kernel_node().digest().hex(),
    )
}

fn expect(text: &str, pred: fn(&Error) -> bool) {
    match wire::parse(text) {
        Err(e) if pred(&e) => {}
        Err(e) => panic!("wrong refusal: {e}"),
        Ok(_) => panic!("document was accepted but should have been refused"),
    }
}

#[test]
fn accepts_valid_document() {
    let s = wire::parse(&valid_doc()).expect("valid document");
    assert_eq!(s.len(), 3);
}

#[test]
fn r01_bad_magic() {
    let doc = valid_doc().replacen(wire::MAGIC, "SOMETHING-ELSE v1", 1);
    expect(&doc, |e| matches!(e, Error::BadMagic));
}

#[test]
fn r02_unsupported_version() {
    let doc = valid_doc().replacen(wire::MAGIC, "TEKMERION-EVIDENCE v2", 1);
    expect(&doc, |e| matches!(e, Error::UnsupportedVersion(_)));
}

#[test]
fn r03_wrong_field_count() {
    // Drop the (empty) pin column from the spec line.
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
fn r04_empty_unit_name() {
    let doc = valid_doc().replacen("spec\tspec/redc", "spec\t", 1);
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
fn r07_unknown_unit() {
    let doc = valid_doc().replacen("m:spec/redc@", "m:spec/undeclared@", 1);
    expect(
        &doc,
        |e| matches!(e, Error::UnknownUnit(u) if u.as_str() == "spec/undeclared"),
    );
}

#[test]
fn r08_pin_digest_not_current() {
    let doc = valid_doc().replacen(
        &format!("m:spec/redc@{}", spec_node().digest().hex()),
        &format!("m:spec/redc@{}", hx("stale")),
        1,
    );
    expect(&doc, |e| matches!(e, Error::PinDigestNotCurrent(_)));
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
    let doc = valid_doc().replacen("spec\tspec/redc", "theorem\tspec/redc", 1);
    expect(&doc, |e| matches!(e, Error::UnknownKind(_)));
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

    let doc = format!("{}\nartefact\tobj/x\t-\t-\t\n", wire::MAGIC);
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
fn r16_duplicate_unit() {
    let doc = format!("{}spec\tspec/redc\t{}\t-\t\n", valid_doc(), hx("other"));
    expect(&doc, |e| matches!(e, Error::DuplicateUnit(_)));
}

#[test]
fn all_edge_sigils_are_accepted() {
    let sd = spec_node().digest().hex();
    let kd = kernel_node().digest().hex();

    for (sigil, expected) in [
        ('m', EdgeKind::Semantic),
        ('s', EdgeKind::Subject),
        ('a', EdgeKind::Attestation),
    ] {
        let doc = format!(
            "{}\nspec\tspec/redc\t{}\t{}\t\nkernel\thol/k\t{}\t{}\t\nthm\tt\t{}\t{}\t{}:spec/redc@{}\n",
            wire::MAGIC,
            hx("spec-sem"),
            hx("spec-src"),
            hx("logic"),
            hx("kbin"),
            hx("thm-stmt"),
            hx("thm-cert"),
            sigil,
            sd,
        );
        let parsed = wire::parse(&doc).expect("sigil accepted");
        let d = parsed.unit_named(&UnitId::new("t").unwrap()).unwrap();
        assert_eq!(parsed.node(&d).unwrap().pins[0].kind, expected);
        let _ = &kd;
    }
}
