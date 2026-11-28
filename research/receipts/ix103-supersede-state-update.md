# IX.103 — Tekmerion-owned supersede state update: add −44,714, sub −43,826, neg −34,890

**Date:** 2026-11-28
**Branch:** `staging`; `src/tools/emit.rs` (Tekmerion-owned inlined tactic), 17 ARM answer
fixtures, `tests/ix89/instr_gen.pl` (measurement harness only).
**Counter:** `hol-light-stamp2`; every build under `tools/cap.sh 12G -- make -j1`, one at a time, no
cap fired (peaks 1.88–2.01 GB). 94–96 GB available.

## 1. What changed

The simulation probe found that 26.7 % of the `assume` bucket is spent **failing** to carry facts
about components a step overwrites — they are superseded anyway. The fix is a **Tekmerion-owned**
tactic, not a framework change:

- `src/tools/emit.rs::TEKMERION_ARM_TACTIC` inlines `TEKMERION_SUPERSEDED` (written-component set
  from the `write` spine vs `read c s` subterms), filtering
  `TEKMERION_ASSUMPTION_STATE_UPDATE_TAC`; `TEKMERION_ARM_STEP_TAC` mirrors the framework's
  `ARM_STEP_TAC` with that one substitution; `TEKMERION_ARM_ACCSTEPS_TAC` mirrors `ARM_ACCSTEPS_TAC`.
- `arm_simulation()` emits `TEKMERION_ARM_ACCSTEPS_TAC`; `inlined_answer_with()` appends the tactic
  source only when that name is called, so x86 answers and every answer not using it are
  byte-identical.
- The **shared s2n-bignum framework is untouched**: `components.ml`, `relational.ml`, `arm.ml` diff
  identical to the upstream bench tree.

Two parse-gate details: `TEKMERION_TRY = TRY` (the gate lower-cases HOL value names); and the
framework's `MAP_EVERY` line is kept verbatim because `MATCH_MP` is eager in HOL Light, so the
framework's per-step cost is preserved in the comparison.

## 2. Measurement (strict; `plain == phase`, statement-checked)

| task | simulation before → after | Δ sim | IX.102 | IX.103 | Δ total | ratio to human |
|---|---:|---:|---:|---:|---:|---:|
| add | 787,891 → **743,177** | −44,714 | 3,885,488 | **3,840,774** | −44,714 (−1.15 %) | **2.08×** |
| sub | 674,249 → **630,423** | −43,826 | 4,831,144 | **4,787,318** | −43,826 (−0.91 %) | **3.73×** |
| neg | 528,862 → **493,972** | −34,890 | 2,380,714 | **2,345,824** | −34,890 (−1.47 %) | **3.25×** |

Every run `STATEMENT_OK aconv=1 hyps=0 axioms=3`; the **bridge hypothesis sets are byte-identical**
(add/sub 13 = 13, neg 7 = 7); `plain == phase`. Re-measuring the same answers with only the call name
reverted reproduces the published baselines exactly (3,885,488 / 4,831,144 / 2,380,714), so the
inlined tactic adds **0** kernel inferences of its own.

## 3. Matched session (derived, L7 = 1,496,597)

| task | matched | ratio to human |
|---|---:|---:|
| add | 2,344,177 | **1.27×** |
| sub | 3,290,721 | **2.56×** |
| neg | **849,227** | **1.177×** |

Neg is now within **18 %** of the human proof under matched-session accounting, by subtraction.

## 4. What remains

- **The `MAYCHANGE` frame merge (193,789 on neg) is the remaining simulation blocker.** Deferring it
  was measured worse (+76,687); the four needed requirements were recorded in
  `xxx-state-propagation-feasibility.md`: read/write-set liveness, a relaxed discard, an out-of-band
  write set with a linear one-shot frame proof, and component-indexed resolution instead of a proof
  per write.
- **The liveness stand-in is not shipped**: the prototype's version was program-specific (hard-coded
  steps); a general one needs a read-set/last-use analysis over the decoded sequence.
- `arm/neg_p521` uses `ARM_STEPS_TAC` (empty accumulate set) and is unchanged; a
  `TEKMERION_ARM_STEPS_TAC` counterpart is the natural follow-up so every ARM task benefits.

## 5. Not established

- Matched figures are subtraction; timing medians not measured.
- The Tekmerion tactic is an inlined OCaml definition; its correctness rests on the kernel accepting
  every step it produces, and on the identical hypothesis sets, not on a separate soundness proof.
