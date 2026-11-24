# IX.65 — the same-obligation ratio (XIV.G2) and where the non-replay work goes (XIV.B3 / XIII.E5)

- **Date:** 2026-11-24
- **Branch:** `staging`
- **Change under test:** `tools/csat_lib.ml` gains a guarded `csat_mark` tactic and
  two replay brackets inside `CSAT_REFUTE_GEN`; `src/tools/emit.rs` gains
  `--mode phase`, which emits the *same* tactic with `csat_mark` boundaries at the
  emitter's own block boundaries. `--mode answer` is untouched.
- **Part 0 answer:** with the obligation now identical (IX.64), the per-task
  `_CORRECT` proof-body inference ratio is **median 60.20×, range 12.93×–109.07×**
  over **n = 6** measured pairs; the aggregate is **70.87×**
  (993,573,633 / 14,019,340). The derived `_SUBROUTINE_CORRECT` step (0.33–0.42 s,
  IX.64) is **not** in either number.
- **Part 1 answer:** after trimming, **98.2%** of the six measured proofs' primitive
  inferences are inside `BRIDGE_*_REF_TAC`, of which **30.35%** is certified LRAT
  replay and **67.85%** is the bridge's own extension/encoding work. Simulation is
  **0.77%**. The largest non-replay cost is therefore **bridge-minus-replay**, not
  simulation.
- **E4 verdict from the data:** `XIII.E4` is aimed at the **right block** — the cost
  is in the bridge, not in the simulation a carry lemma would not touch — but this
  measurement **does not** show that a width/modulus-parametric lemma removes it:
  the block also contains the CNF encoding and bitblast that surround the replay,
  and those are not separated here.

---

## 0. What the marker is and how it is switched on

`tools/csat_lib.ml` now defines (as ordinary `let` phrases, because the emitted
answer inlines each phrase as a nested binding):

| phrase | role |
|---|---|
| `csat_mark_enabled` | `bool ref`, **false** in every normal answer |
| `csat_mark_reader` | `(unit -> int) ref`; defaults to a function that raises |
| `csat_mark : string -> tactic` | identity on the goal; prints `PHASE_MARK <label> <count>` only when enabled |

`CSAT_REFUTE_GEN` brackets `LRAT_REFUTE_GEN` with two more markers when enabled:

```ocaml
let _ = if !csat_trim_enabled then csat_trim_lrat lratname in
let _ = if !csat_mark_enabled then
          Printf.printf "PHASE_MARK bridge:pre-replay %d\n%!" (!csat_mark_reader ()) in
let res = LRAT_REFUTE_GEN amap cnfname lratname in
let _ = if !csat_mark_enabled then
          Printf.printf "PHASE_MARK bridge:post-replay %d\n%!" (!csat_mark_reader ()) in
```

A measurement answer is the normal answer with one prefix and the markers inserted:

```text
(csat_mark_reader := inference_count;
csat_mark_enabled := true;
csat_mark "00 rewrite" THEN <line 0> THEN
csat_mark "01 rewrite" THEN <line 1> THEN
...
)
```

The block boundaries are **the emitter's own lines** (it joins the chain with
` THEN\n`), plus the `ENSURES_INIT_TAC` that shares a line with the
`BIGNUM_FROM_MEMORY_BYTES` rewrite. `phase_class` is mechanical and total: a line
containing `*_ACCSTEPS_TAC`/`*_STEPS_TAC` is `play`, `BRIDGE_` is `bridge`,
`ENSURES_INIT_TAC`/`ENSURES_FINAL_STATE_TAC`/`DISCARD_STATE_TAC` is
`finalisation`, everything else is `rewrite`.

There is no trailing marker: `BRIDGE_*_TAC` solves the goal, so a tactic after it
never runs. The last block is closed by `STAMP_INF_AFTER` in the reconciliation.

**Normal emission is unchanged.** Three checks, verbatim:

```text
$ bash .bench/ix65/prove_normal_unchanged.sh
NORMAL_TACTIC_UNCHANGED true (pre-IX.65 a1.answer vs post-IX.65 normal.now)
PHASE_MODE_IS_NORMAL_PLUS_MARKS true
LIBRARY_PREFIX_IDENTICAL true
ANSWER_DIFF_HUNKS 2
ANSWER_DIFF_ADDED_LINES 36
ANSWER_DIFF_REMOVED_LINES 1
```

`a1.answer` is the same task emitted by the **pre-IX.65** binary; its tactic region
is byte-identical to the post-IX.65 `--mode answer`. Removing the markers and the
enable prefix from a `--mode phase` answer restores the `--mode answer` tactic
byte-for-byte, and the two answers share the same library prefix. The 36 added
lines are the guarded `csat_mark` phrases and the two replay brackets in
`csat_lib.ml`; the 1 removed line is the anchor they were inserted before. The
inlined library text therefore changes for every answer, exactly as the IX.63
trimmer changed it; the **tactic** and the theorem do not. The IX.63 retirement of
Python-era byte identity therefore still holds, and `tests/tools_receipt.rs`'s four
digests are repinned to the post-IX.65 text.

---

## 1. Part 0 — the same-obligation comparison (XIV.G2)

### 1.1 Why a quotient is now legal

IX.64 (`PREMISE total=27 same=27`) established that our built
`BIGNUM_*_CORRECT` conclusion is the **same statement** as upstream's own
`BIGNUM_*_CORRECT` quotation from the pinned `.ml`. Part 0 compares **that**
statement's proof body on both sides:

- **same contract** — `BIGNUM_<OP>_<FAM>_CORRECT` on both sides, identical
  statement (IX.64), not `_CORRECT` vs `_SUBROUTINE_CORRECT`;
- **same kernel** — `.bench/hol-light-stamp2`, axiom-stamp v2, same pin;
- **same host** — one machine, one `-j1` run per task, `make -B`;
- **same scope** — primitive inferences from immediately before the
  `_CORRECT` `prove` to immediately after it. The ELF decode, the shared
  bootstrap, and the derived `_SUBROUTINE_CORRECT` step are excluded on both
  sides. On the generated side `inference_delta` is the `STAMP_INFERENCES` delta
  around `prove`; on the human side `HUMAN_CORRECT_INF` is the delta around the
  `let BIGNUM_*_CORRECT = time prove ...;;` phrase.
- **derived step kept separate** — the `_SUBROUTINE_CORRECT` derivation costs
  **0.33–0.36 s (ARM)** and **0.39–0.42 s (x86 NOIBT) + 0.0002–0.0004 s (IBT)**
  (IX.64), and is added to neither column. Wall seconds below are the `_CORRECT`
  prove alone.

The human measurement is a **fresh** run. IX.60's `human_inferences` is the whole
file body after `needs` (decode + `_CORRECT` + `_SUBROUTINE_CORRECT` wrapper), and
its `human_seconds` is the *last* `Running time` in the file, i.e. the wrapper's.
Neither is the `_CORRECT` body, so neither is used. The new run instruments a
**scratch copy** of the human tree at `.bench/ix65/human` (84 MB, built with
`rsync --exclude '*.native' --exclude '*.correct'`); `.bench/human-baseline` was
never written. Each scratch proof file is restored from the pristine tree after the
run and sha256-matched against it:

```text
RESTORED arm/proofs/bignum_add_p25519.ml
RESTORED arm/proofs/bignum_add_p384.ml
RESTORED arm/proofs/bignum_add_p521.ml
RESTORED arm/proofs/bignum_neg_p25519.ml
RESTORED x86/proofs/bignum_add_p25519.ml
RESTORED x86/proofs/bignum_neg_sm2.ml
```

All six human runs exited 0 (so `build-proof.sh`'s `check_axioms ()` passed) and
all six generated runs exited 0.

**Denominator: n = 6 matched pairs of 27 tasks.** The six are the mandatory theorems
of Part 1. The other 21 were not measured inside the 90-minute compute budget; no
number below is extended to them.

### 1.2 The comparison command and table

```text
$ bash .bench/ix65/build_part0.sh            # .bench/ix65/{human,generated}.json
$ ./target/release/dropin matched-table \
    --human .bench/ix65/human.json \
    --generated .bench/ix65/generated.json \
    --human-kernel hol-light-stamp2 --generated-kernel hol-light-stamp2 \
    --json research/receipts/ix65-same-obligation.json
MATCHED 6/6 PAIRS; UNMATCHED 0; HUMAN ROWS 6; GENERATED ROWS 6
```

The typed machinery (`MeasuredProof` / `compare` / `MatchedComparison`) is what
constructs every row; it refuses a row whose theorem identity, executable identity,
scope, or judgment differs. All six are accepted, so `matched-table` exits 0.

| arch | task | human `_CORRECT` inf | generated `_CORRECT` inf | ratio | human s | generated s | ratio |
|---|---|---:|---:|---:|---:|---:|---:|
| arm | curve25519/add_p25519 | 1,848,494 | 119,194,788 | 64.48× | 0.499 | 42 | 84.17× |
| arm | p384/add_p384 | 3,989,992 | 241,041,106 | 60.41× | 1.261 | 64 | 50.75× |
| arm | p521/add_p521 | 4,312,009 | 470,294,392 | 109.07× | 1.034 | 103 | 99.61× |
| arm | curve25519/neg_p25519 | 721,473 | 33,981,360 | 47.10× | 0.177 | 29 | 163.84× |
| x86 | curve25519/add_p25519 | 1,877,546 | 112,639,551 | 59.99× | 0.586 | 67 | 114.33× |
| x86 | sm2/neg_sm2 | 1,269,826 | 16,422,436 | 12.93× | 0.373 | 51 | 136.73× |

Machine-derived aggregates (`matched_totals` in the JSON, denominator n = 6):

| metric | value |
|---|---:|
| matched pairs (n) | 6 |
| human inferences | 14,019,340 |
| generated inferences | 993,573,633 |
| aggregate inference ratio | **70.87×** |
| per-task inference ratio median | **60.20×** |
| inference ratio range | **12.93×–109.07×** |
| human seconds | 3.930 |
| generated seconds | 356.000 |
| aggregate seconds ratio | **90.59×** |
| per-task seconds ratio median | **106.97×** |
| seconds ratio range | **50.75×–163.84×** |

The four fixed values to quote are: **median 60.20×, range 12.93×–109.07×,
aggregate 70.87×, n = 6** (of 27), over identical `_CORRECT` statements, one
kernel pin, one host, `-j1`.

### 1.3 What moved relative to IX.60

IX.60's human bodies were 2.7M–17.6M because they were the **whole file** after
`needs`. The `_CORRECT` proof alone is **0.72M–4.31M**, and its wall time is
**0.18–1.26 s**, not 24–54 s: IX.60's `human_seconds` was the `Running time` of
the last `time prove` in the file, which on these files is the
`_SUBROUTINE_CORRECT` wrapper (`ARM_ADD_RETURN_NOSTACK_TAC` /
`X86_PROMOTE_RETURN_NOSTACK_TAC`), not the `_CORRECT` proof. The ratio therefore
moves **up** from the unquotable IX.60 pair-of-counts to 12.93×–109.07×. This is
the F3 number; it is a quotient of the same obligation, and the denominator is
printed with it.

---

## 2. Part 1 — phase attribution of the non-replay work

### 2.1 Runs

Six tasks, one at a time, `make -j1 -B`, axiom-stamp v2, answers emitted with
`--mode phase` and assembled with `dropin assemble --stamp`. Verbatim driver and
per-task totals:

```text
$ bash .bench/ix65/run_phase.sh <arch> <dir> <op> <family>   # once per task
$ bash .bench/ix65/run_rest.sh
RUN arm_add_p25519 rc=0 phase_marks=21
RUN arm_add_p384 rc=0 phase_marks=21
RUN arm_add_p521 rc=0 phase_marks=21
RUN arm_neg_p25519 rc=0 phase_marks=17
RUN x86_add_p25519 rc=0 phase_marks=21
RUN x86_neg_sm2 rc=0 phase_marks=16
```

The measured totals reproduce the IX.63 trimmed baseline **exactly**, task for
task:

| task | IX.63 `after_inferences` | this run `STAMP_INFERENCES` |
|---|---:|---:|
| arm/add_p25519 | 119,194,788 | 119,194,788 |
| arm/add_p384 | 241,041,106 | 241,041,106 |
| arm/add_p521 | 470,294,392 | 470,294,392 |
| arm/neg_p25519 | 33,981,360 | 33,981,360 |
| x86/add_p25519 | 112,639,551 | 112,639,551 |
| x86/neg_sm2 | 16,422,436 | 16,422,436 |

This is the control that the marker is a no-op on the derivation.

### 2.2 The mark sequences

```text
### arm/add_p25519
PHASE_MARK 00 rewrite 69123179
PHASE_MARK 01 rewrite 69123179
PHASE_MARK 02 rewrite 69123311
PHASE_MARK 03 finalisation 69123312
PHASE_MARK init finalisation 69123474
PHASE_MARK 04 rewrite 69124148
PHASE_MARK 05 rewrite 69124509
PHASE_MARK 06 play 69124974
PHASE_MARK 07 finalisation 69912865
PHASE_MARK 08 rewrite 70100089
PHASE_MARK 09 rewrite 70100287
PHASE_MARK 10 finalisation 70100693
PHASE_MARK 11 bridge 70100749
PHASE_MARK bridge:pre-replay 89286945
PHASE_MARK bridge:post-replay 101148295
PHASE_MARK bridge:pre-replay 119656383
PHASE_MARK bridge:post-replay 124311759
PHASE_MARK bridge:pre-replay 143735719
PHASE_MARK bridge:post-replay 156641972
PHASE_MARK bridge:pre-replay 175201816
PHASE_MARK bridge:post-replay 186587670

### arm/add_p384
PHASE_MARK 00 rewrite 69219561
PHASE_MARK 01 rewrite 69219561
PHASE_MARK 02 rewrite 69219693
PHASE_MARK 03 finalisation 69219694
PHASE_MARK init finalisation 69219856
PHASE_MARK 04 rewrite 69220534
PHASE_MARK 05 rewrite 69221011
PHASE_MARK 06 play 69221668
PHASE_MARK 07 finalisation 70932730
PHASE_MARK 08 rewrite 71142242
PHASE_MARK 09 rewrite 71143275
PHASE_MARK 10 finalisation 71143897
PHASE_MARK 11 bridge 71143967
PHASE_MARK bridge:pre-replay 113426160
PHASE_MARK bridge:post-replay 127215559
PHASE_MARK bridge:pre-replay 169549439
PHASE_MARK bridge:post-replay 198347461
PHASE_MARK bridge:pre-replay 241034643
PHASE_MARK bridge:post-replay 254322155
PHASE_MARK bridge:pre-replay 296631852
PHASE_MARK bridge:post-replay 308219587

### arm/add_p521
PHASE_MARK 00 rewrite 69188703
PHASE_MARK 01 rewrite 69188703
PHASE_MARK 02 rewrite 69188835
PHASE_MARK 03 finalisation 69188836
PHASE_MARK init finalisation 69188998
PHASE_MARK 04 rewrite 69189702
PHASE_MARK 05 rewrite 69190383
PHASE_MARK 06 play 69191388
PHASE_MARK 07 finalisation 71459916
PHASE_MARK 08 rewrite 71793769
PHASE_MARK 09 rewrite 71794149
PHASE_MARK 10 finalisation 71795092
PHASE_MARK 11 bridge 71795183
PHASE_MARK bridge:pre-replay 150523858
PHASE_MARK bridge:post-replay 207360362
PHASE_MARK bridge:pre-replay 286615884
PHASE_MARK bridge:post-replay 329240702
PHASE_MARK bridge:pre-replay 405738156
PHASE_MARK bridge:post-replay 450843930
PHASE_MARK bridge:pre-replay 527122597
PHASE_MARK bridge:post-replay 537090961

### arm/neg_p25519
PHASE_MARK 00 rewrite 69124876
PHASE_MARK 01 rewrite 69124876
PHASE_MARK 02 rewrite 69124988
PHASE_MARK 03 finalisation 69124989
PHASE_MARK init finalisation 69125109
PHASE_MARK 04 rewrite 69125643
PHASE_MARK 05 play 69125952
PHASE_MARK 06 finalisation 69654814
PHASE_MARK 07 rewrite 69797386
PHASE_MARK 08 finalisation 69797717
PHASE_MARK 09 bridge 69797797
PHASE_MARK bridge:pre-replay 78624881
PHASE_MARK bridge:post-replay 78861634
PHASE_MARK bridge:pre-replay 89150550
PHASE_MARK bridge:post-replay 91148841
PHASE_MARK bridge:pre-replay 101225049
PHASE_MARK bridge:post-replay 101500076

### x86/add_p25519
PHASE_MARK 00 rewrite 83145618
PHASE_MARK 01 rewrite 83145618
PHASE_MARK 02 rewrite 83145750
PHASE_MARK 03 finalisation 83145751
PHASE_MARK init finalisation 83145913
PHASE_MARK 04 rewrite 83146599
PHASE_MARK 05 rewrite 83146960
PHASE_MARK 06 play 83147425
PHASE_MARK 07 finalisation 84492385
PHASE_MARK 08 rewrite 84725471
PHASE_MARK 09 rewrite 84725665
PHASE_MARK 10 finalisation 84726071
PHASE_MARK 11 bridge 84726127
PHASE_MARK bridge:pre-replay 103704471
PHASE_MARK bridge:post-replay 118323623
PHASE_MARK bridge:pre-replay 136622969
PHASE_MARK bridge:post-replay 141764879
PHASE_MARK bridge:pre-replay 160903073
PHASE_MARK bridge:post-replay 170984166
PHASE_MARK bridge:pre-replay 189261016
PHASE_MARK bridge:post-replay 194019204

### x86/neg_sm2
PHASE_MARK 00 rewrite 83141614
PHASE_MARK 01 rewrite 83141614
PHASE_MARK 02 rewrite 83141726
PHASE_MARK 03 finalisation 83141727
PHASE_MARK init finalisation 83141847
PHASE_MARK 04 rewrite 83142397
PHASE_MARK 05 play 83142706
PHASE_MARK 06 finalisation 84108525
PHASE_MARK 07 rewrite 84323553
PHASE_MARK 08 finalisation 84323884
PHASE_MARK 09 rewrite 84323952
PHASE_MARK 10 bridge 84323992
PHASE_MARK bridge:pre-replay 87269193
PHASE_MARK bridge:post-replay 87408478
PHASE_MARK bridge:pre-replay 96406534
PHASE_MARK bridge:post-replay 97937511
```

### 2.3 The block-cost command

```text
$ for f in arm_add_p25519 arm_add_p384 arm_add_p521 \
           arm_neg_p25519 x86_add_p25519 x86_neg_sm2; do
    awk -f .bench/ix65/phases.awk ".bench/ix65/$f.correct"
  done > .bench/ix65/blocks.tsv
$ bash .bench/ix65/build_phases.sh    # writes research/receipts/ix65-phases.json
```

`blocks.tsv` is `<tag> <index> <class> <preceding-label> <inferences>`; a block's
cost is the difference between consecutive marks, `initial` is
`STAMP_INF_BEFORE → first mark`, and the last block is `STAMP_INF_AFTER − last
mark`. `research/receipts/ix65-phases.json` carries every block and every class
total per task.

### 2.4 Reconciliation: `sum(blocks) + initial = total`

| task | initial | sum(blocks) | sum + initial | `STAMP_INFERENCES` | equal |
|---|---:|---:|---:|---:|:--:|
| arm/add_p25519 | 1,476,272 | 117,718,516 | 119,194,788 | 119,194,788 | yes |
| arm/add_p384 | 1,476,928 | 239,564,178 | 241,041,106 | 241,041,106 | yes |
| arm/add_p521 | 1,477,902 | 468,816,490 | 470,294,392 | 470,294,392 | yes |
| arm/neg_p25519 | 1,475,563 | 32,505,797 | 33,981,360 | 33,981,360 | yes |
| x86/add_p25519 | 1,511,665 | 111,127,886 | 112,639,551 | 112,639,551 | yes |
| x86/neg_sm2 | 1,510,972 | 14,911,464 | 16,422,436 | 16,422,436 | yes |

Every row is exact, not rounded. `initial` is the ~1.5M inferences between the
`STAMP_INF_BEFORE` mark and the first marker (tactic application and goal access).

### 2.5 Per-class split

`replay` is the certified LRAT replay (`LRAT_REFUTE_GEN`), measured by the
`bridge:pre-replay` / `bridge:post-replay` brackets. `bridge` below is the
**non-replay** bridge work: `ELIM_DIGIT_EQS_TAC`, the modulus rewrite, the bignum
expansion, `BRIDGE_WORD_OF_DIGITS`, the condition split, `BRIDGE_TO_WORD_CONV`, and
`CSAT_PROVE`'s definitionalisation/clausification. `bridge/SAT = replay + bridge`.

| arch | task | total | initial | play | rewrite | finalisation | replay | bridge (non-replay) | bridge/SAT | bridge/SAT % |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| arm | curve25519/add_p25519 | 119,194,788 | 1,476,272 | 787,891 | 1,563 | 188,116 | 40,808,833 | 75,932,113 | 116,740,946 | 97.9% |
| arm | p384/add_p384 | 241,041,106 | 1,476,928 | 1,711,062 | 2,922 | 210,422 | 67,462,668 | 170,177,104 | 237,639,772 | 98.6% |
| arm | p521/add_p521 | 470,294,392 | 1,477,902 | 2,268,528 | 3,142 | 334,810 | 154,535,460 | 311,674,550 | 466,210,010 | 99.1% |
| arm | curve25519/neg_p25519 | 33,981,360 | 1,475,563 | 528,862 | 753 | 143,306 | 2,510,071 | 29,322,805 | 31,832,876 | 93.7% |
| x86 | curve25519/add_p25519 | 112,639,551 | 1,511,665 | 1,344,960 | 1,559 | 233,990 | 34,600,343 | 74,947,034 | 109,547,377 | 97.3% |
| x86 | sm2/neg_sm2 | 16,422,436 | 1,510,972 | 965,819 | 793 | 215,766 | 1,670,262 | 12,058,824 | 13,729,086 | 83.6% |
| **all** | **6 tasks** | **993,573,633** | **8,929,302** | **7,607,122** | **10,732** | **1,326,410** | **301,587,637** | **674,112,430** | **975,700,067** | **98.2%** |

Shares of the six-task total: initial **0.90%**, play **0.77%**, rewrite
**0.00%**, finalisation **0.13%**, replay **30.35%**, bridge (non-replay)
**67.85%**. Non-replay work is **69.65%** of the total; **97.4%** of that
non-replay work is the bridge's own extension/encoding.

`arm/add_p521` specifically:

| class | inferences | share |
|---|---:|---:|
| initial | 1,477,902 | 0.31% |
| play (ACCSTEPS simulation) | 2,268,528 | 0.48% |
| rewrite / normalisation | 3,142 | 0.00% |
| finalisation | 334,810 | 0.07% |
| replay (certified LRAT) | 154,535,460 | 32.86% |
| bridge (non-replay) | 311,674,550 | 66.27% |
| **total** | **470,294,392** | **100%** |

Its four closes account for 154,535,460 replay inferences. The bridge-minus-replay
piece is **311.7M — 2.0× the replay and 172× the human `_CORRECT` body** (4.31M).

### 2.6 The concentration finding

- The excess is **not** simulation. `ARM_ACCSTEPS_TAC` / `X86_ACCSTEPS_TAC` is
  **0.48–1.5%** of each add proof and **1.6–5.9%** of each neg proof.
- The excess is **not** the kernel replay alone. Trimmed replay is **30.35%** of
  the six-task total (32.86% for `add_p521`). Note: the 22M replay figure used in
  the IX.65 brief is not reproduced here; the measured trimmed replay for
  `arm/add_p25519` is **40,808,833**, and the retained LRAT is 246,832 addition
  lines, i.e. ≈165 primitive inferences per retained line. IX.63's earlier
  "each replayed as one primitive inference" line is wrong for the same reason —
  the receipt is not amended here, this note is the correction of record.
- The excess is the **bridge outside the replay**: the rewrite/wordification/
  encoding that surrounds each of the four SAT closes, plus the split that
  produces them. It is **67.85%** of the six-task total.

### 2.7 E4 verdict, from the numbers

**A width/modulus-parametric lemma is aimed at the right block.** The cost is not
in simulation, so the warning in the brief ("no, the cost is in simulation, which
parametric lemmas do not touch") does **not** apply. The 67.85% is in
`BRIDGE_*_REF_TAC`, whose per-theorem obligation is exactly the thing XIII.E4
proposes to replace with a parametric carry lemma: it already instantiates the
width-parametric `ADD_REF_VAL_WORD` (`INST_TYPE [wt,:W]`), and the remaining cost
is the per-instance proof of `val (ref_add (word p) x y) = val w` by expansion,
wordification and four bitblast/SAT closes.

**What the data does not establish:** that such a lemma removes the cost. The
non-replay bridge block also contains `CSAT_PROVE`'s definitionalisation and
clausification and `BRIDGE_TO_WORD_CONV`, i.e. the CNF encoding. This run does not
separate "arithmetic rewriting" from "encoding"; a parametric lemma that closes the
word equation would remove the encoding only if it bypasses `BITBLAST_THEN`
entirely (as a direct rewrite would). So the verdict is **localised and
plausible, not proved**: E4 targets 67.85% of the measured work, and the next
measurement it needs is a split of the bridge block into `BRIDGE_PREPARE_TAC`,
`CSAT_PROVE` encoding, and the replay — not a bigger carry lemma.

---

## 3. Part 2 — the honest limit

- **Search inside a successful block is not separated.** The counter is monotone.
  A tactic attempt that fails and is retried (or a `THENL` branch that is
  abandoned) leaves its primitive inferences in whichever block it ran in. The
  marker cannot tell discarded work from the derivation that survives.
- **What *is* separated.** (i) The certified replay is bracketed exactly, so
  "derivation that the kernel checks" is a separate number from the bridge's own
  work. (ii) CaDiCaL's own search produces **zero** primitive inferences — it is
  outside the kernel — so no solver search hides inside the replay number; the
  replay is exactly `LRAT_REFUTE_GEN`. (iii) The blocks themselves are disjoint and
  sum exactly to the total.
- **What is *not* separated inside the bridge.** The 67.85% is the sum of the
  bridge's rewriting/wordification and `CSAT_PROVE`'s encoding. Two further markers
  (one at the start of `CSAT_PROVE`, one before its `CSAT_REFUTE_GEN` call) would
  split encoding from replay; they were not added, because that would change the
  emitted library text again for a second number, and the E4 question is already
  answered at the current resolution (the cost is not in simulation).
- **`accumulator/arith` is not a separate measured block.** The only accumulator
  reasoning in the emitted chain is inside upstream's single `*_ACCSTEPS_TAC`, which
  cannot be split without editing `.bench/s2n-bignum`. It is therefore reported
  under `play`, and its cost is the 0.77% in §2.5. This is a limit of the emitted
  text, not of the marker.
- **n = 6 of 27** for Part 0 (and the six mandatory theorems for Part 1). The other
  21 human `_CORRECT` bodies were not measured in the 90-minute budget. No
  cross-task aggregate is extended beyond the six.
- **One run per task, one host, one pin, one kernel.** No repetition measure; the
  inference counts are deterministic and reproduce IX.63 exactly, but the wall
  seconds are single-shot.

---

## 4. Verification and provenance

```text
$ cargo test --offline
... all suites green (305 tests passed, 0 failed)
$ cargo clippy --offline --all-targets -- -D warnings
clippy_rc=0
$ cargo fmt --check
fmt_rc=0
$ git ls-files '*.py' | wc -l
0
```

- Every one of the six generated runs exited 0 with `check_axioms` passing, and its
  `STAMP_INFERENCES` equals the IX.63 trimmed baseline exactly (§2.1).
- The marker is inert in `--mode answer`: `strip_phase_marks(phase_tactic(t)) == t`
  is a unit test (`tests/tools_emit.rs`), together with
  `phase_mode_is_the_normal_emitter_plus_marks` and the pre-vs-post tactic
  comparison (§0).
- Human measurements ran on `.bench/ix65/human` only; `.bench/human-baseline` was
  not written, and the six instrumented files were restored with matching sha256
  (§1.1).
- No Python was used and none is tracked. Scratch is under `.bench/`, git-ignored.
- **Freeze `v6` is still required before any sealed run.** `tools/csat_lib.ml`,
  `src/tools/emit.rs` and `tests/` moved, so the `freeze-rust-closure-v5` identity
  is stale. This receipt does not create `v6` and no sealed run is claimed.

## 5. Reproduction

```text
$ bash .bench/ix65/run_phase.sh arm curve25519 add p25519     # and the other five
$ bash .bench/ix65/run_human.sh arm curve25519 add p25519     # and the other five
$ bash .bench/ix65/build_part0.sh
$ ./target/release/dropin matched-table --human .bench/ix65/human.json \
    --generated .bench/ix65/generated.json \
    --human-kernel hol-light-stamp2 --generated-kernel hol-light-stamp2 \
    --json research/receipts/ix65-same-obligation.json
$ for f in ...; do awk -f .bench/ix65/phases.awk ".bench/ix65/$f.correct"; done \
    > .bench/ix65/blocks.tsv
$ bash .bench/ix65/build_phases.sh
$ bash .bench/ix65/prove_normal_unchanged.sh
```

---

## Appendix A — the two scratch programs, verbatim

The `.bench/ix65/*.sh` drivers are scratch (git-ignored). The two programs that
actually decide a number are small enough to carry here so the receipt is
reproducible without them.

### A.1 Block costs (`phases.awk`)

```awk
FNR == 1 {
  tag = FILENAME
  sub(/^.*\//, "", tag)
  sub(/\.correct$/, "", tag)
  n = 0
}
function class(l) {
  if (l == "initial") return "initial"
  if (l ~ /:pre-replay$/) return "replay"
  if (l ~ /:post-replay$/) return "bridge"
  if (l ~ / play$/) return "play"
  if (l ~ / bridge$/) return "bridge"
  if (l ~ / finalisation$/) return "finalisation"
  return "rewrite"
}
/^STAMP_INF_BEFORE / { before = $2 + 0; next }
/^STAMP_INF_AFTER /  { after = $2 + 0; next }
/^PHASE_MARK / {
  label = ""
  for (i = 2; i < NF; i++) label = label (i > 2 ? " " : "") $i
  n++; lab[n] = label; cnt[n] = $NF + 0
}
END {
  print tag "\t0\tinitial\tinitial\t" (cnt[1] - before)
  for (i = 1; i < n; i++)
    print tag "\t" i "\t" class(lab[i]) "\t" lab[i] "\t" (cnt[i + 1] - cnt[i])
  print tag "\t" n "\t" class(lab[n]) "\t" lab[n] "\t" (after - cnt[n])
}
```

Run one file at a time (`awk -f phases.awk FILE.correct`); `END` is per process.
The stamp lines are printed after the proof, so the marks are buffered and the
costs are computed in `END`.

### A.2 Human `_CORRECT` instrumentation

```awk
# TH = BIGNUM_<OP>_<FAM>_CORRECT, BASE = bignum_<op>_<fam>
BEGIN { state = 0 }
{
  if (state == 0 && index($0, "let " TH " = time prove") == 1) {
    print "let stamp_correct_before_" BASE " = inference_count ();;"
    print "let stamp_correct_t0_" BASE " = Unix.gettimeofday ();;"
    print; state = 1; next
  }
  print
  if (state == 1 && $0 ~ /;;[ \t]*$/) {
    print "let stamp_correct_after_" BASE " = inference_count ();;"
    print "Printf.printf \"HUMAN_CORRECT_INF " BASE " %d\\n%!\" (stamp_correct_after_" BASE " - stamp_correct_before_" BASE ");;"
    print "Printf.printf \"HUMAN_CORRECT_SECONDS " BASE " %.3f\\n%!\" (Unix.gettimeofday () -. stamp_correct_t0_" BASE ");;"
    state = 2
  }
}
```

The `_CORRECT` declaration is `let <TH> = time prove` followed by a term and a
tactic; its first line ending in `;;` terminates the phrase. Only the first
matching declaration is instrumented.

### A.3 Emission

`--mode phase` is the only new emitter path. It was run with a scratch `--spent`
allow-list (`.bench/ix65/spent-manifest.json`) because all 27 tasks are spent and
published; no held-out target was opened and no proof text was read by the
emitter.
