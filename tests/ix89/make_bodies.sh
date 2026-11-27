#!/bin/bash
# IX.89: emit the fully instrumented human tactic bodies (phase + per-tactic
# markers) for arm Curve25519 {add,sub,neg}.  The bodies are the upstream
# tactics with identity `ix89_mark` calls inserted; no other change is made.
# Usage: bash tests/ix89/make_bodies.sh <outdir>
set -eu
out=${1:?usage: make_bodies.sh <outdir>}
mkdir -p "$out"

# gen_sim <EXEC> <anums-comma> <first> <last>
# One ARM_ACCSTEPS_TAC per instruction (identical to the single MAP_EVERY call
# upstream makes, since ARM_ACCSTEPS_TAC is defined by MAP_EVERY over the step
# list).  An instruction in <anums> is an accumulated step, others are plain.
gen_sim() {
  exec="$1"; anums="$2"; a="$3"; b="$4"
  semi=$(echo "$anums" | tr ',' ';')
  for n in $(seq "$a" "$b"); do
    cls=plain
    case ",$anums," in *",$n,"*) cls=acc;; esac
    printf 'ix89_mark "B:step%02d:%s" THEN\nARM_ACCSTEPS_TAC %s [%s] (%d--%d) THEN\n' \
      "$n" "$cls" "$exec" "$semi" "$n" "$n"
  done
}

# ---------------------------------------------------------------- add --------
{
cat <<'EOF'
ix89_mark "A:init" THEN
MAP_EVERY X_GEN_TAC
 [`z:int64`; `x:int64`; `y:int64`; `m:num`; `n:num`; `pc:num`] THEN
REWRITE_TAC[C_ARGUMENTS; C_RETURN; SOME_FLAGS; NONOVERLAPPING_CLAUSES] THEN
DISCH_THEN(REPEAT_TCL CONJUNCTS_THEN ASSUME_TAC) THEN
REWRITE_TAC[BIGNUM_FROM_MEMORY_BYTES] THEN ENSURES_INIT_TAC "s0" THEN
ix89_mark "A:digitize_m" THEN
BIGNUM_LDIGITIZE_TAC "m_" `read (memory :> bytes (x,8 * 4)) s0` THEN
ix89_mark "A:digitize_n" THEN
BIGNUM_LDIGITIZE_TAC "n_" `read (memory :> bytes (y,8 * 4)) s0` THEN
ix89_mark "B:start" THEN
EOF
gen_sim BIGNUM_ADD_P25519_EXEC 3,4,7,8,10,11,12,13 1 19
cat <<'EOF'
ix89_mark "E:finalise_state" THEN
ENSURES_FINAL_STATE_TAC THEN ASM_REWRITE_TAC[] THEN STRIP_TAC THEN
ix89_mark "E:prep_bound" THEN
SUBGOAL_THEN `m DIV 2 EXP 192 < 2 EXP 63 /\ n DIV 2 EXP 192 < 2 EXP 63`
MP_TAC THENL
 [ix89_mark "E:pb1:pre" THEN
  MAP_EVERY UNDISCH_TAC [`m < p_25519`; `n < p_25519`] THEN
  REWRITE_TAC[p_25519] THEN
  ix89_mark "E:pb1:ARITH_TAC" THEN ARITH_TAC;
  ix89_mark "E:pb2:expand" THEN
  MAP_EVERY EXPAND_TAC ["m"; "n"] THEN
  CONV_TAC(ONCE_DEPTH_CONV BIGNUM_OF_WORDLIST_DIV_CONV) THEN
  ix89_mark "E:pb2:NUM_REDUCE_CONV" THEN
  DISCH_THEN(STRIP_ASSUME_TAC o CONV_RULE NUM_REDUCE_CONV)] THEN
ix89_mark "E:post_bound" THEN
ix89_mark "C:flag" THEN
SUBGOAL_THEN `carry_s13 <=> p_25519 <= m + n` SUBST_ALL_TAC THENL
 [ix89_mark "C:flag:pre" THEN
  MATCH_MP_TAC FLAG_FROM_CARRY_LE THEN EXISTS_TAC `256` THEN
  MAP_EVERY EXPAND_TAC ["m"; "n"] THEN
  REWRITE_TAC[p_25519] THEN
  ix89_mark "C:flag:NUM_REDUCE_CONV" THEN
  CONV_TAC NUM_REDUCE_CONV THEN
  ix89_mark "C:flag:decarrry" THEN
  REWRITE_TAC[bignum_of_wordlist; GSYM REAL_OF_NUM_CLAUSES] THEN
  ACCUMULATOR_ASSUM_LIST(fun ths -> ASSUM_LIST (fun thc ->
    MP_TAC(end_itlist CONJ (GEN_DECARRY_RULE thc ths)))) THEN
  DISCH_THEN(fun th -> REWRITE_TAC[th]) THEN
  ix89_mark "C:flag:BOUNDER_TAC" THEN BOUNDER_TAC[];
  ix89_mark "C:flag:cont" THEN ALL_TAC] THEN
ix89_mark "C:post_flag" THEN
ix89_mark "E:final:expand" THEN
CONV_TAC(LAND_CONV BIGNUM_EXPAND_CONV) THEN
ix89_mark "E:final:rewrite1" THEN
ASM_REWRITE_TAC[] THEN DISCARD_STATE_TAC "s19" THEN
ix89_mark "E:final:modadd" THEN
W(MP_TAC o PART_MATCH (lhand o rand) MOD_ADD_CASES o rand o snd) THEN
ASM_REWRITE_TAC[] THEN DISCH_THEN SUBST1_TAC THEN
ix89_mark "E:final:rewrite2" THEN
REWRITE_TAC[GSYM NOT_LE; COND_SWAP] THEN
REWRITE_TAC[GSYM REAL_OF_NUM_EQ] THEN ONCE_REWRITE_TAC[COND_RAND] THEN
SIMP_TAC[GSYM REAL_OF_NUM_SUB] THEN
ix89_mark "E:final:abbrev" THEN
ABBREV_TAC `bb <=> p_25519 <= m + n` THEN
MAP_EVERY EXPAND_TAC ["m"; "n"] THEN
REWRITE_TAC[GSYM REAL_OF_NUM_CLAUSES; bignum_of_wordlist; p_25519] THEN
ix89_mark "D:final:NUM_REDUCE_CONV" THEN
CONV_TAC NUM_REDUCE_CONV THEN
ix89_mark "E:final:cond" THEN
GEN_REWRITE_TAC (LAND_CONV o ONCE_DEPTH_CONV) [COND_RAND] THEN
FIRST_X_ASSUM(SUBST1_TAC o MATCH_MP (REAL_ARITH
 `e * c + s:real = z ==> s = z - e * c`)) THEN
ix89_mark "E:final:decarrry" THEN
ACCUMULATOR_ASSUM_LIST(fun ths -> ASSUM_LIST (fun thc ->
  MP_TAC(end_itlist CONJ (GEN_DECARRY_RULE thc ths)))) THEN
DISCH_THEN(fun th -> REWRITE_TAC[th]) THEN
ix89_mark "E:final:condcases" THEN
COND_CASES_TAC THEN ASM_REWRITE_TAC[BITVAL_CLAUSES] THEN
ix89_mark "D:final:REAL_ARITH_TAC" THEN
REAL_ARITH_TAC
EOF
} > "$out/add.tac"

# ---------------------------------------------------------------- sub --------
{
cat <<'EOF'
ix89_mark "A:init" THEN
MAP_EVERY X_GEN_TAC
 [`z:int64`; `x:int64`; `y:int64`; `m:num`; `n:num`; `pc:num`] THEN
REWRITE_TAC[C_ARGUMENTS; C_RETURN; SOME_FLAGS; NONOVERLAPPING_CLAUSES] THEN
DISCH_THEN(REPEAT_TCL CONJUNCTS_THEN ASSUME_TAC) THEN
REWRITE_TAC[BIGNUM_FROM_MEMORY_BYTES] THEN ENSURES_INIT_TAC "s0" THEN
ix89_mark "A:digitize_m" THEN
BIGNUM_DIGITIZE_TAC "m_" `read (memory :> bytes (x,8 * 4)) s0` THEN
ix89_mark "A:digitize_n" THEN
BIGNUM_DIGITIZE_TAC "n_" `read (memory :> bytes (y,8 * 4)) s0` THEN
ix89_mark "B:start" THEN
EOF
gen_sim BIGNUM_SUB_P25519_EXEC 1,2,3,4,5,6,7,8 1 8
cat <<'EOF'
ix89_mark "C:flag" THEN
SUBGOAL_THEN `carry_s8 <=> m < n` SUBST_ALL_TAC THENL
 [ix89_mark "C:flag:pre" THEN
  MATCH_MP_TAC FLAG_FROM_CARRY_LT THEN EXISTS_TAC `256` THEN
  MAP_EVERY EXPAND_TAC ["m"; "n"] THEN REWRITE_TAC[GSYM REAL_OF_NUM_ADD] THEN
  REWRITE_TAC[GSYM REAL_OF_NUM_MUL; GSYM REAL_OF_NUM_POW] THEN
  ACCUMULATOR_ASSUM_LIST(MP_TAC o end_itlist CONJ o DECARRY_RULE) THEN
  DISCH_THEN(fun th -> REWRITE_TAC[th]) THEN
  ix89_mark "C:flag:BOUNDER_TAC" THEN BOUNDER_TAC[];
  ix89_mark "C:flag:cont" THEN ALL_TAC] THEN
ix89_mark "C:post_flag" THEN
EOF
gen_sim BIGNUM_SUB_P25519_EXEC 11,12,13,14 9 17
cat <<'EOF'
ix89_mark "E:finalise_state" THEN
ENSURES_FINAL_STATE_TAC THEN ASM_REWRITE_TAC[] THEN STRIP_TAC THEN
ix89_mark "E:final:expand" THEN
CONV_TAC(LAND_CONV(RAND_CONV BIGNUM_EXPAND_CONV)) THEN
ASM_REWRITE_TAC[] THEN DISCARD_STATE_TAC "s17" THEN
ix89_mark "E:final:trans" THEN
TRANS_TAC EQ_TRANS `if m < n then &m - &n + &p_25519:int else &m - &n` THEN
CONJ_TAC THENL
 [ix89_mark "E:final:rewrite1" THEN
  ONCE_REWRITE_TAC[int_eq] THEN ONCE_REWRITE_TAC[COND_RAND] THEN
  REWRITE_TAC[int_of_num_th; int_sub_th; int_add_th];
  ix89_mark "D:final:INT_ARITH_TAC" THEN
  CONV_TAC SYM_CONV THEN MATCH_MP_TAC INT_REM_UNIQ THEN
  EXISTS_TAC `if m:num < n then --(&1):int else &0` THEN
  MAP_EVERY UNDISCH_TAC [`m < p_25519`; `n < p_25519`] THEN
  REWRITE_TAC[GSYM INT_OF_NUM_CLAUSES; p_25519] THEN INT_ARITH_TAC] THEN
ix89_mark "E:final:cong" THEN
MATCH_MP_TAC EQUAL_FROM_CONGRUENT_REAL THEN
MAP_EVERY EXISTS_TAC [`255`; `&0:real`] THEN
CONJ_TAC THENL
 [ix89_mark "D:final:BOUNDER_TAC" THEN BOUNDER_TAC[];
  ix89_mark "E:final:cong:cont" THEN ALL_TAC] THEN
CONJ_TAC THENL
 [ix89_mark "E:final:numreduce" THEN
  MAP_EVERY UNDISCH_TAC [`m < p_25519`; `n < p_25519`] THEN
  REWRITE_TAC[GSYM REAL_OF_NUM_CLAUSES; p_25519] THEN
  ix89_mark "D:final:NUM_REDUCE_CONV" THEN
  CONV_TAC NUM_REDUCE_CONV THEN
  ix89_mark "D:final:REAL_ARITH_TAC" THEN REAL_ARITH_TAC;
  ix89_mark "E:final:integer_closed" THEN REWRITE_TAC[INTEGER_CLOSED]] THEN
ix89_mark "E:final:abbrev" THEN
ABBREV_TAC `bb <=> m:num < n` THEN MAP_EVERY EXPAND_TAC ["m"; "n"] THEN
REWRITE_TAC[GSYM(NUM_REDUCE_CONV `2 EXP 63 - 1`); p_25519] THEN
REWRITE_TAC[VAL_WORD_AND_MASK_WORD] THEN
ix89_mark "D:final:NUM_REDUCE_CONV.2" THEN
CONV_TAC NUM_REDUCE_CONV THEN
ix89_mark "E:final:rewrite2" THEN
REWRITE_TAC[GSYM REAL_OF_NUM_CLAUSES; REAL_OF_NUM_MOD] THEN
ACCUMULATOR_POP_ASSUM_LIST(MP_TAC o end_itlist CONJ o DESUM_RULE) THEN
ix89_mark "E:final:condcases" THEN
COND_CASES_TAC THEN ASM_REWRITE_TAC[BITVAL_CLAUSES] THEN
CONV_TAC(DEPTH_CONV WORD_NUM_RED_CONV) THEN
DISCH_THEN(fun th -> REWRITE_TAC[th]) THEN
ix89_mark "D:final:REAL_INTEGER_TAC" THEN
REAL_INTEGER_TAC
EOF
} > "$out/sub.tac"

# ---------------------------------------------------------------- neg --------
{
cat <<'EOF'
ix89_mark "A:init" THEN
MAP_EVERY X_GEN_TAC [`z:int64`; `x:int64`; `n:num`; `pc:num`] THEN
REWRITE_TAC[C_ARGUMENTS; C_RETURN; SOME_FLAGS; NONOVERLAPPING_CLAUSES] THEN
DISCH_THEN(REPEAT_TCL CONJUNCTS_THEN ASSUME_TAC) THEN
REWRITE_TAC[BIGNUM_FROM_MEMORY_BYTES] THEN ENSURES_INIT_TAC "s0" THEN
ix89_mark "A:digitize_n" THEN
BIGNUM_LDIGITIZE_TAC "n_" `read (memory :> bytes (x,8 * 4)) s0` THEN
ix89_mark "B:start" THEN
EOF
gen_sim BIGNUM_NEG_P25519_EXEC 4,6,9,12 1 19
cat <<'EOF'
ix89_mark "E:finalise_state" THEN
ENSURES_FINAL_STATE_TAC THEN ASM_REWRITE_TAC[] THEN STRIP_TAC THEN
ix89_mark "E:final:expand" THEN
CONV_TAC(LAND_CONV BIGNUM_EXPAND_CONV) THEN
ASM_REWRITE_TAC[] THEN DISCARD_STATE_TAC "s19" THEN
ix89_mark "E:final:sym" THEN
CONV_TAC SYM_CONV THEN MATCH_MP_TAC EQUAL_FROM_CONGRUENT_MOD_MOD THEN
MAP_EVERY EXISTS_TAC [`256`; `if n = 0 then &0:real else &p_25519 - &n`] THEN
ix89_mark "E:final:conj" THEN
REPEAT CONJ_TAC THENL
 [ix89_mark "D:final:BOUNDER_TAC" THEN BOUNDER_TAC[];
  ix89_mark "D:final:ARITH_TAC.1" THEN
  REWRITE_TAC[p_25519] THEN ARITH_TAC;
  ix89_mark "D:final:ARITH_TAC.2" THEN
  REWRITE_TAC[p_25519] THEN ARITH_TAC;
  ix89_mark "E:final:b4_all" THEN
  ALL_TAC;
  ix89_mark "E:final:b5_setup" THEN
  COND_CASES_TAC THEN ASM_REWRITE_TAC[SUB_0; MOD_REFL] THEN
  ASM_SIMP_TAC[REAL_OF_NUM_SUB; REAL_OF_NUM_CLAUSES] THEN
  MATCH_MP_TAC MOD_LT THEN
  ix89_mark "D:final:ASM_ARITH_TAC" THEN ASM_ARITH_TAC] THEN
ix89_mark "E:final:rewrite1" THEN
EXPAND_TAC "n" THEN REWRITE_TAC[BIGNUM_OF_WORDLIST_EQ_0; ALL] THEN
REWRITE_TAC[VAL_EQ_0; WORD_SUB_EQ_0; WORD_OR_EQ_0] THEN
REWRITE_TAC[COND_SWAP; CONJ_ACI; bignum_of_wordlist] THEN
ix89_mark "E:final:condcases" THEN
COND_CASES_TAC THEN ASM_REWRITE_TAC[VAL_WORD_0; p_25519] THEN
ix89_mark "D:final:NUM_REDUCE_CONV" THEN
CONV_TAC NUM_REDUCE_CONV THEN CONV_TAC REAL_RAT_REDUCE_CONV THEN
ix89_mark "E:final:rewrite2" THEN
REWRITE_TAC[INTEGER_CLOSED; GSYM REAL_OF_NUM_CLAUSES] THEN
ACCUMULATOR_POP_ASSUM_LIST(MP_TAC o end_itlist CONJ o DESUM_RULE) THEN
DISCH_THEN(fun th -> REWRITE_TAC[th]) THEN
ix89_mark "D:final:REAL_INTEGER_TAC" THEN
REAL_INTEGER_TAC
EOF
} > "$out/neg.tac"

echo "wrote $out/add.tac $out/sub.tac $out/neg.tac"
wc -l "$out"/*.tac
