(* Failure-side extension to TacticTrace.

   Load this file inside HOL Light *after* exportTrace.ml:

     loadt "exportTrace.ml";;
     loadt "exptrace_failure.ml";;

   or with the mechanism the surrounding project uses for exportTrace.ml.

   Scope of a record
   -----------------
   A record written here means exactly: "this tactic application raised at
   least once". It does NOT mean "the proof failed". The wrapper sees one
   tactic application and cannot see whether a combinator, TRY/ORELSE, or a
   later step recovered, nor what the attempt's terminal outcome was. Any
   downstream label must therefore combine these records with the attempt's
   outcome, which this module does not observe.

   Write-through, not buffering
   ----------------------------
   TacticTrace writes only when exptrace_dump () is called at the end of a
   successful proof, so a proof that raises loses all evidence. This module
   must not repeat that defect. At record time it appends one JSON object per
   line to <tacname>.failures.jsonl and closes the file before re-raising.
   No later call is required: if the process exits immediately, the evidence
   is already on disk.

   exptrace_failures_dump () is only an optional consolidation step. It
   merges the journals into <tacname>.failures.json arrays in TacticTrace's
   dump style, sorted deterministically, refusing to overwrite output. It is
   safe never to call it.

   Dependency-free: only the OCaml standard library and pieces HOL Light
   already requires. No Yojson.

   HOL Light globals relied on:
     - `goal`          : the tactic goal type, term list * term (fusion.ml)
     - `pp_print_goal` : the goal pretty-printer TacticTrace uses in its dump
                         (printer.ml); failure goal_before must serialise the
                         same way as success goal_before or the two channels
                         cannot be compared.
     - `ExportTrace`   : loaded before this file; used only to locate the same
                         output directory TacticTrace writes to.
   `string_of_term` (HOL Light) is deliberately not used for goal_before:
   TacticTrace serialises the whole goal with pp_print_goal. If
   hypothesis-level text is added later, render it as
   `string_of_term (concl th)` and escape it with String.escaped, exactly as
   exportTrace.ml does for added/removed assumptions.

   The success-record path is untouched. exptrace_guard wraps only the
   tactic application expression and re-raises the original exception
   unchanged, so HOL Light's semantics are identical to calling the
   application directly.
*)

unset_jrh_lexer;;

type tac_failure = {
  scope: string;
  tactic: string;
  goal_before: goal;
  exception_text: string;
  subgoal_index: int option;
  elapsed_cpu_s: float;
}

let exptrace_failure_scope = "tactic"

(* Only counts are kept in memory. The evidence itself is on disk as soon as
   it is recorded. *)
let exptrace_failure_counts : (string, int) Hashtbl.t = Hashtbl.create 32

let exptrace_failure_dir_cache : string option ref = ref None

(* The failure channel uses the same output directory as TacticTrace: consult
   ExportTrace (loaded first) instead of duplicating its configuration logic.
   This is the one place the module reads TacticTrace state. *)
let exptrace_failures_dir () : string =
  match !exptrace_failure_dir_cache with
  | Some dir -> dir
  | None ->
    let dir =
      ExportTrace.effective_output_dir (ExportTrace.get_sampling_config ())
    in
    exptrace_failure_dir_cache := Some dir;
    dir

let exptrace_ensure_dir (dir : string) : unit =
  if Sys.file_exists dir then begin
    if not (Sys.is_directory dir) then
      invalid_arg ("Failure trace directory is not a directory: " ^ dir)
  end else
    (try Sys.mkdir dir 0o777
     with Sys_error _ ->
       (* Another writer may have won the race; only fail if the directory
          still does not exist. *)
       if not (Sys.file_exists dir) then
         invalid_arg ("Cannot create failure trace directory: " ^ dir))

let exptrace_goal_to_string (g : goal) : string =
  Format.asprintf "%a" pp_print_goal g

let exptrace_failure_journal_path (name : string) : string =
  exptrace_failures_dir () ^ "/" ^ name ^ ".failures.jsonl"

(* One JSON object per line, flushed and closed immediately. The key style
   mirrors TacticTrace's dump: String.escaped values, same field spelling.
   Append is intentional: a tactic may raise more than once in one run. *)
let exptrace_write_failure_line (r : tac_failure) : unit =
  let dir = exptrace_failures_dir () in
  exptrace_ensure_dir dir;
  let path = exptrace_failure_journal_path r.tactic in
  let oc =
    open_out_gen [Open_wronly; Open_creat; Open_append; Open_text] 0o666 path
  in
  try
    Printf.fprintf oc
      "{\"scope\": \"%s\",\"tactic\":\"%s\",\"goal_before\": \"%s\",\"exception_text\": \"%s\",\"subgoal_index\": %s,\"elapsed_cpu_s\": %g}\n"
      (String.escaped r.scope)
      (String.escaped r.tactic)
      (String.escaped (exptrace_goal_to_string r.goal_before))
      (String.escaped r.exception_text)
      (match r.subgoal_index with
       | None -> "null"
       | Some i -> string_of_int i)
      r.elapsed_cpu_s;
    close_out oc
  with e ->
    close_out_noerr oc;
    raise e

let exptrace_record_failure
    (name : string) (goal_before : goal)
    (exception_text : string) (elapsed_cpu_s : float) : unit =
  let record : tac_failure = {
    scope = exptrace_failure_scope;
    tactic = name;
    goal_before;
    exception_text;
    subgoal_index = None;  (* Unknown until combinators are instrumented;
                              see research/03-INSTRUMENT.md section 4. *)
    elapsed_cpu_s;
  } in
  exptrace_write_failure_line record;
  (* Count only after the line is closed, so the count is a lower bound on
     durable records, not on attempted writes. *)
  let previous =
    match Hashtbl.find_opt exptrace_failure_counts name with
    | None -> 0
    | Some n -> n
  in
  Hashtbl.replace exptrace_failure_counts name (previous + 1)

(* [exptrace_guard name goal_before f] has exactly the semantics of [f ()]:
   it returns the same value, or raises the same exception. Its only extra
   effect is the write-through record. Timing uses Sys.time (CPU time); the
   Unix library is not needed.

   If recording itself fails, the original exception is still re-raised: the
   observer must never replace the behaviour it observes. A warning is
   printed because losing evidence silently would be worse, but the
   exception value is unchanged. *)
let exptrace_guard (name : string) (goal_before : goal) (f : unit -> 'a) : 'a =
  let started = Sys.time () in
  try f () with e ->
    let elapsed_cpu_s = Sys.time () -. started in
    (try
       exptrace_record_failure name goal_before (Printexc.to_string e)
         elapsed_cpu_s
     with record_error ->
       prerr_endline
         ("exptrace_guard: could not record failure for " ^ name ^ ": " ^
          Printexc.to_string record_error));
    raise e

(* ------------------------------------------------------------------------- *)
(* Optional consolidation: .jsonl journals -> .failures.json arrays.          *)
(* ------------------------------------------------------------------------- *)

let exptrace_read_lines (path : string) : string list =
  let ic = open_in path in
  let rec loop acc =
    match input_line ic with
    | line -> loop (line :: acc)
    | exception End_of_file -> close_in_noerr ic; List.rev acc
    | exception e -> close_in_noerr ic; raise e
  in
  loop []

(* Split on commas that are outside JSON strings. The journals contain only
   flat objects, so this is enough to recover the original field texts. *)
let exptrace_split_top_level_commas (s : string) : string list =
  let n = String.length s in
  let parts = ref [] in
  let start = ref 0 in
  let in_string = ref false in
  let escaped = ref false in
  for i = 0 to n - 1 do
    let c = s.[i] in
    if !in_string then begin
      if !escaped then escaped := false
      else if c = '\\' then escaped := true
      else if c = '"' then in_string := false
    end else begin
      if c = '"' then in_string := true
      else if c = ',' then begin
        parts := String.sub s !start (i - !start) :: !parts;
        start := i + 1
      end
    end
  done;
  List.rev (String.sub s !start (n - !start) :: !parts)

let exptrace_journal_fields
    (path : string) (lineno : int) (line : string) : string list =
  let n = String.length line in
  if n < 2 || line.[0] <> '{' || line.[n - 1] <> '}' then
    invalid_arg
      (Printf.sprintf "Malformed failure journal line %s:%d" path lineno);
  let fields =
    exptrace_split_top_level_commas (String.sub line 1 (n - 2))
  in
  if List.exists (fun field -> field = "") fields then
    invalid_arg
      (Printf.sprintf "Malformed failure journal line %s:%d" path lineno);
  fields

let exptrace_write_consolidated_file
    (path : string) (lines : string list) : unit =
  (* Validate and sort before opening anything, so a bad journal never
     leaves a partial .json file behind. *)
  let parsed =
    List.mapi
      (fun i line -> line, exptrace_journal_fields path (i + 1) line)
      lines
  in
  let parsed =
    List.sort (fun (a, _) (b, _) -> String.compare a b) parsed
  in
  let total = List.length parsed in
  if Sys.file_exists path then
    invalid_arg ("Failure trace already exists: " ^ path);
  let oc =
    try open_out_gen [Open_wronly; Open_creat; Open_excl; Open_text] 0o666 path
    with Sys_error msg ->
      invalid_arg ("Cannot create failure trace " ^ path ^ ": " ^ msg)
  in
  try
    Printf.fprintf oc "[\n";
    List.iteri (fun i (_, fields) ->
        Printf.fprintf oc "  {\n";
        let last = List.length fields - 1 in
        List.iteri (fun j field ->
            Printf.fprintf oc "    %s%s\n" field
              (if j = last then "" else ","))
          fields;
        Printf.fprintf oc "  }%s\n"
          (if i + 1 = total then "" else ","))
      parsed;
    Printf.fprintf oc "]\n";
    close_out oc;
    Printf.printf "Dumped to %s\n" path
  with e ->
    close_out_noerr oc;
    (* Do not leave a half-written file that would block a retry. *)
    (try Sys.remove path with Sys_error _ -> ());
    raise e

(* Optional. No-op when nothing was recorded. For every tactic with a
   journal, write <dir>/<tacname>.failures.json once. Every destination is
   checked before any file is opened. It is safe to never call this. *)
let exptrace_failures_dump () : unit =
  if Hashtbl.length exptrace_failure_counts = 0 then () else begin
    let dir = exptrace_failures_dir () in
    if Sys.file_exists dir && not (Sys.is_directory dir) then
      invalid_arg ("Failure trace directory is not a directory: " ^ dir);
    let tactics =
      List.sort compare
        (Hashtbl.fold
           (fun name _ acc -> name :: acc)
           exptrace_failure_counts [])
    in
    let jobs =
      List.map (fun name ->
          let journal = exptrace_failure_journal_path name in
          let consolidated = dir ^ "/" ^ name ^ ".failures.json" in
          let expected =
            match Hashtbl.find_opt exptrace_failure_counts name with
            | None -> 0
            | Some n -> n
          in
          name, journal, consolidated, expected)
        tactics
    in
    List.iter (fun (name, journal, consolidated, _) ->
        if not (Sys.file_exists journal) then
          invalid_arg
            ("Failure journal is missing for " ^ name ^ ": " ^ journal);
        if Sys.file_exists consolidated then
          invalid_arg ("Failure trace already exists: " ^ consolidated))
      jobs;
    List.iter (fun (name, journal, consolidated, expected) ->
        let lines = exptrace_read_lines journal in
        if List.length lines <> expected then
          invalid_arg
            (Printf.sprintf
               "Failure journal for %s has %d line(s), expected %d: %s"
               name (List.length lines) expected journal);
        exptrace_write_consolidated_file consolidated lines)
      jobs
  end;;

set_jrh_lexer;;
