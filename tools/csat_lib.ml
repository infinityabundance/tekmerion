(* ========================================================================= *)
(* csat_lib.ml -- certified-SAT close for HOL Light word goals.               *)
(*                                                                           *)
(* Connects the missing BITBLAST_THEN -> CADICAL_PROVE edge.  The pinned     *)
(* Examples/bitblast.ml defines BITBLAST_CADICAL only inside a (**** ...     *)
(* ****) block, so it is never loaded.  This file supplies it.               *)
(*                                                                           *)
(* Kernel trust: every SAT refutation is replayed by LRAT_REFUTE_GEN inside  *)
(* the HOL Light kernel (Cadical/ldrup.ml).  No solver certificate is trusted *)
(* and no compiler enters the trusted base.                                  *)
(*                                                                           *)
(* CaDiCaL 2.1.2 emits LRAT text directly with --no-binary --lrat; the       *)
(* lrat-trim post-pass used by Cadical/cadical.ml is therefore optional.     *)
(* CSAT_PROVE is CADICAL_PROVE with that native-LRAT refuter substituted.    *)
(* ========================================================================= *)

(* Unix (wall-clock) is loaded by the driver before this file is loadt-ed. *)
let csat_now () = Unix.gettimeofday ();;
let csat_debug = ref false;;

(* --- native-LRAT refuter ------------------------------------------------- *)

let CSAT_REFUTE_GEN amap tm =
  let cnfname = Filename.temp_file "csathol" ".cnf"
  and lratname = Filename.temp_file "csathol" ".lrat"
  and logname = Filename.temp_file "csathol" ".log" in
  let _ = dimacs_of_hol cnfname tm in
  let command =
    "(cadical --no-binary --lrat "^cnfname^" "^lratname^") >"^logname in
  let rc = Sys.command command in
  if rc <> 20 then
    (ignore (Sys.command ("rm -f "^cnfname^" "^lratname^" "^logname));
     failwith ("CSAT: cadical did not report UNSAT, exit "^string_of_int rc));
  let res = LRAT_REFUTE_GEN amap cnfname lratname in
  let _ = if !csat_debug then ()
          else ignore (Sys.command
                 ("rm -f "^cnfname^" "^lratname^" "^logname)) in
  res;;

let CSAT_REFUTE =
  let false_tm = `F` in
  fun tm ->
    let aar = Array.of_list (false_tm::atoms tm) in
    let th = CSAT_REFUTE_GEN (Array.get aar) tm in
    if hyp th = [tm] && concl th = false_tm then th
    else failwith "CSAT_REFUTE: Sanity check failure";;

(* --- definitional-CNF prover (mirrors CADICAL_PROVE) --------------------- *)

let CSAT_PROVE =
  let false_tm = `F`
  and p_tm = `p:bool` and q_tm = `q:bool`
  and pth_triv = TAUT `(~p <=> F) <=> p`
  and pth_main = UNDISCH_ALL(TAUT `(~p <=> q) ==> (q ==> F) ==> p`) in
  let presimp_conv = GEN_REWRITE_CONV DEPTH_CONV
    [NOT_CLAUSES; AND_CLAUSES; OR_CLAUSES; IMP_CLAUSES; EQ_CLAUSES]
  and triv_rule p th = EQ_MP(INST [p,p_tm] pth_triv) th
  and main_rule p q sth th =
    itlist PROVE_HYP [sth; DISCH_ALL th] (INST [p,p_tm; q,q_tm] pth_main) in
  fun tm ->
    if is_neg tm && is_strictcnf (rand tm) then
      NOT_INTRO(DISCH_ALL(CSAT_REFUTE(rand tm)))
    else if is_imp tm && rand tm = false_tm && is_strictcnf (lhand tm) then
      DISCH_ALL(CSAT_REFUTE(lhand tm))
    else
      let sth = presimp_conv(mk_neg tm) in
      let stm = rand(concl sth) in
      if stm = false_tm then triv_rule tm sth else
      let th = ASSUME stm in
      let ths = if !exploit_conjunctive_structure then GCONJUNCTS th
                else [th] in
      let n,tops,defs,lfn =
        itlist definitionalize ths (-1,[],undefined,undefined) in
      let defg = foldl (fun a t nv -> (t,nv)::a) [] defs in
      let mdefs = filter (fun (r,_) -> not (is_atom r)) defg in
      let eqs = map (fun (r,l) -> mk_iff(l,r)) mdefs in
      let clausify eq cls =
        let fvs = frees eq and eth = (NNFC_CONV THENC CNF_CONV) eq in
        let tth = INST (map (fun v -> apply lfn v,v) fvs) eth in
        let xth = ADD_ASSUM stm (EQ_MP tth (REFL(apply lfn (lhand eq)))) in
        zip (conjuncts(rand(concl eth))) (CONJUNCTS xth) @ cls in
      let all_clauses = itlist clausify eqs tops in
      let ntdcnf = list_mk_conj (map fst all_clauses) in
      let expat = map (apply lfn) (atoms ntdcnf) in
      let rar = Array.of_list(false_tm::expat) in
      let cth = CSAT_REFUTE_GEN (Array.get rar) ntdcnf in
      let dth = PROVE_HYP (end_itlist CONJ (map snd all_clauses)) cth in
      let fth = main_rule tm stm sth dth in
      if concl fth = tm && hyp fth = [] then fth
      else failwith "CSAT_PROVE: Sanity check";;

let CSAT_TAC = BITBLAST_THEN (CONV_TAC o K CSAT_PROVE);;

(* --- attempt a goal and report ------------------------------------------- *)

(* A goal (asl,w) becomes the theorem |- asl ==> w, closed with the
   certified-SAT tactic.  Returns (closed, seconds, message, theorem-term). *)
let csat_list_mk_imp (l,t) = itlist (fun a b -> mk_imp(a,b)) l t;;

let csat_attempt_goal (asl,w) =
  let tm = if asl = [] then w else
    csat_list_mk_imp (map (fun (_,th) -> concl th) asl, w) in
  let t0 = csat_now () in
  let closed,msg =
    try
      let th = prove(tm, REPEAT STRIP_TAC THEN CSAT_TAC) in
      (hyp th = [] && concl th = tm, "")
    with Failure s -> (false, s)
       | _ -> (false, "unknown exception") in
  (closed, csat_now () -. t0, msg, tm);;

let report_attempt tag (closed,dt,msg,_) =
  if closed then
    Printf.printf "CSAT_RESULT %s closed=1 seconds=%.3f\n" tag dt
  else
    let s = String.concat " " (String.split_on_char '\n' msg) in
    Printf.printf "CSAT_RESULT %s closed=0 seconds=%.3f reason=%s\n" tag dt s;;
