#load "unix.cma";;
needs "arm/proofs/base.ml";;
let SUB_ADD_BACK = prove
 (`!a b m:num. m <= a + b ==> ((a + b) - m) + m = a + b`,
  REPEAT GEN_TAC THEN DISCH_TAC THEN MATCH_MP_TAC SUB_ADD THEN ASM_REWRITE_TAC[]);;
let SUB_LT_DROP = prove
 (`!a b m:num. b < m /\ m <= a + b ==> (a + b) - m < a`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  MATCH_MP_TAC(ARITH_RULE `!a b m. b < m /\ m <= a + b ==> (a + b) - m < a`) THEN
  ASM_REWRITE_TAC[]);;

let stmt1 = `!(x:W word) (y:W word).
        val x < 2 EXP dimindex(:W) /\ val y < 2 EXP dimindex(:W) /\
        val(word_add x y) < val x
        ==> val(word_add x y) + 2 EXP dimindex(:W) = val x + val y`;;
set_goal([], stmt1);;
e (REPEAT GEN_TAC THEN STRIP_TAC);;
e (ASM_CASES_TAC `val(x:W word) + val(y:W word) < 2 EXP dimindex(:W)`);;
Printf.printf "=== CARRY goalstack after case split ===\n";;
p();;
