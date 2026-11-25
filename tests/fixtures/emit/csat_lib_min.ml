(* Synthetic stand-in for the IX.66 marker anchors in tools/csat_lib.ml.
 *
 * tests/tools_emit.rs applies the emitter's own (plain, marked) table to this
 * text and then its inverse, so the round trip
 * `deinstrument ∘ instrument = id` can be proved without the `.bench` HOL Light
 * sources (the Cadical library is not part of the instrumented set anyway).
 *
 * Every anchor the table names appears exactly once, in the same lexical form
 * as in the tracked library. This file is not compiled and not loaded; it is a
 * fixture for the text edits only.
 *)

(* CSAT_PROVE's handover from BITBLAST_THEN, and the end of the closure. *)
let rec CSAT_PROVE g =
  let tm = snd g in
  fun tm ->
    if is_neg tm && is_strictcnf (rand tm) then
      let ntd = rand tm in
      CSAT_REFUTE_GEN [||] ntd g
    else
      let rar = [| rand tm |] in
      let ntdcnf = rand tm in
      let cth = CSAT_REFUTE_GEN (Array.get rar) ntdcnf in
      PROVE_HYP cth (ACCEPT_TAC (concl cth)) g;;

(* The DIMACS size the solver is handed, and the dummy tactic tail. *)
let CSAT_REFUTE_GEN amap tm : tactic =
  fun g ->
  let cnfname = Filename.temp_file "csat" ".cnf" in
  let _ = dimacs_of_hol cnfname tm in
  let lratname = cnfname ^ ".lrat" in
  let res = LRAT_REFUTE_GEN amap cnfname lratname in
  res g;;

(* The anchor that opens with a bare `ALL_TAC g;;`. *)
let CSAT_TRIM_HOOK : tactic =
  fun g ->
    ALL_TAC g;;

let csat_trim_id = ref true;;
