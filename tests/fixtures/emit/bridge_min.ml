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

let BRIDGE_SUB_REF_TAC expand pth : tactic =
  ELIM_DIGIT_EQS_TAC THEN BRIDGE_REWRITE_MODULUS pth THEN expand THEN
  BRIDGE_SPLIT_CONDITIONS_KEEP_TAC;;

let BRIDGE_NEG_REF_TAC expand pth : tactic =
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
    PROVE_HYP th (ACCEPT_TAC w);;

let bridge_anchor_count = 6;;
