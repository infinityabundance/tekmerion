# Instrument specification — failure-side trajectory capture

The programme's thesis is that failures carry the learning signal. The best existing
instrument, `TacticTrace`, records only successes: its README states that "the proof ends
with an explicit `exptrace_dump ()`, so a proof that fails writes no traces." This document
specifies the smallest correct extension, and — more importantly — what it cannot capture.

Non-negotiable: this instrument is **untrusted**. It produces observations, not theorems.
Nothing it emits may be counted as a proof, a validity claim, or a label of truth.

---

## 1. What `TacticTrace` already gives us

Its generated tactic wrapper (from `tracer.ml`, `make-wrappers`) has this shape:

```ocaml
let <tname> : tactic =
  fun (trace_asl, trace_g) ->
    let trace_goal_before = (trace_asl, trace_g) in
    let (trace_ga_inst, trace_goals_after, trace_justf) = <original> args (trace_asl, trace_g) in
    exptrace_add_tac "<tname>" {
      definition_line_number = (...);
      user_line_number = (...);
      goal_before = trace_goal_before;
      goals_after = trace_goals_after;
      num_subgoals = List.length trace_goals_after;
    } (fun () -> { ... });
    (trace_ga_inst, trace_goals_after, trace_justf)
```

The hook point is exact and the change is small: the original application is the only
expression that can raise.

## 2. The extension

Wrap the application, record a failure record, re-raise unchanged:

```ocaml
    let trace_outcome =
      try Ok (<original> args (trace_asl, trace_g))
      with e -> Error e
    in
    match trace_outcome with
    | Ok (trace_ga_inst, trace_goals_after, trace_justf) ->
        exptrace_add_tac "<tname>" { (* unchanged *) };
        (trace_ga_inst, trace_goals_after, trace_justf)
    | Error e ->
        exptrace_add_tac_failure "<tname>" {
          definition_line_number = (...);
          user_line_number = (...);
          goal_before = trace_goal_before;
          exception_text = Printexc.to_string e;
          subgoal_index = Unknown;        (* see §4 *)
          elapsed_ns = <measured>;
        };
        raise e
```

Two rules:

1. **Never swallow.** `raise e` preserves HOL Light's semantics exactly. A verifier that
   changes the behaviour it observes is measuring itself.
2. **Never sample the trajectory streams.** `TacticTrace` keeps at most 20 records per tactic
   across five subgoal buckets and prefers the shortest goals. That is acceptable for browsing
   successes and unacceptable for any statistic: it biases both failure rates and forecast
   error. The C1 stream keeps every success and every failure for a given attempt, in order.

## 3. Record schema

A strict superset of the `TacticTrace` record. `success` records must serialise identically
to `TacticTrace`'s, which is the conformance test.

```json
{
  "schema": "tekmerion/trace/v1",
  "task_id": "arm.bignum_montmul_p256.BIGNUM_MONTMUL_P256_SUBROUTINE_CORRECT",
  "attempt_id": "sha256:...",
  "step": 41,
  "outcome": "failure",
  "tactic": "REWRITE_TAC",
  "site": { "file": "hol_lib_inlined.ml", "line": 1234, "user_line": 57 },
  "goal_before": { "hyps": ["..."], "concl": "..." },
  "goals_after": [],
  "num_subgoals": 0,
  "subgoal_index": null,
  "exception": { "text": "Failure(\"...\")" },
  "elapsed_ns": 1830021
}
```

Fields and their status:

| Field | Type | Status |
|---|---|---|
| `goal_before` | goal | **exact** (from the wrapper) |
| `tactic`, `site` | string / location | **exact** |
| `goals_after` | goal list | **exact** on success; `[]` on failure |
| `num_subgoals` | int | **exact** |
| `exception.text` | string | **exact text, lossy structure** |
| `elapsed_ns` | int | measured, not exact in a virtualised environment |
| `subgoal_index` | int or null | **unknown** unless combinators are instrumented (§4) |

Anything not in the "exact" column must be treated as `Structured + Unknown`. In
particular, do not infer a subgoal index from goal equality: two subgoals can be syntactically
identical.

## 4. What this instrument cannot see — state it, do not paper over it

- **Subgoal index.** `THEN`/`THENL`/`EVERY` apply a tactic to goals chosen by the
  combinator. The wrapper sees one goal, not its position in the parent proof state. Getting
  the index requires instrumenting the combinators, which are ordinary HOL Light definitions
  and therefore also wrappable — but the interaction with `TacticTrace`'s AST rewriting must
  be verified, not assumed.
- **Why the tactic failed.** The exception text is `Failure "..."` from wherever the
  underlying primitive gave up. It is not a structured reason. Do not parse it into a cause;
  that is the residual analyst's *interpretation*, and interpretations are not ground truth.
- **R4 (certificate near-misses).** Not visible at the tactic layer. SAT/SMT certificate
  progress needs the certificate engines' own instrumentation.
- **R5 (cross-engine disagreement).** Requires two proposers on the same step. It is a
  property of the *attempt set*, computable after capture, not during.
- **Rejections by the kernel as opposed to the tactic layer.** A kernel failure surfaces as
  an exception in the same way, but distinguishing "the rule rejected this" from "the tactic
  could not find an instantiation" needs the rule name; the exception text often has it, and
  when it does not, it stays unknown.

This is the point of the exercise: the instrument's honest scope is narrower than the
brief's R1–R6 table implies, and the gap is where the research actually is.

## 5. Channels, computed from what is captured

| Channel | Computation | Honesty note |
|---|---|---|
| R1 goal complexity | `term_size(goal_before)`, `num_subgoals`, open-metavariable count where available | metavariable count is not exposed by HOL Light's tactic interface; report as unknown unless measured |
| R2 rejection structure | `exception.text` plus tactic name and site | categorical; do **not** embed as a continuous quantity |
| R3 forecast | structural distance between predicted and observed `goals_after` | only available on success records; failure records have no observed result to compare against |
| R6 replay | replay the same step against a changed definition or model; compare `goal_before` and outcome | exact when deterministic |

R3 needs one correction before it is usable. `goals_after` is recorded on every success, so
forecast residuals are computable without new instrumentation — but not from stock traces.
`TacticTrace` keeps at most 20 records per tactic, prefers the shortest printed goals, and
records no step order. Forecast error computed on that sample is biased downward: the easiest
goals to predict are the ones most likely to be retained. E4 must therefore consume C1's
unsampled stream, and any figure computed from stock traces must be labelled as a lower bound
on difficulty, not as a result.

## 6. Conformance test (the gate for E1)

1. Run a proof file with no failures through stock `TacticTrace` and through the extended
   wrapper.
2. Normalise both outputs to the same schema.
3. Assert **byte equality** on `goal_before`, `goals_after`, `num_subgoals`, `tactic`, and
   `site` for every record.
4. Report the diff count. It must be 0. A non-zero diff means our wrapper changed HOL
   Light's behaviour or its recorded view of it.

Test with at least three proof files of increasing size, including one that uses
conversions as well as tactics, since the conversion path is a separate code generator.

## 7. Storage and custody

- One directory per `(task_id, attempt_id)`; step files are append-only.
- Content-address each attempt by the digest of its raw trace plus the input query digest.
  Never rewrite a captured trace (invariant 4). A corrected interpretation is a new object
  that references the old one.
- Record the digests of HOL Light, `TacticTrace`, the model, and the sampling configuration
  alongside each attempt. A trajectory without its producer digest is not evidence.

## 8. What would falsify this instrument

- A conformance diff (it changed behaviour).
- A replay that does not reproduce the recorded exception for the same input.
- A sampled failure set whose distribution over tactics differs from the unsampled set
  (means the sampling is biased in a way that matters).

Report all three, every time.

## 9. Scaffold: files and commands

This section records the scaffold for the failure-side extension. It is a scaffold, not a
validated instrument: no HOL Light or s2n-bignum build was performed. Everything below runs
offline. The OCaml module is syntax-checked and compiled against a small stub harness that
exercises the write-through path; it is not type-checked against HOL Light's real `goal`,
`pp_print_goal`, or `ExportTrace`.

**Scope of a failure record.** A record means "this tactic application raised at least
once". It does **not** mean "the proof failed". The wrapper sees one application and cannot
see whether a combinator or `TRY`/`ORELSE` recovered, nor what the attempt's terminal
outcome was. Downstream labelling must combine these records with that outcome, which this
module does not observe. Every record carries `"scope": "tactic"` to make the limit
unmistakable.

### Files

| File | Purpose |
|---|---|
| `trace/exptrace_failure.ml` | `tac_failure` record with `scope = "tactic"`, `exptrace_guard`, and optional `exptrace_failures_dump ()`. `exptrace_guard` writes one JSON object per line to `<dir>/<tacname>.failures.jsonl` at record time, closes it, and then re-raises unchanged. Only a per-tactic counter is held in memory. The dump only consolidates journals into arrays. Loaded inside HOL Light after `exportTrace.ml`; dependency-free (no Yojson); uses the standard library plus HOL Light's `goal`/`pp_print_goal` and TacticTrace's `ExportTrace` for directory selection. |
| `tools/patch_wrapper.py` | Rewrites only the application line of each generated tactic wrapper to `exptrace_guard "<tacname>" trace_goal_before (fun () -> <application> (trace_asl,trace_g))`. Leaves the `exptrace_add_tac` emission byte-for-byte intact. Idempotent; fails closed if no tactic wrapper or no tactic name is found; counts conversions left untouched. |
| `tools/test_write_through.py` | Compiles the real module against stubs for `goal`, `pp_print_goal`, and `ExportTrace`; records one failure; calls no dump; asserts `<tacname>.failures.jsonl` already contains exactly one valid JSON object with the expected fields and `"scope": "tactic"`. Also exercises consolidation and its overwrite refusal. |
| `tools/trace_conformance.py` | Aligns stock and patched success records by `(tactic, ordinal within file)` and asserts equality on `tactic`, `definition_line_number`, `user_line_number`, `goal_before`, `goals_after`, `num_subgoals`, `arg_names`, `arg_types`, `arg_values`, `arg_exprs`. Normalises JSON key order and whitespace only; values are compared exactly. Prints a per-field diff and exits non-zero on any difference. Skips conversion traces, which are out of scope. |
| `trace/fixtures/kernel_wrapper_before.ml` | Faithful generated wrapper excerpt: two tactic wrappers (zero-arg and one-arg) plus one conversion wrapper. |
| `trace/fixtures/kernel_wrapper_after.ml` | Expected patched output. The conversion wrapper is unchanged. |
| `trace/fixtures/conformance/stock/REWRITE_TAC.json` | Stock success records used by the conformance self-test. |
| `trace/fixtures/conformance/patched/REWRITE_TAC.json` | Identical content, standing in for the patched run. |
| `trace/fixtures/conformance/diverged/REWRITE_TAC.json` | Same content with one `num_subgoals` value changed, so the self-test must fail and name that field. |
| `tools/selftest.sh` | Runs the OCaml syntax check, the write-through stub test, both Python self-tests, `cargo test --offline`, and `tools/mutate.py`. Exits non-zero on any failure. Uses a temporary directory for OCaml artifacts. |

### Exact integration sequence with TacticTrace

1. Generate the stock wrapper file exactly as before:

   ```sh
   tracer make-wrappers proof_inlined.ml proof.ast proof.mli kernel_wrapper.ml
   ```

2. Patch the tactic application expressions. The `exptrace_add_tac` blocks are not touched:

   ```sh
   python3 tools/patch_wrapper.py kernel_wrapper.ml kernel_wrapper.patched.ml
   ```

   The patched form of each application is:

   ```ocaml
       let (trace_ga_inst,trace_goals_after,trace_justf) =
         exptrace_guard "<tacname>" trace_goal_before (fun () -> <application> (trace_asl,trace_g)) in
   ```

3. Load the OCaml modules inside HOL Light in this order. `exptrace_failure.ml` uses
   `ExportTrace.effective_output_dir`, so `exportTrace.ml` must already be loaded:

   ```ocaml
   loadt "exportTrace.ml";;
   loadt "exptrace_failure.ml";;
   ```

4. Choose the run directory before the proof. This is the same directory TacticTrace uses,
   and it must not already exist:

   ```ocaml
   exptrace_set_default_dir "/abs/path/to/run-001";;
   ```

5. No failure-path driver hook is required. When a guarded application raises,
   `exptrace_guard` appends one JSON object to `<dir>/<tacname>.failures.jsonl`, closes the
   file, and re-raises unchanged. The journal is on disk before the exception can unwind out
   of the process; a failed proof leaves it there even if nothing else runs.

6. On a **successful** proof, emit the success channel as before:

   ```ocaml
   exptrace_dump ();;
   ```

   The failure journals are already present. No failure-side call is needed to keep the
   evidence.

7. **Optional** consolidation into arrays in TacticTrace's dump style:

   ```ocaml
   exptrace_failures_dump ();;
   ```

   This merges each `<tacname>.failures.jsonl` into `<tacname>.failures.json`, sorted
   deterministically by canonical journal line, and refuses to overwrite an existing array.
   It is safe never to call it; the `.jsonl` journals remain the raw evidence. On a failed
   proof, if arrays are wanted, call it in the exception path before re-raising:

   ```ocaml
   (try run_proof ()
    with e ->
      exptrace_failures_dump ();
      raise e);;
   ```

   Do not call `exptrace_dump ()` on this path; it is the success channel and its existence
   check is for a fresh run directory.

8. Check conformance between a stock run and a patched run over the same success-only proof
   file:

   ```sh
   python3 tools/trace_conformance.py stock_dir patched_dir
   ```

9. Run the offline scaffold checks:

   ```sh
   bash tools/selftest.sh
   ```

### Limitations of this scaffold

- **Conversions are not guarded in this version.** `patch_wrapper.py` counts conversion
  wrappers and leaves them untouched. A conversion that raises is still invisible to the
  failure channel. The conversion path is a separate generator branch in `tracer.ml`, with a
  different wrapper shape (`: thm`, `exptrace_add_conv`), and needs its own guard and its own
  fixture. The conformance tool skips conversion traces, so extending it later is a separate
  change rather than a modification of the success path.
- **The OCaml module is compiled against stubs, not against HOL Light.**
  `tools/test_write_through.py` proves the write-through and JSONL shape with `type goal =
  string`, a stub `pp_print_goal`, and a stub `ExportTrace`. It does not resolve HOL Light's
  real `goal`/`pp_print_goal`/`ExportTrace`, the generated wrapper's identifiers, or
  conversion behaviour. Those require a HOL Light run, which this scaffold deliberately does
  not perform (HOL Light is roughly 20 GB and an hour to build).
- **Write-through survives process exit, not necessarily machine loss.** The journal line is
  flushed by `close_out` before the exception is re-raised, so an ordinary exit preserves it.
  There is no `fsync`, because that would require the Unix library; a power loss or kernel
  crash can still lose the tail of the journal.
