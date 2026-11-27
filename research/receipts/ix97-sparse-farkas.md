# IX.97 — sparse Farkas certificate: add 4,926,695 → 4,396,406, sub 6,646,550 → 6,259,618

**Date:** 2026-11-27
**Branch:** `wip/xxvii-b11-farkas` (based on `staging` `c760f1a`);
`tools/bridge.ml` sha256 `ae942c74d63fbe1f3f450bc3ca3199c264960a7653c96a517de8312116fb42ae`.
**Counter:** `hol-light-stamp2`; each proof under `tools/cap.sh 12G -- make -j1`, one at a time, no
cap fired (peak group RSS 1.86–2.03 GB). 54–72 GB available.

## 1. What was added

A **sparse Farkas certificate** in `BRIDGE_ORACLE_TAC`, after the linear-identity certificate and
before the slim close, with fall-through. Detection is pure `Num` arithmetic on the linear forms, so a
no-match spends **zero kernel inferences**.

- Each antecedent comparison is normalised to `0 ≤ P`: strict `a<b ↦ a+1 ≤ b` (`INT_LT_DISCRETE`),
  negations via `INT_NOT_LE`/`INT_NOT_LT` + `INT_SUB_LE`.
- Supports of size ≤ 3 are searched with small integer multipliers; residual atoms are cancelled with
  the free type facts `0 ≤ &(val …)` / `0 ≤ &(bitval …)` (`INT_POS`, coefficient must be ≤ 0); the
  certificate is accepted only when the summed constant is negative.
- The proof is **derived-rule only**: scale with `INT_LE_MUL`, add with `INT_LE_ADD`, prove the sum
  with `CONV_TAC INT_RING`, close with `INT_LET_ANTISYM` against a ground `&c < 0`. **No `INT_ARITH`
  of the goal and no new axiom.**

Search bounds: supports ≤ 3, multipliers 1..3, **≤ 300 trials**; one `INT_RING` + one ground
`INT_ARITH` per fired branch. The cap never fired. **Attempt cost: 0 kernel inferences** (the search
is arithmetic on numbers, not proofs).

## 2. Measurement (strict; `plain == phase`, `diff 0`)

| task | IX.96 | IX.97 | Δ | ratio to human | close (enter→exit) |
|---|---:|---:|---:|---:|---:|
| add | 4,926,695 | **4,396,406** | **−530,289 (−10.76 %)** | **2.38×** | 786,931 → **259,274** |
| sub | 6,646,550 | **6,259,618** | **−386,932 (−5.82 %)** | **4.88×** | 1,217,732 → **831,616** |
| neg | 4,121,086 | 4,121,086 | 0 | 5.71× | 2,057 |

Human: 1,848,494 / 1,283,174 / 721,473.

**11 branches fired** (4 `sub`, 7 `add`):

| branch | before | after |
|---|---:|---:|
| sub ×4 | 103,949 / 104,972 / 101,532 / 102,555 | **6,723 each** |
| add | 56,296 · 98,156 · 107,002 · 60,810 · 68,077 · 109,733 · 72,596 | **4,609 · 8,161 · 6,491 · 6,491 · 4,609 · 8,161 · 6,491** |

Routes after: `add` farkas 7, cert 3, slim 1, full 1; `sub` order 12, cert 2, farkas 4, slim 3,
full 3; `neg` prop 3, literal 3, linear 1, AC 1, range 2. No branch increased; `neg` untouched.

## 3. Honest boundary (reverted, reported)

Four slim branches remain — `sub` #15/#16 (`p`, 100,884/101,907), `sub` #22 (`0`, 72,644), `add` #11
(`0`, 80,728). Their certificate needs an **equation row with a large coefficient** (the chain
aggregate). That was implemented (each integer equation exposed as `l ≤ r` and `r ≤ l`) and
**measured to regress** — `INT_RING` over the aggregate costs ≈100K and it perturbed two good
branches — so it was **reverted**, not hidden.

## 4. What is not established

- **`equals_thm` / `axioms()`**: not re-run (no offline harness yet). Mandatory before any victory
  claim.
- **Matched-session accounting** and timing medians: not measured.
- The per-branch figures are close costs, not whole-proof savings; no cross-experiment subtraction.
- The certificate covers 11 of the 15 reachable slim branches; the other 4 need the aggregate row
  handled more cheaply than `INT_RING` over it.
