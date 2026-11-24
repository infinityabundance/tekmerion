//! Native checker conformance: two real identities, the trusted rules, the tamper
//! suite, determinism, and the no-process guarantee.
//!
//! The CNF/LRAT fixtures were generated offline. CaDiCaL 2.1.2 proposed the LRAT
//! refutations; `examples/native_fixtures.rs` packaged them. Neither is invoked by
//! these tests. The checker reads the committed bytes.

use std::collections::BTreeMap;

use tekmerion::native::{check, check_lrat, Certificate, Error, LratError, Step, Term};
use tekmerion::native::{Blob, BlobKind, Rule};

const ADD_COMM_CERT: &str = include_str!("fixtures/native/add_comm_8.cert");
const SUB_ADD_CERT: &str = include_str!("fixtures/native/sub_add_64.cert");

fn run(text: &str) -> Result<tekmerion::native::Theorem, Error> {
    let cert = Certificate::parse(text)?;
    check(&cert)
}

fn var(width: u32, name: &str) -> Term {
    Term::var(width, name).expect("valid variable")
}

fn eq_of(left: Term, right: Term) -> Term {
    Term::eq(left, right).expect("well-sorted equality")
}

#[test]
fn proves_add_comm_8_natively() {
    let theorem = run(ADD_COMM_CERT).expect("8-bit addition commutes");
    assert_eq!(
        theorem.render(),
        "(eq bv8 (add bv8 (var bv8 x) (var bv8 y)) (add bv8 (var bv8 y) (var bv8 x)))"
    );
}

#[test]
fn proves_sub_add_64_natively() {
    let theorem = run(SUB_ADD_CERT).expect("64-bit (x + y) - y = x");
    assert_eq!(
        theorem.render(),
        "(eq bv64 (sub bv64 (add bv64 (var bv64 x) (var bv64 y)) (var bv64 y)) (var bv64 x))"
    );
    // The stated side condition is vacuous: subtraction is modulo 2^64, so the
    // carry out is discarded. Check extreme assignments against the same
    // semantics the bit-blaster encodes.
    let x = var(64, "x");
    let y = var(64, "y");
    let sum = Term::add(x.clone(), y.clone()).unwrap();
    let identity = eq_of(Term::sub(sum, y.clone()).unwrap(), x.clone());
    for (xv, yv) in [
        (0, 0),
        (1, 0),
        (0, 1),
        (u64::MAX, 1),
        (u64::MAX, u64::MAX),
        (1 << 63, 1 << 63),
        (0x0123_4567_89ab_cdef, 0xfedc_ba98_7654_3210),
    ] {
        let mut env = BTreeMap::new();
        env.insert("x".to_string(), xv);
        env.insert("y".to_string(), yv);
        assert_eq!(
            identity.eval(&env).unwrap(),
            tekmerion::native::Value::Bool(true),
            "counterexample x={xv:#x} y={yv:#x}"
        );
    }
}

#[test]
fn determinism_same_verdict_and_digest() {
    for text in [ADD_COMM_CERT, SUB_ADD_CERT] {
        let a = run(text).expect("first check");
        let b = run(text).expect("second check");
        assert_eq!(a.digest(), b.digest());
        assert_eq!(a.render(), b.render());
    }
}

#[test]
fn refl_sym_trans_congr_are_sound_and_checked() {
    let x = var(8, "x");
    let y = var(8, "y");
    let xx = eq_of(x.clone(), x.clone());
    let xy = eq_of(x.clone(), y.clone());
    let yx = eq_of(y.clone(), x.clone());

    let refl = Step::refl(0, xx.clone());
    let sym = Step::sym(1, 0, xx.clone());
    let trans = Step::trans(2, 0, 1, xx.clone());
    let congr = Step::congr(
        3,
        0,
        eq_of(
            Term::add(x.clone(), y.clone()).unwrap(),
            Term::add(x.clone(), y.clone()).unwrap(),
        ),
    );

    let cert = Certificate::new(3, vec![refl, sym, trans, congr], vec![]).expect("valid");
    let theorem = check(&cert).expect("rules check");
    assert_eq!(
        theorem.render(),
        "(eq bv8 (add bv8 (var bv8 x) (var bv8 y)) (add bv8 (var bv8 x) (var bv8 y)))"
    );

    // A sym step that does not exchange the sides is refused.
    let yy = eq_of(y.clone(), y.clone());
    let bad = Certificate::new(
        0,
        vec![Step::sym(0, 1, xx.clone()), Step::refl(1, yy)],
        vec![],
    )
    .expect("structurally valid");
    assert!(matches!(check(&bad), Err(Error::RuleMismatch(_))));
    let _ = (xy, yx);
}

#[test]
fn tamper_flip_one_hint_is_rejected() {
    let text = mutate_payload(ADD_COMM_CERT, BlobKind::Lrat, |bytes| {
        let s = std::str::from_utf8(bytes).unwrap();
        let mut changed = String::new();
        let mut done = false;
        for line in s.lines() {
            if !done && !line.starts_with('d') {
                if let Some(z) = line.split(' ').position(|t| t == "0") {
                    let mut tokens: Vec<&str> = line.split(' ').collect();
                    if z + 1 < tokens.len() {
                        tokens[z + 1] = "999999";
                        changed.push_str(&tokens.join(" "));
                        changed.push('\n');
                        done = true;
                        continue;
                    }
                }
            }
            changed.push_str(line);
            changed.push('\n');
        }
        changed.into_bytes()
    });
    let err = run(&text).expect_err("tampered hint must be rejected");
    assert!(
        matches!(err, Error::Lrat(LratError::UnknownClause { .. })),
        "got {err}"
    );
    assert!(err.to_string().contains("R-L07"));
}

#[test]
fn tamper_flip_one_hint_sign_is_rejected() {
    let text = mutate_payload(ADD_COMM_CERT, BlobKind::Lrat, |bytes| {
        let s = std::str::from_utf8(bytes).unwrap();
        let mut changed = String::new();
        let mut done = false;
        for line in s.lines() {
            if !done && !line.starts_with('d') {
                if let Some(z) = line.split(' ').position(|t| t == "0") {
                    let mut tokens: Vec<String> = line.split(' ').map(|t| t.to_string()).collect();
                    if z + 1 < tokens.len() && tokens[z + 1] != "0" {
                        tokens[z + 1] = format!("-{}", tokens[z + 1]);
                        changed.push_str(&tokens.join(" "));
                        changed.push('\n');
                        done = true;
                        continue;
                    }
                }
            }
            changed.push_str(line);
            changed.push('\n');
        }
        changed.into_bytes()
    });
    let err = run(&text).expect_err("a negative (RAT) hint must be rejected");
    assert!(
        matches!(err, Error::Lrat(LratError::RatUnsupported { .. })),
        "got {err}"
    );
    assert!(err.to_string().contains("R-L10"));
}

#[test]
fn tamper_delete_a_clause_is_rejected() {
    let text = mutate_payload(ADD_COMM_CERT, BlobKind::Lrat, |bytes| {
        let s = std::str::from_utf8(bytes).unwrap();
        // Drop the first addition. Later hints reference it.
        s.lines()
            .skip(1)
            .map(|l| format!("{l}\n"))
            .collect::<String>()
            .into_bytes()
    });
    let err = run(&text).expect_err("deleted clause must be rejected");
    assert!(
        matches!(err, Error::Lrat(LratError::UnknownClause { .. })),
        "got {err}"
    );
}

#[test]
fn tamper_duplicate_field_is_rejected() {
    let mut text = String::new();
    for line in ADD_COMM_CERT.lines() {
        text.push_str(line);
        text.push('\n');
        if line.starts_with("blob lrat ") {
            text.push_str(line);
            text.push('\n');
        }
    }
    let err = Certificate::parse(&text).expect_err("duplicate blob must be rejected");
    assert!(matches!(err, Error::DuplicateBlob(_)), "got {err}");
    assert!(err.to_string().contains("R-N32"));
}

#[test]
fn tamper_duplicate_step_is_rejected() {
    let mut text = String::new();
    for line in ADD_COMM_CERT.lines() {
        text.push_str(line);
        text.push('\n');
        if line.starts_with("step ") {
            text.push_str(line);
            text.push('\n');
        }
    }
    let err = Certificate::parse(&text).expect_err("duplicate step must be rejected");
    assert!(matches!(err, Error::DuplicateStep(_)), "got {err}");
    assert!(err.to_string().contains("R-N27"));
}

#[test]
fn tamper_truncated_file_is_rejected() {
    let text = &ADD_COMM_CERT[..ADD_COMM_CERT.len() - 10];
    let err = Certificate::parse(text).expect_err("truncated certificate must be rejected");
    assert!(matches!(err, Error::Truncated), "got {err}");
    assert!(err.to_string().contains("R-N22"));
}

#[test]
fn tamper_trailing_bytes_are_rejected() {
    let text = format!("{ADD_COMM_CERT}trailing\n");
    let err = Certificate::parse(&text).expect_err("trailing bytes must be rejected");
    assert!(matches!(err, Error::TrailingGarbage), "got {err}");
    assert!(err.to_string().contains("R-N23"));
}

#[test]
fn tamper_cycle_is_rejected() {
    let x = var(8, "x");
    let y = var(8, "y");
    let xy = eq_of(x.clone(), y.clone());
    let yx = eq_of(y, x);
    let steps = vec![Step::sym(0, 1, xy), Step::sym(1, 0, yx)];
    let err = Certificate::new(0, steps, vec![]).expect_err("cycle must be rejected");
    assert!(matches!(err, Error::Cycle(_)), "got {err}");
    assert!(err.to_string().contains("R-N29"));
}

#[test]
fn tamper_unknown_rule_is_rejected() {
    let text = ADD_COMM_CERT.replacen(" bitblast ", " bogus ", 1);
    let err = Certificate::parse(&text).expect_err("unknown rule must be rejected");
    assert!(matches!(err, Error::UnknownRule(_)), "got {err}");
    assert!(err.to_string().contains("R-N26"));
}

#[test]
fn tamper_missing_step_is_rejected() {
    let x = var(8, "x");
    let y = var(8, "y");
    let yx = eq_of(y, x);
    let err = Certificate::new(0, vec![Step::sym(0, 7, yx)], vec![])
        .expect_err("dangling reference must be rejected");
    assert!(matches!(err, Error::DanglingRef(7)), "got {err}");
    assert!(err.to_string().contains("R-N28"));
}

#[test]
fn tamper_cnf_is_rejected() {
    let text = mutate_payload(ADD_COMM_CERT, BlobKind::Cnf, |bytes| {
        let s = std::str::from_utf8(bytes).unwrap();
        let mut lines: Vec<String> = s.lines().map(|l| l.to_string()).collect();
        // Flip the sign of the last literal of the first clause. The result is
        // still canonical DIMACS but is not the bit-blast of the proposition.
        lines[1] = "-1 -9 17 0".to_string();
        let mut out = String::new();
        for line in lines {
            out.push_str(&line);
            out.push('\n');
        }
        out.into_bytes()
    });
    let err = run(&text).expect_err("non-canonical CNF must be rejected");
    assert!(matches!(err, Error::CnfMismatch), "got {err}");
    assert!(err.to_string().contains("R-N51"));
}

#[test]
fn tamper_lrat_trailing_garbage_is_rejected() {
    let text = mutate_payload(ADD_COMM_CERT, BlobKind::Lrat, |bytes| {
        let s = std::str::from_utf8(bytes).unwrap();
        format!("{s}1 0 0\n").into_bytes()
    });
    let err = run(&text).expect_err("garbage after empty clause must be rejected");
    assert!(
        matches!(err, Error::Lrat(LratError::TrailingGarbage { .. })),
        "got {err}"
    );
}

#[test]
fn tamper_lrat_unknown_literal_is_rejected() {
    let text = mutate_payload(ADD_COMM_CERT, BlobKind::Lrat, |bytes| {
        let s = std::str::from_utf8(bytes).unwrap();
        let mut done = false;
        let mut out = String::new();
        for line in s.lines() {
            if !done && !line.starts_with('d') {
                let mut tokens: Vec<&str> = line.split(' ').collect();
                tokens[1] = "200";
                out.push_str(&tokens.join(" "));
                out.push('\n');
                done = true;
                continue;
            }
            out.push_str(line);
            out.push('\n');
        }
        out.into_bytes()
    });
    let err = run(&text).expect_err("out-of-range literal must be rejected");
    assert!(
        matches!(err, Error::Lrat(LratError::LiteralOutOfRange { .. })),
        "got {err}"
    );
}

#[test]
fn lrat_truncation_is_rejected_directly() {
    let cert = Certificate::parse(ADD_COMM_CERT).expect("fixture");
    let cnf_bytes = cert
        .blobs()
        .iter()
        .find(|b| b.kind() == BlobKind::Cnf)
        .unwrap()
        .bytes();
    let cnf = tekmerion::native::Cnf::parse_dimacs(cnf_bytes).expect("canonical CNF");
    let mut proof = String::from_utf8(
        cert.blobs()
            .iter()
            .find(|b| b.kind() == BlobKind::Lrat)
            .unwrap()
            .bytes()
            .to_vec(),
    )
    .unwrap();
    proof.pop(); // remove the final newline
    let err = check_lrat(&cnf, &proof).expect_err("truncated LRAT must be rejected");
    assert!(matches!(err, LratError::Truncated { .. }), "got {err}");
}

#[test]
fn lrat_small_refutation_accepts_and_deletion_is_honoured() {
    let cnf = tekmerion::native::Cnf::parse_dimacs(b"p cnf 2 3\n1 2 0\n-1 2 0\n-2 0\n")
        .expect("small CNF");
    let proof = "4 1 0 3 1 0\n5 0 4 3 2 0\n";
    let stats = check_lrat(&cnf, proof).expect("valid RUP refutation");
    assert_eq!(stats.additions, 2);
    assert_eq!(stats.hints, 5);

    // Deleting a clause that the refutation still needs is refused.
    let deleted = "4 1 0 3 1 0\nd 3 0\n5 0 4 3 2 0\n";
    let err = check_lrat(&cnf, deleted).expect_err("deleted hint must fail");
    assert!(matches!(err, LratError::DeletedClause { .. }), "got {err}");
}

#[test]
fn lrat_bad_hint_and_missing_empty_clause_are_rejected() {
    let cnf = tekmerion::native::Cnf::parse_dimacs(b"p cnf 2 3\n1 2 0\n-1 2 0\n-2 0\n")
        .expect("small CNF");
    // The second use of clause 1 is satisfied, not unit.
    let bad = "4 1 0 1 1 0\n5 0 4 3 2 0\n";
    let err = check_lrat(&cnf, bad).expect_err("satisfied hint must fail");
    assert!(matches!(err, LratError::BadHint { .. }), "got {err}");

    let no_empty = "4 1 0 3 1 0\n";
    let err = check_lrat(&cnf, no_empty).expect_err("missing empty clause must fail");
    assert!(matches!(err, LratError::NoEmptyClause), "got {err}");
}

#[test]
fn lrat_negative_zero_is_rejected() {
    let cnf = tekmerion::native::Cnf::parse_dimacs(b"p cnf 2 3\n1 2 0\n-1 2 0\n-2 0\n")
        .expect("small CNF");
    let err =
        check_lrat(&cnf, "4 -0 1 0 3 1 0\n5 0 4 3 2 0\n").expect_err("-0 is not a canonical zero");
    assert!(matches!(err, LratError::BadInteger { .. }), "got {err}");
}

#[test]
fn hand_built_ill_typed_terms_are_refused_not_panicked() {
    let bad = Term::Add {
        width: 8,
        left: Box::new(Term::Bool(true)),
        right: Box::new(Term::Bool(false)),
    };
    let conclusion = Term::Eq {
        width: 8,
        left: Box::new(bad.clone()),
        right: Box::new(bad),
    };
    let err = Certificate::new(0, vec![Step::refl(0, conclusion)], vec![])
        .expect_err("ill-typed conclusion must be refused");
    assert!(matches!(err, Error::SortMismatch { .. }), "got {err}");

    let err = tekmerion::native::bitblast(&Term::Const {
        width: 100,
        value: 0,
    })
    .expect_err("out-of-range width must be refused");
    assert!(matches!(err, Error::WidthOutOfRange(100)), "got {err}");
}

#[test]
fn tamper_reused_proof_for_a_different_statement_is_rejected() {
    // Keeping the CNF/LRAT but changing the claimed proposition must fail: the
    // attached CNF is recomputed from the proposition and compared exactly.
    let cert = Certificate::parse(ADD_COMM_CERT).expect("fixture");
    let x = var(8, "x");
    let wrong = eq_of(
        Term::add(x.clone(), Term::constant(8, 1).unwrap()).unwrap(),
        x,
    );
    let steps: Vec<Step> = cert
        .steps()
        .iter()
        .map(|s| match s.rule() {
            Rule::Bitblast { cnf, lrat } => Step::bitblast(s.id(), *cnf, *lrat, wrong.clone()),
            _ => s.clone(),
        })
        .collect();
    let cert = Certificate::new(cert.theorem_id(), steps, cert.blobs().to_vec())
        .expect("structurally valid");
    let err = check(&cert).expect_err("transplanted proof must be rejected");
    assert!(matches!(err, Error::CnfMismatch), "got {err}");
}

#[test]
fn term_grammar_refuses_unknown_and_ambiguous_encodings() {
    assert!(matches!(
        Term::parse("(xor bv8 (var bv8 x) (var bv8 y))"),
        Err(Error::UnknownTermTag(_))
    ));
    assert!(matches!(
        Term::parse("(var bv8 9x)"),
        Err(Error::BadVarName(_))
    ));
    assert!(matches!(
        Term::parse("(var bv8 x )"),
        Err(Error::NonCanonicalTerm(_))
    ));
    assert!(matches!(
        Term::parse("(const bv8 300)"),
        Err(Error::ConstOutOfRange { .. })
    ));
    assert!(matches!(
        Term::parse("(eq bv8 true false)"),
        Err(Error::SortMismatch { .. })
    ));
    assert!(matches!(
        Term::parse("(add bv8 (var bv4 x) (var bv8 y))"),
        Err(Error::WidthMismatch { .. })
    ));
    assert!(matches!(
        Term::parse("(var bv0 x)"),
        Err(Error::WidthOutOfRange(0))
    ));
    assert!(matches!(
        Term::parse("(var bv65 x)"),
        Err(Error::WidthOutOfRange(65))
    ));
}

#[test]
fn bitblast_agrees_with_semantics_on_small_cases() {
    // A valid identity must bit-blast to UNSAT; a false one must remain SAT.
    let x2 = var(2, "x");
    let one = Term::constant(2, 1).unwrap();
    let wrong = eq_of(Term::add(x2.clone(), one.clone()).unwrap(), x2.clone());
    assert!(
        cnf_satisfiable(&tekmerion::native::bitblast(&wrong).unwrap()),
        "x + 1 = x is false and must not be refutable"
    );

    let x3 = var(3, "x");
    let y3 = var(3, "y");
    let comm = eq_of(
        Term::add(x3.clone(), y3.clone()).unwrap(),
        Term::add(y3.clone(), x3.clone()).unwrap(),
    );
    assert!(!cnf_satisfiable(
        &tekmerion::native::bitblast(&comm).unwrap()
    ));

    let non_identity = eq_of(
        Term::add(Term::add(x3.clone(), y3.clone()).unwrap(), y3.clone()).unwrap(),
        x3.clone(),
    );
    assert!(cnf_satisfiable(
        &tekmerion::native::bitblast(&non_identity).unwrap()
    ));

    // Closed terms exercise literal folding and wrap-around.
    let sum = Term::add(Term::constant(2, 3).unwrap(), Term::constant(2, 1).unwrap()).unwrap();
    assert!(!cnf_satisfiable(
        &tekmerion::native::bitblast(&eq_of(sum, Term::constant(2, 0).unwrap())).unwrap()
    ));
    let wrong_closed = eq_of(
        Term::add(Term::constant(2, 1).unwrap(), Term::constant(2, 1).unwrap()).unwrap(),
        Term::constant(2, 3).unwrap(),
    );
    assert!(cnf_satisfiable(
        &tekmerion::native::bitblast(&wrong_closed).unwrap()
    ));
}

fn cnf_satisfiable(cnf: &tekmerion::native::Cnf) -> bool {
    let mut assign = vec![0i8; cnf.num_vars() as usize + 1];
    dpll(cnf.clauses(), &mut assign)
}

fn dpll(clauses: &[Vec<i32>], assign: &mut [i8]) -> bool {
    loop {
        let mut progress = false;
        for clause in clauses {
            let mut open = 0usize;
            let mut last = 0i32;
            let mut satisfied = false;
            for &lit in clause {
                match assign[lit.unsigned_abs() as usize] {
                    0 => {
                        open += 1;
                        last = lit;
                    }
                    v if (v > 0) == (lit > 0) => {
                        satisfied = true;
                        break;
                    }
                    _ => {}
                }
            }
            if satisfied {
                continue;
            }
            if open == 0 {
                return false;
            }
            if open == 1 {
                assign[last.unsigned_abs() as usize] = if last > 0 { 1 } else { -1 };
                progress = true;
            }
        }
        if !progress {
            break;
        }
    }

    for clause in clauses {
        let mut satisfied = false;
        let mut target = None;
        for &lit in clause {
            let v = lit.unsigned_abs() as usize;
            match assign[v] {
                0 => {
                    target = Some(v);
                    break;
                }
                x if (x > 0) == (lit > 0) => {
                    satisfied = true;
                    break;
                }
                _ => {}
            }
        }
        if satisfied {
            continue;
        }
        if let Some(v) = target {
            let mut trial = assign.to_vec();
            trial[v] = 1;
            if dpll(clauses, &mut trial) {
                assign.copy_from_slice(&trial);
                return true;
            }
            assign[v] = -1;
            return dpll(clauses, assign);
        }
    }
    true
}

#[test]
fn checker_path_contains_no_process_spawning() {
    let modules: [(&str, &str); 6] = [
        ("native/mod.rs", include_str!("../src/native/mod.rs")),
        ("native/logic.rs", include_str!("../src/native/logic.rs")),
        (
            "native/bitblast.rs",
            include_str!("../src/native/bitblast.rs"),
        ),
        ("native/lrat.rs", include_str!("../src/native/lrat.rs")),
        ("native/cert.rs", include_str!("../src/native/cert.rs")),
        ("native/check.rs", include_str!("../src/native/check.rs")),
    ];
    for (name, src) in modules {
        for needle in ["std::process", "process::", "Command", "spawn("] {
            assert!(
                !src.contains(needle),
                "checker module {name} contains process-spawning token {needle:?}"
            );
        }
    }
}

/// Rebuild a certificate after replacing one payload blob. The step digest and the
/// blob digest are recomputed, so the only change the checker can see is the
/// payload itself.
fn mutate_payload<F>(text: &str, kind: BlobKind, f: F) -> String
where
    F: FnOnce(&[u8]) -> Vec<u8>,
{
    let cert = Certificate::parse(text).expect("fixture parses");
    let target = cert
        .blobs()
        .iter()
        .find(|b| b.kind() == kind)
        .expect("payload present");
    let old_digest = target.digest();
    let new_blob = Blob::new(kind, f(target.bytes()));
    let new_digest = new_blob.digest();

    let steps: Vec<Step> = cert
        .steps()
        .iter()
        .map(|s| match s.rule() {
            Rule::Bitblast { cnf, lrat } => {
                let cnf = if *cnf == old_digest { new_digest } else { *cnf };
                let lrat = if *lrat == old_digest {
                    new_digest
                } else {
                    *lrat
                };
                Step::bitblast(s.id(), cnf, lrat, s.conclusion().clone())
            }
            _ => s.clone(),
        })
        .collect();
    let blobs: Vec<Blob> = cert
        .blobs()
        .iter()
        .filter(|b| b.digest() != old_digest)
        .cloned()
        .chain(std::iter::once(new_blob))
        .collect();
    Certificate::new(cert.theorem_id(), steps, blobs)
        .expect("rebuilt certificate is structurally valid")
        .render()
}

/// TNPS §1.7 (P1): an accepted DIMACS input is exactly its own canonical
/// rendering. `Term::parse` and `Certificate::parse` already enforced this;
/// `Cnf::parse_dimacs` now does too, so the property holds by construction
/// rather than only through the individual structural checks.
#[test]
fn dimacs_acceptance_is_exactly_the_canonical_rendering() {
    let good: [&[u8]; 3] = [
        b"p cnf 2 3\n1 2 0\n-1 2 0\n-2 0\n",
        b"p cnf 1 1\n0\n",
        b"p cnf 3 1\n1 -2 3 0\n",
    ];
    for bytes in good {
        let cnf = tekmerion::native::Cnf::parse_dimacs(bytes).expect("canonical CNF");
        assert_eq!(cnf.to_dimacs().as_bytes(), bytes);
    }

    let bad: [&[u8]; 4] = [
        b"p cnf 2 3\n1 2 0\n-1 2 0\n-2 0",
        b"p cnf 2 3\n1 2 0\n-1 2 0\n-2 0\n\n",
        b"p cnf 2 3\n 1 2 0\n-1 2 0\n-2 0\n",
        b"p cnf 2 3\n1 2 0\n-1 2 0\n-2  0\n",
    ];
    for bytes in bad {
        assert!(
            tekmerion::native::Cnf::parse_dimacs(bytes).is_err(),
            "non-canonical DIMACS accepted: {bytes:?}"
        );
    }
}
