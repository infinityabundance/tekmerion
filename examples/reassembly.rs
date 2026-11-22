//! The reassembly gap, made explicit.
//!
//! s2n-bignum's `SOUNDNESS.md` records a caveat under "reassembly in downstream projects":
//! when a `.S` file is imported and assembled elsewhere, nothing checks that the resulting
//! object bytes are the bytes the proofs were about. This example models that chain and
//! shows what typed revocation reports at each step.
//!
//! It demonstrates the *mechanism*. It is not evidence about any real proof.

use tekmerion::{Digest, EdgeKind, Kind, Node, Pin, Store, UnitId};

fn lbl(s: &str) -> Digest {
    Digest::of_label(s)
}

fn main() {
    let mut store = Store::new();

    let spec = UnitId::new("spec/montmul_p256").unwrap();
    let spec_d = store
        .insert(
            spec.clone(),
            Node::new(
                Kind::Spec,
                Some(lbl("montmul-semantics")),
                Some(lbl("spec-source-v1")),
                vec![],
            ),
        )
        .unwrap();

    let obj = UnitId::new("obj/bignum_montmul_p256.o").unwrap();
    let obj_d = store
        .insert(
            obj.clone(),
            Node::new(Kind::Artefact, None, Some(lbl("object-bytes-v1")), vec![]),
        )
        .unwrap();

    let kernel = UnitId::new("kernel/hol-light").unwrap();
    let kernel_d = store
        .insert(
            kernel.clone(),
            Node::new(
                Kind::Kernel,
                Some(lbl("hol-logic")),
                Some(lbl("hol-binary-v1")),
                vec![],
            ),
        )
        .unwrap();

    let thm = UnitId::new("thm/bignum_montmul_p256_subroutine_correct").unwrap();
    let thm_d = store
        .insert(
            thm.clone(),
            Node::new(
                Kind::Theorem,
                Some(lbl("montmul-statement")),
                Some(lbl("montmul-certificate")),
                vec![
                    Pin::new(spec.clone(), spec_d, EdgeKind::Semantic),
                    Pin::new(obj.clone(), obj_d, EdgeKind::Subject),
                    Pin::new(kernel.clone(), kernel_d, EdgeKind::Attestation),
                ],
            ),
        )
        .unwrap();

    println!("theorem id: {}\n", &thm_d.hex()[..16]);

    println!("1. reformat the specification (meaning unchanged)");
    let r = store
        .update(
            &spec,
            Node::new(
                Kind::Spec,
                Some(lbl("montmul-semantics")),
                Some(lbl("spec-source-v2")),
                vec![],
            ),
        )
        .unwrap();
    println!("   {}\n", r.summary());

    println!("2. reassemble the .S downstream, bytes differ (the documented gap)");
    let r = store
        .update(
            &obj,
            Node::new(Kind::Artefact, None, Some(lbl("object-bytes-v2")), vec![]),
        )
        .unwrap();
    println!("   {}", r.summary());
    println!("   theorem status: {:?}", store.status(&thm_d));
    println!("   -> still true, but no longer covers the shipped bytes\n");

    println!("3. rebuild the checker binary (logic unchanged)");
    let r = store
        .update(
            &kernel,
            Node::new(
                Kind::Kernel,
                Some(lbl("hol-logic")),
                Some(lbl("hol-binary-v2")),
                vec![],
            ),
        )
        .unwrap();
    println!("   {}", r.summary());
    println!("   theorem status: {:?}", store.status(&thm_d));
    println!("   -> the rebuild's event is only attestation-stale; the earlier subject");
    println!("      mismatch is still effective, so the replay alone is not enough\n");

    println!("4. change the meaning of the specification");
    let r = store
        .update(
            &spec,
            Node::new(
                Kind::Spec,
                Some(lbl("montmul-semantics-v2")),
                Some(lbl("spec-source-v3")),
                vec![],
            ),
        )
        .unwrap();
    println!("   {}", r.summary());
    println!("   theorem status: {:?}", store.status(&thm_d));
    println!("   -> the proof must be redone");
}
