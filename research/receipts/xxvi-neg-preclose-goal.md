# XXVI — the negation pre-close goal (`arm.bignum_neg_p25519`)

- **Date:** 2026-11-27
- **Branch:** `staging`, parent `4bc3141` (clean tree; `main` untouched, nothing committed)
- **Purpose:** dump the actual proof obligation handed to the final `INT_ARITH_TAC` in
  `BRIDGE_ORACLE_TAC` for `arm/neg_p25519`, so the witness-compiler prototype can be built
  against the real goal and the real antecedent list.
- **Verdict:** **DUMP CAPTURED (10 branches).**

All numbers below come from command output stored in the scratch tree
`/mnt/1tb_kingston/Tekmerion/.bench/xxvi/` (git-ignored): `neg.correct` (the run log),
`bridge-dump.txt` (the raw 830-line dump). The tracked `tools/bridge.ml` was **not** edited;
the dump was produced from an instrumented *scratch copy* of the emitted phase answer.

---

## 1. Method and commands (exit codes)

The phase answer was emitted, a guarded dump was inserted into the scratch copy at the
`bridge:pre-close` point, and that copy was assembled and proved. Only the emitted answer
(the scratch `.bench/xxvi/neg.answer`) was instrumented; no tracked file was modified.

```
git --no-pager log -n 3 --oneline                                         # 0   HEAD 4bc3141
TEKMERION_ALG_CLOSE=arm.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT \
  ./target/release/emit --task arm.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT \
  --mode phase --out .bench/xxvi/neg.raw.answer \
  --spent --spent-manifest .bench/ix66/spent-manifest.json                # 0   205489 B
python3 .bench/xxvi/inject.py                                             # 0   209735 B (scratch answer)
bash .bench/xxvi/build.sh                                                 # 0
  ./target/release/dropin assemble --task ... --answer .bench/xxvi/neg.answer --stamp  # 0  213238 B
  tools/cap.sh --timeout 1800 12G -- make -j1 -B HOLDIR=... curve25519/bignum_neg_p25519.correct  # rc=0
```

Resource line printed by the capped run:

```
cap.sh: group=<pgid> rc=0 sampled_group_peak_rss_kb=1782076 max_process_vmhwm_kb=1684724
```

`free -g` before the run: 86 GB available (≥ 3× the 12 G cap). Peak group RSS ≈ 1.78 GB, well
under the cap; the cap did **not** fire.

**The proof is unaffected by the instrumentation.** `STAMP_INFERENCES 5683733` — byte-for-byte
the same primitive-inference count IX.83 measured for this task without the dump. All 10
`ORACLE close ok`; `Running time: 29 s`; `val BIGNUM_NEG_P25519_CORRECT` printed. The dump
uses `BRIDGE_DUMP` (a plain env var, no `TEKMERION_*` literal) and is wrapped in
`try ... with _ -> ()`; it only reads terms and writes a file.

### What was inserted (scratch answer only)

1. A one-shot counter `let bridge_dump_counter = ref 0 in` bound in the `BRIDGE_ORACLE_TAC`
   RHS (so it persists across invocations).
2. Immediately after `csat_mark_at "bridge:pre-close";` and before
   `let th = prove(bridge_mk_imps ants w', INT_ARITH_TAC) in`, a guarded block that appends,
   per invocation, the branch ordinal, `!csat_mark_reader ()`, `w'`, the atoms (`keep`), the
   aggregate(s), the aggregate fact(s), every antecedent with its tag, and the
   direct-vs-relational split of `side_hyps` using `bridge_is_direct_bound`.

The exact source of the insertion is `.bench/xxvi/inject.py` (scratch).

---

## 2. Branch inventory

The phase run reaches the oracle **10 times** (`bridge:pre-close` count = 10; `ORACLE close ok`
= 10), grouped under **4** `bridge:algebra` entries. Each leaf is uniquely identified by its
`bridge:pre-close <inference_count>` label, which equals the `inference_count` printed in the
dump header. `bridge:algebra` markers seen, in order: `69855946`, `70059611`, `70075050`,
`72851744` (the middle two are applied to two subgoals before their leaves run).

| # | `bridge:pre-close` | `bridge:algebra-arith` | goal shape | ants | aggs | facts | agg_facts | side_hyps (direct/rel) | extra_eqs | defs | resids |
|---:|---:|---:|:--|---:|---:|---:|---:|---:|---:|---:|---:|
| 1 | 69929370 | 69882601 | `Σs = &0` | 31 | 1 | 1 | 1 | 29 (26/3) | 0 | 4 | 0 |
| 2 | 70027096 | 69985201 | `Σs = &0` | 32 | 1 | 1 | 1 | 30 (26/4) | 0 | 4 | 0 |
| 3 | 70784553 | 70131010 | `Σs = &0` | 32 | 1 | 1 | 1 | 30 (26/4) | 0 | 4 | 0 |
| 4 | 71230391 | 71166749 | `Σs = &0` | 33 | 1 | 1 | 1 | 31 (26/5) | 0 | 4 | 0 |
| 5 | 72004957 | 71350662 | `Σs = &0` | 32 | 1 | 1 | 1 | 30 (26/4) | 0 | 4 | 0 |
| 6 | 72753005 | 72688611 | `Σs = &0` | 33 | 1 | 1 | 1 | 31 (26/5) | 0 | 4 | 0 |
| 7 | 72964563 | 72910748 | `&0 = p−N` | 33 | 1 | 1 | 1 | 31 (26/5) | 0 | 4 | 0 |
| 8 | 73070469 | 73016337 | `&0 = p−N` | 34 | 1 | 1 | 1 | 30 (26/4) | 2 | 4 | 2 |
| 9 | 73169431 | 73114864 | `&0 = p−N` | 33 | 1 | 1 | 1 | 31 (26/5) | 0 | 4 | 0 |
| 10 | 73276171 | 73221287 | `&0 = p−N` | 34 | 1 | 1 | 1 | 30 (26/4) | 2 | 4 | 2 |

Two **distinct goal shapes** appear, matching the hand proof's two witness cases
`if n = 0 then &0 else &p_25519 − &n`:

- **Shape A — the `n = 0` output (branches 1–6).** `w'` says the four written `sum_s*`
  limbs evaluate to 0.
- **Shape B — the `n ≠ 0` output (branches 7–10).** `w'` says `&0 = p − N`, where `N` is the
  flattening of the four input limbs `n_0..n_3`.

**Branches 8 and 10 are the zero-test leaves** (`extra_eqs = 2`, `resids = 2`): they carry the
folded OR test `&(val (word_or (word_or (word_or n_0 n_1) n_2) n_3)) + &2^64 = &0`
(twice) in `extra_eqs`. Branches 1–7 and 9 have `extra_eqs = 0`.

**Note vs. the brief:** the constructed aggregate list has length **1**, not 2
(`aggregates=1` in every leaf). The "corrected" `W_cor` aggregate the comment at
`tools/bridge.ml:2510` mentions is not built for `neg`; only the raw subtraction-chain
aggregate is present. The dump reports the actual count.

---

## 3. Shared structure (identical in all 10 branches)

The single constructed aggregate (`aggregates[0]`) — this is the weighted sum of the
subtract chain, `Σs + N − B^4·carry = p`, verbatim:

```
(((((((&(val sum_s4) + &(val n_0)) + &18446744073709551616 * &(val sum_s6)) +      &18446744073709551616 * &(val n_1)) +     &340282366920938463463374607431768211456 * &(val sum_s9)) +    &340282366920938463463374607431768211456 * &(val n_2)) +   -- &115792089237316195423570985008687907853269984665640564039457584007913129639936 *   &(bitval carry_s12)) +  &6277101735386680763835789423207666416102355444464034512896 * &(val sum_s12)) + &6277101735386680763835789423207666416102355444464034512896 * &(val n_3) = &57896044618658097711785492504343953926634992332820282019728792003956564819949
```

The single aggregate fact (`agg_facts[0]`) — verbatim:

```
&(bitval carry_s12) = &0
```

The **26 direct bounds** (`bridge_is_direct_bound` holds) are the same set in every leaf
(order varies): the `val(word_or …)` pair, the four `val sum_s*` pairs, the four `val n_*`
pairs, and the four `bitval carry_*` pairs.

```
&0 <= &(val (word_or (word_or (word_or n_0 n_1) n_2) n_3))
&(val (word_or (word_or (word_or n_0 n_1) n_2) n_3)) < &18446744073709551616
&0 <= &(val sum_s9)                 &(val sum_s9) < &18446744073709551616
&0 <= &(val sum_s6)                 &(val sum_s6) < &18446744073709551616
&0 <= &(val sum_s4)                 &(val sum_s4) < &18446744073709551616
&0 <= &(val sum_s12)                &(val sum_s12) < &18446744073709551616
&0 <= &(val n_3)                    &(val n_3) < &18446744073709551616
&0 <= &(val n_2)                    &(val n_2) < &18446744073709551616
&0 <= &(val n_1)                    &(val n_1) < &18446744073709551616
&0 <= &(val n_0)                    &(val n_0) < &18446744073709551616
&0 <= &(bitval carry_s9)            &(bitval carry_s9) <= &1
&0 <= &(bitval carry_s6)            &(bitval carry_s6) <= &1
&0 <= &(bitval carry_s4)            &(bitval carry_s4) <= &1
&0 <= &(bitval carry_s12)           &(bitval carry_s12) <= &1
```

The **relational** side hypotheses (what `bridge_is_direct_bound` rejects) are the
branch-defining facts. They are the all-zero / not-all-zero / `N ≤ p` facts, plus the
branch conditions introduced by the disjunction split (e.g. `~(&0 <= &(val (word_or …)))`).
They are listed per branch in §§4–6.

`B = 2^64 = 18446744073709551616`, `p = p_25519 = 57896044618658097711785492504343953926634992332820282019728792003956564819949`.

---

## 4. Branch 1 — `n = 0` output (`bridge:pre-close 69929370`)

Excerpt from `.bench/xxvi/bridge-dump.txt` (branch ordinal 1); the single aggregate line is replaced by a §3 reference and `side_hyp` spacing is lightly realigned, everything else is character-for-character:

```
===== BRANCH 1 (inference_count=69929370) =====
ANT_COUNT 31 aggregates=1 facts=1 agg_facts=1 side_hyps=29 extra_eqs=0 defs=4 resids=0
W_PRIME:
((&6277101735386680763835789423207666416102355444464034512896 *   &(val sum_s12) +   &340282366920938463463374607431768211456 * &(val sum_s9)) +  &18446744073709551616 * &(val sum_s6)) + &(val sum_s4) = &0
ATOMS_OF_W_PRIME:
  atom#0: &(val sum_s12)
  atom#1: &(val sum_s9)
  atom#2: &(val sum_s6)
  atom#3: &(val sum_s4)
AGGREGATES_FULL:
  agg#0: SEE §3 (identical)
AGG_FACTS_FULL:
  agg_fact#0: &(bitval carry_s12) = &0
ANTS:
  #0 [aggregate]: SEE §3 aggregate
  #1 [agg_fact]: &(bitval carry_s12) = &0
  #2  [side_hyp]: &(val (word_or (word_or (word_or n_0 n_1) n_2) n_3)) < &18446744073709551616
  #3  [side_hyp]: &0 <= &(val sum_s9)
  #4  [side_hyp]: &(val sum_s9) < &18446744073709551616
  #5  [side_hyp]: &0 <= &(val sum_s6)
  #6  [side_hyp]: &(val sum_s6) < &18446744073709551616
  #7  [side_hyp]: &0 <= &(val sum_s4)
  #8  [side_hyp]: &(val sum_s4) < &18446744073709551616
  #9  [side_hyp]: &0 <= &(val sum_s12)
  #10 [side_hyp]: &(val sum_s12) < &18446744073709551616
  #11 [side_hyp]: &0 <= &(val n_3)
  #12 [side_hyp]: &(val n_3) < &18446744073709551616
  #13 [side_hyp]: &0 <= &(val n_2)
  #14 [side_hyp]: &(val n_2) < &18446744073709551616
  #15 [side_hyp]: &0 <= &(val n_1)
  #16 [side_hyp]: &(val n_1) < &18446744073709551616
  #17 [side_hyp]: &0 <= &(val n_0)
  #18 [side_hyp]: &(val n_0) < &18446744073709551616
  #19 [side_hyp]: &0 <= &(bitval carry_s9)
  #20 [side_hyp]: &(bitval carry_s9) <= &1
  #21 [side_hyp]: &0 <= &(bitval carry_s6)
  #22 [side_hyp]: &(bitval carry_s6) <= &1
  #23 [side_hyp]: &0 <= &(bitval carry_s4)
  #24 [side_hyp]: &(bitval carry_s4) <= &1
  #25 [side_hyp]: &0 <= &(bitval carry_s12)
  #26 [side_hyp]: &(bitval carry_s12) <= &1
  #27 [side_hyp]: &0 <= &(val (word_or (word_or (word_or n_0 n_1) n_2) n_3))
  #28 [side_hyp]: ~(((&(val n_0) = &0 /\ &(val n_1) = &0) /\ &(val n_2) = &0) /\   &(val n_3) = &0)
  #29 [side_hyp]: &(val n_0) = &0 /\ &(val n_1) = &0 /\ &(val n_2) = &0 /\ &(val n_3) = &0
  #30 [side_hyp]: ((&(val n_0) + &18446744073709551616 * &(val n_1)) +  &340282366920938463463374607431768211456 * &(val n_2)) + &6277101735386680763835789423207666416102355444464034512896 * &(val n_3) <= &57896044618658097711785492504343953926634992332820282019728792003956564819949
SIDE_HYPS direct_bounds=26 relational=3
  RELATIONAL:
    r#0: ~(((&(val n_0) = &0 /\ &(val n_1) = &0) /\ &(val n_2) = &0) /\   &(val n_3) = &0)
    r#1: &(val n_0) = &0 /\ &(val n_1) = &0 /\ &(val n_2) = &0 /\ &(val n_3) = &0
    r#2: ((&(val n_0) + &18446744073709551616 * &(val n_1)) +  &340282366920938463463374607431768211456 * &(val n_2)) + &6277101735386680763835789423207666416102355444464034512896 * &(val n_3) <= &57896044618658097711785492504343953926634992332820282019728792003956564819949
EXTRA_EQS: (none)
```

(The `DIRECT_BOUNDS` block is exactly the 26-item set in §3 and is omitted here only to avoid
repeating it.)

---

## 5. Branch 8 — `n ≠ 0`, zero-test leaf (`bridge:pre-close 73070469`)

This is the branch that carries the folded OR zero test. Excerpt (same convention as §4 — shared aggregate line replaced by a §3 reference, `side_hyp` spacing realigned):

```
===== BRANCH 8 (inference_count=73070469) =====
ANT_COUNT 34 aggregates=1 facts=1 agg_facts=1 side_hyps=30 extra_eqs=2 defs=4 resids=2
W_PRIME:
&0 = (((-- &6277101735386680763835789423207666416102355444464034512896 *    &(val n_3) +    -- &340282366920938463463374607431768211456 * &(val n_2)) +   -- &18446744073709551616 * &(val n_1)) +  -- &1 * &(val n_0)) + &57896044618658097711785492504343953926634992332820282019728792003956564819949
ATOMS_OF_W_PRIME:
  atom#0: &(val n_3)
  atom#1: &(val n_2)
  atom#2: &(val n_1)
  atom#3: &(val n_0)
AGGREGATES_FULL:
  agg#0: SEE §3 (identical)
AGG_FACTS_FULL:
  agg_fact#0: &(bitval carry_s12) = &0
ANTS:
  #0 [aggregate]: SEE §3 aggregate
  #1 [agg_fact]: &(bitval carry_s12) = &0
  #2  [side_hyp]: &0 <= &(val (word_or (word_or (word_or n_0 n_1) n_2) n_3))
  #3  [side_hyp]: &(val (word_or (word_or (word_or n_0 n_1) n_2) n_3)) < &18446744073709551616
  #4  [side_hyp]: &0 <= &(val sum_s9)
  #5  [side_hyp]: &(val sum_s9) < &18446744073709551616
  #6  [side_hyp]: &0 <= &(val sum_s6)
  #7  [side_hyp]: &(val sum_s6) < &18446744073709551616
  #8  [side_hyp]: &0 <= &(val sum_s4)
  #9  [side_hyp]: &(val sum_s4) < &18446744073709551616
  #10 [side_hyp]: &0 <= &(val sum_s12)
  #11 [side_hyp]: &(val sum_s12) < &18446744073709551616
  #12 [side_hyp]: &0 <= &(val n_3)
  #13 [side_hyp]: &(val n_3) < &18446744073709551616
  #14 [side_hyp]: &0 <= &(val n_2)
  #15 [side_hyp]: &(val n_2) < &18446744073709551616
  #16 [side_hyp]: &0 <= &(val n_1)
  #17 [side_hyp]: &(val n_1) < &18446744073709551616
  #18 [side_hyp]: &0 <= &(val n_0)
  #19 [side_hyp]: &(val n_0) < &18446744073709551616
  #20 [side_hyp]: &0 <= &(bitval carry_s9)
  #21 [side_hyp]: &(bitval carry_s9) <= &1
  #22 [side_hyp]: &0 <= &(bitval carry_s6)
  #23 [side_hyp]: &(bitval carry_s6) <= &1
  #24 [side_hyp]: &0 <= &(bitval carry_s4)
  #25 [side_hyp]: &(bitval carry_s4) <= &1
  #26 [side_hyp]: &0 <= &(bitval carry_s12)
  #27 [side_hyp]: &(bitval carry_s12) <= &1
  #28 [side_hyp]: ~(&0 <= &(val (word_or (word_or (word_or n_0 n_1) n_2) n_3)))
  #29 [side_hyp]: ((&6277101735386680763835789423207666416102355444464034512896 * &(val n_3) +   &340282366920938463463374607431768211456 * &(val n_2)) +  &18446744073709551616 * &(val n_1)) + &(val n_0) <= &57896044618658097711785492504343953926634992332820282019728792003956564819949
  #30 [side_hyp]: ~(&(val n_0) = &0 /\ &(val n_1) = &0 /\ &(val n_2) = &0 /\ &(val n_3) = &0)
  #31 [side_hyp]: ((&(val n_0) + &18446744073709551616 * &(val n_1)) +  &340282366920938463463374607431768211456 * &(val n_2)) + &6277101735386680763835789423207666416102355444464034512896 * &(val n_3) <= &57896044618658097711785492504343953926634992332820282019728792003956564819949
  #32 [extra_eq]: &(val (word_or (word_or (word_or n_0 n_1) n_2) n_3)) + &18446744073709551616 = &0
  #33 [extra_eq]: &(val (word_or (word_or (word_or n_0 n_1) n_2) n_3)) + &18446744073709551616 = &0
SIDE_HYPS direct_bounds=26 relational=4
  RELATIONAL:
    r#0: ~(&0 <= &(val (word_or (word_or (word_or n_0 n_1) n_2) n_3)))
    r#1: ((&6277101735386680763835789423207666416102355444464034512896 * &(val n_3) +   &340282366920938463463374607431768211456 * &(val n_2)) +  &18446744073709551616 * &(val n_1)) + &(val n_0) <= &57896044618658097711785492504343953926634992332820282019728792003956564819949
    r#2: ~(&(val n_0) = &0 /\ &(val n_1) = &0 /\ &(val n_2) = &0 /\ &(val n_3) = &0)
    r#3: ((&(val n_0) + &18446744073709551616 * &(val n_1)) +  &340282366920938463463374607431768211456 * &(val n_2)) + &6277101735386680763835789423207666416102355444464034512896 * &(val n_3) <= &57896044618658097711785492504343953926634992332820282019728792003956564819949
EXTRA_EQS:
    e#0: &(val (word_or (word_or (word_or n_0 n_1) n_2) n_3)) + &18446744073709551616 = &0
    e#1: &(val (word_or (word_or (word_or n_0 n_1) n_2) n_3)) + &18446744073709551616 = &0
```

Branch 10 (`bridge:pre-close 73276171`) has the same shape: `w' = &0 = p − N` (its constant
term is `&173688133855974293135356477513031861779904976998460846059186376011869694459885`),
`ATOMS` = `n_3,n_2,n_1,n_0`, `side_hyps` direct/rel = 26/4, and the same two
`word_or … + &2^64 = &0` `extra_eqs` with `resids=2`.

---

## 6. Branch 7 — the clean `n ≠ 0` leaf (`bridge:pre-close 72964563`)

The third distinct leaf: goal shape B (`&0 = p − N`) but **no** folded zero-test equation
(`extra_eqs=0`, `resids=0`); its relational set already has both `allzero` and `~allzero`
facts. Key fields (shared aggregate line replaced by a §3 reference):

```
===== BRANCH 7 (inference_count=72964563) =====
ANT_COUNT 33 aggregates=1 facts=1 agg_facts=1 side_hyps=31 extra_eqs=0 defs=4 resids=0
W_PRIME:
&0 = (((-- &6277101735386680763835789423207666416102355444464034512896 *    &(val n_3) +    -- &340282366920938463463374607431768211456 * &(val n_2)) +   -- &18446744073709551616 * &(val n_1)) +  -- &1 * &(val n_0)) + &57896044618658097711785492504343953926634992332820282019728792003956564819949
ATOMS_OF_W_PRIME:
  atom#0: &(val n_3)
  atom#1: &(val n_2)
  atom#2: &(val n_1)
  atom#3: &(val n_0)
SIDE_HYPS direct_bounds=26 relational=5
  RELATIONAL:
    r#0: ((&6277101735386680763835789423207666416102355444464034512896 * &(val n_3) +   &340282366920938463463374607431768211456 * &(val n_2)) +  &18446744073709551616 * &(val n_1)) + &(val n_0) <= &57896044618658097711785492504343953926634992332820282019728792003956564819949
    r#1: ((&(val n_0) = &0 /\ &(val n_1) = &0) /\ &(val n_2) = &0) /\ &(val n_3) = &0
    r#2: ((&(val n_0) = &0 /\ &(val n_1) = &0) /\ &(val n_2) = &0) /\ &(val n_3) = &0
    r#3: ~(&(val n_0) = &0 /\ &(val n_1) = &0 /\ &(val n_2) = &0 /\ &(val n_3) = &0)
    r#4: ((&(val n_0) + &18446744073709551616 * &(val n_1)) +  &340282366920938463463374607431768211456 * &(val n_2)) + &6277101735386680763835789423207666416102355444464034512896 * &(val n_3) <= &57896044618658097711785492504343953926634992332820282019728792003956564819949
EXTRA_EQS: (none)
```

(ants `#0` = §3 aggregate, `#1` = `&(bitval carry_s12) = &0`, `#2`–`#27` = the 26 direct
bounds of §3, `#28`–`#32` = the five relational facts above.)

---

## 7. Upstream proof (`arm/proofs/bignum_neg_p25519.ml`, read-only)

Source: `.bench/s2n-bignum/arm/proofs/bignum_neg_p25519.ml`; identical read-only to
`.bench/ix89/human/arm/proofs/bignum_neg_p25519.ml` (lines 68–88 diff clean). Line numbers are
that file's.

**Witness and the congruence close** (`:70`–`:72`):

```
70	  CONV_TAC SYM_CONV THEN MATCH_MP_TAC EQUAL_FROM_CONGRUENT_MOD_MOD THEN
71	  MAP_EVERY EXISTS_TAC
72	   [`256`; `if n = 0 then &0:real else &p_25519 - &n`] THEN
```

The hand-typed witness is the pair `(256, if n = 0 then &0:real else &p_25519 − &n)` — the
`REAL` value the written residue is matched to, with the `n = 0` split exactly the two goal
shapes dumped above (Shape A ↔ `&0`, Shape B ↔ `&p_25519 − &n`).

**Zero-test bridge** (`:81`–`:82`), the OR-of-limbs test:

```
81	  EXPAND_TAC "n" THEN REWRITE_TAC[BIGNUM_OF_WORDLIST_EQ_0; ALL] THEN
82	  REWRITE_TAC[VAL_EQ_0; WORD_SUB_EQ_0; WORD_OR_EQ_0] THEN
```

- `BIGNUM_OF_WORDLIST_EQ_0` at `:81` — `bignum_of_wordlist ws = 0 ⇔ (ALL (λw. w = 0) ws)`.
- `VAL_EQ_0` at `:82` — `val w = 0 ⇔ w = 0`.
- `WORD_SUB_EQ_0` at `:82` — `word_sub x y = 0 ⇔ x = y` (pushes the zero test through the
  subtract chain that produced the output limbs).
- `WORD_OR_EQ_0` at `:82` — `word_or x y = 0 ⇔ x = 0 ∧ y = 0` (turns the accumulated OR
  selector into per-limb zero tests). This is the `word_or (word_or (word_or n_0 n_1) n_2) n_3`
  that appears in the dumped relational/extra equations.

**Final tactic order** (`:83`–`:88`):

```
83	  REWRITE_TAC[COND_SWAP; CONJ_ACI; bignum_of_wordlist] THEN
84	  COND_CASES_TAC THEN ASM_REWRITE_TAC[VAL_WORD_0; p_25519] THEN
85	  CONV_TAC NUM_REDUCE_CONV THEN CONV_TAC REAL_RAT_REDUCE_CONV THEN
86	  REWRITE_TAC[INTEGER_CLOSED; GSYM REAL_OF_NUM_CLAUSES] THEN
87	  ACCUMULATOR_POP_ASSUM_LIST(MP_TAC o end_itlist CONJ o DESUM_RULE) THEN
88	  DISCH_THEN(fun th -> REWRITE_TAC[th]) THEN REAL_INTEGER_TAC);;
```

Order: `REWRITE_TAC[COND_SWAP; CONJ_ACI; bignum_of_wordlist]` → `COND_CASES_TAC` →
`ASM_REWRITE_TAC[VAL_WORD_0; p_25519]` → `NUM_REDUCE_CONV` → `REAL_RAT_REDUCE_CONV` →
`REWRITE_TAC[INTEGER_CLOSED; GSYM REAL_OF_NUM_CLAUSES]` → pop/dissolve the accumulator
assumptions with `DESUM_RULE` → `REAL_INTEGER_TAC`.

Before this, the congruence obligations are discharged at `:73`–`:80` (`REPEAT CONJ_TAC
THENL [BOUNDER_TAC[]; … ARITH_TAC; … ARITH_TAC; ALL_TAC; COND_CASES_TAC … MOD_LT …]`), leaving
the zero-test subgoal that `:81`–`:88` closes.

---

## 8. Observations for the prototype

- **Two goal shapes, two witness values.** Shape A (`Σs = &0`, branches 1–6) and Shape B
  (`&0 = p − N`, branches 7–10) are the `n = 0` / `n ≠ 0` cases of the hand witness
  `if n = 0 then &0 else &p − &n`. A compiler that reproduces the witness must produce both.
- **Atoms.** Shape A's atoms are the four output limbs `val sum_s4/s6/s9/s12`; Shape B's are
  the four input limbs `val n_0..n_3`. The two aggregates are over the subtract chain
  (`sum_s*`, `n_*`, `carry_s12`).
- **Zero-test leaves.** Branches 8 and 10 additionally carry the folded OR equation
  `&(val (word_or … n_3)) + &2^64 = &0` (twice) in `extra_eqs` with `resids=2`. These are the
  leaves the upstream `WORD_OR_EQ_0` / `VAL_EQ_0` chain maps onto; the dump is the ground
  truth for how that test is presented to `INT_ARITH`.
- **Direct vs relational.** `bridge_is_direct_bound` classifies 26 of the side hypotheses as
  one-sided bounds on a single atom; the remaining 3–5 per leaf are the all-zero /
  not-all-zero / `N ≤ p` facts and the disjunction-split conditions. Any compiler must keep
  the relational facts (they are what distinguish leaves) while the 26 direct bounds are
  interchangeable context.
- **Count discrepancy.** `aggregates = 1` (the brief's "two constructed chain aggregates" does
  not hold for `neg` at this commit). The single aggregate is the raw subtraction chain; no
  corrected `W_cor` aggregate is constructed.

---

## 9. Reproduction

```sh
# phase answer (uninstrumented)
TEKMERION_ALG_CLOSE=arm.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT \
  ./target/release/emit --task arm.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT \
  --mode phase --out .bench/xxvi/neg.raw.answer \
  --spent --spent-manifest .bench/ix66/spent-manifest.json
# scratch instrument (adds the guarded pre-close dump, no tracked edit)
python3 .bench/xxvi/inject.py
# assemble + single capped proof; BRIDGE_DUMP is set inside build.sh
bash .bench/xxvi/build.sh
# dump: .bench/xxvi/bridge-dump.txt (830 lines, 10 branches)
```

Scratch artefacts (all git-ignored): `.bench/xxvi/{neg.raw.answer,neg.answer,inject.py,
build.sh,neg.log,neg.correct,bridge-dump.txt}`.

**Verdict:** `DUMP CAPTURED (10 branches)` — 10 distinct `bridge:pre-close` leaves under 4
`bridge:algebra` entries, covering both witness goal shapes and the two zero-test leaves, with
the proof's inference count unchanged (`STAMP_INFERENCES 5683733`).
