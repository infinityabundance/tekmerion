//! Integration tests for step derivation, disassembly and the x86 emitter.
//!
//! These replace `tools/test_derive_steps.py` and `tools/test_x86_support.py`.
//! The fast tests need no HOL Light and no benchmark tree; the real-object tests
//! skip themselves when `.bench/s2n-bignum` is absent.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use tekmerion::tools::derive::{self, Form, InstructionSemantics, Step};
use tekmerion::tools::{disasm, emit, guard};

fn steps(rows: &[(&str, &str)]) -> Vec<Step> {
    rows.iter()
        .enumerate()
        .map(|(i, (mnemonic, operands))| Step {
            index: i + 1,
            mnemonic: (*mnemonic).to_string(),
            operands: operands.split_whitespace().map(|o| o.to_string()).collect(),
            raw: String::new(),
            address: 0,
        })
        .collect()
}

fn x86(rows: &[(&str, &str)]) -> Vec<usize> {
    derive::derive_accumulate_with(&steps(rows), &derive::x86_semantics())
}

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn s2n() -> PathBuf {
    repo().join(".bench/s2n-bignum")
}

// ---------------------------------------------------------------------------
// Unit tests on hand-built machine-code fixtures
// ---------------------------------------------------------------------------

#[test]
fn hand_built_carry_chain() {
    // Known answer: a three-deep carry chain plus a flag consumer.
    let exec_steps = steps(&[
        ("ldp", "x4 x5 [x1]"),
        ("ldp", "x6 x7 [x2]"),
        ("adds", "x4 x4 x6"), // 3: carry-out consumed by 4
        ("adcs", "x5 x5 x7"), // 4: carry-in from 3, carry-out to 5
        ("sbcs", "x8 x8 x9"), // 5: carry-in from 4, carry-out to 6
        ("csetm", "x10 cc"),  // 6: consumes the carry flag
        ("stp", "x4 x5 [x0]"),
        ("ret", ""),
    ]);
    assert_eq!(derive::derive_accumulate(&exec_steps), vec![3, 4, 5]);
}

#[test]
fn carry_materialisation_is_excluded() {
    // `ADC Xd, XZR, XZR` only materialises the flag; it is not a digit sum.
    let exec_steps = steps(&[
        ("adds", "x4 x4 x6"),    // carry-out consumed by 2
        ("adc", "x3 xzr xzr"),   // zero sources -> excluded
        ("csel", "x4 x4 x6 cc"), // also consumes the flag
    ]);
    assert_eq!(derive::derive_accumulate(&exec_steps), vec![1]);
}

#[test]
fn flag_writer_without_consumer_is_excluded() {
    let exec_steps = steps(&[
        ("adds", "x4 x4 x6"), // flags are never read
        ("ldp", "x7 x8 [x1]"),
        ("stp", "x4 x4 [x0]"),
    ]);
    assert!(derive::derive_accumulate(&exec_steps).is_empty());
}

#[test]
fn compare_writing_xzr_is_excluded() {
    // A compare whose flag only masks a value is not a carry-chain link.
    let exec_steps = steps(&[
        ("cmp", "x2 #0x0"),
        ("csetm", "x2 ne"),
        ("subs", "x4 x2 x4"), // carry-in from 1 (not a carry read), out to 4
        ("csel", "x4 x4 x4 cc"),
    ]);
    assert_eq!(derive::derive_accumulate(&exec_steps), vec![3]);
}

#[test]
fn flag_only_adcs_and_sbcs_xzr_are_chain_links() {
    // Regression: `CMN`/`ADCS XZR` select with a flag but write no value.
    let exec_steps = steps(&[
        ("cmp", "x5 x4"),       // 1: flag-only, no destination
        ("sbcs", "xzr x9 xzr"), // 2: flag-only, XZR destination
        ("adcs", "xzr x3 xzr"), // 3: carry in from 2, out to 5
        ("csel", "x4 x4 xzr ne"),
        ("adcs", "x5 x5 x4"), // 5: the register write the chain reaches
        ("csel", "x5 x5 x4 eq"),
    ]);
    assert_eq!(derive::derive_accumulate(&exec_steps), vec![1, 2, 3, 5]);
}

#[test]
fn flag_only_step_is_excluded_when_it_feeds_no_register_write() {
    // A flag-only link whose chain ends in a predicate takes no relation.
    let exec_steps = steps(&[
        ("cmp", "x5 #0x0"), // 1: chain ends at a mask, not a write
        ("csetm", "x4 ne"),
        ("eor", "x5 x5 x4"),
        ("stp", "x5 x6 [x0]"),
    ]);
    assert!(derive::derive_accumulate(&exec_steps).is_empty());
}

#[test]
fn flag_only_carry_materialisation_is_excluded() {
    // `ADCS XZR, XZR, XZR` widens the flag; it constrains no word.
    let exec_steps = steps(&[
        ("adcs", "xzr xzr xzr"), // no real register source -> excluded
        ("adcs", "x3 x3 xzr"),
        ("csetm", "x4 ne"),
    ]);
    assert_eq!(derive::derive_accumulate(&exec_steps), vec![2]);
}

#[test]
fn no_flag_consumers_yields_empty() {
    let exec_steps = steps(&[
        ("adds", "x4 x4 x6"),
        ("ldp", "x7 x8 [x1]"),
        ("stp", "x4 x5 [x0]"),
        ("ret", ""),
    ]);
    assert!(derive::derive_accumulate(&exec_steps).is_empty());
}

#[test]
fn ungrounded_carry_in_is_excluded() {
    // ADC/SBC with no preceding flag writer has no carry dataflow to use.
    let exec_steps = steps(&[
        ("adc", "x4 x4 x6"),
        ("sbc", "x5 x5 x7"),
        ("csel", "x4 x4 x6 cc"),
    ]);
    assert!(derive::derive_accumulate(&exec_steps).is_empty());
}

#[test]
fn reader_is_transparent_adds_adc_adcs() {
    // D1 regression: an intervening ADC reads but does not kill C.
    let exec_steps = steps(&[
        ("adds", "x4 x4 x6"), // 1: defines C
        ("adc", "x5 x5 x7"),  // 2: reads C, leaves it unchanged
        ("adcs", "x8 x8 x9"), // 3: reads the same C, writes a new one
        ("csetm", "x10 cc"),  // 4: consumes the final carry
    ]);
    assert_eq!(derive::derive_accumulate(&exec_steps), vec![1, 2, 3]);
}

#[test]
fn writer_kills_carry_out_chain() {
    // A flag *writer* genuinely kills an earlier carry-out.
    let exec_steps = steps(&[
        ("adds", "x4 x4 x6"), // 1: C is overwritten by 2 before any reader
        ("cmp", "x2 #0x0"),   // 2: a genuine writer, kills 1's carry-out
        ("adcs", "x8 x8 x9"), // 3: carry-in from 2
        ("csetm", "x10 cc"),  // 4: consumes 3's carry-out
    ]);
    assert_eq!(derive::derive_accumulate(&exec_steps), vec![2, 3]);
}

#[test]
fn no_accumulate_instructions_yields_empty() {
    let exec_steps = steps(&[
        ("ldp", "x4 x5 [x1]"),
        ("orr", "x3 x4 x5"),
        ("and", "x3 x3 #0xffffffff"),
        ("mov", "x9 #0x1"),
        ("stp", "x4 x5 [x0]"),
        ("ret", ""),
    ]);
    assert!(derive::derive_accumulate(&exec_steps).is_empty());
}

#[test]
fn unknown_mnemonic_is_not_a_candidate() {
    assert!(derive::derive_accumulate(&steps(&[("frobnicate", "x4 x5 x6")])).is_empty());
}

#[test]
fn custom_semantics_is_honoured() {
    let mut sem = derive::default_semantics();
    sem.insert(
        "vadds",
        InstructionSemantics {
            form: Some(Form::Add),
            dest: Some(0),
            sources: &[1, 2],
            reads_carry: false,
            writes_carry: true,
            reads_flags: false,
            self_carry_materialisation: false,
            always_link: false,
        },
    );
    let exec_steps = steps(&[("vadds", "x4 x4 x6"), ("csel", "x4 x4 x6 cc")]);
    assert_eq!(derive::derive_accumulate_with(&exec_steps, &sem), vec![1]);
}

// ---------------------------------------------------------------------------
// Real machine code: list-level comparison with the human tactic calls
// ---------------------------------------------------------------------------

/// Every step number named by an `ARM_ACCSTEPS_TAC` call, as a set-union.
fn human_union(proof: &Path) -> Vec<usize> {
    let src = fs::read_to_string(proof).unwrap();
    let mut out: BTreeSet<usize> = BTreeSet::new();
    let mut search = 0;
    while let Some(rel) = src[search..].find("ARM_ACCSTEPS_TAC") {
        let at = search + rel;
        let after = &src[at + "ARM_ACCSTEPS_TAC".len()..];
        let bytes = after.as_bytes();
        let mut i = 0;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
            i += 1;
        }
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i < bytes.len() && bytes[i] == b'[' {
            let end = after[i..].find(']').unwrap();
            for part in after[i + 1..i + end].split(';') {
                if let Ok(n) = part.trim().parse::<usize>() {
                    out.insert(n);
                }
            }
        } else if i < bytes.len() && bytes[i] == b'(' {
            let end = after[i..].find(')').unwrap();
            let inner = &after[i + 1..i + end];
            if let Some((lo, hi)) = inner.split_once("--") {
                let lo: usize = lo.trim().parse().unwrap();
                let hi: usize = hi.trim().parse().unwrap();
                for n in lo..=hi {
                    out.insert(n);
                }
            }
        }
        search = at + 1;
    }
    out.into_iter().collect()
}

#[test]
fn real_object_derivation_matches_the_reference_lists() {
    if !s2n().join("arm/p256").is_dir() {
        eprintln!(
            "skipping: {} is absent (the s2n-bignum bench is not checked in)",
            s2n().join("arm/p256").display()
        );
        return;
    }
    for op in ["add", "sub", "neg"] {
        let obj = s2n().join(format!("arm/p256/bignum_{op}_p256.o"));
        if !obj.exists() {
            eprintln!("skipping: {} is absent", obj.display());
            return;
        }
        let steps_list = disasm::disassemble(&obj, None, None).expect("disassembly");
        let derived = derive::derive_accumulate(&steps_list);
        let human = human_union(&s2n().join(format!("arm/proofs/bignum_{op}_p256.ml")));

        // Derived steps must all exist and be genuinely accumulating instructions.
        assert!(
            derived.iter().all(|n| human.contains(n)),
            "{op}: derived {derived:?} not within human {human:?}"
        );
        assert!(!derived.is_empty(), "empty derivation for {op}");

        if op == "sub" {
            // The human sub proof uses blanket ranges (1--8) and (10--17); the
            // derived set is the carry-chain subset.
            assert_eq!(derived, vec![3, 4, 7, 8, 10, 12, 13, 15]);
            assert!(derived.len() < human.len());
        } else {
            assert_eq!(
                derived, human,
                "{op} derivation differs from the human list"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// x86 flag mapping and materialisation
// ---------------------------------------------------------------------------

#[test]
fn x86_sub_flag_is_borrow_and_arm_is_no_borrow() {
    let meaning = derive::x86_cf_meaning();
    for op in ["sub", "sbb", "neg", "cmp"] {
        assert!(
            meaning[op].contains("borrow"),
            "{op} must document the borrow meaning"
        );
    }
    assert!(meaning["adc"].contains("carry"));

    let sem = derive::x86_semantics();
    assert!(sem["sbb"].reads_carry);
    assert!(!sem["sub"].reads_carry);
    assert!(sem["sub"].writes_carry);
}

#[test]
fn x86_self_carry_materialisation_is_excluded() {
    // `ADC r,r` / `SBB r,r` copy the flag; they sum no two digits.
    assert_eq!(
        x86(&[
            ("add", "rax rbx"),   // 1: carry-out consumed by 2
            ("adc", "rdx rdx"),   // 2: materialisation -> excluded
            ("sbb", "rcx rdx"),   // 3: reads the carry, writes a digit
            ("mov", "[rdi] rcx"), // 4
        ]),
        vec![1, 3]
    );
}

#[test]
fn x86_stc_grounds_the_first_carry_in() {
    assert_eq!(
        x86(&[
            ("stc", ""),          // 1: defines CF = 1, no value
            ("mov", "rax [rsi]"), // 2
            ("adc", "rax [rdx]"), // 3: carry-in from 1
            ("mov", "[rdi] rax"), // 4
        ]),
        vec![3]
    );
}

#[test]
fn x86_neg_is_a_value_link_but_is_dropped_by_the_emitter() {
    let rows = [
        ("neg", "rax"),       // 1: CF = (rax != 0), read by 2
        ("sbb", "rcx rdx"),   // 2
        ("mov", "[rdi] rcx"), // 3
    ];
    let derived = x86(&rows);
    assert!(derived.contains(&1) && derived.contains(&2));
    assert_eq!(emit::accumulate_set(&steps(&rows)), vec![2]);
}

#[test]
fn x86_logical_form_is_a_chain_boundary_not_a_digit() {
    assert_eq!(
        x86(&[
            ("add", "rax rbx"),   // 1: value link (always_link)
            ("and", "rdx rsi"),   // 2: clears CF; not an add/sub form
            ("adc", "rcx rdx"),   // 3: carry-in grounded by 2
            ("mov", "[rdi] rcx"), // 4
        ]),
        vec![1, 3]
    );
}

#[test]
fn arm_table_is_unchanged_by_the_x86_work() {
    let arm = derive::default_semantics();
    assert!(arm["sbcs"].reads_carry);
    assert!(!arm["adds"].always_link);
    assert!(derive::x86_semantics()["adc"].always_link);
}

// ---------------------------------------------------------------------------
// Real x86 objects: trimming, entry and derivation
// ---------------------------------------------------------------------------

fn x86_object(family: &str, op: &str) -> PathBuf {
    s2n()
        .join("x86")
        .join(family)
        .join(format!("bignum_{op}_{family}.o"))
}

#[test]
fn x86_endbr64_is_trimmed_and_addresses_are_trimmed_relative() {
    if !s2n().join("x86/p384").is_dir() {
        eprintln!(
            "skipping: {} is absent (the s2n-bignum bench is not checked in)",
            s2n().join("x86/p384").display()
        );
        return;
    }
    let steps_x86 = disasm::disassemble(&x86_object("p384", "add"), None, None).unwrap();
    // define_trimmed drops the four-byte ENDBR64, so the first word is MOV.
    assert_eq!(steps_x86[0].mnemonic, "mov");
    assert_eq!(steps_x86[0].address, 0);
    assert_eq!(steps_x86.len(), 42);
}

#[test]
fn x86_entry_offset_skips_the_push_prologue() {
    if !s2n().join("x86/p521").is_dir() {
        eprintln!(
            "skipping: {} is absent (the s2n-bignum bench is not checked in)",
            s2n().join("x86/p521").display()
        );
        return;
    }
    let full = disasm::disassemble(&x86_object("p521", "add"), None, None).unwrap();
    let first: Vec<&str> = full.iter().take(3).map(|s| s.mnemonic.as_str()).collect();
    assert_eq!(first, vec!["push", "push", "stc"]);
    let entered = disasm::enter_at(full, 3).unwrap();
    assert_eq!(entered[0].mnemonic, "stc");
    assert_eq!(entered[0].index, 1);
    assert_eq!(entered[0].address, 3);
}

#[test]
fn x86_real_accumulate_sets() {
    if !s2n().join("x86/p384").is_dir() {
        eprintln!(
            "skipping: {} is absent (the s2n-bignum bench is not checked in)",
            s2n().join("x86/p384").display()
        );
        return;
    }
    let expected: [(&str, &str, Vec<usize>); 3] = [
        (
            "p384",
            "sub",
            vec![2, 4, 6, 8, 10, 12, 17, 18, 20, 25, 27, 29, 31],
        ),
        ("p384", "neg", vec![19, 20, 21, 22, 23, 24]),
        ("p521", "neg", vec![]),
    ];
    for (family, op, want) in expected {
        let obj = x86_object(family, op);
        if !obj.exists() {
            eprintln!("skipping: {} is absent", obj.display());
            return;
        }
        let steps_x86 = disasm::disassemble(&obj, None, None).unwrap();
        let got = emit::accumulate_set(&steps_x86);
        assert_eq!(got, want, "{family}/{op}");
    }
}

// ---------------------------------------------------------------------------
// Held-out refusal
// ---------------------------------------------------------------------------

#[test]
fn x86_emitter_refuses_heldout() {
    for task in [
        "x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT",
        "x86.bignum_sub_p256k1.BIGNUM_SUB_P256K1_CORRECT",
        "x86.bignum_neg_sm2.BIGNUM_NEG_SM2_CORRECT",
    ] {
        let rest = task.split_once('_').unwrap().1;
        let op = rest.split_once('_').unwrap().0;
        assert!(
            emit::build_x86(op, task, &guard::develop_policy(), &repo()).is_err(),
            "{task} was not refused"
        );
    }
}
