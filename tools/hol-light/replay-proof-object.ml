(* ------------------------------------------------------------------------- *)
(* Tekmerion proof-object replay inside HOL Light (XXII.B).                   *)
(*                                                                            *)
(* Usage (see tools/hol-light/replay-object.sh):                              *)
(*   TEKMERION_OBJECT=<file.hlpo> ocaml-hol -init hol.ml -I <hol> < this.ml  *)
(*                                                                            *)
(* The object is parsed from its digest-bound bytes and rebuilt node by node  *)
(* with the primitive rules declared by fusion.ml.  The script has two modes: *)
(*   TEKMERION_PO_MODE=stats   -- parse and report the object's shape.        *)
(*   (default)                 -- replay, then compare the derived root with  *)
(*                                the recorded statement and report axioms(). *)
(* ------------------------------------------------------------------------- *)

#load "unix.cma";;

let po_fail fmt = Printf.ksprintf failwith fmt;;

let read_file p =
  let ic = open_in_bin p in
  let n = in_channel_length ic in
  let s = really_input_string ic n in
  close_in ic;
  s;;

let object_path = Sys.getenv "TEKMERION_OBJECT";;
let mode = try Sys.getenv "TEKMERION_PO_MODE" with Not_found -> "replay";;
let data = read_file object_path;;

(* ------------------------------------------------------------------------- *)
(* A bounds-checked big-endian cursor over the object bytes.                  *)
(* ------------------------------------------------------------------------- *)

type cur = { s : string; mutable p : int; stop : int };;

let cur_make s off len = { s; p = off; stop = off + len };;

let cur_u8 c =
  if c.p >= c.stop then po_fail "replay: truncated at %d" c.p;
  let v = Char.code c.s.[c.p] in
  c.p <- c.p + 1;
  v;;

let cur_u32 c =
  let a = cur_u8 c in
  let b = cur_u8 c in
  let d = cur_u8 c in
  let e = cur_u8 c in
  (a lsl 24) lor (b lsl 16) lor (d lsl 8) lor e;;

let cur_u64 c =
  let hi = cur_u32 c in
  let lo = cur_u32 c in
  (hi lsl 32) lor lo;;

let cur_str c =
  let n = cur_u32 c in
  if c.p + n > c.stop then po_fail "replay: string past end";
  let v = String.sub c.s c.p n in
  c.p <- c.p + n;
  v;;

let cur_bytes c n =
  if c.p + n > c.stop then po_fail "replay: bytes past end";
  let v = String.sub c.s c.p n in
  c.p <- c.p + n;
  v;;

let magic_obj = "HLPROOF1\n";;
let magic_end = "HLEND1\n";;

let total = String.length data;;
let trailer = 32 + String.length magic_end;;
if total < String.length magic_obj + 1 + trailer then po_fail "replay: object too short";;
if String.sub data 0 (String.length magic_obj) <> magic_obj then
  po_fail "replay: bad magic";;
let body_end = total - trailer;;
if String.sub data (body_end + 32) (String.length magic_end) <> magic_end then
  po_fail "replay: bad trailer";;
let digest_body = String.sub data body_end 32;;

let hex_of s =
  let b = Buffer.create (2 * String.length s) in
  String.iter
    (fun ch -> Buffer.add_string b (Printf.sprintf "%02x" (Char.code ch)))
    s;
  Buffer.contents b;;

let h = cur_make data 0 body_end;;
ignore (cur_bytes h (String.length magic_obj));;
let version = cur_u8 h;;
if version <> 2 then po_fail "replay: object version %d" version;;
let name = cur_str h;;
let raw_nodes = cur_u32 h;;
let inference_total = cur_u64 h;;
let inference_begin = cur_u64 h;;
let root_id = cur_u32 h;;
let theory_digest = cur_bytes h 32;;
let statement_id = cur_bytes h 32;;
let stmt_term_id = cur_u32 h;;
let hyp_count = cur_u32 h;;
let hyp_ids = Array.init hyp_count (fun _ -> cur_u32 h);;
let axiom_digest_count = cur_u32 h;;
let axiom_digests = Array.init axiom_digest_count (fun _ -> cur_bytes h 32);;
let import_digest_count = cur_u32 h;;
let import_digests = Array.init import_digest_count (fun _ -> cur_bytes h 32);;
let registry_count = cur_u32 h;;
let registry =
  Array.init registry_count (fun _ ->
      let id = cur_u32 h in
      let dg = cur_str h in
      let txt = cur_str h in
      (id, dg, txt));;

(* ------------------------------------------------------------------------- *)
(* Type and term tables, kept as structure-free keys.  HOL values are built   *)
(* lazily during replay, because a definition node introduces a constant that *)
(* must not be pre-declared.                                                  *)
(* ------------------------------------------------------------------------- *)

type type_key = TVar of string | TApp of string * int array;;

let type_count = cur_u32 h;;
let type_table_start = h.p;;
let type_tag = Array.make (max type_count 1) 0;;
let type_name = Array.make (max type_count 1) "" ;;
let type_args = Array.make (max type_count 1) [||] ;;
for i = 0 to type_count - 1 do
  type_tag.(i) <- cur_u8 h;
  match type_tag.(i) with
  | 0 -> type_name.(i) <- cur_str h
  | 1 ->
      type_name.(i) <- cur_str h;
      let n = cur_u32 h in
      type_args.(i) <- Array.init n (fun _ -> cur_u32 h)
  | t -> po_fail "replay: type tag %d at %d" t i
done;;
let after_types = h.p;;

let term_count = cur_u32 h;;
let term_tag = Array.make (max term_count 1) 0;;
let term_name = Array.make (max term_count 1) "" ;;
let term_ty = Array.make (max term_count 1) 0;;
let term_a = Array.make (max term_count 1) 0;;
let term_b = Array.make (max term_count 1) 0;;
for i = 0 to term_count - 1 do
  term_tag.(i) <- cur_u8 h;
  match term_tag.(i) with
  | 0 | 1 ->
      term_name.(i) <- cur_str h;
      term_ty.(i) <- cur_u32 h
  | 2 | 3 ->
      term_a.(i) <- cur_u32 h;
      term_b.(i) <- cur_u32 h
  | t -> po_fail "replay: term tag %d at %d" t i
done;;

if stmt_term_id >= term_count then po_fail "replay: statement id out of range";;
Array.iter
  (fun t -> if t >= term_count then po_fail "replay: hypothesis id out of range")
  hyp_ids;;

(* ------------------------------------------------------------------------- *)
(* Node table: metadata first so the frontier can be sized before replay.     *)
(* ------------------------------------------------------------------------- *)

let node_count = cur_u32 h;;
let rule = Array.make (max node_count 1) 255;;
let p0 = Array.make (max node_count 1) (-1);;
let p1 = Array.make (max node_count 1) (-1);;
let poff = Array.make (max node_count 1) 0;;
let plen = Array.make (max node_count 1) 0;;
let remaining = Array.make (max node_count 1) 0;;

for i = 0 to node_count - 1 do
  let r = cur_u8 h in
  let a = cur_u32 h in
  let b = cur_u32 h in
  let n = cur_u32 h in
  rule.(i) <- r;
  p0.(i) <- (if a = 0xffff_ffff then -1 else a);
  p1.(i) <- (if b = 0xffff_ffff then -1 else b);
  poff.(i) <- h.p;
  plen.(i) <- n;
  if p0.(i) >= i || p1.(i) >= i then po_fail "replay: node %d parent out of range" i;
  if p0.(i) >= 0 then remaining.(p0.(i)) <- remaining.(p0.(i)) + 1;
  if p1.(i) >= 0 then remaining.(p1.(i)) <- remaining.(p1.(i)) + 1;
  if h.p + n > h.stop then po_fail "replay: payload past body";
  h.p <- h.p + n
done;;
if h.p <> body_end then po_fail "replay: node table does not end at body";;

let rule_name = function
  | 0 -> "REFL" | 1 -> "TRANS" | 2 -> "MK_COMB" | 3 -> "ABS"
  | 4 -> "BETA" | 5 -> "ASSUME" | 6 -> "EQ_MP" | 7 -> "DEDUCT_ANTISYM"
  | 8 -> "INST_TYPE" | 9 -> "INST" | 10 -> "AXIOM" | 11 -> "DEFINITION"
  | 12 -> "TYPEDEF_ABS" | 13 -> "TYPEDEF_REP" | 14 -> "IMPORT"
  | _ -> "UNKNOWN";;

let histogram = Array.make 15 0;;
for i = 0 to node_count - 1 do
  if rule.(i) < 15 then histogram.(rule.(i)) <- histogram.(rule.(i)) + 1
done;;

let import_shape = Array.make 15 0;;
let rec ty_sexp id =
  if type_tag.(id) = 0 then "'" ^ type_name.(id)
  else
    let args = type_args.(id) in
    if Array.length args = 0 then type_name.(id)
    else
      type_name.(id) ^ "("
      ^ String.concat "," (Array.to_list (Array.map ty_sexp args))
      ^ ")";;
let rec tm_sexp depth id =
  if depth <= 0 then "…"
  else
    match term_tag.(id) with
    | 0 -> "v:" ^ term_name.(id)
    | 1 -> "c:" ^ term_name.(id)
    | 2 -> "(" ^ tm_sexp (depth - 1) term_a.(id) ^ " " ^ tm_sexp (depth - 1) term_b.(id) ^ ")"
    | 3 -> "[\\" ^ tm_sexp (depth - 1) term_a.(id) ^ ". " ^ tm_sexp (depth - 1) term_b.(id) ^ "]"
    | _ -> "?";;
if mode = "terms" then begin
  let lo = try int_of_string (Sys.getenv "TEKMERION_PO_LO") with _ -> 0 in
  let hi = try int_of_string (Sys.getenv "TEKMERION_PO_HI") with _ -> 15 in
  for i = lo to min hi (term_count - 1) do
    Printf.printf "TERM %d tag=%d name=%s ty=%d(%s) a=%d b=%d sexp=%s\n" i
      term_tag.(i) term_name.(i) term_ty.(i) (ty_sexp term_ty.(i)) term_a.(i)
      term_b.(i) (if term_tag.(i) < 2 then term_name.(i) else tm_sexp 5 i)
  done;
  exit 0
end;;
if mode = "types" then begin
  let lo = try int_of_string (Sys.getenv "TEKMERION_PO_LO") with _ -> 0 in
  let hi = try int_of_string (Sys.getenv "TEKMERION_PO_HI") with _ -> 15 in
  for i = lo to min hi (type_count - 1) do
    Printf.printf "TYPE %d tag=%d name=%s args=[%s] sexp=%s\n" i type_tag.(i)
      type_name.(i)
      (String.concat "," (Array.to_list (Array.map string_of_int type_args.(i))))
      (ty_sexp i)
  done;
  exit 0
end;;
if mode = "probe" then begin
  let s = get_const_type "=" in
  Printf.printf "PROBE eq=%s\n" (string_of_type s);
  let f, args = dest_type s in
  let a0 = List.hd args in
  Printf.printf "PROBE ctor=%s arity=%d first=%s isvar=%b name=%s\n" f
    (List.length args) (string_of_type a0) (is_vartype a0)
    (if is_vartype a0 then dest_vartype a0 else "-");
  let t = mk_const ("=", [(mk_vartype "A", bool_ty)]) in
  Printf.printf "PROBE inst_A=%s\n" (string_of_type (type_of t));
  let t = mk_const ("=", [(mk_vartype (dest_vartype a0), bool_ty)]) in
  Printf.printf "PROBE inst_named=%s\n" (string_of_type (type_of t));
  (match (try Some (get_const_type ":=") with Failure _ -> None) with
   | Some t -> Printf.printf "PROBE colon_eq=%s\n" (string_of_type t)
   | None -> Printf.printf "PROBE colon_eq=none\n");
  exit 0
end;;
if mode = "stats" then begin
  let with_hyps = ref 0 and max_hyps = ref 0 in
  for i = 0 to node_count - 1 do
    if rule.(i) = 14 then begin
      let c = cur_make data poff.(i) plen.(i) in
      let nh = cur_u32 c in
      if nh > 0 then incr with_hyps;
      if nh > !max_hyps then max_hyps := nh;
      let hyps = List.init nh (fun _ -> cur_u32 c) in
      let concl = cur_u32 c in
      if nh > 0 then
        Printf.printf "STATS import-hyps node=%d nh=%d concl=%s%s\n" i nh
          (tm_sexp 6 concl)
          (if List.exists (fun h -> h = concl) hyps then "  (concl is a hyp)" else "");
      ignore hyps;
      if c.p <> poff.(i) + plen.(i) then po_fail "replay: import payload length"
    end
  done;
  Printf.printf "STATS theorem=%s\n" name;
  Printf.printf "STATS raw_nodes=%d nodes=%d terms=%d types=%d root=%d\n" raw_nodes
    node_count term_count type_count root_id;
  Printf.printf "STATS inference_total=%d inference_begin=%d\n" inference_total
    inference_begin;
  Printf.printf "STATS statement_id=%s\n" (hex_of statement_id);
  Printf.printf "STATS theory_digest=%s\n" (hex_of theory_digest);
  Printf.printf "STATS derivation_digest=%s\n" (hex_of digest_body);
  Printf.printf "STATS body_bytes=%d\n" body_end;
  Printf.printf "STATS table_bytes type_start=%d after_types=%d after_terms=%d\n"
    type_table_start after_types h.p;
  for r = 0 to 14 do
    if histogram.(r) > 0 then
      Printf.printf "STATS rule %s = %d\n" (rule_name r) histogram.(r)
  done;
  Printf.printf "STATS axiom_digests=%d import_digests=%d registry=%d\n"
    axiom_digest_count import_digest_count registry_count;
  Array.iter
    (fun (id, dg, txt) ->
      let txt =
        if String.length txt > 90 then String.sub txt 0 90 ^ "..." else txt
      in
      Printf.printf "STATS registry %d %s %s\n" id dg (String.escaped txt))
    registry;
  Printf.printf "STATS import_nodes_with_hyps=%d max_import_hyps=%d\n" !with_hyps
    !max_hyps;
  exit 0
end;;

(* ------------------------------------------------------------------------- *)
(* Replay with the primitive rules only.                                      *)
(* ------------------------------------------------------------------------- *)

let axioms_before = axioms ();;
let ax_before = List.length axioms_before;;

(* Type and term values are materialised on first use.  A constant introduced
   by a definition node is therefore declared by the kernel at that node, not
   before it.  Constants and type operators the object needs but the session
   has never seen (the recording's prelude) are installed as uninterpreted
   symbols of the recorded type: the object names them but cannot define them. *)
let tys : hol_type option array = Array.make (max type_count 1) None;;
let tms : term option array = Array.make (max term_count 1) None;;
let consts_seen : (string, unit) Hashtbl.t = Hashtbl.create 1024;;
let types_seen : (string, unit) Hashtbl.t = Hashtbl.create 256;;

let rec ty_of_id i =
  match tys.(i) with
  | Some t -> t
  | None ->
      let t =
        if type_tag.(i) = 0 then mk_vartype type_name.(i)
        else begin
          Hashtbl.replace types_seen type_name.(i) ();
          proof_type (type_name.(i), Array.to_list (Array.map ty_of_id type_args.(i)))
        end
      in
      tys.(i) <- Some t;
      t;;

let rec tm_of_id i =
  match tms.(i) with
  | Some t -> t
  | None ->
      let t =
        match term_tag.(i) with
        | 0 -> mk_var (term_name.(i), ty_of_id term_ty.(i))
        | 1 ->
            Hashtbl.replace consts_seen term_name.(i) ();
            proof_const (term_name.(i), ty_of_id term_ty.(i))
        | 2 ->
            let f = tm_of_id term_a.(i) in
            let x = tm_of_id term_b.(i) in
            (try mk_comb (f, x) with Failure s ->
               po_fail "replay: term %d %s: %s (fn type %s / arg %s)" i
                 (tm_sexp 4 i) s (string_of_type (type_of f))
                 (string_of_type (type_of x)))
        | 3 ->
            let v = tm_of_id term_a.(i) in
            let b = tm_of_id term_b.(i) in
            (try mk_abs (v, b) with Failure s ->
               po_fail "replay: term %d %s: %s" i (tm_sexp 4 i) s)
        | t -> po_fail "replay: term tag %d at %d" t i
      in
      tms.(i) <- Some t;
      t;;

let payload_of i = cur_make data poff.(i) plen.(i);;
let term_of c = tm_of_id (cur_u32 c);;
let type_of_id c = ty_of_id (cur_u32 c);;

let frontier : (int, thm) Hashtbl.t = Hashtbl.create 4096;;
let peak_live = ref 0;;
let root_thm = ref None;;
let imports_admitted = ref 0;;
let axioms_admitted = ref 0;;

let lookup_parent i pid =
  match Hashtbl.find_opt frontier pid with
  | Some th -> th
  | None -> po_fail "replay: node %d missing parent %d" i pid;;

(* An IMPORT leaf is a sequent of the recording environment, not an
   inference.  The replay kernel's single added primitive, `proof_import`,
   packages exactly that sequent at axiom mask 0: it is the same premise set
   the Rust checker reads from its `.hlst` store, and it does not touch
   `axioms()`.  Every inference node below still uses a stock primitive. *)
let imports_admitted = ref 0;;
let import_keys : (string, unit) Hashtbl.t = Hashtbl.create 1024;;

let admit_import i ids hyps concl =
  let key = String.concat "\001" (List.map (tm_sexp 120) ids) in
  if not (Hashtbl.mem import_keys key) then Hashtbl.add import_keys key ();
  let th = proof_import (hyps, concl) in
  let got_hyps, got_concl = dest_thm th in
  if List.length got_hyps <> List.length hyps
     || not (List.for_all2 aconv got_hyps hyps)
     || not (aconv got_concl concl)
  then po_fail "replay: node %d trusted input did not round-trip" i;
  incr imports_admitted;
  th;;

let axiom_for_statement t =
  let rec find = function
    | [] -> None
    | th :: rest ->
        let asl, c = dest_thm th in
        if asl = [] && aconv c t then Some th else find rest
  in
  find axioms_before;;

let start = Unix.gettimeofday ();;

let rec loop i =
  if i >= node_count then ()
  else begin
    let c = payload_of i in
    let th =
      match rule.(i) with
      | 0 -> REFL (term_of c)
      | 1 -> TRANS (lookup_parent i p0.(i)) (lookup_parent i p1.(i))
      | 2 -> MK_COMB (lookup_parent i p0.(i), lookup_parent i p1.(i))
      | 3 -> ABS (term_of c) (lookup_parent i p0.(i))
      | 4 -> BETA (term_of c)
      | 5 -> ASSUME (term_of c)
      | 6 -> EQ_MP (lookup_parent i p0.(i)) (lookup_parent i p1.(i))
      | 7 -> DEDUCT_ANTISYM_RULE (lookup_parent i p0.(i)) (lookup_parent i p1.(i))
      | 8 ->
          let n = cur_u32 c in
          let theta =
            List.init n (fun _ ->
                let a = type_of_id c in
                let b = type_of_id c in
                (a, b))
          in
          INST_TYPE theta (lookup_parent i p0.(i))
      | 9 ->
          let n = cur_u32 c in
          let theta =
            List.init n (fun _ ->
                let a = term_of c in
                let b = term_of c in
                (a, b))
          in
          INST theta (lookup_parent i p0.(i))
      | 10 ->
          let t = term_of c in
          (match axiom_for_statement t with
           | Some th -> th
           | None ->
               incr axioms_admitted;
               new_axiom t)
      | 11 -> new_basic_definition (term_of c)
      | 12 | 13 ->
          let tyname = cur_str c in
          let absname = cur_str c in
          let repname = cur_str c in
          let pair =
            new_basic_type_definition tyname (absname, repname)
              (lookup_parent i p0.(i))
          in
          if rule.(i) = 12 then fst pair else snd pair
      | 14 ->
          let nh = cur_u32 c in
          let hids = List.init nh (fun _ -> cur_u32 c) in
          let cid = cur_u32 c in
          let hyps = List.map tm_of_id hids in
          let concl = tm_of_id cid in
          incr imports_admitted;
          admit_import i (cid :: hids) hyps concl
      | r -> po_fail "replay: unknown rule %d at node %d" r i
    in
    if c.p <> poff.(i) + plen.(i) then
      po_fail "replay: node %d payload has %d trailing bytes" i
        (poff.(i) + plen.(i) - c.p);
    if i = root_id then root_thm := Some th;
    let release pid =
      if pid >= 0 then begin
        let left = remaining.(pid) in
        if left <= 0 then po_fail "replay: parent %d has no consumer" pid;
        remaining.(pid) <- left - 1;
        if left = 1 then Hashtbl.remove frontier pid
      end
    in
    release p0.(i);
    release p1.(i);
    (match !root_thm with
     | Some _ when i = root_id -> ()
     | _ ->
         Hashtbl.replace frontier i th;
         if Hashtbl.length frontier > !peak_live then
           peak_live := Hashtbl.length frontier)
  end;
  loop (i + 1);;

loop 0;;

let elapsed = Unix.gettimeofday () -. start;;

let root =
  match !root_thm with
  | Some th -> th
  | None -> po_fail "replay: root node was not validated";;

let root_hyps, root_concl = dest_thm root;;
let recorded_hyps = Array.to_list (Array.map tm_of_id hyp_ids);;
let recorded_concl = tm_of_id stmt_term_id;;

let hyp_set_eq a b =
  List.length a = List.length b
  && List.for_all (fun x -> List.exists (aconv x) b) a;;

let concl_ok = aconv root_concl recorded_concl;;
let hyps_ok = hyp_set_eq root_hyps recorded_hyps;;
let closed = root_hyps = [];;
(* equals_thm is `dest_thm th = dest_thm th'`; applied to a sequent, that is
   structural equality of hypothesis list and conclusion. *)
let seq_exact = dest_thm root = (recorded_hyps, recorded_concl);;

let axioms_after = axioms ();;
let ax_after = List.length axioms_after;;

let vm_hwm_kb =
  try
    let ic = open_in "/proc/self/status" in
    let rec find () =
      match input_line ic with
      | l ->
          if String.length l >= 6 && String.sub l 0 6 = "VmHWM:" then begin
            close_in ic;
            int_of_string (String.trim (String.sub l 6 (String.length l - 6)))
          end
          else find ()
      | exception End_of_file ->
          close_in ic;
          0
    in
    find ()
  with _ -> 0;;

Printf.printf
  "REPLAY theorem=%s verdict=%s nodes=%d terms=%d types=%d replay_s=%.3f peak_live=%d vmhwm_kb=%d\n"
  name
  (if concl_ok && hyps_ok && closed then "ACCEPT" else "REJECT")
  node_count term_count type_count elapsed !peak_live vm_hwm_kb;;
Printf.printf "REPLAY statement_id=%s derivation_digest=%s\n" (hex_of statement_id)
  (hex_of digest_body);;
Printf.printf "REPLAY root_closed=%b conclusion_matches=%b hyps_match=%b sequent_exact=%b\n"
  closed concl_ok hyps_ok seq_exact;;
Printf.printf
  "REPLAY axioms_before=%d axioms_after=%d declared_axiom_digests=%d import_digests=%d\n"
  ax_before ax_after axiom_digest_count import_digest_count;;
Printf.printf "REPLAY imports=%d distinct_imports=%d axioms_admitted=%d constants=%d type_ops=%d\n"
  !imports_admitted (Hashtbl.length import_keys) !axioms_admitted
  (Hashtbl.length consts_seen) (Hashtbl.length types_seen);;
flush stdout;;
