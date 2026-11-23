# R4 — reference route for ARM `add`/`sub`/`neg` (V8.18), 16/18

Run date: **2026-11-23** (UTC), branch `staging`, base commit `ee3e4f9`.
Nothing was committed or pushed. Free disk at the end: **25 GB** (floor 3 GB).

Same declared set as R3: every `arm.bignum_(add|sub|neg)_<family>.` functional-correctness
task, **N = 18** (6 curves × 3 operations). No task was added or removed: the 18 task ids in
`r4-verdicts.csv` are byte-identical to the 18 in `r3-results.tsv` (`diff` empty), and all 18
carry the same 120 s per-task budget in `timeout-map.json`.

## 0. Scope: this is development, not a held-out claim

All 18 tasks had prior visible outcomes in this programme (the R2 tasks, the four P-256/P-384
bridge tasks, and the five R2 training tasks under `.bench/`). Nothing here is blind, and no
number in this report is a held-out result. The denominator is **18**, not 898, and must not be
compared with any other system's rate.

## 1. What was built

### 1.1 The two reference functions

```hol
ref_sub (pw:W word) (x:W word) (y:W word) =
  let d = word_sub x y in
  if word_ult x y then word_add d pw else d

ref_neg (pw:W word) (x:W word) =
  if x = word 0 then word 0 else word_sub pw x
```

`ref_sub` is subtract-then-add-`pw`-on-borrow; `ref_neg` is `pw - x` with `0` at zero. Neither
mentions a modulus, a curve, a limb count or a width.

### 1.2 The lemmas (all proved in a fresh process)

```
SUB_REF_VAL      !pw x y. 0 < val pw /\ val x < val pw /\ val y < val pw
                 ==> val(ref_sub pw x y) = (val x + val pw - val y) MOD val pw
SUB_REF_VAL_WORD !p x y.  0 < p /\ p < 2 EXP dimindex(:W) /\ val x < p /\ val y < p
                 ==> val(ref_sub (word p) x y) = (val x + p - val y) MOD p
NEG_REF_VAL      !pw x.   0 < val pw /\ val x <= val pw
                 ==> val(ref_neg pw x) = (val pw - val x) MOD val pw
NEG_REF_VAL_WORD !p x.    0 < p /\ p < 2 EXP dimindex(:W) /\ val x <= p
                 ==> val(ref_neg (word p) x) = (p - val x) MOD p
SUB_INT_REM_MOD  !p m n.  0 < p /\ m < p /\ n < p
                 ==> ((&m:int) - &n) rem &p = &((m + p - n) MOD p)
```

Supporting lemmas, all generic in `p`/width:

```
MOD_ADD_SELF   !a p. (a + p) MOD p = a MOD p
SUB_REARRANGE  !m n p. n <= p ==> (m + p) - n = m + (p - n)
REF_SUB_KEEP   !a b p. b <= a /\ a < p ==> a - b = (a + p - b) MOD p
REF_SUB_BORROW !a b p t s. a < b /\ b < p /\ p < t /\ s = a + (t - b)
               ==> (s + p) MOD t = (a + p - b) MOD p
```

`SUB_INT_REM_MOD` is the only bridge between the benchmark's `int` `rem` postcondition and the
`num` residue; `SUB_REF_VAL`/`NEG_REF_VAL` are the only lemmas that carry `MOD` on the
machine side, exactly as `ADD_REF_VAL` already did.

Fresh process (`research/receipts/bridge/ref_check.ml`, log `ref_check.log`):

```
AXIOMS before=3
AXIOMS after=3
REF_HYPS ref_sub=0 SUB_REF_VAL=0 SUB_REF_VAL_WORD=0 NEG_REF_VAL=0 NEG_REF_VAL_WORD=0 SUB_INT_REM_MOD=0
REF_CHECK_DONE seconds=2.726
```

Whole-file fresh load of `tools/bridge.ml` through `research/receipts/bridge/check_fresh.ml`:
**81/81** bindings `BRIDGE_CHECK ... OK`, `AXIOMS before=3 after=3`
(`r4-bridge-fresh.log`, 2.745 s; the full multi-megabyte toplevel log stays in
`.bench/ref-dev/bridge_fresh3.log`). 37 of the 81 bindings are theorems — 34 proved
statements plus the three new definitions `ref_add`, `ref_sub`, `ref_neg`; the rest are lemma
lists, conversions and tactics that type-check only.

### 1.3 Mechanical no-curve-constant check

`python3 research/receipts/bridge/curve_check.py tools/bridge.ml`
(`r4-curve-check.log`): **`statements=45 hits=0`**, exit 0. No curve name, no numeral of five
or more digits, no `+ 19` / `2 EXP 255` / `2 EXP 384` / `2 EXP 521` / hex shape appears in any
lemma statement in `tools/bridge.ml`.

### 1.4 The tactics

`BRIDGE_SUB_REF_TAC` and `BRIDGE_NEG_REF_TAC` mirror `BRIDGE_ADD_REF_TAC`:

1. eliminate the digit-equation assumptions and rewrite the modulus constant to its numeral,
2. build the word for each operand and for the written result from their flat digit sums
   (`BRIDGE_WORD_OF_DIGITS`),
3. discharge the range hypotheses from the context (`BRIDGE_FIND_BOUND`),
4. rewrite the postcondition through `SUB_INT_REM_MOD` / `NEG_REF_VAL_WORD` so the residual goal
   is `val(written) = val(ref_{op} (word p) x y)`,
5. case-split the `ref_*` conditionals (`BRIDGE_FINISH_KEEP_TAC`) and close each branch with
   certified SAT (`BRIDGE_CSAT_TAC` -> CaDiCaL -> LRAT replayed in the kernel).

The residual instance was measured, not assumed (`r4-csat-instance.log`: one of the four branches
of `sub_p25519`, printed from a copy of `tools/bridge.ml` with one `Printf` added inside
`BRIDGE_CSAT_TAC`; same answer bytes, same tactic):

```
CSAT_INSTANCE hyps=17 goal=&(val (word_join (word_join (word_join
  (word_and sum_s14 (word 9223372036854775807)) sum_s13) sum_s12) sum_s11)) =
 &(val (word_add (word_sub (word_join (word_join (word_join m_3 m_2) m_1) m_0)
                   (word_join (word_join (word_join n_3 n_2) n_1) n_0))
                 (word 57896044618658097711785492504343953926634992332820282019728792003956564819949)))
CSAT_HYP &(val m_0) + &(val (word_join m_1 (word 0))) + &(val (word_join m_2 (word 0)))
         + &(val (word_join m_3 (word 0))) < &<p>
CSAT_HYP &(val n_0) + &(val (word_join n_1 (word 0))) + &(val (word_join n_2 (word 0)))
         + &(val (word_join n_3 (word 0))) < &<p>
CSAT_HYP word_ult (join m) (join n)
CSAT_HYP -- &(val (word_join (word (bitval carry_s14)) (word 0))) + &(val sum_s14) =
           &(val sum_s8) - &0 - &(bitval carry_s13)          (* x8 accumulator equations *)
...
```

i.e. the written words against the borrow branch of `ref_sub (word p) x y`, over unsigned
words; `x < p` and `y < p` are present as word-valued hypotheses, the `ref_sub` condition
appears as `word_ult (join m) (join n)`, and every carry appears only through its accumulator
relation to word values. No flag meaning and no curve comparison is in the instance.

### 1.5 Two defects found and fixed in the route

* **Numeral representation.** The modulus can reach the goal as `NUMERAL(BIT1(...))` or as the
  same bit chain with the tag stripped. Both print as the decimal literal, so the mismatch is
  invisible in a log and `aconv` misses the range hypothesis (measured on `neg_p25519`: the
  hypothesis is present, `aconv` false, `is_numeral` false). `BRIDGE_TERM_EQ` /
  `BRIDGE_FIND_BOUND` compare modulo the tag and re-tag the goal before use; the word-level
  side conditions are then arithmetic on a numeral.
* **`-` binds tighter than `+` in HOL Light**, so `a + p - b` is `a + (p - b)`. The separation
  lemmas produce `(a + p) - b`; `SUB_REARRANGE` is the explicit bridge. A statement written in
  the wrong shape is *not* the same theorem and would have been a silent weakening, so every
  statement above is checked against the form the rewrite actually meets.

## 2. The 18 declared tasks

Judge directory `.bench/eval-20261123_163250`; verdicts
`python3 collect-verdicts.py eval-20261123_163250 ref-verdicts.csv` wrote
**`OK,16` / `FAIL,2`** into the CSV's summary rows (`r4-verdicts.csv`; a second collection
into a second CSV is byte-identical). "seconds" is the harness `PROFILE` elapsed.

| task | route | verdict | seconds | R3 |
|---|---|---|---:|---|
| `arm.bignum_add_p25519` | ref (`BRIDGE_ADD_REF_TAC`) | **OK** | 22.681 | FAIL |
| `arm.bignum_add_p256` | ref (`BRIDGE_ADD_REF_TAC`) | **OK** | 23.095 | OK |
| `arm.bignum_add_p256k1` | ref (`BRIDGE_ADD_REF_TAC`) | **FAIL** | 10.849 | FAIL |
| `arm.bignum_add_p384` | ref (`BRIDGE_ADD_REF_TAC`) | **FAIL** | 7.249 | OK (*) |
| `arm.bignum_add_p521` | ref (`BRIDGE_ADD_REF_TAC`) | **OK** | 95.864 | FAIL |
| `arm.bignum_add_sm2` | ref (`BRIDGE_ADD_REF_TAC`) | **OK** | 21.959 | OK |
| `arm.bignum_neg_p25519` | ref (`BRIDGE_NEG_REF_TAC`) | **OK** | 5.254 | FAIL |
| `arm.bignum_neg_p256` | ref (`BRIDGE_NEG_REF_TAC`) | **OK** | 6.551 | OK |
| `arm.bignum_neg_p256k1` | ref (`BRIDGE_NEG_REF_TAC`) | **OK** | 6.669 | OK |
| `arm.bignum_neg_p384` | ref (`BRIDGE_NEG_REF_TAC`) | **OK** | 12.798 | OK |
| `arm.bignum_neg_p521` | ref (`BRIDGE_NEG_REF_TAC`) | **OK** | 19.527 | no answer |
| `arm.bignum_neg_sm2` | ref (`BRIDGE_NEG_REF_TAC`) | **OK** | 6.706 | OK |
| `arm.bignum_sub_p25519` | ref (`BRIDGE_SUB_REF_TAC`) | **OK** | 15.668 | FAIL |
| `arm.bignum_sub_p256` | ref (`BRIDGE_SUB_REF_TAC`) | **OK** | 14.536 | OK |
| `arm.bignum_sub_p256k1` | ref (`BRIDGE_SUB_REF_TAC`) | **OK** | 15.697 | FAIL |
| `arm.bignum_sub_p384` | ref (`BRIDGE_SUB_REF_TAC`) | **OK** | 31.107 | OK |
| `arm.bignum_sub_p521` | ref (`BRIDGE_SUB_REF_TAC`) | **OK** | 35.088 | FAIL |
| `arm.bignum_sub_sm2` | ref (`BRIDGE_SUB_REF_TAC`) | **OK** | 15.085 | OK |

Max 95.864 s against the 120 s per-task budget; no timeout. Machine-readable:
`r4-verdicts.csv`, `r4-profile.txt`, `r4-answers.sha256`.

### Per curve

| curve | add | sub | neg | OK/N |
|---|---|---|---|---:|
| P-25519 | OK | OK | OK | **3/3** |
| P-256 | OK | OK | OK | **3/3** |
| secp256k1 | **FAIL** | OK | OK | **2/3** |
| P-384 | **FAIL** | OK | OK | **2/3** |
| P-521 | OK | OK | OK | **3/3** |
| SM2 | OK | OK | OK | **3/3** |
| **total** | **4/6** | **6/6** | **6/6** | **16/18** |

### Hypotheses and axiom delta (diagnostic replay, same answer bytes)

`research/receipts/r4-replay.log` (18 lines, from `.bench/ref-replay-run.sh`, which replaces
the harness `bench_run` with a prover that reports the hypothesis count and the propagating
exception; it does not touch the answers or the verdicts):

* 16 tasks: `closed=1 hyps=0 axioms_before=... axioms_after=... unchanged=true`
  (`2445 -> 2445`, and `6519 -> 6519` for `sub_p256`, whose template loads a larger context).
* 2 tasks: `closed=0 exn=Failure("CSAT: cadical did not report UNSAT, exit 10")`, axioms also
  unchanged.

The judge itself writes `CHEATING` if the axiom set changes and `prove` rejects a theorem with
additional assumptions, so an `OK` verdict already implies both.

## 3. The two failures, with the exact tactic

Both fail inside the certified-SAT close, and both log the exception verbatim:

```
ANSWER_EXCEPTION: Failure("CSAT: cadical did not report UNSAT, exit 10")
```

(native run logs `.bench/eval-20261123_163250/ml_files_arm_bignum_{add_p256k1,add_p384}.run.outlog`;
replay reproduces the same exception in 10.827 s and 7.246 s).

* **Exact tactic:** `BRIDGE_CSAT_TAC` as invoked by `BRIDGE_FINISH_KEEP_TAC`.
* **Meaning:** `cadical` exited **10 = SATISFIABLE**, i.e. the residual instance is
  *satisfiable*. This is not a timeout and not an arithmetic lemma failure; it is an encoding
  gap, and it is a genuine statement about the instance: some bit is free that the theorem
  needs pinned.

**Cause (measured, same for both):** these two routines select their modular correction with
the **Z flag of a chain of flag-only instructions**, not with a carry that an accumulator
relation ties to a word:

* `bignum_add_p256k1`: `MOV/ORR` of `2^32+977`, `CMN X5 X4`, then `ADCS XZR X9 XZR`,
  `ADCS X3 X3 XZR`, `CSEL X4 X4 XZR Condition_NE`.
* `bignum_add_p384`: `CMP` + `SBCS XZR ...` x4 + `ADCS XZR ...` x3, then `CSEL`/`CSETM`
  on `Condition_NE`.

`derive_steps` excludes instructions whose destination is `XZR` (they materialise or probe a
flag rather than a digit), and `ACCUMULATE_ARITH_TAC` can only relate a flag to words through a
*register* destination's read equation. So the carry reaching the `NE` test is a free boolean in
the instance, and SAT can set it to falsify `output = ref_add`. Nothing about the arithmetic
lemmas is wrong: the same `ref_add` closes `add_p25519`, `add_p256`, `add_p521` and `add_sm2`.

(* **R3 comparison for `add_p384`:** R3's `OK` came from a *different* simulation glue that
supplied `carry_s22 <=> p_384 <= bignum_of_wordlist [...]` by `FLAG_FROM_CARRY_LE + BOUNDER_TAC`
(real arithmetic, `DEVELOPMENT_ONLY`, transcribed from `arm/proofs/bignum_add_p384.ml`). The
pure reference route deliberately carries no such subgoal, so it cannot reproduce that close.
That is a measured loss of one task, not a change in the encoding: with the transcribed flag
subgoal the answer is still `OK` and is still stored, judged, in
`.bench/eval-20261123_135102/`.)

The fix that would close both is to give the flag-only `ADCS/SBCS XZR` forms accumulator
relations (a `word_carry`-style hypothesis per instruction, still bit-level and curve-free).
That is a new mechanism, not a lemma tweak, and it is queued as V8.23 below.

## 4. `neg_p521` specifically — answered, and why it needed a different path

`neg_p521` previously emitted **no answer at all**: `tools/emit_proof.py` raised
`ValueError: derive_steps found no accumulating steps`, because the routine is a pure
complement and has no add/sub carry chain:

```
ORR ... (zero test) ; CMP ; CSETM X2 Condition_NE ;
EOR X3..X10 with X2 ; AND X2 X2 (word 511) ; EOR X11 X11 X2
```

The witness generator's precondition (`accumulate` non-empty) is what blocked it, not the
mathematics. The reference route needs no accumulating step: the written value is a pure word
expression in the input limbs (`word_xor`/`word_and`), so the simulation is
`ARM_STEPS_TAC ... (1--N)` with **no** accumulator context and **no** flag-meaning subgoal, and
the close is the same `BRIDGE_NEG_REF_TAC` (`ref_neg` with `p = 2^521-1`, 576-bit words).

Result: **OK, 19.527 s, 0 hypotheses, axioms 2445 -> 2445.** The carry-chain assumption was
not forced anywhere; there is no carry-chain statement in the answer at all.

The routine computes `~n & (2^521-1)` when `n != 0` and keeps the (zero) value otherwise, which
is exactly `ref_neg`: the SAT instance has to prove the 521-bit complement identity, and it
does.

## 5. DEVELOPMENT_ONLY (transcribed, named, not claimed)

Everything below is *idiom*, not a fact about a curve; no curve or reduction appears in any
lemma in `tools/bridge.ml`:

1. `simulation.accsteps` — one `ARM_ACCSTEPS_TAC` block over the whole routine, with the
   accumulate set taken from `derive_steps`. The *single-block* shape is the human proofs'
   idiom; the step set is derived.
2. `post.digit_expansion` — `BIGNUM_EXPAND_CONV` / `BIGNUM_LEXPAND_CONV` with the
   `LAND_CONV`/`RAND_CONV` traversal, and `DISCARD_STATE_TAC`. Human expansion idiom.
3. `post.bitval_rewrites` — `RULE_ASSUM_TAC(REWRITE_RULE[REAL_BITVAL_NOT; ADD_CLAUSES;
   VAL_WORD_BITVAL])` in the add close. Copied from the human add proofs.
4. The `--mode dev` harness of `tools/emit_ref_proof.py` reads the `define_assert_from_elf`
   word list and the modulus definition out of `arm/proofs/bignum_{op}_{family}.ml` to build a
   scratch process. The judged answer does not: it uses the benchmark setup's own definitions.
   This affects only development runs, and the dev scripts are labelled as such.
5. The previous R3 answers' `sub_middle` / P-384 carry subgoals are **not** used anywhere in
   this run.

## 6. Files changed

Modified: `tools/bridge.ml` (+258 lines, 832 -> 1090: section 8 with the two reference definitions, the nine
lemmas above, the numeral-aware bound lookup and the two tactics; the existing `BRIDGE_*_TAC`
split routes are untouched and still load), `research/receipts/bridge/bridge_instrumented.ml`
(regenerated from it).

Added: `tools/emit_ref_proof.py` (the generator), `research/receipts/bridge/ref_check.ml` +
`ref_check.log`, `research/receipts/r4-{verdicts.csv,profile.txt,replay.log,answers.sha256,curve-check.log,csat-instance.log,hashes.txt,reference-route.md}`.
Hashes: `r4-hashes.txt`
(`tools/bridge.ml` sha256 `e9d2369ba97615eb82030928f1dae23226581e2e8663e9620976b0029d6e9002`).

Working artefacts under git-ignored `.bench/`: `ref-work/` (setup+query+answer, 26 MB),
`eval-20261123_163250/` (judge, 542 MB), `ref-replay/` (32 MB), `ref-dev/` (dev scripts and
full logs; the multi-megabyte ones are kept here, not in `research/`).

Not modified: `.bench/s2n-bignum` sources, `objfiles/`, `toplevel-thms/`, and no symlink was
created.

## 7. What improved, and what did not

**Improved.** 16/18 against 10/18 declared in R3, with the failures now a single, precisely
located encoding gap instead of an arithmetic one.

* Seven tasks that R3 could not close now close on one generic route with no flag meaning:
  `add_p25519`, `add_p521`, `sub_p25519`, `sub_p256k1`, `sub_p521`, `neg_p25519`, and
  `neg_p521` (which R3 could not even emit).
* `sub` is 6/6 for the first time, `neg` is 6/6 for the first time, and every `sub`/`neg`
  answer is a pure word instance; the two `BOUNDER_TAC` failures (`sub_* <=> m < n`) and the
  `REAL_INTEGER_TAC` failure (`neg_p25519`) are gone because nothing in the route asks for
  those statements.
* P-256 and P-384 `sub`/`neg` moved off the split route onto the reference route;
  `neg_p256` 9.171 -> 6.551 s, `sub_p256` 18.431 -> 14.536 s (R3 numbers from
  `research/receipts/r3-coverage-run.md` §4).
* The new lemmas are curve-free: `statements=45 hits=0`; fresh load 81/81, axioms 3 -> 3.

**Did not improve.** `add_p256k1` and `add_p384` remain open, and `add_p384` regressed relative
to R3's glued route (see §3). Both need the same missing piece: accumulator relations for
flag-only `ADCS/SBCS XZR` instructions, because their correction is selected by the Z flag of a
flag-only chain. Until that exists, the honest statement is: the reference route closes every
ARM `sub`/`neg` in the declared set and four of six `add`s, and cannot close a routine whose
correction is selected by a flag that is not carried by a register write.

**Budget.** The slowest OK is `add_p521` at 95.864 s of the 120 s per-task budget (0.80). The
reference route is not free: it moves work from real arithmetic into the bit-level instance.
A 576-bit addition chain with a P-521 correction is the expensive case; nothing timed out, but
`add_p521` and `sub_p521` have the least headroom.

## 8. Environment

`cadical` 2.1.2 at
`/home/one/.elan/toolchains/leanprover--lean4---v4.29.1/bin/cadical`, exported on `PATH` for
every run (`run-answers.sh` and `ref-replay-run.sh`). No exit-127 environment failure occurred;
a run without it on `PATH` would be an environment failure, not a proof failure. Python
3.14.10; `combine-answer-and-setup.py ref-work 1` (one core, in-process syntax gate — with
`num_cores > 1` on this Python the pool re-imports the module and the grader raises
`KeyError`, which is a harness/Python-3.14 issue, not a task failure). All 18 answers passed
the `ocamlc -i` syntactic gate and none was dropped. Host: 16 cores, 65 GB free RAM at start;
no OOM. Disk 26 GB before, 25 GB after.
