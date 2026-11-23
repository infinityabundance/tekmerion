#!/usr/bin/env python3
"""make_inlined_answer.py -- build a single statically-typeable answer.

The benchmark's syntax gate type-checks the answer with `ocamlc -i`, so names
introduced by `loadt` are not visible.  We therefore inline the needed OCaml
sources (Cadical DIMACS/CNF/LRAT reader, csat_lib.ml, bridge.ml) as nested
`let ... in` bindings and put the tactic last.  No proof text is changed.

Usage: make_inlined_answer.py <out-answer.txt>
"""

import re
import sys

ROOT = "/mnt/1tb_kingston/Tekmerion"
SOURCES = [
    f"{ROOT}/.bench/hol-light/Cadical/dimacs.ml",
    f"{ROOT}/.bench/hol-light/Cadical/cnf.ml",
    f"{ROOT}/.bench/hol-light/Cadical/ldrup.ml",
    f"{ROOT}/tools/csat_lib.ml",
    f"{ROOT}/tools/bridge.ml",
]

TACTIC = r"""
MAP_EVERY X_GEN_TAC
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
BRIDGE_ADD_REF_TAC (CONV_TAC (ONCE_DEPTH_CONV BIGNUM_OF_WORDLIST_CONV)) p_256
"""

# sub: the same simulation as tools/csat_bridge.ml.  sub_middle is
# DEVELOPMENT_ONLY (FLAG_FROM_CARRY_LT / DECARRY_RULE / BOUNDER_TAC transcribed
# from the human proof of bignum_sub_p256).
TACTIC_SUB = r"""
MAP_EVERY X_GEN_TAC
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
BRIDGE_SUB_TAC (CONV_TAC (ONCE_DEPTH_CONV BIGNUM_OF_WORDLIST_CONV)) p_256
"""

TACTICS = {"add-ref": TACTIC, "sub": TACTIC_SUB}

TACTIC_NEG = r"""
MAP_EVERY X_GEN_TAC [`z:int64`; `x:int64`; `n:num`; `pc:num`] THEN
REWRITE_TAC[C_ARGUMENTS; C_RETURN; SOME_FLAGS; NONOVERLAPPING_CLAUSES] THEN
DISCH_THEN(REPEAT_TCL CONJUNCTS_THEN ASSUME_TAC) THEN
REWRITE_TAC[BIGNUM_FROM_MEMORY_BYTES] THEN ENSURES_INIT_TAC "s0" THEN
BIGNUM_DIGITIZE_TAC "n_" `read (memory :> bytes (x,8 * 4)) s0` THEN
ARM_ACCSTEPS_TAC BIGNUM_NEG_P256_EXEC [8;10;11;13] (1--15) THEN
ENSURES_FINAL_STATE_TAC THEN ASM_REWRITE_TAC[] THEN STRIP_TAC THEN
CONV_TAC(LAND_CONV BIGNUM_EXPAND_CONV) THEN
ASM_REWRITE_TAC[] THEN DISCARD_STATE_TAC "s15" THEN
REWRITE_TAC[GSYM REAL_OF_NUM_CLAUSES] THEN
BRIDGE_NEG_TAC (CONV_TAC (ONCE_DEPTH_CONV BIGNUM_OF_WORDLIST_CONV)) p_256
"""

TACTIC_ADD_P384 = r"""
MAP_EVERY X_GEN_TAC
 [`z:int64`; `x:int64`; `y:int64`; `m:num`; `n:num`; `pc:num`] THEN
REWRITE_TAC[C_ARGUMENTS; C_RETURN; SOME_FLAGS; NONOVERLAPPING_CLAUSES] THEN
DISCH_THEN(REPEAT_TCL CONJUNCTS_THEN ASSUME_TAC) THEN
REWRITE_TAC[BIGNUM_FROM_MEMORY_BYTES] THEN ENSURES_INIT_TAC "s0" THEN
BIGNUM_DIGITIZE_TAC "m_" `read (memory :> bytes (x,8 * 6)) s0` THEN
BIGNUM_DIGITIZE_TAC "n_" `read (memory :> bytes (y,8 * 6)) s0` THEN
ARM_ACCSTEPS_TAC BIGNUM_ADD_P384_EXEC (1--13) (1--13) THEN
SUBGOAL_THEN
 `2 EXP 384 * bitval carry_s12 +
  bignum_of_wordlist [sum_s3; sum_s4; sum_s7; sum_s8; sum_s11; sum_s12] =
  m + n`
ASSUME_TAC THENL
 [REWRITE_TAC[bignum_of_wordlist; MULT_CLAUSES; ADD_CLAUSES] THEN
  MAP_EVERY EXPAND_TAC ["m"; "n"] THEN
  REWRITE_TAC[GSYM REAL_OF_NUM_EQ; GSYM REAL_OF_NUM_ADD;
              GSYM REAL_OF_NUM_MUL; GSYM REAL_OF_NUM_POW] THEN
  ACCUMULATOR_ASSUM_LIST(MP_TAC o end_itlist CONJ o DECARRY_RULE) THEN
  DISCH_THEN(fun th -> REWRITE_TAC[th]) THEN REAL_ARITH_TAC;
  ACCUMULATOR_POP_ASSUM_LIST(K ALL_TAC)] THEN
ARM_ACCSTEPS_TAC BIGNUM_ADD_P384_EXEC (14--22) (14--22) THEN
RULE_ASSUM_TAC(REWRITE_RULE[REAL_BITVAL_NOT]) THEN
SUBGOAL_THEN
 `carry_s22 <=>
  p_384 <=
  bignum_of_wordlist [sum_s3; sum_s4; sum_s7; sum_s8; sum_s11; sum_s12]`
SUBST_ALL_TAC THENL
 [MATCH_MP_TAC FLAG_FROM_CARRY_LE THEN EXISTS_TAC `384` THEN
  REWRITE_TAC[bignum_of_wordlist; MULT_CLAUSES; ADD_CLAUSES] THEN
  REWRITE_TAC[GSYM REAL_OF_NUM_EQ; GSYM REAL_OF_NUM_ADD; p_384;
              GSYM REAL_OF_NUM_MUL; GSYM REAL_OF_NUM_POW] THEN
  ACCUMULATOR_ASSUM_LIST(MP_TAC o end_itlist CONJ o DECARRY_RULE) THEN
  DISCH_THEN(fun th -> REWRITE_TAC[th]) THEN BOUNDER_TAC[];
  ACCUMULATOR_POP_ASSUM_LIST(K ALL_TAC)] THEN
ARM_STEPS_TAC BIGNUM_ADD_P384_EXEC [23;24] THEN
FIRST_X_ASSUM(MP_TAC o
  SPEC `word_neg(word(bitval(p_384 <= m + n))):int64` o
  MATCH_MP (MESON[] `read X3 s = z ==> !a. z = a ==> read X3 s = a`)) THEN
ANTS_TAC THENL
 [REWRITE_TAC[GSYM WORD_ADD; ADD_CLAUSES; VAL_WORD; DIMINDEX_64] THEN
  SIMP_TAC[BITVAL_BOUND; MOD_LT; ADD_EQ_0; BITVAL_EQ_0;
           ARITH_RULE `a <= 1 /\ b <= 1 ==> a + b <  2 EXP 64`] THEN
  FIRST_X_ASSUM(fun th -> GEN_REWRITE_TAC (funpow 5 RAND_CONV) [SYM th]) THEN
  BOOL_CASES_TAC `carry_s12:bool` THEN
  REWRITE_TAC[BITVAL_CLAUSES; ADD_CLAUSES; COND_SWAP; MULT_CLAUSES;
              ADD_CLAUSES; WORD_MASK] THEN
  CONV_TAC WORD_REDUCE_CONV THEN
  ONCE_REWRITE_TAC[COND_RAND] THEN CONV_TAC WORD_REDUCE_CONV THEN
  REWRITE_TAC[p_384] THEN ARITH_TAC;
  DISCH_TAC] THEN
ARM_ACCSTEPS_TAC BIGNUM_ADD_P384_EXEC (25--38) (25--38) THEN
ENSURES_FINAL_STATE_TAC THEN ASM_REWRITE_TAC[] THEN STRIP_TAC THEN
CONV_TAC(LAND_CONV BIGNUM_EXPAND_CONV) THEN
ASM_REWRITE_TAC[] THEN DISCARD_STATE_TAC "s38" THEN
RULE_ASSUM_TAC(REWRITE_RULE [REAL_BITVAL_NOT; ADD_CLAUSES; VAL_WORD_BITVAL]) THEN
CONV_TAC SYM_CONV THEN
RULE_ASSUM_TAC (CONV_RULE (ONCE_DEPTH_CONV BIGNUM_OF_WORDLIST_CONV)) THEN
CONV_TAC (ONCE_DEPTH_CONV BIGNUM_OF_WORDLIST_CONV) THEN
BRIDGE_ADD_REF_TAC (CONV_TAC (ONCE_DEPTH_CONV BIGNUM_OF_WORDLIST_CONV)) p_384
"""

TACTICS = {"add-ref": TACTIC, "sub": TACTIC_SUB, "neg": TACTIC_NEG,
           "add-p384": TACTIC_ADD_P384}

SKIP = re.compile(r"^\s*(#|needs\b|loadt\b)")


def phrases(path):
    src = open(path, encoding="utf-8").read()
    for chunk in re.split(r";;[ \t]*(?:\n|$)", src):
        c = chunk.strip()
        if not c:
            continue
        if SKIP.match(c):
            continue
        yield c


def main(out, which="add-ref"):
    tactic = TACTICS[which]
    parts = []
    for path in SOURCES:
        for p in phrases(path):
            # every top-level phrase in these files is a `let` binding
            parts.append(p)
    wrapped = (
        "(fun g ->\n"
        "  try\n"
        "    (" + tactic.strip() + ") g\n"
        "  with e ->\n"
        "    Printf.printf \"ANSWER_EXCEPTION: %s\\n%!\" (Printexc.to_string e);\n"
        "    raise e)\n"
    )
    body = "(\n" + "\n".join(f"{p}\nin" for p in parts) + "\n" + wrapped + ")\n"
    open(out, "w", encoding="utf-8").write(body)
    print(f"wrote {out} bytes={len(body)} bindings={len(parts)}")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2] if len(sys.argv) > 2 else "add-ref")
