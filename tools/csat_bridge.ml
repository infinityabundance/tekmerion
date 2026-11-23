(* ========================================================================= *)
(* csat_bridge.ml -- development driver: derive steps -> simulate -> bridge  *)
(* -> certified SAT, for the P-256 linear family (training curve).           *)
(* ========================================================================= *)

#load "unix.cma";;
needs "arm/proofs/base.ml";;
needs "Cadical/make.ml";;
loadt (Sys.getenv "CSAT_LIB");;
loadt (Sys.getenv "BRIDGE_LIB");;

let t0 = Unix.gettimeofday ();;
let stamp s = Printf.printf "[%7.2f] %s\n%!" (Unix.gettimeofday () -. t0) s;;

(* does the shape lemma apply with the s2n-bignum expansion conversion *)
let EXPAND_BIGNUM = CONV_TAC (ONCE_DEPTH_CONV BIGNUM_OF_WORDLIST_CONV);;

(* ------------------------------------------------------------------------- *)
(* P-256 definitions (identical machine code to tools/w1_hol_capture.ml).    *)
(* ------------------------------------------------------------------------- *)

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

let bignum_sub_p256_mc = define_assert_from_elf "bignum_sub_p256_mc" "arm/p256/bignum_sub_p256.o"
[
  0xa9401825; 0xa9400c44; 0xeb0400a5; 0xfa0300c6; 0xa9412027; 0xa9410c44;
  0xfa0400e7; 0xfa030108; 0xda9f23e3; 0xab0300a5; 0x92407c64; 0xba0400c6;
  0xba1f00e7; 0x92608064; 0x9a040108; 0xa9001805; 0xa9012007; 0xd65f03c0
];;
let BIGNUM_SUB_P256_EXEC = ARM_MK_EXEC_RULE bignum_sub_p256_mc;;

let sub_goal =
 `!z x y m n pc.
        nonoverlapping (word pc,0x48) (z,8 * 4)
        ==> ensures arm
             (\s. aligned_bytes_loaded s (word pc) bignum_sub_p256_mc /\
                  read PC s = word pc /\ C_ARGUMENTS [z; x; y] s /\
                  bignum_from_memory (x,4) s = m /\
                  bignum_from_memory (y,4) s = n)
             (\s. read PC s = word (pc + 0x44) /\
                  (m < p_256 /\ n < p_256
                   ==> &(bignum_from_memory (z,4) s) = (&m - &n) rem &p_256))
          (MAYCHANGE [PC; X3; X4; X5; X6; X7; X8] ,,
           MAYCHANGE SOME_FLAGS ,, MAYCHANGE [events] ,,
           MAYCHANGE [memory :> bignum(z,4)])`;;

let sub_prefix =
  MAP_EVERY X_GEN_TAC
   [`z:int64`; `x:int64`; `y:int64`; `m:num`; `n:num`; `pc:num`] THEN
  REWRITE_TAC[C_ARGUMENTS; C_RETURN; SOME_FLAGS; NONOVERLAPPING_CLAUSES] THEN
  DISCH_THEN(REPEAT_TCL CONJUNCTS_THEN ASSUME_TAC) THEN
  REWRITE_TAC[BIGNUM_FROM_MEMORY_BYTES] THEN ENSURES_INIT_TAC "s0" THEN
  BIGNUM_DIGITIZE_TAC "m_" `read (memory :> bytes (x,8 * 4)) s0` THEN
  BIGNUM_DIGITIZE_TAC "n_" `read (memory :> bytes (y,8 * 4)) s0` THEN
  ARM_ACCSTEPS_TAC BIGNUM_SUB_P256_EXEC [3;4;7;8;10;12;13;15] (1--8);;

let sub_middle =
  SUBGOAL_THEN `carry_s8 <=> m < n` SUBST_ALL_TAC THENL
   [MATCH_MP_TAC FLAG_FROM_CARRY_LT THEN EXISTS_TAC `256` THEN
    MAP_EVERY EXPAND_TAC ["m"; "n"] THEN REWRITE_TAC[GSYM REAL_OF_NUM_ADD] THEN
    REWRITE_TAC[GSYM REAL_OF_NUM_MUL; GSYM REAL_OF_NUM_POW] THEN
    ACCUMULATOR_ASSUM_LIST(MP_TAC o end_itlist CONJ o DECARRY_RULE) THEN
    DISCH_THEN(fun th -> REWRITE_TAC[th]) THEN BOUNDER_TAC[];
    ALL_TAC];;

let red_sub =
  ENSURES_FINAL_STATE_TAC THEN ASM_REWRITE_TAC[] THEN STRIP_TAC THEN
  CONV_TAC(LAND_CONV(RAND_CONV BIGNUM_EXPAND_CONV)) THEN
  ASM_REWRITE_TAC[] THEN DISCARD_STATE_TAC "s17";;

let bignum_neg_p256_mc = define_assert_from_elf "bignum_neg_p256_mc" "arm/p256/bignum_neg_p256.o"
[
  0xa9401424; 0xa9411c26; 0xaa050083; 0xaa0700c2; 0xaa030042; 0xf100005f;
  0xda9f03e2; 0xeb040044; 0x92407c43; 0xfa050065; 0xfa0603e6; 0x92608043;
  0xda070067; 0xa9001404; 0xa9011c06; 0xd65f03c0
];;
let BIGNUM_NEG_P256_EXEC = ARM_MK_EXEC_RULE bignum_neg_p256_mc;;

let neg_goal =
 `!z x n pc.
        nonoverlapping (word pc,0x40) (z,8 * 4)
        ==> ensures arm
             (\s. aligned_bytes_loaded s (word pc) bignum_neg_p256_mc /\
                  read PC s = word pc /\ C_ARGUMENTS [z; x] s /\
                  bignum_from_memory (x,4) s = n)
             (\s. read PC s = word (pc + 0x3c) /\
                  (n <= p_256
                   ==> bignum_from_memory (z,4) s = (p_256 - n) MOD p_256))
          (MAYCHANGE [PC; X2; X3; X4; X5; X6; X7] ,,
           MAYCHANGE SOME_FLAGS ,, MAYCHANGE [events] ,,
           MAYCHANGE [memory :> bignum(z,4)])`;;

let neg_prefix =
  MAP_EVERY X_GEN_TAC [`z:int64`; `x:int64`; `n:num`; `pc:num`] THEN
  REWRITE_TAC[C_ARGUMENTS; C_RETURN; SOME_FLAGS; NONOVERLAPPING_CLAUSES] THEN
  DISCH_THEN(REPEAT_TCL CONJUNCTS_THEN ASSUME_TAC) THEN
  REWRITE_TAC[BIGNUM_FROM_MEMORY_BYTES] THEN ENSURES_INIT_TAC "s0" THEN
  BIGNUM_DIGITIZE_TAC "n_" `read (memory :> bytes (x,8 * 4)) s0` THEN
  ARM_ACCSTEPS_TAC BIGNUM_NEG_P256_EXEC [8;10;11;13] (1--15);;

let red_neg =
  ENSURES_FINAL_STATE_TAC THEN ASM_REWRITE_TAC[] THEN STRIP_TAC THEN
  CONV_TAC(LAND_CONV BIGNUM_EXPAND_CONV) THEN
  ASM_REWRITE_TAC[] THEN DISCARD_STATE_TAC "s15" THEN
  REWRITE_TAC[GSYM REAL_OF_NUM_CLAUSES];;

(* ------------------------------------------------------------------------- *)
(* driver                                                                    *)
(* ------------------------------------------------------------------------- *)

let showtag tag =
  let (_,gs,_) = hd(!current_goalstack) in
  (match gs with
   | [(asl,w)] ->
       Printf.printf "STAGE %s asms=%d\nCONCL %s\n" tag (List.length asl)
         (String.concat " " (String.split_on_char '\n' (string_of_term w)))
   | _ -> Printf.printf "STAGE %s goals=%d\n" tag (List.length gs));
  flush stdout;;

let trye tag tac =
  try (e tac; true)
  with Failure s ->
    Printf.printf "STAGE_FAIL %s : %s\n" tag
      (String.concat " " (String.split_on_char '\n' s));
    flush stdout; false;;

let run_op op goal sim red bridge =
  stamp (op ^ ": start");
  set_goal([], goal);
  if trye (op^".sim") sim then begin
    showtag (op^".residual");
    let tb = Unix.gettimeofday () in
    if trye (op^".red") red then begin
      showtag (op^".red");
      if trye (op^".bridge") (bridge ()) then begin
        let th = b() in
        Printf.printf "RESULT %s closed=1 hyps=%d seconds=%.3f\n"
          op (List.length (hyp th)) (Unix.gettimeofday () -. tb)
      end else
        Printf.printf "RESULT %s closed=0 seconds=%.3f\n"
          op (Unix.gettimeofday () -. tb)
    end
  end;
  flush stdout;;

stamp "loading done";;

run_op "add" add_goal add_prefix red_add
  (fun () -> BRIDGE_ADD_TAC EXPAND_BIGNUM p_256);;

run_op "sub" sub_goal (sub_prefix THEN sub_middle THEN
   ARM_STEPS_TAC BIGNUM_SUB_P256_EXEC [9] THEN
   RULE_ASSUM_TAC(REWRITE_RULE[WORD_UNMASK_64; NOT_LE]) THEN
   ARM_ACCSTEPS_TAC BIGNUM_SUB_P256_EXEC [3;4;7;8;10;12;13;15] (10--17)) red_sub
  (fun () -> BRIDGE_SUB_TAC EXPAND_BIGNUM p_256);;

run_op "neg" neg_goal neg_prefix red_neg
  (fun () -> BRIDGE_NEG_TAC EXPAND_BIGNUM p_256);;

Printf.printf "BRIDGE_DRIVER_DONE\n%!";;
