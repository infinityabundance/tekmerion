(* Diagnostic: at the exact point where certified SAT is called, list every
   int-level atom in the prepared residue and report whether IWORDIFY_ATOM_CONV
   can wordify it.  An atom it cannot wordify is left opaque by BITBLAST and
   becomes a free proposition in the CNF -- which is exactly how a true
   residue can come back SATISFIABLE.
   Uses the verified tools/bridge.ml unchanged. *)
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

(* --- atom reporter ------------------------------------------------------- *)
let is_int_atom tm =
  match tm with
  | Comb(Comb(Const(c,_),_),_) when type_of tm = `:bool` &&
       (c = "int_lt" || c = "int_le" || c = "int_gt" || c = "int_ge") -> true
  | Comb(Comb(Const("=",_),_),_) when type_of tm = `:bool` &&
       type_of (rand tm) = `:int` -> true
  | _ -> false;;

let print_itype tag t =
  Printf.printf "%s    ival-operand type=%s\n%!" tag (string_of_type(type_of t));;

let report_atoms tag tm =
  let raw = find_terms is_int_atom tm in
  let rec dedup acc = function
    | [] -> List.rev acc
    | h::t -> if List.exists (fun x -> aconv x h) acc then dedup acc t
              else dedup (h::acc) t in
  let atms = dedup [] raw in
  Printf.printf "%s INT_ATOMS total=%d\n%!" tag (List.length atms);
  List.iter (fun a ->
    (* whole-atom wordification *)
    let ok = try ignore(IWORDIFY_ATOM_CONV a); true with _ -> false in
    Printf.printf "%s  ATOM ok=%b %s\n%!" tag ok (string_of_term a);
    (* if the two sides are int terms, what widths does IWORDIFY_CONV pick? *)
    (try
       let th = BINOP_CONV IWORDIFY_CONV a in
       let l = lhand(concl th) and r = rand(concl th) in
       (try print_itype tag (rand l) with _ -> ());
       (try print_itype tag (rand r) with _ -> ())
     with _ -> Printf.printf "%s    (BINOP IWORDIFY_CONV itself failed)\n%!" tag)
  ) atms;;

let REPORT_CSAT_TAC : tactic =
  fun (asl,w) ->
    let hyps = map (fun (_,th) -> concl th) asl in
    let imp = itlist (fun a b -> mk_imp(a,b)) hyps w in
    Printf.printf "\n=== RESIDUE ===\n%s\n=== END ===\n%!" (string_of_term imp);
    report_atoms "R" imp;
    (* leave the goal untouched; the script exits with the goal unproved. *)
    ALL_TAC (asl,w);;

let REPORT_BRIDGE_ADD_TAC expand pth : tactic =
  ELIM_DIGIT_EQS_TAC THEN BRIDGE_REWRITE_MODULUS pth THEN
  (fun (asl,w) ->
    let (lhs,_) = dest_eq w in
    let (sum,p) = dest_binop `(MOD):num->num->num` lhs in
    let cond = mk_comb(mk_comb(`(<):num->num->bool`, sum), p) in
    (BRIDGE_ZERO_LT_TAC p THEN
     ASM_CASES_TAC cond THENL
      [ASM_SIMP_TAC[ADD_MOD_LT] THEN
       (BRIDGE_SPLIT_CONDITIONS_TAC THEN expand THEN
        RULE_ASSUM_TAC (CONV_RULE BRIDGE_TO_WORD_CONV) THEN
        CONV_TAC BRIDGE_TO_WORD_CONV THEN REPORT_CSAT_TAC);
       SUBGOAL_THEN (mk_comb(mk_comb(`(<=):num->num->bool`, p), sum))
         ASSUME_TAC THENL [ASM_ARITH_TAC; ALL_TAC] THEN
       ASM_SIMP_TAC[ADD_MOD_GE] THEN CONV_TAC SYM_CONV THEN
       ASM_SIMP_TAC[ADD_GE_SUM] THEN
       (BRIDGE_SPLIT_CONDITIONS_TAC THEN expand THEN
        RULE_ASSUM_TAC (CONV_RULE BRIDGE_TO_WORD_CONV) THEN
        CONV_TAC BRIDGE_TO_WORD_CONV THEN REPORT_CSAT_TAC)]) (asl,w));;

set_goal([], add_goal);;
e add_prefix;; e red_add;;
let _ = try ignore(e (REPORT_BRIDGE_ADD_TAC EXPAND_BIGNUM p_256))
  with e -> Printf.printf "DIAG_EXN %s\n%!" (Printexc.to_string e);;
Printf.printf "DIAG_DONE\n%!";;
Printf.printf "PROBE_DONE\n%!";;
