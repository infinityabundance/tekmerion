(* V8.4 -- confirm or withdraw the signedness diagnosis.

   The previous debug harness (debug_add_bridge.ml) printed "CSAT_2" as the
   carry_s16=false branch, but its residue still carried the *arithmetic*
   hypothesis `(m+n) < p` and the un-subtracted conclusion `m+n = <written>`.
   That is the false *carry* branch inside the true *mod* branch.  This file
   reproduces exactly that residue, measures what the encoding does with it,
   and runs three controlled variants:

     V0  the residue as-is                              (expect SAT)
     V1  the same residue with p forced to a 258-bit
         unsigned word (the "width 258" experiment)
     V2  the same residue plus the flag equation in a
         wordifiable, explicitly unsigned form
         `bitval c8 = bitval c15 + val sum_s16`
     V3  V1 + V2

   DEVELOPMENT_ONLY: the machine code, goal, simulation prefix and p_256
   numeral are the same transcribed material as probe_add.ml.  Nothing here
   changes tools/bridge.ml. *)

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

(* ------------------------------------------------------------------------- *)
(* False-carry-only split, as in v8_4_confirm.ml.                            *)
(* ------------------------------------------------------------------------- *)

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

(* ------------------------------------------------------------------------- *)
(* The width-258 rewrite: `&p` as the value of a 258-bit unsigned word.      *)
(* (Hard-coded numeral is this driver's p, not a bridge lemma constant.)     *)
(* ------------------------------------------------------------------------- *)

let UNSIGN_P_TH = prove
 (`&115792089210356248762697446949407573530086143415290314195533631308867097853951 =
   &(val(word 115792089210356248762697446949407573530086143415290314195533631308867097853951
             : 258 word))`,
  REWRITE_TAC[VAL_WORD] THEN
  SUBGOAL_THEN
   `115792089210356248762697446949407573530086143415290314195533631308867097853951
    MOD 2 EXP 258 =
    115792089210356248762697446949407573530086143415290314195533631308867097853951`
   (fun th -> REWRITE_TAC[th]) THENL
   [MATCH_MP_TAC MOD_LT THEN CONV_TAC NUM_REDUCE_CONV;
    REFL_TAC]);;

(* the flag equation in a form IWORDIFY can encode *)
let FLAG_EQ = `&(bitval (carry_s8:bool)) =
               &(bitval (carry_s15:bool)) + &(val (sum_s16:int64))`;;

(* ------------------------------------------------------------------------- *)
(* Report helpers.                                                           *)
(* ------------------------------------------------------------------------- *)

let dedup_terms l =
  let rec go acc = function
    | [] -> List.rev acc
    | h::t -> if List.exists (fun x -> aconv x h) acc then go acc t
              else go (h::acc) t in
  go [] l;;

let is_cmp tm =
  type_of tm = `:bool` &&
  (match tm with
   | Comb(Comb(Const(c,_),_),_) ->
       (c = "int_lt" || c = "int_le" || c = "int_gt" || c = "int_ge" ||
        c = "=" ) && type_of (rand tm) = `:int`
   | _ -> false);;

let show_atom_widths tag tm =
  List.iter (fun a ->
    let ok,txt =
      try
        let th = IWORDIFY_ATOM_CONV a in
        true, string_of_term (concl th)
      with _ -> false, "<IWORDIFY failed>" in
    if String.length (string_of_term a) < 400 then
      let out = if String.length txt > 4000 then "<long>" else txt in
      Printf.printf "%s ATOM ok=%b\n  IN : %s\n  OUT: %s\n%!"
        tag ok (string_of_term a) out)
    (dedup_terms (find_terms is_cmp tm));;

(* does the whole implication wordify?  (BITBLAST leaves failures opaque) *)
let whole_atom_count tm =
  let ats = dedup_terms (find_terms is_cmp tm) in
  let bad = List.filter (fun a ->
      try ignore(IWORDIFY_ATOM_CONV a); false with _ -> true) ats in
  (List.length ats, List.length bad);;

let try_csat tag tm =
  let t0 = Unix.gettimeofday () in
  let closed,msg =
    try ignore(prove(tm, CSAT_TAC)); (true,"")
    with Failure s -> (false, s) | e -> (false, Printexc.to_string e) in
  Printf.printf "VARIANT %s closed=%b seconds=%.3f reason=%s\n%!"
    tag closed (Unix.gettimeofday () -. t0)
    (String.concat " " (String.split_on_char '\n' msg));
  closed;;

(* ------------------------------------------------------------------------- *)
(* Drive the true arithmetic branch, then the false carry branch.            *)
(* ------------------------------------------------------------------------- *)

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

set_goal([], add_goal);;
e add_prefix;; e red_add;;
let t0 = Unix.gettimeofday ();;
let _ = try ignore(e (TRUE_THEN_FALSE_CARRY_TAC EXPAND_BIGNUM p_256))
  with e -> Printf.printf "CAPTURE_EXN %s\n%!" (Printexc.to_string e);;
Printf.printf "CAPTURE seconds=%.3f\n%!" (Unix.gettimeofday () -. t0);;

let imp = match !RESIDUE with Some t -> t | None -> failwith "no residue";;

Printf.printf "=== RESIDUE ===\n%s\n=== END ===\n%!" (string_of_term imp);;
let na,nbad = whole_atom_count imp in
Printf.printf "ATOMS total=%d not_wordifiable=%d\n%!" na nbad;;
show_atom_widths "W" imp;;

(* widths of the val/join operands: this is the signedness measurement *)
let vals = dedup_terms (find_terms (fun t -> match t with
  | Comb(Const("val",_),_) -> true | _ -> false) imp) in
List.iter (fun v ->
  Printf.printf "VALWIDTH %s : %s\n%!"
    (string_of_term (rand v)) (string_of_type(type_of(rand v)))) vals;;

(* split the residue into antecedent list and conclusion *)
let rec antecedents tm =
  if is_imp tm then lhand tm :: antecedents (rand tm) else []
and conclusion tm = if is_imp tm then conclusion (rand tm) else tm;;
let hyps = antecedents imp and ccl = conclusion imp;;
Printf.printf "HYPS=%d\n%!" (List.length hyps);;

(* V0: as produced *)
ignore(try_csat "V0_asis" imp);;

(* V1: p at width 258 (explicitly unsigned) *)
let imp_unsign = rand(concl(REWRITE_CONV[UNSIGN_P_TH] imp));;
ignore(try_csat "V1_p258" imp_unsign);;

(* V2: add the flag equation, wordifiable unsigned form *)
let imp_flag = mk_imp(FLAG_EQ, imp);;
ignore(try_csat "V2_flag_eq" imp_flag);;

(* V3: both *)
let imp_both = mk_imp(FLAG_EQ, imp_unsign);;
ignore(try_csat "V3_both" imp_both);;

(* V4: do the equations entail B + p = A_total?
   If the encoding is faithful this must be UNSAT by the algebra in the
   report: B = A - p + 2^256*c15 and A = m+n - 2^256*c8. *)
let pnum = rand(concl p_256);;
let pterm = mk_comb(`int_of_num:num->int`, pnum);;
let (a_total,b_total) = dest_eq ccl;;
let bplusp = mk_comb(mk_comb(`int_add:int->int->int`, b_total), pterm);;
let v4 = itlist (fun a b -> mk_imp(a,b)) hyps (mk_eq(bplusp, a_total)) in
ignore(try_csat "V4_entails_B_plus_p" v4);;

(* V5: do the equations entail A_total = B when c8 = c15 = 0?  This is the
   arithmetic core of the intended contradiction. *)
let v5 = itlist (fun a b -> mk_imp(a,b)) hyps
  (mk_eq(a_total, b_total)) in
Printf.printf "V5_identical_to_V0=%b\n%!" (aconv v5 imp);;

Printf.printf "PROBE_DONE\n%!";;
