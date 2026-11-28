(* ========================================================================= *)
(* tests/ix89/ab_lib.ml -- XXXI.B2 A/B hypothesis-identity harness.         *)
(*                                                                           *)
(* Concatenated by tests/ix89/framework_check.sh into a proof that runs the  *)
(* same task under the shared s2n-bignum simulation backend and the          *)
(* Tekmerion-owned one, in one HOL session, with the support library loaded  *)
(* once (re-loading the inlined library would redefine constants).          *)
(*                                                                           *)
(* Spies record the goal conclusion and every assumption conclusion at each  *)
(* simulation step and at the bridge entry.  `ab_check` compares the two     *)
(* recordings with `aconv` -- goals, every assumption, the frame predicates   *)
(* (counted, and compared as part of every assumption) and the bridge         *)
(* hypotheses -- and fails the comparison on any difference.                 *)
(*                                                                           *)
(* Recording calls no inference rule (only `concl`, list operations and      *)
(* `string_of_term` for the frame tally), so it cannot change                *)
(* `inference_count`; the per-backend totals are the ordinary ones.          *)
(*                                                                           *)
(* Not part of the emitted answer and not part of the upstream framework.    *)
(* ========================================================================= *)

(* A recorded state: (tag, goal conclusion, assumption conclusions in order). *)
let ab_states : (string * term * term list) list ref = ref [];;

let ab_record tag (asl, w) =
  ab_states :=
    (tag, w, List.map (fun (_, th) -> concl th) asl) :: !ab_states;;

(* Record the state the tactic is about to transform. *)
let ab_spy tag (tac : tactic) : tactic =
  fun g -> (ab_record tag g; tac g);;

(* Record the entry state and every goal the step leaves behind. *)
let ab_step tag (tac : tactic) : tactic =
  fun g ->
    ab_record tag g;
    let gs = tac g in
    (match gs with
     | (_, gl, _) ->
         List.iteri
           (fun i gg -> ab_record (tag ^ ":after" ^ string_of_int i) gg) gl);
    gs;;

(* Reproduce `*_ACCSTEPS_TAC` step by step, spying each step. *)
let ab_steps tag (step : int -> tactic) (snums : int list) : tactic =
  MAP_EVERY (fun n -> ab_step (tag ^ ":s" ^ string_of_int n) (step n)) snums;;

(* A frame predicate is an assumption whose term mentions a frame connective.
   Only used to report how many were compared: identity is decided on every
   assumption regardless. *)
let ab_is_frame tm =
  let s = string_of_term tm in
  let contains needle =
    let nl = String.length needle and m = String.length s in
    let rec go i = i + nl <= m && (String.sub s i nl = needle || go (i + 1)) in
    go 0 in
  contains "MAYCHANGE" || contains "ASSIGNS";;

let ab_find tag =
  match List.find_opt (fun (t, _, _) -> t = tag) !ab_states with
  | Some (_, w, asms) -> Some (w, asms)
  | None -> None;;

let ab_prefix p s =
  String.length s >= String.length p && String.sub s 0 (String.length p) = p;;

(* Compare every upstream (`up:`) recording with the Tekmerion (`tk:`) one of
   the same tag.  One greppable mismatch line per difference, then a verdict. *)
let ab_check task =
  let tags = List.rev (List.map (fun (t, _, _) -> t) !ab_states) in
  let up_tags = List.filter (ab_prefix "up:") tags
  and tk_tags = List.filter (ab_prefix "tk:") tags in
  let fail = ref false in
  let mism reason tag =
    (fail := true;
     Printf.printf "AB_MISMATCH task=%s element=%s tag=%s\n%!" task reason tag) in
  if List.length up_tags <> List.length tk_tags then begin
    fail := true;
    Printf.printf "AB_MISMATCH task=%s element=state-count up=%d tk=%d\n%!"
      task (List.length up_tags) (List.length tk_tags)
  end;
  let nstates = ref 0 and nassums = ref 0 and nframes = ref 0 in
  let bridge_hyps = ref 0 in
  List.iter
    (fun utag ->
       let ttag = "tk:" ^ String.sub utag 3 (String.length utag - 3) in
       match ab_find utag, ab_find ttag with
       | Some (uw, uasms), Some (tw, tasms) ->
           incr nstates;
           if not (aconv uw tw) then mism "goal" utag;
           if List.length uasms <> List.length tasms then
             mism "assumption-count" utag
           else
             List.iter2
               (fun a b -> if not (aconv a b) then mism "assumption" utag)
               uasms tasms;
           nassums := !nassums + List.length uasms;
           nframes := !nframes + List.length (List.filter ab_is_frame uasms);
           if utag = "up:bridge" then bridge_hyps := List.length uasms
       | _ -> mism "state-missing" utag)
    up_tags;
  Printf.printf
    "AB_COMPARE task=%s states=%d assumptions=%d frames=%d bridge_hyps=%d\n%!"
    task !nstates !nassums !nframes !bridge_hyps;
  if !fail then
    Printf.printf "AB_RESULT task=%s identical=0\n%!" task
  else
    Printf.printf
      "AB_IDENTICAL task=%s states=%d assumptions=%d frames=%d bridge_hyps=%d\n%!"
      task !nstates !nassums !nframes !bridge_hyps;
  not !fail;;

(* Statement/hypothesis/axiom check for one backend's theorem, reusing the
   validated cache produced by stmt_check.sh.  The canonical `STATEMENT_OK`
   line accepted by `stmt_check.sh verdict` is printed for the Tekmerion
   theorem only; the upstream theorem gets a distinct backend-tagged line so
   the verdict's "exactly one STATEMENT_OK" rule still holds. *)
let ab_statement_check label task theorem source cache (th : thm) =
  let cache_errors =
    try ix89_validate_cache ~task ~theorem ~source cache
    with _ -> ["cache-read-error"] in
  let entries =
    if cache_errors = [] then (try ix89_read_cache cache with _ -> []) else [] in
  let stmt =
    match List.find_opt (fun (k, _) -> k = "STMT") entries with
    | None -> None
    | Some (_, h) -> (try Some (parse_term (ix89_string_of_hex h)) with _ -> None) in
  let hyps = hyp th in
  let actual_axes = List.map (fun t -> string_of_term (concl t)) (axioms ()) in
  let expected_axes =
    List.map (fun t -> string_of_term (concl t)) [INFINITY_AX; SELECT_AX; ETA_AX] in
  let sorted l = List.sort compare l in
  let ax_ok = sorted actual_axes = sorted expected_axes in
  let aconv_ok =
    match stmt with Some s -> aconv (concl th) s | None -> false in
  let nhyps = List.length hyps in
  if cache_errors = [] && aconv_ok && nhyps = 0 && ax_ok then begin
    if label = "tekmerion" then
      Printf.printf "STATEMENT_OK task=%s aconv=1 hyps=0 axioms=%d\n%!"
        task (List.length actual_axes)
    else
      Printf.printf "STATEMENT_OK_UP backend=%s task=%s aconv=1 hyps=0 axioms=%d\n%!"
        label task (List.length actual_axes)
  end else
    Printf.printf
      "STATEMENT_FAIL backend=%s task=%s aconv=%d hyps=%d axioms=%d axioms_ok=%d cache_errors=%s\n%!"
      label task (if aconv_ok then 1 else 0) nhyps
      (List.length actual_axes) (if ax_ok then 1 else 0)
      (String.concat "," cache_errors);;
