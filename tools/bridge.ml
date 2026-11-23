(* ========================================================================= *)
(* bridge.ml -- generic bridge lemmas from a num/MOD (or int/rem, real/&)    *)
(* specification to a word-level circuit statement that certified SAT can    *)
(* discharge.                                                                *)
(*                                                                           *)
(* Parameterisation: the modulus `p` is an argument of every lemma, the limb *)
(* width (64) enters only through the numerals that appear in the shifts,    *)
(* and the number of limbs enters only through the nesting depth of the      *)
(* joins.  There is no P-256 constant, no P-256 limb list and no transposed  *)
(* human tactic fragment in this file: the arithmetic below is the general   *)
(* statement that `(m + n) MOD p`, `(&m - &n) rem &p` and `(p - n) MOD p`    *)
(* are what the machine's written residue is, for any `p`.                   *)
(*                                                                           *)
(* Provenance of the mathematics used:                                       *)
(*   MOD_ADD_CASES / MOD_LT / MOD_REFL / EXP_ADD / LT_MULT_LCANCEL  (arith)  *)
(*   INT_REM_UNIQ / INT_OF_NUM_* / INT_POW_ADD / INT_*_DISTRIB      (int)    *)
(*   VAL_WORD_JOIN_SIMPLE / VAL_WORD_0 / VAL_BOUND / VAL_WORD_BITVAL (words) *)
(*   MOD_UNIQ / SUB_ADD / VAL_WORD_ADD / VAL_WORD_SUB               (arith)  *)
(*                                                                           *)
(* The reference route: `ref_add' is the modular specification in one word    *)
(* operation, and ADD_REF_VAL is the only lemma that carries MOD.  SAT       *)
(* instances built on it are pure word statements.                          *)
(*                                                                           *)
(* Conversion note: INT_SHIFT_TO_JOIN_CONV accepts a *numeral* operand, not   *)
(* only `&(val x)' and `&(bitval b)'.  The case split substitutes a carry    *)
(* by the literal 0 or 1, so `&2 pow 64 * &0' must still become a join; if   *)
(* it does not, the accumulator equation is left as an opaque int atom and   *)
(* the flag it constrains is free.                                          *)
(*                                                                           *)
(* Kernel trust: every SAT refutation is replayed by the LCF kernel inside   *)
(* CSAT_REFUTE (csat_lib.ml); no solver certificate is trusted and no         *)
(* compiler enters the trusted base.                                         *)
(*                                                                           *)
(* DEVELOPMENT_ONLY (not lemmas; recorded so they cannot hide):             *)
(*   BRIDGE_ADD_TAC / BRIDGE_SUB_TAC / BRIDGE_NEG_TAC are *shape dispatch*.  *)
(*   They inspect the postcondition shape (sum MOD p, difference rem p,      *)
(*   complement MOD p), split at the single conditional that shape admits,   *)
(*   and call the generic lemmas above.  The old human tail (DECARRY_RULE,   *)
(*   DESUM_RULE, BOUNDER_TAC, EQUAL_FROM_CONGRUENT_MOD_MOD,                 *)
(*   REAL_INTEGER_TAC) is not used anywhere in this file.                    *)
(* ========================================================================= *)

(* ------------------------------------------------------------------------- *)
(* 1. The modular specification, split at its single conditional correction. *)
(*                                                                           *)
(* Each lemma states the spec side in terms of the value the machine writes, *)
(* so the machine-side obligation that reaches SAT is a pure *sum* identity: *)
(* no MOD, no rem, no subtraction.                                           *)
(* ------------------------------------------------------------------------- *)

(* add: (m + n) MOD p, at most one correction because m < p and n < p. *)
let ADD_MOD_LT = prove
 (`!p m n. m < p /\ n < p /\ m + n < p ==> (m + n) MOD p = m + n`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  SUBGOAL_THEN `(m + n) MOD p = (if m + n < p then m + n else (m + n) - p)`
    (fun th -> REWRITE_TAC[th]) THENL
   [MATCH_MP_TAC MOD_ADD_CASES THEN ASM_REWRITE_TAC[];
    ASM_REWRITE_TAC[]]);;

let ADD_MOD_GE = prove
 (`!p m n. m < p /\ n < p /\ p <= m + n ==> (m + n) MOD p = (m + n) - p`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  SUBGOAL_THEN `(m + n) MOD p = (if m + n < p then m + n else (m + n) - p)`
    (fun th -> REWRITE_TAC[th]) THENL
   [MATCH_MP_TAC MOD_ADD_CASES THEN ASM_REWRITE_TAC[];
    SUBGOAL_THEN `~(m + n < p)` ASSUME_TAC THENL [ASM_ARITH_TAC; ALL_TAC] THEN
    ASM_REWRITE_TAC[]]);;

(* `t = (m + n) - p' in sum form, so that the residue has no subtraction *)
let ADD_GE_SUM = prove
 (`!p m n t. p <= m + n ==> ((t = (m + n) - p) <=> (t + p = m + n))`,
  REPEAT GEN_TAC THEN DISCH_TAC THEN
  MP_TAC(ISPECL [`m + n:num`; `p:num`] SUB_ADD) THEN
  ANTS_TAC THENL [ASM_REWRITE_TAC[]; DISCH_TAC] THEN
  EQ_TAC THENL
   [DISCH_THEN SUBST1_TAC THEN ASM_REWRITE_TAC[];
    DISCH_TAC THEN UNDISCH_TAC `((m + n) - p) + p = m + n` THEN
    ASM_ARITH_TAC]);;

(* the same bridge, stated once in the shape a caller applies *)
let ADD_MOD_SPLIT = prove
 (`!p m n s t.
       0 < p /\ m < p /\ n < p /\
       (m + n < p ==> s = m + n) /\
       (p <= m + n ==> t + p = m + n)
       ==> (m + n) MOD p = (if m + n < p then s else t)`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  ASM_CASES_TAC `m + n < p` THENL
   [ASM_SIMP_TAC[ADD_MOD_LT] THEN ASM_ARITH_TAC;
    SUBGOAL_THEN `p <= m + n` ASSUME_TAC THENL [ASM_ARITH_TAC; ALL_TAC] THEN
    ASM_SIMP_TAC[ADD_MOD_GE] THEN CONV_TAC SYM_CONV THEN
    ASM_SIMP_TAC[ADD_GE_SUM] THEN ASM_ARITH_TAC]);;

(* sub: (&m - &n) rem &p, split at the borrow. *)
let SUB_REM_LT = prove
 (`!p m n. 0 < p /\ m < p /\ n < p /\ m < n
           ==> ((&m:int) - &n) rem (&p) = &((m + p) - n)`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  SUBGOAL_THEN `n <= m + p` ASSUME_TAC THENL [ASM_ARITH_TAC; ALL_TAC] THEN
  SUBGOAL_THEN `&((m + p) - n):int = (&m + &p) - &n` ASSUME_TAC THENL
   [ASM_SIMP_TAC[INT_OF_NUM_SUB; INT_OF_NUM_ADD]; ALL_TAC] THEN
  MATCH_MP_TAC INT_REM_UNIQ THEN EXISTS_TAC `--(&1):int` THEN
  REWRITE_TAC[INT_ABS_NUM] THEN REPEAT CONJ_TAC THENL
   [UNDISCH_TAC `&((m + p) - n):int = (&m + &p) - &n` THEN CONV_TAC INT_ARITH;
    REWRITE_TAC[INT_OF_NUM_LE] THEN ARITH_TAC;
    REWRITE_TAC[INT_OF_NUM_LT] THEN ASM_ARITH_TAC]);;

let SUB_REM_GE = prove
 (`!p m n. 0 < p /\ m < p /\ n < p /\ ~(m < n)
           ==> ((&m:int) - &n) rem (&p) = &(m - n)`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  SUBGOAL_THEN `n <= m` ASSUME_TAC THENL [ASM_ARITH_TAC; ALL_TAC] THEN
  SUBGOAL_THEN `&(m - n):int = &m - &n` ASSUME_TAC THENL
   [ASM_SIMP_TAC[INT_OF_NUM_SUB]; ALL_TAC] THEN
  MATCH_MP_TAC INT_REM_UNIQ THEN EXISTS_TAC `(&0):int` THEN
  REWRITE_TAC[INT_ABS_NUM] THEN REPEAT CONJ_TAC THENL
   [UNDISCH_TAC `&(m - n):int = &m - &n` THEN CONV_TAC INT_ARITH;
    REWRITE_TAC[INT_OF_NUM_LE] THEN ARITH_TAC;
    REWRITE_TAC[INT_OF_NUM_LT] THEN ASM_ARITH_TAC]);;

let SUB_LT_SUM = prove
 (`!p m n t. n < p /\ m < n ==> ((t = (m + p) - n) <=> (t + n = m + p))`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  SUBGOAL_THEN `n <= m + p` ASSUME_TAC THENL [ASM_ARITH_TAC; ALL_TAC] THEN
  MP_TAC(ISPECL [`m + p:num`; `n:num`] SUB_ADD) THEN
  ANTS_TAC THENL [ASM_REWRITE_TAC[]; DISCH_TAC] THEN
  EQ_TAC THENL
   [DISCH_THEN SUBST1_TAC THEN ASM_REWRITE_TAC[];
    DISCH_TAC THEN UNDISCH_TAC `((m + p) - n) + n = m + p` THEN
    ASM_ARITH_TAC]);;

let SUB_GE_SUM = prove
 (`!m n t. ~(m < n) ==> ((t = m - n) <=> (t + n = m))`,
  REPEAT GEN_TAC THEN DISCH_TAC THEN
  SUBGOAL_THEN `n <= m` ASSUME_TAC THENL [ASM_ARITH_TAC; ALL_TAC] THEN
  MP_TAC(ISPECL [`m:num`; `n:num`] SUB_ADD) THEN
  ANTS_TAC THENL [ASM_REWRITE_TAC[]; DISCH_TAC] THEN
  EQ_TAC THENL
   [DISCH_THEN SUBST1_TAC THEN ASM_REWRITE_TAC[];
    DISCH_TAC THEN UNDISCH_TAC `(m - n) + n = m` THEN ASM_ARITH_TAC]);;

let SUB_REM_SPLIT = prove
 (`!p m n s.
       0 < p /\ m < p /\ n < p /\
       (m < n ==> s + n = m + p) /\
       (~(m < n) ==> s + n = m)
       ==> ((&m:int) - &n) rem (&p) = &s`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  ASM_CASES_TAC `m < n` THENL
   [ASM_SIMP_TAC[SUB_REM_LT] THEN REWRITE_TAC[INT_OF_NUM_EQ] THEN
    CONV_TAC SYM_CONV THEN ASM_SIMP_TAC[SUB_LT_SUM];
    ASM_SIMP_TAC[SUB_REM_GE] THEN REWRITE_TAC[INT_OF_NUM_EQ] THEN
    CONV_TAC SYM_CONV THEN ASM_SIMP_TAC[SUB_GE_SUM]]);;

(* neg: (p - n) MOD p, split at n = 0. *)
let NEG_MOD_ZERO = prove
 (`!p n. 0 < p /\ n = 0 ==> (p - n) MOD p = 0`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN ASM_REWRITE_TAC[SUB_0; MOD_REFL]);;

let NEG_MOD_NZ = prove
 (`!p n. 0 < p /\ n <= p /\ ~(n = 0) ==> (p - n) MOD p = (p - n)`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN MATCH_MP_TAC MOD_LT THEN ASM_ARITH_TAC);;

let NEG_NZ_SUM = prove
 (`!p n t. n <= p ==> ((t = p - n) <=> (t + n = p))`,
  REPEAT GEN_TAC THEN DISCH_TAC THEN
  MP_TAC(ISPECL [`p:num`; `n:num`] SUB_ADD) THEN
  ANTS_TAC THENL [ASM_REWRITE_TAC[]; DISCH_TAC] THEN
  EQ_TAC THENL
   [DISCH_THEN SUBST1_TAC THEN ASM_REWRITE_TAC[];
    DISCH_TAC THEN UNDISCH_TAC `(p - n) + n = p` THEN ASM_ARITH_TAC]);;

let NEG_MOD_SPLIT = prove
 (`!p n s.
       0 < p /\ n <= p /\
       (n = 0 ==> s = 0) /\
       (~(n = 0) ==> s + n = p)
       ==> (p - n) MOD p = s`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  ASM_CASES_TAC `n = 0` THENL
   [ASM_SIMP_TAC[NEG_MOD_ZERO] THEN ASM_ARITH_TAC;
    ASM_SIMP_TAC[NEG_MOD_NZ] THEN CONV_TAC SYM_CONV THEN
    ASM_SIMP_TAC[NEG_NZ_SUM] THEN ASM_ARITH_TAC]);;

let BRIDGE_SPEC_LEMMAS =
  [ADD_MOD_LT; ADD_MOD_GE; ADD_GE_SUM; ADD_MOD_SPLIT;
   SUB_REM_LT; SUB_REM_GE; SUB_LT_SUM; SUB_GE_SUM; SUB_REM_SPLIT;
   NEG_MOD_ZERO; NEG_MOD_NZ; NEG_NZ_SUM; NEG_MOD_SPLIT];;

(* ------------------------------------------------------------------------- *)
(* 1b. The reference function.                                               *)
(*                                                                           *)
(* `ref_add pw x y' is the whole modular specification of addition in one     *)
(* word operation: add, and subtract pw once if the sum overflowed or        *)
(* reached pw.  There is no curve constant here and no MOD: the single       *)
(* generic lemma ADD_REF_VAL carries the MOD, so every per-curve SAT         *)
(* instance is a pure word statement (`output = ref_add (word p) x y`) with  *)
(* `x < p' and `y < p' as bit-level hypotheses.                              *)
(* ------------------------------------------------------------------------- *)

let ref_add = new_definition
 `ref_add (pw:W word) (x:W word) (y:W word) =
    let s = word_add x y in
    if word_ult s x \/ word_ule pw s then word_sub s pw else s`;;

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

(* Case: the sum overflowed or reached pw, so the machine subtracts. *)
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

(* Case: neither, so the machine keeps the sum. *)
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

(* The one lemma that carries MOD; proved for every width and every pw. *)
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
  REWRITE_TAC[ref_add; LET_DEF; LET_END_DEF] THEN BETA_TAC THEN
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

(* the same statement for a modulus supplied as a numeral *)
let ADD_REF_VAL_WORD = prove
 (`!p (x:W word) (y:W word).
       0 < p /\ p < 2 EXP dimindex(:W) /\ val x < p /\ val y < p
       ==> val(ref_add (word p : W word) x y) = (val x + val y) MOD p`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  SUBGOAL_THEN `val(word p : W word) = p` ASSUME_TAC THENL
   [REWRITE_TAC[VAL_WORD] THEN MATCH_MP_TAC MOD_LT THEN ASM_REWRITE_TAC[];
    ALL_TAC] THEN
  (* instantiate ADD_REF_VAL at pw = word p, then rewrite val(word p) = p in
     the theorem so its conclusion is syntactically the goal. *)
  FIRST_ASSUM(fun th ->
    MATCH_MP_TAC (REWRITE_RULE[th]
      (SPECL [`(word p : W word)`; `(x:W word)`; `(y:W word)`] ADD_REF_VAL))) THEN
  ASM_REWRITE_TAC[]);;

let BRIDGE_REF_LEMMAS = [ADD_REF_VAL; ADD_REF_VAL_WORD];;
let BRIDGE_SPEC_LEMMAS = BRIDGE_SPEC_LEMMAS @ BRIDGE_REF_LEMMAS;;

(* ------------------------------------------------------------------------- *)
(* 2. Power-of-two scaling as a word join.                                   *)
(*                                                                           *)
(* `word_join' is free under bit-blasting (BIT_WORD_JOIN); a product by a    *)
(* power of two is instead expanded by WORDIFY_CONV into a full multiplier,  *)
(* which dominates the SAT cost.  Rewriting the shifts to joins is what      *)
(* makes the residue cheap, and it is also what removes the unconstrained-   *)
(* width problem: `word_join' has a free result type, so the width must be   *)
(* supplied here (`mk_word_ty').                                            *)
(* ------------------------------------------------------------------------- *)

let EXP2_NZ = prove
 (`!j:num. ~(2 EXP j = 0)`,
  GEN_TAC THEN REWRITE_TAC[EXP_EQ_0] THEN ARITH_TAC);;

let LT_EXP_MULT = prove
 (`!x j k:num. x < 2 EXP k ==> 2 EXP j * x < 2 EXP (j + k)`,
  REPEAT GEN_TAC THEN DISCH_TAC THEN REWRITE_TAC[EXP_ADD] THEN
  REWRITE_TAC[LT_MULT_LCANCEL] THEN
  REWRITE_TAC[EXP_EQ_0; ARITH] THEN ASM_REWRITE_TAC[]);;

(* val(join x 0) = 2^w * val x, i.e. shifting left by a zero word. *)
let VAL_JOIN_ZERO = prove
 (`!x:M word. dimindex(:M) + dimindex(:N) = dimindex(:P)
              ==> val((word_join:M word->N word->P word) x (word 0)) =
                  2 EXP dimindex(:N) * val x`,
  REPEAT STRIP_TAC THEN
  ASM_SIMP_TAC[VAL_WORD_JOIN_SIMPLE; VAL_WORD_0; ADD_0]);;

(* the same statement one type level up, in the shape the residue has *)
let INT_JOIN_ZERO = prove
 (`!x:M word. dimindex(:M) + dimindex(:N) = dimindex(:P)
              ==> (&2:int) pow dimindex(:N) * &(val x) =
                  &(val((word_join:M word->N word->P word) x (word 0)))`,
  REPEAT STRIP_TAC THEN
  ASM_SIMP_TAC[VAL_JOIN_ZERO; GSYM INT_OF_NUM_MUL; INT_OF_NUM_POW]);;

let BITVAL_AS_VAL = prove
 (`!b. bitval b = val(word(bitval b):1 word)`,
  GEN_TAC THEN CONV_TAC SYM_CONV THEN
  MATCH_ACCEPT_TAC (INST_TYPE [`:1`, `:N`] VAL_WORD_BITVAL));;

(* ------------------------------------------------------------------------- *)
(* 3. Conversions.                                                           *)
(* ------------------------------------------------------------------------- *)

(* real equation over integer-valued atoms -> integer equation *)
let REAL_TO_INT_CONV =
  GEN_REWRITE_CONV DEPTH_CONV
   [GSYM int_of_num_th; GSYM int_neg_th; GSYM int_add_th; GSYM int_sub_th;
    GSYM int_mul_th; GSYM int_pow_th; GSYM int_eq];;

(* num equation -> int equation, with `&' pushed inside sum/product/power.    *)
(* INT_OF_NUM_ADD/MUL/POW are stated with `&' *outside* (`&m + &n = &(m + n)`), *)
(* so they must be used in the GSYM direction here; using them forwards folds  *)
(* the whole bignum back into a single `&(...)', which the shift-to-join       *)
(* conversion cannot see (measured: every post-simulation stage stayed SAT).   *)
let NUM_TO_INT_CONV =
  GEN_REWRITE_CONV TOP_DEPTH_CONV
   [GSYM INT_OF_NUM_EQ; GSYM INT_OF_NUM_LT; GSYM INT_OF_NUM_LE;
    GSYM INT_OF_NUM_ADD; GSYM INT_OF_NUM_MUL; GSYM INT_OF_NUM_POW];;

(* (a + b) * c = a * c + b * c, needed because bignum_of_wordlist nests     *)
(* its shifts around a sum, not around a single digit.                       *)
let INT_DISTRIB_CONV =
  GEN_REWRITE_CONV TOP_DEPTH_CONV [INT_ADD_LDISTRIB; INT_ADD_RDISTRIB];;

(* (&2) pow k * &(val x) -> &(val(join x 0)) at the width this x forces.     *)
(* Bottom-up application gives 2^(64*(j+1)) for j nested shifts for free.     *)
let INT_JOIN_ZERO_TH x k =
  (* `dest_word_ty' is shadowed by Library/bitmatch.ml once the ARM base is
     loaded, so the width is read off the type constructor directly. *)
  let m = Num.int_of_num
    (dest_finty(match type_of x with
                | Tyapp("word",[n]) -> n
                | _ -> failwith "INT_JOIN_ZERO_TH: not a word")) in
  let inst = INST_TYPE
    [mk_finty(Num.num_of_int m), `:M`;
     mk_finty(Num.num_of_int k), `:N`;
     mk_finty(Num.num_of_int(m + k)), `:P`] INT_JOIN_ZERO in
  let th1 = SPEC x inst in
  let cond = lhand(concl th1) in
  let side =
    prove(cond,
      CONV_TAC(LAND_CONV(BINOP_CONV DIMINDEX_CONV THENC NUM_ADD_CONV) THENC
               RAND_CONV DIMINDEX_CONV THENC NUM_EQ_CONV)) in
  (* reduce dimindex(:N) on the left so the rule is syntactically applicable.  *)
  (* The left side of the equation is `&2 pow dimindex(:N) * &(val x)'; the     *)
  (* exponent is the *left* operand of that product, so the path is            *)
  (* LAND (LAND (RAND DIMINDEX)).  The old single LAND reached `&(val x)' and  *)
  (* raised DIMINDEX_CONV, which DEPTH_CONV silently swallowed, leaving every  *)
  (* shift unconverted and every SAT call SATISFIABLE.                        *)
  CONV_RULE (LAND_CONV(LAND_CONV(RAND_CONV DIMINDEX_CONV))) (MP th1 side);;

(* smallest b with n < 2^b, at least 1; used to place a literal operand in a
   word narrow enough that `val(word n)' is exact. *)
let bits_of_num n =
  let two = Num.num_of_int 2 and one = Num.num_of_int 1 in
  let rec go b m = if Num.le_num m n then go (b + 1) (Num.mult_num two m) else b in
  let b = go 0 one in
  Num.num_of_int (if b < 1 then 1 else b);;

let INT_SHIFT_TO_JOIN_CONV : conv =
  (* the second operand is `&(val x)', `&(bitval b)', or a *numeral*.  The
     numeral case matters: BRIDGE_SPLIT_CONDITIONS_TAC substitutes a carry by
     the literal 0 or 1, turning `&2 pow 64 * &(bitval c)' into
     `&2 pow 64 * &0'.  Without this case that equation could not be wordified
     at all, BITBLAST left it opaque, and the selected limb was free. *)
  let operand q =
    match q with
      Comb(Const("int_of_num",_),Comb(Const("val",_),x)) -> (x, REFL q)
    | Comb(Const("int_of_num",_),Comb(Const("bitval",_),_)) ->
        let bth = RAND_CONV (GEN_REWRITE_CONV I [BITVAL_AS_VAL]) q in
        (rand(rand(rand(concl bth))), bth)
    | Comb(Const("int_of_num",_),n) when is_numeral n ->
        let bits = bits_of_num (dest_numeral n) in
        let w = mk_comb(mk_const("word",[mk_finty bits, `:N`]), n)
        and v = mk_comb(mk_const("val",[mk_finty bits, `:N`]),
                        mk_comb(mk_const("word",[mk_finty bits, `:N`]), n)) in
        let val_th = prove(mk_eq(v,n),
          REWRITE_TAC[VAL_WORD] THEN
          CONV_TAC(LAND_CONV(RAND_CONV(RAND_CONV DIMINDEX_CONV))) THEN
          MATCH_MP_TAC MOD_LT THEN
          CONV_TAC(RAND_CONV NUM_EXP_CONV THENC NUM_LT_CONV)) in
        (w, AP_TERM `int_of_num:num->int` (SYM val_th))
    | _ -> failwith "INT_SHIFT_TO_JOIN_CONV: operand" in
  let pow2 tm =
    match tm with
      Comb(Comb(Const("int_pow",_),two),n)
        when aconv two `(&2):int` && is_numeral n -> dest_small_numeral n
    | _ -> failwith "INT_SHIFT_TO_JOIN_CONV: multiplier" in
  let mul_left p th =
    AP_TERM (mk_comb(`int_mul:int->int->int`, p)) th in
  let conv tm =
    match tm with
      Comb(Comb(Const("int_mul",_),p),q) when type_of tm = `:int` ->
        (match p with
           Comb(Comb(Const("int_pow",_),_),_) ->
             let k = pow2 p in
             let (x,bth) = operand q in
             TRANS (mul_left p bth) (INT_JOIN_ZERO_TH x k)
         | Comb(Const("int_neg",_),pt) ->
             (* accumulator equations carry the shifted flag negatively       *)
             let k = pow2 pt in
             let (x,bth) = operand q in
             let q' = rand(concl bth) in
             let bth2 = mul_left p bth in
             let negth = AP_TERM `(int_neg):int->int` (INT_JOIN_ZERO_TH x k) in
             TRANS bth2 (TRANS (SPECL [pt; q'] INT_MUL_LNEG) negth)
         | _ -> failwith "INT_SHIFT_TO_JOIN_CONV: multiplier") 
    | _ -> failwith "INT_SHIFT_TO_JOIN_CONV" in
  conv;;

let BRIDGE_SHIFTS_TO_JOINS_CONV = DEPTH_CONV INT_SHIFT_TO_JOIN_CONV;;

(* ------------------------------------------------------------------------- *)
(* 2b. A bignum digit sum as one word.                                       *)
(*                                                                           *)
(* `bignum_of_wordlist' and BIGNUM_DIGITIZE both hand back a flat sum        *)
(*                                                                           *)
(*   val w0 + 2^k * val w1 + 2^(2k) * val w2 + ... + 2^(k*(n-1)) * val w{n-1} *)
(*                                                                           *)
(* over words w0..w{n-1} of one limb width k.  The reference lemmas are      *)
(* stated about ONE word, so the sum has to become a word.  Folding the      *)
(* digits from the most significant end through `word_join' does that:       *)
(*                                                                           *)
(*   val(w1)                  = val w1                                        *)
(*   val(join(acc) w0)        = 2^k * val acc + val w0                       *)
(*                                                                           *)
(* by VAL_WORD_JOIN_SIMPLE, whose only side condition is the dimindex one.   *)
(* The limb width k is read off the words and the limb count from the sum,   *)
(* so nothing here assumes 64 bits or four limbs.                            *)
(* ------------------------------------------------------------------------- *)

let bridge_word_width ty =
  match ty with
  | Tyapp("word",[n]) -> Num.int_of_num (dest_finty n)
  | _ -> failwith "bridge_word_width: not a word type";;

let bridge_finty k = mk_finty (Num.num_of_int k);;

(* word_join with the result width made explicit: it is a type argument, not  *)
(* something HOL infers, so it must be supplied (the same reason             *)
(* INT_JOIN_ZERO_TH supplies `:P').                                          *)
let BRIDGE_WJOIN (x:term) (y:term) =
  let mx = bridge_word_width (type_of x)
  and my = bridge_word_width (type_of y) in
  mk_comb(mk_comb(mk_const("word_join",
      [bridge_finty mx, `:M`; bridge_finty my, `:N`;
       bridge_finty (mx + my), `:P`]), x), y);;

(* |- val(join x y) = 2 EXP k * val x + val y, with k = width of y reduced  *)
let BRIDGE_VAL_JOIN_STEP (x:term) (y:term) =
  let mx = bridge_word_width (type_of x)
  and my = bridge_word_width (type_of y) in
  let inst = INST_TYPE [bridge_finty mx, `:M`; bridge_finty my, `:N`;
                        bridge_finty (mx + my), `:P`] VAL_WORD_JOIN_SIMPLE in
  let th = SPECL [x; y] inst in
  let side = prove(lhand (concl th),
      CONV_TAC(LAND_CONV(BINOP_CONV DIMINDEX_CONV THENC NUM_ADD_CONV) THENC
               RAND_CONV DIMINDEX_CONV THENC NUM_EQ_CONV)) in
  CONV_RULE (RAND_CONV(LAND_CONV(LAND_CONV(RAND_CONV DIMINDEX_CONV))))
            (MP th side);;

(* the little-endian word list of a flat digit sum, in either associativity  *)
let BRIDGE_DIGIT_TERMS tm =
  let rec addends t =
    match t with
    | Comb(Comb(Const("+",_),_),_) -> addends (lhand t) @ addends (rand t)
    | _ -> [t] in
  let dest_val t =
    match t with
    | Comb(Const("val",_),w) -> w
    | _ -> failwith "BRIDGE_DIGIT_TERMS: not `val w'" in
  let dest_pow t =
    match t with
    | Comb(Comb(Const("EXP",_),two),n) when aconv two `2` ->
        (ignore (dest_small_numeral n); ())
    | _ -> failwith "BRIDGE_DIGIT_TERMS: not `2 EXP k'" in
  match addends tm with
  | [] -> failwith "BRIDGE_DIGIT_TERMS: empty"
  | first :: rest ->
      let w0 = dest_val first in
      let ws = map (fun t ->
          match t with
          | Comb(Comb(Const("*",_),p),v) -> (dest_pow p; dest_val v)
          | _ -> failwith "BRIDGE_DIGIT_TERMS: not `2^k * val w'") rest in
      w0 :: ws;;

let bridge_join_of_words ws =
  match List.rev ws with
  | [] -> failwith "bridge_join_of_words: empty"
  | last :: rest ->
      List.fold_left (fun acc w -> BRIDGE_WJOIN acc w) last rest;;

(* |- val(join of ws) = flat                                                      *)
let BRIDGE_VAL_JOIN_FOLD (ws:term list) (flat:term) =
  let j = bridge_join_of_words ws in
  let rec steps = function
    | [] | [_] -> []
    | w :: (_ :: _ as rest) ->
        BRIDGE_VAL_JOIN_STEP (bridge_join_of_words rest) w :: steps rest in
  let sts = steps ws in
  prove(mk_eq(mk_comb(mk_const("val",
      [bridge_finty (bridge_word_width (type_of j)), `:N`]), j), flat),
    REWRITE_TAC sts THEN CONV_TAC NUM_REDUCE_CONV THEN ARITH_TAC);;

(* (the word, |- val word = flat sum)                                            *)
let BRIDGE_WORD_OF_DIGITS tm =
  let ws = BRIDGE_DIGIT_TERMS tm in
  (bridge_join_of_words ws, BRIDGE_VAL_JOIN_FOLD ws tm);;

(* whole-term conversion from any of the three spec types to a word residue *)
let BRIDGE_TO_WORD_CONV =
  REAL_TO_INT_CONV THENC NUM_TO_INT_CONV THENC INT_DISTRIB_CONV THENC
  BRIDGE_SHIFTS_TO_JOINS_CONV;;

(* ------------------------------------------------------------------------- *)
(* 4. Case splitting on the conditionals the simulator leaves behind.        *)
(*                                                                           *)
(* The conditions may sit in the goal (the selected limb of the written      *)
(* value) or in an accumulator equation (a selected mask word).  Both are    *)
(* reduced by the same recursive split, and the condition itself stays as a *)
(* bit-blastable hypothesis, so SAT can see that the branch is consistent.   *)
(* ------------------------------------------------------------------------- *)

let bridge_conditions tm =
  map (fun t -> rand(rator(rator t))) (find_terms is_cond tm);;

let rec BRIDGE_SPLIT_CONDITIONS_TAC (asl,w) =
  let conds = bridge_conditions w @
              List.concat (map (fun (_,th) -> bridge_conditions(concl th)) asl) in
  match conds with
  | [] -> ALL_TAC (asl,w)
  | c::_ ->
      let truerule = ASSUME c
      and falserule = EQF_INTRO (ASSUME (mk_neg c)) in
      let close rule =
        RULE_ASSUM_TAC(REWRITE_RULE[rule; COND_CLAUSES; BITVAL_CLAUSES]) THEN
        CONV_TAC(REWRITE_CONV[rule; COND_CLAUSES; BITVAL_CLAUSES]) THEN
        BRIDGE_SPLIT_CONDITIONS_TAC in
      (ASM_CASES_TAC c THENL [close truerule; close falserule]) (asl,w);;

(* The same split, but the branch condition is *retained* as a hypothesis.
   REWRITE_RULE reads a non-equational theorem `|- c' as the rule `c -> T', so
   the version above rewrites the case hypothesis itself to `true' and the
   branch information is lost from the assumptions (it survives only where a
   lemma restates it arithmetically, as ADD_MOD_LT / SUB_LT_SUM do).  A route
   that carries the condition in the word goal -- the reference route's
   `ref_add' conditional -- needs the fact itself in the SAT instance. *)
let rec BRIDGE_SPLIT_CONDITIONS_KEEP_TAC (asl,w) =
  let conds = bridge_conditions w @
              List.concat (map (fun (_,th) -> bridge_conditions(concl th)) asl) in
  match conds with
  | [] -> ALL_TAC (asl,w)
  | c::_ ->
      let close rule fact =
        RULE_ASSUM_TAC(REWRITE_RULE[rule; COND_CLAUSES; BITVAL_CLAUSES]) THEN
        CONV_TAC(REWRITE_CONV[rule; COND_CLAUSES; BITVAL_CLAUSES]) THEN
        ASSUME_TAC fact THEN
        BRIDGE_SPLIT_CONDITIONS_KEEP_TAC in
      (ASM_CASES_TAC c THENL
        [close (ASSUME c) (ASSUME c);
         close (EQF_INTRO (ASSUME (mk_neg c))) (ASSUME (mk_neg c))])
      (asl,w);;

(* ------------------------------------------------------------------------- *)
(* 5. Eliminate the specification variables from their digit expansions.     *)
(* ------------------------------------------------------------------------- *)

let BRIDGE_HAS_VAL tm =
  mem "val" (map (fst o dest_const)
    (find_terms (fun t -> can (fst o dest_const) t) tm));;

let rec ELIM_DIGIT_EQS_TAC (asl,w) =
  let pick = ref None in
  do_list (fun (_,th) ->
    let tm = concl th in
    if is_eq tm && type_of (lhand tm) = `:num` && not (is_var tm) then
      let (l,r) = dest_eq tm in
      if is_var l && free_in l w && not (free_in l r) then pick := Some th
      else if is_var r && free_in r w && not (free_in r l) then
        pick := Some (SYM th)) asl;
  match !pick with
  | None -> ALL_TAC (asl,w)
  | Some th -> (SUBST_ALL_TAC th THEN ELIM_DIGIT_EQS_TAC) (asl,w);;

(* ------------------------------------------------------------------------- *)
(* 6. Certified-SAT close of the word residue.                               *)
(*                                                                           *)
(* BITBLAST_THEN consumes only word-level content; any atom it cannot expand *)
(* becomes a free proposition and the residue comes back SAT, so everything  *)
(* that is not word-level must be gone before the call.  The residue is      *)
(* presented as one closed implication so that no hypothesis is lost.        *)
(* ------------------------------------------------------------------------- *)

let BRIDGE_CSAT_TAC : tactic =
  fun (asl,w) ->
    let hths = map snd asl in
    let hyps = map concl hths in
    let imp = itlist (fun a b -> mk_imp(a,b)) hyps w in
    let th = prove(imp, CSAT_TAC) in
    (* The old tail was `MP_TAC th THEN ASM_REWRITE_TAC[]'.  That rewrites the
       antecedents away but leaves `w` itself as the goal, so the tactic
       reported success while never closing anything (measured: the four SAT
       refutations all succeeded and four goals remained).  Chaining MP with
       the assumption theorems yields `|- w` directly. *)
    let th' = List.fold_left (fun acc hth -> MP acc hth) th hths in
    ACCEPT_TAC th' (asl,w);;

(* everything that precedes the SAT call, on the goal *)
let BRIDGE_PREPARE_TAC (expand:tactic) : tactic =
  fun g ->
    (BRIDGE_SPLIT_CONDITIONS_TAC THEN
     expand THEN
     RULE_ASSUM_TAC (CONV_RULE BRIDGE_TO_WORD_CONV) THEN
     CONV_TAC BRIDGE_TO_WORD_CONV) g;;

let BRIDGE_FINISH_TAC expand : tactic =
  fun g -> (BRIDGE_PREPARE_TAC expand THEN BRIDGE_CSAT_TAC) g;;

(* the same, keeping every branch condition as a hypothesis *)
let BRIDGE_FINISH_KEEP_TAC expand : tactic =
  fun g ->
    (BRIDGE_SPLIT_CONDITIONS_KEEP_TAC THEN
     expand THEN
     RULE_ASSUM_TAC (CONV_RULE BRIDGE_TO_WORD_CONV) THEN
     CONV_TAC BRIDGE_TO_WORD_CONV THEN
     BRIDGE_CSAT_TAC) g;;

(* ------------------------------------------------------------------------- *)
(* 7. Entry points.  `pth' is the definition theorem of the modulus, so the  *)
(* numeral replaces the constant before anything is decided.                 *)
(* ------------------------------------------------------------------------- *)

let BRIDGE_ZERO_LT_TAC p : tactic =
  SUBGOAL_THEN (mk_comb(mk_comb(`(<):num->num->bool`, `0`), p))
   (fun th -> ASSUME_TAC th) THENL [ARITH_TAC; ALL_TAC];;

(* the modulus constant has to be replaced in the assumptions as well as in   *)
(* the goal: the range hypotheses are stated about the constant.            *)
let BRIDGE_REWRITE_MODULUS pth : tactic =
  RULE_ASSUM_TAC(REWRITE_RULE[pth]) THEN REWRITE_TAC[pth];;

(* add: the spec is `(m + n) MOD p = <written value>' *)
let BRIDGE_ADD_TAC expand pth : tactic =
  ELIM_DIGIT_EQS_TAC THEN BRIDGE_REWRITE_MODULUS pth THEN
  (fun (asl,w) ->
    let (lhs,_) = dest_eq w in
    let (sum,p) = dest_binop `(MOD):num->num->num` lhs in
    let cond = mk_comb(mk_comb(`(<):num->num->bool`, sum), p) in
    (BRIDGE_ZERO_LT_TAC p THEN
     ASM_CASES_TAC cond THENL
      [ASM_SIMP_TAC[ADD_MOD_LT] THEN BRIDGE_FINISH_TAC expand;
       SUBGOAL_THEN (mk_comb(mk_comb(`(<=):num->num->bool`, p), sum))
         ASSUME_TAC THENL [ASM_ARITH_TAC; ALL_TAC] THEN
       ASM_SIMP_TAC[ADD_MOD_GE] THEN CONV_TAC SYM_CONV THEN
       ASM_SIMP_TAC[ADD_GE_SUM] THEN BRIDGE_FINISH_TAC expand]) (asl,w));;

(* add, through the reference function.  The split above decides `(m+n) MOD p'
   by cases on the modulus; this route carries the MOD inside ADD_REF_VAL_WORD
   instead, so the only per-curve obligation is a pure word equation

     val(ref_add (word p) x y) = val w

   with `x < p' and `y < p' among the bit-level hypotheses.  `expand' is the
   caller's bignum expansion (BIGNUM_OF_WORDLIST_CONV), applied first so the
   written value is a flat digit sum we can fold into one word. *)
let BRIDGE_ADD_REF_TAC expand pth : tactic =
  ELIM_DIGIT_EQS_TAC THEN BRIDGE_REWRITE_MODULUS pth THEN expand THEN
  (fun (asl,w) ->
    let (lhs,wv) = dest_eq w in
    let (sum,p) = dest_binop `(MOD):num->num->num` lhs in
    let (bx,by) = dest_binop `(+):num->num->num` sum in
    let (xw,thx) = BRIDGE_WORD_OF_DIGITS bx in
    let (yw,thy) = BRIDGE_WORD_OF_DIGITS by in
    let (ww,thw) = BRIDGE_WORD_OF_DIGITS wv in
    (* the bounds are ambient hypotheses, stated about the digit sums *)
    let lt tm = mk_comb(mk_comb(`(<):num->num->bool`, tm), p) in
    let bound tm =
      try snd(List.find (fun (_,th) -> aconv (concl th) (lt tm)) asl)
      with Not_found ->
        failwith "BRIDGE_ADD_REF_TAC: missing `x < p' hypothesis" in
    let hltx = REWRITE_RULE[GSYM thx] (bound bx)
    and hlty = REWRITE_RULE[GSYM thy] (bound by) in
    (* ADD_REF_VAL_WORD at the operand width, with the two bounds discharged *)
    let wt = bridge_finty (bridge_word_width (type_of xw)) in
    let lem = SPECL [p; xw; yw] (INST_TYPE [wt,`:W`] ADD_REF_VAL_WORD) in
    let ants = conjuncts (lhand (concl lem)) in
    if List.length ants <> 4 then
      failwith "BRIDGE_ADD_REF_TAC: unexpected antecedent shape";
    let s1 = prove(List.nth ants 0, CONV_TAC NUM_LT_CONV) in
    let s2 = prove(List.nth ants 1,
      CONV_TAC(RAND_CONV(RAND_CONV DIMINDEX_CONV) THENC
               RAND_CONV NUM_REDUCE_CONV THENC NUM_LT_CONV)) in
    (* the two bounds stay as hypotheses of the rewrite: they are exactly the
       ambient `val x < p' / `val y < p' assumptions, and ASM_REWRITE_TAC
       below discharges the subgoals they create *)
    let rth = MP lem (end_itlist CONJ [s1; s2; hltx; hlty]) in
    let rth = SYM (REWRITE_RULE[thx; thy] rth) in
    (ASM_REWRITE_TAC[rth; SYM thw; ref_add; LET_DEF; LET_END_DEF] THEN
     BRIDGE_FINISH_KEEP_TAC expand) (asl,w));;

(* sub: the spec is `&<written value> = (&m - &n) rem &p' *)
let BRIDGE_SUB_TAC expand pth : tactic =
  ELIM_DIGIT_EQS_TAC THEN BRIDGE_REWRITE_MODULUS pth THEN
  (fun (asl,w) ->
    let (lhs,rhs) = dest_eq w in
    let (diff,p) = dest_binop `(rem):int->int->int` rhs in
    let (m,n) = dest_binop `(-):int->int->int` diff in
    let cond = mk_comb(mk_comb(`(<):num->num->bool`, rand m), rand n) in
    (BRIDGE_ZERO_LT_TAC (rand p) THEN
     ASM_CASES_TAC cond THENL
      [ASM_SIMP_TAC[SUB_REM_LT] THEN REWRITE_TAC[INT_OF_NUM_EQ] THEN
       ASM_SIMP_TAC[SUB_LT_SUM] THEN BRIDGE_FINISH_TAC expand;
       ASM_SIMP_TAC[SUB_REM_GE] THEN REWRITE_TAC[INT_OF_NUM_EQ] THEN
       ASM_SIMP_TAC[SUB_GE_SUM] THEN BRIDGE_FINISH_TAC expand])
    (asl,w));;

(* neg: the spec is `&<written value> = &((p - n) MOD p)' *)
let BRIDGE_NEG_TAC expand pth : tactic =
  ELIM_DIGIT_EQS_TAC THEN BRIDGE_REWRITE_MODULUS pth THEN
  CONV_TAC REAL_TO_INT_CONV THEN
  (fun (asl,w) ->
    let (_,rhs) = dest_eq w in
    let modtm = rand rhs in
    let (diff,p) = dest_binop `(MOD):num->num->num` modtm in
    let n = rand diff in
    let cond = mk_comb(mk_comb(`(=):num->num->bool`, n), `0`) in
    (BRIDGE_ZERO_LT_TAC p THEN
     ASM_CASES_TAC cond THENL
      [ASM_SIMP_TAC[NEG_MOD_ZERO] THEN BRIDGE_FINISH_KEEP_TAC expand;
       ASM_SIMP_TAC[NEG_MOD_NZ] THEN
       (* NEG_MOD_NZ leaves `&(p - n)' with `&' applied to a *num*
          subtraction.  That atom cannot be wordified (BITBLAST leaves it
          opaque and the equation comes back SAT, measured 24.728 s), and
          INT_OF_NUM_SUB rewrites the wrong way -- its statement is
          `&n - &m = &(n - m)'.  GSYM is the direction that exposes the
          subtraction to the conversion, with `n <= p' the ambient bound. *)
       ASM_SIMP_TAC[GSYM INT_OF_NUM_SUB] THEN
       BRIDGE_FINISH_KEEP_TAC expand])
    (asl,w));;

(* ========================================================================= *)
(* 8. Reference route for `sub` and `neg`.                                   *)
(*                                                                           *)
(* Same shape as `ref_add': the whole modular specification of the operation *)
(* is one word function, and a single generic lemma carries the MOD (or the  *)
(* int `rem'), so every per-task SAT instance is a pure word equation        *)
(*                                                                           *)
(*   val(<written words>) = val(ref_sub (word p) x y)                        *)
(*   val(<written words>) = val(ref_neg (word p) x)                          *)
(*                                                                           *)
(* with `val x < p', `val y < p' (`val x <= p' for neg) among the hypotheses *)
(* and the machine's own accumulator equations (including the flag word's    *)
(* defining equation) among the assumptions.  No flag *meaning* is derived,  *)
(* so no curve-specific comparison has to be proved in real arithmetic: the  *)
(* hardware carry and the reference's word comparison are related inside the *)
(* bit-level instance.                                                       *)
(*                                                                           *)
(* Note on HOL Light parsing: `-` binds tighter than `+', so `a + p - b' is  *)
(* `a + (p - b)'.  Every statement below is written in that form on purpose; *)
(* SUB_REARRANGE is the bridge between it and the `(a + p) - b' form the     *)
(* subtraction lemmas produce.                                               *)
(* ========================================================================= *)

(* sub: subtract, and add pw back exactly when the subtraction borrowed.    *)
let ref_sub = new_definition
 `ref_sub (pw:W word) (x:W word) (y:W word) =
    let d = word_sub x y in
    if word_ult x y then word_add d pw else d`;;

let MOD_ADD_SELF = prove
 (`!a p:num. (a + p) MOD p = a MOD p`,
  REPEAT GEN_TAC THEN MATCH_MP_TAC MOD_EQ THEN
  EXISTS_TAC `1` THEN REWRITE_TAC[MULT_CLAUSES]);;

let SUB_REARRANGE = prove
 (`!m n p:num. n <= p ==> (m + p) - n = m + (p - n)`,
  REPEAT GEN_TAC THEN DISCH_TAC THEN ASM_ARITH_TAC);;

(* without borrow the machine keeps x - y, which is already the residue *)
let REF_SUB_KEEP = prove
 (`!a b p:num. b <= a /\ a < p ==> a - b = (a + p - b) MOD p`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  SUBGOAL_THEN `(a + p - b) = (a - b) + p` SUBST1_TAC THENL
   [ASM_ARITH_TAC; ALL_TAC] THEN
  REWRITE_TAC[MOD_ADD_SELF] THEN
  CONV_TAC SYM_CONV THEN MATCH_MP_TAC MOD_LT THEN ASM_ARITH_TAC);;

(* with borrow the machine writes (x - y mod 2^k) + p, which wraps to x+p-y *)
let REF_SUB_BORROW = prove
 (`!a b p t s:num.
     a < b /\ b < p /\ p < t /\ s = a + (t - b)
     ==> (s + p) MOD t = (a + p - b) MOD p`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  SUBGOAL_THEN `(s + p) = (a + p - b) + t` SUBST1_TAC THENL
   [ASM_ARITH_TAC; ALL_TAC] THEN
  REWRITE_TAC[MOD_ADD_SELF] THEN
  SUBGOAL_THEN `(a + p - b) < p` ASSUME_TAC THENL [ASM_ARITH_TAC; ALL_TAC] THEN
  SUBGOAL_THEN `(a + p - b) < t` ASSUME_TAC THENL [ASM_ARITH_TAC; ALL_TAC] THEN
  ASM_SIMP_TAC[MOD_LT]);;

(* The one lemma that carries MOD for subtraction; every width, every pw. *)
let SUB_REF_VAL = prove
 (`!(pw:W word) (x:W word) (y:W word).
       0 < val pw /\ val x < val pw /\ val y < val pw
       ==> val(ref_sub pw x y) = (val x + val pw - val y) MOD val pw`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  SUBGOAL_THEN
   `val(pw:W word) < 2 EXP dimindex(:W) /\
    val(x:W word) < 2 EXP dimindex(:W) /\
    val(y:W word) < 2 EXP dimindex(:W)`
    STRIP_ASSUME_TAC THENL [REWRITE_TAC[VAL_BOUND]; ALL_TAC] THEN
  REWRITE_TAC[ref_sub; LET_DEF; LET_END_DEF] THEN BETA_TAC THEN
  REWRITE_TAC[WORD_ULT] THEN
  ASM_CASES_TAC `val(x:W word) < val(y:W word)` THENL
   [ SUBGOAL_THEN `~(val(y:W word) <= val(x:W word))` ASSUME_TAC THENL
      [ASM_ARITH_TAC; ALL_TAC] THEN
     ASM_REWRITE_TAC[VAL_WORD_ADD] THEN
     MATCH_MP_TAC REF_SUB_BORROW THEN REPEAT CONJ_TAC THENL
      [ ASM_REWRITE_TAC[]; ASM_REWRITE_TAC[]; ASM_REWRITE_TAC[];
        ASM_SIMP_TAC[VAL_WORD_SUB_CASES] ];
     SUBGOAL_THEN `val(y:W word) <= val(x:W word)` ASSUME_TAC THENL
      [ASM_ARITH_TAC; ALL_TAC] THEN
     ASM_SIMP_TAC[VAL_WORD_SUB_CASES] THEN
     MATCH_MP_TAC REF_SUB_KEEP THEN ASM_REWRITE_TAC[]]);;

let SUB_REF_VAL_WORD = prove
 (`!p (x:W word) (y:W word).
       0 < p /\ p < 2 EXP dimindex(:W) /\ val x < p /\ val y < p
       ==> val(ref_sub (word p : W word) x y) = (val x + p - val y) MOD p`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  SUBGOAL_THEN `val(word p : W word) = p` ASSUME_TAC THENL
   [REWRITE_TAC[VAL_WORD] THEN MATCH_MP_TAC MOD_LT THEN ASM_REWRITE_TAC[];
    ALL_TAC] THEN
  FIRST_ASSUM(fun th ->
    MATCH_MP_TAC (REWRITE_RULE[th]
      (SPECL [`(word p : W word)`; `(x:W word)`; `(y:W word)`] SUB_REF_VAL))) THEN
  ASM_REWRITE_TAC[]);;

(* neg: zero if the input is zero, pw - x otherwise. *)
let ref_neg = new_definition
 `ref_neg (pw:W word) (x:W word) =
    if x = word(0:num) then word(0:num) else word_sub pw x`;;

let NEG_REF_VAL = prove
 (`!(pw:W word) (x:W word).
       0 < val pw /\ val x <= val pw
       ==> val(ref_neg pw x) = (val pw - val x) MOD val pw`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  SUBGOAL_THEN
   `val(pw:W word) < 2 EXP dimindex(:W) /\
    val(x:W word) < 2 EXP dimindex(:W)`
    STRIP_ASSUME_TAC THENL [REWRITE_TAC[VAL_BOUND]; ALL_TAC] THEN
  REWRITE_TAC[ref_neg; GSYM VAL_EQ_0] THEN
  ASM_CASES_TAC `val(x:W word) = 0` THENL
   [ ASM_REWRITE_TAC[VAL_WORD_0; SUB_0] THEN REWRITE_TAC[MOD_REFL];
     ASM_SIMP_TAC[VAL_WORD_SUB_CASES] THEN
     CONV_TAC SYM_CONV THEN MATCH_MP_TAC MOD_LT THEN ASM_ARITH_TAC]);;

let NEG_REF_VAL_WORD = prove
 (`!p (x:W word).
       0 < p /\ p < 2 EXP dimindex(:W) /\ val x <= p
       ==> val(ref_neg (word p : W word) x) = (p - val x) MOD p`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  SUBGOAL_THEN `val(word p : W word) = p` ASSUME_TAC THENL
   [REWRITE_TAC[VAL_WORD] THEN MATCH_MP_TAC MOD_LT THEN ASM_REWRITE_TAC[];
    ALL_TAC] THEN
  FIRST_ASSUM(fun th ->
    MATCH_MP_TAC (REWRITE_RULE[th]
      (SPECL [`(word p : W word)`; `(x:W word)`] NEG_REF_VAL))) THEN
  ASM_REWRITE_TAC[]);;

(* the int `rem' specification of sub, brought to the ref_sub residue *)
let SUB_INT_REM_MOD = prove
 (`!p m n:num. 0 < p /\ m < p /\ n < p
    ==> ((&m:int) - &n) rem &p = &((m + p - n) MOD p)`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  SUBGOAL_THEN `n <= p` ASSUME_TAC THENL [ASM_ARITH_TAC; ALL_TAC] THEN
  ASM_CASES_TAC `m < n` THENL
   [ ASM_SIMP_TAC[SUB_REM_LT] THEN REWRITE_TAC[INT_OF_NUM_EQ] THEN
     ASM_SIMP_TAC[SUB_REARRANGE] THEN
     CONV_TAC SYM_CONV THEN MATCH_MP_TAC MOD_LT THEN ASM_ARITH_TAC;
     ASM_SIMP_TAC[SUB_REM_GE] THEN REWRITE_TAC[INT_OF_NUM_EQ] THEN
     SUBGOAL_THEN `m + p - n = (m - n) + p` SUBST1_TAC THENL
      [ASM_ARITH_TAC; ALL_TAC] THEN
     REWRITE_TAC[MOD_ADD_SELF] THEN
     CONV_TAC SYM_CONV THEN MATCH_MP_TAC MOD_LT THEN ASM_ARITH_TAC]);;

let BRIDGE_REF2_LEMMAS =
  [MOD_ADD_SELF; SUB_REARRANGE; REF_SUB_KEEP; REF_SUB_BORROW; SUB_REF_VAL;
   SUB_REF_VAL_WORD; NEG_REF_VAL; NEG_REF_VAL_WORD; SUB_INT_REM_MOD];;
let BRIDGE_SPEC_LEMMAS = BRIDGE_SPEC_LEMMAS @ BRIDGE_REF2_LEMMAS;;

(* A numeral can reach the goal in two shapes: the tagged form the parser and
   `mk_numeral` build, `NUMERAL(BIT1(BIT0(...)))`, and the same bit chain with
   the tag stripped.  Both print as the decimal literal, so the mismatch is
   invisible in a log and `aconv` alone misses the range hypothesis (measured:
   `neg_p25519`, hypothesis present, `aconv` false, `p` not `is_numeral`).
   Compare modulo the tag. *)
let rec BRIDGE_STRIP_NUMERAL tm =
  match tm with
  | Comb(Const("NUMERAL",_),x) -> BRIDGE_STRIP_NUMERAL x
  | _ -> tm;;

let rec BRIDGE_TERM_EQ (s:term) (t:term) =
  let s = BRIDGE_STRIP_NUMERAL s and t = BRIDGE_STRIP_NUMERAL t in
  aconv s t ||
  (try dest_numeral s = dest_numeral t with Failure _ -> false) ||
  (is_comb s && is_comb t &&
   BRIDGE_TERM_EQ (rator s) (rator t) && BRIDGE_TERM_EQ (rand s) (rand t));;

(* Find `tm < p` (or `tm <= p`) in the context.  Returns the hypothesis, the
   modulus term actually written there (a tagged numeral) and, when the goal's
   copy of the modulus is the untagged shape, the equation that puts the tagged
   one into the goal.  The word-level lemma's side conditions are arithmetic on
   the numeral, so the tagged form is the one to work with. *)
let BRIDGE_FIND_BOUND tag rel p tm asl =
  let pat = mk_comb(mk_comb(rel, tm), p) in
  let th =
    try snd(List.find (fun (_,th) -> BRIDGE_TERM_EQ (concl th) pat) asl)
    with Not_found ->
      failwith (tag^": missing hypothesis "^(string_of_term pat)) in
  let ph = rand (concl th) in
  let eqs =
    if aconv ph p then [] else [prove(mk_eq(p, ph), REWRITE_TAC[NUMERAL])] in
  (th, ph, eqs);;

let BRIDGE_BOUND_HYP tag p tm asl =
  BRIDGE_FIND_BOUND tag `(<):num->num->bool` p tm asl;;

let BRIDGE_LE_HYP tag p tm asl =
  BRIDGE_FIND_BOUND tag `(<=):num->num->bool` p tm asl;;

(* zero and width side conditions of the word-level ref lemmas *)
let BRIDGE_REF_SIDES tag p (lem:thm) : thm * thm =
  let ants = conjuncts (lhand (concl lem)) in
  if List.length ants < 2 then
    failwith (tag^": unexpected ref-lemma shape");
  let pos = prove(List.nth ants 0, CONV_TAC NUM_LT_CONV) in
  let width = prove(List.nth ants 1,
    CONV_TAC(RAND_CONV(RAND_CONV DIMINDEX_CONV) THENC
             RAND_CONV NUM_REDUCE_CONV THENC NUM_LT_CONV)) in
  (pos,width);;

(* sub: the spec is `&<written value> = (&m - &n) rem &p' *)
let BRIDGE_SUB_REF_TAC expand pth : tactic =
  ELIM_DIGIT_EQS_TAC THEN BRIDGE_REWRITE_MODULUS pth THEN expand THEN
  (fun (asl,w) ->
    let (_,rhs) = dest_eq w in
    let (diff,pt) = dest_binop `(rem):int->int->int` rhs in
    let (mt,nt) = dest_binop `(-):int->int->int` diff in
    let m = rand mt and n = rand nt and p0 = rand pt in
    let (hm,pm,eqm) = BRIDGE_BOUND_HYP "BRIDGE_SUB_REF_TAC" p0 m asl
    and (hn,pn,eqn) = BRIDGE_BOUND_HYP "BRIDGE_SUB_REF_TAC" p0 n asl in
    if not (aconv pm pn) then
      failwith "BRIDGE_SUB_REF_TAC: bounds use different moduli";
    let p = pm in
    let lem = SPECL [p; m; n] SUB_INT_REM_MOD in
    let ants = conjuncts (lhand (concl lem)) in
    if List.length ants <> 3 then
      failwith "BRIDGE_SUB_REF_TAC: unexpected spec-lemma shape";
    let zpos = prove(List.nth ants 0, CONV_TAC NUM_LT_CONV) in
    let sth = MP lem (end_itlist CONJ [zpos; hm; hn]) in
    let (xw,thx) = BRIDGE_WORD_OF_DIGITS m in
    let (yw,thy) = BRIDGE_WORD_OF_DIGITS n in
    let (lhs,_) = dest_eq w in
    let wv = rand lhs in
    let (_,thw) = BRIDGE_WORD_OF_DIGITS wv in
    let wt = bridge_finty (bridge_word_width (type_of xw)) in
    let lem2 = SPECL [p; xw; yw] (INST_TYPE [wt,`:W`] SUB_REF_VAL_WORD) in
    let (a0,a1) = BRIDGE_REF_SIDES "BRIDGE_SUB_REF_TAC" p lem2 in
    let hx = REWRITE_RULE[GSYM thx] hm and hy = REWRITE_RULE[GSYM thy] hn in
    let rth = MP lem2 (end_itlist CONJ [a0; a1; hx; hy]) in
    let rth = SYM (REWRITE_RULE[thx; thy] rth) in
    (REWRITE_TAC(eqm @ eqn) THEN
     ASM_REWRITE_TAC[sth; INT_OF_NUM_EQ; rth; SYM thw; ref_sub;
                     LET_DEF; LET_END_DEF] THEN
     BRIDGE_FINISH_KEEP_TAC expand) (asl,w));;

(* neg: the spec is `bignum_from_memory (...) s = (p - n) MOD p' *)
let BRIDGE_NEG_REF_TAC expand pth : tactic =
  ELIM_DIGIT_EQS_TAC THEN BRIDGE_REWRITE_MODULUS pth THEN expand THEN
  (fun (asl,w) ->
    let (lhs,rhs) = dest_eq w in
    let (dif,pt) = dest_binop `(MOD):num->num->num` rhs in
    let n = rand dif and p0 = rand pt in
    let (hn,p,eqn) = BRIDGE_LE_HYP "BRIDGE_NEG_REF_TAC" p0 n asl in
    let (xw,thx) = BRIDGE_WORD_OF_DIGITS n in
    let (ww,thw) = BRIDGE_WORD_OF_DIGITS lhs in
    let wt = bridge_finty (bridge_word_width (type_of xw)) in
    let lem = SPECL [p; xw] (INST_TYPE [wt,`:W`] NEG_REF_VAL_WORD) in
    let (a0,a1) = BRIDGE_REF_SIDES "BRIDGE_NEG_REF_TAC" p lem in
    let hx = REWRITE_RULE[GSYM thx] hn in
    let rth = MP lem (end_itlist CONJ [a0; a1; hx]) in
    let rth = SYM (REWRITE_RULE[thx] rth) in
    (REWRITE_TAC eqn THEN
     REWRITE_TAC[rth; SYM thw; ref_neg; LET_DEF; LET_END_DEF] THEN
     BRIDGE_FINISH_KEEP_TAC expand) (asl,w));;
