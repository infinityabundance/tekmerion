# IX.66 — splitting the bridge block: bitblast, clausification, normalisation, replay

- **Date:** 2026-11-24
- **Branch:** `staging`
- **Change under test:** `src/tools/emit.rs` gains IX.66 boundary markers that are
  applied **only to the copies of `tools/csat_lib.ml` and `tools/bridge.ml` that
  `--mode phase` inlines**. `--mode answer` is untouched, and `tools/` is
  unmodified (§5). The markers bracket, with `inference_count ()`:
  the entry to each reference close, the goal handed to each SAT close, the handover
  from `BITBLAST_THEN` to `CSAT_PROVE`, the end of clausification, the IX.65
  replay bracket, and the return of each SAT close.
- **Answer, one line:** the 67.85% that IX.65 left merged is **49.03% bit-blasting**
  (487,167,871 of 993,573,633) and **17.95% definitionalise+clausify**
  (178,375,918). The certified replay is **30.35%** (301,587,637, unchanged from
  IX.65). `BRIDGE_TO_WORD_CONV` and the surrounding normalisation — the class the
  brief suspected — is **0.04%** (412,923). The residual is **0.82%**, of which the
  DIMACS/solver/trim step is **exactly zero** primitive inferences on all six
  theorems.
- **Verdict:** the class that would matter if removed is **bitblast**. It is the
  largest single class, larger than the replay by 1.6×, and its absolute size is what
  makes `add_p521` 470.3M rather than 241.4M. **Upper bound, not a prediction:**
  removing the bit-blasting of the goal term entirely would take `add_p521` from
  470,294,392 to **241,354,034** and the six-task total from 993,573,633 to
  **506,405,762**. No such lever exists today (see §4).

---

## 1. What was measured, and what was added

IX.65 bracketed the emitted chain at the emitter's own lines. `BRIDGE_*_REF_TAC`
occupies one such line, so everything from `ELIM_DIGIT_EQS_TAC` to the last certified
replay came out as one number. The boundaries inside it are inside library functions
and cannot be bracketed from the call site, so the functions had to be instrumented.

They were instrumented **without editing them**. `--mode phase` now reads
`tools/csat_lib.ml` and `tools/bridge.ml` and applies a fixed table of
`(plain, marked)` text pairs (`CSAT_LIB_EDITS`, `BRIDGE_EDITS` in `emit.rs`) before
inlining; `--mode answer` inlines the tracked files verbatim. This differs from
IX.65, which put its markers in the tracked library, and it is what makes the
normal answer byte-identical to IX.65's (§5) rather than merely
tactic-identical. Each anchor is required to occur **exactly once**, so a drifted
anchor fails the emission instead of silently measuring nothing.

The marker phrases, all inert when `csat_mark_enabled` is false:

| marker | where | opens/closes |
|---|---|---|
| `PHASE_GOAL bridge:goal <n>` | `BRIDGE_{ADD,SUB,NEG}_REF_TAC`, `..._NEG_REF_ZERO_TAC` entry | size of the obligation the bridge inherits |
| `bridge:pre-bitblast` | `BRIDGE_CSAT_TAC` entry, before `prove(imp, CSAT_TAC)` | start of one SAT close |
| `PHASE_GOAL bridge:word-goal <n>` | same point | size of the word goal |
| `bridge:prove-entry` | `CSAT_PROVE`, first statement | end of `BITBLAST_THEN`; start of clausification |
| `PHASE_GOAL bridge:blast <n>` | same point | size of the propositional term |
| `bridge:cnf-built` | `CSAT_PROVE`, just before `CSAT_REFUTE_GEN` | end of definitionalise+clausify |
| `PHASE_CNF <atoms> <clauses> <size>` | `CSAT_REFUTE_GEN`, after `dimacs_of_hol` | the DIMACS instance |
| `bridge:pre-replay` / `bridge:post-replay` | IX.65, unchanged | `LRAT_REFUTE_GEN` |
| `bridge:sat-done` | after `prove(imp, CSAT_TAC)` returns | end of the close |

Class of an interval is decided by the **pair** of labels that bounds it, because a
label alone cannot distinguish a SAT close from one that `BITBLAST_THEN`'s final
`REWRITE_TAC[]` closed (that close has `pre-bitblast` but no `prove-entry`;
`arm/neg_p25519` has one). The five required classes and the measured decomposition
of the residual:

| class | interval(s) | what it contains |
|---|---|---|
| **bitblast** | `pre-bitblast → prove-entry`, and `pre-bitblast → sat-done` when rewriting closed the goal | `BITBLAST_THEN`: `WORD_EQ_BITS_ALT`, `EXPAND_CASES_CONV`, `WORDIFY_*`, `BIT_WORD_CONV`, `BITBLAST_EQUATION` on each assumption |
| **defn_clausify** | `prove-entry → cnf-built` | `presimp_conv`, `definitionalize`, the `NNFC_CONV THENC CNF_CONV` loop, `list_mk_conj` |
| **bridge rewrite** | `NN bridge → first pre-bitblast`, and `sat-done → next pre-bitblast` | `ELIM_DIGIT_EQS_TAC`, `BRIDGE_REWRITE_MODULUS`, the bignum `expand`, `BRIDGE_WORD_OF_DIGITS`, `BRIDGE_SPLIT_CONDITIONS_KEEP_TAC`, `BRIDGE_TO_WORD_CONV`, `ASM_REWRITE_TAC` |
| **replay** | `pre-replay → post-replay` | `LRAT_REFUTE_GEN` (IX.65's bracket, unchanged) |
| **residual** | `cnf-built → pre-replay`, `post-replay → sat-done`, last `sat-done → STAMP_INF_AFTER` | DIMACS writing + solver + LRAT trim; `PROVE_HYP`/`main_rule`; final `ACCEPT_TAC`. Reported as three measured sub-numbers, never hidden |

## 2. Runs

Six tasks, one at a time, `make -j1 -B`, axiom-stamp v2 kernel
`.bench/hol-light-stamp2`, same environment as IX.62/IX.65
(`ix57-axiom-stamp.md` §7), `--mode phase` then `dropin assemble --stamp`:

```text
$ bash .bench/ix66/run_phase.sh arm curve25519 neg p25519
$ bash .bench/ix66/run_rest.sh
===== arm curve25519 add p25519 =====
assembled /mnt/1tb_kingston/Tekmerion/.bench/dropin/arm/proofs/bignum_add_p25519.ml (117436 bytes)
RUN arm_add_p25519 rc=0 phase_marks=37
exit=0
===== arm p384 add p384 =====
RUN arm_add_p384 rc=0 phase_marks=37
exit=0
===== arm p521 add p521 =====
RUN arm_add_p521 rc=0 phase_marks=37
exit=0
===== x86 curve25519 add p25519 =====
RUN x86_add_p25519 rc=0 phase_marks=37
exit=0
===== x86 sm2 neg sm2 =====
RUN x86_neg_sm2 rc=0 phase_marks=24
exit=0
ALL_PHASE_RUNS_DONE
$ bash .bench/ix66/run_phase.sh arm curve25519 neg p25519   # first run
RUN arm_neg_p25519 rc=0 phase_marks=31
```

Every run exited 0, so `check_axioms` passed. The control that the markers are inert
on the derivation is that all six `STAMP_INFERENCES` reproduce IX.63/IX.65
**exactly**:

| task | IX.63 baseline = IX.65 | this run |
|---|---:|---:|
| arm/add_p25519 | 119,194,788 | 119,194,788 |
| arm/add_p384 | 241,041,106 | 241,041,106 |
| arm/add_p521 | 470,294,392 | 470,294,392 |
| arm/neg_p25519 | 33,981,360 | 33,981,360 |
| x86/add_p25519 | 112,639,551 | 112,639,551 |
| x86/neg_sm2 | 16,422,436 | 16,422,436 |

## 3. The split

### 3.1 Reconciliation — mandatory, exact

Two checks per theorem: `sum(all classes) == STAMP_INFERENCES`, and
`bridge block == IX.65's replay + bridge` (the block IX.65 could not split; if the
new brackets were incomplete, these two would disagree).

| task | sum(all classes) | `STAMP_INFERENCES` | equal | bridge block | IX.65 bridge/SAT | equal |
|---|---:|---:|:--:|---:|---:|:--:|
| `arm_add_p25519` | 119194788 | 119194788 | yes | 116740946 | 116740946 | yes |
| `arm_add_p384` | 241041106 | 241041106 | yes | 237639772 | 237639772 | yes |
| `arm_add_p521` | 470294392 | 470294392 | yes | 466210010 | 466210010 | yes |
| `arm_neg_p25519` | 33981360 | 33981360 | yes | 31832876 | 31832876 | yes |
| `x86_add_p25519` | 112639551 | 112639551 | yes | 109547377 | 109547377 | yes |
| `x86_neg_sm2` | 16422436 | 16422436 | yes | 13729086 | 13729086 | yes |
| **all 6** | **993573633** | **993573633** | **yes** | **975700067** | **975700067** | **yes** |

### 3.2 The five-way split

| task | total | bitblast | defn+clausify | bridge rewrite | replay | residual | of which encode | of which post-solve |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| `arm_add_p25519` | 119194788 | 53397616 (44.80%) | 21500946 (18.04%) | 50943 (0.04%) | 40808833 (34.24%) | 982608 (0.82%) | 0 | 982608 |
| `arm_add_p384` | 241041106 | 121965262 (50.60%) | 45989280 (19.08%) | 89917 (0.04%) | 67462668 (27.99%) | 2132645 (0.88%) | 0 | 2132645 |
| `arm_add_p521` | 470294392 | 228940358 (48.68%) | 79017426 (16.80%) | 162873 (0.03%) | 154535460 (32.86%) | 3553893 (0.76%) | 0 | 3553893 |
| `arm_neg_p25519` | 33981360 | 21288386 (62.65%) | 7639874 (22.48%) | 32924 (0.10%) | 2510071 (7.39%) | 361621 (1.06%) | 0 | 361621 |
| `x86_add_p25519` | 112639551 | 52776810 (46.85%) | 21148161 (18.78%) | 50526 (0.04%) | 34600343 (30.72%) | 971537 (0.86%) | 0 | 971537 |
| `x86_neg_sm2` | 16422436 | 8799439 (53.58%) | 3080231 (18.76%) | 25740 (0.16%) | 1670262 (10.17%) | 153414 (0.93%) | 0 | 153414 |
| **all 6** | **993573633** | **487167871 (49.03%)** | **178375918 (17.95%)** | **412923 (0.04%)** | **301587637 (30.35%)** | **8155718 (0.82%)** | **0** | **8155718** |

Shares of the bridge block alone (975,700,067):

| class | inferences | share of block | share of six-task total |
|---|---:|---:|---:|
| bitblast | 487167871 | 49.93% | 49.03% |
| defn_clausify | 178375918 | 18.28% | 17.95% |
| replay | 301587637 | 30.91% | 30.35% |
| bridge_rewrite | 412923 | 0.04% | 0.04% |
| residual (encode + post_solve) | 8155718 | 0.84% | 0.82% |

`arm/add_p521` specifically: bitblast **228,940,358 (48.68%)**, defn+clausify
**79,017,426 (16.80%)**, replay **154,535,460 (32.86%)**, bridge rewrite
**162,873 (0.035%)**, residual **3,553,893 (0.76%)** of 470,294,392.

The outer classes reproduce IX.65 exactly (control):

| task | initial | play | rewrite | finalisation |
|---|---:|---:|---:|---:|
| `arm_add_p25519` | 1476272 | 787891 | 1563 | 188116 |
| `arm_add_p384` | 1476928 | 1711062 | 2922 | 210422 |
| `arm_add_p521` | 1477902 | 2268528 | 3142 | 334810 |
| `arm_neg_p25519` | 1475563 | 528862 | 753 | 143306 |
| `x86_add_p25519` | 1511665 | 1344960 | 1559 | 233990 |
| `x86_neg_sm2` | 1510972 | 965819 | 793 | 215766 |
| **all 6** | **8929302** | **7607122** | **10732** | **1326410** |

### 3.3 Size and close columns

Term size is the number of subterms (`Comb`/`Abs`/`Const`/`Var` nodes). "Goal" is the
obligation at `BRIDGE_*_REF_TAC` entry; "word goal" is the goal at `BRIDGE_CSAT_TAC`
entry; "bit-blasted term" is the proposition handed to `CSAT_PROVE`; atoms/clauses
are the `p cnf` line `dimacs_of_hol` writes.

| task | goal (bridge entry) | word goal per SAT attempt | bit-blasted term per SAT close | CNF atoms | CNF clauses | SAT closes / CSAT attempts |
|---|---:|---|---:|---:|---:|---:|
| `arm_add_p25519` | 63 | 571/53/571/53 | 511257/486209/511319/486271 | 22018/21249/22023/21254 | 60831/58529/60845/58543 | 4 / 4 |
| `arm_add_p384` | 39 | 853/853/77/77 | 1116315/1099583/1116317/1099585 | 46515/45930/46512/45927 | 128956/127216/128950/127210 | 4 / 4 |
| `arm_add_p521` | 51 | 1163/1163/113/113 | 1990719/1988795/1928583/1926659 | 78761/78768/77032/77039 | 218702/218850/213520/213668 | 4 / 4 |
| `arm_neg_p25519` | 287 | 29/45/555/571 | 231041/259185/255977 | 9789/10934/10807 | 26730/30297/29595 | 3 / 4 |
| `x86_add_p25519` | 261 | 705/187/571/53 | 504635/479587/504657/479609 | 21692/20922/21692/20923 | 59918/57614/59920/57618 | 4 / 4 |
| `x86_neg_sm2` | 159 | 43/569 | 77471/218061 | 3463/9232 | 9252/25526 | 2 / 2 |

Two facts worth reading against the costs:

- The word goal is tiny (29–1163 subterms) while the bit-blasted term is 77k–2.0M.
  The growth is not the goal: `BRIDGE_CSAT_TAC` builds
  `(accumulator equations) ==> (word goal)` and `BITBLAST_THEN` expands the
  **hypotheses** bit by bit. That is why a 51-subterm goal costs 57M inferences per
  close on `add_p521`.
- Cost per unit is stable across widths and architectures, which is the useful part
  for a lever:

| task | bitblast per bit-blasted subterm | defn+clausify per CNF clause | replay per CNF clause | replay per bit-blasted subterm |
|---|---:|---:|---:|---:|
| `arm_add_p25519` | 26.8 | 90.1 | 170.9 | 20.5 |
| `arm_add_p384` | 27.5 | 89.8 | 131.7 | 15.2 |
| `arm_add_p521` | 29.2 | 91.4 | 178.7 | 19.7 |
| `arm_neg_p25519` | 28.5 | 88.2 | 29.0 | 3.4 |
| `x86_add_p25519` | 26.8 | 90.0 | 147.2 | 17.6 |
| `x86_neg_sm2` | 29.8 | 88.6 | 48.0 | 5.7 |
| **all 6** | **28.2** | **90.4** | **152.9** | **17.5** |

Also visible: the four closes come in two pairs of nearly identical size
(`add_p521`: 1.99/1.99/1.93/1.93 M subterms). The four instances are the condition
split's four branches, and each is a full bit-blast of the same equations.

## 4. Verdict, and what each lever would buy

**The class that would matter is bitblast: 487,167,871 inferences, 49.03% of the
six-task total, 49.93% of the bridge block.** It is the single largest class, 1.62×
the replay. The normalisation the brief suspected (`BRIDGE_TO_WORD_CONV` and
friends) is 0.04% and cannot matter.

The table below is arithmetic on the measured columns. Each cell is
`total − (removed classes)` for that theorem, i.e. an **upper bound on the saving if
the class cost nothing**, not a prediction that any lever reaches it:

| task | total | remove bitblast | remove defn+clausify | remove replay | remove encoding (bitblast+defn) | remove whole bridge block |
|---|---:|---:|---:|---:|---:|---:|
| `arm_add_p25519` | 119194788 | 65797172 | 97693842 | 78385955 | 44296226 | 2453842 |
| `arm_add_p384` | 241041106 | 119075844 | 195051826 | 173578438 | 73086564 | 3401334 |
| `arm_add_p521` | 470294392 | **241354034** | 391276966 | 315758932 | 162336608 | 4084382 |
| `arm_neg_p25519` | 33981360 | 12692974 | 26341486 | 31471289 | 5053100 | 2148484 |
| `x86_add_p25519` | 112639551 | 59862741 | 91491390 | 78039208 | 38714580 | 3092174 |
| `x86_neg_sm2` | 16422436 | 7622997 | 13342205 | 14752174 | 4542766 | 2693350 |
| **all 6** | **993573633** | **506405762** | **815197715** | **691985996** | **328029844** | **17873566** |

Read the rows as follows.

- **A lever that removes the bit-blasting, keeping the SAT closes**, is not
  available: the CNF instance does not exist before the blast. So the
  `remove bitblast` column is a bookkeeping bound on the class, not a plan.
- **A lever that removes the SAT close entirely** (an E4-style parametric lemma that
  *rewrites* `val (ref_add (word p) x y) = val w` instead of deciding it) removes
  bitblast + defn+clausify + replay + the 0.84% residual together and caps at the
  `remove whole bridge block` column: `add_p521` → **4,084,382** (from 470,294,392,
  a 115× reduction of the primitive count), six-task total → **17,873,566**. That is
  the 98.2% IX.65 pointed at.
- **A cheaper encoding that keeps the SAT close** removes bitblast + defn+clausify:
  `add_p521` → 162,336,608, six-task → 328,029,844. The replay (301.6M) then
  dominates what is left; no further large win without also shrinking the CNF.
- **A smaller certificate** (shrink the replay only) leaves `add_p521` at 315.8M and
  the six-task total at 692.0M: the replay is 30.35%, so it cannot be the main lever.

For scale: `add_p521`'s bitblast alone is **53.1×** its human `_CORRECT` body
(4,312,009); defn+clausify is 18.3×; replay is 35.8×.

## 5. Normal emission is unchanged — proven

`tools/csat_lib.ml` and `tools/bridge.ml` are **not modified** (the write scope
allowed them; the instrumentation went into the phase-mode copy instead, which is
what makes this section literal rather than approximate):

```text
$ git --no-pager diff --stat HEAD -- tools/
(empty)
$ bash .bench/ix66/prove_normal_unchanged.sh
NORMAL_ANSWER_BYTE_IDENTICAL_TO_IX65 true
NORMAL_TACTIC_UNCHANGED true (post-IX.66 normal.now vs post-IX.66 phase.now)
PHASE_LIBRARY_MARKER_LINES 11
NORMAL_LIBRARY_MARKER_LINES 0
```

`NORMAL_ANSWER_BYTE_IDENTICAL_TO_IX65` is a whole-file `cmp` against
`.bench/ix65/normal.now`, the answer IX.65's binary emitted for
`arm.bignum_add_p25519`. `NORMAL_TACTIC_UNCHANGED` is the IX.65 check, same style:
strip every `csat_mark` from the phase tactic and it equals the normal tactic byte
for byte.

Two unit tests in `tests/tools_emit.rs` carry the same claim in the other direction:

- `ix66_markers_are_additive_and_the_normal_answer_is_recovered`: applying the
  emitter's edit table and then its inverse restores both tracked files **byte for
  byte** (`deinstrument ∘ instrument = id`), for `csat_lib.ml` and `bridge.ml`.
- `phase_mode_is_the_normal_emitter_plus_marks` (updated): the phase answer's tactic
  is `phase_tactic(tactic)`, and `strip_phase_marks` recovers `tactic` exactly. The
  IX.65 assertion that the two **libraries** are byte-identical is retired here — the
  phase library legitimately carries the inert markers — and replaced by the
  round-trip above. This is the one IX.65 invariant IX.66 refines.

## 6. The honest limit

- **Search inside a successful block is still not separated.** The counter is
  monotone; a failed attempt or an abandoned `THENL` branch leaves its inferences in
  whichever class it ran in. In particular `BRIDGE_SPLIT_CONDITIONS_KEEP_TAC`
  recurses per branch, and `BITBLAST_THEN` itself is a chain of `REPEAT`/`TRY`
  tactics. So "bitblast" is the cost of *reaching* the blasted term, which includes
  any discarded rewriting inside it. IX.65's limit stands unchanged.
- **`encode` is measured, not assumed.** `dimacs_of_hol`, the CaDiCaL subprocess and
  the IX.63 LRAT trim cost **0** primitive inferences on all six theorems (30 of 30
  closes), as predicted: none of them calls the kernel. The 0 is a measurement.
- **`bridge rewrite` here includes the final `ACCEPT_TAC`** of the emitted goal
  (`sat-done → STAMP_INF_AFTER`, 10,714 on `neg_p25519`) and the `REWRITE_TAC`/
  `ASM_REWRITE_TAC` inside `BRIDGE_*_REF_TAC` before the split. Both are part of the
  same 0.04%; separating them further would cost another library edit for a number
  three orders of magnitude below the verdict.
- **One attempt is not a close.** `arm/neg_p25519` has 4 `bridge:pre-bitblast`
  attempts and 3 `sat_closes`: one branch is discharged by `BITBLAST_THEN`'s final
  `REWRITE_TAC[]`. That class is still charged to bitblast (it is the same tactic's
  work) and the close count is reported separately, so the ratio of closes to
  attempts is visible per theorem.
- **Sizes are term sizes, not bytes and not proof sizes.** `csat_term_size` counts
  subterms; it is a shape measure, not a cost model. The unit-cost table (§3.3) is
  the only place where a size and a count are divided, and it is reported as
  inferences per subterm/clause, not as a law.
- **n = 6 one host, one pin, one run per task.** The inference counts are
  deterministic and reproduce IX.63/IX.65 exactly; no repetition measure exists for
  the wall times, and none are quoted as new numbers here.
- **The other 21 tasks were not measured.** Nothing in §3.2 is extended beyond the
  six.

## 7. Verification and provenance

```text
$ df -h /mnt/1tb_kingston | tail -1
/dev/nvme1n1p1  1.3T  1.3T  9.2G 100% /mnt/1tb_kingston
$ cargo test --offline
... (14 suites; every suite `ok`)
test result: ok. 306 passed; 0 failed
$ cargo clippy --offline --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.52s
clippy_rc=0
$ cargo fmt --check
fmt_rc=0
$ git ls-files '*.py' | wc -l
0
$ df -h /mnt/1tb_kingston | tail -1
/dev/nvme1n1p1  1.3T  1.3T  9.2G 100% /mnt/1tb_kingston
```

- Every run's `STAMP_INFERENCES` equals the IX.63 trimmed baseline exactly (§2).
- The phase-mode instrumentation is a *copy*: `tools/` has no diff (§5).
- No Python was used or is tracked; scratch is under `.bench/` (git-ignored).
- **Freeze `v6` is still required before any sealed run.** `src/tools/emit.rs` and
  `tests/tools_emit.rs` moved, so `freeze-rust-closure-v5`'s identity is stale for
  this commit too. This receipt does not create `v6` and no sealed run is claimed.

## 8. Reproduction

```text
$ bash .bench/ix66/run_phase.sh arm curve25519 neg p25519
$ bash .bench/ix66/run_rest.sh
$ for f in arm_add_p25519 arm_add_p384 arm_add_p521 arm_neg_p25519 \
           x86_add_p25519 x86_neg_sm2; do
    awk -f .bench/ix66/phases66.awk ".bench/ix66/$f.correct"
  done > .bench/ix66/blocks.tsv 2> .bench/ix66/aux.txt
$ awk -f .bench/ix66/build66.awk .bench/ix66/*.correct \
    .bench/ix66/blocks.tsv .bench/ix66/aux.txt > .bench/ix66/report.out
$ bash .bench/ix66/prove_normal_unchanged.sh
```

---

## Appendix A — the two scratch programs, verbatim

### A.1 Block costs with the bridge split (`phases66.awk`)

```awk
FNR == 1 {
  tag = FILENAME
  sub(/^.*\//, "", tag)
  sub(/\.correct$/, "", tag)
  n = 0; attempts = 0; closes = 0
}

function norm(l) {
  if (l ~ /^[0-9][0-9] /) return substr(l, 4)
  return l
}

function klass(a, b) {
  a = norm(a); b = norm(b)
  if (a == "initial") return "initial"
  if (a == "play") return "play"
  if (a == "finalisation" || a == "init finalisation") return "finalisation"
  if (a == "rewrite") return "rewrite"
  if (a == "bridge") return "bridge_rewrite"
  if (a == "bridge:sat-done")
    return (b == "STAMP_INF_AFTER") ? "post_solve" : "bridge_rewrite"
  if (a == "bridge:pre-bitblast") return "bitblast"
  if (a == "bridge:prove-entry") return "defn_clausify"
  if (a == "bridge:cnf-built") return "encode"
  if (a == "bridge:pre-replay") return "replay"
  if (a == "bridge:post-replay") return "post_solve"
  return "unmatched"
}

/^STAMP_INF_BEFORE / { before = $2 + 0; next }
/^STAMP_INF_AFTER /  { after = $2 + 0; next }
/^PHASE_CNF /        { cnf_atoms[closes] = $2 + 0; cnf_clauses[closes] = $3 + 0; next }
/^PHASE_GOAL / {
  label = ""
  for (i = 2; i < NF; i++) label = label (i > 2 ? " " : "") $i
  if (label == "bridge:goal") goal = $NF + 0
  else if (label == "bridge:blast") blast[closes] = $NF + 0
  else if (label == "bridge:word-goal") wordgoal[attempts] = $NF + 0
  next
}
/^PHASE_MARK / {
  label = ""
  for (i = 2; i < NF; i++) label = label (i > 2 ? " " : "") $i
  if (norm(label) == "bridge:pre-bitblast") attempts++
  if (norm(label) == "bridge:prove-entry") closes++
  n++; lab[n] = label; cnt[n] = $NF + 0
}

END {
  print tag "\tinitial\tinitial\t" lab[1] "\t" (cnt[1] - before)
  for (i = 1; i < n; i++)
    print tag "\t" klass(lab[i], lab[i + 1]) "\t" lab[i] "\t" lab[i + 1] "\t" (cnt[i + 1] - cnt[i])
  print tag "\t" klass(lab[n], "STAMP_INF_AFTER") "\t" lab[n] "\tSTAMP_INF_AFTER\t" (after - cnt[n])
  printf "TOTAL\t%s\t%d\t%d\t%d\t%d\t%d\t%d\n", tag, before, after, after - before, goal, attempts, closes > "/dev/stderr"
  for (i = 1; i <= attempts; i++)
    printf "ATT\t%s\t%d\t%d\n", tag, i, wordgoal[i] > "/dev/stderr"
  for (i = 1; i <= closes; i++)
    printf "BLAST\t%s\t%d\t%d\n", tag, i, blast[i] > "/dev/stderr"
  for (i = 1; i <= closes; i++)
    printf "CNF\t%s\t%d\t%d\t%d\n", tag, i, cnf_atoms[i], cnf_clauses[i] > "/dev/stderr"
}
```

### A.2 Table, shares, levers and JSON (`build66.awk`)

The second program reads the six `.correct` files for `STAMP_INF_*`, the block rows
from `A.1`, and the `ATT`/`BLAST`/`CNF` rows, and prints every table in §3, the
lever table in §4 and `research/receipts/ix66-bridge-split.json`. It is 200 lines of
awk; it is scratch (`.bench/ix66/build66.awk`) and every number it prints is a sum of
block rows plus a quotient by a measured total, so the receipt can be re-derived from
the checked-in `.correct` files and the two programs alone. The IX.65 cross-check
column (`ix65_bridge_sat`) is the only quoted number in it, and it is quoted from
IX.65 §2.5 for comparison, not used in any measured result.
