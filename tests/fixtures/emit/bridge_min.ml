(* Synthetic stand-in for the IX.66 marker anchors in tools/bridge.ml.
 *
 * Same purpose as csat_lib_min.ml: the emitter's marker table must be applied
 * and inverted here without the `.bench` HOL Light sources, which the property
 * `deinstrument ∘ instrument = id` does not need. Every anchor the table names
 * appears exactly once, in the tracked library's lexical form.
 *)

let BRIDGE_ADD_REF_TAC expand pth : tactic =
  ELIM_DIGIT_EQS_TAC THEN BRIDGE_REWRITE_MODULUS pth THEN expand THEN
  BRIDGE_SPLIT_CONDITIONS_KEEP_TAC;;

let bridge_sub_ref_core tag finish expand pth : tactic =
  ELIM_DIGIT_EQS_TAC THEN BRIDGE_REWRITE_MODULUS pth THEN expand THEN
  BRIDGE_SPLIT_CONDITIONS_KEEP_TAC;;

let bridge_neg_ref_core tag finish expand pth : tactic =
  ELIM_DIGIT_EQS_TAC THEN BRIDGE_REWRITE_MODULUS pth THEN expand THEN
  BRIDGE_SPLIT_CONDITIONS_KEEP_TAC;;

let BRIDGE_NEG_REF_ZERO_TAC expand pth : tactic =
  ELIM_DIGIT_EQS_TAC THEN BRIDGE_REWRITE_MODULUS pth THEN expand THEN
  BRIDGE_SPLIT_CONDITIONS_KEEP_TAC;;

(* One SAT close: the goal handed to it, and the theorem coming back. *)
let BRIDGE_CSAT_TAC : tactic =
  fun (asl,w) ->
    let hths = map snd asl in
    let imp = list_mk_imp (hths, w) in
    let th = prove(imp, CSAT_TAC) in
    let th' = List.fold_left (fun acc hth -> MP acc hth) th hths in
    ACCEPT_TAC th' (asl,w);;

(* everything that precedes the SAT call, on the goal *)
let BRIDGE_PREPARE_TAC expand : tactic = ALL_TAC;;

(* IX.70: the opt-in algebraic entry point and its phase anchors. *)
let BRIDGE_ADD_REF_ALG_TAC expand pth : tactic =
  ELIM_DIGIT_EQS_TAC THEN BRIDGE_REWRITE_MODULUS pth THEN expand THEN
  ALL_TAC;;

let BRIDGE_ALG_NORM_TAC : tactic = ALL_TAC;;

let BRIDGE_ALGEBRA_TAC : tactic =
  let str t = "x" in
  let at stage (t:tactic) : tactic = fun g -> t g in
  fun g ->
    (at "normalise" BRIDGE_ALG_NORM_TAC THEN
     at "oracle" BRIDGE_ORACLE_TAC) g;;

let bridge_anchor_count = 6;;

(* IX.86: the oracle-close anchors added to the measurement table.  Synthetic,
 * like the rest of this file: only the exact anchor lines matter. *)
let bridge_aggregate_facts aggregates all_hyps =
  let is_int_eq th = false and conjunctive th = true in
  let side_hyps =
    List.filter
      (fun th -> conjunctive th &&
                 (not (is_int_eq th) || not (bridge_int_eq_has_carry th)))
      all_hyps in
  aggregates;;

let BRIDGE_ORACLE_TAC : tactic =
  fun (asl,w) ->
      let aggregates = bridge_aggregates eqs in
      let all_hyps = List.map (fun (_,th) -> bridge_flat_th th) (bridge_int_hyps asl) in
      let agg_facts = bridge_aggregate_facts aggregates all_hyps in
      bridge_slice_note "ORACLE close ants=%d size=%d"
        (List.length ants)
        (List.fold_left (fun a th -> a + bridge_term_size (concl th)) 0 ants);
      match prop_closed with
      | Some closed ->
          bridge_slice_note "ORACLE close prop-refutation ants=%d"
            (List.length ants);
          ACCEPT_TAC closed (asl,w)
      | None ->
      match range_closed with
      | Some closed ->
          bridge_slice_note "ORACLE close range-refutation ants=%d"
            (List.length ants);
          ACCEPT_TAC closed (asl,w)
      | None ->
      match slim_closed with
      | Some closed -> ACCEPT_TAC closed (asl,w)
      | None ->
      let th = prove(bridge_mk_imps ants w', INT_ARITH_TAC) in
      bridge_slice_note "ORACLE close ok";
      ACCEPT_TAC closed (asl,w);;
