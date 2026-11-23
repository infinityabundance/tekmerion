(* ========================================================================= *)
(* bridge.ml -- generic bridge lemmas from a num/MOD (or int/rem) spec to a   *)
(* word-level circuit statement that certified SAT can discharge.            *)
(*                                                                           *)
(* Parameterisation: the modulus `p` is an argument, the limb width is the   *)
(* 64 of the target ISA, and the limb count enters only through the powers   *)
(* of two that the bit-blaster sees.  There is no P-256 constant, no P-256   *)
(* limb list, and no transposed human tactic fragment in this file.          *)
(*                                                                           *)
(* Provenance of the mathematics used:                                       *)
(*   MOD_LT / MOD_EQ / MOD_REFL                  (core arith)                *)
(*   INT_REM_UNIQ / INT_OF_NUM_LT / INT_ARITH    (core int)                  *)
(*   VAL_WORD_JOIN / VAL_WORD_ADD / VAL_WORD_0 / VAL_WORD_BITVAL             *)
(*   INT_OF_NUM_* / INT_OF_NUM_POW                (core int)                 *)
(*                                                                           *)
(* Kernel trust: every SAT refutation is replayed by the LCF kernel inside   *)
(* CSAT_REFUTE (csat_lib.ml); no solver certificate is trusted.              *)
(* ========================================================================= *)

(* ------------------------------------------------------------------------- *)
(* 1. The modular specification in "sum form".                               *)
(*                                                                           *)
(* Each lemma states the spec-side identity in terms of the value the        *)
(* machine actually writes, split at the single conditional correction.  The *)
(* machine-side obligations are then pure *sum* equalities, which is what    *)
(* makes the residue bit-blastable (no subtraction, no MOD).                 *)
(* ------------------------------------------------------------------------- *)

(* add: (m + n) MOD p, at most one correction because m < p and n < p. *)
let ADD_MOD_LT = prove
 (`!p m n. m < p /\ n < p /\ m + n < p ==> (m + n) MOD p = m + n`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  SUBGOAL_THEN `(m + n) MOD p = (if m + n < p then m + n else (m + n) - p)`
    SUBST1_TAC THENL
   [MATCH_MP_TAC MOD_ADD_CASES THEN ASM_REWRITE_TAC[];
    ASM_REWRITE_TAC[]]);;

let ADD_MOD_GE = prove
 (`!p m n. m < p /\ n < p /\ p <= m + n ==> (m + n) MOD p = (m + n) - p`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  SUBGOAL_THEN `(m + n) MOD p = (if m + n < p then m + n else (m + n) - p)`
    SUBST1_TAC THENL
   [MATCH_MP_TAC MOD_ADD_CASES THEN ASM_REWRITE_TAC[];
    SUBGOAL_THEN `~(m + n < p)` ASSUME_TAC THENL [ASM_ARITH_TAC; ALL_TAC] THEN
    ASM_REWRITE_TAC[]]);;

(* `t = (m + n) - p' to sum form, removing the subtraction *)
let ADD_GE_SUM = prove
 (`!p m n t. p <= m + n ==> ((t = (m + n) - p) <=> (t + p = m + n))`,
  REPEAT GEN_TAC THEN DISCH_TAC THEN
  MP_TAC(ISPECL [`m + n:num`; `p:num`] SUB_ADD) THEN
  ANTS_TAC THENL [ASM_REWRITE_TAC[]; DISCH_TAC] THEN
  EQ_TAC THENL
   [DISCH_THEN SUBST1_TAC THEN ASM_REWRITE_TAC[];
    DISCH_TAC THEN UNDISCH_TAC `((m + n) - p) + p = m + n` THEN
    ASM_ARITH_TAC]);;

(* the same bridge, stated once in the shape the pipeline applies *)
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

(* ------------------------------------------------------------------------- *)
(* 2. Word-level encoding of `2^(64*j) * (val x)' and `2^(64*j) * bitval b'. *)
(*                                                                           *)
(* `word_join' is free under bit-blasting (BIT_WORD_JOIN); a product by a    *)
(* power of two expands into a full multiplier instead, which dominates the  *)
(* cost.  Rewriting the shifts to joins is what makes the residue cheap.     *)
(* ------------------------------------------------------------------------- *)

(* A reusable power-of-two bound: multiplying a bound by 2^j preserves it. *)
let EXP2_NZ = prove
 (`!j:num. ~(2 EXP j = 0)`,
  GEN_TAC THEN REWRITE_TAC[EXP_EQ_0] THEN ARITH_TAC);;

let POW2_MULT_LT = prove
 (`!v:num j k. v < 2 EXP k ==> 2 EXP j * v < 2 EXP j * 2 EXP k`,
  REPEAT GEN_TAC THEN DISCH_TAC THEN
  REWRITE_TAC[LT_MULT_LCANCEL] THEN
  REWRITE_TAC[EXP2_NZ] THEN ASM_REWRITE_TAC[]);;

let VAL_JOIN_J1 = prove
 (`!x:int64. val(word_join x (word 0:int64):128 word) = 2 EXP 64 * val x`,
  GEN_TAC THEN CONV_TAC(LAND_CONV(ONCE_DEPTH_CONV VAL_WORD_JOIN)) THEN
  CONV_TAC(DEPTH_CONV DIMINDEX_CONV) THEN
  REWRITE_TAC[VAL_WORD_0; ADD_0] THEN
  MATCH_MP_TAC MOD_LT THEN
  SUBGOAL_THEN `2 EXP 128 = 2 EXP 64 * 2 EXP 64`
    (fun th -> REWRITE_TAC[th]) THENL
   [REWRITE_TAC[ARITH_RULE `128 = 64 + 64`; EXP_ADD];
    MATCH_MP_TAC POW2_MULT_LT THEN
    MP_TAC(ISPEC `x:int64` VAL_BOUND) THEN REWRITE_TAC[DIMINDEX_64] THEN
    DISCH_TAC THEN ASM_REWRITE_TAC[]]);;

let VAL_JOIN_J1_BITS = prove
 (`!b:bool. val(word_join (word(bitval b):1 word) (word 0:int64):65 word) =
            2 EXP 64 * bitval b`,
  GEN_TAC THEN CONV_TAC(LAND_CONV(ONCE_DEPTH_CONV VAL_WORD_JOIN)) THEN
  CONV_TAC(DEPTH_CONV DIMINDEX_CONV) THEN
  REWRITE_TAC[VAL_WORD_0; ADD_0; VAL_WORD_BITVAL] THEN
  MATCH_MP_TAC MOD_LT THEN
  SUBGOAL_THEN `2 EXP 65 = 2 EXP 64 * 2 EXP 1`
    (fun th -> REWRITE_TAC[th]) THENL
   [CONV_TAC NUM_REDUCE_CONV;
    MATCH_MP_TAC POW2_MULT_LT THEN
    MP_TAC(ISPEC `b:bool` BITVAL_BOUND) THEN
    REWRITE_TAC[EXP_ONE] THEN ARITH_TAC]);;

let VAL_JOIN_J2 = prove
 (`!x:int64.
     val(word_join (word_join x (word 0:int64):128 word) (word 0:int64):192 word) =
     2 EXP 128 * val x`,
  GEN_TAC THEN CONV_TAC(LAND_CONV(ONCE_DEPTH_CONV VAL_WORD_JOIN)) THEN
  CONV_TAC(DEPTH_CONV DIMINDEX_CONV) THEN
  REWRITE_TAC[VAL_WORD_0; ADD_0; VAL_JOIN_J1] THEN
  SUBGOAL_THEN `2 EXP 128 * val(x:int64) = 2 EXP 64 * (2 EXP 64 * val x)`
    (fun th -> REWRITE_TAC[th]) THENL
   [SUBGOAL_THEN `2 EXP 128 = 2 EXP 64 * 2 EXP 64`
      (fun th -> REWRITE_TAC[th]) THENL
     [CONV_TAC NUM_REDUCE_CONV; REWRITE_TAC[MULT_ASSOC]];
    ALL_TAC] THEN
  MATCH_MP_TAC MOD_LT THEN
  SUBGOAL_THEN `2 EXP 192 = 2 EXP 64 * 2 EXP (64 + 64)`
    (fun th -> REWRITE_TAC[th]) THENL
   [CONV_TAC NUM_REDUCE_CONV; ALL_TAC] THEN
  SUBGOAL_THEN `2 EXP 64 * val(x:int64) < 2 EXP (64 + 64)` ASSUME_TAC THENL
   [REWRITE_TAC[EXP_ADD] THEN MATCH_MP_TAC POW2_MULT_LT THEN
    MP_TAC(ISPEC `x:int64` VAL_BOUND) THEN REWRITE_TAC[DIMINDEX_64] THEN
    DISCH_TAC THEN ASM_REWRITE_TAC[];
    ALL_TAC] THEN
  MATCH_MP_TAC POW2_MULT_LT THEN ASM_REWRITE_TAC[]);;

let VAL_JOIN_J3 = prove
 (`!x:int64.
     val(word_join (word_join (word_join x (word 0:int64):128 word)
                              (word 0:int64):192 word) (word 0:int64):256 word) =
     2 EXP 192 * val x`,
  GEN_TAC THEN CONV_TAC(LAND_CONV(ONCE_DEPTH_CONV VAL_WORD_JOIN)) THEN
  CONV_TAC(DEPTH_CONV DIMINDEX_CONV) THEN
  REWRITE_TAC[VAL_WORD_0; ADD_0] THEN
  SUBGOAL_THEN
   `val(word_join (word_join x (word 0:int64):128 word) (word 0:int64):192 word) =
    2 EXP 64 * (2 EXP 64 * val(x:int64))`
    (fun th -> REWRITE_TAC[th]) THENL
   [REWRITE_TAC[VAL_JOIN_J2] THEN
    SUBGOAL_THEN `2 EXP 128 = 2 EXP 64 * 2 EXP 64`
      (fun th -> REWRITE_TAC[th]) THENL
     [CONV_TAC NUM_REDUCE_CONV; REWRITE_TAC[MULT_ASSOC]];
    ALL_TAC] THEN
  SUBGOAL_THEN
   `2 EXP 192 * val(x:int64) = 2 EXP 64 * (2 EXP 64 * (2 EXP 64 * val x))`
    (fun th -> REWRITE_TAC[th]) THENL
   [SUBGOAL_THEN `2 EXP 192 = 2 EXP 64 * (2 EXP 64 * 2 EXP 64)`
      (fun th -> REWRITE_TAC[th]) THENL
     [CONV_TAC NUM_REDUCE_CONV; REWRITE_TAC[MULT_ASSOC]];
    ALL_TAC] THEN
  MATCH_MP_TAC MOD_LT THEN
  SUBGOAL_THEN `2 EXP 256 = 2 EXP 64 * 2 EXP (64 + (64 + 64))`
    (fun th -> REWRITE_TAC[th]) THENL
   [CONV_TAC NUM_REDUCE_CONV; ALL_TAC] THEN
  SUBGOAL_THEN `2 EXP 64 * (2 EXP 64 * val(x:int64)) < 2 EXP (64 + (64 + 64))`
    ASSUME_TAC THENL
   [REWRITE_TAC[EXP_ADD] THEN
    SUBGOAL_THEN `2 EXP 64 * val(x:int64) < 2 EXP (64 + 64)` ASSUME_TAC THENL
     [REWRITE_TAC[EXP_ADD] THEN MATCH_MP_TAC POW2_MULT_LT THEN
      MP_TAC(ISPEC `x:int64` VAL_BOUND) THEN REWRITE_TAC[DIMINDEX_64] THEN
      DISCH_TAC THEN ASM_REWRITE_TAC[];
      ALL_TAC] THEN
    MATCH_MP_TAC POW2_MULT_LT THEN ASM_REWRITE_TAC[];
    ALL_TAC] THEN
  MATCH_MP_TAC POW2_MULT_LT THEN ASM_REWRITE_TAC[]);;

let VAL_JOIN_J1_BITS_256 = prove
 (`!b:bool.
     val(word_join (word(bitval b):1 word) (word 0:256 word):257 word) =
     2 EXP 256 * bitval b`,
  GEN_TAC THEN CONV_TAC(LAND_CONV(ONCE_DEPTH_CONV VAL_WORD_JOIN)) THEN
  CONV_TAC(DEPTH_CONV DIMINDEX_CONV) THEN
  REWRITE_TAC[VAL_WORD_0; ADD_0; VAL_WORD_BITVAL] THEN
  MATCH_MP_TAC MOD_LT THEN
  SUBGOAL_THEN `2 EXP 257 = 2 EXP 256 * 2 EXP 1`
    (fun th -> REWRITE_TAC[th]) THENL
   [CONV_TAC NUM_REDUCE_CONV;
    MATCH_MP_TAC POW2_MULT_LT THEN
    MP_TAC(ISPEC `b:bool` BITVAL_BOUND) THEN
    REWRITE_TAC[EXP_ONE] THEN ARITH_TAC]);;

(* integer forms, matching the shape produced by lifting a num goal to int *)
let INT_JOIN_VAL1 = prove
 (`!x:int64. (&2:int) pow 64 * &(val x) =
             &(val(word_join x (word 0:int64):128 word))`,
  GEN_TAC THEN REWRITE_TAC[GSYM INT_OF_NUM_POW; GSYM INT_OF_NUM_MUL] THEN
  AP_TERM_TAC THEN REWRITE_TAC[VAL_JOIN_J1]);;

let INT_JOIN_BIT1 = prove
 (`!b:bool. (&2:int) pow 64 * &(bitval b) =
            &(val(word_join (word(bitval b):1 word) (word 0:int64):65 word))`,
  GEN_TAC THEN REWRITE_TAC[GSYM INT_OF_NUM_POW; GSYM INT_OF_NUM_MUL] THEN
  AP_TERM_TAC THEN REWRITE_TAC[VAL_JOIN_J1_BITS]);;

let INT_JOIN_VAL2 = prove
 (`!x:int64. (&2:int) pow 128 * &(val x) =
             &(val(word_join (word_join x (word 0:int64):128 word) (word 0:int64):192 word))`,
  GEN_TAC THEN REWRITE_TAC[GSYM INT_OF_NUM_POW; GSYM INT_OF_NUM_MUL] THEN
  AP_TERM_TAC THEN REWRITE_TAC[VAL_JOIN_J2]);;

let INT_JOIN_VAL3 = prove
 (`!x:int64.
     (&2:int) pow 192 * &(val x) =
     &(val(word_join (word_join (word_join x (word 0:int64):128 word)
                                (word 0:int64):192 word) (word 0:int64):256 word))`,
  GEN_TAC THEN REWRITE_TAC[GSYM INT_OF_NUM_POW; GSYM INT_OF_NUM_MUL] THEN
  AP_TERM_TAC THEN REWRITE_TAC[VAL_JOIN_J3]);;

let INT_JOIN_BIT4 = prove
 (`!b:bool. (&2:int) pow 256 * &(bitval b) =
            &(val(word_join (word(bitval b):1 word) (word 0:256 word):257 word))`,
  GEN_TAC THEN REWRITE_TAC[GSYM INT_OF_NUM_POW; GSYM INT_OF_NUM_MUL] THEN
  AP_TERM_TAC THEN REWRITE_TAC[VAL_JOIN_J1_BITS_256]);;

let BRIDGE_JOIN_LEMMAS =
  [INT_JOIN_BIT1; INT_JOIN_VAL1; INT_JOIN_VAL2; INT_JOIN_VAL3; INT_JOIN_BIT4];;

(* ------------------------------------------------------------------------- *)
(* 3. Conversions.                                                           *)
(* ------------------------------------------------------------------------- *)

(* real accumulator equation -> integer equation (atoms `&(val w)',
   `&(bitval b)').  int subtraction is total, so negative differences keep    *)
(* their meaning and nothing has to be reoriented.                           *)
let REAL_TO_INT_CONV =
  GEN_REWRITE_CONV DEPTH_CONV
   [GSYM int_of_num_th; GSYM int_neg_th; GSYM int_add_th;
    GSYM int_sub_th; GSYM int_mul_th; GSYM int_pow_th; GSYM int_eq];;

(* num equation -> int equation, with `&' pushed inside sum/product/power *)
let NUM_TO_INT_CONV =
  GEN_REWRITE_CONV DEPTH_CONV
   [GSYM INT_OF_NUM_EQ; GSYM INT_OF_NUM_LT; GSYM INT_OF_NUM_LE;
    INT_OF_NUM_ADD; INT_OF_NUM_MUL; INT_OF_NUM_POW];;

(* products by powers of two -> word joins *)
let INT_SHIFT_TO_JOIN_CONV =
  GEN_REWRITE_CONV DEPTH_CONV BRIDGE_JOIN_LEMMAS;;

(* ------------------------------------------------------------------------- *)
(* 4. Certified-SAT close.                                                   *)
(*                                                                           *)
(* BITBLAST_THEN consumes only *word equation* assumptions, and DEPTH_CONV   *)
(* silently leaves unconvertible atoms alone (so an unconverted atom becomes  *)
(* a free proposition and the residue is reported SAT).  The residue is      *)
(* therefore presented to it as one closed implication: arithmetic           *)
(* hypotheses are conjoined into the antecedent and the goal stack is        *)
(* emptied first.  Dropping hypotheses is sound; it strengthens the goal.    *)
(* ------------------------------------------------------------------------- *)

let BRIDGE_ARITH_BLACKLIST =
  ["read"; "MAYCHANGE"; "nonoverlapping_modulo"; "aligned_bytes_loaded";
   "eventually"; "ensures"; "bignum_from_memory"; "bytes"; "memory";
   "PC"; "events"];;

let bridge_is_arith th =
  let tm = concl th in
  let cs = find_terms is_const tm in
  let bad = exists (fun c -> mem (fst (dest_const c)) BRIDGE_ARITH_BLACKLIST) cs in
  let tys = map (fun v -> snd (dest_var v)) (frees tm) in
  let okty ty = ty = `:num` || ty = `:int` || ty = `:bool` in
  not bad && forall okty tys;;

let BRIDGE_KEEP_ARITH_TAC : tactic =
  POP_ASSUM_LIST (fun ths ->
    MAP_EVERY ASSUME_TAC (filter bridge_is_arith ths));;

let BRIDGE_CSAT_TAC : tactic =
  fun (asl,w) ->
    let hyps = map concl asl in
    let imp = if hyps = [] then w else list_mk_imp(hyps,w) in
    let th = prove(imp, CSAT_TAC) in
    (MATCH_MP_TAC th THEN ASM_REWRITE_TAC[]) (asl,w);;

(* everything that precedes the SAT call *)
let BRIDGE_ASSUM_TO_INT_CONV = REAL_TO_INT_CONV THENC NUM_TO_INT_CONV;;

let BRIDGE_GOAL_TO_INT_CONV =
  NUM_TO_INT_CONV THENC
  GEN_REWRITE_CONV DEPTH_CONV [GSYM INT_BITVAL_NOT] THENC
  INT_SHIFT_TO_JOIN_CONV;;

(* Expand `bignum_of_wordlist [..]' to the flat sum of shifted digits, so    *)
(* that the shifts are the units the join lemmas match.  Passed in because  *)
(* the conversion lives in s2n-bignum's common/bignum.ml (the harness), not  *)
(* in HOL Light core.  `ALL_TAC' disables it.                                *)
let BRIDGE_PREPARE_TAC (expand:tactic) : tactic =
  fun g ->
    (BRIDGE_KEEP_ARITH_TAC THEN
     expand THEN
     RULE_ASSUM_TAC (CONV_RULE BRIDGE_ASSUM_TO_INT_CONV) THEN
     RULE_ASSUM_TAC (REWRITE_RULE (GSYM INT_BITVAL_NOT :: BRIDGE_JOIN_LEMMAS)) THEN
     CONV_TAC BRIDGE_GOAL_TO_INT_CONV) g;;

(* ------------------------------------------------------------------------- *)
(* 5. Case splitting on the conditionals the simulator leaves in the goal.   *)
(* ------------------------------------------------------------------------- *)

let rec BRIDGE_SPLIT_CONDITIONS_TAC (asl,w) =
  let conds = map (fun t -> rand(rator t)) (find_terms is_cond w) in
  match conds with
  | [] -> ALL_TAC (asl,w)
  | c::_ ->
      (ASM_CASES_TAC c THEN
       ASM_REWRITE_TAC[COND_CLAUSES; BITVAL_CLAUSES] THEN
       BRIDGE_SPLIT_CONDITIONS_TAC) (asl,w);;

(* ------------------------------------------------------------------------- *)
(* 6. Eliminate the specification variables from their digit expansions.     *)
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
(* 7. Entry points.  `pth' is the definition theorem of the modulus, so that *)
(* the numeral replaces the constant before bit-blasting.                    *)
(* ------------------------------------------------------------------------- *)

let BRIDGE_ZERO_LT_TAC p : tactic =
  SUBGOAL_THEN (mk_comb(mk_comb(`(<):num->num->bool`, `0`), p))
   (fun th -> ASSUME_TAC th) THENL [ARITH_TAC; ALL_TAC];;

let BRIDGE_FINISH_TAC expand : tactic =
  fun g ->
    (BRIDGE_SPLIT_CONDITIONS_TAC THEN
     BRIDGE_PREPARE_TAC expand THEN
     BRIDGE_CSAT_TAC) g;;

(* add: the spec is `(m + n) MOD p = <written value>' *)
let BRIDGE_ADD_TAC expand pth : tactic =
  ELIM_DIGIT_EQS_TAC THEN REWRITE_TAC[pth] THEN
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

(* sub: the spec is `&<written value> = (&m - &n) rem &p' (int) *)
let BRIDGE_SUB_TAC expand pth : tactic =
  ELIM_DIGIT_EQS_TAC THEN REWRITE_TAC[pth] THEN
  (fun (asl,w) ->
    let (_,rhs) = dest_eq w in
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

(* neg: the spec is `<written value> = (p - n) MOD p' *)
let BRIDGE_NEG_TAC expand pth : tactic =
  ELIM_DIGIT_EQS_TAC THEN REWRITE_TAC[pth] THEN
  (fun (asl,w) ->
    let (_,rhs) = dest_eq w in
    let (diff,p) = dest_binop `(MOD):num->num->num` rhs in
    let n = rand diff in
    let cond = mk_comb(mk_comb(`(=):num->num->bool`, n), `0`) in
    (BRIDGE_ZERO_LT_TAC p THEN
     ASM_CASES_TAC cond THENL
      [ASM_SIMP_TAC[NEG_MOD_ZERO] THEN BRIDGE_FINISH_TAC expand;
       ASM_SIMP_TAC[NEG_MOD_NZ] THEN
       ASM_SIMP_TAC[NEG_NZ_SUM] THEN BRIDGE_FINISH_TAC expand]) (asl,w));;
