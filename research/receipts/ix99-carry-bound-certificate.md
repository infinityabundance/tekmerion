# IX.99 — generic carry-bound certificate: sub 5,795,082 → 4,969,556, add → 4,088,374

**Date:** 2026-11-27
**Branch:** `staging`, `tools/bridge.ml` sha256
`00ff20c5a2f19b2debcbea52e52260c8cd3040923304167cd5ac6cb8b948eb33`.
**Counter:** `hol-light-stamp2`; each proof under `tools/cap.sh 12G -- make -j1`, one at a time, no
cap fired (peaks 1.77–2.03 GB). 76 GB available.

## 1. The certificate

From the aggregate `S + N − B^n·c = p` and the bounds `0 ≤ S ≤ B^n − 1`, `0 ≤ N ≤ p`, `c ∈ {0,1}`:
`B^n·c = S + N − p ≤ B^n − 1`, so `c < 1`, so **`c = 0`**. Width-independent: `B` and `n` are derived
from the carrier coefficient multiset (smallest magnitude > 1 is the limb base; every magnitude a
power of it; `|K| = B^n`). No width or limb constant is hard-coded.

- Detection is pure `Num` arithmetic; a shape/value mismatch costs **zero** inferences.
- The proof is one kernel-checked `INT_ARITH` over only the used equations, the bound hypotheses and
  `&0 <= c`. The old `INT_REDUCE` bit-value finish is gone.
- It runs in both the pre-split hoist and the branches, so IX.98's `aconv` reuse and hoist are
  preserved; the `prove_zero_lt`/`prove_fact` fallback is byte-unchanged.

## 2. Measurement (strict; `plain == phase`, statement-checked)

| task | IX.98 | IX.99 | Δ | ratio to human | fact phase before → after |
|---|---:|---:|---:|---:|---:|
| add | 4,310,114 | **4,088,374** | **−221,740 (−5.14 %)** | **2.21×** | 613,764 → **392,024 (−36.1 %)** |
| sub | 5,795,082 | **4,969,556** | **−825,526 (−14.2 %)** | **3.87×** | 1,252,636 → **427,110 (−65.9 %)** |
| neg | 2,941,309 | **2,848,020** | **−93,289 (−3.17 %)** | **3.95×** | 336,655 → **243,366 (−27.7 %)** |

Every run: `STATEMENT_OK task=… aconv=1 hyps=0 axioms=3`; `plain == phase` (4,088,374 / 4,969,556 /
2,848,020). Human: 1,848,494 / 1,283,174 / 721,473.

Certificate fires: **add 8/8 · sub 4/4 · neg 4/4; fallback 0**. Premise counts 7–12 (e.g. neg: corrected
aggregate; four `val sum < B`; `N ≤ p`; `0 ≤ &(bitval c)`).

## 3. Cost, and what is left

The certificate's `INT_ARITH` measures **30,622 inferences per fire** on `neg`; splitting it into
bound-then-final measured **46,141** (both calls normalise the aggregate), so the single small-set call
was kept. So the remaining fact phase is mostly **one `INT_ARITH` normalisation of the aggregate per
fire**. The next step is canonical-row replay (`XXVIII.B`): normalise the aggregate **once** and
combine by merging canonical forms, so the per-fire cost is no longer ~30K.

## 4. What is not established

- **Matched-session accounting**: derived, not measured. At this commit `T` = total − library =
  2,590,313 / 3,471,577 / 1,350,668 → **1.40× / 2.71× / 1.87×** human (derived); `X = H − T − 1` =
  −741,820 / −2,188,404 / −629,196 (all still negative).
- **Timing medians** and the step-theorem cache: not measured.
- The per-fire cost is an inference figure from a scratch marker, not a whole-proof saving.
