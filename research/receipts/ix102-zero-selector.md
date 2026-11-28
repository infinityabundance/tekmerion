# IX.102 — canonical zero selector before the split: neg 2,728,198 → 2,380,714, branches 10 → 2

**Date:** 2026-11-28
**Branch:** `staging`, `tools/bridge.ml` sha256
`a14b1ff005f2f0981db6a449b5686f3ddc5513cf7f5df6326f1a93856793d210`.
**Counter:** `hol-light-stamp2`; each proof under `tools/cap.sh 12G -- make -j1`, one at a time, no
cap fired (peaks 1.87–2.01 GB). 97 GB available.

## 1. What changed

The negation zero test reached the bridge as
`val (word_sub (word_or (word_or (word_or n_0 n_1) n_2) n_3) (word 0)) = 0`. With `WORD_SUB_EQ_0`
absent, the normaliser expanded the subtraction into conditional wrap semantics, creating impossible
branches. A new canonicalisation pass (`tools/bridge.ml` §6c.2c2) rewrites, using library theorems
only:

| shape | rewrite | lemmas |
|---|---|---|
| `val (word_sub x (word 0)) = 0` | per-limb `nᵢ = word 0` | `VAL_EQ_0`, `WORD_SUB_EQ_0`, `WORD_OR_EQ_0` (+`CONJ_ACI`) |
| `&(val (word_sub x (word 0))) = &0` | same (integer image) | `INT_OF_NUM_EQ` + the above |
| `word_join a b = word 0` | `a = 0 ∧ b = 0`, nested to fixpoint | `WORD_JOIN_EQ_0` |
| `bignum_of_wordlist l = 0` | `ALL (λx. x = 0) l`, then per-limb | `BIGNUM_OF_WORDLIST_EQ_0`, `ALL` |

It runs **before** `BRIDGE_ALG_NORM_ASSUM` and **before the first `BRIDGE_SPLIT_CONDITIONS_KEEP_TAC`**,
in both entry routes. `CONJ_ACI` puts the machine OR form and the reference's `if x = word 0` (join)
form in one normal form, so the splitter makes **one** split and no `word_sub` survives inside a zero
test for `INT_VAL_WORD_SUB_CASES` to expand.

**Kernel justification.** A pure structural test (`BRIDGE_ZERO_SELECT_TEST`) only *nominates*; the
rewrite is built with `REWRITE_CONV` from the library theorems, and `CONV_TAC` proves the original
goal from the rewritten one. No branch is deleted: the splitter still splits, and the zero case is
*closed* (the goal reduces to `T`). A term with no selector returns `REFL`; an ungated pass cost
`add +221` and an unconditional `RULE_ASSUM_TAC` cost `+24`, so the gate stays.

## 2. Measurement (strict; `plain == phase`, statement-checked)

| task | IX.101 | IX.102 | Δ | ratio to human | branches |
|---|---:|---:|---:|---:|---:|
| add | 3,885,488 | 3,885,488 | 0 | 2.10× | 12 → 12 |
| sub | 4,831,144 | 4,831,144 | 0 | 3.77× | 24 → 24 |
| neg | 2,728,198 | **2,380,714** | **−347,484 (−12.74 %)** | **3.30×** | **10 → 2** |

`add`/`sub` route lines are byte-identical to IX.101 (no selector of this shape). On `neg` the close
falls 2,057 → 1,348; `ORACLE extra-eqs=0` on every leaf — the `&(val V) + &2^64 = &0` equation is gone
from the proof entirely — and the −346,775 balance is pre-close work on the eight vanished leaves.
Every run `STATEMENT_OK aconv=1 hyps=0 axioms=3`.

## 3. Matched session (derived from the measured L7 = 1,496,597)

| task | matched | ratio to human |
|---|---:|---:|
| add | 2,388,891 | **1.29×** |
| sub | 3,334,547 | **2.60×** |
| neg | **884,117** | **1.225×** |

Neg is now within **23 %** of the human proof under matched-session accounting, by subtraction. The
last fresh matched run was `7acf887`.

## 4. What remains

- **Neg still has two leaves reached (the human structure), but the non-zero case still splits the
  256-bit `VAL_WORD_SUB_CASES` guard** into two closes. Reaching the human's flat two leaves is a
  separate guard, not the zero selector.
- **`add`/`sub` have no selector of this shape**, so the mechanism does not touch them; their branch
  counts (12/24) come from arithmetic disjunctions.
- **The step-theorem cache** (neg simulation 528,862) is the lever that would put neg's
  matched-session below the human; the library diet is still needed for a **strict** win.
- Neg's ~0.58M unaccounted (preamble/gaps/tail) is still uninstrumented.

## 5. Not established

- Matched figures are subtraction; timing medians not measured.
- The canonicalisation is proved by `CONV_TAC`; no separate soundness proof is claimed beyond the
  kernel accepting it.
