(* Driver for tests/bridge_failure_selftest.sh (XVI.D3).
 *
 * The shell script prepends the text between BEGIN/END
 * BRIDGE_FAILURE_STANDALONE in tools/bridge.ml, so these assertions exercise
 * the shipped classifier and retry policy verbatim outside HOL Light. One case
 * per class: closed, mathematical, unsupported, resource exhausted (memory,
 * stack, timeout, signal), and internal. The resource cases assert that the
 * fallback counter stays at zero: a resource-exhausted attempt must never
 * retry the larger full hypothesis set. *)

let failures = ref 0

let check name cond =
  if cond then Printf.printf "OK %s\n%!" name
  else begin
    Printf.printf "FAIL %s\n%!" name;
    incr failures
  end

let notes = ref []

let note s = notes := s :: !notes

let reset () =
  notes := [];
  bridge_fallback_count := 0

let has_note s = List.exists (fun x -> x = s) !notes

let retry ~attempt ~fallback =
  bridge_slice_retry ~task:"t" ~kept:3 ~total:5
    ~attempt ~fallback ~note

(* Run a failing attempt through the shipped retry policy. [expect] receives the
   re-raised exception; the fallback must not run and the counter must stay 0. *)
let expect_raises name expect e =
  let fell = ref false in
  let matched = ref false in
  (try
     ignore
       (retry ~attempt:(fun () -> raise e)
          ~fallback:(fun () -> fell := true; 0))
   with e' -> matched := expect e');
  check (name ^ ": re-raised") !matched;
  check (name ^ ": no fallback") (not !fell);
  check (name ^ ": counter zero") (!bridge_fallback_count = 0)

(* --- closed -------------------------------------------------------------- *)
let () =
  reset ();
  let fell = ref false in
  let v = retry ~attempt:(fun () -> 7) ~fallback:(fun () -> fell := true; 42) in
  check "closed: result" (v = 7);
  check "closed: no fallback" (not !fell);
  check "closed: counter zero" (!bridge_fallback_count = 0);
  check "closed: no note" (!notes = [])

(* --- mathematical -------------------------------------------------------- *)
let () =
  reset ();
  let fell = ref false in
  let v =
    retry
      ~attempt:(fun () ->
        raise (Failure "CSAT: cadical did not report UNSAT, exit 10"))
      ~fallback:(fun () -> fell := true; 42) in
  check "mathematical: fallback ran" !fell;
  check "mathematical: result is the fallback's" (v = 42);
  check "mathematical: counter one" (!bridge_fallback_count = 1);
  check "mathematical: note"
    (has_note "SLICE fallback=t reason=mathematical kept=3/5")

(* A slice-too-strong report is the second allow-listed mathematical shape. *)
let () =
  reset ();
  let fell = ref false in
  let v =
    retry
      ~attempt:(fun () -> raise (Failure "SLICE_TOO_STRONG: kept 3/5"))
      ~fallback:(fun () -> fell := true; 43) in
  check "mathematical-slice: fallback ran" !fell;
  check "mathematical-slice: result is the fallback's" (v = 43);
  check "mathematical-slice: counter one" (!bridge_fallback_count = 1);
  check "mathematical-slice: note"
    (has_note "SLICE fallback=t reason=mathematical kept=3/5")

(* A generic Failure is NOT mathematical. It can equally be a broken rewrite,
   a missing lemma, or a violated invariant. Only the explicit allow-list may
   retry; everything else is internal and propagates. *)
let () =
  reset ();
  expect_raises "internal-generic"
    (function Failure s -> s = "boom" | _ -> false)
    (Failure "boom");
  check "internal-generic: note"
    (has_note "CSAT internal=unrecognised Failure: boom")

(* --- unsupported input --------------------------------------------------- *)
let () =
  reset ();
  expect_raises "unsupported"
    (function Invalid_argument s -> s = "no word type" | _ -> false)
    (Invalid_argument "no word type");
  check "unsupported: note"
    (has_note "CSAT unsupported=no word type kept=3/5")

(* --- resource exhausted: memory ----------------------------------------- *)
let () =
  reset ();
  expect_raises "resource-memory"
    (function Out_of_memory -> true | _ -> false)
    Out_of_memory;
  check "resource-memory: note"
    (has_note "CSAT capped=memory limit=address space (RLIMIT_AS) kept=3/5")

(* --- resource exhausted: stack overflow (the IX.70 death) ---------------- *)
let () =
  reset ();
  expect_raises "resource-stack"
    (function Stack_overflow -> true | _ -> false)
    Stack_overflow;
  check "resource-stack: note"
    (has_note "CSAT capped=stack limit=OCaml stack limit kept=3/5")

(* --- resource exhausted: timeout and signal death ------------------------ *)
let () =
  check "classify timeout flag"
    (bridge_failure_class ~timeout:true (Failure "x") =
     ("resource", "timeout", "wall-clock budget"));
  check "classify exit 124"
    (bridge_failure_class (Failure "CSAT: cadical did not report UNSAT, exit 124") =
     ("resource", "timeout", "exit 124"));
  check "classify signal 137"
    (bridge_failure_class (Failure "CSAT: cadical did not report UNSAT, exit 137") =
     ("resource", "signal", "exit 137"));
  reset ();
  expect_raises "resource-signal"
    (function Failure _ -> true | _ -> false)
    (Failure "CSAT: cadical did not report UNSAT, exit 137");
  check "resource-signal: note"
    (has_note "CSAT capped=signal limit=exit 137 kept=3/5")

(* --- internal error ------------------------------------------------------ *)
let () =
  check "classify generic Failure is internal"
    (match bridge_failure_class (Failure "boom") with
     | ("internal", _, _) -> true
     | _ -> false);
  check "classify allow-listed exit 10 is mathematical"
    (bridge_failure_class
       (Failure "CSAT: cadical did not report UNSAT, exit 10") =
     ("mathematical", "CSAT: cadical did not report UNSAT, exit 10", ""));
  check "classify SATISFIABLE is mathematical"
    (match bridge_failure_class (Failure "solver: SATISFIABLE") with
     | ("mathematical", _, _) -> true
     | _ -> false);
  check "classify slice-too-strong is mathematical"
    (match bridge_failure_class (Failure "SLICE_TOO_STRONG: x") with
     | ("mathematical", _, _) -> true
     | _ -> false)

let () =
  reset ();
  expect_raises "internal"
    (function Failure s -> s = "CSAT_PROVE: Sanity check" | _ -> false)
    (Failure "CSAT_PROVE: Sanity check");
  check "internal: note" (has_note "CSAT internal=CSAT_PROVE: Sanity check")

let () =
  reset ();
  expect_raises "internal-unknown"
    (function Not_found -> true | _ -> false)
    Not_found;
  check "internal-unknown: note" (has_note "CSAT internal=Not_found")

(* --- summary ------------------------------------------------------------- *)
let () =
  if !failures = 0 then begin
    print_string "OK bridge_failure_taxonomy\n";
    flush stdout;
    exit 0
  end else begin
    Printf.printf "FAIL bridge_failure_taxonomy: %d check(s)\n%!" !failures;
    exit 1
  end
