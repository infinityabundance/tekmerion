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

(* --- backward-closure LRAT trim (IX.63) ---------------------------------- *)
(*
   Between the solver writing its LRAT and the kernel replaying it, keep only
   the additions the empty clause depends on through positive RUP hints, in
   the original file order, and drop every deletion line.  A kept clause may
   still be hinted by a kept clause, so the replay is the same kernel code on
   a smaller file.

   Conservative by construction.  If any hint in the file is negative (RAT),
   or the last addition does not assert the empty clause, or the file is not
   understood (blank/duplicate ids/malformed line/IO error), the file is left
   untouched and `CSAT_TRIM skipped=<reason>` is printed.  There is no
   fallback on replay failure: a wrong closure makes LRAT_REFUTE_GEN fail,
   which is the intended behaviour.

   This block is delimited by BEGIN/END CSAT_TRIM_STANDALONE so that
   tests/csat_trim_selftest.sh can exercise exactly the shipped text outside
   HOL Light.  Keep it free of HOL Light names and of structure items other
   than `let` (the emitted answer inlines each phrase as a nested binding). *)
(* BEGIN CSAT_TRIM_STANDALONE *)
let csat_lrat_tokens s =
  let n = String.length s and out = ref [] and buf = Buffer.create 24 in
  let flush () =
    if Buffer.length buf > 0 then
      (out := Buffer.contents buf :: !out; Buffer.clear buf) in
  for i = 0 to n - 1 do
    match s.[i] with
    | ' ' | '\t' | '\r' -> flush ()
    | c -> Buffer.add_char buf c
  done;
  flush ();
  List.rev !out;;

let csat_lrat_parse raw toks =
  let all_nonneg ts =
    List.for_all
      (fun t -> try int_of_string t >= 0 with _ -> false) ts in
  match toks with
  | id_s :: "d" :: rest ->
      (try
         if int_of_string id_s = 0 || not (all_nonneg rest) then None
         else Some (int_of_string id_s, None, raw)
       with _ -> None)
  | id_s :: rest ->
      (try
         let id = int_of_string id_s in
         if id = 0 then None else
         let rec first_zero acc = function
           | [] -> None
           | "0" :: tl -> Some (List.rev acc, tl)
           | t :: tl -> first_zero (t :: acc) tl in
         match first_zero [] rest with
         | None -> None
         | Some (lits, tail) ->
             List.iter (fun t -> ignore (int_of_string t)) lits;
             let hs = List.fold_left
               (fun acc t ->
                  let v = int_of_string t in
                  if v = 0 then acc else v :: acc)
               [] tail in
             Some (id, Some (Array.of_list (List.rev hs)), raw)
       with _ -> None)
  | [] -> None;;

(* (id, hints option, raw) for every non-blank line, plus the file size. *)
let csat_lrat_read path =
  try
    let ic = open_in_bin path in
    let n = in_channel_length ic in
    let text = really_input_string ic n in
    close_in ic;
    let parsed = ref [] and start = ref 0 and bad = ref None in
    let add_line stop =
      let raw = String.sub text !start (stop - !start) in
      let toks = csat_lrat_tokens raw in
      if toks <> [] then
        (match csat_lrat_parse raw toks with
         | None -> bad := Some "malformed"
         | Some p -> parsed := p :: !parsed) in
    for i = 0 to n - 1 do
      if text.[i] = '\n' then (add_line i; start := i + 1)
    done;
    if !start < n then add_line n;
    match !bad with
    | Some r -> Error r
    | None -> Ok (Array.of_list (List.rev !parsed), n)
  with _ -> Error "io_error";;

let csat_trim_lrat path =
  let skipped reason =
    Printf.printf "CSAT_TRIM skipped=%s\n%!" reason in
  match csat_lrat_read path with
  | Error reason -> skipped reason
  | Ok (parsed, before_bytes) ->
      let m = Array.length parsed in
      if m = 0 then skipped "empty_lrat" else
      let ids = Array.init m (fun i -> let (x,_,_) = parsed.(i) in x)
      and hints = Array.init m (fun i -> let (_,x,_) = parsed.(i) in x)
      and raws = Array.init m (fun i -> let (_,_,x) = parsed.(i) in x) in
      let additions = ref 0 and last = ref (-1) in
      for i = 0 to m - 1 do
        if hints.(i) <> None then (incr additions; last := i)
      done;
      if !additions = 0 then skipped "no_additions" else
      let root = !last in
      let root_lits =
        match csat_lrat_tokens raws.(root) with
        | _ :: rest ->
            let rec count = function
              | "0" :: _ -> 0
              | _ :: tl -> 1 + count tl
              | [] -> 0 in
            count rest
        | [] -> 0 in
      if root_lits <> 0 then skipped "no_empty_clause" else
      let has_negative =
        let found = ref false in
        Array.iter
          (function
            | None -> ()
            | Some hs -> Array.iter (fun v -> if v < 0 then found := true) hs)
          hints;
        !found in
      if has_negative then skipped "negative_hint" else
      let index = Hashtbl.create (2 * !additions + 1) in
      let dup = ref false in
      for i = 0 to m - 1 do
        if hints.(i) <> None then
          (if Hashtbl.mem index ids.(i) then dup := true;
           Hashtbl.replace index ids.(i) i)
      done;
      if !dup then skipped "duplicate_id" else
      let keep = Array.make m false in
      let steps_after = ref 0 in
      let stack = ref [root] in
      while !stack <> [] do
        match !stack with
        | [] -> ()
        | pos :: rest ->
            stack := rest;
            if not keep.(pos) then
              (keep.(pos) <- true;
               incr steps_after;
               match hints.(pos) with
               | None -> ()
               | Some hs ->
                   Array.iter
                     (fun h ->
                        if h > 0 then
                          try
                            let j = Hashtbl.find index h in
                            if not keep.(j) then stack := j :: !stack
                          with Not_found -> ())
                     hs)
      done;
      let buf = Buffer.create (before_bytes / 2 + 1) in
      for i = 0 to m - 1 do
        if keep.(i) then
          (Buffer.add_string buf raws.(i); Buffer.add_char buf '\n')
      done;
      let out = Buffer.contents buf in
      (try
         let tmp = path ^ ".trim" in
         let oc = open_out_bin tmp in
         output_string oc out;
         close_out oc;
         Sys.rename tmp path;
         Printf.printf
           "CSAT_TRIM before=%d after=%d steps_before=%d steps_after=%d\n%!"
           before_bytes (String.length out) !additions !steps_after
       with _ ->
         (try Sys.remove (path ^ ".trim") with _ -> ());
         skipped "write_error");;
(* END CSAT_TRIM_STANDALONE *)

(* Disabled only by a controlled comparison run (T4), never by the emitter. *)
let csat_trim_enabled = ref true;;

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
  let _ = if !csat_trim_enabled then csat_trim_lrat lratname in
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
