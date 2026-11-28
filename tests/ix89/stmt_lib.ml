(* ========================================================================= *)
(* tests/ix89/stmt_lib.ml -- XXVIII.A / XXIX.A statement-check helpers.      *)
(*                                                                           *)
(* Shared by the extraction driver (loads the upstream `_CORRECT` proof once *)
(* and caches its conclusion) and by the block injected into the generated   *)
(* proof (re-parses the cached conclusion and compares it with `aconv` in    *)
(* the same session that produces the theorem).  No inference rule is        *)
(* invoked here, so loading this file cannot change `inference_count`.       *)
(*                                                                           *)
(* XXIX.A1/A2: a cache is only usable if it has exactly one of each         *)
(* mandatory field, exactly one REPARSE marker whose value is 1, a STMT     *)
(* payload that is valid lower-case hex, a STMT_LEN that agrees with the    *)
(* decoded byte length, and the provenance fields written by the extractor. *)
(* Digest equality against the upstream tree is checked by stmt_check.sh    *)
(* immediately before injection; this file checks the structure so that a   *)
(* truncated or hand-edited cache can never be accepted in-session.         *)
(* ========================================================================= *)

(* Payloads are hex so that terms containing spaces, newlines, `*` or
   backslashes survive a line-oriented cache unchanged. *)
let ix89_hex_of_string s =
  let b = Buffer.create (2 * String.length s) in
  String.iter
    (fun c -> Buffer.add_string b (Printf.sprintf "%02x" (Char.code c)))
    s;
  Buffer.contents b;;

let ix89_string_of_hex h =
  let n = String.length h / 2 in
  let b = Bytes.create n in
  for i = 0 to n - 1 do
    Bytes.set b i (Char.chr (int_of_string ("0x" ^ String.sub h (2 * i) 2)))
  done;
  Bytes.to_string b;;

(* A cache is a list of "KEY value" lines; a line with no space has an empty
   value.  Duplicate keys are preserved, so malformed caches can be detected
   rather than silently merged. *)
let ix89_read_cache path =
  let ic = open_in path in
  let rec loop acc =
    match (try Some (input_line ic) with End_of_file -> None) with
      None -> close_in ic; List.rev acc
    | Some line ->
        (match String.index_opt line ' ' with
           None -> loop ((line, "") :: acc)
         | Some i ->
             let k = String.sub line 0 i in
             let v = String.sub line (i + 1) (String.length line - i - 1) in
             loop ((k, v) :: acc))
  in
  loop [];;

let ix89_write_cache path lines =
  let oc = open_out path in
  List.iter (fun l -> output_string oc l; output_char oc '\n') lines;
  close_out oc;;

(* Lower-case even-length hex, non-empty.  Upper case is rejected on purpose:
   the writer emits lower case, so upper case means the file was edited. *)
let ix89_is_hex h =
  let n = String.length h in
  let ok = ref (n > 0 && n mod 2 = 0) in
  String.iter
    (fun c ->
       if not ((c >= '0' && c <= '9') || (c >= 'a' && c <= 'f'))
       then ok := false)
    h;
  !ok;;

let ix89_is_hex_of_length len h =
  String.length h = len &&
  (let ok = ref true in
   String.iter
     (fun c ->
        if not ((c >= '0' && c <= '9') || (c >= 'a' && c <= 'f'))
        then ok := false)
     h;
   !ok);;

let ix89_is_digits s =
  String.length s > 0 &&
  (let ok = ref true in
   String.iter (fun c -> if c < '0' || c > '9' then ok := false) s;
   !ok);;

let ix89_mem x xs = List.exists (fun y -> y = x) xs;;

(* Structural validation.  Reason names match stmt_check.sh's bash validator
   so the self-test can assert a single vocabulary.  Returns [] iff usable. *)
let ix89_validate_cache ~task ~theorem ~source path =
  let errors = ref [] in
  let err s = if not (ix89_mem s !errors) then errors := s :: !errors in
  let allowed =
    ["TASK"; "THEOREM"; "SOURCE"; "REPARSE"; "STMT_LEN"; "STMT";
     "UPSTREAM_COMMIT"; "SOURCE_DIGEST"; "THEORY_DIGEST"; "OBJECT_DIGEST";
     "AXIOM"] in
  let entries = try ix89_read_cache path with _ -> [] in
  if entries = [] then err "cache-empty"
  else begin
    List.iter
      (fun (k, _) -> if not (ix89_mem k allowed) then err "unknown-field")
      entries;
    let fields k = List.filter (fun (k', _) -> k' = k) entries in
    let count k = List.length (fields k) in
    let value k = match fields k with [(_, v)] -> Some v | _ -> None in
    let require_one k = if count k <> 1 then err (String.lowercase_ascii k ^ "-field") in
    (* TASK / THEOREM / SOURCE *)
    require_one "TASK";
    (match value "TASK" with
       | Some v when v = task -> ()
       | Some _ -> err "task-mismatch"
       | None -> ());
    require_one "THEOREM";
    (match value "THEOREM" with
       | Some v when v = theorem -> ()
       | Some _ -> err "theorem-mismatch"
       | None -> ());
    require_one "SOURCE";
    (match value "SOURCE" with
       | Some v when v = source -> ()
       | Some _ -> err "source-mismatch"
       | None -> ());
    (* REPARSE: exactly one, and its value must be 1. *)
    require_one "REPARSE";
    (match value "REPARSE" with
       | Some "1" -> ()
       | Some _ -> err "reparse-not-1"
       | None -> ());
    (* STMT_LEN / STMT, with the length checked against the decoded bytes. *)
    require_one "STMT_LEN";
    require_one "STMT";
    (match value "STMT" with
       | None -> ()
       | Some h ->
           if not (ix89_is_hex h) then err "stmt-encoding"
           else
             (match value "STMT_LEN" with
                | None -> ()
                | Some n ->
                    if not (ix89_is_digits n) then err "stmt-length-malformed"
                    else if int_of_string n <> String.length h / 2 then
                      err "stmt-length-mismatch"));
    (* Provenance fields: format only here; digests are compared in bash. *)
    require_one "UPSTREAM_COMMIT";
    (match value "UPSTREAM_COMMIT" with
       | Some v when ix89_is_hex_of_length 40 v -> ()
       | Some _ -> err "upstream-commit-malformed"
       | None -> ());
    List.iter
      (fun (k, reason) ->
         require_one k;
         match value k with
           | Some v when ix89_is_hex_of_length 64 v -> ()
           | Some _ -> err (reason ^ "-malformed")
           | None -> ())
      [("SOURCE_DIGEST", "source-digest");
       ("THEORY_DIGEST", "theory-digest");
       ("OBJECT_DIGEST", "object-digest")];
    (* The standard theory has exactly three axioms; the in-session check
       compares their terms with axioms(). *)
    if count "AXIOM" <> 3 then err "axiom-count"
    else List.iter
      (fun (_, v) -> if not (ix89_is_hex v) then err "axiom-encoding")
      (fields "AXIOM")
  end;
  List.rev !errors;;

let ix89_truncate n s =
  if String.length s <= n then s
  else String.sub s 0 n ^ "...[truncated]";;

(* Terms pretty-print over several lines; a failure record must stay on one
   greppable line. *)
let ix89_oneline s =
  String.concat " " (String.split_on_char '\n' s);;
