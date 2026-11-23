(* Interactive debugging of the word-level ADD_REF_VAL goal. *)
#load "unix.cma";;
needs "arm/proofs/base.ml";;
loadt "/mnt/1tb_kingston/Tekmerion/research/receipts/bridge/add_ref_num_dev.ml";;

let ref_add = new_definition
 `ref_add (pw:W word) (x:W word) (y:W word) =
    let s = word_add x y in
    if word_ult s x \/ word_ule pw s then word_sub s pw else s`;;

set_goal ([], `!(pw:W word) (x:W word) (y:W word).
       0 < val pw /\ val x < val pw /\ val y < val pw
       ==> val(ref_add pw x y) = (val x + val y) MOD val pw`);;
e (REPEAT GEN_TAC THEN STRIP_TAC);;
e (SUBGOAL_THEN `0 < 2 EXP dimindex(:W)` ASSUME_TAC THENL
     [REWRITE_TAC[EXP_LT_0; ARITH]; ALL_TAC]);;
e (SUBGOAL_THEN
     `val(pw:W word) < 2 EXP dimindex(:W) /\
      val(x:W word) < 2 EXP dimindex(:W) /\
      val(y:W word) < 2 EXP dimindex(:W)`
     STRIP_ASSUME_TAC THENL [REWRITE_TAC[VAL_BOUND]; ALL_TAC]);;
e (REWRITE_TAC[ref_add; LET_DEF] THEN BETA_TAC);;
e (REWRITE_TAC[WORD_ULT; WORD_ULE; VAL_WORD_ADD]);;
e (ASM_CASES_TAC
     `((val(x:W word) + val(y:W word)) MOD (2 EXP dimindex(:W))) < val(x:W word)`);;
(* -> inspect which branch is first; the log shows it *)
e (ASM_REWRITE_TAC[]);;
e (REWRITE_TAC[VAL_WORD_SUB; VAL_WORD_ADD]);;
e (MATCH_MP_TAC MOD_SUB_REF_A1);;
e (REPEAT CONJ_TAC);;
e (ASM_REWRITE_TAC[]);;
Printf.printf "BRANCH1_DONE\n%!";;
