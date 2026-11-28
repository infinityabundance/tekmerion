# XXVIII.C4 — per-branch pre-close cost of `arm.bignum_neg_p25519`

- **Date:** 2026-11-28
- **Branch:** `staging`, HEAD `fd3c188` (clean tree; `main` untouched; nothing committed, nothing pushed)
- **Purpose:** measure, for `arm/neg_p25519`, the pre-close cost of each of the ten oracle
  branches and the close cost, so the ten branches plus the work outside the oracle reconcile to
  `STAMP_INFERENCES`.
- **Verdict:** **MEASURED — 10 branches reconcile exactly to 4,121,086.** The aggregate-fact
  phase is **100 % per-branch** (1,511,696; once = 0). Pruning the nine vacuous branches at
  creation is **less** than a perfect facts certificate (1,089,215 vs 1,511,696) and is a
  *complement* to it, not a substitute.

All numbers below are from the scratch files under `.bench/xxviii/` (git-ignored):
`neg.correct` (the run log, 22,681 lines), `neg-branch-dump.txt` (the parsed per-branch dump),
`neg.raw.answer` / `neg.answer` (the phase answer, before/after the ordinal tag). The tracked
`tools/bridge.ml` and `src/` were **not** edited.

---

## 1. Method, commands, resources

### 1.1 What was run

The capture reuses the `--mode phase` answer. That mode already emits the boundaries added by
IX.86/IX.94 (`bridge:post-agg`, `bridge:post-flat`, `bridge:post-facts`, `bridge:close-enter`,
`bridge:close-exit`), so the XXVI dump harness (which injected a pre-close dump) is not needed;
the only scratch addition is a branch-ordinal counter at the oracle entry. No tracked file was
touched.

```
free -g                                                                 # 0   75 GB available
TEKMERION_ALG_CLOSE=arm.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT \
  ./target/release/emit --task arm.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT \
  --mode phase --out .bench/xxviii/neg.raw.answer \
  --spent --spent-manifest .bench/ix66/spent-manifest.json               # 0   248,278 B
python3 .bench/xxviii/inject_ord.py                                     # 0   248,429 B
bash .bench/xxviii/build.sh                                             # 0
  ./target/release/dropin assemble --task ... --answer .bench/xxviii/neg.answer --stamp  # 0
  tests/ix89/stmt_check.sh inject neg                                    # 0
  tools/cap.sh --timeout 1800 12G -- make -j1 -B HOLDIR=... \
    curve25519/bignum_neg_p25519.correct                                 # rc=0 (26 s)
  tests/ix89/stmt_check.sh verdict neg .bench/xxviii/neg.correct         # 0   STATEMENT_OK
python3 .bench/xxviii/analyze.py                                        # 0
```

**One scratch iteration, reported for honesty:** the first injection used `let xxviii_ord = ref 0;;`
inside a `let ... in` chain and failed to parse (rc=2, `[and_let_binding] expected`). The corrected
form is `let xxviii_ord = ref 0 in`; the receipted run is the corrected one.

Resource line printed by the capped run:

```
cap.sh: group=2704398 rc=0 sampled_group_peak_rss_kb=2012276 max_process_vmhwm_kb=1822912
```

Peak group RSS ≈ 1.92 GB, max process `VmHWM` ≈ 1.74 GB, well under the 12 G cap; the cap did
**not** fire. `free -g` before the run: **75 GB available** (≥ 3× the 12 G cap, i.e. 36 G; the
cap stayed 12 G).

### 1.2 `STAMP_INFERENCES` before / after

| run | file | `STAMP_INFERENCES` |
|---|---|---:|
| baseline plain, recorded 2026-11-28 00:04:55 | `.bench/ix89/gen/arm_neg_p25519.plain.correct` | **4,121,086** |
| baseline phase, recorded 2026-11-28 00:08 | `.bench/ix89/gen/arm_neg_p25519.phase.correct` | **4,121,086** |
| this instrumented run | `.bench/xxviii/neg.correct` | **4,121,086** |

The instrumented run is byte-identical in cost: `STAMP_INF_BEFORE 67649313`,
`STAMP_INF_AFTER 71770399`, difference **4,121,086**, equal to `STAMP_INFERENCES`. The ordinal tag
is an `incr`, a `Printf.printf` and a read of `!csat_mark_reader ()` (which calls
`inference_count`); it calls no kernel function. The statement/axiom check is in the same session:

```
STATEMENT_OK task=arm.bignum_neg_p25519 aconv=1 hyps=0 axioms=3
```

Hashes of the inputs and the run:

```
ae942c74d63fbe1f3f450bc3ca3199c264960a7653c96a517de8312116fb42ae  tools/bridge.ml
7011beb8629648ca7d7e4c16c37b2bbb1f4f8b7c41b7df0d1473f7e85ea8c2c1  src/tools/emit.rs
ade61d8abe796a9b5e4e8427c5bcc1ede011d71f2981399fa08a76776c5f0f89  .bench/xxviii/neg.answer
1a2592bd398f90e8788b411be61838e1ad118c03cfb9129b9753b3a93a2157a6  .bench/xxviii/neg.correct
```

---

## 2. What each mark measures

The marks are emitted by `src/tools/emit.rs` (`BRIDGE_EDITS`), inserted into the `--mode phase`
copy of `tools/bridge.ml`; they are absent from `--mode answer`, so the normal proof is unchanged.
Their positions in `BRIDGE_ORACLE_TAC` are:

| mark | code point | meaning |
|---|---|---|
| `XXVIII_BRANCH n x` | wrapper around the oracle entry (scratch) | ordinal `n` and `inference_count` at entry, before any work |
| `bridge:algebra-arith` | oracle entry wrapper | same point, official mark |
| `bridge:pre-agg` | just before `let aggregates = bridge_aggregates eqs` | end of goal parse + Gauss-Jordan + `bridge_oracle_facts` |
| `bridge:post-agg` | just after `bridge_aggregates` | end of aggregate construction |
| `bridge:post-flat` | just after `let all_hyps = List.map ... bridge_flat_th ...` | **facts start**; end of hypothesis flattening |
| `bridge:post-facts` | just after `bridge_aggregate_facts` | **facts end** |
| `bridge:close-enter` | after `ants` is fixed, before the prop/range/slim/full attempts | end of pre-close |
| `bridge:close-exit` | on every route that returns a closed theorem | end of close |

**Code order vs the brief's labels.** In the code, `bridge_aggregate_facts` runs *after* the
Gauss-Jordan reduction, aggregate construction and hypothesis flattening. Therefore the brief's
`oracle_entry → facts_start` bucket contains those three, and `facts_end → close_enter` is only
the antecedent assembly. §3.2 splits the first bucket into its actual contents. This is a
definition difference, not a measurement defect.

`bridge:pre-agg == bridge:algebra-arith` in **every** branch (0 inferences): on this task the
goal parsing, the Gauss-Jordan reduction and `bridge_oracle_facts` add no primitive inferences
(the relaxation path emits no facts here — every carrier logs
`ORACLE undetermined ... why=relaxation`).

---

## 3. The ten branches

### 3.1 Requested intervals (inferences)

Columns are the brief's labels: `E→FS` = `oracle_entry → facts_start`,
`FS→FE` = `facts_start → facts_end` (the fact derivation),
`FE→CE` = `facts_end → close_enter` (antecedent assembly),
`CE→CX` = `close_enter → close_exit` (the close), and the branch total `E→CX`.
`route` is the close route actually taken (`ORACLE close` line).

| # | route | E→FS | FS→FE (facts) | FE→CE | CE→CX (close) | branch total |
|---:|:---|---:|---:|---:|---:|---:|
| 1 | prop | 18,013 | 27,017 | 1,739 | 108 | 46,877 |
| 2 | literal | 17,997 | 22,159 | 1,739 | 15 | 41,910 |
| 3 | linear | 19,739 | **629,173** | 4,631 | 1,235 | 654,778 |
| 4 | literal | 19,723 | 39,288 | 4,631 | 15 | 63,657 |
| 5 | ac | 19,741 | **629,243** | 5,311 | 113 | 654,408 |
| 6 | literal | 19,725 | 39,358 | 5,311 | 15 | 64,409 |
| 7 | prop | 19,767 | 31,151 | 2,897 | 108 | 53,923 |
| 8 | range | 19,727 | 31,508 | 2,897 | 170 | 54,302 |
| 9 | prop | 19,769 | 31,221 | 3,577 | 108 | 54,675 |
| 10 | range | 19,729 | 31,578 | 3,577 | 170 | 55,054 |
| **sum** | | **193,930** | **1,511,696** | **36,310** | **2,057** | **1,743,993** |

The close sum is **2,057**, the published IX.95 figure, reproduced per branch
(108, 15, 1,235, 15, 113, 15, 108, 170, 108, 170).

### 3.2 The `E→FS` bucket split into what it actually contains

| # | Gauss-Jordan + `bridge_oracle_facts` | aggregate construction | hypothesis flattening | E→FS |
|---:|---:|---:|---:|---:|
| 1 | 0 | 12,213 | 5,800 | 18,013 |
| 2 | 0 | 12,213 | 5,784 | 17,997 |
| 3 | 0 | 12,213 | 7,526 | 19,739 |
| 4 | 0 | 12,213 | 7,510 | 19,723 |
| 5 | 0 | 12,213 | 7,528 | 19,741 |
| 6 | 0 | 12,213 | 7,512 | 19,725 |
| 7 | 0 | 12,213 | 7,554 | 19,767 |
| 8 | 0 | 12,213 | 7,514 | 19,727 |
| 9 | 0 | 12,213 | 7,556 | 19,769 |
| 10 | 0 | 12,213 | 7,516 | 19,729 |
| **sum** | **0** | **122,130** | **71,800** | **193,930** |

The full aggregate-fact phase (`oracle_entry → facts_end`) is therefore
**0 + 122,130 + 71,800 + 1,511,696 = 1,705,626**, and it is entirely per branch.

### 3.3 Absolute mark values

The raw dump (`.bench/xxviii/neg-branch-dump.txt`) carries these; repeated here so the receipt is
self-contained.

| # | entry | pre-agg | post-agg | post-flat | post-facts | close-enter | close-exit |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 1 | 69,882,601 | 69,882,601 | 69,894,814 | 69,900,614 | 69,927,631 | 69,929,370 | 69,929,478 |
| 2 | 69,929,478 | 69,929,478 | 69,941,691 | 69,947,475 | 69,969,634 | 69,971,373 | 69,971,388 |
| 3 | 70,043,307 | 70,043,307 | 70,055,520 | 70,063,046 | 70,692,219 | 70,696,850 | 70,698,085 |
| 4 | 70,698,085 | 70,698,085 | 70,710,298 | 70,717,808 | 70,757,096 | 70,761,727 | 70,761,742 |
| 5 | 70,761,742 | 70,761,742 | 70,773,955 | 70,781,483 | 71,410,726 | 71,416,037 | 71,416,150 |
| 6 | 71,416,150 | 71,416,150 | 71,428,363 | 71,435,875 | 71,475,233 | 71,480,544 | 71,480,559 |
| 7 | 71,540,731 | 71,540,731 | 71,552,944 | 71,560,498 | 71,591,649 | 71,594,546 | 71,594,654 |
| 8 | 71,594,654 | 71,594,654 | 71,606,867 | 71,614,381 | 71,645,889 | 71,648,786 | 71,648,956 |
| 9 | 71,648,956 | 71,648,956 | 71,661,169 | 71,668,725 | 71,699,946 | 71,703,523 | 71,703,631 |
| 10 | 71,703,631 | 71,703,631 | 71,715,844 | 71,723,360 | 71,754,938 | 71,758,515 | 71,758,685 |

### 3.4 Fact route and premise set

Every branch takes the same route, `ORACLE fact &(bitval carry_s12) = 0 why=aggregate-band`
(`aggregates = 1` in every branch). `FACT_PREM` prints the premise count and total term size at
the call:

| # | route | facts | `FACT_PREM hyps` | `size` |
|---:|:---|---:|---:|---:|
| 1 | prop | 27,017 | 29 | 3,009 |
| 2 | literal | 22,159 | 30 | 3,138 |
| 3 | linear | **629,173** | 30 | 4,356 |
| 4 | literal | 39,288 | 31 | 4,485 |
| 5 | ac | **629,243** | 30 | 4,358 |
| 6 | literal | 39,358 | 31 | 4,487 |
| 7 | prop | 31,151 | 31 | 4,415 |
| 8 | range | 31,508 | 32 | 4,646 |
| 9 | prop | 31,221 | 31 | 4,417 |
| 10 | range | 31,578 | 32 | 4,648 |

The two expensive branches are not the two largest premise sets (B8/B10 are larger and cheap).
They are the leaf whose premises are **consistent** (B3, the only substantive leaf) and the leaf
whose inconsistency is **polynomial-AC only** (B5, which the close has to see through
`+`/`*` flattening). The other eight leaves carry a literal or `∧`-AC complementary pair, and
their fact proof is cheap. The inference counts support this correlation; the exact prover path is
not separately instrumented here.

---

## 4. Reconciliation to `STAMP_INFERENCES`

| bucket | inferences |
|---|---:|
| preamble: `STAMP_INF_BEFORE` → first oracle entry | 2,233,288 |
| — support library (`STAMP_INF_BEFORE` → `00 rewrite`) | 1,497,352 |
| — init + digitise (phase A) | 1,076 |
| — simulation (phase B) | 528,862 |
| — finalisation / rewrite / bridge entry (phase E) | 205,998 |
| ten oracle branches, entry → exit | 1,743,993 |
| — of which pre-close (`entry → close-enter`) | 1,741,936 |
| — of which close (`close-enter → close-exit`) | 2,057 |
| split / assembly gap, branch 2 → 3 | 71,919 |
| split / assembly gap, branch 6 → 7 | 60,172 |
| tail: last oracle exit → `STAMP_INF_AFTER` | 11,714 |
| **total** | **4,121,086** |

Check: `2,233,288 + 1,743,993 + 71,919 + 60,172 + 11,714 = 4,121,086`, equal to
`STAMP_INF_AFTER − STAMP_INF_BEFORE = 71,770,399 − 67,649,313 = 4,121,086`, equal to
`STAMP_INFERENCES`. The difference outside the oracle is **2,377,093** (57.68 %), composed of the
preamble (library + simulation + entry), the two split gaps, and the tail assembly.

The two gaps are the only inter-branch work not inside an oracle invocation; all other adjacent
branches abut exactly (branch `n` exit = branch `n+1` entry). The gaps sit between branches 2→3
and 6→7, where the splitter had to re-enter the algebra route.

---

## 5. Direct answers

### 5.1 How much of the ≈1.51 M fact phase is per branch vs once?

**Per branch: 1,511,696 (100 %). Once: 0.** Each of the ten branches calls
`bridge_aggregate_facts` once, and the same theorem `&(bitval carry_s12) = 0` is re-proved in
every branch. There is no once-only derivation in the current code. The distribution is highly
skewed: B3 (629,173) + B5 (629,243) = **1,258,416 (83.2 %)**, the other eight = **253,280
(16.8 %)**. Adding the per-branch aggregate construction (122,130) and flattening (71,800), the
whole `oracle_entry → facts_end` phase is **1,705,626**, also 100 % per branch. Whether the fact
can be proved once from a branch-independent premise set is the design question of `XXVIII.C2`;
it is not established by this measurement.

### 5.2 Do the nine vacuous branches pay the same fact cost as the one substantive branch?

**Not uniformly, but they pay more in total.** The nine vacuous branches pay **882,523 (58.4 %
of the facts phase)**; the one substantive branch B3 pays **629,173 (41.6 %)**. As a group the nine
vacuous pay **1.40×** B3. Individually:

- **B5** (vacuous, `ac` route) pays **629,243**, i.e. **+70 inferences (+0.011 %)** over B3. It
  pays the full substantive price because its contradiction is only polynomial-AC and the fact
  prover does not exploit it.
- The other **eight** vacuous branches pay **22,159–39,358** each (sum **253,280**), i.e.
  **3.5–6.3 %** of B3.
- All ten branches reach and pay the fact phase; vacuity does **not** bypass it. The only branch
  that *must* pay is B3 (consistent), and one vacuous branch (B5) pays the same.

### 5.3 Is pruning-at-creation worth more than certificate-based facts?

**No — it is worth less, and the two are complementary.** The numbers:

| lever | inferences saved | % of 4,121,086 |
|---|---:|---:|
| prune the nine vacuous branches at creation (whole branch) | **1,089,215** | 26.43 % |
| — of which fact derivation | 882,523 | 21.41 % |
| — of which pre-close non-fact (aggregate + flatten + gauss) | 174,191 | — |
| — of which antecedent assembly | 31,679 | — |
| — of which close | 822 | — |
| prune *and* the two split gaps vanish (upper bound) | ≤ 1,221,306 | 29.64 % |
| perfect certificate-based facts, all ten branches | **1,511,696** | 36.68 % |
| prune nine + certificate the remaining B3 facts | **1,718,388** | 41.70 % |

- A perfect facts certificate is the **larger single lever**: 1,511,696 vs 1,089,215
  (**1.39×**; even against the gap-inclusive pruning bound, 1.51 M vs 1.22 M, **1.24×**). It also
  reaches the 41.6 % of the fact phase that sits on the **unprunable** substantive branch B3.
- Pruning is the only lever for the **206,692** of non-fact work on the nine vacuous branches
  (174,191 pre-close + 31,679 assembly + 822 close). After a perfect facts certificate, pruning's
  marginal value is exactly that 206,692; after pruning, a facts certificate's marginal value is
  B3's 629,173. Certificates have the higher marginal value in either order.
- The best combination saves **1,718,388** (41.70 %) and leaves **2,402,698**:
  `2,377,093` outside the oracle + B3's non-fact `25,605`. Doing either lever alone leaves
  `4,121,086 − 1,089,215 = 3,031,871` (prune only) or `4,121,086 − 1,511,696 = 2,609,390`
  (certificate only).

**Conclusion:** the premise “pruning at creation beats any further fact optimisation” is **not
supported** by the inference counts. The fact phase is per-branch, but 41.6 % of it is on the one
branch that cannot be pruned, and the nine vacuous branches carry only 0.21 M of non-fact work.
Certificate-based facts is the larger lever; pruning-at-creation is the complement that recovers
the remaining 0.21 M.

---

## 6. Provenance of the 1,511,696 figure

The measured `facts_start → facts_end` sum is **1,511,696**, exactly the IX.89/IX.91 phase-C bucket
(`research/receipts/ix89-human-phases.json`, `generated_algebraic.neg.phases.C = 1511696`;
`research/receipts/ix91-b1-fastpath.md` line 49). This measurement is at `fd3c188`, after
IX.92–IX.97, and the fact sub-phase is unchanged; the *full* aggregate-fact phase including
aggregate construction and flattening is **1,705,626**. The historical figure is therefore a
facts-only number, not the whole build-to-facts region. No cross-experiment subtraction is used:
the close is the separately measured `close-enter → close-exit` interval (2,057), not a phase-D
fallback bucket.

---

## 7. Reproduction

```
cd /mnt/1tb_kingston/Tekmerion
free -g
TEKMERION_ALG_CLOSE=arm.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT \
  ./target/release/emit --task arm.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT \
  --mode phase --out .bench/xxviii/neg.raw.answer \
  --spent --spent-manifest .bench/ix66/spent-manifest.json
python3 .bench/xxviii/inject_ord.py
bash .bench/xxviii/build.sh          # capped: tools/cap.sh --timeout 1800 12G -- make -j1
python3 .bench/xxviii/analyze.py     # writes .bench/xxviii/neg-branch-dump.txt
grep -E '^XXVIII_BRANCH|^PHASE_MARK bridge:(algebra-arith|post-agg|post-flat|post-facts|close-enter|close-exit)|^STAMP_INF' \
  .bench/xxviii/neg.correct
```

Scratch harness: `.bench/xxviii/inject_ord.py`, `.bench/xxviii/build.sh`,
`.bench/xxviii/analyze.py`. Raw dump: `.bench/xxviii/neg-branch-dump.txt`.

---

## 8. One-line verdict

```
PER-BRANCH: facts total 1511696 (per-branch 1511696 = 100%, avg 151170; vacuous 882523), close 2057; pruning-at-creation less than fact certificates (1089215 vs 1511696; both = 1718388)
```
