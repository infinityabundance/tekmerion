# XXX.A — carry-certificate bound arithmetic: fixes and exhaustive tests

**Date:** 2026-11-27
**Branch:** `staging`; `tools/bridge.ml`, `tests/cert_bounds_{shim,selftest_driver}.ml`,
`tests/cert_bounds_selftest.sh` (new), `tools/selftest.sh`, `.github/workflows/ci.yml`.

Nothing about the proofs changed; this fixes the numeric layer that nominates certificate bounds.
The HOL step already re-checked every conclusion, so no theorem was unsound; the defects could emit
wrong candidates and had to be fixed before the layer is a foundation.

## Fixes

1. **`bridge_cert_join_lo`** now takes the **maximum** of two lower bounds (the tighter one); the
   upper joiner already took the minimum.
2. **`bridge_cert_bound`** integrality is exact: a pure endpoint core emits an integer, non-strict
   bound for `k·u + c REL 0`; non-exact division is rounded, never silently dropped.
3. **`bridge_cert_hi`/`lo`** strict tightening uses the endpoint rules for rational `q`:
   `x<q ⟹ x ≤ ceil(q)−1`, `x≤q ⟹ x ≤ floor(q)`, `x>q ⟹ x ≥ floor(q)+1`, `x≥q ⟹ x ≥ ceil(q)`.
   A rational-valued remainder is not rounded.

## Exhaustive tests (`bash tests/cert_bounds_selftest.sh`, wired into `selftest.sh` and CI)

- `bound_core`: 8 multipliers × 17 constants × strict/non-strict = **272 cases**, brute-forced
  against the true feasible integer endpoint.
- Joins: **289** bound pairs, compared on 25 rational sample points.
- `hi`/`lo`: 22 cases against the true max/min.
- Five rule checks including the rational-multiplier guard.

```
bound_core grid: cases=272 failures=0
join grid: 17 x 17 pairs=289 lower_failures=0 upper_failures=0
case x<1/2: inner bound=1/2, legacy(v-1)=-1/2, true max=0
carry-cert bound checks: 34, failures: 0
OK cert_bounds_selftest
```

**Mutation proof** (throwaway copies; tree untouched): reverting `join_lo` → 3 failures; reverting
tightening to `v−1` → 8 failures; removing endpoint rounding → 6 failures. Cases that were wrong
before: `x<1/2` gave −1/2 (true max 0), the half-integer family, non-exact division emitted no bound,
and the old `join_lo` was rejected by the oracle on 224/289 pairs.

## Regression

| op | `STAMP_INFERENCES` (plain = phase) | `STATEMENT_OK` |
|---|---:|---|
| add | 4,088,374 | yes (`aconv=1 hyps=0 axioms=3`) |
| sub | 4,969,556 | yes |
| neg | 2,848,020 | yes |

No candidate changed, so no cost changed. `cargo test`, clippy, `SELFTEST_SKIP_HEAVY=1
tools/selftest.sh`, and the interval-engine self-test are green. Fixtures re-recorded (`BRIDGE_DIGEST`
`576316b6…`, 27 answers, curve_check `statements=95`).

## Queued observation

The separate **interval** joiners `bridge_bound_join_lo`/`hi` also appear inverted against their
comments, and `tests/interval_engine_selftest_driver.ml`'s `sem_max_lo`/`sem_min_hi` clone the same
code, so that check cannot catch it. Both variants are sound but looser. Left untouched (different
layer, would change baked-in expectations); recorded for the queue.
