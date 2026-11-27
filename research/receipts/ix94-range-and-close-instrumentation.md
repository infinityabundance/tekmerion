# IX.94 — range refutation, literal close instrumentation, and the neg branch profile

**Date:** 2026-11-27
**Branch:** `wip/xxiii-g-propref`, merge-base `ed55b94`; `tools/bridge.ml` sha256
`47eee00aa0326b45aa48a17f983e23af8fefbfce163f4ff25cfe7eee8e9b00f9`.
**Counter:** `hol-light-stamp2`; every proof under `tools/cap.sh 12G -- make -j1`, one at a time,
no cap raised, none fired. 78 GB available before the sweep.

## 1. What changed

1. **Propositional-refutation fast path** (IX.93) integrated into `BRIDGE_ORACLE_TAC`: an antecedent
   `P` and `~Q` with `P`, `Q` AC-equivalent conjunctions close the branch via `CONTR` (108
   inferences).
2. **Range-refutation fast path**: `&0 <= &(val x)` together with `&(val x) + &k = &0` (or
   `&(val x) = -- &k`), `k > 0`, closes via a hand chain
   `INT_EQ_SUB_LADD`/`INT_SUB_LZERO`/`INT_REDUCE_CONV`/`INT_LET_ANTISYM` (170 inferences), no
   `INT_ARITH`.
3. **Phase markers now bracket the whole close.** `src/tools/emit.rs` emits `bridge:close-enter`
   before the propositional attempt and `bridge:close-exit` on every returning branch; the legacy
   `pre-close`/`post-close` and the `instr_gen.pl` anchors are unchanged. Phase runs show exactly one
   enter and one exit per branch (add 12/12, sub 24/24, neg 10/10). **The close cost is now the
   enter→exit delta**; the `parse_gen` D bucket still bills only full-set fallbacks.

## 2. Measurement (strict self-contained)

| task | IX.92 | IX.94 | Δ | ratio to human | close (enter→exit) |
|---|---:|---:|---:|---:|---:|
| add | 5,139,300 | 5,139,300 | **0** | 2.78× | 999,526 |
| sub | 7,985,276 | 7,985,276 | **0** | 6.22× | 2,554,010 |
| neg | 5,469,909 | **5,289,824** | **−180,085 (−3.29 %)** | **7.33×** | **1,170,099** |

`plain == phase` on all three (`STAMP_INFERENCES` identical; `parse_gen` `diff 0`; removing the
phase lines leaves a 0-line diff). Human: 1,848,494 / 1,283,174 / 721,473.

`add`/`sub` are **byte-identical** to IX.92: the refutation paths construct nothing on a no-match, so
they are free when they do not fire. They fire only on `neg`.

## 3. Neg, branch by branch

Routes: **prop 3, range 2, slim 5, full 0**. Per-branch close cost, in branch order:

```
108, 21809, 333122, 101565, 634580, 78359, 108, 170, 108, 170
```

The five slim branches are the remaining cost: **1,169,435 of the 1,170,099**. The classification
(`xxvii-neg-branch-classification.md`) says:

- branch 3 is the **one arithmetic branch** — the goal is a linear identity (`#0 + #1` *is* the goal),
  yet it costs **333,122**;
- branch 5 is a literal/AC complement that the current prop path misses (it only recognises `∧`-AC,
  not `P`/`~P` on arbitrary atoms, nor `+`/`*`-AC), costing **634,580**;
- branches 2, 4, 6 (21,809 / 101,565 / 78,359) are literal complements (`&0 <= &(val V)` vs
  `~(&0 <= &(val V))`), also missed by the `∧`-only matcher.

So the next two levers are exact and small: **literal/AC complement matching** (turns 21,809 /
101,565 / 78,359 / 634,580 into ~108 each) and a **linear-identity certificate** for branch 3
(should be hundreds, not 333,122).

## 4. What this does and does not establish

- **Establishes:** the refutation paths are sound (kernel proofs), free on no-match, and cut `neg`
  by 180,085. The close instrumentation now measures the whole close.
- **Does not establish:** `equals_thm` (no offline harness; not run), matched-session accounting,
  timing medians. No cross-experiment subtraction from IX.93.
- **Not a witness proof.** The canonical-residue witness still has to be compiled for the branch
  that carries real arithmetic (branch 3).
- **The 108/170 figures are per-branch close costs**, not whole-proof savings.
