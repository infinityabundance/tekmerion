# XXV.F — local gate green at `4ca0d69`

**Date:** 2026-11-27
**Branch:** `staging`, commit `4ca0d69`, tree clean before and after.
**Host:** 125 GB RAM, 82 GB available (≈6.8× the 12 G cap); 250 GB disk free.

The last GitHub Actions run was **cancelled** during the "Scaffold self-test (offline)" step. This
receipt establishes that the same step and the full local gate pass on this tree. It does not explain
the cancellation: a GitHub cancellation (superseding push, or a workflow timeout) leaves no failing
local exit code either way.

## Results (all under `tools/cap.sh`, one heavy command at a time)

| stage | command | exit |
|---|---|---|
| scaffold self-test | `tools/cap.sh --timeout 3600 12G -- bash tools/selftest.sh` | **0** |
| full suite | `tools/cap.sh --timeout 2400 12G -- cargo test --offline --no-fail-fast` | **0** |
| lint | `cargo clippy --offline --all-targets -- -D warnings` | **0** |
| mutation operator 2 | `MUTATE_GUARD_OPERATOR=accept … --bin mutate_guard` | **0** |

`cargo test`: 45 targets, **396 passed, 0 failed, 0 ignored**. No cap fired; no timeout. Self-test
peaked at ≈3.9 GB group RSS, the accept run at ≈4.6 GB, both far under 12 G.

`tools/selftest.sh` runs, in order: OCaml parse check of `trace/exptrace_failure.ml`; `run_traces.sh`
syntax and its three negative argument probes; `run_traces_inject`, `attempt_id`,
`attempt_lifecycle`, `patch_wrapper`, `trace_conformance` self-tests; `write_through`; the full
`cargo test`; the 16-site `mutate` harness; and the lifecycle/parser `mutate_lifecycle` harness. It
does **not** load HOL Light or touch s2n-bignum. It ends `SELF-TEST PASSED`.

Mutation scores on this tree:

```
reachability  101/101 (0 invalid)
accept         61/61  (40 invalid: `Ok` payload has no `Default`)
selftest's own: mutate 16/16, lifecycle 5/5, parser 16/16
```

## What this establishes and does not

- **Establishes:** every locally runnable CI stage is green at `4ca0d69`, including the stage where
  the remote run was cancelled.
- **Does not establish:** that the GitHub Actions workflow completes; the cancellation cause is a
  CI-infrastructure question (look at `timeout-minutes: 30` and any concurrency/superseding-push
  setting). Do not describe a cancellation as a pass.
- Swap usage rose from 0 to ≈3 GB across the runs. Not a cap firing, but worth watching on the next
  long proof run.
