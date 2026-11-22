//! Behavioural tests for typed revocation.

use std::collections::BTreeSet;

use tekmerion::wire;
use tekmerion::{Digest, EdgeKind, Kind, Node, Pin, Revocation, Severity, Store, UnitId};

fn lbl(s: &str) -> Digest {
    Digest::of_label(s)
}

fn unit(s: &str) -> UnitId {
    UnitId::new(s).unwrap()
}

fn pin(store: &Store, u: &UnitId, kind: EdgeKind) -> Pin {
    Pin::new(u.clone(), store.unit_named(u).unwrap(), kind)
}

fn theorem(stmt: &str, cert: &str, pins: Vec<Pin>) -> Node {
    Node::new(Kind::Theorem, Some(lbl(stmt)), Some(lbl(cert)), pins)
}

// ---------------------------------------------------------------------------
// The three edge kinds
// ---------------------------------------------------------------------------

#[test]
fn subject_change_revokes_applicability() {
    let mut s = Store::new();
    let obj = unit("obj/mul_p25519");
    s.insert(
        obj.clone(),
        Node::new(Kind::Artefact, None, Some(lbl("obj-bytes-v1")), vec![]),
    )
    .unwrap();

    let t = unit("thm/mul_correct");
    let t_digest = s
        .insert(
            t.clone(),
            theorem(
                "mul-stmt",
                "mul-cert",
                vec![pin(&s, &obj, EdgeKind::Subject)],
            ),
        )
        .unwrap();

    let rev = s
        .update(
            &obj,
            Node::new(Kind::Artefact, None, Some(lbl("obj-bytes-v2")), vec![]),
        )
        .unwrap();

    assert_eq!(rev.affected.get(&t_digest), Some(&Severity::SubjectChanged));
    assert_eq!(rev.count(Severity::SemanticsChanged), 0);
}

#[test]
fn attestation_change_is_stale_not_broken() {
    let mut s = Store::new();
    let k = unit("kernel/hol-light");
    s.insert(
        k.clone(),
        Node::new(
            Kind::Kernel,
            Some(lbl("logic-hol")),
            Some(lbl("kernel-bin-v1")),
            vec![],
        ),
    )
    .unwrap();

    let t = unit("thm/mul_correct");
    let t_digest = s
        .insert(
            t.clone(),
            theorem(
                "mul-stmt",
                "mul-cert",
                vec![pin(&s, &k, EdgeKind::Attestation)],
            ),
        )
        .unwrap();

    // Rebuild the checker binary; the logic is unchanged.
    let rev = s
        .update(
            &k,
            Node::new(
                Kind::Kernel,
                Some(lbl("logic-hol")),
                Some(lbl("kernel-bin-v2")),
                vec![],
            ),
        )
        .unwrap();

    assert_eq!(
        rev.affected.get(&t_digest),
        Some(&Severity::AttestationStale)
    );
    assert_eq!(rev.count(Severity::SemanticsChanged), 0);
    assert_eq!(rev.count(Severity::SubjectChanged), 0);
}

#[test]
fn formatting_change_revokes_nothing() {
    let mut s = Store::new();
    let sp = unit("spec/redc");
    s.insert(
        sp.clone(),
        Node::new(
            Kind::Spec,
            Some(lbl("redc-meaning")),
            Some(lbl("redc-src-v1")),
            vec![],
        ),
    )
    .unwrap();

    let t = unit("thm/redc_correct");
    s.insert(
        t.clone(),
        theorem(
            "redc-stmt",
            "redc-cert",
            vec![pin(&s, &sp, EdgeKind::Semantic)],
        ),
    )
    .unwrap();

    // Same meaning, different source bytes (a reformat).
    let rev = s
        .update(
            &sp,
            Node::new(
                Kind::Spec,
                Some(lbl("redc-meaning")),
                Some(lbl("redc-src-v2")),
                vec![],
            ),
        )
        .unwrap();

    assert!(
        rev.is_empty(),
        "reformatting must not revoke: {}",
        rev.summary()
    );
}

#[test]
fn semantic_change_propagates_through_lemma() {
    let mut s = Store::new();
    let sp = unit("spec/redc");
    s.insert(
        sp.clone(),
        Node::new(
            Kind::Spec,
            Some(lbl("redc-meaning-v1")),
            Some(lbl("redc-src-v1")),
            vec![],
        ),
    )
    .unwrap();

    let lemma = unit("lemma/carry_bound");
    s.insert(
        lemma.clone(),
        theorem(
            "carry-stmt",
            "carry-cert",
            vec![pin(&s, &sp, EdgeKind::Semantic)],
        ),
    )
    .unwrap();

    let top = unit("thm/redc_correct");
    let top_digest = s
        .insert(
            top.clone(),
            theorem(
                "redc-stmt",
                "redc-cert",
                vec![pin(&s, &lemma, EdgeKind::Semantic)],
            ),
        )
        .unwrap();

    let rev = s
        .update(
            &sp,
            Node::new(
                Kind::Spec,
                Some(lbl("redc-meaning-v2")),
                Some(lbl("redc-src-v2")),
                vec![],
            ),
        )
        .unwrap();

    assert_eq!(
        rev.affected.get(&top_digest),
        Some(&Severity::SemanticsChanged)
    );
    assert_eq!(rev.count(Severity::SemanticsChanged), 2); // lemma + theorem
}

#[test]
fn unrelated_evidence_survives() {
    let mut s = Store::new();
    let a = unit("spec/a");
    let b = unit("spec/b");
    s.insert(
        a.clone(),
        Node::new(Kind::Spec, Some(lbl("a-meaning")), None, vec![]),
    )
    .unwrap();
    s.insert(
        b.clone(),
        Node::new(Kind::Spec, Some(lbl("b-meaning")), None, vec![]),
    )
    .unwrap();

    let ta = unit("thm/a");
    s.insert(
        ta.clone(),
        theorem("a-stmt", "a-cert", vec![pin(&s, &a, EdgeKind::Semantic)]),
    )
    .unwrap();
    let tb = unit("thm/b");
    let tb_digest = s
        .insert(
            tb.clone(),
            theorem("b-stmt", "b-cert", vec![pin(&s, &b, EdgeKind::Semantic)]),
        )
        .unwrap();

    let rev = s
        .update(
            &b,
            Node::new(Kind::Spec, Some(lbl("b-meaning-v2")), None, vec![]),
        )
        .unwrap();

    assert!(rev.affected.contains_key(&tb_digest));
    let ta_digest = s.unit_named(&ta).unwrap();
    assert!(!rev.affected.contains_key(&ta_digest));
}

#[test]
fn worst_severity_wins() {
    let mut s = Store::new();
    let ar = unit("obj/x");
    let ke = unit("kernel/hol");
    s.insert(
        ar.clone(),
        Node::new(Kind::Artefact, None, Some(lbl("x-bytes-1")), vec![]),
    )
    .unwrap();
    s.insert(
        ke.clone(),
        Node::new(Kind::Kernel, Some(lbl("logic")), Some(lbl("bin-1")), vec![]),
    )
    .unwrap();

    let t = unit("thm/t");
    let t_digest = s
        .insert(
            t.clone(),
            theorem(
                "t-stmt",
                "t-cert",
                vec![
                    pin(&s, &ar, EdgeKind::Subject),
                    pin(&s, &ke, EdgeKind::Attestation),
                ],
            ),
        )
        .unwrap();

    // Both the artefact and the checker binary change.
    s.update(
        &ar,
        Node::new(Kind::Artefact, None, Some(lbl("x-bytes-2")), vec![]),
    )
    .unwrap();
    let rev = s
        .update(
            &ke,
            Node::new(Kind::Kernel, Some(lbl("logic")), Some(lbl("bin-2")), vec![]),
        )
        .unwrap();

    // Direct hit on the kernel is stale; the artefact hit already made it subject-stale.
    assert_eq!(
        rev.affected.get(&t_digest),
        Some(&Severity::AttestationStale)
    );

    // And the semantic promotion path: a logic change dominates.
    let rev = s
        .update(
            &ke,
            Node::new(
                Kind::Kernel,
                Some(lbl("logic-v2")),
                Some(lbl("bin-3")),
                vec![],
            ),
        )
        .unwrap();
    assert_eq!(
        rev.affected.get(&t_digest),
        Some(&Severity::SemanticsChanged)
    );
}

#[test]
fn identical_update_is_empty() {
    let mut s = Store::new();
    let sp = unit("spec/s");
    let node = Node::new(Kind::Spec, Some(lbl("m")), Some(lbl("b")), vec![]);
    s.insert(sp.clone(), node.clone()).unwrap();
    let t = unit("thm/t");
    s.insert(
        t.clone(),
        theorem("st", "ct", vec![pin(&s, &sp, EdgeKind::Semantic)]),
    )
    .unwrap();
    let rev = s.update(&sp, node).unwrap();
    assert!(rev.is_empty());
}

// ---------------------------------------------------------------------------
// Identity
// ---------------------------------------------------------------------------

#[test]
fn identity_is_derived_from_content() {
    let a = Node::new(Kind::Spec, Some(lbl("m")), Some(lbl("b")), vec![]);
    let b = Node::new(Kind::Spec, Some(lbl("m")), Some(lbl("b")), vec![]);
    assert_eq!(a.digest(), b.digest());

    let c = Node::new(Kind::Spec, Some(lbl("m2")), Some(lbl("b")), vec![]);
    assert_ne!(a.digest(), c.digest());
}

#[test]
fn edge_kind_is_part_of_identity() {
    let mut s = Store::new();
    let sp = unit("spec/s");
    let d = s
        .insert(
            sp.clone(),
            Node::new(Kind::Spec, Some(lbl("m")), None, vec![]),
        )
        .unwrap();

    let semantic = theorem(
        "st",
        "ct",
        vec![Pin::new(sp.clone(), d, EdgeKind::Semantic)],
    );
    let subject = theorem("st", "ct", vec![Pin::new(sp, d, EdgeKind::Subject)]);
    assert_ne!(semantic.digest(), subject.digest());
}

#[test]
fn wire_round_trip_preserves_identity() {
    let mut s = Store::new();
    // Deliberately insert in an order that is not topological by unit name.
    let sp = unit("zzz/spec");
    s.insert(
        sp.clone(),
        Node::new(Kind::Spec, Some(lbl("m")), Some(lbl("b")), vec![]),
    )
    .unwrap();
    let ar = unit("aaa/obj");
    s.insert(
        ar.clone(),
        Node::new(Kind::Artefact, None, Some(lbl("bytes")), vec![]),
    )
    .unwrap();
    let t = unit("mmm/thm");
    s.insert(
        t.clone(),
        theorem(
            "st",
            "ct",
            vec![
                pin(&s, &sp, EdgeKind::Semantic),
                pin(&s, &ar, EdgeKind::Subject),
            ],
        ),
    )
    .unwrap();

    let text = wire::render(&s);
    let parsed = wire::parse(&text).expect("rendered document must parse");

    for u in [&sp, &ar, &t] {
        assert_eq!(
            s.unit_named(u),
            parsed.unit_named(u),
            "unit {u} identity changed"
        );
    }
    assert_eq!(s.len(), parsed.len());
    assert_eq!(wire::render(&parsed), text, "render is not a fixed point");
}

// ---------------------------------------------------------------------------
// B1: the v2 node table round-trips after every class of update
// ---------------------------------------------------------------------------

fn assert_round_trip(s: &Store) {
    let text = wire::render(s);
    let parsed = match wire::parse(&text) {
        Ok(p) => p,
        Err(e) => panic!("parse(render(store)) failed: {e}\n--- document ---\n{text}"),
    };

    let mut heads: Vec<(UnitId, Digest)> = s.units().map(|(u, d)| (u.clone(), *d)).collect();
    heads.sort();
    let mut parsed_heads: Vec<(UnitId, Digest)> =
        parsed.units().map(|(u, d)| (u.clone(), *d)).collect();
    parsed_heads.sort();
    assert_eq!(heads, parsed_heads, "unit -> head map changed");

    let nodes: BTreeSet<Digest> = s.nodes().map(|(d, _)| *d).collect();
    let parsed_nodes: BTreeSet<Digest> = parsed.nodes().map(|(d, _)| *d).collect();
    assert_eq!(nodes, parsed_nodes, "node set changed");

    for d in &nodes {
        assert_eq!(
            s.status(d),
            parsed.status(d),
            "effective status changed for {}",
            d.hex()
        );
    }

    assert_eq!(wire::render(&parsed), text, "render is not a fixed point");
}

#[test]
fn round_trip_after_semantic_update() {
    let mut s = Store::new();
    let sp = unit("spec/s");
    s.insert(
        sp.clone(),
        Node::new(
            Kind::Spec,
            Some(lbl("s-meaning-v1")),
            Some(lbl("s-src-v1")),
            vec![],
        ),
    )
    .unwrap();
    let t = unit("thm/t");
    s.insert(
        t,
        theorem("t-stmt", "t-cert", vec![pin(&s, &sp, EdgeKind::Semantic)]),
    )
    .unwrap();

    s.update(
        &sp,
        Node::new(
            Kind::Spec,
            Some(lbl("s-meaning-v2")),
            Some(lbl("s-src-v2")),
            vec![],
        ),
    )
    .unwrap();
    assert_round_trip(&s);
}

#[test]
fn round_trip_after_subject_update() {
    let mut s = Store::new();
    let ar = unit("obj/a");
    s.insert(
        ar.clone(),
        Node::new(Kind::Artefact, None, Some(lbl("a-bytes-v1")), vec![]),
    )
    .unwrap();
    let t = unit("thm/t");
    s.insert(
        t,
        theorem("t-stmt", "t-cert", vec![pin(&s, &ar, EdgeKind::Subject)]),
    )
    .unwrap();

    s.update(
        &ar,
        Node::new(Kind::Artefact, None, Some(lbl("a-bytes-v2")), vec![]),
    )
    .unwrap();
    assert_round_trip(&s);
}

#[test]
fn round_trip_after_attestation_update() {
    let mut s = Store::new();
    let ke = unit("kernel/k");
    s.insert(
        ke.clone(),
        Node::new(
            Kind::Kernel,
            Some(lbl("logic")),
            Some(lbl("kbin-v1")),
            vec![],
        ),
    )
    .unwrap();
    let t = unit("thm/t");
    s.insert(
        t,
        theorem(
            "t-stmt",
            "t-cert",
            vec![pin(&s, &ke, EdgeKind::Attestation)],
        ),
    )
    .unwrap();

    s.update(
        &ke,
        Node::new(
            Kind::Kernel,
            Some(lbl("logic")),
            Some(lbl("kbin-v2")),
            vec![],
        ),
    )
    .unwrap();
    assert_round_trip(&s);
}

#[test]
fn round_trip_after_repin_with_unchanged_semantic_and_bytes() {
    let mut s = Store::new();
    let sp = unit("spec/s");
    s.insert(
        sp.clone(),
        Node::new(Kind::Spec, Some(lbl("s-sem")), Some(lbl("s-src")), vec![]),
    )
    .unwrap();

    let u = unit("lemma/u");
    s.insert(
        u.clone(),
        Node::new(
            Kind::Theorem,
            Some(lbl("u-stmt")),
            Some(lbl("u-cert")),
            vec![],
        ),
    )
    .unwrap();

    let t = unit("thm/t");
    s.insert(
        t,
        theorem("t-stmt", "t-cert", vec![pin(&s, &u, EdgeKind::Semantic)]),
    )
    .unwrap();

    // Same meaning and bytes; only the derivation (pins) changes.
    s.update(
        &u,
        Node::new(
            Kind::Theorem,
            Some(lbl("u-stmt")),
            Some(lbl("u-cert")),
            vec![pin(&s, &sp, EdgeKind::Semantic)],
        ),
    )
    .unwrap();

    assert_round_trip(&s);
}

// ---------------------------------------------------------------------------
// B3: effective status, not the event delta
// ---------------------------------------------------------------------------

#[test]
fn effective_status_survives_artefact_change_then_checker_rebuild() {
    let mut s = Store::new();
    let ar = unit("obj/a");
    let ke = unit("kernel/k");
    s.insert(
        ar.clone(),
        Node::new(Kind::Artefact, None, Some(lbl("a-bytes-v1")), vec![]),
    )
    .unwrap();
    s.insert(
        ke.clone(),
        Node::new(
            Kind::Kernel,
            Some(lbl("logic")),
            Some(lbl("kbin-v1")),
            vec![],
        ),
    )
    .unwrap();
    let t = unit("thm/t");
    let t_digest = s
        .insert(
            t,
            theorem(
                "t-stmt",
                "t-cert",
                vec![
                    pin(&s, &ar, EdgeKind::Subject),
                    pin(&s, &ke, EdgeKind::Attestation),
                ],
            ),
        )
        .unwrap();

    s.update(
        &ar,
        Node::new(Kind::Artefact, None, Some(lbl("a-bytes-v2")), vec![]),
    )
    .unwrap();
    assert_eq!(s.status(&t_digest), Some(Severity::SubjectChanged));

    // The rebuild's own event is only an attestation delta...
    let rev = s
        .update(
            &ke,
            Node::new(
                Kind::Kernel,
                Some(lbl("logic")),
                Some(lbl("kbin-v2")),
                vec![],
            ),
        )
        .unwrap();
    assert_eq!(
        rev.affected.get(&t_digest),
        Some(&Severity::AttestationStale)
    );

    // ...but the effective status still holds the unresolved subject mismatch.
    assert_eq!(s.status(&t_digest), Some(Severity::SubjectChanged));
    assert!(!s.is_admissible(&t_digest));
}

#[test]
fn effective_status_logic_change_is_semantic() {
    let mut s = Store::new();
    let ke = unit("kernel/k");
    s.insert(
        ke.clone(),
        Node::new(
            Kind::Kernel,
            Some(lbl("logic-v1")),
            Some(lbl("kbin")),
            vec![],
        ),
    )
    .unwrap();
    let t = unit("thm/t");
    let t_digest = s
        .insert(
            t,
            theorem(
                "t-stmt",
                "t-cert",
                vec![pin(&s, &ke, EdgeKind::Attestation)],
            ),
        )
        .unwrap();

    s.update(
        &ke,
        Node::new(
            Kind::Kernel,
            Some(lbl("logic-v2")),
            Some(lbl("kbin")),
            vec![],
        ),
    )
    .unwrap();
    assert_eq!(s.status(&t_digest), Some(Severity::SemanticsChanged));
}

#[test]
fn effective_status_reformat_is_none() {
    let mut s = Store::new();
    let sp = unit("spec/s");
    s.insert(
        sp.clone(),
        Node::new(
            Kind::Spec,
            Some(lbl("meaning")),
            Some(lbl("src-v1")),
            vec![],
        ),
    )
    .unwrap();
    let t = unit("thm/t");
    let t_digest = s
        .insert(
            t,
            theorem("t-stmt", "t-cert", vec![pin(&s, &sp, EdgeKind::Semantic)]),
        )
        .unwrap();

    s.update(
        &sp,
        Node::new(
            Kind::Spec,
            Some(lbl("meaning")),
            Some(lbl("src-v2")),
            vec![],
        ),
    )
    .unwrap();
    assert_eq!(s.status(&t_digest), None);
    assert!(s.is_admissible(&t_digest));
}

#[test]
fn effective_status_checker_only_rebuild_is_stale() {
    let mut s = Store::new();
    let ke = unit("kernel/k");
    s.insert(
        ke.clone(),
        Node::new(
            Kind::Kernel,
            Some(lbl("logic")),
            Some(lbl("kbin-v1")),
            vec![],
        ),
    )
    .unwrap();
    let t = unit("thm/t");
    let t_digest = s
        .insert(
            t,
            theorem(
                "t-stmt",
                "t-cert",
                vec![pin(&s, &ke, EdgeKind::Attestation)],
            ),
        )
        .unwrap();

    s.update(
        &ke,
        Node::new(
            Kind::Kernel,
            Some(lbl("logic")),
            Some(lbl("kbin-v2")),
            vec![],
        ),
    )
    .unwrap();
    assert_eq!(s.status(&t_digest), Some(Severity::AttestationStale));
}

#[test]
fn effective_status_repin_is_semantic() {
    let mut s = Store::new();
    let sp = unit("spec/s");
    s.insert(
        sp.clone(),
        Node::new(Kind::Spec, Some(lbl("s-sem")), Some(lbl("s-src")), vec![]),
    )
    .unwrap();

    let u = unit("lemma/u");
    s.insert(
        u.clone(),
        Node::new(
            Kind::Theorem,
            Some(lbl("u-stmt")),
            Some(lbl("u-cert")),
            vec![],
        ),
    )
    .unwrap();
    let t = unit("thm/t");
    let t_digest = s
        .insert(
            t,
            theorem("t-stmt", "t-cert", vec![pin(&s, &u, EdgeKind::Semantic)]),
        )
        .unwrap();

    let rev = s
        .update(
            &u,
            Node::new(
                Kind::Theorem,
                Some(lbl("u-stmt")),
                Some(lbl("u-cert")),
                vec![pin(&s, &sp, EdgeKind::Semantic)],
            ),
        )
        .unwrap();

    assert_eq!(
        rev.affected.get(&t_digest),
        Some(&Severity::SemanticsChanged)
    );
    assert_eq!(s.status(&t_digest), Some(Severity::SemanticsChanged));
}

#[test]
fn effective_status_closes_over_unchanged_intermediates() {
    let mut s = Store::new();
    let sp = unit("spec/s");
    s.insert(
        sp.clone(),
        Node::new(
            Kind::Spec,
            Some(lbl("meaning-v1")),
            Some(lbl("src-v1")),
            vec![],
        ),
    )
    .unwrap();

    let lemma = unit("lemma/l");
    s.insert(
        lemma.clone(),
        theorem("l-stmt", "l-cert", vec![pin(&s, &sp, EdgeKind::Semantic)]),
    )
    .unwrap();

    let top = unit("thm/t");
    let top_digest = s
        .insert(
            top,
            theorem(
                "t-stmt",
                "t-cert",
                vec![pin(&s, &lemma, EdgeKind::Semantic)],
            ),
        )
        .unwrap();

    // Change the spec but leave the lemma at its old node. The theorem's direct pin is
    // unchanged; only the transitive closure can see the revocation.
    s.update(
        &sp,
        Node::new(
            Kind::Spec,
            Some(lbl("meaning-v2")),
            Some(lbl("src-v2")),
            vec![],
        ),
    )
    .unwrap();

    assert_eq!(s.status(&top_digest), Some(Severity::SemanticsChanged));
}

// ---------------------------------------------------------------------------
// Model conformance on a synthetic corpus
//
// This is a regression test for the *model*, on synthetic data. It is not empirical
// evidence about any real proof corpus, and it establishes nothing about soundness.
// ---------------------------------------------------------------------------

fn naive_closure(store: &Store, changed: &Digest) -> BTreeSet<Digest> {
    let mut seen = BTreeSet::new();
    let mut work = vec![*changed];
    while let Some(d) = work.pop() {
        for (dependent, _kind) in store.dependents_of(&d) {
            if seen.insert(dependent) {
                work.push(dependent);
            }
        }
    }
    seen
}

#[test]
fn naive_vs_typed_on_synthetic_corpus() {
    const N: usize = 30;
    let mut s = Store::new();

    let mut specs = Vec::new();
    let mut objs = Vec::new();
    let mut kernels = Vec::new();

    for i in 0..N {
        let u = unit(&format!("spec/{i}"));
        s.insert(
            u.clone(),
            Node::new(
                Kind::Spec,
                Some(lbl(&format!("spec-sem-{i}"))),
                Some(lbl(&format!("spec-src-{i}-v1"))),
                vec![],
            ),
        )
        .unwrap();
        specs.push(u);

        let u = unit(&format!("obj/{i}"));
        s.insert(
            u.clone(),
            Node::new(
                Kind::Artefact,
                None,
                Some(lbl(&format!("obj-bytes-{i}-v1"))),
                vec![],
            ),
        )
        .unwrap();
        objs.push(u);

        let u = unit(&format!("kernel/{i}"));
        s.insert(
            u.clone(),
            Node::new(
                Kind::Kernel,
                Some(lbl(&format!("logic-{i}"))),
                Some(lbl(&format!("kbin-{i}-v1"))),
                vec![],
            ),
        )
        .unwrap();
        kernels.push(u);
    }

    for i in 0..(N * 3) {
        let pins = vec![
            pin(&s, &specs[i % N], EdgeKind::Semantic),
            pin(&s, &objs[i % N], EdgeKind::Subject),
            pin(&s, &kernels[i % N], EdgeKind::Attestation),
        ];
        s.insert(
            unit(&format!("lemma/{i}")),
            theorem(&format!("lemma-stmt-{i}"), &format!("lemma-cert-{i}"), pins),
        )
        .unwrap();
    }

    for i in 0..(N * 3) {
        let lu = unit(&format!("lemma/{i}"));
        s.insert(
            unit(&format!("thm/{i}")),
            theorem(
                &format!("thm-stmt-{i}"),
                &format!("thm-cert-{i}"),
                vec![pin(&s, &lu, EdgeKind::Semantic)],
            ),
        )
        .unwrap();
    }

    // 10 reformats, 10 meaning changes, 10 checker rebuilds, 10 artefact rebuilds.
    let mut changes: Vec<(UnitId, Node)> = Vec::new();
    for i in 0..10 {
        changes.push((
            unit(&format!("spec/{i}")),
            Node::new(
                Kind::Spec,
                Some(lbl(&format!("spec-sem-{i}"))),
                Some(lbl(&format!("spec-src-{i}-v2"))),
                vec![],
            ),
        ));
    }
    for i in 10..20 {
        changes.push((
            unit(&format!("spec/{i}")),
            Node::new(
                Kind::Spec,
                Some(lbl(&format!("spec-sem-{i}-v2"))),
                Some(lbl(&format!("spec-src-{i}-v2"))),
                vec![],
            ),
        ));
    }
    for i in 0..10 {
        changes.push((
            unit(&format!("kernel/{i}")),
            Node::new(
                Kind::Kernel,
                Some(lbl(&format!("logic-{i}"))),
                Some(lbl(&format!("kbin-{i}-v2"))),
                vec![],
            ),
        ));
    }
    for i in 0..10 {
        changes.push((
            unit(&format!("obj/{i}")),
            Node::new(
                Kind::Artefact,
                None,
                Some(lbl(&format!("obj-bytes-{i}-v2"))),
                vec![],
            ),
        ));
    }

    let mut naive_total = 0usize;
    let mut typed_total = 0usize;
    let mut typed_semantics = 0usize;
    let mut typed_subject = 0usize;
    let mut typed_stale = 0usize;

    for (idx, (u, new)) in changes.iter().enumerate() {
        let old = s.unit_named(u).unwrap();
        let naive = naive_closure(&s, &old);
        naive_total += naive.len();

        let rev: Revocation = s.update(u, new.clone()).unwrap();
        typed_total += rev.affected.len();
        typed_semantics += rev.count(Severity::SemanticsChanged);
        typed_subject += rev.count(Severity::SubjectChanged);
        typed_stale += rev.count(Severity::AttestationStale);

        if idx < 10 {
            // Reformat: nothing at all may move.
            assert!(rev.is_empty(), "reformat {u} revoked {}", rev.summary());
        }
        if (20..30).contains(&idx) {
            // Checker rebuild: stale at most, never semantic.
            assert_eq!(
                rev.count(Severity::SemanticsChanged),
                0,
                "kernel rebuild caused reproof"
            );
            assert!(
                rev.count(Severity::AttestationStale) > 0,
                "kernel rebuild not recorded"
            );
        }
        if (30..40).contains(&idx) {
            // Artefact rebuild: applicability, never meaning.
            assert_eq!(
                rev.count(Severity::SemanticsChanged),
                0,
                "artefact rebuild caused reproof"
            );
            assert!(
                rev.count(Severity::SubjectChanged) > 0,
                "artefact rebuild not recorded"
            );
        }
    }

    println!(
        "synthetic corpus: objects={} changes={}",
        s.len(),
        changes.len()
    );
    println!("naive affected (all needing reproof): {naive_total}");
    println!(
        "typed affected: {typed_total} (semantics={typed_semantics}, subject={typed_subject}, attestation={typed_stale})"
    );
    println!(
        "reproof work removed vs naive: {:.1}%",
        100.0 * (1.0 - typed_semantics as f64 / naive_total as f64)
    );

    // Soundness of the model: every meaning change is still reported as a meaning change.
    assert_eq!(
        typed_semantics, 60,
        "10 spec changes x (3 lemmas + 3 theorems)"
    );
    // And the naive engine would have demanded 10x more reproof work.
    assert!(typed_semantics < naive_total);
}
