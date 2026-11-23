(* Generic reference-function lemma: abstract the width to a variable m. *)
#load "unix.cma";;
needs "arm/proofs/base.ml";;

let SUB_MOVE = prove
 (`!r m q:num. q <= m ==> r + (m - q) = (r + m) - q`,
  REPEAT GEN_TAC THEN REWRITE_TAC[LE_EXISTS] THEN
  DISCH_THEN(X_CHOOSE_TAC `d:num`) THEN
  ASM_REWRITE_TAC[ADD_SUB2] THEN
  SUBGOAL_THEN `r + (q + d) = q + (r + d)` (fun th -> REWRITE_TAC[th; ADD_SUB2]) THEN
  ARITH_TAC);;

let SUB_CANCEL = prove
 (`!s q m:num. q <= m /\ m <= s ==> (s - m) + (m - q) = s - q`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN ARITH_TAC);;

let MOD_SUB_ONE = prove
 (`!s q:num. q <= s /\ s < 2 * q ==> s MOD q = s - q`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  MATCH_MP_TAC MOD_UNIQ THEN EXISTS_TAC `1` THEN
  REWRITE_TAC[MULT_CLAUSES] THEN CONJ_TAC THENL
   [ REWRITE_TAC[ADD_SYM] THEN MATCH_MP_TAC SUB_ADD THEN ASM_REWRITE_TAC[];
     ASM_ARITH_TAC ]);;

let MOD_ADD_GE_VAR = prove
 (`!a b m:num. a < m /\ b < m /\ ~(a + b < m)
       ==> (a + b) MOD m = a + b - m`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  MP_TAC (ISPECL [`a:num`; `b:num`; `m:num`] MOD_ADD_CASES) THEN
  ASM_REWRITE_TAC[] THEN DISCH_TAC THEN ASM_REWRITE_TAC[]);;

let SUB_MOD = prove
 (`!s q m:num. q <= s /\ s < m ==> (s + (m - q)) MOD m = s - q`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  MATCH_MP_TAC MOD_UNIQ THEN EXISTS_TAC `1` THEN
  REWRITE_TAC[MULT_CLAUSES] THEN CONJ_TAC THENL
   [ MP_TAC (ISPECL [`s:num`; `m:num`; `q:num`] SUB_MOVE) THEN
     ASM_REWRITE_TAC[] THEN DISCH_TAC THEN
     SUBGOAL_THEN `(s + m) - q = m + (s - q)`
       (fun th -> ASM_REWRITE_TAC[th]) THEN
     REWRITE_TAC[LE_EXISTS] THEN
     SUBGOAL_THEN `?d:num. s = q + d` (X_CHOOSE_TAC `d:num`) THENL
      [ ASM_MESON_TAC[LE_EXISTS]; ALL_TAC ] THEN
     ASM_REWRITE_TAC[ADD_SUB2] THEN
     SUBGOAL_THEN `q + d + m = q + (d + m)`
       (fun th -> REWRITE_TAC[th; ADD_SUB2]) THEN
     ARITH_TAC;
     ASM_ARITH_TAC ]);;

let ref_add = new_definition
 `ref_add (pw:W word) (x:W word) (y:W word) =
    let s = word_add x y in
    if word_ult s x \/ word_ule pw s then word_sub s pw else s`;;

let ADD_REF_VAL = prove
 (`!(pw:W word) (x:W word) (y:W word).
       0 < val pw /\ val x < val pw /\ val y < val pw
       ==> val(ref_add pw x y) = (val x + val y) MOD val pw`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  SUBGOAL_THEN `?m:num. m = 2 EXP dimindex(:W)` (X_CHOOSE_TAC `m:num`) THENL
   [ EXISTS_TAC `2 EXP dimindex(:W)` THEN REFL_TAC; ALL_TAC ] THEN
  SUBGOAL_THEN `2 EXP dimindex(:W) = m` (fun hM ->
    RULE_ASSUM_TAC(REWRITE_RULE[hM]) THEN REWRITE_TAC[hM] THEN
    SUBGOAL_THEN `0 < m` ASSUME_TAC THENL
     [ REWRITE_TAC[GSYM hM; EXP_EQ_0; ARITH]; ALL_TAC ] THEN
    SUBGOAL_THEN `val(x:W word) < m /\ val(y:W word) < m /\
                  val(pw:W word) < m`
      STRIP_ASSUME_TAC THENL
     [ REWRITE_TAC[VAL_BOUND] THEN REWRITE_TAC[hM]; ALL_TAC ] THEN
    SUBGOAL_THEN
     `val(word_add (x:W word) (y:W word)) = (val x + val y) MOD m`
      ASSUME_TAC THENL [REWRITE_TAC[VAL_WORD_ADD; hM]; ALL_TAC] THEN
    REWRITE_TAC[ref_add; WORD_ULT; WORD_ULE] THEN
    ASM_REWRITE_TAC[] THEN
    ASM_CASES_TAC
     `(val(x:W word) + val(y:W word)) MOD m < val(x:W word) \/
      val(pw:W word) <= (val(x:W word) + val(y:W word)) MOD m` THENL
     [ (* ---- subtract ---- *)
       SUBGOAL_THEN
        `val(word_sub (word_add (x:W word) (y:W word)) (pw:W word)) =
         (((val x + val y) MOD m) + (m - val pw)) MOD m`
         ASSUME_TAC THENL
        [ REWRITE_TAC[VAL_WORD_SUB; hM] THEN ASM_REWRITE_TAC[]; ALL_TAC ] THEN
       ASM_REWRITE_TAC[] THEN
       ASM_CASES_TAC `val(x:W word) + val(y:W word) < m` THENL
        [ (* no carry *)
          SUBGOAL_THEN
           `(val(x:W word) + val(y:W word)) MOD m =
            val(x:W word) + val(y:W word)`
            ASSUME_TAC THENL [MATCH_MP_TAC MOD_LT THEN ASM_REWRITE_TAC[]; ALL_TAC] THEN
          ASM_REWRITE_TAC[] THEN
          SUBGOAL_THEN `val(pw:W word) <= val(x:W word) + val(y:W word)`
            ASSUME_TAC THENL [ASM_ARITH_TAC; ALL_TAC] THEN
          SUBGOAL_THEN `val(x:W word) + val(y:W word) < 2 * val(pw:W word)`
            ASSUME_TAC THENL [ASM_ARITH_TAC; ALL_TAC] THEN
          SUBGOAL_THEN
           `(val(x:W word) + val(y:W word) + (m - val pw)) MOD m =
            val(x:W word) + val(y:W word) - val(pw:W word)`
            (fun th -> REWRITE_TAC[th]) THENL
           [ MATCH_MP_TAC SUB_MOD THEN ASM_REWRITE_TAC[]; ALL_TAC ] THEN
          SUBGOAL_THEN
           `(val(x:W word) + val(y:W word)) MOD val(pw:W word) =
            val(x:W word) + val(y:W word) - val(pw:W word)`
            (fun th -> REWRITE_TAC[th]) THEN
          MATCH_MP_TAC MOD_SUB_ONE THEN ASM_REWRITE_TAC[];
          (* carry *)
          SUBGOAL_THEN
           `(val(x:W word) + val(y:W word)) MOD m =
            val(x:W word) + val(y:W word) - m`
            ASSUME_TAC THENL
           [ MATCH_MP_TAC MOD_ADD_GE_VAR THEN ASM_REWRITE_TAC[]; ALL_TAC ] THEN
          ASM_REWRITE_TAC[] THEN
          SUBGOAL_THEN `val(pw:W word) <= val(x:W word) + val(y:W word)`
            ASSUME_TAC THENL [ASM_ARITH_TAC; ALL_TAC ] THEN
          SUBGOAL_THEN `val(x:W word) + val(y:W word) < 2 * val(pw:W word)`
            ASSUME_TAC THENL [ASM_ARITH_TAC; ALL_TAC] THEN
          SUBGOAL_THEN `val(pw:W word) <= m` ASSUME_TAC THENL
           [ASM_ARITH_TAC; ALL_TAC] THEN
          SUBGOAL_THEN
           `(val(x:W word) + val(y:W word) - m) + (m - val pw) =
            val(x:W word) + val(y:W word) - val(pw:W word)`
            ASSUME_TAC THENL
           [ MATCH_MP_TAC SUB_CANCEL THEN ASM_REWRITE_TAC[]; ALL_TAC ] THEN
          ASM_REWRITE_TAC[] THEN
          SUBGOAL_THEN
           `val(x:W word) + val(y:W word) - val(pw:W word) < m`
            ASSUME_TAC THENL [ASM_ARITH_TAC; ALL_TAC] THEN
          SUBGOAL_THEN
           `(val(x:W word) + val(y:W word) - val(pw:W word)) MOD m =
            val(x:W word) + val(y:W word) - val(pw:W word)`
            (fun th -> REWRITE_TAC[th]) THENL
           [ MATCH_MP_TAC MOD_LT THEN ASM_REWRITE_TAC[]; ALL_TAC ] THEN
          SUBGOAL_THEN
           `(val(x:W word) + val(y:W word)) MOD val(pw:W word) =
            val(x:W word) + val(y:W word) - val(pw:W word)`
            (fun th -> REWRITE_TAC[th]) THEN
          MATCH_MP_TAC MOD_SUB_ONE THEN ASM_REWRITE_TAC[] ];
       (* ---- keep ---- *)
       ASM_REWRITE_TAC[] THEN
       ASM_CASES_TAC `val(x:W word) + val(y:W word) < m` THENL
        [ SUBGOAL_THEN
           `(val(x:W word) + val(y:W word)) MOD m =
            val(x:W word) + val(y:W word)`
            ASSUME_TAC THENL [MATCH_MP_TAC MOD_LT THEN ASM_REWRITE_TAC[]; ALL_TAC] THEN
          ASM_REWRITE_TAC[] THEN
          SUBGOAL_THEN `val(x:W word) + val(y:W word) < val(pw:W word)`
            ASSUME_TAC THENL [ASM_ARITH_TAC; ALL_TAC] THEN
          MATCH_MP_TAC MOD_LT THEN ASM_REWRITE_TAC[];
          SUBGOAL_THEN
           `(val(x:W word) + val(y:W word)) MOD m =
            val(x:W word) + val(y:W word) - m`
            ASSUME_TAC THENL
           [ MATCH_MP_TAC MOD_ADD_GE_VAR THEN ASM_REWRITE_TAC[]; ALL_TAC ] THEN
          ASM_REWRITE_TAC[] THEN
          SUBGOAL_THEN
           `val(x:W word) + val(y:W word) - m < val(x:W word)`
            ASSUME_TAC THENL [ASM_ARITH_TAC; ASM_MESON_TAC[] ] ] ]) THEN
  ASM_MESON_TAC[]);;

let ADD_REF_VAL_WORD = prove
 (`!p (x:W word) (y:W word).
       0 < p /\ p < 2 EXP dimindex(:W) /\ val x < p /\ val y < p
       ==> val(ref_add (word p : W word) x y) = (val x + val y) MOD p`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  MATCH_MP_TAC ADD_REF_VAL THEN
  ASM_SIMP_TAC[VAL_WORD; MOD_LT]);;

Printf.printf "REF3_DONE\n%!";;
