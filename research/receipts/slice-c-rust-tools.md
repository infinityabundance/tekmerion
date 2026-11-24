# Slice C — remaining `tools/` Python removed (receipt)

Status: complete. Every live tool under `tools/` is now Rust; every superseded or
legacy Python file is deleted. No commit was made; the orchestrator owns git.

Scope honoured: no change to `src/lib.rs`, `src/wire.rs`, `src/native/**`,
`Cargo.toml`, or `research/**` other than this receipt. No new dependency was
added. All builds and tests used `--offline`.

## Classification

| File | Classification | Replacement / reason |
|---|---|---|
| `tools/attempt_id.py` | **live** — invoked by `tools/run_traces.sh` (line 240) and `tools/selftest.sh` §2b | `src/tools/attempt_id.rs` + `src/bin/attempt_id.rs`; `--self-test` preserved; tests in `tests/tools_attempt.rs` |
| `tools/attempt_lifecycle.py` | **live** — invoked by `tools/run_traces.sh` (line 326) and `tools/selftest.sh` §2c | `src/tools/attempt_lifecycle.rs` + `src/bin/attempt_lifecycle.rs`; `--self-test` preserved; tests in `tests/tools_attempt.rs` |
| `tools/run_traces_inject.py` | **live** — invoked by `tools/run_traces.sh` (line 221) and syntax-checked by `tools/selftest.sh` §2 | `src/tools/run_traces_inject.rs` + `src/bin/run_traces_inject.rs`; a `--self-test` was added so §2 exercises the binary; tests in `tests/tools_attempt.rs` |
| `tools/patch_wrapper.py` | **live** — invoked by `tools/run_traces.sh` (line 338) and `tools/selftest.sh` §3 | `src/tools/patch_wrapper.rs` + `src/bin/patch_wrapper.rs`; golden fixture and refusals in `tests/tools_patch_wrapper.rs` |
| `tools/trace_conformance.py` | **live** — invoked by `tools/selftest.sh` §4 and documented in `README.md` | `src/tools/trace_conformance.rs` + `src/bin/trace_conformance.rs`; fixture pair and refusals in `tests/tools_trace_conformance.rs` |
| `tools/test_write_through.py` | **live** — invoked by `tools/selftest.sh` §5, named by CI and `README.md` | `tests/write_through.rs`. It is **not** an oracle for the Rust evidence wire format (`src/wire.rs`); it is the independent harness for the still-live OCaml module `trace/exptrace_failure.ml`. The port compiles the real module against the same stubs and keeps the independent SHA-256 and JSON checks (against the crate hasher and the strict JSON reader). |
| `tools/fcdist.py` | **live** — the command behind `research/receipts/v3-recount-5e0fc782.md` and named in `TODO.md:424` | `src/tools/fcdist.rs` + `src/bin/fcdist.rs`; the Python output at the pinned checkout is recorded in `tests/fixtures/fcdist_report.{txt,json}` and compared byte-for-byte by `tests/tools_fcdist.rs` |
| `tools/emit_proof.py` | **superseded + broken** — imports `heldout_guard`, `disasm_steps`, `w2_witnesses` (deleted in Slice B) | Superseded as the answer emitter by `src/tools/emit.rs` / `src/bin/emit.rs` and `src/bin/emit_x86.rs` (`--mode answer`). No tracked script invoked it. |
| `tools/w1_gate.py` | **legacy + broken** — imports `heldout_guard`, `derive_steps`, `disasm_steps` (deleted in Slice B) | Its derivation basis is now `src/tools/derive.rs` + `src/tools/disasm.rs`, with admission in `src/tools/guard.rs`. The captured measurement remains in `research/receipts/w1-accsteps-gate.{md,json}`; the HOL driver `tools/w1_hol_capture.ml` is retained but has no driver. No tracked script invoked it. |
| `tools/w2_witnesses.py` | **legacy + broken** — imports `heldout_guard`, `derive_steps`, `disasm_steps`; only entry point was `emit_proof.py` | Part of the retired Python witness pipeline; step derivation now lives in `src/tools/derive.rs`/`disasm.rs`. No tracked script invoked it. |
| `tools/witness.py` | **legacy** — no entry point apart from `emit_proof.py`/`w4_close.py` | Retired witness schema. No tracked script invoked it. |
| `tools/w4_close.py` | **legacy** — imported only by `emit_proof.py` and (in prose) `csat_close.py` | Retired congruence closer. No tracked script invoked it. |
| `tools/csat_close.py` | **legacy + broken** — imports the three Slice B modules, and needs `cadical` + HOL Light | A measurement driver whose recorded negative result stays in `research/receipts/csat-close.{md,json}`. No tracked script invoked it. |

The OCaml files under `tools/` (`bridge.ml`, `csat_*.ml`, `w1_hol_capture.ml`,
`hol_tests/*.ml`) and `tools/run_traces.sh`/`selftest.sh` are not Python and were
left in place.

## Behaviour identity of the live ports

Each comparison was run before the Python files were deleted. "IDENTICAL" means
`cmp`/`diff` exit 0 on the full files.

```
$ python3 tools/attempt_id.py --theorem-id arm/proofs/bignum_add_p256.ml \
    --proof .bench/s2n-bignum/arm/proofs/bignum_add_p256.ml \
    --inject 'TRY (FAIL_TAC "probe")' --wrapper-sha aa --module-sha bb
6fc735ae68cde65c535fbb3ee5fc3724e27340c907546d591b37666b2bbaf48f
$ ./target/debug/attempt_id ...same arguments...
6fc735ae68cde65c535fbb3ee5fc3724e27340c907546d591b37666b2bbaf48f
ATTEMPT-ID-IDENTICAL

$ python3 tools/attempt_lifecycle.py /tmp/tmpl.ml /tmp/lc-py.ml
$ ./target/debug/attempt_lifecycle /tmp/tmpl.ml /tmp/lc-rs.ml
LIFECYCLE-IDENTICAL

$ python3 tools/run_traces_inject.py .bench/s2n-bignum/arm/proofs/bignum_add_p256.ml \
    /tmp/inj-py.ml 'TRY (FAIL_TAC "probe")'
$ ./target/debug/run_traces_inject .bench/s2n-bignum/arm/proofs/bignum_add_p256.ml \
    /tmp/inj-rs.ml 'TRY (FAIL_TAC "probe")'
INJECT-IDENTICAL

$ python3 tools/patch_wrapper.py trace/fixtures/kernel_wrapper_before.ml /tmp/pw-py.ml
$ ./target/debug/patch_wrapper trace/fixtures/kernel_wrapper_before.ml /tmp/pw-rs.ml
PATCH-IDENTICAL
```

`patch_wrapper --self-test` and `trace_conformance --self-test` print byte-identical
transcripts; the golden fixture output and both conformance reports are identical:

```
PATCH-SELFTEST-IDENTICAL
CONFORMANCE-SELFTEST-IDENTICAL
CONFORMANCE-IDENTICAL
DIVERGED-IDENTICAL        # diverging pair, rc=1 on both sides
```

`fcdist` at the pinned checkout (`5e0fc782…`) is byte-identical in both output
modes, including the corpus cross-check:

```
$ python3 tools/fcdist.py .bench/s2n-bignum --corpus .bench/problems.json
$ ./target/debug/fcdist .bench/s2n-bignum --corpus .bench/problems.json
HUMAN-IDENTICAL

$ python3 tools/fcdist.py .bench/s2n-bignum --corpus .bench/problems.json --json
$ ./target/debug/fcdist .bench/s2n-bignum --corpus .bench/problems.json --json
JSON-IDENTICAL

$ ... without --corpus ...
NO-CORPUS-IDENTICAL
```

The ported self-tests, run after deletion:

```
$ ./target/debug/attempt_id --self-test
  ok: id is deterministic, 64 lowercase hex characters
  ok: changing theorem_id changes the id
  ok: changing proof_text changes the id
  ok: changing injection changes the id
  ok: changing wrapper changes the id
  ok: changing module changes the id
  ok: length framing prevents component-boundary collisions
SELF-TEST PASSED

$ ./target/debug/attempt_lifecycle --self-test
  ok: sentinels expand to the lifecycle calls
  ok: missing sentinels are refused: expected exactly one of each sentinel; missing or duplicated: (* TEKMERION_ATTEMPT_START *), (* TEKMERION_ATTEMPT_TIMEOUT *), (* TEKMERION_ATTEMPT_END *)
  ok: duplicated sentinels are refused: expected exactly one of each sentinel; missing or duplicated: (* TEKMERION_ATTEMPT_START *)
SELF-TEST PASSED

$ ./target/debug/run_traces_inject --self-test
  ok: the tactic is inserted at the first proof step
  ok: a multi-line tactic is refused: the injected tactic must be a single line
  ok: an empty tactic is refused: the injected tactic is empty
  ok: input without `prove` is refused: no `prove` found
  ok: a proof with no tactic after its goal is refused: the first proof has no tactic after its goal
SELF-TEST PASSED

$ ./target/debug/patch_wrapper --self-test
  ok: golden fixture matches kernel_wrapper_after.ml (tactics patched 2, conversions patched 1)
  ok: idempotent (tactics patched 0, already patched 2; conversions patched 0, already patched 1)
  ok: conversion-only input is patched
  ok: missing exptrace_add_tac name is refused: line 1: cannot find the tactic name on line 2 (expected `    exptrace_add_tac "<tacname>" {`)
  ok: missing exptrace_add_conv name is refused: line 1: cannot find the conversion name on line 2 (expected `  exptrace_add_conv "<convname>" {`)
  ok: missing conversion input is refused: line 3: cannot find the conversion input term (expected `    input = <term>;`)
  ok: unrecognised input is refused: no tactic or conversion wrapper matched; this does not look like a generated kernel_wrapper.ml
  ok: patch_file round-trip
SELF-TEST PASSED

$ ./target/debug/trace_conformance --self-test
matching fixture pair (must pass):
checked 2 aligned record pair(s); 0 difference(s)
CONFORMANCE PASSED
  ok: matching pair passed
diverging fixture pair (must fail and name num_subgoals):
  DIFF REWRITE_TAC[1] field 'num_subgoals' (num_subgoals): stock=2 patched=3 [REWRITE_TAC.json vs REWRITE_TAC.json]
  ok: detected 1 difference(s) across 2 pair(s), including num_subgoals
SELF-TEST PASSED
```

`fcdist` had no `--self-test` in Python; its differential check is the fixture
test above.

## Deletions and grep evidence

After `tools/selftest.sh` and `tools/run_traces.sh` were pointed at the Rust
binaries, `git grep` (tracked files only; `.bench/` and `target/` excluded) finds
no invocation of any deleted file. The commands and their complete output:

```
$ for f in attempt_id attempt_lifecycle csat_close emit_proof fcdist patch_wrapper \
    run_traces_inject test_write_through trace_conformance w1_gate w2_witnesses \
    w4_close witness; do
    echo "### $f.py  (non-research references)"
    git grep -n -e "${f}.py" -- ':!research' ':!.bench' ':!target' || echo '(no matches)'
    echo "### import ${f}"
    git grep -n -E "^[[:space:]]*(from[[:space:]]+${f}[[:space:]]+import|import[[:space:]]+${f}([[:space:]]|$))" \
      -- ':!research' ':!.bench' ':!target' || echo '(no matches)'
  done
### attempt_id.py  (non-research references)
(no matches)
### import attempt_id
(no matches)

### attempt_lifecycle.py  (non-research references)
(no matches)
### import attempt_lifecycle
(no matches)

### csat_close.py  (non-research references)
(no matches)
### import csat_close
(no matches)

### emit_proof.py  (non-research references)
(no matches)
### import emit_proof
(no matches)

### fcdist.py  (non-research references)
TODO.md:424:  Script: `tools/fcdist.py`; receipt: `research/receipts/v3-recount-5e0fc782.md`.
### import fcdist
(no matches)

### patch_wrapper.py  (non-research references)
TODO.md:104:- [x] **C4. Guard conversions.** Implemented in `tools/patch_wrapper.py` (211 conversion wrappers). An injected `BETA_CONV` failure wrote `BETA_CONV.conv.failures.jsonl`; stock and patched success trees are byte-identical (`diff -rq` exit 0).
### import patch_wrapper
(no matches)

### run_traces_inject.py  (non-research references)
(no matches)
### import run_traces_inject
(no matches)

### test_write_through.py  (non-research references)
(no matches)
### import test_write_through
(no matches)

### trace_conformance.py  (non-research references)
(no matches)
### import trace_conformance
(no matches)

### w1_gate.py  (non-research references)
tools/w1_hol_capture.ml:7:(* The accumulate lists arrive in $W1_LISTS, generated by tools/w1_gate.py:  *)
### import w1_gate
(no matches)

### w2_witnesses.py  (non-research references)
TODO.md:619:- [ ] **D2. `w2_witnesses.py` invariant field is the postcondition**, not an inductive loop
### import w2_witnesses
(no matches)

### w4_close.py  (non-research references)
tools/csat_hol.ml:6:(* reductions that tools/w4_close.py applies before its congruence            *)
### import w4_close
(no matches)

### witness.py  (non-research references)
(no matches)
### import witness
(no matches)
```

The five hits are prose/comment references, not invocations:

- `TODO.md` is the programme queue (owned by the orchestrator) and quotes the old
  tool names as history. It is not a script and was not rewritten.
- `tools/w1_hol_capture.ml:7` and `tools/csat_hol.ml:6,202` are OCaml comments
  recording where a captured list came from; neither file runs a Python tool.

`research/**` also contains historical mentions; that tree is outside this
slice's write scope by instruction.

## Verification

All four commands were run after the deletions and after the test-only edits.

```
$ cargo build --offline
   Compiling tekmerion v0.1.0 (/mnt/1tb_kingston/Tekmerion)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.12s
build-rc=0

$ cargo clippy --offline --all-targets -- -D warnings
    Checking tekmerion v0.1.0 (/mnt/1tb_kingston/Tekmerion)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.06s
clippy-rc=0

$ cargo test --offline   # result lines, verbatim
running 3 tests
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
running 27 tests
test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
running 7 tests
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
running 25 tests
test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
running 21 tests
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
running 13 tests
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
running 14 tests
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
running 25 tests
test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
running 5 tests
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.12s
running 7 tests
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.02s
running 26 tests
test result: ok. 26 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
running 7 tests
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
running 11 tests
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
running 1 test
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test-rc=0

(14 non-empty suites + the bin/doc harnesses; 192 tests passed, 0 failed.)

$ bash tools/selftest.sh   # section headers and verdict lines, verbatim
=== 1/7 OCaml syntax check (syntax only; type-checking needs HOL Light) ===
OK: trace/exptrace_failure.ml parses.
=== 2/7 runner script syntax ===
OK: tools/run_traces.sh parses
SELF-TEST PASSED
OK: run_traces_inject self-test passed
=== 2b/7 attempt_id self-test ===
SELF-TEST PASSED
OK: attempt_id self-test passed
=== 2c/7 attempt_lifecycle self-test ===
SELF-TEST PASSED
OK: attempt_lifecycle self-test passed
=== 2d/7 run_traces.sh argument parsing ===
OK: an unknown option is rejected
OK: --proof is parsed and a missing path is reported
OK: --proof and a theorem name are mutually exclusive
=== 3/7 patch_wrapper --self-test ===
SELF-TEST PASSED
OK: patch_wrapper self-test passed
=== 4/7 trace_conformance --self-test ===
SELF-TEST PASSED
OK: trace_conformance self-test passed
=== 5/7 write-through self-test (module compiled against stubs, no dump) ===
OK: write-through self-test passed
=== 6/7a existing Rust suite (cargo test --offline) ===
OK: cargo test --offline passed
=== 6/7b existing mutation harness (cargo run --bin mutate) ===
OK: cargo run --bin mutate passed
=== 7/7 lifecycle mutation harness (cargo run --bin mutate_lifecycle) ===
OK: cargo run --bin mutate_lifecycle passed
SELF-TEST PASSED
selftest-rc=0
```

## Deviations and notes

1. **`tools/test_write_through.py` is not an oracle for `src/wire.rs`.** It
   compiles `trace/exptrace_failure.ml` against HOL Light stubs and checks the
   failure-journal and attempt-stream wire format. That module is still live, so
   the harness was ported rather than deleted. `tests/write_through.rs` keeps two
   genuinely independent checks: the module's `exptrace_sha256_hex` output is
   compared with the crate's Rust SHA-256, and the module's `exptrace_json_escape`
   output is parsed by the crate's strict JSON reader and round-tripped.

2. **`run_traces.sh` build key.** The key still hashes one digest per transform
   tool, but now hashes `src/tools/{run_traces_inject,patch_wrapper,attempt_lifecycle}.rs`.
   A `--reuse` work directory created before this slice therefore rebuilds once;
   no output changes.

3. **`fcdist` examples branch.** Python's
   `for f, n in c["only_direct_examples"]` unpacks a 3-tuple into two names and
   would raise `ValueError` if it ever ran. Both counters are zero at the pinned
   checkout (and at the v3 revision recorded in the receipt), so the branch never
   fired. The Rust port prints the tuple instead of reproducing the crash. The
   measured report is byte-identical to Python's.

4. **`trace_conformance` diff diagnostics.** The shared JSON reader stores object
   members in sorted order, so a diff message that prints an *object value* may
   order its keys differently from Python's `repr`. Field-by-field comparison
   results and exit codes are unchanged; the fixture transcripts are identical.

5. **`fcdist` error text.** A non-checkout prints the same bare
   `not an s2n-bignum checkout (no arm/proofs): …` message as Python's
   `SystemExit`, exit 1. Argparse-style usage errors are reflowed but keep exit 2
   and the same rejection.

6. **`run_traces_inject` gained `--self-test`.** The Python file had only a
   syntax check in `selftest.sh`; a real `--self-test` lets the section exercise
   the Rust binary, as the brief requires.

7. **Golden fixture provenance.** `tests/fixtures/fcdist_report.{txt,json}` are
   the Python tool's output at commit `5e0fc782ae6d587c2ae40faf2c2c4d0b637e6240`,
   with the absolute checkout and corpus paths replaced by `<S2N>` and
   `<PROBLEMS>`. The test skips when the checkout is absent (CI) or not at that
   commit.

8. **Not run.** `tools/run_traces.sh` itself was not executed end to end: it
   requires the built HOL Light tree and the 20 GB bench. Its argument-parsing
   paths are covered by `selftest.sh` §2d, and the four tools it calls were each
   run directly against the pinned bench inputs above.

9. **No Python remains under `tools/`:**

```
$ find tools -name '*.py' -print
py-count=0
```

`tools/__pycache__/` and `tools/.pytest_cache/` were untracked bytecode caches of
the deleted modules and were removed with them.

10. **Receipts that quote old commands do not make those tools live.**
    `research/receipts/r2-heldout-eval.md` shows historical
    `python3 tools/emit_proof.py --allow-heldout …` commands, and
    `w1-accsteps-gate.md` / `csat-close.json` name their generator. Those are
    frozen snapshots, scoped to the commits they name, not invocations against
    the current tree:

    - the `--allow-heldout` flag no longer exists anywhere in tracked source or
      scripts (`git grep allow-heldout -- src tests tools` has no match), so the
      quoted command is already non-reproducible in this tree;
    - `emit_proof.py`, `w1_gate.py`, `w2_witnesses.py` and `csat_close.py`
      cannot run at all after Slice B deleted the modules they import;
    - the current answer emitter is `src/bin/emit.rs` / `src/bin/emit_x86.rs`,
      and the current step derivation is `src/tools/derive.rs` +
      `src/tools/disasm.rs`.

    The receipts themselves are untouched, so every historical number keeps its
    citation and its stated revision.
