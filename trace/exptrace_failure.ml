(* Failure-side extension to TacticTrace, with an attempt-level event stream.

   Load this file inside HOL Light *after* exportTrace.ml:

     loadt 'exportTrace.ml';;
     loadt 'exptrace_failure.ml';;

   or with the mechanism the surrounding project uses for exportTrace.ml.

   This module has no dependency on ExportTrace and no dependency on Yojson or
   Unix. It is standalone: the only names it needs from the surrounding session
   are HOL Light's `goal`, `pp_print_goal`, `term`, and `string_of_term`.

   Two artefacts, one contract
   ---------------------------
   1. Attempt stream. `exptrace_attempt_start` opens
      <attempt_dir>/<attempt_id>.jsonl and writes `attempt_start` immediately.
      Every guarded application then contributes one event to an in-memory
      retention buffer, and `exptrace_attempt_end outcome` appends the retained
      events in strictly increasing `seq` order followed by `attempt_end`, then
      writes <attempt_id>.manifest.json. The stream is the primary artefact.

   2. Legacy failure journals. `<name>.failures.jsonl` (tactics) and
      `<name>.conv.failures.jsonl` (conversions) keep the records that named
      this project in the first place. They now carry a digest and a byte size
      per record, and in attempt mode they are written only for retained
      records. A dropped record is still counted, with a rolling digest and a
      byte total, in the manifest.

   Bounded retention (I2)
   ----------------------
   Event volume is dominated by conversions: one `word_max` run raised ~3.67M
   conversion failures (578 MB when every input term was written). The policy:

     * every event updates its wrapper's count, kind counts, rolling digest and
       serialized byte total - nothing is silently lost;
     * a full serialized term is kept for the first 32 and last 32 events of
       each (scope, name) wrapper, and only if the term is at most
       `exptrace_max_term_bytes` (16 KiB) and the channel's retained-byte cap
       (16 MiB) has room;
     * events outside that window are counted, digested and sized, but not
       written to the stream;
     * the manifest states, per channel and per wrapper, total events,
       retained, dropped, the cap, the rolling digest and the byte totals, and
       sets `stream_complete = false` whenever anything was dropped.

   The declared per-attempt budget is `exptrace_declared_budget_bytes` (128 MiB)
   for the attempt stream plus journals plus manifest. The hard caps above keep
   the instrument below it with a wide margin.

   Ordering and write-through
   --------------------------
   `attempt_start` (seq 0) is flushed at start. Retained events are held in a
   bounded buffer so that the file can be written in strictly increasing `seq`
   order at the end; `attempt_end` is last. `attempt_end` flushes and closes the
   stream before anything else runs, and an `at_exit` handler finalises with
   outcome 'unknown' if the attempt never reached an explicit end, so an
   ordinary process exit cannot lose the buffer.

   What a record does and does not mean
   ------------------------------------
   A `tactic_raised` event means 'this tactic application raised at least
   once'. It does NOT mean 'the proof failed': a combinator, TRY/ORELSE, or a
   later step may have recovered. Only `attempt_end.outcome` states the
   attempt's terminal result, and `timeout` / `unknown` are real values. A
   `conv_raised` event is the same observation for one conversion application.
   Subgoal indices are not observable here; see research/03-INSTRUMENT.md
   section 4.

   Conversion input evaluation order (I4)
   ---------------------------------------
   `exptrace_guard_conv` takes `(input : unit -> term)`. It never forces the
   thunk before the conversion runs. On failure it forces it to record the
   input that raised. On success it forces it only when an attempt is active,
   after the conversion has returned, to compute the success event's digest and
   size; this matches stock TacticTrace, which records the conversion input
   only after the application succeeds. With no active attempt the thunk is
   not forced on success at all.

   JSON encoding (I5)
   ------------------
   `exptrace_json_escape` escapes `'`, `\`, and every character below 0x20 as
   `\u00XX`. This is the writer used by every new artefact in this module. The
   TacticTrace success writer (`exportTrace.ml`) is untouched and still uses
   `String.escaped`, because conformance requires its bytes to be identical.
   Caveat: the encoder passes bytes >= 0x80 through unchanged; the output is
   valid JSON only if those bytes form valid UTF-8. HOL Light's printer emits
   ASCII, so this holds for the current corpus, but it is not enforced.

   Directory selection
   -------------------
     failure journals:
       1. exptrace_failures_set_dir '<dir>';
       2. TEKMERION_FAILURE_DIR;
       3. './tekmerion-failures'.
     attempt stream:
       1. exptrace_attempt_set_dir '<dir>';
       2. TEKMERION_ATTEMPT_DIR;
       3. './tekmerion-attempts'.

   A missing parent is an error; only the leaf directory is created, matching
   TacticTrace's exptrace_dump.

   The success-record path is untouched. `exptrace_guard` and
   `exptrace_guard_conv` wrap only the application expression and re-raise the
   original exception unchanged, so HOL Light's semantics are identical to
   calling the application directly. If recording itself fails, a warning is
   printed and the original result or exception still propagates.
*)

unset_jrh_lexer;;

(* ------------------------------------------------------------------------- *)
(* JSON escaping for our own writers (I5).                                    *)
(* ------------------------------------------------------------------------- *)

let exptrace_json_escape (s : string) : string =
  let b = Buffer.create (String.length s + 16) in
  String.iter
    (fun c ->
      match c with
      | '"' -> Buffer.add_string b "\\\""
      | '\\' -> Buffer.add_string b "\\\\"
      | c when Char.code c < 0x20 ->
          Buffer.add_string b (Printf.sprintf "\\u%04x" (Char.code c))
      | c -> Buffer.add_char b c)
    s;
  Buffer.contents b

(* ------------------------------------------------------------------------- *)
(* SHA-256. Dependency-free, standard FIPS 180-4.                             *)
(* ------------------------------------------------------------------------- *)

let exptrace_rotr (x : int32) (n : int) : int32 =
  Int32.logor (Int32.shift_right_logical x n) (Int32.shift_left x (32 - n))

let exptrace_sha256_hex (s : string) : string =
  let k =
    [| 0x428a2f98l; 0x71374491l; 0xb5c0fbcfl; 0xe9b5dba5l;
       0x3956c25bl; 0x59f111f1l; 0x923f82a4l; 0xab1c5ed5l;
       0xd807aa98l; 0x12835b01l; 0x243185bel; 0x550c7dc3l;
       0x72be5d74l; 0x80deb1fel; 0x9bdc06a7l; 0xc19bf174l;
       0xe49b69c1l; 0xefbe4786l; 0x0fc19dc6l; 0x240ca1ccl;
       0x2de92c6fl; 0x4a7484aal; 0x5cb0a9dcl; 0x76f988dal;
       0x983e5152l; 0xa831c66dl; 0xb00327c8l; 0xbf597fc7l;
       0xc6e00bf3l; 0xd5a79147l; 0x06ca6351l; 0x14292967l;
       0x27b70a85l; 0x2e1b2138l; 0x4d2c6dfcl; 0x53380d13l;
       0x650a7354l; 0x766a0abbl; 0x81c2c92el; 0x92722c85l;
       0xa2bfe8a1l; 0xa81a664bl; 0xc24b8b70l; 0xc76c51a3l;
       0xd192e819l; 0xd6990624l; 0xf40e3585l; 0x106aa070l;
       0x19a4c116l; 0x1e376c08l; 0x2748774cl; 0x34b0bcb5l;
       0x391c0cb3l; 0x4ed8aa4al; 0x5b9cca4fl; 0x682e6ff3l;
       0x748f82eel; 0x78a5636fl; 0x84c87814l; 0x8cc70208l;
       0x90befffal; 0xa4506cebl; 0xbef9a3f7l; 0xc67178f2l |] in
  let h0 = ref 0x6a09e667l and h1 = ref 0xbb67ae85l in
  let h2 = ref 0x3c6ef372l and h3 = ref 0xa54ff53al in
  let h4 = ref 0x510e527fl and h5 = ref 0x9b05688cl in
  let h6 = ref 0x1f83d9abl and h7 = ref 0x5be0cd19l in
  let ml = String.length s in
  let padded_len = ((ml + 9 + 63) / 64) * 64 in
  let msg = Bytes.make padded_len '\000' in
  Bytes.blit_string s 0 msg 0 ml;
  Bytes.set msg ml '\x80';
  let bits = Int64.mul (Int64.of_int ml) 8L in
  for i = 0 to 7 do
    let shift = 8 * (7 - i) in
    let byte =
      Int64.to_int
        (Int64.logand (Int64.shift_right_logical bits shift) 0xFFL)
    in
    Bytes.set msg (padded_len - 8 + i) (Char.chr byte)
  done;
  let w = Array.make 64 0l in
  for block = 0 to (padded_len / 64) - 1 do
    let base = block * 64 in
    for t = 0 to 15 do
      let i = base + (t * 4) in
      w.(t) <-
        Int32.logor
          (Int32.shift_left (Int32.of_int (Char.code (Bytes.get msg i))) 24)
          (Int32.logor
             (Int32.shift_left
                (Int32.of_int (Char.code (Bytes.get msg (i + 1)))) 16)
             (Int32.logor
                (Int32.shift_left
                   (Int32.of_int (Char.code (Bytes.get msg (i + 2)))) 8)
                (Int32.of_int (Char.code (Bytes.get msg (i + 3))))))
    done;
    for t = 16 to 63 do
      let x = w.(t - 15) and y = w.(t - 2) in
      let s0 =
        Int32.logxor (exptrace_rotr x 7)
          (Int32.logxor (exptrace_rotr x 18) (Int32.shift_right_logical x 3))
      in
      let s1 =
        Int32.logxor (exptrace_rotr y 17)
          (Int32.logxor (exptrace_rotr y 19) (Int32.shift_right_logical y 10))
      in
      w.(t) <- Int32.add (Int32.add w.(t - 16) s0) (Int32.add w.(t - 7) s1)
    done;
    let a = ref !h0 and b = ref !h1 and c = ref !h2 and d = ref !h3 in
    let e = ref !h4 and f = ref !h5 and g = ref !h6 and h = ref !h7 in
    for t = 0 to 63 do
      let s1 =
        Int32.logxor (exptrace_rotr !e 6)
          (Int32.logxor (exptrace_rotr !e 11) (exptrace_rotr !e 25))
      in
      let ch =
        Int32.logxor (Int32.logand !e !f)
          (Int32.logand (Int32.lognot !e) !g)
      in
      let temp1 =
        Int32.add
          (Int32.add (Int32.add !h s1) (Int32.add ch k.(t)))
          w.(t)
      in
      let s0 =
        Int32.logxor (exptrace_rotr !a 2)
          (Int32.logxor (exptrace_rotr !a 13) (exptrace_rotr !a 22))
      in
      let maj =
        Int32.logxor (Int32.logand !a !b)
          (Int32.logxor (Int32.logand !a !c) (Int32.logand !b !c))
      in
      let temp2 = Int32.add s0 maj in
      h := !g;
      g := !f;
      f := !e;
      e := Int32.add !d temp1;
      d := !c;
      c := !b;
      b := !a;
      a := Int32.add temp1 temp2
    done;
    h0 := Int32.add !h0 !a;
    h1 := Int32.add !h1 !b;
    h2 := Int32.add !h2 !c;
    h3 := Int32.add !h3 !d;
    h4 := Int32.add !h4 !e;
    h5 := Int32.add !h5 !f;
    h6 := Int32.add !h6 !g;
    h7 := Int32.add !h7 !h
  done;
  let buf = Buffer.create 64 in
  List.iter
    (fun x ->
      for i = 7 downto 0 do
        let nib =
          Int32.to_int
            (Int32.logand (Int32.shift_right_logical x (4 * i)) 0xFl)
        in
        Buffer.add_char buf "0123456789abcdef".[nib]
      done)
    [ !h0; !h1; !h2; !h3; !h4; !h5; !h6; !h7 ];
  Buffer.contents buf

(* ------------------------------------------------------------------------- *)
(* Retention policy and declared budget.                                     *)
(* ------------------------------------------------------------------------- *)

let exptrace_first_n = 32
let exptrace_last_n = 32
let exptrace_max_term_bytes = 16 * 1024
let exptrace_channel_retained_cap_bytes = 16 * 1024 * 1024
let exptrace_exception_text_max = 1024
let exptrace_declared_budget_bytes = 128 * 1024 * 1024

(* ------------------------------------------------------------------------- *)
(* Records written to the legacy per-name journals.                          *)
(* ------------------------------------------------------------------------- *)

type tac_failure = {
  scope: string;
  tactic: string;
  goal_before: goal;
  exception_text: string;
  subgoal_index: int option;
  elapsed_cpu_s: float;
}

type conv_failure = {
  conv_scope: string;
  conv: string;
  input: term;
  conv_exception_text: string;
  conv_elapsed_cpu_s: float;
}

let exptrace_failure_scope = "tactic"
let exptrace_conv_failure_scope = "conv"

(* Only counts are kept for the legacy consolidation step. Separate tables
   keep tactic and conversion names from colliding. *)
let exptrace_failure_counts : (string, int) Hashtbl.t = Hashtbl.create 32
let exptrace_conv_failure_counts : (string, int) Hashtbl.t = Hashtbl.create 32

let exptrace_failure_dir_cache : string option ref = ref None
let exptrace_attempt_dir_cache : string option ref = ref None
let exptrace_attempt_mode_cache : bool option ref = ref None

let exptrace_failures_set_dir (dir : string) : unit =
  if String.length dir = 0 then
    invalid_arg "exptrace_failures_set_dir: empty directory";
  exptrace_failure_dir_cache := Some dir

let exptrace_attempt_set_dir (dir : string) : unit =
  if String.length dir = 0 then
    invalid_arg "exptrace_attempt_set_dir: empty directory";
  exptrace_attempt_dir_cache := Some dir;
  exptrace_attempt_mode_cache := Some true

let exptrace_failures_dir () : string =
  match !exptrace_failure_dir_cache with
  | Some dir -> dir
  | None ->
    let dir =
      match Sys.getenv_opt "TEKMERION_FAILURE_DIR" with
      | Some value when String.length value > 0 -> value
      | _ -> "./tekmerion-failures"
    in
    exptrace_failure_dir_cache := Some dir;
    dir

let exptrace_attempt_dir () : string =
  match !exptrace_attempt_dir_cache with
  | Some dir -> dir
  | None ->
    let dir =
      match Sys.getenv_opt "TEKMERION_ATTEMPT_DIR" with
      | Some value when String.length value > 0 -> value
      | _ -> "./tekmerion-attempts"
    in
    exptrace_attempt_dir_cache := Some dir;
    dir

let exptrace_ensure_dir (dir : string) : unit =
  if Sys.file_exists dir then begin
    if not (Sys.is_directory dir) then
      invalid_arg ("Trace directory is not a directory: " ^ dir)
  end else
    (try Sys.mkdir dir 0o777
     with Sys_error _ ->
       if not (Sys.file_exists dir) then
         invalid_arg ("Cannot create trace directory: " ^ dir))

let exptrace_goal_to_string (g : goal) : string =
  Format.asprintf "%a" pp_print_goal g

(* ------------------------------------------------------------------------- *)
(* Shared state: per-channel and per-wrapper accumulators.                   *)
(* ------------------------------------------------------------------------- *)

type exptrace_channel = {
  ch_scope: string;
  mutable ch_total: int;
  mutable ch_ok: int;
  mutable ch_raised: int;
  mutable ch_streamed: int;
  mutable ch_dropped: int;
  mutable ch_terms_retained: int;
  mutable ch_terms_dropped: int;
  mutable ch_term_bytes: int;
  mutable ch_retained_bytes: int;
  mutable ch_digest: string;
  mutable ch_roll: int64;
}

let exptrace_new_channel (scope : string) : exptrace_channel = {
  ch_scope = scope;
  ch_total = 0;
  ch_ok = 0;
  ch_raised = 0;
  ch_streamed = 0;
  ch_dropped = 0;
  ch_terms_retained = 0;
  ch_terms_dropped = 0;
  ch_term_bytes = 0;
  ch_retained_bytes = 0;
  ch_digest = "";
  ch_roll = 0L;
}

let exptrace_tactic_channel = exptrace_new_channel exptrace_failure_scope
let exptrace_conv_channel = exptrace_new_channel exptrace_conv_failure_scope

let exptrace_channel_of_scope (scope : string) : exptrace_channel =
  if scope = exptrace_conv_failure_scope then exptrace_conv_channel
  else exptrace_tactic_channel

(* A retained event keeps the raw HOL value only until it is written; a middle
   event is never serialized at all.  This is what keeps the instrument off the
   hot conversion path: only the first 32 and last 32 events of a wrapper ever
   pay for `pp_print_goal`/`string_of_term`. *)
type exptrace_raw =
  | RawGoal of goal
  | RawTerm of term
  | RawAbsent

type exptrace_pending = {
  p_seq: int;
  p_kind: string;
  p_name: string;
  p_scope: string;
  mutable p_digest: string;
  mutable p_bytes: int;
  p_exception: string option;
  p_exception_bytes: int;
  p_exception_truncated: bool;
  p_elapsed: float;
  mutable p_term: string option;
  mutable p_raw: exptrace_raw option;
  mutable p_journal_written: bool;
}

type exptrace_wrapper = {
  w_scope: string;
  w_name: string;
  mutable w_total: int;
  mutable w_ok: int;
  mutable w_raised: int;
  mutable w_streamed: int;
  mutable w_dropped: int;
  mutable w_terms_retained: int;
  mutable w_terms_dropped: int;
  mutable w_term_bytes: int;
  mutable w_digest: string;
  mutable w_roll: int64;
  mutable w_seen: int;
  (* Newest first; bounded by exptrace_last_n. *)
  mutable w_tail: exptrace_pending list;
}

let exptrace_tactic_wrappers : (string, exptrace_wrapper) Hashtbl.t =
  Hashtbl.create 160
let exptrace_conv_wrappers : (string, exptrace_wrapper) Hashtbl.t =
  Hashtbl.create 256

let exptrace_wrapper_table (scope : string) =
  if scope = exptrace_conv_failure_scope then exptrace_conv_wrappers
  else exptrace_tactic_wrappers

let exptrace_get_wrapper (scope : string) (name : string) : exptrace_wrapper =
  let table = exptrace_wrapper_table scope in
  match Hashtbl.find_opt table name with
  | Some w -> w
  | None ->
    let w = {
      w_scope = scope;
      w_name = name;
      w_total = 0;
      w_ok = 0;
      w_raised = 0;
      w_streamed = 0;
      w_dropped = 0;
      w_terms_retained = 0;
      w_terms_dropped = 0;
      w_term_bytes = 0;
      w_digest = "";
      w_roll = 0L;
      w_seen = 0;
      w_tail = [];
    } in
    Hashtbl.replace table name w;
    w

let exptrace_roll (state : int64) (fingerprint : int) : int64 =
  Int64.add (Int64.mul state 1000003L) (Int64.of_int fingerprint)

(* ------------------------------------------------------------------------- *)
(* Attempt state.                                                            *)
(* ------------------------------------------------------------------------- *)

type exptrace_attempt = {
  a_id: string;
  a_theorem: string;
  a_proof_sha: string;
  a_injection: string;
  a_wrapper_sha: string;
  a_module_sha: string;
  a_dir: string;
  a_stream_path: string;
  mutable a_stream: out_channel option;
  mutable a_seq: int;
  mutable a_ended: bool;
  mutable a_error: string option;
  mutable a_retained: exptrace_pending list;  (* reverse order *)
  a_cpu_started: float;
}

let exptrace_attempt : exptrace_attempt option ref = ref None

(* Whether attempt recording is configured at all.  The runner exports
   TEKMERION_ATTEMPT_DIR before the process starts, so library initialisation
   (which runs millions of guarded applications before the proof) can be
   skipped cheaply: configured but not yet started means "ignore".  Without
   that variable the module keeps its legacy, unbounded failure-journal mode. *)
let exptrace_attempt_configured () : bool =
  match !exptrace_attempt_mode_cache with
  | Some flag -> flag
  | None ->
    let flag =
      match Sys.getenv_opt "TEKMERION_ATTEMPT_DIR" with
      | Some value when String.length value > 0 -> true
      | _ -> false
    in
    exptrace_attempt_mode_cache := Some flag;
    flag

(* Outcome used by the at_exit finaliser if the attempt never reaches an
   explicit exptrace_attempt_end.  The runner sets this to "failed" before
   loading the proof: an uncaught tactic exception then preserves HOL Light's
   semantics (the process still exits non-zero) while the stream still ends
   with attempt_end outcome="failed". *)
let exptrace_expected_outcome = ref "unknown"

let exptrace_attempt_expect (outcome : string) : unit =
  exptrace_expected_outcome := outcome

let exptrace_kind_counts : (string, int) Hashtbl.t = Hashtbl.create 8

let exptrace_reset_state () : unit =
  Hashtbl.reset exptrace_tactic_wrappers;
  Hashtbl.reset exptrace_conv_wrappers;
  Hashtbl.reset exptrace_kind_counts;
  let reset (c : exptrace_channel) : unit =
    c.ch_total <- 0;
    c.ch_ok <- 0;
    c.ch_raised <- 0;
    c.ch_streamed <- 0;
    c.ch_dropped <- 0;
    c.ch_terms_retained <- 0;
    c.ch_terms_dropped <- 0;
    c.ch_term_bytes <- 0;
    c.ch_retained_bytes <- 0;
    c.ch_digest <- "";
    c.ch_roll <- 0L
  in
  reset exptrace_tactic_channel;
  reset exptrace_conv_channel

let exptrace_bump_kind (kind : string) : unit =
  let previous =
    match Hashtbl.find_opt exptrace_kind_counts kind with
    | None -> 0
    | Some n -> n
  in
  Hashtbl.replace exptrace_kind_counts kind (previous + 1)

(* ------------------------------------------------------------------------- *)
(* Journal writing.                                                          *)
(* ------------------------------------------------------------------------- *)

let exptrace_journal_path (scope : string) (name : string) : string =
  exptrace_failures_dir () ^ "/" ^
  (if scope = exptrace_conv_failure_scope then name ^ ".conv.failures.jsonl"
   else name ^ ".failures.jsonl")

let exptrace_string_or_null (value : string option) : string =
  match value with
  | None -> "null"
  | Some s -> "\"" ^ exptrace_json_escape s ^ "\""

let exptrace_journal_line (p : exptrace_pending) : string =
  let exception_json =
    match p.p_exception with
    | None -> "null"
    | Some s -> "\"" ^ exptrace_json_escape s ^ "\""
  in
  if p.p_scope = exptrace_conv_failure_scope then
    Printf.sprintf
      "{\"scope\": \"conv\",\"conv\":\"%s\",\"input\": %s,\"input_sha256\": \"%s\",\"input_bytes\": %d,\"exception_text\": %s,\"exception_text_bytes\": %d,\"exception_truncated\": %b,\"elapsed_cpu_s\": %g}\n"
      (exptrace_json_escape p.p_name)
      (exptrace_string_or_null p.p_term)
      p.p_digest p.p_bytes exception_json p.p_exception_bytes
      p.p_exception_truncated p.p_elapsed
  else
    Printf.sprintf
      "{\"scope\": \"tactic\",\"tactic\":\"%s\",\"goal_before\": %s,\"goal_sha256\": \"%s\",\"goal_bytes\": %d,\"exception_text\": %s,\"exception_text_bytes\": %d,\"exception_truncated\": %b,\"subgoal_index\": null,\"elapsed_cpu_s\": %g}\n"
      (exptrace_json_escape p.p_name)
      (exptrace_string_or_null p.p_term)
      p.p_digest p.p_bytes exception_json p.p_exception_bytes
      p.p_exception_truncated p.p_elapsed

let exptrace_write_journal_line (p : exptrace_pending) : unit =
  let dir = exptrace_failures_dir () in
  exptrace_ensure_dir dir;
  let path = exptrace_journal_path p.p_scope p.p_name in
  let oc =
    open_out_gen [Open_wronly; Open_creat; Open_append; Open_text] 0o666 path
  in
  (try
     output_string oc (exptrace_journal_line p);
     close_out oc
   with e ->
     close_out_noerr oc;
     raise e);
  p.p_journal_written <- true;
  let table =
    if p.p_scope = exptrace_conv_failure_scope then
      exptrace_conv_failure_counts
    else exptrace_failure_counts
  in
  let previous =
    match Hashtbl.find_opt table p.p_name with
    | None -> 0
    | Some n -> n
  in
  Hashtbl.replace table p.p_name (previous + 1)

(* ------------------------------------------------------------------------- *)
(* Event recording.                                                          *)
(* ------------------------------------------------------------------------- *)

let exptrace_truncate_exception (text : string) : string * int * bool =
  let n = String.length text in
  if n <= exptrace_exception_text_max then (text, n, false)
  else (String.sub text 0 exptrace_exception_text_max, n, true)

let exptrace_raw_to_string (raw : exptrace_raw) : string =
  match raw with
  | RawGoal g -> (try exptrace_goal_to_string g with _ -> "")
  | RawTerm t -> (try string_of_term t with _ -> "")
  | RawAbsent -> ""

let exptrace_raw_fingerprint (raw : exptrace_raw) : int =
  match raw with
  | RawGoal g -> Hashtbl.hash g
  | RawTerm t -> Hashtbl.hash t
  | RawAbsent -> 0

let exptrace_make_pending
    ~(seq : int) ~(kind : string) ~(name : string) ~(scope : string)
    ~(exception_text : string option) ~(elapsed : float)
    ~(raw : exptrace_raw option) : exptrace_pending =
  let exception_text, exception_bytes, exception_truncated =
    match exception_text with
    | None -> (None, 0, false)
    | Some text ->
      let text, bytes, truncated = exptrace_truncate_exception text in
      (Some text, bytes, truncated)
  in
  {
    p_seq = seq;
    p_kind = kind;
    p_name = name;
    p_scope = scope;
    p_digest = "";
    p_bytes = 0;
    p_exception = exception_text;
    p_exception_bytes = exception_bytes;
    p_exception_truncated = exception_truncated;
    p_elapsed = elapsed;
    p_term = None;
    p_raw = raw;
    p_journal_written = false;
  }

let exptrace_attempt_active () : bool =
  match !exptrace_attempt with
  | Some a when not a.a_ended -> true
  | _ -> false

let exptrace_legacy_mode () : bool = not (exptrace_attempt_configured ())

(* Account for one retained term against the channel's byte cap.  Returns
   whether the term may be kept. *)
let exptrace_reserve_term
    (channel : exptrace_channel) (term : string) : bool =
  let n = String.length term in
  if n > exptrace_max_term_bytes then false
  else if channel.ch_retained_bytes + n > exptrace_channel_retained_cap_bytes
  then false
  else begin
    channel.ch_retained_bytes <- channel.ch_retained_bytes + n;
    channel.ch_terms_retained <- channel.ch_terms_retained + 1;
    true
  end

(* Serialize one retained event and update the true-digest chains.  Called
   only for the first/last windows, never for a middle event. *)
let exptrace_serialize_pending
    (channel : exptrace_channel) (p : exptrace_pending) : unit =
  match p.p_raw with
  | None -> ()
  | Some raw ->
    p.p_raw <- None;
    let text = exptrace_raw_to_string raw in
    p.p_digest <- exptrace_sha256_hex text;
    p.p_bytes <- String.length text;
    channel.ch_digest <- exptrace_sha256_hex (channel.ch_digest ^ p.p_digest);
    let wrapper = exptrace_get_wrapper p.p_scope p.p_name in
    wrapper.w_digest <- exptrace_sha256_hex (wrapper.w_digest ^ p.p_digest);
    channel.ch_term_bytes <- channel.ch_term_bytes + p.p_bytes;
    wrapper.w_term_bytes <- wrapper.w_term_bytes + p.p_bytes;
    if exptrace_reserve_term channel text then p.p_term <- Some text

let exptrace_drop_oldest_tail (wrapper : exptrace_wrapper) : unit =
  if List.length wrapper.w_tail > exptrace_last_n then begin
    let rec split_last acc = function
      | [] -> (List.rev acc, None)
      | [ x ] -> (List.rev acc, Some x)
      | x :: xs -> split_last (x :: acc) xs
    in
    let kept, _evicted = split_last [] wrapper.w_tail in
    wrapper.w_tail <- kept
  end

(* Update counters and retention for one event.  A middle event (outside the
   first/last window of its wrapper) is counted and fingerprinted but never
   serialized, which is what keeps conversion-heavy proofs tractable. *)
let exptrace_note_event
    ~(scope : string) ~(name : string) ~(kind : string)
    ~(raw : exptrace_raw) ~(exception_text : string option)
    ~(elapsed : float) : unit =
  let channel = exptrace_channel_of_scope scope in
  let wrapper = exptrace_get_wrapper scope name in
  let fingerprint = exptrace_raw_fingerprint raw in
  channel.ch_total <- channel.ch_total + 1;
  channel.ch_roll <- exptrace_roll channel.ch_roll fingerprint;
  wrapper.w_total <- wrapper.w_total + 1;
  wrapper.w_roll <- exptrace_roll wrapper.w_roll fingerprint;
  if kind = "tactic_ok" || kind = "conv_ok" then begin
    channel.ch_ok <- channel.ch_ok + 1;
    wrapper.w_ok <- wrapper.w_ok + 1
  end else begin
    channel.ch_raised <- channel.ch_raised + 1;
    wrapper.w_raised <- wrapper.w_raised + 1
  end;
  exptrace_bump_kind kind;
  let is_failure = kind = "tactic_raised" || kind = "conv_raised" in
  match !exptrace_attempt with
  | Some a when not a.a_ended ->
    let seq = a.a_seq in
    a.a_seq <- a.a_seq + 1;
    wrapper.w_seen <- wrapper.w_seen + 1;
    let pending =
      exptrace_make_pending ~seq ~kind ~name ~scope ~exception_text ~elapsed
        ~raw:(Some raw)
    in
    if wrapper.w_seen <= exptrace_first_n then begin
      (* Head window: serialize at once. *)
      exptrace_serialize_pending channel pending;
      a.a_retained <- pending :: a.a_retained;
      if is_failure then exptrace_write_journal_line pending
    end else begin
      (* Tail window: hold up to exptrace_last_n, evicting the oldest.  The
         evicted event is dropped without ever being serialized. *)
      wrapper.w_tail <- pending :: wrapper.w_tail;
      exptrace_drop_oldest_tail wrapper
      (* Tail failures are journaled at finalisation, once retention is
         settled.  Nothing is written yet. *)
    end
  | _ ->
    (* No active attempt: legacy write-through for failures only.  The guards
       call this branch only for a raising application, so successes cost
       nothing. *)
    let pending =
      exptrace_make_pending ~seq:0 ~kind ~name ~scope ~exception_text ~elapsed
        ~raw:(Some raw)
    in
    exptrace_serialize_pending channel pending;
    if is_failure then exptrace_write_journal_line pending

(* ------------------------------------------------------------------------- *)
(* Stream and manifest rendering.                                            *)
(* ------------------------------------------------------------------------- *)

let exptrace_pending_line (p : exptrace_pending) : string =
  let b = Buffer.create 256 in
  Buffer.add_string b "{\"seq\":";
  Buffer.add_string b (string_of_int p.p_seq);
  Buffer.add_string b ",\"kind\":\"";
  Buffer.add_string b (exptrace_json_escape p.p_kind);
  Buffer.add_string b "\",\"name\":\"";
  Buffer.add_string b (exptrace_json_escape p.p_name);
  Buffer.add_string b "\",\"scope\":\"";
  Buffer.add_string b (exptrace_json_escape p.p_scope);
  Buffer.add_string b "\",\"goal_sha256\":\"";
  Buffer.add_string b p.p_digest;
  Buffer.add_string b "\",\"goal_bytes\":";
  Buffer.add_string b (string_of_int p.p_bytes);
  (match p.p_exception with
   | None -> ()
   | Some text ->
     Buffer.add_string b ",\"exception_text\":\"";
     Buffer.add_string b (exptrace_json_escape text);
     Buffer.add_string b "\",\"exception_text_bytes\":";
     Buffer.add_string b (string_of_int p.p_exception_bytes);
     Buffer.add_string b ",\"exception_truncated\":";
     Buffer.add_string b (if p.p_exception_truncated then "true" else "false"));
  (match p.p_term with
   | Some term ->
     Buffer.add_string b ",\"term_retained\":true,\"goal\":\"";
     Buffer.add_string b (exptrace_json_escape term);
     Buffer.add_string b "\""
   | None -> Buffer.add_string b ",\"term_retained\":false");
  Buffer.add_string b ",\"elapsed_cpu_s\":";
  Buffer.add_string b (Printf.sprintf "%g" p.p_elapsed);
  Buffer.add_string b "}\n";
  Buffer.contents b

let exptrace_attempt_start_line (a : exptrace_attempt) : string =
  Printf.sprintf
    "{\"seq\":0,\"kind\":\"attempt_start\",\"name\":\"attempt_start\",\"scope\":\"attempt\",\"goal_sha256\":null,\"goal_bytes\":0,\"elapsed_cpu_s\":0,\"attempt_id\":\"%s\",\"theorem_id\":\"%s\",\"proof_sha256\":\"%s\",\"injection\":\"%s\",\"wrapper_sha256\":\"%s\",\"module_sha256\":\"%s\"}\n"
    (exptrace_json_escape a.a_id)
    (exptrace_json_escape a.a_theorem)
    (exptrace_json_escape a.a_proof_sha)
    (exptrace_json_escape a.a_injection)
    (exptrace_json_escape a.a_wrapper_sha)
    (exptrace_json_escape a.a_module_sha)

let exptrace_kind_count (kind : string) : int =
  match Hashtbl.find_opt exptrace_kind_counts kind with
  | None -> 0
  | Some n -> n

let exptrace_attempt_end_line
    (a : exptrace_attempt) (outcome : string) (elapsed : float) : string =
  let events = max 0 (a.a_seq - 1) in
  let total_streamed =
    exptrace_tactic_channel.ch_streamed + exptrace_conv_channel.ch_streamed
  in
  let total_dropped =
    exptrace_tactic_channel.ch_dropped + exptrace_conv_channel.ch_dropped
  in
  Printf.sprintf
    "{\"seq\":%d,\"kind\":\"attempt_end\",\"name\":\"attempt_end\",\"scope\":\"attempt\",\"goal_sha256\":null,\"goal_bytes\":0,\"elapsed_cpu_s\":%g,\"outcome\":\"%s\",\"events\":%d,\"tactic_events\":%d,\"conv_events\":%d,\"tactic_ok\":%d,\"tactic_raised\":%d,\"conv_ok\":%d,\"conv_raised\":%d,\"kinds\":{\"attempt_start\":1,\"tactic_ok\":%d,\"tactic_raised\":%d,\"conv_ok\":%d,\"conv_raised\":%d},\"streamed_events\":%d,\"dropped_events\":%d,\"stream_complete\":false}\n"
    a.a_seq
    elapsed
    (exptrace_json_escape outcome)
    events
    exptrace_tactic_channel.ch_total
    exptrace_conv_channel.ch_total
    (exptrace_kind_count "tactic_ok")
    (exptrace_kind_count "tactic_raised")
    (exptrace_kind_count "conv_ok")
    (exptrace_kind_count "conv_raised")
    (exptrace_kind_count "tactic_ok")
    (exptrace_kind_count "tactic_raised")
    (exptrace_kind_count "conv_ok")
    (exptrace_kind_count "conv_raised")
    total_streamed
    total_dropped

let exptrace_rolling_digest (roll : int64) (total : int) : string =
  exptrace_sha256_hex
    ("tekmerion-roll:" ^ Int64.to_string roll ^ ":" ^ string_of_int total)

let exptrace_channel_manifest (c : exptrace_channel) : string =
  Printf.sprintf
    "{\n      \"total_events\": %d,\n      \"ok\": %d,\n      \"raised\": %d,\n      \"streamed\": %d,\n      \"dropped\": %d,\n      \"retained_terms\": %d,\n      \"dropped_terms\": %d,\n      \"term_bytes\": %d,\n      \"cap\": {\"first\": %d, \"last\": %d, \"max_term_bytes\": %d, \"retained_cap_bytes\": %d},\n      \"digest\": \"%s\",\n      \"digest_kind\": \"sha256-chain over the serialized terms of retained events\",\n      \"rolling_digest\": \"%s\",\n      \"rolling_digest_kind\": \"sha256 of a 64-bit fingerprint roll over every event (structural hash; not a term digest)\",\n      \"complete\": false\n    }"
    c.ch_total c.ch_ok c.ch_raised c.ch_streamed c.ch_dropped
    c.ch_terms_retained c.ch_terms_dropped c.ch_term_bytes
    exptrace_first_n exptrace_last_n exptrace_max_term_bytes
    exptrace_channel_retained_cap_bytes c.ch_digest
    (exptrace_rolling_digest c.ch_roll c.ch_total)

let exptrace_wrapper_manifest (w : exptrace_wrapper) : string =
  Printf.sprintf
    "    {\"scope\":\"%s\",\"name\":\"%s\",\"total\":%d,\"ok\":%d,\"raised\":%d,\"streamed\":%d,\"dropped\":%d,\"retained_terms\":%d,\"dropped_terms\":%d,\"term_bytes\":%d,\"digest\":\"%s\",\"rolling_digest\":\"%s\"}"
    (exptrace_json_escape w.w_scope)
    (exptrace_json_escape w.w_name)
    w.w_total w.w_ok w.w_raised w.w_streamed w.w_dropped
    w.w_terms_retained w.w_terms_dropped w.w_term_bytes w.w_digest
    (exptrace_rolling_digest w.w_roll w.w_total)

let exptrace_wrapped_names () : exptrace_wrapper list =
  let fold table acc =
    Hashtbl.fold (fun _ w acc -> w :: acc) table acc
  in
  let all = fold exptrace_tactic_wrappers [] in
  let all = fold exptrace_conv_wrappers all in
  List.sort (fun a b ->
      let c = String.compare a.w_scope b.w_scope in
      if c <> 0 then c else String.compare a.w_name b.w_name)
    all

(* ------------------------------------------------------------------------- *)
(* Attempt lifecycle.                                                        *)
(* ------------------------------------------------------------------------- *)

let exptrace_close_stream (a : exptrace_attempt) : unit =
  match a.a_stream with
  | None -> ()
  | Some oc ->
    a.a_stream <- None;
    (try flush oc; close_out oc with _ -> ())

let exptrace_write_manifest
    (a : exptrace_attempt) (outcome : string) (elapsed : float)
    (stream_bytes : int) : unit =
  let dir = a.a_dir in
  exptrace_ensure_dir dir;
  let path = dir ^ "/" ^ a.a_id ^ ".manifest.json" in
  let oc =
    open_out_gen [Open_wronly; Open_creat; Open_excl; Open_text] 0o666 path
  in
  let wrappers = exptrace_wrapped_names () in
  try
    Printf.fprintf oc "{\n";
    Printf.fprintf oc "  \"schema\": \"tekmerion/attempt-manifest/v1\",\n";
    Printf.fprintf oc "  \"attempt_id\": \"%s\",\n"
      (exptrace_json_escape a.a_id);
    Printf.fprintf oc "  \"theorem_id\": \"%s\",\n"
      (exptrace_json_escape a.a_theorem);
    Printf.fprintf oc "  \"proof_sha256\": \"%s\",\n"
      (exptrace_json_escape a.a_proof_sha);
    Printf.fprintf oc "  \"injection\": \"%s\",\n"
      (exptrace_json_escape a.a_injection);
    Printf.fprintf oc "  \"wrapper_sha256\": \"%s\",\n"
      (exptrace_json_escape a.a_wrapper_sha);
    Printf.fprintf oc "  \"module_sha256\": \"%s\",\n"
      (exptrace_json_escape a.a_module_sha);
    Printf.fprintf oc "  \"outcome\": \"%s\",\n"
      (exptrace_json_escape outcome);
    Printf.fprintf oc "  \"elapsed_cpu_s\": %g,\n" elapsed;
    Printf.fprintf oc "  \"stream_file\": \"%s\",\n"
      (exptrace_json_escape a.a_stream_path);
    Printf.fprintf oc "  \"stream_bytes\": %d,\n" stream_bytes;
    Printf.fprintf oc "  \"declared_budget_bytes\": %d,\n"
      exptrace_declared_budget_bytes;
    Printf.fprintf oc
      "  \"budget_note\": \"declared per-attempt budget for attempt stream plus failure journals plus manifest; the hard retention caps bound the total well below it\",\n";
    Printf.fprintf oc "  \"retention\": {\n";
    Printf.fprintf oc "    \"policy\": \"first_and_last_per_wrapper\",\n";
    Printf.fprintf oc "    \"first\": %d,\n" exptrace_first_n;
    Printf.fprintf oc "    \"last\": %d,\n" exptrace_last_n;
    Printf.fprintf oc "    \"max_term_bytes\": %d,\n"
      exptrace_max_term_bytes;
    Printf.fprintf oc "    \"channel_retained_cap_bytes\": %d,\n"
      exptrace_channel_retained_cap_bytes;
    Printf.fprintf oc "    \"stream_complete\": false,\n";
    Printf.fprintf oc
      "    \"note\": \"events outside the per-wrapper window are counted and fingerprinted but not serialized or written to the stream; retained events are serialized and digested. The stream is never a complete event log and must not be presented as one\"\n";
    Printf.fprintf oc "  },\n";
    Printf.fprintf oc "  \"channels\": {\n";
    Printf.fprintf oc "    \"tactic\": %s,\n"
      (exptrace_channel_manifest exptrace_tactic_channel);
    Printf.fprintf oc "    \"conv\": %s\n"
      (exptrace_channel_manifest exptrace_conv_channel);
    Printf.fprintf oc "  },\n";
    Printf.fprintf oc
      "  \"event_counts\": {\"events\": %d, \"tactic_ok\": %d, \"tactic_raised\": %d, \"conv_ok\": %d, \"conv_raised\": %d},\n"
      (max 0 (a.a_seq - 1))
      (exptrace_kind_count "tactic_ok")
      (exptrace_kind_count "tactic_raised")
      (exptrace_kind_count "conv_ok")
      (exptrace_kind_count "conv_raised");
    Printf.fprintf oc "  \"wrappers\": [\n";
    let n = List.length wrappers in
    List.iteri
      (fun i w ->
        Printf.fprintf oc "%s%s\n"
          (exptrace_wrapper_manifest w)
          (if i + 1 = n then "" else ","))
      wrappers;
    Printf.fprintf oc "  ]\n";
    Printf.fprintf oc "}\n";
    close_out oc
  with e ->
    close_out_noerr oc;
    (try Sys.remove path with Sys_error _ -> ());
    raise e

let exptrace_finish (outcome : string) : unit =
  match !exptrace_attempt with
  | None -> ()
  | Some a when a.a_ended -> ()
  | Some a ->
    a.a_ended <- true;
    let elapsed = Sys.time () -. a.a_cpu_started in
    (* Settle streamed/dropped accounting from the retained set.  An event is
       streamed iff it lies in the first N or the last N of its wrapper. *)
    let wrappers = exptrace_wrapped_names () in
    (* Serialize the tail events now, in order, so their digests and byte
       sizes are known before the manifest is written. *)
    List.iter
      (fun w ->
        List.iter
          (fun p ->
            exptrace_serialize_pending
              (exptrace_channel_of_scope p.p_scope) p)
          (List.rev w.w_tail))
      wrappers;
    List.iter
      (fun w ->
        w.w_streamed <- min w.w_total (exptrace_first_n + exptrace_last_n);
        w.w_dropped <- w.w_total - w.w_streamed;
        w.w_terms_dropped <- w.w_total - w.w_terms_retained)
      wrappers;
    List.iter
      (fun c ->
        let scope = c.ch_scope in
        let sum f =
          List.fold_left
            (fun acc w -> if w.w_scope = scope then acc + f w else acc)
            0 wrappers
        in
        c.ch_streamed <- sum (fun w -> w.w_streamed);
        c.ch_dropped <- c.ch_total - c.ch_streamed;
        c.ch_terms_dropped <- c.ch_total - c.ch_terms_retained)
      [ exptrace_tactic_channel; exptrace_conv_channel ];
    (* Collect the tail windows.  Every retained event, head or tail, is
       written in strictly increasing seq order so the stream is one ordered
       stream. *)
    let tail_events =
      List.concat_map (fun w -> List.rev w.w_tail) wrappers
    in
    let retained = List.rev_append a.a_retained tail_events in
    let retained = List.sort (fun x y -> compare x.p_seq y.p_seq) retained in
    let stream_bytes = ref 0 in
    (try
       (match a.a_stream with
        | None -> ()
        | Some oc ->
          List.iter
            (fun p ->
              if (p.p_kind = "tactic_raised" || p.p_kind = "conv_raised")
                 && not p.p_journal_written
              then (try exptrace_write_journal_line p with _ -> ());
              let line = exptrace_pending_line p in
              stream_bytes := !stream_bytes + String.length line;
              output_string oc line)
            retained;
          let end_line = exptrace_attempt_end_line a outcome elapsed in
          stream_bytes := !stream_bytes + String.length end_line;
          output_string oc end_line;
          flush oc)
     with e ->
       a.a_error <- Some (Printexc.to_string e);
       prerr_endline
         ("exptrace_attempt_end: could not finish the attempt stream: " ^
          Printexc.to_string e));
    exptrace_close_stream a;
    (try
       exptrace_write_manifest a outcome elapsed !stream_bytes
     with e ->
       prerr_endline
         ("exptrace_attempt_end: could not write the manifest: " ^
          Printexc.to_string e))

let exptrace_attempt_start
    (attempt_id : string) (theorem : string) (proof_sha : string)
    (injection : string) (wrapper_sha : string) (module_sha : string) : unit =
  if String.length attempt_id = 0 then
    invalid_arg "exptrace_attempt_start: empty attempt id";
  (match !exptrace_attempt with
   | Some a when not a.a_ended ->
     invalid_arg
       ("exptrace_attempt_start: an attempt is already active: " ^ a.a_id)
   | _ -> ());
  exptrace_reset_state ();
  exptrace_expected_outcome := "unknown";
  let dir = exptrace_attempt_dir () in
  exptrace_ensure_dir dir;
  let stream_path = dir ^ "/" ^ attempt_id ^ ".jsonl" in
  let oc =
    try
      open_out_gen [Open_wronly; Open_creat; Open_excl; Open_text] 0o666
        stream_path
    with Sys_error msg ->
      invalid_arg
        ("exptrace_attempt_start: cannot create attempt stream " ^
         stream_path ^ ": " ^ msg)
  in
  let a = {
    a_id = attempt_id;
    a_theorem = theorem;
    a_proof_sha = proof_sha;
    a_injection = injection;
    a_wrapper_sha = wrapper_sha;
    a_module_sha = module_sha;
    a_dir = dir;
    a_stream_path = stream_path;
    a_stream = Some oc;
    a_seq = 1;
    a_ended = false;
    a_error = None;
    a_retained = [];
    a_cpu_started = Sys.time ();
  } in
  exptrace_attempt := Some a;
  (* attempt_start has seq 0 and is flushed immediately: a process that dies
     before attempt_end still leaves evidence that the attempt began. *)
  let line = exptrace_attempt_start_line a in
  output_string oc line;
  flush oc

let exptrace_attempt_end (outcome : string) : unit =
  exptrace_finish outcome

let () =
  at_exit (fun () ->
      match !exptrace_attempt with
      | Some a when not a.a_ended -> exptrace_finish !exptrace_expected_outcome
      | _ -> ())

(* ------------------------------------------------------------------------- *)
(* Guards.                                                                   *)
(* ------------------------------------------------------------------------- *)

(* [exptrace_guard name goal_before f] has exactly the semantics of [f ()]:
   it returns the same value, or raises the same exception. Its only extra
   effect is recording. Timing uses Sys.time (CPU time); Unix is not needed.
   A recording failure warns and leaves the result or the exception alone. *)
let exptrace_guard (name : string) (goal_before : goal) (f : unit -> 'a) : 'a =
  let started = Sys.time () in
  match (try Ok (f ()) with e -> Error e) with
  | Ok value ->
    if exptrace_attempt_active () then begin
      let elapsed = Sys.time () -. started in
      (try
         exptrace_note_event ~scope:exptrace_failure_scope ~name
           ~kind:"tactic_ok" ~raw:(RawGoal goal_before)
           ~exception_text:None ~elapsed
       with record_error ->
         prerr_endline
           ("exptrace_guard: could not record success for " ^ name ^ ": " ^
            Printexc.to_string record_error))
    end;
    value
  | Error e ->
    if exptrace_attempt_active () || exptrace_legacy_mode () then begin
      let elapsed = Sys.time () -. started in
      (try
         exptrace_note_event ~scope:exptrace_failure_scope ~name
           ~kind:"tactic_raised" ~raw:(RawGoal goal_before)
           ~exception_text:(Some (Printexc.to_string e)) ~elapsed
       with record_error ->
         prerr_endline
           ("exptrace_guard: could not record failure for " ^ name ^ ": " ^
            Printexc.to_string record_error))
    end;
    raise e

(* The conversion counterpart. The input is a thunk: it is never forced before
   the conversion runs. It is forced on failure for the failure record, and on
   success only when an attempt is active (after the conversion returned) so
   the success event can carry a digest and byte size. See the header note. *)
let exptrace_guard_conv
    (name : string) (input : unit -> term) (f : unit -> 'a) : 'a =
  let started = Sys.time () in
  match (try Ok (f ()) with e -> Error e) with
  | Ok value ->
    let elapsed = Sys.time () -. started in
    if exptrace_attempt_active () then begin
      let raw =
        match (try Some (input ()) with _ -> None) with
        | None -> RawAbsent
        | Some t -> RawTerm t
      in
      (try
         exptrace_note_event ~scope:exptrace_conv_failure_scope ~name
           ~kind:"conv_ok" ~raw ~exception_text:None ~elapsed
       with record_error ->
         prerr_endline
           ("exptrace_guard_conv: could not record success for " ^ name ^
            ": " ^ Printexc.to_string record_error))
    end;
    value
  | Error e ->
    if exptrace_attempt_active () || exptrace_legacy_mode () then begin
      let elapsed = Sys.time () -. started in
      let raw =
        match (try Some (input ()) with _ -> None) with
        | None -> RawAbsent
        | Some t -> RawTerm t
      in
      (try
         exptrace_note_event ~scope:exptrace_conv_failure_scope ~name
           ~kind:"conv_raised" ~raw
           ~exception_text:(Some (Printexc.to_string e)) ~elapsed
       with record_error ->
         prerr_endline
           ("exptrace_guard_conv: could not record failure for " ^ name ^
            ": " ^ Printexc.to_string record_error))
    end;
    raise e

(* Legacy entry points kept for compatibility with the offline harness. *)
let exptrace_record_failure
    (name : string) (goal_before : goal)
    (exception_text : string) (elapsed_cpu_s : float) : unit =
  exptrace_note_event ~scope:exptrace_failure_scope ~name
    ~kind:"tactic_raised" ~raw:(RawGoal goal_before)
    ~exception_text:(Some exception_text) ~elapsed:elapsed_cpu_s

let exptrace_record_conv_failure
    (name : string) (input : term)
    (exception_text : string) (elapsed_cpu_s : float) : unit =
  exptrace_note_event ~scope:exptrace_conv_failure_scope ~name
    ~kind:"conv_raised" ~raw:(RawTerm input)
    ~exception_text:(Some exception_text) ~elapsed:elapsed_cpu_s

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
    (try Sys.remove path with Sys_error _ -> ());
    raise e

let exptrace_failures_dump (dir : string) : unit =
  if Hashtbl.length exptrace_failure_counts = 0 &&
     Hashtbl.length exptrace_conv_failure_counts = 0 then () else begin
    if Sys.file_exists dir && not (Sys.is_directory dir) then
      invalid_arg ("Failure trace directory is not a directory: " ^ dir);
    let tactic_jobs =
      List.map (fun name ->
          let journal = dir ^ "/" ^ name ^ ".failures.jsonl" in
          let consolidated = dir ^ "/" ^ name ^ ".failures.json" in
          let expected =
            match Hashtbl.find_opt exptrace_failure_counts name with
            | None -> 0
            | Some n -> n
          in
          name, journal, consolidated, expected)
        (List.sort compare
           (Hashtbl.fold
              (fun name _ acc -> name :: acc)
              exptrace_failure_counts []))
    in
    let conv_jobs =
      List.map (fun name ->
          let journal = dir ^ "/" ^ name ^ ".conv.failures.jsonl" in
          let consolidated = dir ^ "/" ^ name ^ ".conv.failures.json" in
          let expected =
            match Hashtbl.find_opt exptrace_conv_failure_counts name with
            | None -> 0
            | Some n -> n
          in
          name, journal, consolidated, expected)
        (List.sort compare
           (Hashtbl.fold
              (fun name _ acc -> name :: acc)
              exptrace_conv_failure_counts []))
    in
    let jobs = tactic_jobs @ conv_jobs in
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
