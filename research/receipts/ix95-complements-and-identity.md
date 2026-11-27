# IX.95 — complements and the linear-identity certificate: neg's close falls from 1,170,099 to 2,057

**Date:** 2026-11-27
**Branch:** `wip/xxvii-b5-complements` (based on `staging` `d3c7184`);
`tools/bridge.ml` sha256 `b9a1926870f0f6c2fa01a4b56e49da9913f9c6efd5db6459ab05f094c48772ec`.
**Counter:** `hol-light-stamp2`; every proof under `tools/cap.sh 12G -- make -j1`, one at a time,
no cap fired (peak group RSS 1.77–1.90 GB). 76 GB available before the sweep.

## 1. What changed (all in `BRIDGE_ORACLE_TAC`, each with fall-through)

1. **Literal complement** (`P`, `~P`, `aconv`): `MP (NOT_ELIM nth) pth` → `CONTR`. **15 inferences**
   per branch, no conversion.
2. **Polynomial-AC complement**: decide `P ≡ Q` up to `+`/`*`/`∧` AC by comparing sorted linear-form
   keys (pure, zero inferences on no-match); on a match normalise both sides with **`INT_POLY_CONV`**
   (a ring normaliser, not a search) and transport. **113 inferences** per branch (≈49 per side for
   the two conversions).
3. **Linear-identity certificate**: from `bridge_lf_row_of_thm` linear forms, find a small subset and
   integer weights whose combination is the goal, then build the proof with
   `bridge_eq_scale`/`bridge_eq_add` over `bridge_flat_th` and close by `TRANS`/`INT_EQ_SUB_RADD`.
   The certificate prints its weights.

## 2. Measurement (strict self-contained; `plain == phase`, `diff 0` on all three)

| task | IX.94 | IX.95 | Δ | ratio to human | close (enter→exit) |
|---|---:|---:|---:|---:|---:|
| add | 5,139,300 | **4,926,695** | **−212,605 (−4.14 %)** | **2.67×** | 999,526 → **786,931** |
| sub | 7,985,276 | 7,985,276 | 0 | 6.22× | 2,554,010 (unchanged) |
| neg | 5,289,824 | **4,121,086** | **−1,168,738 (−22.09 %)** | **5.71×** | 1,170,099 → **2,057** |

Human: 1,848,494 / 1,283,174 / 721,473.

The baselines are not hand-typed: re-running the IX.94 `bridge.ml` (`47eee00a…`) on the same tree
reproduces `neg 5,289,824`, `add 5,139,300` and neg's per-branch close table verbatim.

## 3. Neg, branch by branch

```
IX.94:  108, 21809, 333122, 101565, 634580, 78359, 108, 170, 108, 170   total 1,170,099
IX.95:  108,    15,   1235,     15,    113,    15, 108, 170, 108, 170   total     2,057
        prop   lit   linear     lit     ac     lit  prop range prop range
```

Routes: **prop-conj 3, literal 3, linear 1, ac 1, range 2; no slim, no full.**

- Branch 3 (the arithmetic identity) is `#0 + 2^256·#1` — the chain aggregate plus the decided top
  carry — exactly as classified. **333,122 → 1,235.** No inequality is needed (`N ≤ p` is inert).
- Branch 5 (AC complement): **634,580 → 113.**
- Branches 2/4/6 (literal complements): **21,809 / 101,565 / 78,359 → 15 each.**

`add`: routes full 1, slim 8, linear 3 (e.g. B5 `−#1 + 2^256·#2`, 1,842 vs 57,589; B7/B10
3-premise, 2,446/2,631 vs 76,193/85,732). `sub`: no new path fires and nothing costs on no-match, so
it is byte-identical to IX.94.

## 4. Against the pre-registered target (`XXVI.C1`)

| criterion | target | result |
|---|---|---|
| neg closing phase < human 44,425 | < 44,425 | **2,057 — met (21.6× below)*** |
| fact + close together below 3,074,910 | lower | **1,513,753 — met** |
| matched-session total < 721,473 | < 721,473 | not measured |
| `equals_thm` | mandatory | **not run** (no offline harness) |
| SAT closes | 0 | inherited (algebraic route) |

\*Boundary caveat: 2,057 is the oracle close (`close-enter`→`close-exit`); the human 44,425 is the
final-arithmetic phase. They are close but not identical scopes; read this as "the generated close
region is now far below the human closing phase", not as a theorem-identical phase comparison.

**This is the first complete proof whose *whole* close is closed by certificates** (neg: 10/10
branches, no slim and no `INT_ARITH` of the goal). The principle that took two obligations from
~54,000 to 108 now governs the entire close of one routine.

## 5. Honest notes

- **B8/B10 stay on the range path (170)** because the range attempt runs before the new matching;
  the literal path would close each at 15, so ≈310 more inferences are available by reordering.
- The `close-enter`/`close-exit` delta and the total delta differ by 10 (`add`) and 696 (`neg`); that
  work is outside the bracket and is not attributed.
- **Not established:** `equals_thm`, `axioms()` (not re-run), matched-session accounting, timing
  medians. `sub` is unchanged by this experiment.
- The AC decision uses `INT_POLY_CONV`, a normaliser; no `INT_ACI`/`REAL_ACI` exists in this build.
