(* Development: ADD_REF_VAL.  Parentheses are fully explicit. *)
#load "unix.cma";;
needs "arm/proofs/base.ml";;

let MOD_SUB_ONE = prove
 (`!s q:num. q <= s /\ s < 2 * q ==> s MOD q = (s - q)`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  MATCH_MP_TAC MOD_UNIQ THEN EXISTS_TAC `1` THEN
  REWRITE_TAC[MULT_CLAUSES] THEN CONJ_TAC THENL
   [ASM_ARITH_TAC; ASM_ARITH_TAC]);;

let SUB_MOD = prove
 (`!s q m:num. q <= s /\ s < m ==> ((s + (m - q)) MOD m) = (s - q)`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  MATCH_MP_TAC MOD_UNIQ THEN EXISTS_TAC `1` THEN
  REWRITE_TAC[MULT_CLAUSES] THEN CONJ_TAC THENL
   [ASM_ARITH_TAC; ASM_ARITH_TAC]);;

(* Case: (a+b) MOD m < a.  The machine subtracts. *)
let MOD_SUB_REF_A1 = prove
 (`!p a b m:num.
      0 < p /\ p < m /\ a < p /\ b < p /\ ((a + b) MOD m) < a
      ==> ((((a + b) MOD m) + (m - p)) MOD m) = (a + b) MOD p`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  SUBGOAL_THEN `(a + b) < (2 * p)` ASSUME_TAC THENL
   [ASM_ARITH_TAC; ALL_TAC] THEN
  ASM_CASES_TAC `(a + b) < m` THENL
   [ SUBGOAL_THEN `(a + b) MOD m = (a + b)` ASSUME_TAC THENL
      [MATCH_MP_TAC MOD_LT THEN ASM_REWRITE_TAC[]; ALL_TAC] THEN
     SUBGOAL_THEN `(a + b) < a` ASSUME_TAC THENL
      [MP_TAC (ASSUME `((a + b) MOD m) < a`) THEN
       ASM_REWRITE_TAC[] THEN ASM_ARITH_TAC; ALL_TAC] THEN
     ASM_ARITH_TAC;
     SUBGOAL_THEN `(a + b) MOD m = ((a + b) - m)` ASSUME_TAC THENL
      [MATCH_MP_TAC MOD_SUB_ONE THEN ASM_ARITH_TAC; ALL_TAC] THEN
     ASM_REWRITE_TAC[] THEN
     SUBGOAL_THEN `((((a + b) - m) + (m - p)) = ((a + b) - p))`
       ASSUME_TAC THENL [ASM_ARITH_TAC; ALL_TAC] THEN
     SUBGOAL_THEN `(((a + b) - p) MOD m) = ((a + b) - p)` ASSUME_TAC THENL
      [MATCH_MP_TAC MOD_LT THEN ASM_ARITH_TAC; ALL_TAC] THEN
     SUBGOAL_THEN `(a + b) MOD p = ((a + b) - p)` ASSUME_TAC THENL
      [MATCH_MP_TAC MOD_SUB_ONE THEN ASM_ARITH_TAC; ALL_TAC] THEN
     ASM_REWRITE_TAC[] ]);;

(* Case: p <= (a+b) MOD m.  The machine subtracts. *)
let MOD_SUB_REF_A2 = prove
 (`!p a b m:num.
      0 < p /\ p < m /\ a < p /\ b < p /\ p <= ((a + b) MOD m)
      ==> ((((a + b) MOD m) + (m - p)) MOD m) = (a + b) MOD p`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  SUBGOAL_THEN `(a + b) < (2 * p)` ASSUME_TAC THENL
   [ASM_ARITH_TAC; ALL_TAC] THEN
  ASM_CASES_TAC `(a + b) < m` THENL
   [ SUBGOAL_THEN `(a + b) MOD m = (a + b)` ASSUME_TAC THENL
      [MATCH_MP_TAC MOD_LT THEN ASM_REWRITE_TAC[]; ALL_TAC] THEN
     SUBGOAL_THEN `p <= (a + b)` ASSUME_TAC THENL
      [MP_TAC (ASSUME `p <= ((a + b) MOD m)`) THEN
       ASM_REWRITE_TAC[] THEN ASM_ARITH_TAC; ALL_TAC] THEN
     ASM_REWRITE_TAC[] THEN
     SUBGOAL_THEN `(((a + b) + (m - p)) MOD m) = ((a + b) - p)`
       ASSUME_TAC THENL
      [MATCH_MP_TAC SUB_MOD THEN ASM_ARITH_TAC; ALL_TAC] THEN
     SUBGOAL_THEN `(a + b) MOD p = ((a + b) - p)` ASSUME_TAC THENL
      [MATCH_MP_TAC MOD_SUB_ONE THEN ASM_ARITH_TAC; ALL_TAC] THEN
     ASM_REWRITE_TAC[];
     SUBGOAL_THEN `(a + b) MOD m = ((a + b) - m)` ASSUME_TAC THENL
      [MATCH_MP_TAC MOD_SUB_ONE THEN ASM_ARITH_TAC; ALL_TAC] THEN
     ASM_REWRITE_TAC[] THEN
     SUBGOAL_THEN `((((a + b) - m) + (m - p)) = ((a + b) - p))`
       ASSUME_TAC THENL [ASM_ARITH_TAC; ALL_TAC] THEN
     SUBGOAL_THEN `(((a + b) - p) MOD m) = ((a + b) - p)` ASSUME_TAC THENL
      [MATCH_MP_TAC MOD_LT THEN ASM_ARITH_TAC; ALL_TAC] THEN
     SUBGOAL_THEN `(a + b) MOD p = ((a + b) - p)` ASSUME_TAC THENL
      [MATCH_MP_TAC MOD_SUB_ONE THEN ASM_ARITH_TAC; ALL_TAC] THEN
     ASM_REWRITE_TAC[] ]);;

(* Case: neither.  The machine keeps the sum. *)
let MOD_KEEP_REF = prove
 (`!p a b m:num.
      0 < p /\ p < m /\ a < p /\ b < p /\
      ~(((a + b) MOD m) < a) /\ ~(p <= ((a + b) MOD m))
      ==> (a + b) MOD m = (a + b) MOD p`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  ASM_CASES_TAC `(a + b) < m` THENL
   [ SUBGOAL_THEN `(a + b) MOD m = (a + b)` ASSUME_TAC THENL
      [MATCH_MP_TAC MOD_LT THEN ASM_REWRITE_TAC[]; ALL_TAC] THEN
     SUBGOAL_THEN `(a + b) < p` ASSUME_TAC THENL
      [MP_TAC (ASSUME `~(p <= ((a + b) MOD m))`) THEN
       ASM_REWRITE_TAC[] THEN ASM_ARITH_TAC; ALL_TAC] THEN
     SUBGOAL_THEN `(a + b) MOD p = (a + b)` ASSUME_TAC THENL
      [MATCH_MP_TAC MOD_LT THEN ASM_ARITH_TAC; ALL_TAC] THEN
     ASM_REWRITE_TAC[];
     SUBGOAL_THEN `(a + b) MOD m = ((a + b) - m)` ASSUME_TAC THENL
      [MATCH_MP_TAC MOD_SUB_ONE THEN ASM_ARITH_TAC; ALL_TAC] THEN
     SUBGOAL_THEN `((a + b) MOD m) < a` ASSUME_TAC THENL
      [ASM_ARITH_TAC; ALL_TAC] THEN
     UNDISCH_TAC `~(((a + b) MOD m) < a)` THEN ASM_REWRITE_TAC[] ]);;

let ref_add = new_definition
 `ref_add (pw:W word) (x:W word) (y:W word) =
    let s = word_add x y in
    if word_ult s x \/ word_ule pw s then word_sub s pw else s`;;

let ADD_REF_VAL = prove
 (`!(pw:W word) (x:W word) (y:W word).
       0 < val pw /\ val x < val pw /\ val y < val pw
       ==> val(ref_add pw x y) = (val x + val y) MOD val pw`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  SUBGOAL_THEN `0 < 2 EXP dimindex(:W)` ASSUME_TAC THENL
   [REWRITE_TAC[EXP_LT_0; ARITH]; ALL_TAC] THEN
  SUBGOAL_THEN
   `val(pw:W word) < 2 EXP dimindex(:W) /\
    val(x:W word) < 2 EXP dimindex(:W) /\
    val(y:W word) < 2 EXP dimindex(:W)`
    STRIP_ASSUME_TAC THENL
   [REWRITE_TAC[VAL_BOUND]; ALL_TAC] THEN
  REWRITE_TAC[ref_add; LET_DEF] THEN BETA_TAC THEN
  REWRITE_TAC[WORD_ULT; WORD_ULE; VAL_WORD_ADD] THEN
  ASM_CASES_TAC
   `((val(x:W word) + val(y:W word)) MOD (2 EXP dimindex(:W))) < val(x:W word)`
   THENL
   [ ASM_REWRITE_TAC[] THEN
     REWRITE_TAC[VAL_WORD_SUB; VAL_WORD_ADD] THEN
     MATCH_MP_TAC MOD_SUB_REF_A1 THEN REPEAT CONJ_TAC THEN ASM_REWRITE_TAC[];
     ASM_CASES_TAC
      `val(pw:W word) <=
       ((val(x:W word) + val(y:W word)) MOD (2 EXP dimindex(:W)))`
      THENL
      [ ASM_REWRITE_TAC[] THEN
        REWRITE_TAC[VAL_WORD_SUB; VAL_WORD_ADD] THEN
        MATCH_MP_TAC MOD_SUB_REF_A2 THEN REPEAT CONJ_TAC THEN ASM_REWRITE_TAC[];
        ASM_REWRITE_TAC[] THEN
        REWRITE_TAC[VAL_WORD_ADD] THEN
        MATCH_MP_TAC MOD_KEEP_REF THEN REPEAT CONJ_TAC THEN ASM_REWRITE_TAC[] ] ]);;

let ADD_REF_VAL_WORD = prove
 (`!p (x:W word) (y:W word).
       0 < p /\ p < 2 EXP dimindex(:W) /\ val x < p /\ val y < p
       ==> val(ref_add (word p : W word) x y) = (val x + val y) MOD p`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  MATCH_MP_TAC ADD_REF_VAL THEN
  ASM_SIMP_TAC[VAL_WORD; MOD_LT]);;

Printf.printf "ADD_REF_DONE axioms=%d\n%!" (List.length (axioms ()));;
