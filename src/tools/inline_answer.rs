//! Build a single statically-typeable answer by inlining the sources the tactic
//! needs as nested `let ... in` bindings.
//!
//! Port of `research/receipts/bridge/make_inlined_answer.py`. The benchmark's
//! syntax gate type-checks an answer with `ocamlc -i`, so names introduced by
//! `loadt` are invisible. This inlines the needed OCaml sources in a fixed order
//! and puts the tactic last. **No proof text is changed.**
//!
//! The four tactic strings are reproduced character for character from the
//! Python original, including the trailing `p_256` / `p_384` lines. Two of them
//! are marked DEVELOPMENT_ONLY by the original and remain so here:
//!
//! * `sub` — the simulation, plus `FLAG_FROM_CARRY_LT` / `DECARRY_RULE` /
//!   `BOUNDER_TAC`, transcribed from the human proof of `bignum_sub_p256`.
//! * `add-p384` — the carry telescope, compare-flag meaning and CSETM mask are
//!   now derived by `BRIDGE_CARRY_CONSERVE_TAC` / `BRIDGE_CMP_FLAG_TAC` /
//!   `BRIDGE_MASK_SELECT_TAC`; the simulation grouping, the bignum expansion
//!   pass and the post bitval rewrites are still DEVELOPMENT_ONLY. See
//!   `research/receipts/d9-derived-flags.md`.
//!
//! Output is byte-identical to the Python original for all four modes.
//! **Not on the verdict path.**

use std::fs;
use std::path::PathBuf;

use crate::tools::instrument::split_chunks;

const USAGE: &str = "\
usage: inline_answer <out-answer.txt> [add-ref|sub|neg|add-p384]

  Inlines the Cadical DIMACS/CNF/LRUDP readers, tools/csat_lib.ml and
  tools/bridge.ml as nested `let ... in` bindings, then appends the selected
  tactic. Sources are resolved under the repository this binary was built in.
  Default mode is `add-ref`.
";

/// Sources inlined, in order. Relative to the repository root.
const SOURCES: [&str; 5] = [
    ".bench/hol-light/Cadical/dimacs.ml",
    ".bench/hol-light/Cadical/cnf.ml",
    ".bench/hol-light/Cadical/ldrup.ml",
    "tools/csat_lib.ml",
    "tools/bridge.ml",
];

const TACTIC_ADD_REF: &str = r#"MAP_EVERY X_GEN_TAC
 [`z:int64`; `x:int64`; `y:int64`; `m:num`; `n:num`; `pc:num`] THEN
REWRITE_TAC[C_ARGUMENTS; C_RETURN; SOME_FLAGS; NONOVERLAPPING_CLAUSES] THEN
DISCH_THEN(REPEAT_TCL CONJUNCTS_THEN ASSUME_TAC) THEN
REWRITE_TAC[BIGNUM_FROM_MEMORY_BYTES] THEN ENSURES_INIT_TAC "s0" THEN
BIGNUM_DIGITIZE_TAC "m_" `read (memory :> bytes (x,8 * 4)) s0` THEN
BIGNUM_DIGITIZE_TAC "n_" `read (memory :> bytes (y,8 * 4)) s0` THEN
ARM_ACCSTEPS_TAC BIGNUM_ADD_P256_EXEC [3;4;7;8;10;12;13;15;16] (1--22) THEN
ENSURES_FINAL_STATE_TAC THEN ASM_REWRITE_TAC[] THEN STRIP_TAC THEN
RULE_ASSUM_TAC(REWRITE_RULE [REAL_BITVAL_NOT; ADD_CLAUSES; VAL_WORD_BITVAL]) THEN
CONV_TAC SYM_CONV THEN CONV_TAC(RAND_CONV BIGNUM_LEXPAND_CONV) THEN
ASM_REWRITE_TAC[] THEN DISCARD_STATE_TAC "s22" THEN
BRIDGE_ADD_REF_TAC (CONV_TAC (ONCE_DEPTH_CONV BIGNUM_OF_WORDLIST_CONV)) p_256"#;

const TACTIC_SUB: &str = r#"MAP_EVERY X_GEN_TAC
 [`z:int64`; `x:int64`; `y:int64`; `m:num`; `n:num`; `pc:num`] THEN
REWRITE_TAC[C_ARGUMENTS; C_RETURN; SOME_FLAGS; NONOVERLAPPING_CLAUSES] THEN
DISCH_THEN(REPEAT_TCL CONJUNCTS_THEN ASSUME_TAC) THEN
REWRITE_TAC[BIGNUM_FROM_MEMORY_BYTES] THEN ENSURES_INIT_TAC "s0" THEN
BIGNUM_DIGITIZE_TAC "m_" `read (memory :> bytes (x,8 * 4)) s0` THEN
BIGNUM_DIGITIZE_TAC "n_" `read (memory :> bytes (y,8 * 4)) s0` THEN
ARM_ACCSTEPS_TAC BIGNUM_SUB_P256_EXEC [3;4;7;8;10;12;13;15] (1--8) THEN
SUBGOAL_THEN `carry_s8 <=> m < n` SUBST_ALL_TAC THENL
 [MATCH_MP_TAC FLAG_FROM_CARRY_LT THEN EXISTS_TAC `256` THEN
  MAP_EVERY EXPAND_TAC ["m"; "n"] THEN REWRITE_TAC[GSYM REAL_OF_NUM_ADD] THEN
  REWRITE_TAC[GSYM REAL_OF_NUM_MUL; GSYM REAL_OF_NUM_POW] THEN
  ACCUMULATOR_ASSUM_LIST(MP_TAC o end_itlist CONJ o DECARRY_RULE) THEN
  DISCH_THEN(fun th -> REWRITE_TAC[th]) THEN BOUNDER_TAC[];
  ALL_TAC] THEN
ARM_STEPS_TAC BIGNUM_SUB_P256_EXEC [9] THEN
RULE_ASSUM_TAC(REWRITE_RULE[WORD_UNMASK_64; NOT_LE]) THEN
ARM_ACCSTEPS_TAC BIGNUM_SUB_P256_EXEC [3;4;7;8;10;12;13;15] (10--17) THEN
ENSURES_FINAL_STATE_TAC THEN ASM_REWRITE_TAC[] THEN STRIP_TAC THEN
CONV_TAC(LAND_CONV(RAND_CONV BIGNUM_EXPAND_CONV)) THEN
ASM_REWRITE_TAC[] THEN DISCARD_STATE_TAC "s17" THEN
BRIDGE_SUB_TAC (CONV_TAC (ONCE_DEPTH_CONV BIGNUM_OF_WORDLIST_CONV)) p_256"#;

const TACTIC_NEG: &str = r#"MAP_EVERY X_GEN_TAC [`z:int64`; `x:int64`; `n:num`; `pc:num`] THEN
REWRITE_TAC[C_ARGUMENTS; C_RETURN; SOME_FLAGS; NONOVERLAPPING_CLAUSES] THEN
DISCH_THEN(REPEAT_TCL CONJUNCTS_THEN ASSUME_TAC) THEN
REWRITE_TAC[BIGNUM_FROM_MEMORY_BYTES] THEN ENSURES_INIT_TAC "s0" THEN
BIGNUM_DIGITIZE_TAC "n_" `read (memory :> bytes (x,8 * 4)) s0` THEN
ARM_ACCSTEPS_TAC BIGNUM_NEG_P256_EXEC [8;10;11;13] (1--15) THEN
ENSURES_FINAL_STATE_TAC THEN ASM_REWRITE_TAC[] THEN STRIP_TAC THEN
CONV_TAC(LAND_CONV BIGNUM_EXPAND_CONV) THEN
ASM_REWRITE_TAC[] THEN DISCARD_STATE_TAC "s15" THEN
REWRITE_TAC[GSYM REAL_OF_NUM_CLAUSES] THEN
BRIDGE_NEG_TAC (CONV_TAC (ONCE_DEPTH_CONV BIGNUM_OF_WORDLIST_CONV)) p_256"#;

const TACTIC_ADD_P384: &str = r#"MAP_EVERY X_GEN_TAC
 [`z:int64`; `x:int64`; `y:int64`; `m:num`; `n:num`; `pc:num`] THEN
REWRITE_TAC[C_ARGUMENTS; C_RETURN; SOME_FLAGS; NONOVERLAPPING_CLAUSES] THEN
DISCH_THEN(REPEAT_TCL CONJUNCTS_THEN ASSUME_TAC) THEN
REWRITE_TAC[BIGNUM_FROM_MEMORY_BYTES] THEN ENSURES_INIT_TAC "s0" THEN
BIGNUM_DIGITIZE_TAC "m_" `read (memory :> bytes (x,8 * 6)) s0` THEN
BIGNUM_DIGITIZE_TAC "n_" `read (memory :> bytes (y,8 * 6)) s0` THEN
ARM_ACCSTEPS_TAC BIGNUM_ADD_P384_EXEC (1--13) (1--13) THEN
BRIDGE_CARRY_CONSERVE_TAC THEN
ARM_ACCSTEPS_TAC BIGNUM_ADD_P384_EXEC (14--22) (14--22) THEN
BRIDGE_CMP_FLAG_TAC p_384 THEN
ARM_STEPS_TAC BIGNUM_ADD_P384_EXEC [23;24] THEN
BRIDGE_MASK_SELECT_TAC THEN
ARM_ACCSTEPS_TAC BIGNUM_ADD_P384_EXEC (25--38) (25--38) THEN
ENSURES_FINAL_STATE_TAC THEN ASM_REWRITE_TAC[] THEN STRIP_TAC THEN
CONV_TAC(LAND_CONV BIGNUM_EXPAND_CONV) THEN
ASM_REWRITE_TAC[] THEN DISCARD_STATE_TAC "s38" THEN
RULE_ASSUM_TAC(REWRITE_RULE [REAL_BITVAL_NOT; ADD_CLAUSES; VAL_WORD_BITVAL]) THEN
CONV_TAC SYM_CONV THEN
RULE_ASSUM_TAC (CONV_RULE (ONCE_DEPTH_CONV BIGNUM_OF_WORDLIST_CONV)) THEN
CONV_TAC (ONCE_DEPTH_CONV BIGNUM_OF_WORDLIST_CONV) THEN
BRIDGE_ADD_REF_TAC (CONV_TAC (ONCE_DEPTH_CONV BIGNUM_OF_WORDLIST_CONV)) p_384"#;

fn tactic(mode: &str) -> Option<&'static str> {
    Some(match mode {
        "add-ref" => TACTIC_ADD_REF,
        "sub" => TACTIC_SUB,
        "neg" => TACTIC_NEG,
        "add-p384" => TACTIC_ADD_P384,
        _ => return None,
    })
}

fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// `^\s*(#|needs\b|loadt\b)` on an already-trimmed phrase.
fn is_skipped(phrase: &str) -> bool {
    if phrase.starts_with('#') {
        return true;
    }
    for keyword in ["needs", "loadt"] {
        if let Some(rest) = phrase.strip_prefix(keyword) {
            match rest.chars().next() {
                None => return true,
                Some(c) if !is_word_char(c) => return true,
                _ => {}
            }
        }
    }
    false
}

/// Non-blank, non-skipped top-level phrases of one source, trimmed.
pub fn phrases_from(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    for chunk in split_chunks(src) {
        let phrase = chunk.trim();
        if phrase.is_empty() || is_skipped(phrase) {
            continue;
        }
        out.push(phrase.to_string());
    }
    out
}

/// Render the answer body for the given phrases and mode.
pub fn render(parts: &[String], mode: &str) -> Result<String, String> {
    let tactic = tactic(mode).ok_or_else(|| {
        format!("unknown mode {mode:?}; expected one of add-ref, sub, neg, add-p384")
    })?;

    let mut body = String::from("(\n");
    for phrase in parts {
        body.push_str(phrase);
        body.push_str("\nin\n");
    }
    body.push_str("(fun g ->\n  try\n    (");
    body.push_str(tactic);
    body.push_str(
        ") g\n  with e ->\n    Printf.printf \"ANSWER_EXCEPTION: %s\\n%!\" (Printexc.to_string e);\n    raise e)\n",
    );
    body.push_str(")\n");
    Ok(body)
}

/// Collect every phrase from the fixed source order.
pub fn collect(repo: &std::path::Path) -> Result<Vec<String>, String> {
    let mut parts = Vec::new();
    for rel in SOURCES {
        let path = repo.join(rel);
        let src = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        parts.extend(phrases_from(&src));
    }
    Ok(parts)
}

/// CLI entry point. `args` excludes the program name.
pub fn run(args: &[String]) -> i32 {
    if args.len() == 1 && matches!(args[0].as_str(), "-h" | "--help") {
        print!("{USAGE}");
        return 0;
    }
    if args.is_empty() || args.len() > 2 {
        eprint!("{USAGE}");
        return 2;
    }
    let out = &args[0];
    let mode = args.get(1).map(String::as_str).unwrap_or("add-ref");
    if tactic(mode).is_none() {
        eprintln!("ERROR unknown mode {mode:?}; expected one of add-ref, sub, neg, add-p384");
        return 2;
    }
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let parts = match collect(&repo) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("ERROR {e}");
            return 2;
        }
    };
    let body = match render(&parts, mode) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("ERROR {e}");
            return 2;
        }
    };
    if let Err(e) = fs::write(out, body.as_bytes()) {
        eprintln!("ERROR {out}: {e}");
        return 2;
    }
    println!(
        "wrote {out} bytes={} bindings={}",
        body.chars().count(),
        parts.len()
    );
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phrases_drop_blank_and_directive_chunks() {
        let src = "#use \"x\";;\nneeds \"base.ml\";;\nloadt \"y.ml\";;\n\
                   let a = 1;;\n\nlet b = 2;;\nneedsome = 3;;\n";
        let got = phrases_from(src);
        assert_eq!(
            got,
            vec![
                "let a = 1".to_string(),
                "let b = 2".to_string(),
                "needsome = 3".to_string()
            ]
        );
    }

    #[test]
    fn render_wraps_each_phrase_and_appends_the_tactic() {
        let parts = vec!["let a = 1".to_string(), "let b = 2".to_string()];
        let body = render(&parts, "neg").unwrap();
        assert!(body.starts_with("(\nlet a = 1\nin\nlet b = 2\nin\n(fun g ->\n"));
        assert!(body.contains("BRIDGE_NEG_TAC"));
        assert!(body.ends_with("    raise e)\n)\n"));
    }

    #[test]
    fn unknown_mode_is_refused() {
        assert!(render(&[], "nope").is_err());
    }
}
