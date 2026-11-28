# IX.104 — one-shot MAYCHANGE frame proof: add 3,749,057, sub 4,712,887, neg 2,276,021

**Date:** 2026-11-28
**Branch:** `staging`; `src/tools/emit.rs` (+195/−9) and 17 ARM answer fixtures. Shared framework
untouched; no new axiom.
**Counter:** `hol-light-stamp2`; every build under `tools/cap.sh 12G -- make -j1`, one at a time, no
cap fired. 94–95 GB available.

## 1. What changed

The shipped `MAYCHANGE_STATE_UPDATE_TAC` canonicalises the **whole accumulated prefix** after every
write and commutes the new component back through it. On `neg` (61 writes): `canon_l` 14,777 +
`canon_r` 1,281 + `move` 176,822 = 192,880; **45 writes absorb, 15 do not, and those 15 burn 120
swaps / 76,133 inferences commuting a component that is then appended anyway** (the proof is
discarded).

The Tekmerion-owned replacement (`TEKMERION_MAYCHANGE_STATE_UPDATE_TAC` and helpers, inlined in the
generated answer):

- reads the step's write spine as a list **out of band**, canonicalises the incoming frame **once**
  and each new entry **once**;
- decides membership by comparing component terms (`aconv`) instead of commuting;
- does one move-back only for components already present, and converts to the shipped `MAYCHANGE`
  form once at the end.

So 2×19 canonicalisations instead of 2×61, and zero commutations for the 15 new components: linear in
(frame length + writes), not (writes × frame length). The frame term after every step is asserted
equal to the shipped rule's, and the A/B diffs every goal and all 26 assumption conclusions
byte-identically, all 19 frame predicates, and all 7 bridge hypotheses.

Component-indexed resolution is a memoised swap lemma (293 calls, 72 first-time proofs; the
framework re-proved `memory :> bignum(z,4)` every time).

## 2. Measurement (strict; `plain == phase`, statement-checked)

| task | simulation before → after | Δ sim | IX.103 | IX.104 | Δ total | ratio to human |
|---|---:|---:|---:|---:|---:|---:|
| add | 743,177 → **637,768** | −105,409 | 3,840,774 | **3,749,057** | **−91,717 (−2.39 %)** | **2.03×** |
| sub | 630,423 → **546,616** | −83,807 | 4,787,318 | **4,712,887** | **−74,431 (−1.55 %)** | **3.67×** |
| neg | 493,972 → **414,793** | −79,179 | 2,345,824 | **2,276,021** | **−69,803 (−2.98 %)** | **3.15×** |

Every run `STATEMENT_OK aconv=1 hyps=0 axioms=3`; baselines re-measured in-tree with only the call
name reverted reproduce IX.103 exactly. Representative per-step (`neg`): s1 3,896 → 583; s4 23,700 →
5,593; s7 20,131 → 7,207; s18 23,527 → 6,363; s19 29,703 → 8,269.

**Measured caveat:** 9,376 (`sub`/`neg`) / 13,692 (`add`) of the sim saving reappears in the final
subsumption segment because the shipped rule's discarded commutations warmed the framework's global
`assigns_swap_conv_cache`, which `MONOTONE_MAYCHANGE_TAC` reuses; clearing that cache after the last
step costs the *shipped* path +24,477. Net is still a win on every task, and that segment is not the
simulation bucket.

## 3. Matched session (derived, L7 = 1,496,597)

| task | matched | ratio to human |
|---|---:|---:|
| add | 2,252,460 | **1.219×** |
| sub | 3,216,290 | **2.506×** |
| neg | **779,424** | **1.080×** |

**Neg is now within 8 % of the human proof under matched-session accounting** (by subtraction).

## 4. What remains

- **Arm's `ARM_CONV` bucket is ≈101K**; the step cache would cut it (~77K on neg, pays when its
  derivation is shared in L7), which would put neg's matched-session **below** the human (779,424 −
  ≈77K ≈ 702K < 721,473).
- Relaxed discard was not exercised: the frame path carries one hypothesis and IX.103's supersede
  filter already removes superseded reads.
- Liveness was not attempted (the prototype needed a program-specific hard-code).

## 5. Not established

- Matched figures are subtraction; timing medians not measured.
- The inlined tactics are OCaml definitions; correctness rests on the kernel accepting every step and
  on the byte-identical hypothesis/frame terms, not a separate soundness proof.
