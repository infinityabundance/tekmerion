# XXVII — classification of the ten `arm.bignum_neg_p25519` pre-close branches, and the branches 8/10 wrap equation

- **Date:** 2026-11-27
- **Branch:** `staging`, HEAD `bb289dd`, clean tree. Nothing committed, nothing pushed, `main`
  untouched, **no tracked file modified**. The only new tracked-path file is this receipt (untracked
  until the orchestrator commits it).
- **Inputs:** `research/receipts/xxvi-neg-preclose-goal.md` (committed), the raw dump
  `.bench/xxvi/bridge-dump.txt` (git-ignored), and one new instrumented bridge run writing
  `.bench/xxvii/probe.txt` (git-ignored).
- **Verdict:** `CLASSIFIED: 9 prop, 2 range, 0 constant, 1 arithmetic, 0 witness;
  BRANCH 8/10 = (a)` — the splitter is exploring an **unsatisfiable branch of the subtraction that
  implements the machine's zero test**, not mis-folding a `bitval`.

---

## 1. Method, commands, resources

### 1.1 What was run

| # | command | rc | output |
|---:|---|---:|---|
| 1 | `python3 .bench/xxvii/inject2.py` | 0 | `.bench/xxvii/neg2.answer` (209454 B) |
| 2 | `bash .bench/xxvii/build2.sh` | 0 | `.bench/xxvii/neg2.log`, `.bench/xxvii/neg2.correct`, `.bench/xxvii/probe.txt` |
| 3 | `tools/cap.sh --timeout 900 12G -- ./hol.sh < .bench/xxvii/repro_fold13.ml` | 0 | `.bench/xxvii/repro_fold13.log` |
| 4 | (earlier iterations of the same HOL script, kept for diagnosis) | 0 | `.bench/xxvii/repro_fold.log`, `repro_fold{3..12}.log` |

Only step 3 is the final reproduction; the earlier logs are kept because they show which rewrite
step fails, not because they are evidence for anything else.

Resource lines (verbatim, one per capped run):

```
cap.sh: group=2051031 rc=0 sampled_group_peak_rss_kb=1779456 max_process_vmhwm_kb=1685676   # bridge run
cap.sh: group=2087615 rc=0 sampled_group_peak_rss_kb=237200  max_process_vmhwm_kb=224760    # HOL run
```

`free -g` before each run: 79 GB available (≥ 3× the 12 G cap). Neither cap fired. `make -j1`, one
target at a time, never raised.

The instrumented bridge run is **proof-neutral**, exactly as in XXVI:
`STAMP_INFERENCES 5683733` (`.bench/xxvii/neg2.correct`), the same primitive-inference count as the
published baseline; `ORACLE close ok` ×10; `val BIGNUM_NEG_P25519_CORRECT` printed;
`Running time: 28.0 s`.

### 1.2 What the second instrumented run adds

`inject2.py` inserts two guarded dumps into a *scratch copy* of the emitted phase answer (the same
technique as `.bench/xxvi/inject.py`; no tracked file):

1. **entry dump** at `BRIDGE_FINISH_KEEP_ALG_TAC` (`tools/bridge.ml:3454`), *before* any split:
   every hypothesis of the word-level goal, with the bridge's own `bridge_int_eq_has_carry` and
   `bridge_is_direct_bound` classification and an `aconv` duplicate scan;
2. **pre-close dump** just before `w_eq` is built (`tools/bridge.ml:2542`): the `extra_eqs`, their
   pairwise `aconv`, and the Gauss-Jordan residual forms.

### 1.3 The input dump

`.bench/xxvi/bridge-dump.txt` (830 lines, 10 branches) is unchanged from XXVI; every index quoted
below is an antecedent index in that file.

---

## 2. The fixed frame of every leaf

All ten leaves share (indices except where noted):

- `#0 [aggregate]`: `Σs + N − B^4·&(bitval carry_s12) = p` (`Σs`, `N`, `B` as in §3.1);
- `#1 [agg_fact]`: `&(bitval carry_s12) = &0`;
- **26 direct bounds** — `&0 <= V`, `V < &B`; `&0 <= &(val sum_sj)`, `&(val sum_sj) < &B` for
  `j ∈ {4,6,9,12}`; `&0 <= &(val n_i)`, `&(val n_i) < &B` for `i ∈ {0,1,2,3}`; `&0 <= &(bitval c)`,
  `&(bitval c) <= &1` for `c ∈ {carry_s4,s6,s9,s12}`, where

  ```
  V = &(val (word_or (word_or (word_or n_0 n_1) n_2) n_3))
  ```

  These are **synthesised by `BRIDGE_ATOM_BOUNDS_TAC`** (`tools/bridge.ml:1299–1312`, from
  `BRIDGE_BOUND_NONNEG` / `BRIDGE_BOUND_VAL_HI` / `BRIDGE_BOUND_BITVAL_HI`), i.e. they are part of
  the close set but are *not* upstream hypotheses of the theorem;
- **4–5 relational** (`bridge_is_direct_bound` false) facts: the branch-defining ones.

`B = 2^64`, `p = 57896044618658097711785492504343953926634992332820282019728792003956564819949`.

---

## 3. Part 1 — the ten branches

### 3.1 The goal shapes actually present (correction to XXVI §2)

XXVI §2 compresses branches 1–6 as "goal shape A: `Σs = &0`". **That holds only for branches 1 and
2.** The raw dump gives a right-hand side that is a constant *minus* `N` in eight of the ten leaves:

```
Σs := ((B^3·&(val sum_s12) + B^2·&(val sum_s9)) + B·&(val sum_s6)) + &(val sum_s4)
N  := ((&(val n_0) + B·&(val n_1)) + B^2·&(val n_2)) + B^3·&(val n_3)
C  := p + 2^256 = 173688133855974293135356477513031861779904976998460846059186376011869694459885
```

| branches | `w'` (verbatim shape) | note |
|---|---|---|
| 1, 2 | `Σs = &0` | shape A of XXVI |
| 3, 4 | `Σs = (−N) + p` | |
| 5, 6 | `Σs = (−N) + C` | `C = p + 2^256` |
| 7, 8 | `&0 = (−N) + p` | shape B of XXVI |
| 9, 10 | `&0 = (−N) + C` | |

The `C` leaves are where the **256-bit** `word_sub (word p) x` of `ref_neg`
(`tools/bridge.ml:2886–2888`) took its own wrap branch, contributing `2^256`; that is a *different*
wrap from the 64-bit one of Part 2 and must not be conflated with it. `C = p + 2^256` was checked
numerically (`python3`, 3 lines, output `True`).

### 3.2 Final table

`prop` = a conjunction `P` and an AC-equivalent `¬Q` are both present.
`range` = `0 ≤ a` and `a + k = 0` (`k > 0`) are both present.
`const` = an equation reducing to `0 = k ≠ 0`.

| # | `bridge:pre-close` | ants | goal (`w'`) | prop (indices) | range (indices) | const | primary reason |
|---:|---:|---:|---|---|---|---|---|
| 1 | 69929370 | 31 | `Σs = 0` | **yes** #28 / #29 | no | no | prop: `~allzero` vs `allzero` (∧-AC) |
| 2 | 70027096 | 32 | `Σs = 0` | **yes** #2 / #28 | no | no | prop: literal `0 ≤ V` vs `~(0 ≤ V)` |
| 3 | 70784553 | 32 | `Σs = p − N` | no | no | no | **arithmetic**: `#0 + #1` is the goal |
| 4 | 71230391 | 33 | `Σs = p − N` | **yes** #2 / #28 | no | no | prop: literal complement |
| 5 | 72004957 | 32 | `Σs = p + 2^256 − N` | **yes** #28 / #31 (AC of `+`,`*`) | no | no | prop modulo polynomial AC |
| 6 | 72753005 | 33 | `Σs = p + 2^256 − N` | **yes** #2 / #28 (also #29 / #32) | no | no | prop: literal complement |
| 7 | 72964563 | 33 | `0 = p − N` | **yes** #29,#30 / #31 | no | no | prop: `allzero` vs `~allzero` (∧-AC) |
| 8 | 73070469 | 34 | `0 = p − N` | **yes** #2 / #28 | **yes** #2 / #32,#33 | no | prop **and** range (same leaf) |
| 9 | 73169431 | 33 | `0 = p + 2^256 − N` | **yes** #29,#30 / #31 (also #28 / #32) | no | no | prop: `allzero` vs `~allzero` |
| 10 | 73276171 | 34 | `0 = p + 2^256 − N` | **yes** #2 / #28 (also #29 / #31) | **yes** #2 / #32,#33 | no | prop **and** range (same leaf) |

Totals: **9 prop, 2 range (both already prop), 0 constant, 1 arithmetic, 0 witness.**

### 3.3 Branch by branch, with the contradicting terms

- **B1** (`~allzero` #28 vs `allzero` #29). `#28` is
  `~((((&(val n_0) = &0 /\ &(val n_1) = &0) /\ &(val n_2) = &0) /\ &(val n_3) = &0))`,
  `#29` is `&(val n_0) = &0 /\ &(val n_1) = &0 /\ &(val n_2) = &0 /\ &(val n_3) = &0`. Same
  conjuncts in the same order, differing only in `∧`-association → `CONJ_ACI`-equivalent.
  Executed independently in IX.93 (`research/receipts/ix93-branch-certificate-probe.md` §2, ants
  `#28`/`#29`, 108 inferences).
- **B2** (`#2` vs `#28`). `#2` is `&0 <= V`; `#28` is `~(&0 <= V)` — the *same* atom, literal
  complement, no AC needed.
- **B3** — the one **consistent** leaf, and the only arithmetic one. Relational set:
  `#28` = `N_desc <= p`, `#29`/`#30` = `~allzero` (both `∧`-associations), `#31` = `N_asc <= p`.
  All four are satisfiable together, so there is no logical contradiction to find: both
  all-zero facts are negative and both `N ≤ p` facts are positive (and AC-equal to each other).
  The leaf closes by a **linear identity**: `w'` is `Σs = (−N) + p`, `#0` is
  `Σs + N − B^4·&(bitval carry_s12) = p`, and `#1` is `carry_s12 = 0`; substituting `#1` into `#0`
  gives `Σs + N = p`, i.e. `Σs = p − N` — the goal term for term (same monomials, same
  coefficients). `INT_ARITH` therefore closes B3 from **`#0` and `#1` alone**; the four relational
  facts `#28`–`#31` are inert. The atoms involved are `&(val sum_s4)`, `&(val sum_s6)`,
  `&(val sum_s9)`, `&(val sum_s12)`, `&(val n_0)` … `&(val n_3)` and `&(bitval carry_s12)` (the last
  eliminated by `#1`), with `B` and `p` as numerals — hence "requires arithmetic" (linear
  recombination), not a contradiction.
- **B4** (`#2` vs `#28` literal, and that is the only contradiction here). `#29` and `#32` are both
  *positive* `N ≤ p` (AC-equal to each other), and `#30`/`#31` are the negative
  `V + 2^64 = 0` and the negative all-zero conjunction — no pair of them contradicts.
- **B5** (`#28` vs `#31`, **polynomial AC**).
  `#28 = ~((((B^3·v3 + B^2·v2) + B·v1) + v0) <= p)`,
  `#31 = (((v0 + B·v1) + B^2·v2) + B^3·v3) <= p`. Same monomials, different nesting/order. This is a
  propositional contradiction only after the `+`/`*` flattening that `bridge_flat_conv`
  (`tools/bridge.ml:1867–1882`) performs; a certificate that only does `CONJ_ACI` would **not** see
  it. Both all-zero facts in B5 are negative.
- **B6** (`#2` vs `#28` literal; also `#29` vs `#32` polynomial AC).
- **B7** (`#29`,`#30` all-zero positive vs `#31` all-zero negative, ∧-AC). Executed in IX.93
  (§2, ants `#29`/`#31`, 108 inferences).
- **B8** (`#2` vs `#28` literal). Apart from `#28`, the relational facts are two positive `N ≤ p`
  (`#29`, `#31`) and one negative all-zero (`#30`), so this is the cleanest leaf of the ten.
  Additionally the range contradiction `#2 : &0 <= V` with
  `#32`,`#33 : V + &18446744073709551616 = &0` — this is the leaf examined in Part 2.
- **B9** (`#29`,`#30` all-zero positive vs `#31` negative; and `#28` negative `N ≤ p` vs `#32`
  positive `N ≤ p`, polynomial AC).
- **B10** (`#2` vs `#28`; and `#29` vs `#31` polynomial AC; and the range pair `#2` / `#32`,`#33`).

**On the brief's operational definition, B8 and B10 are range-inconsistent (indices `#2` with
`#32`,`#33`).** The qualification is that the `a + k = 0` member is bridge-generated (Part 2), not an
upstream premise; no branch has a range inconsistency inherited from the theorem. No branch contains
a constant inconsistency, and **no branch needs a witness**: on nine of ten leaves the close is
vacuous, and on the tenth (B3) it is the aggregate identity.

### 3.4 Deduced vs executed

- **All ten classifications above are deductions from the dump** (term comparison of the antecedent
  lists, `∧`/polynomial AC by inspection). No proof was run for the Part 1 table in this session.
- **Independently executed:** B1 and B7's propositional contradiction (IX.93, committed receipt:
  `full`/`slim`/`min`/`cert` runs, `cert` at 108 inferences, `STAMP_INFERENCES 5576042`); and the
  *closing* of all ten leaves (`ORACLE close ok` ×10 in `.bench/xxvii/neg2.correct`, unchanged
  `STAMP_INFERENCES 5683733`).
- **Executed in this session:** the unsatisfiability of `V + &2^64 = &0` for `val V ≥ 0`
  (Part 2, `STEP7`), and the production of that equation from the machine's zero test
  (`STEP1`–`STEP5`).

---

## 4. Part 2 — branches 8 and 10: `&(val (word_or …)) + &18446744073709551616 = &0`

### 4.1 The observable

`.bench/xxvi/bridge-dump.txt` branch 8 (`73070469`, 34 antecedents):

```
  #32 [extra_eq]: &(val (word_or (word_or (word_or n_0 n_1) n_2) n_3)) + &18446744073709551616 = &0
  #33 [extra_eq]: &(val (word_or (word_or (word_or n_0 n_1) n_2) n_3)) + &18446744073709551616 = &0
```

`resids = 2`, `defs = 4`, `extra_eqs = 2`. Branch 10 (`73276171`) is the same. The other eight
leaves have `extra_eqs = 0`, `resids = 0`.

Reproduced in the fresh run (`.bench/xxvii/probe.txt`, executed):

```
----- PRE-CLOSE EXTRAS -----
EXTRAS 2 RESIDS 2 DEFS 4
  E#0: &(val (word_or (word_or (word_or n_0 n_1) n_2) n_3)) + &18446744073709551616 = &0
  E#1: &(val (word_or (word_or (word_or n_0 n_1) n_2) n_3)) + &18446744073709551616 = &0
  EXTRA-ACONV 0 1
  RESID#0: &(val (word_or (word_or (word_or n_0 n_1) n_2) n_3)) = -- &18446744073709551616
  RESID#1: &(val (word_or (word_or (word_or n_0 n_1) n_2) n_3)) = -- &18446744073709551616
```

and in the bridge's own slice notes (`.bench/xxvii/neg2.correct`, two of ten leaves):

```
ORACLE system defs=4 residual=2 facts=1 aggregates=1
ORACLE extra-eqs=2 [&(val (word_or (word_or (word_or n_0 n_1) n_2) n_3)) + &18446744073709551616 =
&0 | &(val (word_or (word_or (word_or n_0 n_1) n_2) n_3)) + &18446744073709551616 =
&0]
ORACLE close ants=34 size=7366
```

### 4.2 Where the equation comes from — the code path

**(i) The condition is the ARM `CMP` zero flag, not an OR-zero equation.** The entry dump
(`.bench/xxvii/probe.txt`, executed; one entry per proof, `…` marks an elided repetition) contains
the written limbs as

```
GOAL: val (word_join (word_join (word_join
        (if ~(val (word_sub (word_or (word_or (word_or n_0 n_1) n_2) n_3) (word 0)) = 0)
         then sum_s12 else word 0) … )) = val (if word_join (…) = word 0 then word 0
                                                 else word_sub (word p_25519) (word_join (…)))
```

i.e. every written limb is selected by

```
C ≔ ~(val (word_sub (word_or (word_or (word_or n_0 n_1) n_2) n_3) (word 0)) = 0)
```

with four copies (one per limb). That term is produced by the machine model:

- `arm_CSEL` (`.bench/s2n-bignum/arm/proofs/instruction.ml:1177–1181`) is
  `Rd := if condition_semantics cc s then read Rn s else read Rm s`;
- `condition_semantics Condition_NE s ⇔ ~read ZF s` (same file, `:553–562`);
- `arm_CMP Rm Rn = arm_SUBS ZR Rm Rn` (`:3054`), and `arm_SUBS`
  (`:1752–1762`) sets `d = word_sub m n` and `ZF := (val d = 0)`;
- the instruction is `arm_CMP X6 XZR` (`.bench/s2n-bignum/arm/proofs/bignum_neg_p25519.ml:29`), with
  `X6` the `ORR` chain of the four limbs (`:19`, `:24`, `:27`), and the four selects are the four
  `arm_CSEL … Condition_NE` at `:30`–`:33`.

So `C` is literally "the OR of the limbs is non-zero", computed the way the hardware computes it:
by a `word_sub … (word 0)` and a zero flag. The 4 × `word_sub` copies are why the condition occurs
four times in one goal.

**(ii) `C` is not what the zero fold handles.**
`BRIDGE_ZERO_FOLD_TAC` (`tools/bridge.ml:2083–2087`) reduces zero tests with
`INT_VAL_WORD_OR_EQ_0` (`:2074–2081`), whose only shape is

```
(int_of_num(val(word_or x y)) = &0) <=> (&(val x) = &0 /\ &(val y) = &0)
```

`BRIDGE_ZERO_TEST_TAC` (`:2057–2063`) adds the word-level `WORD_OR_EQ_0` and
`BRIDGE_JOIN_ZERO_CONV`. **Neither has a rule for `word_sub x (word 0)`:** `WORD_SUB_EQ_0`
(`.bench/hol-light/Library/words.ml:4782`) occurs nowhere in `tools/bridge.ml`, and `VAL_EQ_0`
(`:668`) appears only as `GSYM VAL_EQ_0` in the *normaliser* (`:1104`, `:1121`), never in either
zero-test tactic. The human proof does use the pair — `REWRITE_TAC[VAL_EQ_0; WORD_SUB_EQ_0;
WORD_OR_EQ_0]` (`bignum_neg_p25519.ml:82`) — and so eliminates the `word_sub` before any wrap
expansion can happen.

**(iii) Without that step, the normaliser expands the subtraction's wrap.**
`BRIDGE_ALG_NORM_ASSUM` (`tools/bridge.ml:1102–1115`, line 1109) rewrites with
`INT_VAL_WORD_SUB_CASES` (`.bench/hol-light/Library/words.ml:2330–2338`):

```
&(val(word_sub x y)) = if &(val y) <= &(val x) then &(val x) − &(val y)
                       else &(val x) + (&2 pow dimindex(:N) − &(val y))
```

Of the `*_SUB_CASES` sites in `tools/bridge.ml`, only this one is applied by the normaliser
(`:1109`, `:1125`); `VAL_WORD_SUB_CASES` (`:2866`–`:2869`, `:2902`) occurs inside the proofs of the
reference lemmas, never on a goal or hypothesis. So this is the rule that produced the dumped
numeral. The exact rewrite *order* inside `BRIDGE_ALG_NORM_ASSUM` (it rewrites `GSYM VAL_EQ_0` first,
`:1104`, then converts num→int, then applies this at `:1109`) is not instrumented, and is not needed
for the verdict.

Executed (`STEP1`/`STEP2`, `.bench/xxvii/repro_fold13.log`), at `x` an or-chain and `y = word 0`:

```
&(val (word_sub (word_or (word_or (word_or n0 n1) n2) n3) (word 0))) =
(if &0 <= &(val (word_or (word_or (word_or n0 n1) n2) n3))
 then &(val (word_or (word_or (word_or n0 n1) n2) n3)) - &0
 else &(val (word_or (word_or (word_or n0 n1) n2) n3)) + &2 pow 64 - &0)
```

**(iv) The splitter then keeps the wrap branch as a hypothesis.**
`BRIDGE_SPLIT_CONDITIONS_KEEP_TAC` (`tools/bridge.ml:641–655`) does `ASM_CASES_TAC c` on every
conditional's condition and *retains the branch fact* (`ASSUME_TAC fact`, line 650; the false branch
is `EQF_INTRO (ASSUME (mk_neg c))`, line 654). It runs twice: once at the head of
`BRIDGE_FINISH_KEEP_ALG_TAC` (`:3454–3462`, on the un-normalised goal) and once inside
`BRIDGE_ALG_TAC` ("split-conditions", `:2618`), i.e. after the `INT_VAL_WORD_SUB_CASES` expansion
has exposed the integer guard `&0 <= &(val V)`.

**(v) In the wrap branch the flag's zero equation is exactly the dumped equation.**
With the branch assumption `~(&0 <= &(val V))` and `INT_SUB_RZERO` — literally the first step of
`BRIDGE_ZERO_FOLD_TAC`, `tools/bridge.ml:2084` — the cases instance's else value is

```
&(val (word_sub V (word 0))) = &(val V) + &18446744073709551616
```

Executed (`STEP3`, `STEP4`, `STEP5` of `.bench/xxvii/repro_fold13.ml`):

```
STEP3  &2 pow 64 = &18446744073709551616
STEP4  &(val (word_sub x (word 0))) =
       (if &0 <= &(val x) then &(val x) - &0 else &(val x) + &18446744073709551616 - &0)
STEP5  forall x. ~(&0 <= &(val x))
         ==> &(val (word_sub x (word 0))) = &(val x) + &18446744073709551616
```

so the fact kept in the wrap branch — "the flag says zero" — *is* `&(val V) + &2^64 = &0`.

**(vi) Why it lands in `extra_eqs` rather than in the aggregate.**
`bridge_int_eq_has_carry` (`tools/bridge.ml:2319–2325`) classifies an integer equation as a chain
limb only if it carries a `&(bitval _)` atom; an equation with none is a branch fact. This one has
none, so it is not a limb of any carry chain (no aggregate takes it as a premise — `bridge_aggregates`
at `:2501` only builds chains out of `bitval`-carrying limb equations) and line 2533–2535 puts it in
`extra_eqs` verbatim. In the run, `defs = 4` is exactly the four carries
(`.bench/xxvii/neg2.correct:21942–21963`: `ORACLE def &(bitval carry_s4)` … `carry_s12`), i.e. the
`val V` atom is *not* eliminated: it stays in the two residual rows.

### 4.3 Verdict: **(a)**, with one qualification

The equation is a **real branch condition of the machine model that is simply impossible** — the
wrap branch of the subtraction that implements `CMP X6, XZR`. It is **not** (b): no `bitval` is
mis-folded anywhere on this path. All four `&(bitval carry_s*)` atoms survive to the close as atoms
(their bounds are in the dump), `BRIDGE_DECIDE_BITVALS_TAC` (`:1144–1162`) only substitutes a
`bitval` when the *boolean* is already a hypothesis (none of the four is), and the equation contains
no `bitval` at all.

The executable qualification: the branch is refutable twice over, and the bridge refutes it for a
*different* reason than the one the equation suggests —
`~(&0 <= &(val V))` contradicts the bound `&0 <= &(val V)` that `BRIDGE_ATOM_BOUNDS_TAC` itself adds
(executed, `STEP6`: `|- !x:int64. ~(&0 <= &(val x)) ==> F`), so the `+2^64` equation is *inert* in
the final close. It is dead weight, not a proof hazard: **nothing is unsound**.

### 4.4 The two copies, and `resids = 2`

- **The two copies are the same term.** Executed: `EXTRA-ACONV 0 1` in `.bench/xxvii/probe.txt`
  (`aconv` on the two `asl` conclusions). The printing is byte-identical too, including the
  `word_or` association and the numeral, so there is no association difference to exploit.
- **`resids = 2` is two rows for two hypotheses, not two distinct facts.**
  `bridge_int_hyps` (`tools/bridge.ml:2441–2446`) only filters `asl`; it does not `setify`.
  `all_hyps` (`:2503`) therefore keeps both, `extra_eqs` (`:2533–2535`) keeps both, and
  `bridge_gauss_jordan` (`:1649–1673`) removes only the row it uses as a pivot
  (`others = List.filter (fun (j,_,_) -> j <> i) rows`) and keeps every other row — there is no
  de-duplication anywhere on this path. The residue of each of the two rows is
  `&(val V) = -- &18446744073709551616` (executed, `RESID#0`/`RESID#1`), which is the same equation
  in `l = r` normal form.
- **Both copies are bridge-generated, not simulator hypotheses.** Executed: the entry dump of
  `BRIDGE_FINISH_KEEP_ALG_TAC` (`.bench/xxvii/probe.txt`) has **7** hypotheses and none of them is
  this equation:

  ```
  H#0..H#4 : the five limb equations (e.g. H#4: --(&2 pow 64) * &(bitval carry_s4) + &(val sum_s4)
             = &18446744073709551597 - &(val n_0))
  H#5      : true
  H#6      : nonoverlapping_modulo (2 EXP 64) (pc,80) (val z,8 * 4)
  ```

  So the equation appears *between* the entry dump and the pre-close dump: it is produced by
  `BRIDGE_SPLIT_CONDITIONS_KEEP_TAC` + the `INT_VAL_WORD_SUB_CASES` normalisation, and it is kept
  because a split fact is kept. A plausible account of the *duplication* (not a measurement) is that
  the pre-split goal contains the condition `C` four times, and the split tactic assumes a fact per
  distinct condition term per pass, in a two-pass schedule (§4.2 iv). The measurements that would
  turn this into a determination are listed in §4.6.

### 4.5 Reproduction (executed)

Script: `.bench/xxvii/repro_fold13.ml`; log: `.bench/xxvii/repro_fold13.log`; run under
`tools/cap.sh --timeout 900 12G` in the scratch counter `.bench/hol-light-stamp2` (peak RSS 237 MB).
All seven steps succeed:

| step | statement proved | result |
|---:|---|---|
| 1 | `INT_VAL_WORD_SUB_CASES` at `(or-chain, word 0) : int64` | the two-branch formula (§4.2 iii) |
| 2 | the same after `VAL_WORD_0` and `dimindex(:64)` | else value `&(val V) + &2 pow 64 - &0` |
| 3 | `&2 pow 64 = (&18446744073709551616:int)` | the numeral in the dump |
| 4 | the instance carrying the numeral | `… else &(val V) + &18446744073709551616 - &0` |
| 5 | `~(&0 <= &(val x)) ==> &(val (word_sub x (word 0))) = &(val x) + &18446744073709551616` | **the fold** |
| 6 | `!x:int64. ~(&0 <= &(val x)) ==> F` | the wrap guard is refutable |
| 7 | `!x:int64. ~(&(val x) + &18446744073709551616 = &0)` | the equation is range-unsatisfiable |

Note for anyone re-running: in this HOL Light a bare `&2 pow 64 = &18446744073709551616` parses at
`:real`; the annotations in the script are load-bearing, and `INT_OF_NUM_POW` has the orientation
`(&x) pow n = &(x EXP n)` (so `REWRITE_TAC[INT_OF_NUM_POW]`, not `GSYM`), followed by
`CONV_TAC NUM_REDUCE_CONV` and `REFL_TAC`.

### 4.6 Not determined (and what would settle it)

The **root cause of the equation** is determined: hypothesis **(a)**, §4.2–§4.3, executed. What is
**not** determined is *which two* `ASSUME_TAC` events deposit the two identical hypotheses: the
pre-split state has four occurrences of one condition and the pipeline splits in two passes, but
`probe.txt` shows only the *result*, not the per-split fact log. Three observations would settle it:

1. log every `ASM_CASES_TAC c` / `ASSUME_TAC fact` inside `BRIDGE_SPLIT_CONDITIONS_KEEP_TAC`
   (condition term, leaf ordinal, pass number);
2. the `asl` length before and after each pass of the splitter, per leaf;
3. an `aconv` scan of `asl` after each `BRIDGE_ALG_NORM_ASSUM` (which is where the `if` first
   appears in a hypothesis).

None of this changes the verdict, the classification, or the fix in Part 3.

---

## 5. Part 3 — implication for the OR-reduction selector

### 5.1 Can the selector be built on this folding as it stands?

**No. The fold must be fixed first.** The reason is not soundness (the leaves close, and the
impossible branch is refuted propositionally) but that the reduction
`q = [n ≠ 0]` via `BIGNUM_OF_WORDLIST_EQ_0` / `VAL_EQ_0` / `WORD_SUB_EQ_0` / `WORD_OR_EQ_0` is
*never performed on the machine's zero test*:

1. the machine's test is presented as `val (word_sub (or-chain) (word 0)) = 0` (§4.2 i), and no
   bridge tactic rewrites `word_sub x y = word 0` (`WORD_SUB_EQ_0` is absent from
   `tools/bridge.ml`; `VAL_EQ_0` appears only as `GSYM VAL_EQ_0` in `BRIDGE_ALG_NORM_ASSUM`);
2. so instead of a per-limb conjunction the pipeline gets the *wrap conditional* of
   `INT_VAL_WORD_SUB_CASES`, splits it, and explores a branch that cannot exist;
3. the OR reduction (`INT_VAL_WORD_OR_EQ_0`) only fires *inside* the surviving sub-branch, i.e.
   after the split, on `&(val V) = &0` — which is why the leaves contain
   `~((((n_0=0 ∧ n_1=0) ∧ n_2=0) ∧ n_3=0))` (the machine's OR form, left-nested) next to
   `n_0=0 ∧ … ∧ n_3=0` (the reference's join form, right-nested): the reduction is present but as a
   *by-product of a case split*, not as "the selector's value is decided".

Consequences for a selector built on this as it stands: one extra leaf per zero test (the two-pass
split), a dead equation in `extra_eqs` and in the Jordan residue on the two wrap leaves, and no
usable statement of the form `q = [n ≠ 0]` before the split. It works for `neg` only because the
impossible branch is vacuous and the true branch happens to carry both zero-test forms.

### 5.2 Exactly what has to change

Add the missing zero-test step, in the *human proof's* order, to the bridge's fold, before
`INT_VAL_WORD_SUB_CASES` can expand the subtraction:

1. **`BRIDGE_ZERO_TEST_TAC` (`tools/bridge.ml:2057–2063`).** Add `VAL_EQ_0` and `WORD_SUB_EQ_0` to
   the rule list, ahead of `WORD_OR_EQ_0`:

   ```ocaml
   (RULE_ASSUM_TAC(REWRITE_RULE[VAL_EQ_0; WORD_SUB_EQ_0; WORD_OR_EQ_0]) THEN
    REWRITE_TAC[VAL_EQ_0; WORD_SUB_EQ_0; WORD_OR_EQ_0] THEN
    … BRIDGE_JOIN_ZERO_CONV …)
   ```

   `VAL_EQ_0` (`words.ml:668`) turns `val w = 0` into `w = word 0`; `WORD_SUB_EQ_0`
   (`words.ml:4782`) turns `word_sub x y = word 0` into `x = y`, so
   `val (word_sub X (word 0)) = 0` becomes `X = word 0` and the existing `WORD_OR_EQ_0` reduces the
   OR chain to per-limb equations. Both lemmas are side-condition-free and width-polymorphic.
2. **`BRIDGE_ZERO_FOLD_TAC` (`tools/bridge.ml:2083–2087`), int level.** Add the matching int lemma
   and use it *before* `INT_SUB_RZERO`/`INT_VAL_WORD_OR_EQ_0`, e.g.

   ```ocaml
   let INT_VAL_WORD_SUB_ZERO = prove
    (`!x y:(N)word. (int_of_num(val(word_sub x y)) = &0) <=> (val x = val y)`, …)
   ```

   so `&(val (word_sub V (word 0))) = &0` becomes `val V = 0` and then
   `INT_VAL_WORD_OR_EQ_0` splits it per limb. (Equivalently: guard `INT_VAL_WORD_SUB_CASES` in
   `BRIDGE_ALG_NORM_ASSUM` so that it never fires on a term that is the `val` node of a zero test.)
3. Only then can the selector's rewrite chain (`BIGNUM_OF_WORDLIST_EQ_0`, `VAL_EQ_0`,
   `WORD_SUB_EQ_0`, `WORD_OR_EQ_0`) be reproduced structurally: `q` becomes a finite conjunction of
   per-limb word equations, the case split on the wrap disappears, and `extra_eqs` stops receiving a
   `+2^64` equation.

With that in place, the *classification* of Part 1 does not change (the leaves are already vacuous),
but the branches 8/10 leaf becomes the same shape as branches 2/4/6 and the per-limb zero facts are
available *before* the split, which is what a selector needs.

---

## 6. Reproduction

```sh
# 1. instrument a scratch copy of the phase answer (no tracked edit)
python3 .bench/xxvii/inject2.py
# 2. assemble + one capped proof; BRIDGE_DUMP2 is set inside build2.sh
bash .bench/xxvii/build2.sh
#    -> .bench/xxvii/{neg2.log,neg2.correct,probe.txt}
# 3. the standalone fold reproduction
cd .bench/hol-light-stamp2 && \
  ../../tools/cap.sh --timeout 900 12G -- ./hol.sh < ../xxvii/repro_fold13.ml
#    -> .bench/xxvii/repro_fold13.log
```

Scratch artefacts (all under the git-ignored `.bench/`, none tracked):
`.bench/xxvii/{inject2.py,build2.sh,neg2.answer,neg2.log,neg2.correct,probe.txt,
repro_fold.ml,repro_fold{,3..13}.ml,repro_fold{,3..13}.log}`.
The dropin build also rewrote `.bench/dropin/arm/proofs/bignum_neg_p25519.ml`,
`.bench/dropin/arm/curve25519/bignum_neg_p25519.{native,correct}` — the same scratch tree XXVI used.

---

**Verdict.** Ten branches classified: 9 are propositionally inconsistent (one of them, B5, only
after `+`/`*`-AC flattening), 2 of those are additionally range-inconsistent, 0 are
constant-inconsistent, 1 (B3) is a linear identity from the aggregate and the carry fact, and none
requires a witness. Branches 8/10 carry `&(val V) + &2^64 = &0` twice because the ARM `CMP` zero
test is `val (word_sub X (word 0)) = 0`, the bridge has no `WORD_SUB_EQ_0` step, and
`INT_VAL_WORD_SUB_CASES` therefore exposes — and the splitter then keeps — the impossible wrap
branch: **(a)**, not (b). The OR-reduction selector cannot be built on this folding as it stands;
`BRIDGE_ZERO_TEST_TAC`/`BRIDGE_ZERO_FOLD_TAC` need the `VAL_EQ_0; WORD_SUB_EQ_0` step (§5.2).
