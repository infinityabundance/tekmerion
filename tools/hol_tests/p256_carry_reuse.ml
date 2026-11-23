(* DEVELOPMENT_ONLY: reuse check for BRIDGE_CARRY_CONSERVE_TAC on P-256 add. *)
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
set_goal([], add_goal);;
e (MAP_EVERY X_GEN_TAC
    [`z:int64`; `x:int64`; `y:int64`; `m:num`; `n:num`; `pc:num`] THEN
   REWRITE_TAC[C_ARGUMENTS; C_RETURN; SOME_FLAGS; NONOVERLAPPING_CLAUSES] THEN
   DISCH_THEN(REPEAT_TCL CONJUNCTS_THEN ASSUME_TAC) THEN
   REWRITE_TAC[BIGNUM_FROM_MEMORY_BYTES] THEN ENSURES_INIT_TAC "s0" THEN
   BIGNUM_DIGITIZE_TAC "m_" `read (memory :> bytes (x,8 * 4)) s0` THEN
   BIGNUM_DIGITIZE_TAC "n_" `read (memory :> bytes (y,8 * 4)) s0` THEN
   ARM_ACCSTEPS_TAC BIGNUM_ADD_P256_EXEC (1--9) (1--9) THEN
   BRIDGE_CARRY_CONSERVE_TAC);;
let (_,gs,_) = hd(!current_goalstack) in
(match gs with
 | [(asl,_)] ->
     List.iter (fun (_,th) ->
       let t = concl th in
       if can (term_match [] `bignum_of_wordlist (a:int64 list) +
                                  bignum_of_wordlist (b:int64 list) +
                                  bitval (c:bool) =
                                  bignum_of_wordlist (d:int64 list) +
                                  2 EXP (e:num) * bitval (f:bool)`) t
       then (Printf.printf "P256_CARRY %s\n%!" (string_of_term t);
             Printf.printf "P256_CARRY_HYPS %d\n%!" (List.length (hyp th))))
       asl
 | _ -> Printf.printf "P256_MULTI\n%!");;
Printf.printf "P256_REUSE_DONE\n%!";;
