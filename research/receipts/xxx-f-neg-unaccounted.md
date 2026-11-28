# XXX.F — where `neg`'s 2,380,714 goes: the unaccounted preamble, gaps and tail

- **Date:** 2026-11-28
- **Branch:** `staging`, HEAD `cdbcb9f` (clean tree; `main` untouched; nothing committed, nothing pushed)
- **Purpose:** give `arm/neg_p25519` a full inference breakdown that reconciles exactly to
  `STAMP_INFERENCES`, now that IX.102 collapsed the branches 10 → 2, and place every remaining
  inference in a named bucket.
- **Verdict:** **MEASURED — 2,380,714 reconciles exactly, gap = 0.** The "unaccounted" part is
  **353,424** (14.84 % of the proof). Of that, **227,699 (64.4 %) is once-only** and **125,725
  (35.6 %) is branch-dependent**. The largest single unaccounted bucket is the simulator's
  **`ENSURES_FINAL_STATE_TAC` at 142,511 (40.3 % of the unaccounted)** — it is *not* touched by the
  step-theorem cache or the library diet; only an upstream pipeline change reaches it.

All numbers below are from the scratch files under `.bench/xxx-f/` (git-ignored). No tracked file
was edited; the only new tracked file is this receipt.

---

## 1. Method, commands, resources

### 1.1 The capture harness

The capture reuses the **`--mode phase`** answer (which already carries the IX.86/IX.94 marks
`bridge:algebra`, `bridge:algebra-arith`, `bridge:post-flat`, `bridge:pre-agg`, `bridge:post-agg`,
`bridge:post-facts`, `bridge:close-enter`, `bridge:close-exit`), runs it through the IX.89
per-instruction instrumenter (`tests/ix89/instr_gen.pl`, which adds `B:stepNN:*` and the
`G:INT_RING:*` brackets), and then adds a scratch `X:` layer:

- `X:INV` / `X:LEAF` — ordinal + `inference_count` at each `BRIDGE_ALGEBRA_TAC` entry and at each
  oracle (`BRIDGE_ORACLE_TAC`) entry;
- `X:SPLIT` — one line per `ASM_CASES_TAC` in `BRIDGE_SPLIT_CONDITIONS_KEEP_TAC`, so the split
  tree is visible;
- `X:w-*` / `X:stage-*` — wrapper and algebra-stage boundaries;
- `X:hoist-*` / `X:replay-*` — enter, post-`bridge_aggregates`, post-facts, and around the LRAT
  replay inside the carry certificate;
- `X:r-*` — the `bridge_neg_ref_core` reduction stages;
- `X:t-*` — the post-simulation finalisation block.

Every insertion is a `csat_mark` / `csat_mark_at` identity (plus a counter print); none calls a
kernel primitive. Two full runs were made: run 1 with the split/hoist/replay marks, run 2 with the
finalisation and reference-reduction marks added. Both reproduce the baseline exactly (§1.3).

### 1.2 Commands and exit codes

```
free -g                                                              # 96 GB available
TEKMERION_ALG_CLOSE=arm.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT \
  ./target/release/emit --task arm.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT \
  --mode phase --out .bench/xxx-f/neg.raw.answer \
  --spent --spent-manifest .bench/ix66/spent-manifest.json           # rc=0, 304,009 B
perl tests/ix89/instr_gen.pl .bench/xxx-f/neg.raw.answer .bench/xxx-f/neg.sim.answer  # rc=0
python3 .bench/xxx-f/inject.py                                       # rc=0, 307,698 B
bash .bench/xxx-f/build.sh                                           # rc=0 (run 2)
  ./target/release/dropin assemble --task ... --answer .bench/xxx-f/neg.answer --stamp   # rc=0
  tests/ix89/stmt_check.sh inject neg                                # rc=0
  tools/cap.sh --timeout 1800 12G -- make -j1 -B HOLDIR=... curve25519/bignum_neg_p25519.correct
  tests/ix89/stmt_check.sh verdict neg .bench/xxx-f/neg.correct      # rc=0 STATEMENT_OK
python3 .bench/xxx-f/analyze.py                                      # rc=0
```

The emitted phase answer is **byte-identical** to the IX.89 capture:
`sha256 59bf32c6…8df7fd` for both `.bench/xxx-f/neg.raw.answer` and
`.bench/ix89/gen/arm_neg_p25519.phase.raw.answer`.

**One build at a time, capped, never raised.** Two capped runs, sequential; the cap did not fire:

```
run 1: cap.sh: group=3291718 rc=0 sampled_group_peak_rss_kb=1886420 max_process_vmhwm_kb=1785432
run 2: cap.sh: group=3294401 rc=0 sampled_group_peak_rss_kb=1884024 max_process_vmhwm_kb=1800116
```

`free -g` before each run: **96 GB available** (≥ 3 × the 12 G cap). Wall time 24–25 s each.

### 1.3 `STAMP_INFERENCES` before / after

| run | file | `STAMP_INFERENCES` |
|---|---|---:|
| baseline plain (`cdbcbf9`) | `.bench/ix89/gen/arm_neg_p25519.plain.correct` | **2,380,714** |
| baseline phase (`cdbcbf9`) | `.bench/ix89/gen/arm_neg_p25519.phase.correct` | **2,380,714** |
| scratch run 1 | `.bench/xxx-f/neg.correct.run1` | **2,380,714** |
| scratch run 2 (receipted) | `.bench/xxx-f/neg.correct` | **2,380,714** |

Run 2: `STAMP_INF_BEFORE 67649313`, `STAMP_INF_AFTER 70030027`, difference **2,380,714**, equal to
`STAMP_INFERENCES`. Statement check in the same session:

```
STATEMENT_OK task=arm.bignum_neg_p25519 aconv=1 hyps=0 axioms=3
```

The instrumentation is therefore inference-neutral. `git --no-optional-locks status --short` is
empty; `.bench/` is ignored by `.gitignore:11`.

---

## 2. The split structure (why 3 tactical leaves, 2 oracle branches)

The scratch ordinal marks resolve the IX.102 "2 branches" precisely:

```
X:SPLIT 1  69891151          wrapper BRIDGE_SPLIT_CONDITIONS_KEEP_TAC
X:w-postsplit 69892519 (x2)  -> two subgoals
X:INV 1 69893417             BRIDGE_ALGEBRA_TAC on the zero case
X:SPLIT 2 69947440           the non-zero case splits again
X:LEAF 1 69968436            oracle leaf 1
X:LEAF 2 69994009            oracle leaf 2
```

So the proof has **three tactical leaves**: the **zero case**, closed by `BRIDGE_INT_REDUCE_TAC`
without ever reaching the oracle, and the **two non-zero oracle leaves**. This is exactly IX.102
§1 ("the zero case is *closed* … the non-zero case still splits the 256-bit guard into two closes").
The two oracle leaves are the "2 branches"; the zero case is a cheap leaf, not an oracle branch.
The two oracle leaves abut exactly (`close-exit` 69,994,009 = `algebra-arith` 69,994,009): the
inter-split gap is **0**.

---

## 3. Reconciliation table (the mandatory exact sum)

`analyze.py` asserts `sum(buckets) == STAMP_INFERENCES`; it passes.

| # | bucket | inferences | share | once / per-branch |
|---|---|---:|---:|---|
| 1 | support library (PRE: `STAMP_INF_BEFORE` → `00 rewrite`) | 1,497,352 | 62.90 % | once |
| 2 | setup / init + digitise (`00 rewrite` → `05 play`) | 1,076 | 0.05 % | once |
| 3 | simulation (19 instructions, `05 play` → `06 finalisation`) | 528,862 | 22.21 % | once |
| 4 | post-sim finalisation (`06 finalisation` → `09 bridge`) | 142,983 | 6.01 % | once |
| 5 | bridge: reference reduction (`09 bridge` → `X:w-enter`) | 24,705 | 1.04 % | once |
| 6 | bridge: wrapper canon-zero / TO_WORD / ALG_NORM | 10,097 | 0.42 % | once |
| 7 | bridge: hoist aggregate construction | 12,213 | 0.51 % | once |
| 8 | bridge: hoist fact setup (pre-replay) | 10,978 | 0.46 % | once |
| 9 | bridge: LRAT replay (carry certificate) | 13,572 | 0.57 % | once |
| 10 | bridge: wrapper split + zero-test + expand | 2,266 | 0.10 % | once |
| 11 | branch: zero-case leaf (closed by reduce, no oracle) | 25,110 | 1.05 % | per leaf |
| 12 | branch: non-zero group setup (to first oracle entry) | 49,909 | 2.10 % | once for the group |
| 13 | branch: oracle leaf 1 (`linear-cert`) | 25,573 | 1.07 % | per branch |
| 14 | branch: oracle leaf 2 (`prop-kind=ac`) | 25,133 | 1.06 % | per branch |
| 15 | inter-split gap (leaf 1 → leaf 2) | **0** | 0.00 % | — |
| 16 | tail: last close-exit → `STAMP_INF_AFTER` | 10,885 | 0.46 % | once |
| | **total** | **2,380,714** | 100.00 % | |

Check: `1,497,352 + 1,076 + 528,862 + 142,983 + 24,705 + 10,097 + 12,213 + 10,978 + 13,572 +
2,266 + 25,110 + 49,909 + 25,573 + 25,133 + 0 + 10,885 = 2,380,714`
`= STAMP_INF_AFTER − STAMP_INF_BEFORE = 70,030,027 − 67,649,313`. Shares are rounded; the sum is
asserted, not rounded.

The **unaccounted** region (everything beyond library + simulation + init) is
`2,380,714 − 1,497,352 − 1,076 − 528,862 = 353,424`.

### 3.1 Post-simulation finalisation, split (bucket 4)

| inferences | stage |
|---:|---|
| **142,511** | `ENSURES_FINAL_STATE_TAC` |
| 61 | explicit `ASM_REWRITE_TAC[]` |
| 331 | `CONV_TAC(LAND_CONV BIGNUM_EXPAND_CONV)` (`07 rewrite`) |
| 80 | `ASM_REWRITE_TAC[]` + `DISCARD_STATE_TAC "s19"` (`08 finalisation`) |

### 3.2 Reference reduction, split (bucket 5)

| inferences | stage |
|---:|---|
| 150 | `ELIM_DIGIT_EQS_TAC` |
| 86 | `BRIDGE_REWRITE_MODULUS` |
| 6 | `expand` head (`ONCE_DEPTH_CONV BIGNUM_OF_WORDLIST_CONV`) |
| **24,372** | reference lemma construction (`NEG_REF_VAL_WORD` instantiation, `BRIDGE_REF_SIDES`, `MP`) |
| 34 | `REWRITE_TAC eqn` |
| 57 | `REWRITE_TAC[rth; ref_neg; …]` |

### 3.3 The two oracle leaves (buckets 13–14)

| sub-phase | leaf 1 (`linear-cert`) | leaf 2 (`prop-kind=ac`) |
|---|---:|---:|
| hypothesis flattening (`entry` → `post-flat`) | 7,494 | 7,496 |
| aggregate construction (`pre-agg` → `post-agg`) | 12,213 | 12,213 |
| facts (`post-agg` → `post-facts`) | **0** | **0** |
| antecedent assembly (`post-facts` → `close-enter`) | 4,631 | 5,311 |
| close (`close-enter` → `close-exit`) | 1,235 | 113 |
| **total** | **25,573** | **25,133** |

The close sum is **1,348** (1,235 + 113), the IX.102 figure reproduced per leaf.
**Per-branch facts are 0**: the carried fact `&(bitval carry_s12) = 0` is `reused
why=antecedent-hoist` in both leaves. IX.98's hoist moved the whole fact derivation once, above the
split; the once-only derivation is the LRAT replay, **13,572** — exactly the "facts 13,572" line.

---

## 4. Simulation, per instruction

Cost of instruction *n* = `B:step(n+1)` − `B:step n` (the last is `06 finalisation` − `B:step19`).

| step | class | inferences | | step | class | inferences |
|---|---|---:|---|---|---|---:|
| 01 | plain | 22,445 | | 11 | plain | 13,334 |
| 02 | plain | 16,901 | | 12 | acc | 16,665 |
| 03 | plain | 15,162 | | 13 | plain | 27,808 |
| 04 | acc | 53,034 | | 14 | plain | 18,598 |
| 05 | plain | 20,219 | | 15 | plain | 16,481 |
| 06 | acc | 43,761 | | 16 | plain | 13,269 |
| 07 | plain | 47,189 | | 17 | plain | 13,231 |
| 08 | plain | 18,518 | | 18 | plain | 50,187 |
| 09 | acc | 42,448 | | 19 | plain | 61,125 |
| 10 | plain | 18,487 | | | | |

- **total = 528,862**
- acc steps (4): **155,908** (29.48 %); plain steps (15): **372,954** (70.52 %)
- largest six steps (19, 04, 18, 07, 06, 09): **297,744** (56.31 % of simulation)

---

## 5. Direct answers

### 5.1 Exact reconciliation

Yes: §3 sums to **2,380,714** exactly. The gap between the two oracle leaves is **0**; every
inference is inside a bracketed stage. The only post-branch region is the 10,885 tail
(finalisation/assembly/returns after the last `close-exit`), bracketed by `bridge:close-exit` and
`STAMP_INF_AFTER`.

### 5.2 Once-only vs per-branch

Within the **unaccounted 353,424**:

| kind | inferences | share of unaccounted |
|---|---:|---:|
| once-only (finalisation 142,983 + bridge preamble 73,831 + tail 10,885) | **227,699** | 64.42 % |
| branch-dependent (zero-case leaf 25,110 + group setup 49,909 + 2 oracle leaves 50,706) | **125,725** | 35.58 % |

Over the whole proof: once-only **2,254,989 (94.72 %)**, branch-dependent **125,725 (5.28 %)**.
Strictly per **oracle leaf** the cost is **50,706 (2.13 % of the proof)**, average **25,353**.

### 5.3 Largest single unaccounted bucket, and the lever

**Largest single unaccounted bucket: `ENSURES_FINAL_STATE_TAC` = 142,511** (40.3 % of the
353,424 unaccounted; 5.99 % of the proof). Its source is the s2n-bignum framework
(`ENSURES_OR_ENSURES_N_TAC`, `BETA_THM`, `CONJ_ACI_RULE`, `MONOTONE_MAYCHANGE_TAC`), applied to the
machine state after the simulation. Against the three levers:

| lever | reaches it? | number |
|---|---|---:|
| (a) step-theorem cache | **no** — it derives no simulation step lemma; the cache targets the 528,862 simulation block | **0** of 142,511 |
| (b) library diet | **no** — the diet's target is the PRE construction (1,497,352); this is a tactic application over the state, not library construction | **0** measured overlap |
| (c) bridge/pipeline change | **only route** — but it is *upstream* of the bridge (the reference/simulator finalisation), so it needs a change to the emitted finalisation, not to `bridge.ml` | up to **142,511** |

If "tail" is read to include finalisation, the whole tail is **153,868** (142,983 finalisation +
10,885 return).

**Largest *bridge*/pipeline-addressable unaccounted bucket** is the **non-zero group setup, 49,909**
(2.10 %). Inside the branch side the concrete, measured opportunity is the **repeated aggregate
construction**:

| call | inferences |
|---|---:|
| pre-split hoist | 12,213 |
| zero-case leaf hoist | 10,338 |
| non-zero group hoist | 12,213 |
| oracle leaf 1 | 12,213 |
| oracle leaf 2 | 12,213 |
| **total across 5 calls** | **59,190** |

The `AGGREGATE` term printed in all five calls is identical (`limbs=4`, base `2^64`, weights
`[1, 2^64, 2^128, 2^192]`), so a pipeline change that builds it once and threads the theorem into
the hoist and the two oracle closes has a **ceiling of 46,977** (5 × 12,213 − 12,213). This is a
design ceiling from identical printed terms, **not** a measured ablation. The zero-case leaf
(25,110) is a second pipeline candidate: it is closed by `reduce` without an oracle and could be
fast-pathed.

**Overall, the single largest addressable bucket is not in the unaccounted region at all**: it is
the support library, **1,497,352 (62.90 %)**, which only lever (b), the library diet, reaches.

### 5.4 Simulation vs the cache target

Measured simulation total = **528,862**, confirming the review's **529 K** baseline. The per-
instruction split (§4) shows the cache must cut both classes: 4 acc steps cost 155,908 (29.5 %) and
15 plain steps cost 372,954 (70.5 %); six steps alone are 297,744 (56.3 %).

Against the projection **529 K → ~100 K**: the required saving is **428,862** (a **4.29×**
reduction on the measured total; 5.29× from 100 K), i.e. **18.01 %** of the whole 2,380,714 proof.
This receipt confirms the baseline exactly; it does **not** demonstrate that the cache reaches
~100 K — that is the cache's own measurement.

---

## 6. Files and reproduction

Scratch (git-ignored):
- `.bench/xxx-f/neg.raw.answer` — `--mode phase` answer (304,009 B)
- `.bench/xxx-f/neg.sim.answer` — after `instr_gen.pl`
- `.bench/xxx-f/neg.answer` — after `inject.py` (307,698 B)
- `.bench/xxx-f/neg.correct`, `.bench/xxx-f/neg.correct.run1` — the two run logs
- `.bench/xxx-f/inject.py`, `.bench/xxx-f/build.sh`, `.bench/xxx-f/analyze.py`
- `.bench/xxx-f/neg-breakdown.txt` — the raw dump this receipt is generated from

```
cd /mnt/1tb_kingston/Tekmerion
free -g
TEKMERION_ALG_CLOSE=arm.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT \
  ./target/release/emit --task arm.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT \
  --mode phase --out .bench/xxx-f/neg.raw.answer \
  --spent --spent-manifest .bench/ix66/spent-manifest.json
perl tests/ix89/instr_gen.pl .bench/xxx-f/neg.raw.answer .bench/xxx-f/neg.sim.answer
python3 .bench/xxx-f/inject.py
bash .bench/xxx-f/build.sh          # capped: tools/cap.sh --timeout 1800 12G -- make -j1
python3 .bench/xxx-f/analyze.py     # asserts sum(buckets) == STAMP_INFERENCES
```

---

## 7. Not established

- The `ENSURES_FINAL_STATE_TAC` lever attribution is **analytic** (the tactic source is quoted in
  §5.3), not an experimental ablation.
- The 46,977 aggregate-sharing figure is a **ceiling** from five identical printed aggregate terms,
  not a measured saving; the branch contexts differ syntactically even though the built term is
  the same.
- Whether the step-theorem cache reaches ~100 K is the cache's own measurement; only the 528,862
  baseline is confirmed here.
- Shares in §3 are rounded; the exact sum is asserted, not rounded.

---

## 8. One-line verdict

```
NEG BREAKDOWN: library 1497352, simulation 528862, branches 125725, gaps 0, tail 153868, other 74907; largest unaccounted ENSURES_FINAL_STATE_TAC 142511 -> (c) pipeline only, not (a)/(b); largest bridge-addressable unaccounted = non-zero group setup 49909 (repeated aggregate ceiling 46977)
```
