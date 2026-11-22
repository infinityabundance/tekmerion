# Receipt — G1 real failure tracing (2026-11-22)

Environment: `.bench/` at the pins in `research/receipts/environment-gate.md`. No model calls.
Full working notes: `.bench/G1-TRACE.md` (git-ignored).

## Gate closure

- x86 `make build_proofs -j8` ran 17:10:54Z → 18:50:15Z and was **terminated at the 3 GB disk
  floor** (2.93 GB free at 18:48:54). No exit status exists; 335 of 342 proof binaries were
  produced and 7 were in flight.
- The corpus was **not** harmed: `build-proof.sh` writes `toplevel-thms` before compiling, and all
  seven in-flight modules already had their triplets.
- `collect-problems.py` → **2,622**, matching the leaderboard's current view exactly:

| category | count |
|---|---|
| generic | 686 |
| program_state | 601 |
| functional_correctness_arm | 463 |
| bit_vector | 437 |
| functional_correctness_x86 | 435 |
| **total** | **2,622** |

This closes the count dispute for this pin: 2,622 full; 2,301 and 2,284 are not produced here.

## G1 — failure tracing inside real HOL Light

Thesis: `arm/proofs/word_max.ml` (3 instructions; unmodified pass confirmed, 35.0 s).

| Check | Result |
|---|---|
| Stock baseline | `.bench/G1/traces/stock/`, 266 JSON files |
| Wrapper regeneration | byte-identical to the pinned generation (sha `11943794…`) |
| Success-record conformance, failure channel active | **0 differences over 1,569 aligned pairs** |
| Recovered failure `TRY (FAIL_TAC …)` | 1 journal line, `"scope": "tactic"`, written 2.7 s **before** the end-of-proof dump |
| Terminal failure | exception `Failure("G1-terminal-failure")`, exit 2, no trace dir created, yet **30 journals** persisted |
| Replay | byte-identical line, same `goal_before`, same sha256 (`e5a9f4ce…`) |
| Journal validity | 110,755 lines across 121 files, all valid JSON with `scope: tactic` |
| TacticTrace restore | `git checkout -- .`; HEAD `7ea3808…`; `git status` empty; wrapper sha restored |

This is the first evidence that the instrument survives a proof that dies. It is exactly the
behaviour `TacticTrace` lacks ("a proof that fails writes no traces").

## Defects found — all must be fixed before the corpus is used

1. **`trace/exptrace_failure.ml` does not compile against pinned TacticTrace `7ea3808`.**
   It calls `ExportTrace.effective_output_dir` and `ExportTrace.get_sampling_config`, which do
   not exist in that commit. The experiment used a derived copy with that one body replaced by
   `Sys.getenv "G1_FAILURE_DIR"`. The tracked file is wrong and must be made standalone.
2. **`research/03-INSTRUMENT.md` §9 has the wrong signature.** It documents `exptrace_dump ()`;
   the real function is `string -> unit`.
3. **The harness never produces traces.** `setup.sh` runs `make build_proofs`, which *builds*
   proof binaries but does not *execute* them; measured 0 JSON under `.bench/trace-logs` after a
   successful ARM run. In addition, `build-proof.sh:88` pre-creates the dump directory that
   `exportTrace.ml:192` then `Sys.mkdir`s, so even an execution path would fail. A runner that
   executes the binaries is required.
4. **Conversions remain unguarded** — 211 conversion wrappers; 288 conversion trace files skipped
   by the conformance tool.

## Disk

667 `.native` artefacts (regenerable) were deleted, freeing 27,136 MiB. Volume now has 29 GB
free. Rebuilding them costs another full `make build_proofs`; do not delete `objfiles/` or
`toplevel-thms/`.
