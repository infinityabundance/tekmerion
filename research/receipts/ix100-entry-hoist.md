# IX.100 — entry hoist above the written-value split: neg 2,848,020 → 2,745,276, facts 243,378 → 36,321

**Date:** 2026-11-27
**Branch:** `staging`, `tools/bridge.ml` sha256
`9f003c54ca6d99e99376df5499d7b3c635e776748cf08f61bd6a3c84f2877f75`.
**Counter:** `hol-light-stamp2`; each proof under `tools/cap.sh 12G -- make -j1`, one at a time, no
cap fired. 95 GB available.

## 1. What changed

- **Entry hoist.** `BRIDGE_FINISH_KEEP_ALG_TAC` now converts the **assumptions** to the integer
  fragment and runs `BRIDGE_HOIST_AGG_FACTS_TAC` **before** its `BRIDGE_SPLIT_CONDITIONS_KEEP_TAC`
  (the goal and the split conditions are untouched). The in-algebra hoist stays as a fallback.
- **Inheritance.** `bridge_oracle_facts` returns a fact already assumed with the same `v = &0`/`v =
  &1` conclusion (`aconv`) instead of re-deriving it by relaxation. It is a kernel theorem already in
  scope; no new assumption and no proof step.
- Both load-bearing controls are intact: the **single-carrier gate** and **`band_only`**. A
  relaxed-gate probe regressed `add` by **+121,581**, confirming the gate still matters.

## 2. Measurement (strict; `plain == phase`, statement-checked)

| task | IX.99 | IX.100 | Δ | ratio to human | fact phase before → after | fact proofs |
|---|---:|---:|---:|---:|---:|---:|
| add | 4,088,374 | **4,060,716** | **−27,658 (−0.68 %)** | **2.20×** | 392,782 → 392,782 | 14 → **8** |
| sub | 4,969,556 | **4,907,921** | **−61,635 (−1.24 %)** | **3.83×** | 402,550 → 402,550 | 28 → **4** |
| neg | 2,848,020 | **2,745,276** | **−102,744 (−3.61 %)** | **3.81×** | 243,378 → **36,321** | 14 → **1** |

Every run prints `STATEMENT_OK aconv=1 hyps=0 axioms=3`. Human: 1,848,494 / 1,283,174 /
721,473. `plain == phase`, `diff 0`.

**`neg` now proves the final-carry fact once per theorem** (1 fact proof, was 14; the 10 branches and
4 in-algebra hoists are `aconv` reuses). `sub` proves it 4 times (per pre-split goal; the 24
per-branch relaxation proofs are now reuses). `add` proves it 8 times: 2 per-goal plus **6 per-branch
carry certificates in the two goals the single-carrier gate skips** (fallback kept deliberately).

## 3. Matched-session (measured L7 from `xxx-e`, recomputed by subtraction)

L7 = **1,496,597**. Derived at this commit: matched = strict − L7:

| task | matched | ratio to human |
|---|---:|---:|
| add | **2,564,119** | **1.387×** |
| sub | **3,411,324** | **2.659×** |
| neg | **1,248,679** | **1.731×** |

These are `strict − L7`, not fresh matched-session runs; the last true matched run was `7acf887`
(1.40× / 2.71× / 1.87×). Re-measure after the next structural change.

## 4. What remains

- **`add`/`sub` still prove the fact more than once** because a second structural carrier makes the
  single-carrier gate skip the entry hoist. The selector fold (`XXX.B`) — `p·x + (1−p)·y`, one split
  at the end — is the lever that makes it once per theorem for every task.
- **Canonical-row replay (`XXX.D`)** is now worth doing only after the fold, so it optimises ~one
  carry-certificate call per theorem.
- The ~0.58M unaccounted `neg` cost (preamble, inter-split gaps, tail) is still uninstrumented
  (`XXX.F`).

## 5. Not established

- The matched-session figures here are subtraction, not fresh runs.
- Timing medians; the step-theorem cache; the library diet. Strict thresholds remain negative.
