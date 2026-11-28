# IX.98 — hoisting the aggregate and final-carry fact before the split: neg 4,121,086 → 2,941,309

**Date:** 2026-11-27
**Branch:** `staging`, `tools/bridge.ml` sha256
`a1b9a1e05b54d4d483a3888caecd9d13e2651d0f22e63d91f3aec8bfe6f41a53`.
**Counter:** `hol-light-stamp2`; each proof under `tools/cap.sh 12G -- make -j1`, one at a time, no
cap fired (peaks 1.91–3.56 GB). 76 GB available.

## 1. What changed

The aggregate-fact phase was 100 % per branch (`xxviii-neg-per-branch-cost.md`): the same
`&(bitval carry) = &0` was re-proved in every oracle branch. This change proves it **before the first
conditional split** and lets every branch reuse it.

- `BRIDGE_HOIST_AGG_FACTS_TAC` runs between the first normalisation and
  `BRIDGE_SPLIT_CONDITIONS_KEEP_TAC`: it builds the chain aggregate from the equations in `asl`,
  constructs the non-branch bounds (`&0 <= &(val w)`, `&0 <= &(bitval b)` — closed theorems, no
  hypothesis), proves the branch-independent facts, and `ASSUME_TAC`s them.
- `bridge_aggregate_facts` gained an optional `?band_only`; the call site is byte-identical, so the
  `emit.rs` markers and `instr_gen.pl` anchors are untouched.
- A fact whose conclusion `v = &0`/`v = &1` is already an antecedent (`aconv`) is **reused, not
  re-proved**. A failed hoist falls back to the per-branch path unchanged.

Two controls are load-bearing and measured: without the **single-carrier gate** `add` regressed
**+488,528** (a second, branch-decided carrier wasted band/`v=k` attempts); without
**`band_only`** on the hoist, `sub`'s fact phase rose **+214,336** from failed `v = k` searches.

## 2. Measurement (strict; `plain == phase`, statement-checked)

| task | IX.97 | IX.98 | Δ | ratio to human | fact phase before → after |
|---|---:|---:|---:|---:|---:|
| add | 4,396,406 | **4,310,114** | **−86,292 (−1.96 %)** | **2.33×** | 701,306 → **613,764** |
| sub | 6,259,618 | **5,795,082** | **−464,536 (−7.42 %)** | **4.52×** | 1,343,866 → **1,252,636** |
| neg | 4,121,086 | **2,941,309** | **−1,179,777 (−28.63 %)** | **4.08×** | 1,511,696 → **336,655** |

Every run prints `STATEMENT_OK aconv=1 hyps=0 axioms=3`; `plain == phase` with `diff 0` on all
three. Human: 1,848,494 / 1,283,174 / 721,473.

## 3. How many fact proofs remain

| task | hoisted | failed pre-split | per-branch proofs | branches reusing |
|---|---:|---:|---:|---:|
| neg | 4 (`carry_s12 = 0`) | 0 | **0** (was 10) | 10/10 |
| add | 2 (`carry_s8 = 0`) | 0 | 6 (was 12) | 6/12 |
| sub | 2 (`carry_s14 = 0`) | 2 → fallback | 12 (was 24) | 12/24 |

**The fact does not need a branch hypothesis** — it is proved from the aggregate and closed bounds.
It cannot yet be hoisted to *once per theorem* because the entry route
`BRIDGE_FINISH_KEEP_ALG_TAC` already splits on the written-value conditions before `expand`, so the
hoist currently fires once per **pre-split goal** (4 on `neg`, 2 on `add`/`sub`). Hoisting above that
route is the next lever for `add`/`sub`, where 6 and 12 per-branch proofs remain.

## 4. What is not established

- **Matched-session accounting** and timing medians: not measured.
- **Further hoisting** above the entry split: not done; it is the remaining repeated-fact cost on
  `add`/`sub`.
- The per-branch figures are inference costs; the two controls show composition matters (a naive
  hoist regresses one task).
