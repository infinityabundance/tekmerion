(* V8.4 -- corrected confirmation experiment.

   Earlier attempts built their lemmas with `&` in quotations, which HOL
   resolves to real_of_num, not int_of_num.  This file never quotes the
   arithmetic: it extracts the flag equation's own num subterms from the
   residue and rebuilds the equation with int_of_num/int_add explicitly.

   V0  residue as produced                                    (expect SAT)
   V2  residue + the flag equation in wordifiable int form     (expect UNSAT if the
                                                                diagnosis is the flag)
   V4  flag equation + residue ==> B + p = A_total            (expect UNSAT)

   DEVELOPMENT_ONLY machine code / goal / prefix, as probe_add.ml. *)

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

let rec FALSE_SPLIT_CONDITIONS_TAC (asl,w) =
  let conds = bridge_conditions w @
              List.concat (map (fun (_,th) -> bridge_conditions(concl th)) asl) in
  match conds with
  | [] -> ALL_TAC (asl,w)
  | c::_ ->
      let false_th = ASSUME (mk_neg c) in
      let rule = EQF_INTRO false_th in
      let rws = [rule; COND_CLAUSES; BITVAL_CLAUSES] in
      let asl' = ("FALSE_SPLIT", false_th) ::
                 map (fun (lbl,th) -> (lbl, REWRITE_RULE rws th)) asl in
      let w' = rand (concl (REWRITE_CONV rws w)) in
      FALSE_SPLIT_CONDITIONS_TAC (asl', w');;

let RESIDUE = ref None;;

let CAPTURE_RESIDUE : tactic =
  fun (asl,w) -> RESIDUE := Some (itlist (fun a b -> mk_imp(a,b))
                    (map (fun (_,th) -> concl th) asl) w); ALL_TAC (asl,w);;

let TRUE_THEN_FALSE_CARRY_TAC expand pth : tactic =
  ELIM_DIGIT_EQS_TAC THEN BRIDGE_REWRITE_MODULUS pth THEN
  (fun (asl,w) ->
    let (lhs,_) = dest_eq w in
    let (sum,p) = dest_binop `(MOD):num->num->num` lhs in
    let cond = mk_comb(mk_comb(`(<):num->num->bool`, sum), p) in
    (BRIDGE_ZERO_LT_TAC p THEN
     ASM_CASES_TAC cond THENL
      [ASM_SIMP_TAC[ADD_MOD_LT] THEN
       FALSE_SPLIT_CONDITIONS_TAC THEN expand THEN
       RULE_ASSUM_TAC (CONV_RULE BRIDGE_TO_WORD_CONV) THEN
       CONV_TAC BRIDGE_TO_WORD_CONV THEN CAPTURE_RESIDUE;
       ALL_TAC]) (asl,w));;

let try_csat tag tm =
  let t0 = Unix.gettimeofday () in
  let closed,msg =
    try ignore(prove(tm, CSAT_TAC)); (true,"")
    with Failure s -> (false, s) | e -> (false, Printexc.to_string e) in
  Printf.printf "VARIANT %s closed=%b seconds=%.3f reason=%s\n%!"
    tag closed (Unix.gettimeofday () -. t0)
    (String.concat " " (String.split_on_char '\n' msg));;

set_goal([], add_goal);;
e add_prefix;; e red_add;;
let t0 = Unix.gettimeofday ();;
let _ = try ignore(e (TRUE_THEN_FALSE_CARRY_TAC EXPAND_BIGNUM p_256))
  with e -> Printf.printf "CAPTURE_EXN %s\n%!" (Printexc.to_string e);;
Printf.printf "CAPTURE seconds=%.3f\n%!" (Unix.gettimeofday () -. t0);;

let imp = match !RESIDUE with Some t -> t | None -> failwith "no residue";;
Printf.printf "=== RESIDUE ===\n%s\n=== END ===\n%!" (string_of_term imp);;

let rec antecedents tm =
  if is_imp tm then lhand tm :: antecedents (rand tm) else []
and conclusion_of tm = if is_imp tm then conclusion_of (rand tm) else tm;;
let hyps = antecedents imp and ccl = conclusion_of imp;;

(* find the flag equation (the only antecedent containing int_mul) *)
let has_mul h =
  find_terms (fun t -> match t with
    | Comb(Const("int_mul",_),_) -> true | _ -> false) h <> [];;
let flag_hyp = List.find has_mul hyps;;
Printf.printf "FLAG_HYP %s\n%!" (string_of_term flag_hyp);;

let all t pred = find_terms pred t;;
let named_val name t =
  match t with
  | Comb(Const("bitval",_),v) | Comb(Const("val",_),v) ->
      (try fst(dest_var v) = name with _ -> false)
  | _ -> false;;

let bv name =
  List.find (named_val name) (all flag_hyp (fun t -> match t with
    | Comb(Const("bitval",_),_) -> true | _ -> false));;
let vs = List.find (named_val "sum_s16") (all flag_hyp (fun t -> match t with
    | Comb(Const("val",_),_) -> true | _ -> false));;

let ib x = mk_comb(`int_of_num:num->int`, x);;
let iadd a b = mk_comb(mk_comb(`int_add:int->int->int`, a), b);;

(* the flag equation, rearranged:  c8 = c15 + s16 *)
let flag_eq_int = mk_eq (ib (bv "carry_s8"),
                         iadd (ib (bv "carry_s15")) (ib vs));;
Printf.printf "FLAG_EQ_INT %s\n%!" (string_of_term flag_eq_int);;

(* V0: as produced *)
try_csat "V0_asis" imp;;

(* V2: add the correctly typed, wordifiable flag equation *)
try_csat "V2_flag_int" (mk_imp(flag_eq_int, imp));;

(* V4: with the flag equation, the equations must entail B + p = A_total *)
let pnum = rand(concl p_256);;
let pterm = ib pnum;;
let (a_total,b_total) = dest_eq ccl;;
let bplusp = iadd b_total pterm;;
let v4 = mk_imp(flag_eq_int,
   itlist (fun a b -> mk_imp(a,b)) hyps (mk_eq(bplusp, a_total)));;
try_csat "V4_entails_B_plus_p" v4;;

Printf.printf "PROBE_DONE\n%!";;
