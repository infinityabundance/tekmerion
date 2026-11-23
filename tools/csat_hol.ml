(* ========================================================================= *)
(* csat_hol.ml -- certified-SAT close on the P-256 linear post-simulation     *)
(* goals (add, sub, neg).                                                    *)
(*                                                                           *)
(* For each operation: run the simulated prefix, then the deterministic       *)
(* reductions that tools/w4_close.py applies before its congruence            *)
(* application, and attempt the certified-SAT close at each stage.  The       *)
(* receipt therefore shows exactly which construct defeats SAT.               *)
(*                                                                           *)
(* The only trusted component is the LCF kernel, which replays every LRAT     *)
(* refutation (tools/csat_lib.ml).                                           *)
(* ========================================================================= *)

#load "unix.cma";;

needs "arm/proofs/base.ml";;
needs "Cadical/make.ml";;
loadt (Sys.getenv "CSAT_LIB");;

let env_anums name default =
  let s = try Sys.getenv name with Not_found -> "" in
  if s = "" then default else map int_of_string (String.split_on_char ';' s);;

let anums_add = env_anums "CSAT_ANUMS_ADD" [3;4;7;8;10;12;13;15;16];;
let anums_sub = env_anums "CSAT_ANUMS_SUB" [3;4;7;8;10;12;13;15];;
let anums_neg = env_anums "CSAT_ANUMS_NEG" [8;10;11;13];;

(* ------------------------------------------------------------------------- *)
(* P-256 definitions (identical machine code to tools/w1_hol_capture.ml).    *)
(* ------------------------------------------------------------------------- *)

let p_256 = new_definition `p_256 = 115792089210356248762697446949407573530086143415290314195533631308867097853951`;;

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
  ARM_ACCSTEPS_TAC BIGNUM_ADD_P256_EXEC anums_add (1--22);;

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
  ARM_ACCSTEPS_TAC BIGNUM_SUB_P256_EXEC anums_sub (1--8);;

let sub_middle =
  SUBGOAL_THEN `carry_s8 <=> m < n` SUBST_ALL_TAC THENL
   [MATCH_MP_TAC FLAG_FROM_CARRY_LT THEN EXISTS_TAC `256` THEN
    MAP_EVERY EXPAND_TAC ["m"; "n"] THEN REWRITE_TAC[GSYM REAL_OF_NUM_ADD] THEN
    REWRITE_TAC[GSYM REAL_OF_NUM_MUL; GSYM REAL_OF_NUM_POW] THEN
    ACCUMULATOR_ASSUM_LIST(MP_TAC o end_itlist CONJ o DECARRY_RULE) THEN
    DISCH_THEN(fun th -> REWRITE_TAC[th]) THEN BOUNDER_TAC[];
    ALL_TAC];;

let sub_simulate =
  sub_prefix THEN sub_middle THEN
  ARM_STEPS_TAC BIGNUM_SUB_P256_EXEC [9] THEN
  RULE_ASSUM_TAC(REWRITE_RULE[WORD_UNMASK_64; NOT_LE]) THEN
  ARM_ACCSTEPS_TAC BIGNUM_SUB_P256_EXEC anums_sub (10--17);;

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
  ARM_ACCSTEPS_TAC BIGNUM_NEG_P256_EXEC anums_neg (1--15);;

(* ------------------------------------------------------------------------- *)
(* Goal capture and certified-SAT attempts.                                  *)
(* ------------------------------------------------------------------------- *)

let stages = ref [];;
let dump_dir = try Sys.getenv "CSAT_DUMP_DIR" with Not_found -> "";;

let canon s =
  let b = Buffer.create (String.length s) in
  let sp = ref true in
  String.iter
    (fun c ->
      if c = ' ' || c = '\n' || c = '\t' || c = '\r' then
        (if not !sp then Buffer.add_char b ' '; sp := true)
      else (Buffer.add_char b c; sp := false)) s;
  Buffer.contents b;;

let dump_stage tag gs =
  if dump_dir <> "" then begin
    let oc = open_out (Filename.concat dump_dir (tag ^ ".txt")) in
    List.iter
      (fun (asl,w) ->
         Printf.fprintf oc "GOAL ASMS %d\n" (List.length asl);
         List.iter
           (fun (_,th) ->
              Printf.fprintf oc "ASM %s\n" (canon (string_of_term (concl th))))
           asl;
         Printf.fprintf oc "CONCL %s\n" (canon (string_of_term w)))
      gs;
    close_out oc
  end;;

let capture tag =
  let (_,gs,_) = hd(!current_goalstack) in
  stages := !stages @ [(tag, gs)];
  dump_stage tag gs;
  Printf.printf "CSAT_STAGE %s goals=%d\n" tag (List.length gs);
  flush stdout;;

let try_e tag tac =
  try (e tac; true)
  with Failure s ->
    Printf.printf "CSAT_STAGE_FAIL %s %s\n" tag
      (String.concat " " (String.split_on_char '\n' s));
    flush stdout; false;;

let attempt_all tag =
  List.iter
    (fun (stage,gs) ->
       let asl,w = match gs with [g] -> g | g::_ -> g | [] -> failwith "empty" in
       let name = stage in
       Printf.printf "CSAT_ATTEMPT %s goals=%d asms=%d\n"
         name (List.length gs) (List.length asl); flush stdout;
       report_attempt name (csat_attempt_goal (asl,w)))
    !stages;;

(* reductions applied before the congruence: exactly w4_close's prefix *)
let red_ensures =
  ENSURES_FINAL_STATE_TAC THEN ASM_REWRITE_TAC[] THEN STRIP_TAC;;

let red_add =
  RULE_ASSUM_TAC(REWRITE_RULE [REAL_BITVAL_NOT; ADD_CLAUSES; VAL_WORD_BITVAL]) THEN
  CONV_TAC SYM_CONV THEN CONV_TAC(RAND_CONV BIGNUM_LEXPAND_CONV) THEN
  ASM_REWRITE_TAC[] THEN DISCARD_STATE_TAC "s22";;

let red_sub =
  CONV_TAC(LAND_CONV(RAND_CONV BIGNUM_EXPAND_CONV)) THEN
  ASM_REWRITE_TAC[] THEN DISCARD_STATE_TAC "s17";;

let red_neg =
  CONV_TAC(LAND_CONV BIGNUM_EXPAND_CONV) THEN
  ASM_REWRITE_TAC[] THEN DISCARD_STATE_TAC "s15" THEN
  REWRITE_TAC[GSYM REAL_OF_NUM_CLAUSES];;

let run_one tag goal sim red =
  stages := [];
  set_goal([], goal);
  if try_e (tag ^ ".sim") sim then begin
    capture (tag ^ ".raw");
    if try_e (tag ^ ".ensures") red_ensures then begin
      capture (tag ^ ".ensures");
      if try_e (tag ^ ".expand") red then capture (tag ^ ".expand")
    end
  end;
  attempt_all tag;;

run_one "add" add_goal add_prefix red_add;;
run_one "sub" sub_goal sub_simulate red_sub;;
run_one "neg" neg_goal neg_prefix red_neg;;

Printf.printf "CSAT_POST_DONE\n";;
flush stdout;;
