#load "unix.cma";;
needs "arm/proofs/base.ml";;
Printf.printf "MOD_UNIQ: %s\n" (string_of_thm MOD_UNIQ);;
set_goal([], `!(x:W word) (y:W word).
   ~(val x + val y < 2 EXP dimindex(:W))
   ==> val(word_add x y) = val x + val y - 2 EXP dimindex(:W)`);;
e (REPEAT GEN_TAC THEN DISCH_TAC);;
e (ASM_SIMP_TAC[VAL_WORD_ADD_CASES]);;
Printf.printf "=== add_ge residual ===\n";;
p();;

set_goal([], `!s q:num. q <= s /\ s < 2*q ==> s MOD q = s - q`);;
e (REPEAT GEN_TAC THEN STRIP_TAC);;
e (MATCH_MP_TAC MOD_UNIQ THEN EXISTS_TAC `1`);;
Printf.printf "=== mod_sub residual ===\n";;
p();;
