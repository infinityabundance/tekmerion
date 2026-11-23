(* Interactive development of the two numeric branch lemmas. *)
#load "unix.cma";;
needs "arm/proofs/base.ml";;

let MOD_SUB_ONE = prove
 (`!s q:num. q <= s /\ s < 2 * q ==> s MOD q = s - q`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  MATCH_MP_TAC MOD_UNIQ THEN EXISTS_TAC `1` THEN
  REWRITE_TAC[MULT_CLAUSES] THEN CONJ_TAC THENL
   [ASM_ARITH_TAC; ASM_ARITH_TAC]);;

let SUB_MOD = prove
 (`!s q m:num. q <= s /\ s < m ==> (s + m - q) MOD m = s - q`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  MATCH_MP_TAC MOD_UNIQ THEN EXISTS_TAC `1` THEN
  REWRITE_TAC[MULT_CLAUSES] THEN CONJ_TAC THENL
   [ASM_ARITH_TAC; ASM_ARITH_TAC]);;

(* ---------------- Lemma A ---------------- *)
set_goal ([], `!p a b m:num.
     0 < p /\ p < m /\ a < p /\ b < p /\
     ((a + b) MOD m < a \/ p <= (a + b) MOD m)
     ==> ((a + b) MOD m + m - p) MOD m = (a + b) MOD p`);;
e (REPEAT GEN_TAC THEN STRIP_TAC);;
e (SUBGOAL_THEN `a + b < 2 * p` ASSUME_TAC THENL [ASM_ARITH_TAC; ALL_TAC]);;
e (ASM_CASES_TAC `a + b < m`);;
(* true branch *)
e (SUBGOAL_THEN `(a + b) MOD m = a + b` ASSUME_TAC THENL
     [MATCH_MP_TAC MOD_LT THEN ASM_REWRITE_TAC[]; ALL_TAC]);;
e (SUBGOAL_THEN `p <= a + b` ASSUME_TAC THENL
     [UNDISCH_TAC `(a + b) MOD m < a \/ p <= (a + b) MOD m` THEN
      ASM_REWRITE_TAC[] THEN ASM_ARITH_TAC; ALL_TAC]);;
e (ASM_REWRITE_TAC[]);;
e (SUBGOAL_THEN `(a + b + m - p) MOD m = a + b - p` ASSUME_TAC THENL
     [MATCH_MP_TAC SUB_MOD THEN ASM_ARITH_TAC; ALL_TAC]);;
e (SUBGOAL_THEN `(a + b) MOD p = a + b - p` ASSUME_TAC THENL
     [MATCH_MP_TAC MOD_SUB_ONE THEN ASM_ARITH_TAC; ALL_TAC]);;
e (ASM_REWRITE_TAC[]);;
(* false branch *)
e (SUBGOAL_THEN `(a + b) MOD m = a + b - m` ASSUME_TAC THENL
     [MATCH_MP_TAC MOD_SUB_ONE THEN ASM_ARITH_TAC; ALL_TAC]);;
e (ASM_REWRITE_TAC[]);;
e (SUBGOAL_THEN `a + b - m + m - p = a + b - p` ASSUME_TAC THENL
     [ASM_ARITH_TAC; ALL_TAC]);;
e (SUBGOAL_THEN `(a + b - p) MOD m = a + b - p` ASSUME_TAC THENL
     [MATCH_MP_TAC MOD_LT THEN ASM_ARITH_TAC; ALL_TAC]);;
e (SUBGOAL_THEN `(a + b) MOD p = a + b - p` ASSUME_TAC THENL
     [MATCH_MP_TAC MOD_SUB_ONE THEN ASM_ARITH_TAC; ALL_TAC]);;
e (ASM_REWRITE_TAC[]);;

let MOD_SUB_REF = top_thm();;
Printf.printf "LEMMA_A_DONE\n%!";;

(* ---------------- Lemma B ---------------- *)
set_goal ([], `!p a b m:num.
     0 < p /\ p < m /\ a < p /\ b < p /\
     ~((a + b) MOD m < a \/ p <= (a + b) MOD m)
     ==> (a + b) MOD m = (a + b) MOD p`);;
e (REPEAT GEN_TAC THEN STRIP_TAC);;
e (ASM_CASES_TAC `a + b < m`);;
(* true *)
e (SUBGOAL_THEN `(a + b) MOD m = a + b` ASSUME_TAC THENL
     [MATCH_MP_TAC MOD_LT THEN ASM_REWRITE_TAC[]; ALL_TAC]);;
e (SUBGOAL_THEN `~(p <= a + b)` ASSUME_TAC THENL
     [UNDISCH_TAC `~((a + b) MOD m < a \/ p <= (a + b) MOD m)` THEN
      ASM_REWRITE_TAC[] THEN ASM_ARITH_TAC; ALL_TAC]);;
e (SUBGOAL_THEN `(a + b) MOD p = a + b` ASSUME_TAC THENL
     [MATCH_MP_TAC MOD_LT THEN ASM_ARITH_TAC; ALL_TAC]);;
e (ASM_REWRITE_TAC[]);;
(* false *)
e (SUBGOAL_THEN `(a + b) MOD m = a + b - m` ASSUME_TAC THENL
     [MATCH_MP_TAC MOD_SUB_ONE THEN ASM_ARITH_TAC; ALL_TAC]);;
e (SUBGOAL_THEN `(a + b) MOD m < a` ASSUME_TAC THENL [ASM_ARITH_TAC; ALL_TAC]);;
e (UNDISCH_TAC `~((a + b) MOD m < a \/ p <= (a + b) MOD m)` THEN
    ASM_REWRITE_TAC[]);;

let MOD_KEEP_REF = top_thm();;
Printf.printf "LEMMA_B_DONE axioms=%d\n%!" (List.length (axioms ()));;
