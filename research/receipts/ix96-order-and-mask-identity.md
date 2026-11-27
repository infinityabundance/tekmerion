# IX.96 — order-antisymmetry refutation and the mask-substitution identity: sub 7,985,276 → 6,646,550

**Date:** 2026-11-27
**Branch:** `wip/xxvii-b9-sub-general` (based on `staging` `add38b3`);
`tools/bridge.ml` sha256 `cae44c88ad46a740f1396ab5310c08158a8012fac331639becb6fbfb83c4a081`.
**Counter:** `hol-light-stamp2`; each proof under `tools/cap.sh 12G -- make -j1`, one at a time, no
cap fired (peak group RSS 1.78–1.88 GB). 55–57 GB available.

## 1. What was added (fall-through, real kernel proofs)

1. **Order-antisymmetry refutation.** Normalise every hypothesis to `a ≤ b` / `a < b`, reading a
   negated comparison as its complement (`INT_NOT_LE`/`INT_NOT_LT`) lazily (zero cost on no-match).
   An `a ≤ b` with `b < a` closes via `INT_LET_ANTISYM`; `a < b` with `b < a` via `INT_LT_ANTISYM`.
   This is the two-premise Farkas certificate with `λ = (1,1)`. Cost **40** (ordinary) / **84**
   (negated) per branch. It runs after literal/AC, so `neg` keeps its 15-inference route.
2. **Mask-substitution identity search.** A bounded backtracking search inside the linear
   certificate, entered only after the one/two/greedy searches miss; it accepts a step that cancels
   the residual's leading atom even when support does not shrink (the `mask → val` substitution),
   depth 6, each premise once. Candidates are still checked by `INT_POLY_CONV`, so nothing unsound is
   built. Cost 3,322 and 2,177 on the two branches it closes.

## 2. Measurement (strict; `plain == phase`, `diff 0`)

| task | IX.95 | IX.96 | Δ | ratio to human | close (enter→exit) |
|---|---:|---:|---:|---:|---:|
| add | 4,926,695 | 4,926,695 | 0 | 2.67× | 786,931 |
| sub | 7,985,276 | **6,646,550** | **−1,338,726 (−16.77 %)** | **5.18×** | 2,554,010 → **1,217,732** |
| neg | 4,121,086 | 4,121,086 | 0 | 5.71× | 2,057 |

Human: 1,848,494 / 1,283,174 / 721,473. `add` and `neg` do not regress.

## 3. The `sub` branch profile (24 branches)

**12 order-refutations (40/84 each), 2 linear identities (2,177 / 3,322), 7 slim (≈100K each), 3
full (168K–179K).** No `sub` branch is propositionally inconsistent or a single-atom range
contradiction. The remaining **10 branches are genuine multi-premise linear arithmetic** over two
aggregates, the carry fact, the mask equation and five relational facts — nothing cheaper than
`INT_ARITH` on a reduced set has been found for them yet.

`add` (12 branches): **3 identities, 8 slim, 1 full** — unchanged by this experiment.

## 4. What remains and what is not established

- **Remaining close:** `sub` 1,217,732 (7 slim + 3 full ≈ 1.21M), `add` 786,931 (8 slim + 1 full
  ≈ 0.79M). These are the targets for a **general sparse Farkas certificate** (level 4,
  `XXVII.B`), which has not been built.
- **Not established:** `equals_thm` / `axioms()` (no offline harness yet), matched-session
  accounting, timing medians. `add` and `neg` unchanged is a statement about this commit only.
- The 40/84-inference figures are per-branch close costs, not whole-proof savings.
