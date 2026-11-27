# XXIV.C P0 — the two surviving freeze-parser mutants are killed

**Date:** 2026-11-27
**Branch:** `staging`
**Tree:** clean at the start and at the end (`git status --porcelain` empty).
**Host:** same host as IX.89; `free -g` before the run: total 125, available 92 (≥ 3 × the 12 G
cap, so the run was permitted). Disk 250 G available.

## What was wrong

The guard/freeze mutation harness (`src/tools/mutate_guard.rs`) mutates every
`Err(Refused::new(` site in `src/tools/guard.rs` and `src/tools/freeze.rs` to `Ok(todo!())`
and requires the matching integration-test target to notice. Two `src/tools/freeze.rs` sites
survived:

- `src/tools/freeze.rs:886` — `"{env_where}: {key} must be a string or null"` (env value type);
- `src/tools/freeze.rs:918` — `"{flags_where} slice must be on or off: {other:?}"` (slice flag).

The existing `tests/tools_freeze.rs` exercised a missing variable and a malformed `stamp` tri, but
not a non-string env value and not a malformed `slice`.

## The fix

Commit `fd627ec` adds two tests to `tests/tools_freeze.rs`:

- `a_configuration_env_value_of_the_wrong_type_is_refused()` — `"TEKMERION_FAULT": 3` must be
  refused with `must be a string or null`;
- `a_malformed_slice_flag_is_refused()` — `"slice": "maybe"` must be refused with
  `slice must be on or off`.

A shared `ALL_ENV_UNSET` constant keeps both manifests complete, so the "missing variable" refusal
cannot fire first.

## The decisive run

```
tools/cap.sh --timeout 3600 12G -- cargo run --offline --quiet --bin mutate_guard
```

Exit code `0`, no cap firing, no timeout. Verbatim output:

```
src/tools/guard.rs refusal sites: 60 (0 invalid)
killed: 60  survived: 0
mutation score: 60/60 = 1.000

src/tools/freeze.rs refusal sites: 41 (0 invalid)
killed: 41  survived: 0
mutation score: 41/41 = 1.000

all targets: 101 refusal sites (0 invalid), killed 101, survived 0
mutation score: 101/101 = 1.000
all survivors are recorded equivalent mutants
cap.sh: group=1019792 rc=0 sampled_group_peak_rss_kb=4701636 max_process_vmhwm_kb=1115644
```

Group peak RSS ≈ 4.70 GB, highest per-process `VmHWM` ≈ 1.12 GB — under the 12 G per-process cap;
`rc=0`.

## Scope

This is the local run of the same command CI executes (`cargo run --offline --quiet --bin
mutate_guard`, `.github/workflows/ci.yml` step "Guard refusal-site mutation harness"). It establishes
that the harness is green on this tree. It does **not** by itself establish that the full CI workflow
is green; the remaining CI steps are unchanged and were green at intake per the handoff, but the
next full CI run is what confirms it.

**Verdict:** P0 GREEN locally — 101/101 refusal sites killed, 0 survived, 0 invalid; in particular
neither `freeze.rs:886` (env value type) nor `freeze.rs:918` (slice flag) survives.
