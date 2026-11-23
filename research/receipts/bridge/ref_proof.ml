(* Candidate generic reference-function lemma for addition, all widths.
   Development harness; proven forms are moved into tools/bridge.ml. *)
#load "unix.cma";;
needs "arm/proofs/base.ml";;

(* ------------------------------------------------------------------------- *)
(* Plain-variable arithmetic (no dimindex/EXP inside, so ARITH is reliable).  *)
(* ------------------------------------------------------------------------- *)
let SUB_ADD_BACK = prove
 (`!a b m:num. m <= a + b ==> ((a + b) - m) + m = a + b`,
  REPEAT GEN_TAC THEN DISCH_TAC THEN
  MATCH_MP_TAC SUB_ADD THEN ASM_REWRITE_TAC[]);;

let SUB_LT_DROP = prove
 (`!a b m:num. b < m /\ m <= a + b ==> (a + b) - m < a`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  MATCH_MP_TAC(ARITH_RULE `!a b m. b < m /\ m <= a + b ==> (a + b) - m < a`) THEN
  ASM_REWRITE_TAC[]);;

let SUB_MOVE = prove
 (`!r m q:num. q <= m ==> r + (m - q) = (r + m) - q`,
  REPEAT GEN_TAC THEN REWRITE_TAC[LE_EXISTS] THEN
  DISCH_THEN(X_CHOOSE_TAC `d:num`) THEN
  ASM_REWRITE_TAC[ADD_SUB2; ADD_AC]);;

let MOD_SUB_ONE = prove
 (`!s q:num. q <= s /\ s < 2 * q ==> s MOD q = s - q`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  MATCH_MP_TAC MOD_UNIQ THEN EXISTS_TAC `1` THEN
  REWRITE_TAC[MULT_CLAUSES] THEN CONJ_TAC THENL
   [ REWRITE_TAC[ADD_SYM] THEN MATCH_MP_TAC SUB_ADD THEN ASM_REWRITE_TAC[];
     ASM_ARITH_TAC ]);;

(* ------------------------------------------------------------------------- *)
(* Value of word_add / word_sub under each case, as rewrite rules.            *)
(* ------------------------------------------------------------------------- *)
let VAL_WORD_ADD_LT = prove
 (`!(x:W word) (y:W word).
        val x + val y < 2 EXP dimindex(:W)
        ==> val(word_add x y) = val x + val y`,
  REPEAT GEN_TAC THEN DISCH_TAC THEN
  ASM_SIMP_TAC[VAL_WORD_ADD_CASES]);;

let VAL_WORD_ADD_GE = prove
 (`!(x:W word) (y:W word).
        ~(val x + val y < 2 EXP dimindex(:W))
        ==> val(word_add x y) =
            val x + val y - 2 EXP dimindex(:W)`,
  REPEAT GEN_TAC THEN DISCH_TAC THEN
  ASM_SIMP_TAC[VAL_WORD_ADD_CASES] THEN REFL_TAC);;

let VAL_WORD_SUB_GE = prove
 (`!(s:W word) (pw:W word).
        val pw <= val s
        ==> val(word_sub s pw) = val s - val pw`,
  REPEAT GEN_TAC THEN DISCH_TAC THEN
  ASM_SIMP_TAC[VAL_WORD_SUB_CASES]);;

let VAL_WORD_SUB_LT = prove
 (`!(s:W word) (pw:W word).
        ~(val pw <= val s)
        ==> val(word_sub s pw) =
            val s + (2 EXP dimindex(:W) - val pw)`,
  REPEAT GEN_TAC THEN DISCH_TAC THEN
  ASM_SIMP_TAC[VAL_WORD_SUB_CASES]);;

(* ------------------------------------------------------------------------- *)
(* The reference: a word-sized x+y with its carry folded into the condition. *)
(* ------------------------------------------------------------------------- *)
let ref_add = new_definition
 `ref_add (pw:W word) (x:W word) (y:W word) =
    let s = word_add x y in
    if word_ult s x \/ word_ule pw s then word_sub s pw else s`;;

let WORD_ADD_EQ_CARRY = prove
 (`!(x:W word) (y:W word).
        val x < 2 EXP dimindex(:W) /\ val y < 2 EXP dimindex(:W) /\
        val(word_add x y) < val x
        ==> val(word_add x y) +
            2 EXP dimindex(:W) = val x + val y`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  ASM_CASES_TAC `val(x:W word) + val(y:W word) < 2 EXP dimindex(:W)` THENL
   [ ASM_SIMP_TAC[VAL_WORD_ADD_LT] THEN ASM_ARITH_TAC;
     ASM_SIMP_TAC[VAL_WORD_ADD_GE; NOT_LT] THEN
     SUBGOAL_THEN `2 EXP dimindex(:W) <= val(x:W word) + val(y:W word)`
       (fun ant -> SUBST1_TAC (MATCH_MP
          (ISPECL [`val(x:W word)`; `val(y:W word)`; `2 EXP dimindex(:W)`]
                  SUB_ADD_BACK) ant)) THEN
     ASM_SIMP_TAC[NOT_LT] ]);;

let WORD_ADD_EQ_NOCARRY = prove
 (`!(x:W word) (y:W word).
        val x < 2 EXP dimindex(:W) /\ val y < 2 EXP dimindex(:W) /\
        ~(val(word_add x y) < val x)
        ==> val(word_add x y) = val x + val y`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  ASM_CASES_TAC `val(x:W word) + val(y:W word) < 2 EXP dimindex(:W)` THENL
   [ ASM_SIMP_TAC[VAL_WORD_ADD_LT];
     ASM_SIMP_TAC[VAL_WORD_ADD_GE; NOT_LT] THEN
     SUBGOAL_THEN
      `val(x:W word) + val(y:W word) - 2 EXP dimindex(:W) < val(x:W word)`
       (fun ineq ->
          ASSUME_TAC ineq THEN
          UNDISCH_TAC
           `~(val(x:W word) + val(y:W word) -
              2 EXP dimindex(:W) < val(x:W word))` THEN
          ASM_REWRITE_TAC[]) THEN
     SUBGOAL_THEN
      `val(y:W word) < 2 EXP dimindex(:W) /\
       2 EXP dimindex(:W) <= val(x:W word) + val(y:W word)`
       (fun ant -> ACCEPT_TAC (MATCH_MP
          (ISPECL [`val(x:W word)`; `val(y:W word)`; `2 EXP dimindex(:W)`]
                  SUB_LT_DROP) ant)) THEN
     ASM_SIMP_TAC[NOT_LT] ]);;

let ADD_REF_VAL = prove
 (`!(pw:W word) (x:W word) (y:W word).
       0 < val pw /\ val x < val pw /\ val y < val pw
       ==> val(ref_add pw x y) = (val x + val y) MOD val pw`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  REWRITE_TAC[ref_add] THEN
  SUBGOAL_THEN
   `val(x:W word) < 2 EXP dimindex(:W) /\
    val(y:W word) < 2 EXP dimindex(:W) /\
    val(pw:W word) < 2 EXP dimindex(:W)`
   STRIP_ASSUME_TAC THENL [REWRITE_TAC[VAL_BOUND]; ALL_TAC] THEN
  ASM_CASES_TAC
   `word_ult (word_add (x:W word) (y:W word)) (x:W word) \/
    word_ule (pw:W word) (word_add (x:W word) (y:W word))` THENL
   [ (* ---- subtract branch ---- *)
     UNDISCH_TAC
      `word_ult (word_add (x:W word) (y:W word)) (x:W word) \/
       word_ule (pw:W word) (word_add (x:W word) (y:W word))` THEN
     REWRITE_TAC[WORD_ULT; WORD_ULE] THEN DISCH_TAC THEN
     ASM_REWRITE_TAC[WORD_ULT; WORD_ULE] THEN
     SUBGOAL_THEN `val(x:W word) + val(y:W word) < 2 * val(pw:W word)`
       ASSUME_TAC THENL [ASM_ARITH_TAC; ALL_TAC] THEN
     SUBGOAL_THEN `val(pw:W word) <= val(x:W word) + val(y:W word)`
       ASSUME_TAC THENL
      [ ASM_CASES_TAC `val(word_add (x:W word) (y:W word)) < val(x:W word)`
        THENL
         [ MP_TAC (ISPECL [`x:W word`; `y:W word`] WORD_ADD_EQ_CARRY) THEN
           ASM_REWRITE_TAC[] THEN DISCH_TAC THEN ASM_ARITH_TAC;
           MP_TAC (ISPECL [`x:W word`; `y:W word`] WORD_ADD_EQ_NOCARRY) THEN
           ASM_REWRITE_TAC[] THEN DISCH_TAC THEN ASM_ARITH_TAC ];
        ALL_TAC ] THEN
     SUBGOAL_THEN
      `val(word_sub (word_add (x:W word) (y:W word)) (pw:W word)) =
       val(x:W word) + val(y:W word) - val(pw:W word)`
       ASSUME_TAC THENL
      [ ASM_CASES_TAC `val(word_add (x:W word) (y:W word)) < val(x:W word)`
        THENL
         [ (* carry: q <= r is impossible *)
           SUBGOAL_THEN
            `~(val(pw:W word) <= val(word_add (x:W word) (y:W word)))`
            ASSUME_TAC THENL
            [ MP_TAC (ISPECL [`x:W word`; `y:W word`] WORD_ADD_EQ_CARRY) THEN
              ASM_REWRITE_TAC[] THEN DISCH_TAC THEN ASM_ARITH_TAC;
              ALL_TAC ] THEN
           ASM_SIMP_TAC[VAL_WORD_SUB_LT] THEN
           MP_TAC (ISPECL [`x:W word`; `y:W word`] WORD_ADD_EQ_CARRY) THEN
           ASM_REWRITE_TAC[] THEN DISCH_TAC THEN
           MATCH_MP_TAC(ARITH_RULE
            `!r m a b q:num. q <= m /\ r + m = a + b
             ==> r + (m - q) = a + b - q`) THEN
           ASM_REWRITE_TAC[] THEN ASM_ARITH_TAC;
           (* no carry: val = r - q = a + b - q *)
           SUBGOAL_THEN
            `val(pw:W word) <= val(word_add (x:W word) (y:W word))`
            ASSUME_TAC THENL [ASM_ARITH_TAC; ALL_TAC] THEN
           ASM_SIMP_TAC[VAL_WORD_SUB_GE] THEN
           MP_TAC (ISPECL [`x:W word`; `y:W word`] WORD_ADD_EQ_NOCARRY) THEN
           ASM_REWRITE_TAC[] THEN DISCH_TAC THEN ASM_ARITH_TAC ];
        ALL_TAC ] THEN
     ASM_REWRITE_TAC[] THEN
     SUBGOAL_THEN `(val(x:W word) + val(y:W word)) MOD val(pw:W word) =
                   val(x:W word) + val(y:W word) - val(pw:W word)`
       (fun th -> ACCEPT_TAC (SYM th)) THEN
     MATCH_MP_TAC MOD_SUB_ONE THEN ASM_ARITH_TAC;
     (* ---- keep branch ---- *)
     UNDISCH_TAC
      `~(word_ult (word_add (x:W word) (y:W word)) (x:W word) \/
         word_ule (pw:W word) (word_add (x:W word) (y:W word)))` THEN
     REWRITE_TAC[WORD_ULT; WORD_ULE; DE_MORGAN_THM] THEN
     STRIP_TAC THEN ASM_REWRITE_TAC[WORD_ULT; WORD_ULE] THEN
     SUBGOAL_THEN
      `val(word_add (x:W word) (y:W word)) = val(x:W word) + val(y:W word)`
      ASSUME_TAC THENL
      [ MP_TAC (ISPECL [`x:W word`; `y:W word`] WORD_ADD_EQ_NOCARRY) THEN
        ASM_REWRITE_TAC[];
        ALL_TAC ] THEN
     ASM_REWRITE_TAC[] THEN MATCH_MP_TAC MOD_LT THEN ASM_ARITH_TAC ]);;

let ADD_REF_VAL_WORD = prove
 (`!p (x:W word) (y:W word).
       0 < p /\ p < 2 EXP dimindex(:W) /\ val x < p /\ val y < p
       ==> val(ref_add (word p : W word) x y) = (val x + val y) MOD p`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  MATCH_MP_TAC ADD_REF_VAL THEN
  ASM_SIMP_TAC[VAL_WORD; MOD_LT]);;

Printf.printf "REF_LEMMA_DONE\n%!";;
