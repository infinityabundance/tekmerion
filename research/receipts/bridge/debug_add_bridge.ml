(* Debug: replicate BRIDGE_ADD_TAC with per-subgoal logging of the exact
   implication passed to certified SAT.  Uses the verified bridge.ml. *)
#load "unix.cma";;
needs "arm/proofs/base.ml";;
needs "Cadical/make.ml";;
loadt "/mnt/1tb_kingston/Tekmerion/tools/csat_lib.ml";;
loadt "/mnt/1tb_kingston/Tekmerion/tools/bridge.ml";;

let p_256 = new_definition
 `p_256 = 115792089210356248762697446949407573530086143415290314195533631308867097853951`;;

let bignum_add_p256_mc = define_assert_from_elf "bignum_add_p256_mc" "arm/p256/bignum_add_p256.o"
[
  0xa9401424; 0xa9402448; 0xab080084; 0xba0900a5; 0xa9411c26; 0xa9412c4a;
  0xba0a00c6; 0xba0b00e7; 0x9a1f03e3; 0xb1000488; 0xb2407fe9; 0xfa0900a9;
  0xfa1f00ca; 0xb26083eb; 0xfa0b00eb; 0xfa1f0063; 0x9a883084; 0x9a8930a5;
  0x9a8a30c6; 0x9a8b30e7; 0xa9001404; 0xa9011c06; 0xd65f03c0
];;
let BIGNUM_ADD_P256_EXEC = ARM_MK_EXEC_RULE bignum_add_p256_mc;;

let add_goal =
 `!z x y m n pc.
        nonoverlapping (word pc,0x5c) (z,8 * 4)
        ==> ensures arm
             (\s. aligned_bytes_loaded s (word pc) bignum_add_p256_mc /\
                  read PC s = word pc /\ C_ARGUMENTS [z; x; y] s /\
                  bignum_from_memory (x,4) s = m /\
                  bignum_from_memory (y,4) s = n)
             (\s. read PC s = word (pc + 0x58) /\
                  (m < p_256 /\ n < p_256
                   ==> bignum_from_memory (z,4) s = (m + n) MOD p_256))
          (MAYCHANGE [PC; X3; X4; X5; X6; X7; X8; X9; X10; X11] ,,
           MAYCHANGE SOME_FLAGS ,, MAYCHANGE [events] ,,
           MAYCHANGE [memory :> bignum(z,4)])`;;

let add_prefix =
  MAP_EVERY X_GEN_TAC
   [`z:int64`; `x:int64`; `y:int64`; `m:num`; `n:num`; `pc:num`] THEN
  REWRITE_TAC[C_ARGUMENTS; C_RETURN; SOME_FLAGS; NONOVERLAPPING_CLAUSES] THEN
  DISCH_THEN(REPEAT_TCL CONJUNCTS_THEN ASSUME_TAC) THEN
  REWRITE_TAC[BIGNUM_FROM_MEMORY_BYTES] THEN ENSURES_INIT_TAC "s0" THEN
  BIGNUM_DIGITIZE_TAC "m_" `read (memory :> bytes (x,8 * 4)) s0` THEN
  BIGNUM_DIGITIZE_TAC "n_" `read (memory :> bytes (y,8 * 4)) s0` THEN
  ARM_ACCSTEPS_TAC BIGNUM_ADD_P256_EXEC [3;4;7;8;10;12;13;15;16] (1--22);;

let red_add =
  ENSURES_FINAL_STATE_TAC THEN ASM_REWRITE_TAC[] THEN STRIP_TAC THEN
  RULE_ASSUM_TAC(REWRITE_RULE [REAL_BITVAL_NOT; ADD_CLAUSES; VAL_WORD_BITVAL]) THEN
  CONV_TAC SYM_CONV THEN CONV_TAC(RAND_CONV BIGNUM_LEXPAND_CONV) THEN
  ASM_REWRITE_TAC[] THEN DISCARD_STATE_TAC "s22";;

let EXPAND_BIGNUM = CONV_TAC (ONCE_DEPTH_CONV BIGNUM_OF_WORDLIST_CONV);;

let ngoal = ref 0;;
let DEBUG_CSAT_TAC : tactic =
  fun (asl,w) ->
    let hyps = map (fun (_,th) -> concl th) asl in
    let imp = itlist (fun a b -> mk_imp(a,b)) hyps w in
    incr ngoal;
    Printf.printf "DEBUG_CSAT_%d_START\n%s\nDEBUG_CSAT_%d_END\n%!" !ngoal (string_of_term imp) !ngoal;
    let t0 = Unix.gettimeofday () in
    let th =
      try prove(imp, CSAT_TAC)
      with e ->
        Printf.printf "DEBUG_CSAT_%d_FAIL %s %.3f\n%!" !ngoal (Printexc.to_string e)
          (Unix.gettimeofday () -. t0);
        raise e in
    Printf.printf "DEBUG_CSAT_%d_OK %.3f\n%!" !ngoal (Unix.gettimeofday () -. t0);
    (MP_TAC th THEN ASM_REWRITE_TAC[]) (asl,w);;

let DEBUG_BRIDGE_PREPARE_TAC expand : tactic =
  fun g -> (BRIDGE_SPLIT_CONDITIONS_TAC THEN expand THEN
            RULE_ASSUM_TAC (CONV_RULE BRIDGE_TO_WORD_CONV) THEN
            CONV_TAC BRIDGE_TO_WORD_CONV) g;;

let DEBUG_BRIDGE_ADD_TAC expand pth : tactic =
  ELIM_DIGIT_EQS_TAC THEN BRIDGE_REWRITE_MODULUS pth THEN
  (fun (asl,w) ->
    let (lhs,_) = dest_eq w in
    let (sum,p) = dest_binop `(MOD):num->num->num` lhs in
    let cond = mk_comb(mk_comb(`(<):num->num->bool`, sum), p) in
    (BRIDGE_ZERO_LT_TAC p THEN
     ASM_CASES_TAC cond THENL
      [ASM_SIMP_TAC[ADD_MOD_LT] THEN DEBUG_BRIDGE_PREPARE_TAC expand THEN DEBUG_CSAT_TAC;
       SUBGOAL_THEN (mk_comb(mk_comb(`(<=):num->num->bool`, p), sum)) ASSUME_TAC THENL
         [ASM_ARITH_TAC; ALL_TAC] THEN
       ASM_SIMP_TAC[ADD_MOD_GE] THEN CONV_TAC SYM_CONV THEN
       ASM_SIMP_TAC[ADD_GE_SUM] THEN DEBUG_BRIDGE_PREPARE_TAC expand THEN DEBUG_CSAT_TAC]) (asl,w));;

set_goal([], add_goal);;
e add_prefix;; e red_add;;
let tb = Unix.gettimeofday ();;
let closed = try (e (DEBUG_BRIDGE_ADD_TAC EXPAND_BIGNUM p_256); true)
  with e -> Printf.printf "DEBUG_BRIDGE_FAIL %s\n%!" (Printexc.to_string e); false;;
Printf.printf "DEBUG_BRIDGE_DONE closed=%b goals=%d seconds=%.3f\n%!" closed
  (if closed then 0 else List.length (fst (hd(!current_goalstack)))) (Unix.gettimeofday () -. tb);;
Printf.printf "PROBE_DONE\n%!";;
