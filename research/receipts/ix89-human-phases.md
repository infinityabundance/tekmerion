# IX.89 — per-phase inference breakdown, human vs generated, ARM Curve25519 add/sub/neg

- **Date:** 2026-11-27
- **Branch:** `staging` at `a58618d` (one commit added on top; `main`, and the tags
  `freeze-rust-closure-v6` / `v0.2.0` untouched).
- **Task:** `TODO.md` `XXIII.A1`–`XXIII.A4`. A per-phase inference breakdown, human and
  generated, for ARM Curve25519 **add, sub and neg**, with **identical phase boundaries**.
- **Label:** **measurement only.** This receipt does not claim a win, does not compare
  `equals_thm`, and changes no theorem. It measures where the primitive inferences go.
- **Counter:** `inference_count` of `.bench/hol-light-stamp2`, the same stamp kernel on both
  sides. Scope: the `BIGNUM_<OP>_P25519_CORRECT` proof body only (from immediately before the
  `_CORRECT` `prove` to immediately after it). The derived `_SUBROUTINE_CORRECT`, the ELF
  decode and the shared bootstrap are excluded on both sides, as in IX.65.
- **Machine-readable data:** `research/receipts/ix89-human-phases.json` (every number below is
  parsed from the recorded `.correct` runs; nothing is typed by hand).

---

## 0. What was measured

| side | route | run |
|---|---|---|
| human | upstream `arm/proofs/bignum_{add,sub,neg}_p25519.ml` (`_CORRECT` only) | scratch copy `.bench/ix89/human`, instrumented, `make -j1 -B` |
| generated | algebraic aggregate close (`TEKMERION_ALG_CLOSE=<task>`) | `target/release/emit --mode phase`, post-instrumented |
| generated (comparison) | default sliced SAT close | `target/release/emit --mode phase`, gate unset |

All runs are `tools/cap.sh 12G -- make -j1 -B`, **one process at a time**. No cap fired.
No file in `.bench/s2n-bignum`, `.bench/human-baseline`, `.bench/hol-light` was written; the
human proof files are copied into `.bench/ix89/human` and restored from pristine after each run.
The three upstream proof files are **byte-identical** between the two read-only trees at pin
`5e0fc782` (`diff` empty for add/sub/neg); `.bench/human-baseline` is used as the build tree
because it carries the prebuilt `.o` files. The tracked emitter (`src/tools/emit.rs`) and
`tools/bridge.ml` are **not modified**.

---

## 1. Phase boundaries (fixed before the phase runs)

These were written down before any phase-instrumented run and applied to both sides. The
plain (ablation) runs were made first; the phase runs were made after, with only identity
markers added.

- **(a) init + digitise.** Quantiier stripping, assumption splitting,
  `REWRITE_TAC[BIGNUM_FROM_MEMORY_BYTES]`, `ENSURES_INIT_TAC`, and every
  `BIGNUM_*DIGITIZE_TAC`.
- **(b) simulation, per instruction.** One `ARM_ACCSTEPS_TAC` per instruction (identical to
  the single `MAP_EVERY` call upstream makes). Each step is labelled **accumulated** if its
  index is in the tactic's accumulator list, **plain** otherwise.
- **(c) carry/borrow characterisation.** The `SUBGOAL_THEN carry_* <=> …` block driven by
  `FLAG_FROM_CARRY_LE/LT`, including its `BOUNDER_TAC` side goal and its numeral
  normalisation. On the generated side the direct analogue is the **aggregate fact proof**
  (`bridge_aggregate_facts`: `carry = 0` from the weighted-sum aggregate); the generated
  route has no `FLAG_FROM_CARRY_*` tactic.
- **(d) final arithmetic, split by tactic.** Every `ARITH_TAC`, `REAL_ARITH_TAC`,
  `INT_ARITH_TAC`, `REAL_INTEGER_TAC`, `ASM_ARITH_TAC`, `NUM_REDUCE_CONV` and `BOUNDER_TAC`
  call in the closing arithmetic, isolated and named. On the generated side the closing
  arithmetic is the linear close (`INT_ARITH_TAC`), and `INT_RING`/`INT_REDUCE_CONV` are
  reported alongside it.
- **(e) everything else, itemised.** State finalisation, rewrites/expansions, the
  accumulator DECARRY/normalisation, aggregate construction, hypothesis flattening, close
  assembly, branch returns, tail, and — on the generated side — the per-task **support
  library construction**.

The pre-first-marker interval is reported as **PRE**: on the human side this is
tactic-argument evaluation (a few construction-time conversions); on the generated side it is
the evaluation of the inlined support library, whose top-level `prove`s (61 lemmas in
`tools/bridge.ml`) run before the first tactic.

A segment is charged to the phase of the marker that **starts** it; the last segment is closed
by the recorded after-counter. Every table below reconciles to the stamped total (`diff 0`).

---

## 2. Ablation check (required before trusting the split)

The plain run keeps the upstream tactic (or the unmodified `--mode phase` answer); the phase
run re-emits it with identity `ix89_mark`/`csat_mark` calls only. The totals must be identical.

| side | task | plain | phase | diff |
|---|---|---:|---:|---:|
| human | add | 1,848,494 | 1,848,494 | **0** |
| human | sub | 1,283,174 | 1,283,174 | **0** |
| human | neg | 721,473 | 721,473 | **0** |
| generated | add | 5,745,767 | 5,745,767 | **0** |
| generated | sub | 9,029,510 | 9,029,510 | **0** |
| generated | neg | 5,683,733 | 5,683,733 | **0** |

The generated plain numbers reproduce the recorded standing values **exactly**
(add 5,745,767; sub 9,029,510; neg 5,683,733). The human add/neg reproduce IX.65
(1,848,494 / 721,473). **Human `sub` is new: 1,283,174.**

A second, non-inference check confirms the insertions are additive: after removing the
`ix89_mark` tokens and the per-instruction split, the instrumented human tactic is
token-for-token the upstream tactic (all three tasks); the pre-mark and post-marker counters
are exact.

---

## 3. The six tables (3 tasks × 2 sides)

All figures are primitive inferences. `PRE` is the pre-first-marker interval (human:
construction; generated: support-library construction). `(e)` is the residual of the five
named phases.

### 3.1 Human (`arm/proofs/bignum_*_p25519.ml`, `_CORRECT` body)

| phase | add | sub | neg |
|---|---:|---:|---:|
| PRE — construction | 4,978 | 4,214 | 1,951 |
| (a) init + digitise | 1,791 | 1,805 | 1,074 |
| (b) simulation | 786,151 | 744,158 | 528,166 |
| (c) carry/borrow characterisation | 490,962 | 293,773 | **0** |
| (d) final arithmetic | 49,680 | 90,227 | 44,425 |
| (e) everything else | 514,932 | 148,997 | 145,857 |
| **total** | **1,848,494** | **1,283,174** | **721,473** |
| check | 0 | 0 | 0 |

`neg` has **no** `FLAG_FROM_CARRY_*` block: its borrow is discharged inside the final
`EQUAL_FROM_CONGRUENT_MOD_MOD` arithmetic, so `(c) = 0` by construction of the upstream proof.

### 3.2 Generated, algebraic aggregate close

| phase | add | sub | neg |
|---|---:|---:|---:|
| PRE — support library | 1,498,061 | 1,497,979 | 1,497,352 |
| (a) init + digitise | 1,795 | 1,805 | 1,076 |
| (b) simulation | 787,891 | 674,249 | 528,862 |
| (c) carry characterisation (aggregate fact) | 1,178,682 | 2,155,782 | 1,511,696 |
| (d) final arithmetic (linear close) | 1,128,617 | 2,786,328 | 1,563,214 |
| (e) everything else | 1,150,721 | 1,913,367 | 581,533 |
| **total** | **5,745,767** | **9,029,510** | **5,683,733** |
| check | 0 | 0 | 0 |

### 3.3 Ratios to the human `_CORRECT` body

| task | generated | human | ratio |
|---|---:|---:|---:|
| add | 5,745,767 | 1,848,494 | 3.11× |
| sub | 9,029,510 | 1,283,174 | **7.04×** |
| neg | 5,683,733 | 721,473 | **7.88×** |

The previously unmeasured human `sub` and `neg` make the real gap much larger than the
`add`-only 3.11×.

---

## 4. Per-tactic splits

### 4.1 Human

The numbers are the inferences under each named decision procedure, wherever it occurs. The
`(c)`-internal `BOUNDER_TAC` is included in the row but is part of `(c)`, not `(d)`.

| tactic | add | sub | neg |
|---|---:|---:|---:|
| `ARITH_TAC` | 15,058 | 0 | 701 |
| `REAL_ARITH_TAC` | 49,670 | 7,963 | 0 |
| `INT_ARITH_TAC` | 0 | 21,215 | 0 |
| `REAL_INTEGER_TAC` | 0 | 33,135 | 16,290 |
| `ASM_ARITH_TAC` | 0 | 0 | 11,216 |
| `NUM_REDUCE_CONV` | 349 | 3,227 | 3,038 |
| `BOUNDER_TAC` | 62,568 | 76,299 | 13,180 |
| **named-tactic total** | **127,645** | **141,839** | **44,425** |

`(d)` alone (the closing block) is: add 49,680 (`REAL_ARITH_TAC` 49,670 + `NUM_REDUCE_CONV`
10); sub 90,227 (`REAL_INTEGER_TAC` 33,135 + `BOUNDER_TAC` 24,687 + `INT_ARITH_TAC` 21,215 +
`REAL_ARITH_TAC` 7,963 + `NUM_REDUCE_CONV` 3,227); neg 44,425 (`REAL_INTEGER_TAC` 16,290 +
`BOUNDER_TAC` 13,180 + `ASM_ARITH_TAC` 11,216 + `NUM_REDUCE_CONV` 3,038 + `ARITH_TAC` 701).

The dominant *unnamed* arithmetic is the accumulator discharge: `GEN_DECARRY_RULE` /
`DECARRY_RULE` is **427,124** inside `(c)` and **307,097** inside `(e)` for add, and
**241,546** inside `(c)` for sub. It is a conversion, not one of the named tactics, so the
named-tactic table understates the human arithmetic.

### 4.2 Generated (algebraic)

| tactic | add | sub | neg |
|---|---:|---:|---:|
| `INT_ARITH_TAC` | 2,304,599 | 4,936,710 | 3,072,660 |
| `INT_RING` (aggregate + flatten) | 524,212 | 1,053,644 | 228,528 |
| `INT_REDUCE_CONV` (bitval close) | 2,700 | 5,400 | 2,250 |
| **named-tactic total** | **2,831,511** | **5,995,754** | **3,303,438** |

`INT_ARITH_TAC` is split as `(c)` aggregate facts + `(d)` linear close. `INT_RING` is the
weighted-sum aggregate construction **and** the hypothesis flattening (`bridge_flat_conv`);
both occur inside `(e)` but are arithmetic and are counted as such in §6.

---

## 5. The preamble, decomposed (the question that gated the programme)

For add the generated preamble (everything before the first `bridge:algebra`) is
**2,527,490**, more than the entire human budget. It decomposes as:

| component | add | share of preamble | sub | neg |
|---|---:|---:|---:|---:|
| support-library construction (PRE) | 1,498,061 | **59.3%** | 1,497,979 | 1,497,352 |
| init + digitise | 1,795 | 0.1% | 1,805 | 1,076 |
| simulation (19 instructions) | 787,891 | **31.2%** | 674,249 | 528,862 |
| state finalisation + rewrites + bridge entry | 239,743 | 9.5% | 187,935 | 179,343 |
| **preamble** | **2,527,490** | 100% | 2,361,968 | 2,206,633 |

**The preamble is not simulation.** 59.3% is the per-task proof of the inlined support
library (61 top-level `prove`s in `tools/bridge.ml` — arithmetic lemmas such as
`ADD_MOD_LT`, `SUB_INT_REM_MOD`, `SUB_REF_VAL_WORD`, `NEG_REF_VAL_WORD`); simulation is
31.2%. The construction cost is essentially constant across the three tasks (1,497,352–
1,498,061), which is the fingerprint of a shared library, not of the instruction count.
Under `XXIII.0` rule 3 this library is counted in full in every task; upstream's own lemmas
are free to both sides (rule 4), so this is a genuine asymmetry.

---

## 6. Verdict — what fraction is simulation, what fraction is arithmetic

Two honest readings. "Arithmetic (c+d)" is the phase-based arithmetic; "named arithmetic"
is the measured decision-procedure total; "certificate side" adds the generated support
library.

| side | task | simulation (b) | arithmetic (c)+(d) | named arithmetic | support/construction (PRE) |
|---|---|---:|---:|---:|---:|
| human | add | 42.5% | 29.2% | 6.9% | 0.3% |
| human | sub | 58.0% | 29.9% | 11.1% | 0.3% |
| human | neg | 73.2% | 6.2% | 6.2% | 0.3% |
| generated | add | 13.7% | 40.2% | 49.3% | 26.1% |
| generated | sub | 7.5% | 54.7% | 66.4% | 16.6% |
| generated | neg | 9.3% | 54.1% | 58.1% | 26.3% |

Adding the support library to the generated named arithmetic gives the **certificate side**:

| task | certificate side | simulation | other |
|---|---:|---:|---:|
| add | 4,329,572 = **75.4%** | 13.7% | 10.9% |
| sub | 7,493,733 = **83.0%** | 7.5% | 9.5% |
| neg | 4,800,790 = **84.5%** | 9.3% | 6.2% |

**The lever is split by side, and not evenly.**

- On the **human** proof the cost is **simulation** (42.5–73.2%, rising as the instruction
  count falls). A step-theorem cache (`XXIII.D`) is the lever for *that* proof.
- On **our** algebraic proof the cost is the **certificate**: named arithmetic 49.3–66.4%
  plus the per-task support library 16.6–26.3%, i.e. **75.4–84.5%**. Simulation is only
  **7.5–13.7%**.
- The decisive test for our race: remove simulation entirely. The residual is
  4,957,876 / 8,355,261 / 5,154,871 for add/sub/neg, still **2.68× / 6.51× / 7.15×** human.
  Remove the support library as well and the residual is 3,459,815 / 6,857,282 / 3,657,519,
  still **1.87× / 5.34× / 5.07×** human. **Simulation cannot decide our race.**

**Verdict line.** Simulation vs certificate replay is **split by side**: the human proof is
simulation-dominated (42.5–73.2% vs 6.2–29.9% arithmetic), while ours is
certificate-dominated (75.4–84.5% certificate vs 7.5–13.7% simulation). Because the race is
against the human total, the lever the data identifies for us is **certificate replay /
premise reduction (`XXIII.B`/`XXIII.C`) together with removing or reusing the per-task
support library** — not the simulation cache. `XXIII.D` targets the human's own dominant
cost and would repay at most 7.5–13.7% of ours.

---

## 7. SAT route, for comparison

Default sliced SAT close, gate unset, same skeleton and same preamble:

| task | total | preamble | simulation inside it | bitblast | clausify | replay | assemble+tail |
|---|---:|---:|---:|---:|---:|---:|---:|
| add | 111,523,776 | 2,526,370 | 787,891 | 53,396,030 | 21,500,774 | 33,117,926 | 982,676 |
| sub | 87,971,295 | 2,360,880 | 674,249 | 50,996,024 | 20,764,090 | 12,909,883 | 940,418 |
| neg | 33,910,447 | 2,203,065 | 528,862 | 21,281,137 | 7,639,793 | 2,420,424 | 361,611 |

The SAT route has ~0.0% arithmetic and spends its cost on **bitblast (47.9%) + clausify
(19.3%) + certified replay (29.7%)** for add. It is 19.4× the algebraic route and 60.3×
human for add, 9.7×/68.6× for sub, 6.0×/47.0× for neg. This confirms the algebraic
certificate route is the correct direction and that its remaining cost is arithmetic, not
simulation.

---

## 8. Exact commands (decisive)

All capped; all sequential. `free -g | head -2` and `df -h /mnt/1tb_kingston | tail -1` were
checked before every long run; available memory stayed above 3× the cap and disk fell only
inside `.bench/`.

```text
$ bash tests/ix89/make_bodies.sh tests/ix89/bodies
$ bash tests/ix89/run_human.sh plain add      # HUMAN_CORRECT_INF 1848494
$ bash tests/ix89/run_human.sh plain sub      # HUMAN_CORRECT_INF 1283174
$ bash tests/ix89/run_human.sh plain neg      # HUMAN_CORRECT_INF 721473
$ bash tests/ix89/run_human.sh phase add      # HUMAN_CORRECT_INF 1848494
$ bash tests/ix89/run_human.sh phase sub      # HUMAN_CORRECT_INF 1283174
$ bash tests/ix89/run_human.sh phase neg      # HUMAN_CORRECT_INF 721473

$ bash tests/ix89/run_gen.sh plain add        # STAMP_INFERENCES 5745767
$ bash tests/ix89/run_gen.sh plain sub        # STAMP_INFERENCES 9029510
$ bash tests/ix89/run_gen.sh plain neg        # STAMP_INFERENCES 5683733
$ bash tests/ix89/run_gen.sh phase add        # STAMP_INFERENCES 5745767, 992 phase marks
$ bash tests/ix89/run_gen.sh phase sub        # STAMP_INFERENCES 9029510, 2345 phase marks
$ bash tests/ix89/run_gen.sh phase neg        # STAMP_INFERENCES 5683733, 568 phase marks

$ env -u TEKMERION_ALG_CLOSE bash .bench/ix69/run_task.sh arm curve25519 add p25519 .bench/ix89/gen
    STAMP_INFERENCES 111523776
$ env -u TEKMERION_ALG_CLOSE bash .bench/ix69/run_task.sh arm curve25519 sub p25519 .bench/ix89/gen
    STAMP_INFERENCES 87971295
$ env -u TEKMERION_ALG_CLOSE bash .bench/ix69/run_task.sh arm curve25519 neg p25519 .bench/ix89/gen
    STAMP_INFERENCES 33910447

$ awk -f tests/ix89/parse_human.awk .bench/ix89/human_out/arm_add_p25519.phase.correct
$ awk -f tests/ix89/parse_gen.awk   .bench/ix89/gen/arm_add_p25519.phase.correct
$ awk -f tests/ix89/parse_sat.awk   .bench/ix89/sat/arm_add_p25519.sat.correct
$ bash tests/ix89/build_receipt_json.sh
    human add sim=786151 total=1848494  gen add sim=787891 total=5745767
    ablation gen add diff=0 human add diff=0
```

Every proof build is wrapped by `tools/cap.sh 12G` (inside the scripts); `cargo` by
`tools/cap.sh 20G`. No cap fired. `git ls-files '*.py'` is empty.

Environment at measurement time:

```text
$ free -g | head -2
               total        used        free      shared  buff/cache   available
Mem:             125          22          10           1          90         102
$ df -h /mnt/1tb_kingston | tail -1
/dev/nvme1n1p1  1.3T 1012G  250G  81% /mnt/1tb_kingston
```

---

## 9. Instrumentation and write set (what moved)

- `tests/ix89/make_bodies.sh` — emits the three instrumented human tactic bodies (upstream
  tactic plus identity markers; the per-instruction split is exactly the `MAP_EVERY` that
  `ARM_ACCSTEPS_TAC` performs).
- `tests/ix89/instr_human.pl` — wraps a upstream `_CORRECT` proof: `plain` keeps the tactic
  and adds before/after counters; `phase` substitutes the instrumented body. The theorem
  statement is copied verbatim.
- `tests/ix89/run_human.sh` — scratch-tree build+run (`.bench/ix89/human`), capped, restored.
- `tests/ix89/instr_gen.pl` — post-processes a `--mode phase` generated answer: per-instruction
  `B:step` markers and per-tactic `G:` markers around `INT_RING`/`INT_ARITH`/`INT_REDUCE`
  calls. It edits the **scratch answer only**.
- `tests/ix89/run_gen.sh` — emit/assemble/build/run, capped.
- `tests/ix89/{parse_human,parse_gen,parse_sat,to_json}.awk` and `build_receipt_json.sh` —
  parsers and JSON assembly.

No change to `tools/bridge.ml`, `src/tools/emit.rs`, `.bench/s2n-bignum/**`,
`.bench/human-baseline/**`, `.bench/hol-light/**`, `.bench/*.py`, or any forbidden Rust file.
`.bench/` is git-ignored; all run artefacts (answers, `.correct`, logs) live under
`.bench/ix89/`.

---

## 10. What is not established

- **No performance claim, no `equals_thm`.** This is a cost measurement of proof bodies. It
  does not assert that any generated theorem equals the upstream theorem (that is `XXIII.0`
  rule 2, checked elsewhere) and it does not claim a win.
- **`(d)` on the generated side is the linear close plus the named `INT_RING`/`INT_REDUCE`
  work.** The normalisation conversions (`NUM_TO_INT_CONV`, `BRIDGE_VAL_*_CONV`,
  `INT_REDUCE_CONV` inside `BRIDGE_ALG_NORM_TAC`) are itemised under `(e)`; they are not
  split to a single tactic call because `BRIDGE_ALG_NORM_TAC` is a fixed conversion chain.
- **Human `(c)` uses `GEN_DECARRY_RULE`/`DECARRY_RULE`, not a named decision procedure.**
  The DECARRY totals are given in §4.1 so the arithmetic is not understated, but they are
  not broken down per rule.
- **The SAT comparison is coarse** (preamble / bitblast / clausify / replay / assemble); it
  is not subdivided per slice beyond the IX.66 markers, and the SAT route's simulation is
  not split per instruction (it is one `ARM_ACCSTEPS_TAC` call and is not the lever here).
- **One host, one kernel, one pin** (`5e0fc782`), median-of-5 wall time is not reported
  because the task is an inference count, not wall time.
- The generated support-library construction is measured with the standard `--mode phase`
  library (emit.rs's inert markers). It is not decomposed lemma-by-lemma.
