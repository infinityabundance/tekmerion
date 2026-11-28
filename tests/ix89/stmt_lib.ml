(* ========================================================================= *)
(* tests/ix89/stmt_lib.ml -- XXVIII.A statement-check helpers.               *)
(*                                                                           *)
(* Shared by the extraction driver (loads the upstream `_CORRECT` proof once *)
(* and caches its conclusion) and by the block injected into the generated   *)
(* proof (re-parses the cached conclusion and compares it with `aconv` in    *)
(* the same session that produces the theorem).  No inference rule is        *)
(* invoked here, so loading this file cannot change `inference_count`.       *)
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
   value.  Duplicate keys are preserved, so the axiom list stays complete. *)
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

let ix89_truncate n s =
  if String.length s <= n then s
  else String.sub s 0 n ^ "...[truncated]";;

(* Terms pretty-print over several lines; a failure record must stay on one
   greppable line. *)
let ix89_oneline s =
  String.concat " " (String.split_on_char '\n' s);;
