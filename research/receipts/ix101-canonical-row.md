# IX.101 — canonical-row replay of the carry certificate: add 3,885,488, sub 4,831,144, neg 2,728,198

**Date:** 2026-11-28
**Branch:** `staging`, `tools/bridge.ml` sha256
`a901ea4c4eb13175c3d5948d16133da73e09c667c8a95ebbebd946ce9db3026c`.
**Counter:** `hol-light-stamp2`; each proof under `tools/cap.sh 12G -- make -j1`, one at a time, no
cap fired (peaks 1.77–1.80 GB). 95–97 GB available.

## 1. What changed

The IX.99 carry certificate found the premises numerically and then called one `INT_ARITH` over the
raw aggregate (**≈30,622 inferences per fire** because the aggregate was re-normalised). This change
replays a **canonical row** instead:

1. The numeric search (pure `Num`, zero inferences) returns the aggregate combination and its weights
   (including the elimination the `sub` fires need).
2. `bridge_cert_combo_eq` builds that combination's equation with derived rules only (`AP_TERM`,
   `MK_COMB`/`AP_TERM (+)`); no `prove`, no axiom.
3. A **canonical kit** (`lhs−rhs = |k|·v + rest`, `−rest = target`) is derived with two `INT_RING`
   calls and **cached by the aggregate's conclusion + carrier (`aconv`)**; a repeated fire rebuilds
   only cheap `EQ_MP`/`TRANS`/`REWRITE` steps.
4. Each fire proves `target ≤ U` by scaling the per-atom literal and relational bounds
   (`INT_LE_LMUL`, `INT_LE_NEG2`, `INT_LE_ADD2`) and closes with `INT_LET_TRANS` →
   `INT_LT_LMUL_EQ` → discreteness → `INT_LE_ANTISYM`. Ground facts are `INT_REDUCE_CONV`, not
   `INT_ARITH`.
5. Any mismatch returns `None`; the IX.99 `INT_ARITH` line is **byte-identical** and remains the
   fallback. The single-carrier gate, `band_only` and the `aconv` reuse are untouched.
   `REPLAY ok … bound=… weights=…` records the coefficients and the integer combination.

## 2. Measurement (strict; `plain == phase`, statement-checked)

| task | IX.100 | IX.101 | Δ | ratio to human | certificate cost before → after |
|---|---:|---:|---:|---:|---:|
| add | 4,060,716 | **3,885,488** | **−175,228 (−4.32 %)** | **2.10×** | 266,746 → **91,774 (−65.6 %)** |
| sub | 4,907,921 | **4,831,144** | **−76,777 (−1.56 %)** | **3.77×** | 120,654 → **44,053 (−63.5 %)** |
| neg | 2,745,276 | **2,728,198** | **−17,078 (−0.62 %)** | **3.78×** | 30,622 → **13,572 (−55.7 %)** |

All 13 fires converted (add 8, sub 4, neg 1); every run `STATEMENT_OK aconv=1 hyps=0 axioms=3`.
Human: 1,848,494 / 1,283,174 / 721,473.

Canonical kit builds: **add 1** (7 cache hits, 12,760 inf), **sub 2** (2 hits, 10,004 + 4,879),
**neg 1** (0 hits, 5,895). Per-fire cost before → after: add 34,895 / 22,481 → ≈9,700 / 11,027;
sub 44,102 / 16,225 → 3,256–11,329; neg 30,622 → 7,677 (13,572 including the kit).

## 3. Matched session (derived from the measured L7 = 1,496,597)

| task | matched | ratio to human |
|---|---:|---:|
| add | **2,388,891** | **1.29×** |
| sub | **3,334,547** | **2.60×** |
| neg | **1,231,601** | **1.71×** |

Subtraction, not fresh runs; the last true matched run was `7acf887`.

## 4. What remains

- **`add`/`sub` still fire the certificate 8/4 times** because the single-carrier gate skips the entry
  hoist when there is a second structural carrier. The **selector fold** (`XXX.B`, `p·x+(1−p)·y`,
  one split at the end) is the lever that makes it once per theorem for every task.
- **Neg's ~0.58M unaccounted** (preamble, inter-split gaps, tail) is still uninstrumented (`XXX.F`).
- The step-theorem cache and the library diet are still the large remaining items.

## 5. Not established

- Matched figures here are subtraction; timing medians not measured.
- The canonical kit's `INT_RING` cost is paid once per distinct aggregate; on `sub` there are two.
