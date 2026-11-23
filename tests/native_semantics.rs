//! Milestone B conformance: the signed checker against an independent oracle.
//!
//! The oracle in [`tekmerion::native::reference`] is test-only. This file
//!
//! 1. generates genuine LRAT refutations for small bit-widths with a deterministic
//!    DPLL prover (no SAT solver, no process, no new crate),
//! 2. compares the signed checker and the oracle on acceptance and on the existing
//!    mutation families plus new ones,
//! 3. checks parser canonicality (parse-render round trips, malformed input), and
//! 4. fuzzes the parsers with a fixed-seed PRNG, asserting only "no panic".
//!
//! Nothing here is part of the trusted path.

use std::collections::BTreeMap;

use tekmerion::native::reference;
use tekmerion::native::{
    bitblast, check, check_lrat, Blob, BlobKind, Certificate, Cnf, Rule, Step, Term, Value,
};

// ---------------------------------------------------------------------------
// Deterministic LRAT generation
// ---------------------------------------------------------------------------

/// Clause database plus an LRAT writer. Original clauses keep ids `1..=m`; every
/// derived clause gets the next fresh id.
struct Prover {
    db: BTreeMap<u64, Vec<i32>>,
    next_id: u64,
    max_id: u64,
    nvars: u32,
    lines: String,
}

impl Prover {
    fn new(cnf: &Cnf) -> Self {
        let mut db = BTreeMap::new();
        for (i, clause) in cnf.clauses().iter().enumerate() {
            db.insert((i + 1) as u64, clause.clone());
        }
        let m = cnf.clauses().len() as u64;
        Prover {
            db,
            next_id: m + 1,
            max_id: m,
            nvars: cnf.num_vars(),
            lines: String::new(),
        }
    }

    fn add(&mut self, clause: Vec<i32>, hints: &[u64]) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.lines.push_str(&id.to_string());
        for lit in &clause {
            self.lines.push(' ');
            self.lines.push_str(&lit.to_string());
        }
        self.lines.push_str(" 0");
        for hint in hints {
            self.lines.push(' ');
            self.lines.push_str(&hint.to_string());
        }
        self.lines.push_str(" 0\n");
        debug_assert!(id > self.max_id);
        self.max_id = id;
        self.db.insert(id, clause);
        id
    }

    /// Unit propagation to fixpoint. Returns the first conflicting clause id.
    fn propagate(
        &self,
        assign: &mut [i8],
        reason: &mut [u64],
        order: &mut Vec<i32>,
    ) -> Option<u64> {
        loop {
            let mut changed = false;
            for (&cid, clause) in self.db.iter() {
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
                    return Some(cid);
                }
                if open == 1 {
                    let var = last.unsigned_abs() as usize;
                    if assign[var] == 0 {
                        assign[var] = if last > 0 { 1 } else { -1 };
                        reason[var] = cid;
                        order.push(last);
                        changed = true;
                    }
                }
            }
            if !changed {
                return None;
            }
        }
    }

    /// Build a refutation and return the id of the empty clause.
    fn prove(&mut self) -> Result<u64, String> {
        let mut decisions: Vec<i32> = Vec::new();
        let root = self.solve(&mut decisions)?;
        if !self.db.get(&root).map(Vec::is_empty).unwrap_or(false) {
            return Err("root clause is not empty".to_string());
        }
        Ok(root)
    }

    fn solve(&mut self, decisions: &mut Vec<i32>) -> Result<u64, String> {
        let mut assign = vec![0i8; self.nvars as usize + 1];
        let mut reason = vec![0u64; self.nvars as usize + 1];
        let mut order: Vec<i32> = Vec::new();
        for &lit in decisions.iter() {
            assign[lit.unsigned_abs() as usize] = if lit > 0 { 1 } else { -1 };
        }

        if let Some(conflict) = self.propagate(&mut assign, &mut reason, &mut order) {
            return self.derive_decision_clause(decisions, &order, &reason, conflict);
        }

        // No conflict: pick a literal from the shortest still-open clause.
        let mut choice: Option<(usize, i32)> = None;
        for clause in self.db.values() {
            let mut open = 0usize;
            let mut first = 0i32;
            let mut satisfied = false;
            for &lit in clause {
                match assign[lit.unsigned_abs() as usize] {
                    0 => {
                        open += 1;
                        if first == 0 {
                            first = lit;
                        }
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
            if choice.map(|(best, _)| open < best).unwrap_or(true) {
                choice = Some((open, first));
            }
        }
        let Some((_, lit)) = choice else {
            return Err("formula is satisfiable: no refutation exists".to_string());
        };
        let var = lit.unsigned_abs();

        decisions.push(var as i32);
        let left = self.solve(decisions)?;
        decisions.pop();
        decisions.push(-(var as i32));
        let right = self.solve(decisions)?;
        decisions.pop();

        let negated = negate(decisions);
        Ok(self.add(negated, &[left, right]))
    }

    /// From a conflict under the closure of `decisions`, derive exactly `¬decisions`.
    ///
    /// Checking `¬decisions` assumes `decisions`; the recorded reasons then
    /// unit-propagate in the same order and the conflicting clause closes the
    /// derivation. No closure literals appear in the conclusion, so no hint is
    /// already satisfied when it is replayed.
    fn derive_decision_clause(
        &mut self,
        decisions: &[i32],
        order: &[i32],
        reason: &[u64],
        conflict: u64,
    ) -> Result<u64, String> {
        let mut hints: Vec<u64> = order
            .iter()
            .map(|lit| reason[lit.unsigned_abs() as usize])
            .collect();
        if hints.iter().any(|&r| r == 0) {
            return Err("propagated literal has no reason".to_string());
        }
        hints.push(conflict);
        Ok(self.add(negate(decisions), &hints))
    }
}

fn negate(literals: &[i32]) -> Vec<i32> {
    literals.iter().map(|&l| -l).collect()
}

/// Build a one-step `bitblast` certificate for `prop` using the local prover.
fn build_cert(prop: &Term) -> Certificate {
    let cnf = bitblast(prop).expect("bit-blast succeeds");
    let mut prover = Prover::new(&cnf);
    prover.prove().expect("small instance is refutable");
    let cnf_blob = Blob::new(BlobKind::Cnf, cnf.to_dimacs().into_bytes());
    let lrat_blob = Blob::new(BlobKind::Lrat, prover.lines.into_bytes());
    let step = Step::bitblast(0, cnf_blob.digest(), lrat_blob.digest(), prop.clone());
    Certificate::new(0, vec![step], vec![cnf_blob, lrat_blob]).expect("valid certificate")
}

// ---------------------------------------------------------------------------
// Propositions
// ---------------------------------------------------------------------------

fn word_var(width: u32, name: &str) -> Term {
    Term::var(width, name).expect("valid variable")
}

fn eq_of(left: Term, right: Term) -> Term {
    Term::eq(left, right).expect("well-sorted equality")
}

fn add_comm(width: u32) -> Term {
    let x = word_var(width, "x");
    let y = word_var(width, "y");
    eq_of(
        Term::add(x.clone(), y.clone()).unwrap(),
        Term::add(y, x).unwrap(),
    )
}

fn sub_add(width: u32) -> Term {
    let x = word_var(width, "x");
    let y = word_var(width, "y");
    let sum = Term::add(x.clone(), y.clone()).unwrap();
    eq_of(Term::sub(sum, y).unwrap(), x)
}

fn add_zero(width: u32) -> Term {
    let x = word_var(width, "x");
    let zero = Term::constant(width, 0).unwrap();
    eq_of(Term::add(x.clone(), zero).unwrap(), x)
}

fn add_assoc(width: u32) -> Term {
    let x = word_var(width, "x");
    let y = word_var(width, "y");
    let z = word_var(width, "z");
    let left = Term::add(Term::add(x.clone(), y.clone()).unwrap(), z.clone()).unwrap();
    let right = Term::add(x, Term::add(y, z).unwrap()).unwrap();
    eq_of(left, right)
}

// ---------------------------------------------------------------------------
// Verdicts and mutation helpers
// ---------------------------------------------------------------------------

/// `(signed accepted, oracle accepted)`. A front-end parse failure is a rejection
/// by both, since neither checker is reached with ill-formed bytes.
fn verdicts(text: &str) -> (bool, bool) {
    match Certificate::parse(text) {
        Err(_) => (false, false),
        Ok(cert) => (check(&cert).is_ok(), reference::accepts(&cert).is_ok()),
    }
}

fn assert_agree(text: &str, expected: bool, label: &str) {
    let (signed, oracle) = verdicts(text);
    assert_eq!(signed, oracle, "checkers disagree on {label}");
    assert_eq!(signed, expected, "unexpected verdict on {label}");
}

fn rebuild_with_blob<F>(cert: &Certificate, kind: BlobKind, f: F) -> String
where
    F: FnOnce(&[u8]) -> Vec<u8>,
{
    let target = cert
        .blobs()
        .iter()
        .find(|b| b.kind() == kind)
        .expect("payload present");
    let old = target.digest();
    let new_blob = Blob::new(kind, f(target.bytes()));
    let new_digest = new_blob.digest();
    let steps: Vec<Step> = cert
        .steps()
        .iter()
        .map(|s| match s.rule() {
            Rule::Bitblast { cnf, lrat } => Step::bitblast(
                s.id(),
                if *cnf == old { new_digest } else { *cnf },
                if *lrat == old { new_digest } else { *lrat },
                s.conclusion().clone(),
            ),
            _ => s.clone(),
        })
        .collect();
    let blobs: Vec<Blob> = cert
        .blobs()
        .iter()
        .filter(|b| b.digest() != old)
        .cloned()
        .chain(std::iter::once(new_blob))
        .collect();
    Certificate::new(cert.theorem_id(), steps, blobs)
        .expect("rebuilt certificate is structurally valid")
        .render()
}

fn rewrite_lrat<F>(cert: &Certificate, f: F) -> String
where
    F: FnOnce(&str) -> String,
{
    rebuild_with_blob(cert, BlobKind::Lrat, |bytes| {
        f(std::str::from_utf8(bytes).expect("LRAT is UTF-8")).into_bytes()
    })
}

/// Replace the claimed proposition of every `bitblast` step, keeping the blobs.
fn with_conclusion(cert: &Certificate, wrong: &Term) -> String {
    let steps: Vec<Step> = cert
        .steps()
        .iter()
        .map(|s| match s.rule() {
            Rule::Bitblast { cnf, lrat } => Step::bitblast(s.id(), *cnf, *lrat, wrong.clone()),
            _ => s.clone(),
        })
        .collect();
    Certificate::new(cert.theorem_id(), steps, cert.blobs().to_vec())
        .expect("structurally valid")
        .render()
}

// ---------------------------------------------------------------------------
// 1. Differential acceptance
// ---------------------------------------------------------------------------

#[test]
fn differential_acceptance_across_small_widths() {
    let mut cases = 0usize;
    for width in 1..=3u32 {
        for (name, prop) in [
            ("add_comm", add_comm(width)),
            ("sub_add", sub_add(width)),
            ("add_zero", add_zero(width)),
        ] {
            let cert = build_cert(&prop);
            let text = cert.render();
            let parsed = Certificate::parse(&text).unwrap_or_else(|e| {
                panic!("{name} bv{width}: generated certificate does not round-trip: {e}")
            });
            assert_eq!(&parsed, &cert);

            let signed = check(&parsed)
                .unwrap_or_else(|e| panic!("{name} bv{width}: signed checker refused: {e}"));
            assert_eq!(signed.render(), prop.render());
            reference::accepts(&parsed).unwrap_or_else(|e| {
                panic!("{name} bv{width}: oracle refused: {} {}", e.code, e.detail)
            });
            assert_eq!(verdicts(&text), (true, true));
            cases += 1;
        }
    }
    // Association needs three variables; width 2 keeps the generated refutation small.
    let prop = add_assoc(2);
    let cert = build_cert(&prop);
    assert!(check(&Certificate::parse(&cert.render()).unwrap()).is_ok());
    reference::accepts(&Certificate::parse(&cert.render()).unwrap()).unwrap();
    cases += 1;

    assert_eq!(cases, 10, "differential acceptance case count changed");
    println!("differential acceptance cases: {cases}");
}

// ---------------------------------------------------------------------------
// 2. Differential rejection of the mutation families
// ---------------------------------------------------------------------------

#[test]
fn differential_mutations_agree() {
    let prop = add_comm(3);
    let cert = build_cert(&prop);
    let base = cert.render();
    let mut cases = 0usize;

    // --- certificate container mutations -----------------------------------
    let duplicate_blob = {
        let mut s = String::new();
        for line in base.lines() {
            s.push_str(line);
            s.push('\n');
            if line.starts_with("blob lrat ") {
                s.push_str(line);
                s.push('\n');
            }
        }
        s
    };
    assert_agree(&duplicate_blob, false, "duplicate blob");
    cases += 1;

    let duplicate_step = {
        let mut s = String::new();
        for line in base.lines() {
            s.push_str(line);
            s.push('\n');
            if line.starts_with("step ") {
                s.push_str(line);
                s.push('\n');
            }
        }
        s
    };
    assert_agree(&duplicate_step, false, "duplicate step");
    cases += 1;

    let truncated = &base[..base.len() - 10];
    assert_agree(truncated, false, "truncated certificate");
    cases += 1;

    assert_agree(&format!("{base}trailing\n"), false, "trailing bytes");
    cases += 1;

    let unknown_rule = base.replacen(" bitblast ", " frobnicate ", 1);
    assert_agree(&unknown_rule, false, "unknown rule");
    cases += 1;

    let dangling = base.replacen(" bitblast ", " sym 999999 ", 1);
    assert_agree(&dangling, false, "dangling premise");
    cases += 1;

    // --- CNF payload -------------------------------------------------------
    let flipped_cnf = rebuild_with_blob(&cert, BlobKind::Cnf, |bytes| {
        let s = std::str::from_utf8(bytes).unwrap();
        let mut lines: Vec<String> = s.lines().map(|l| l.to_string()).collect();
        lines[1] = lines[1]
            .split(' ')
            .map(|t| match t.parse::<i32>() {
                Ok(0) => "0".to_string(),
                Ok(v) => (-v).to_string(),
                Err(_) => t.to_string(),
            })
            .collect::<Vec<_>>()
            .join(" ");
        let mut out = String::new();
        for line in lines {
            out.push_str(&line);
            out.push('\n');
        }
        out.into_bytes()
    });
    assert_agree(&flipped_cnf, false, "flipped CNF literal");
    cases += 1;

    let non_canonical_cnf = rebuild_with_blob(&cert, BlobKind::Cnf, |bytes| {
        let mut v = bytes.to_vec();
        v.push(b' ');
        v
    });
    assert_agree(&non_canonical_cnf, false, "non-canonical CNF bytes");
    cases += 1;

    // --- LRAT payload ------------------------------------------------------
    let hint_to_unknown = rewrite_lrat(&cert, |s| {
        let mut out = String::new();
        let mut done = false;
        for line in s.lines() {
            if !done && !line.starts_with('d') {
                let mut tokens: Vec<String> = line.split(' ').map(|t| t.to_string()).collect();
                if let Some(z) = tokens.iter().position(|t| t == "0") {
                    if z + 1 < tokens.len() {
                        tokens[z + 1] = "999999".to_string();
                        out.push_str(&tokens.join(" "));
                        out.push('\n');
                        done = true;
                        continue;
                    }
                }
            }
            out.push_str(line);
            out.push('\n');
        }
        out
    });
    assert_agree(&hint_to_unknown, false, "hint names an unknown clause");
    cases += 1;

    let negative_hint = rewrite_lrat(&cert, |s| {
        let mut out = String::new();
        let mut done = false;
        for line in s.lines() {
            if !done && !line.starts_with('d') {
                let mut tokens: Vec<String> = line.split(' ').map(|t| t.to_string()).collect();
                if let Some(z) = tokens.iter().position(|t| t == "0") {
                    if z + 1 < tokens.len() && tokens[z + 1] != "0" {
                        tokens[z + 1] = format!("-{}", tokens[z + 1]);
                        out.push_str(&tokens.join(" "));
                        out.push('\n');
                        done = true;
                        continue;
                    }
                }
            }
            out.push_str(line);
            out.push('\n');
        }
        out
    });
    assert_agree(&negative_hint, false, "negative (RAT) hint");
    cases += 1;

    let trailing_garbage = rewrite_lrat(&cert, |s| format!("{s}1 0 0\n"));
    assert_agree(&trailing_garbage, false, "record after empty clause");
    cases += 1;

    let unknown_literal = rewrite_lrat(&cert, |s| {
        let mut out = String::new();
        let mut done = false;
        for line in s.lines() {
            if !done && !line.starts_with('d') {
                let mut tokens: Vec<String> = line.split(' ').map(|t| t.to_string()).collect();
                if tokens.len() > 1 && tokens[1] != "0" {
                    tokens[1] = "200".to_string();
                    out.push_str(&tokens.join(" "));
                    out.push('\n');
                    done = true;
                    continue;
                }
            }
            out.push_str(line);
            out.push('\n');
        }
        out
    });
    assert_agree(&unknown_literal, false, "literal out of range");
    cases += 1;

    let no_empty = rewrite_lrat(&cert, |s| {
        let lines: Vec<&str> = s.lines().collect();
        let mut out = String::new();
        for line in &lines[..lines.len() - 1] {
            out.push_str(line);
            out.push('\n');
        }
        out
    });
    assert_agree(&no_empty, false, "missing empty clause");
    cases += 1;

    let deleted_needed = rewrite_lrat(&cert, |s| {
        // Delete original clause 1 first; the proof's hints still name it.
        format!("d 1 0\n{s}")
    });
    assert_agree(&deleted_needed, false, "deletion of a needed clause");
    cases += 1;

    let truncated_lrat = rewrite_lrat(&cert, |s| s.trim_end_matches('\n').to_string());
    assert_agree(&truncated_lrat, false, "LRAT without final newline");
    cases += 1;

    let tail_dropped = rewrite_lrat(&cert, |s| {
        let lines: Vec<&str> = s.lines().collect();
        let mut out = String::new();
        for line in &lines[1..] {
            out.push_str(line);
            out.push('\n');
        }
        out
    });
    assert_agree(&tail_dropped, false, "first derivation line erased");
    cases += 1;

    // --- rule / statement mismatches --------------------------------------
    let x = word_var(3, "x");
    let one = Term::constant(3, 1).unwrap();
    let false_identity = eq_of(Term::add(x.clone(), one).unwrap(), x);
    let transplanted = with_conclusion(&cert, &false_identity);
    assert_agree(
        &transplanted,
        false,
        "proof transplanted to a false identity",
    );
    cases += 1;

    let y = word_var(3, "y");
    let bogus_sym = {
        let refl = Step::refl(0, eq_of(word_var(3, "x"), word_var(3, "x")));
        let sym = Step::sym(1, 0, eq_of(y, word_var(3, "x")));
        Certificate::new(1, vec![refl, sym], vec![])
            .expect("structurally valid")
            .render()
    };
    assert_agree(&bogus_sym, false, "sym conclusion does not exchange sides");
    cases += 1;

    // --- benign mutation: the two must agree on acceptance too ------------
    let reordered = rewrite_lrat(&cert, |s| {
        let mut out = String::new();
        for line in s.lines() {
            if line.starts_with('d') {
                out.push_str(line);
                out.push('\n');
                continue;
            }
            let tokens: Vec<&str> = line.split(' ').collect();
            if let Some(z) = tokens.iter().position(|t| *t == "0") {
                // Reverse the literals before the clause terminator only.
                if z > 1 {
                    let mut lits: Vec<&str> = tokens[1..z].to_vec();
                    lits.reverse();
                    let mut rebuilt = vec![tokens[0]];
                    rebuilt.extend(lits);
                    rebuilt.push("0");
                    rebuilt.extend(tokens[z + 1..].iter().copied());
                    out.push_str(&rebuilt.join(" "));
                    out.push('\n');
                    continue;
                }
            }
            out.push_str(line);
            out.push('\n');
        }
        out
    });
    assert_agree(&reordered, true, "benign literal reordering");
    cases += 1;

    println!("differential mutation cases: {cases}");
    assert_eq!(cases, 19, "mutation case count changed");
}

// ---------------------------------------------------------------------------
// 3. Parser conformance and canonicality
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

fn random_width(rng: &mut Rng) -> u32 {
    match rng.below(4) {
        0 => 1,
        1 => 8,
        2 => 16,
        _ => 64,
    }
}

fn random_name(rng: &mut Rng) -> String {
    const NAMES: [&str; 6] = ["x", "y", "z", "_a", "b1", "carry"];
    NAMES[rng.below(NAMES.len() as u64) as usize].to_string()
}

fn random_word(rng: &mut Rng, depth: u32, width: u32) -> Term {
    match depth {
        0 => {
            if rng.below(2) == 0 {
                let name = random_name(rng);
                Term::var(width, &name).expect("valid variable")
            } else {
                let value = rng.below(1u64 << width.min(20));
                Term::constant(width, value).expect("literal fits")
            }
        }
        _ => match rng.below(3) {
            0 => {
                let name = random_name(rng);
                Term::var(width, &name).expect("valid variable")
            }
            1 => {
                let l = random_word(rng, depth - 1, width);
                let r = random_word(rng, depth - 1, width);
                Term::add(l, r).expect("same width")
            }
            _ => {
                let l = random_word(rng, depth - 1, width);
                let r = random_word(rng, depth - 1, width);
                Term::sub(l, r).expect("same width")
            }
        },
    }
}

fn random_bool(rng: &mut Rng, depth: u32) -> Term {
    if depth == 0 {
        return Term::Bool(rng.below(2) == 1);
    }
    match rng.below(4) {
        0 => Term::Bool(rng.below(2) == 1),
        1 => {
            let inner = random_bool(rng, depth - 1);
            Term::not(inner).expect("Boolean")
        }
        _ => {
            let width = random_width(rng);
            let l = random_word(rng, depth - 1, width);
            let r = random_word(rng, depth - 1, width);
            Term::eq(l, r).expect("same width")
        }
    }
}

#[test]
fn parser_round_trip_is_canonical() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let mut terms = 0usize;
    for _ in 0..500 {
        let term = random_bool(&mut rng, 4);
        let rendered = term.render();
        let parsed = Term::parse(&rendered)
            .unwrap_or_else(|e| panic!("term did not re-parse: {rendered:?}: {e}"));
        assert_eq!(parsed, term);
        assert_eq!(parsed.render(), rendered);
        terms += 1;
    }
    println!("term round-trip cases: {terms}");

    let mut certs = 0usize;
    for width in 1..=3 {
        for prop in [add_comm(width), sub_add(width)] {
            let cert = build_cert(&prop);
            let rendered = cert.render();
            let parsed = Certificate::parse(&rendered).expect("certificate re-parses");
            assert_eq!(parsed, cert);
            assert_eq!(parsed.render(), rendered);
            certs += 1;
        }
    }
    println!("certificate round-trip cases: {certs}");
}

#[test]
fn malformed_inputs_are_rejected() {
    let bad_terms = [
        "",
        " ",
        "true ",
        "(var bv8 x",
        "(var bv8 x) ",
        "(var bv8 9x)",
        "(var bv08 x)",
        "(var bv0 x)",
        "(var bv65 x)",
        "(var bv8 )",
        "(const bv8 256)",
        "(const bv8 007)",
        "(eq bv8 true false)",
        "(eq bv4 (var bv8 x) (var bv8 y))",
        "(add bv8 (var bv4 x) (var bv8 y))",
        "(not (var bv8 x))",
        "(xor bv8 (var bv8 x) (var bv8 y))",
        "(add bv8 (var bv8 x) (var bv8 y)) extra",
    ];
    for text in bad_terms {
        assert!(
            Term::parse(text).is_err(),
            "malformed term was accepted: {text:?}"
        );
    }

    let cert = build_cert(&add_comm(2));
    let rendered = cert.render();
    let bogus = [
        String::new(),
        "not a certificate\n".to_string(),
        rendered.replacen("tekmerion-native-cert v1", "tekmerion-native-cert v2", 1),
        rendered.replace("step 0 bitblast", "step 0 bitblast "),
        rendered.to_ascii_uppercase(),
        rendered.trim_end_matches('\n').to_string(),
        format!("{rendered}end\n"),
        rendered.replacen("theorem 0", "theorem 00", 1),
    ];
    for text in bogus {
        assert!(
            Certificate::parse(&text).is_err(),
            "malformed certificate was accepted: {:?}",
            &text[..text.len().min(40)]
        );
    }

    // Canonical DIMACS is strict too.
    for bytes in [
        b"p cnf 2 3\n1 2 0\n-1 2 0\n-2 0\n".as_slice(),
        b"p cnf 2 3\n1 2 0\n-1 2 0\n-2 0".as_slice(),
        b"p cnf 2 2\n1 2 0\n-1 2 0\n-2 0\n".as_slice(),
        b"p cnf 2 3\n1 2 0\n-1 2 0\n-2 0\n\n".as_slice(),
    ] {
        if bytes == b"p cnf 2 3\n1 2 0\n-1 2 0\n-2 0\n".as_slice() {
            assert!(Cnf::parse_dimacs(bytes).is_ok());
        } else {
            assert!(
                Cnf::parse_dimacs(bytes).is_err(),
                "non-canonical DIMACS accepted: {:?}",
                String::from_utf8_lossy(bytes)
            );
        }
    }
}

#[test]
fn fuzz_never_panics() {
    let mut rng = Rng(0x1234_5678_9ABC_DEF0);
    let small_cnf = Cnf::parse_dimacs(b"p cnf 2 3\n1 2 0\n-1 2 0\n-2 0\n").expect("small CNF");
    let fixture = Certificate::parse(&build_cert(&add_comm(2)).render()).unwrap();
    let fixture_text = fixture.render();
    let prop_for_lrat = add_comm(2);

    for i in 0..2000 {
        let len = rng.below(257) as usize;
        let mut bytes = vec![0u8; len];
        for b in &mut bytes {
            *b = rng.below(256) as u8;
        }
        let lossy = String::from_utf8_lossy(&bytes);

        // Must not panic. Any verdict is acceptable.
        let _ = Certificate::parse(&lossy);
        let _ = Term::parse(&lossy);
        let _ = Cnf::parse_dimacs(&bytes);
        let _ = check_lrat(&small_cnf, &lossy);

        // The oracle's LRAT parser must also survive arbitrary bytes.
        if i % 20 == 0 {
            let cnf = bitblast(&prop_for_lrat).unwrap();
            let mut lrat = bytes.clone();
            while lrat.len() > 64 {
                lrat.pop();
            }
            let cnf_blob = Blob::new(BlobKind::Cnf, cnf.to_dimacs().into_bytes());
            let lrat_blob = Blob::new(BlobKind::Lrat, lrat);
            let step = Step::bitblast(
                0,
                cnf_blob.digest(),
                lrat_blob.digest(),
                prop_for_lrat.clone(),
            );
            let cert = Certificate::new(0, vec![step], vec![cnf_blob, lrat_blob]).unwrap();
            let _ = reference::accepts(&cert);
        }
    }

    // Byte-level mutations of a valid certificate must not panic either.
    let fixture_bytes = fixture_text.as_bytes();
    for _ in 0..1000 {
        let mut bytes = fixture_bytes.to_vec();
        if bytes.is_empty() {
            continue;
        }
        let flips = 1 + rng.below(4) as usize;
        for _ in 0..flips {
            let i = rng.below(bytes.len() as u64) as usize;
            bytes[i] ^= 1 << (rng.below(8) as u8);
        }
        let lossy = String::from_utf8_lossy(&bytes);
        if let Ok(cert) = Certificate::parse(&lossy) {
            let _ = check(&cert);
            let _ = reference::accepts(&cert);
        }
    }
}

// ---------------------------------------------------------------------------
// 4. Exhaustive refusal of a false identity
// ---------------------------------------------------------------------------

#[test]
fn false_identity_is_rejected_by_both() {
    // Width 2 has four assignments; the exhaustive oracle visits all of them.
    let x = word_var(2, "x");
    let one = Term::constant(2, 1).unwrap();
    let false_identity = eq_of(Term::add(x.clone(), one).unwrap(), x.clone());

    let mut false_count = 0;
    for value in 0u64..4 {
        let mut env = BTreeMap::new();
        env.insert("x".to_string(), value);
        assert_eq!(false_identity.eval(&env).unwrap(), Value::Bool(false));
        false_count += 1;
    }
    assert_eq!(
        false_count, 4,
        "all four width-2 assignments are counterexamples"
    );

    // The oracle's independent evaluator must find a counterexample too.
    let witness = reference::falsifying_assignment(&false_identity)
        .expect("semantic check runs")
        .expect("a false identity has a witness");
    assert!(witness.contains_key("x"));

    // The canonical bit-blast must remain satisfiable (no refutation can exist).
    assert!(
        cnf_satisfiable(&bitblast(&false_identity).unwrap()),
        "x + 1 = x must not bit-blast to UNSAT"
    );

    // Forge an empty-clause refutation: both checkers must refuse it.
    let cnf = bitblast(&false_identity).unwrap();
    let forged_lrat = format!("{} 0 0\n", cnf.clauses().len() + 1);
    let cnf_blob = Blob::new(BlobKind::Cnf, cnf.to_dimacs().into_bytes());
    let lrat_blob = Blob::new(BlobKind::Lrat, forged_lrat.into_bytes());
    let step = Step::bitblast(
        0,
        cnf_blob.digest(),
        lrat_blob.digest(),
        false_identity.clone(),
    );
    let forged = Certificate::new(0, vec![step], vec![cnf_blob, lrat_blob])
        .expect("structurally valid")
        .render();
    assert_agree(&forged, false, "forged empty-clause refutation");

    // Transplant a genuine refutation for a true identity. Both must refuse,
    // because the attached CNF is not the bit-blast of the claimed statement.
    let valid = build_cert(&add_comm(2));
    let transplanted = with_conclusion(&valid, &false_identity);
    assert_agree(&transplanted, false, "genuine proof transplanted");

    // Also swap the CNF only: the canonical CNF of the false identity with the
    // genuine LRAT. Both must reject the non-RUP payload.
    let swapped = {
        let target = valid
            .blobs()
            .iter()
            .find(|b| b.kind() == BlobKind::Cnf)
            .unwrap();
        let old = target.digest();
        let new_blob = Blob::new(BlobKind::Cnf, cnf.to_dimacs().into_bytes());
        let new_digest = new_blob.digest();
        let steps: Vec<Step> = valid
            .steps()
            .iter()
            .map(|s| match s.rule() {
                Rule::Bitblast { cnf, lrat } => Step::bitblast(
                    s.id(),
                    if *cnf == old { new_digest } else { *cnf },
                    *lrat,
                    false_identity.clone(),
                ),
                _ => s.clone(),
            })
            .collect();
        let blobs: Vec<Blob> = valid
            .blobs()
            .iter()
            .filter(|b| b.digest() != old)
            .cloned()
            .chain(std::iter::once(new_blob))
            .collect();
        Certificate::new(0, steps, blobs)
            .expect("structurally valid")
            .render()
    };
    let (signed, oracle) = verdicts(&swapped);
    assert!(!signed && !oracle, "swapped CNF must be refused by both");
}

fn cnf_satisfiable(cnf: &Cnf) -> bool {
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
            let var = lit.unsigned_abs() as usize;
            match assign[var] {
                0 => {
                    target = Some(var);
                    break;
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
        if let Some(var) = target {
            let mut trial = assign.to_vec();
            trial[var] = 1;
            if dpll(clauses, &mut trial) {
                assign.copy_from_slice(&trial);
                return true;
            }
            assign[var] = -1;
            return dpll(clauses, assign);
        }
    }
    true
}

// ---------------------------------------------------------------------------
// 5. The oracle is not on the trusted path
// ---------------------------------------------------------------------------

#[test]
fn reference_oracle_is_not_on_the_trusted_path() {
    // The trusted modules must not mention the oracle at all. `mod.rs` declares
    // the module but must not call it.
    let trusted: [(&str, &str); 5] = [
        ("native/check.rs", include_str!("../src/native/check.rs")),
        ("native/cert.rs", include_str!("../src/native/cert.rs")),
        ("native/lrat.rs", include_str!("../src/native/lrat.rs")),
        (
            "native/bitblast.rs",
            include_str!("../src/native/bitblast.rs"),
        ),
        ("native/logic.rs", include_str!("../src/native/logic.rs")),
    ];
    for (name, src) in trusted {
        for needle in [
            "reference::",
            "pub mod reference",
            "use reference",
            "use crate::native::reference",
        ] {
            assert!(
                !src.contains(needle),
                "trusted module {name} depends on the test oracle ({needle})"
            );
        }
    }

    let mod_src = include_str!("../src/native/mod.rs");
    for needle in [
        "reference::",
        "use reference",
        "use crate::native::reference",
    ] {
        assert!(
            !mod_src.contains(needle),
            "native/mod.rs calls the test oracle ({needle})"
        );
    }
    assert!(mod_src.contains("pub mod reference;"));

    // The only module that reaches the oracle is this test file itself.
    let check_src = include_str!("../src/native/check.rs");
    assert!(!check_src.contains("Reference"));
    assert!(!check_src.contains("oracle"));
}
