# IX.69 — hypothesis-cone slicing before the blast (XV.B1–XV.B4)

- **Date:** 2026-11-25
- **Branch:** `staging`
- **Change under test:** `tools/bridge.ml` gains a hypothesis-cone slice in
  `BRIDGE_CSAT_TAC`: the connected component of the goal under "shares a free
  variable with", transitively, is handed to `CSAT_TAC`; the full set is retried
  if that close fails, and the fallback is printed as
  `SLICE fallback=<task> reason=<...> kept=<k>/<n>`. `src/tools/emit.rs`'s phase
  prefix now binds `bridge_slice_task` so a `SLICE` line names its task. The
  slice is active in `--mode answer` too but prints nothing there.
- **Answer, six measured tasks:** the slice is **sound and deterministic** and
  drops 3–9 hypotheses per SAT close (every one of 27 tasks slices, 0 fallbacks),
  but the six-task total moves only **993,573,633 → 944,555,577 (−4.93%)**. The
  **30% target is not met.** The whole saving is in the certified replay
  (301,587,637 → 253,762,924); bit-blasting moves by **−0.19%** and
  definitionalise+clausify by **−0.14%**, because the hypotheses the criterion
  keeps are exactly the ones the word obligation needs. One task *regresses*
  (`x86_add_p25519`, +6.58%).
- **Verdict:** slicing is not the lever the 60× gap needs. The kept set is
  dominated by the two input digit equations, the range/`word_ult` condition and
  the accumulator chain; S3 shows dropping any of them fails the close. A
  smaller CNF can also *lengthen* the LRAT proof, so the replay saving is
  non-monotone. §7 states the blocker with numbers.

---

## 1. Mechanism

`BRIDGE_CSAT_TAC` (`tools/bridge.ml`, §6a) previously built one implication over
`map concl (map snd asl)` and chained `MP` over every assumption theorem. It now:

1. computes `bridge_slice_hyps hths w`: seed `reach := frees w`; repeatedly keep a
   hypothesis whose conclusion shares a free variable with `reach` and add its
   free variables to `reach`; preserve `asl` order;
2. proves `(hyp_1 ==> … ==> hyp_k ==> w)` for the kept list only, and `MP`s only
   those hypotheses;
3. on failure retries with the **full** list exactly as before and prints
   `SLICE fallback=<task> reason=<...> kept=<k>/<n>`.

The criterion is syntactic and deterministic. There are no heuristics: a
hypothesis is kept iff it is connected to the goal's free variables. The only
extra additions are audit controls (`TEKMERION_SLICE_CONTROL`), which can only
*remove* hypotheses and therefore can only strengthen the obligation:

| control | effect |
|---|---|
| unset | the criterion above |
| `keep-none` | keep no hypothesis (XV.B3) |
| `drop-one` | remove the last kept hypothesis |
| `drop-largest` | remove the kept hypothesis with the largest term (S3) |
| `drop-non-eq` | from the component, keep only `is_eq` conclusions (S3) |
| `only-true` / `drop-true` | keep only / drop only hypotheses concluding `T` |
| `drop-nonoverlap` | drop only hypotheses containing `nonoverlapping_modulo` |

Measurement mode is IX.66's `--mode phase`; `SLICE`/`SLICE_HYP` lines are printed
only when `csat_mark_enabled` is true, so a normal answer is silent. The
`SLICE_HYP … kept=0/1` lines (behind `TEKMERION_SLICE_DEBUG=1`) are what identify
which hypothesis the criterion keeps and which it drops.

## 2. Why this is sound (and why the controls suffice)

`BRIDGE_CSAT_TAC` proves the implication from the hypotheses and then `MP`s the
assumption theorems. If `S` is a **subset** of the hypotheses, proving
`(conj S) ==> w` and applying `MP` only to `S` still yields `|- w` with exactly
the same conclusion. Removing a hypothesis makes the obligation *stronger*,
never weaker: any model of the sliced antecedent is a model of the full one. So a
slice can never produce a false theorem; it can only fail to close. Every
failure path retries the full set, so the theorem produced is the same
proposition, and incompleteness of the criterion is a **measurement** (the
`fallback` count), not a correctness risk. This is why S3 is a sufficient
refutation of vacuity: the criterion bites exactly when the drop makes the
obligation unprovable.

## 3. Controls S1–S5, verbatim

### S1 — statement unchanged, `check_axioms` passing

Each generated proof still proves the **frozen `query`** from
`.bench/problems.json` (`src/tools/dropin.rs`, `assemble_core`: `prove (query,
<answer>)`), which was not touched; `prove` returns a theorem of exactly that
term. The observable check compares the kernel-printed statements of the baseline
and sliced runs for every measured theorem (`_CORRECT` and the derived
`_SUBROUTINE_CORRECT`), and all six are byte-identical:

```text
$ for t in arm_add_p25519 arm_add_p384 arm_add_p521 arm_neg_p25519 x86_add_p25519 x86_neg_sm2; do
    grep '^val BIGNUM_.*_CORRECT : thm = ' .bench/ix69/baseline/$t.correct > /tmp/b.$t
    grep '^val BIGNUM_.*_CORRECT : thm = ' .bench/ix69/sliced/$t.correct   > /tmp/s.$t
    cmp -s /tmp/b.$t /tmp/s.$t && echo "$t STAMP_STATEMENT_IDENTICAL true lines=$(wc -l < /tmp/b.$t)"
  done
arm_add_p25519 STAMP_STATEMENT_IDENTICAL true lines=3
arm_add_p384 STAMP_STATEMENT_IDENTICAL true lines=3
arm_add_p521 STAMP_STATEMENT_IDENTICAL true lines=3
arm_neg_p25519 STAMP_STATEMENT_IDENTICAL true lines=3
x86_add_p25519 STAMP_STATEMENT_IDENTICAL true lines=4
x86_neg_sm2 STAMP_STATEMENT_IDENTICAL true lines=4
```

`check_axioms` passes for all six: every run exits 0, which the driver treats as
`check_axioms = true` (`src/tools/dropin.rs`: "Implied by exit 0: upstream's
template runs `check_axioms ()`"). **Not established:** `equals_thm` was not
called on two live theorems in one session, because that would require replaying
the expensive proof twice in one HOL process. Identity is by construction
(`prove` against the frozen `query`) and observed by the identical printed term.

### S2 — the slice does work

Every one of the 27 runs has `kept < total`. Smallest: `arm_neg_p521` keeps 3/7;
`x86_neg_p25519` keeps 4/9; `x86_neg_sm2` keeps 3/8 and 5/8; the adds keep
11–23 of 17–28. Per-close counts are in the JSON (`kept`,
`hypotheses`). No theorem slices to zero: `arm_neg_p25519`'s second close has
`kept=0/11` yet closes (the goal is then proved directly), every other close
keeps at least 3.

### S3 — over-slicing must fail (the criterion is not vacuous)

`arm/add_p521`, four SAT closes. The largest **kept** hypothesis is the
range/`word_ult` condition (1227 subterms, 18 free variables). Removing it makes
two of the four closes return SAT and fall back; removing all non-equational
kept hypotheses does the same; removing everything fails all four:

```text
$ TEKMERION_SLICE_CONTROL=drop-largest bash .bench/ix69/run_task.sh arm p521 add p521 ...
SLICE fallback=arm.bignum_add_p521.BIGNUM_ADD_P521_CORRECT reason=CSAT: cadical did not report UNSAT, exit 10 kept=21/28
SLICE fallback=arm.bignum_add_p521.BIGNUM_ADD_P521_CORRECT reason=CSAT: cadical did not report UNSAT, exit 10 kept=21/28

$ TEKMERION_SLICE_CONTROL=drop-non-eq bash .bench/ix69/run_task.sh arm p521 add p521 ...
SLICE fallback=arm.bignum_add_p521.BIGNUM_ADD_P521_CORRECT reason=CSAT: cadical did not report UNSAT, exit 10 kept=19/28
SLICE fallback=arm.bignum_add_p521.BIGNUM_ADD_P521_CORRECT reason=CSAT: cadical did not report UNSAT, exit 10 kept=19/28

$ TEKMERION_SLICE_CONTROL=keep-none bash .bench/ix69/run_task.sh arm p521 add p521 ...
SLICE fallback=arm.bignum_add_p521.BIGNUM_ADD_P521_CORRECT reason=CSAT: cadical did not report UNSAT, exit 10 kept=0/28
SLICE fallback=arm.bignum_add_p521.BIGNUM_ADD_P521_CORRECT reason=CSAT: cadical did not report UNSAT, exit 10 kept=0/28
SLICE fallback=arm.bignum_add_p521.BIGNUM_ADD_P521_CORRECT reason=CSAT: cadical did not report UNSAT, exit 10 kept=0/28
SLICE fallback=arm.bignum_add_p521.BIGNUM_ADD_P521_CORRECT reason=CSAT: cadical did not report UNSAT, exit 10 kept=0/28
```

All three control runs still exit 0 and the theorem is accepted: the fallback is
what saves them. The `word_ult` hypothesis the output depends on is **kept** by
the criterion (`TEKMERION_SLICE_DEBUG=1` prints `SLICE_HYP 3/28 kept=1 size=1227`),
which is the positive half of XV.B3.

### S4 — no regression in acceptance

All 27 tasks are accepted (`check_axioms`, exit 0), each either sliced
(all of them; `kept < total`) or would be marked `fallback`. **Default-slice
fallback rate: 0/27.** The six measured runs are in §4; the other 21 are in §6.

### S5 — the target

**Not met.** Six-task total **993,573,633 → 944,555,577**, i.e. **4.93% off**, not
≥30%. The blocker with numbers is §7.

## 4. Per-theorem table (generated from `ix69-slicing.json`)

Command:

```text
$ bash tests/ix69_report.sh json .bench/ix69 > research/receipts/ix69-slicing.json
$ bash tests/ix69_report.sh table research/receipts/ix69-slicing.json
```

| task | total before | total after | delta | off | kept / hypotheses | blasted subterms | CNF clauses | bitblast | defn+clausify | replay | fallbacks |
|---|---:|---:|---:|---:|---|---:|---:|---:|---:|---:|---:|
| `arm_add_p25519` | 119194788 | 111501987 | -7692801 | 6.45% | 11/17 11/17 13/17 13/17 | 1995056 -> 1994316 | 238748 -> 238742 | 53397616 -> 53396030 | 21500946 -> 21500774 | 40808833 -> 33117926 | 0 |
| `arm_add_p384` | 241041106 | 240835826 | -205280 | 0.09% | 23/28 23/28 23/28 23/28 | 4431800 -> 4431064 | 512332 -> 512328 | 121965262 -> 121963402 | 45989280 -> 45989247 | 67462668 -> 67259409 | 0 |
| `arm_add_p521` | 470294392 | 422831878 | -47462514 | 10.09% | 22/28 22/28 22/28 22/28 | 7834756 -> 7833992 | 864740 -> 864732 | 228940358 -> 228937670 | 79017426 -> 79017482 | 154535460 -> 107075738 | 0 |
| `arm_neg_p25519` | 33981360 | 33888658 | -92702 | 0.27% | 7/11 0/11 7/11 8/11 | 746203 -> 745657 | 86622 -> 86619 | 21288386 -> 21285554 | 7639874 -> 7639793 | 2510071 -> 2420424 | 0 |
| `x86_add_p25519` | 112639551 | 120053161 | +7413610 | -6.58% | 12/17 12/17 12/17 12/17 | 1968488 -> 1967760 | 235070 -> 235066 | 52776810 -> 52775406 | 21148161 -> 21147975 | 34600343 -> 42015671 | 0 |
| `x86_neg_sm2` | 16422436 | 15444067 | -978369 | 5.96% | 3/8 5/8 | 295532 -> 269590 | 34778 -> 32143 | 8799439 -> 7870824 | 3080231 -> 2837863 | 1670262 -> 1873756 | 0 |

Six-task aggregate, from the same JSON:

```text
"aggregate": {"baseline_total": 993573633, "sliced_total": 944555577,
  "delta": -49018056, "percent_off": 4.93,
  "baseline_bitblast": 487167871, "sliced_bitblast": 486228886,
  "baseline_defn_clausify": 178375918, "sliced_defn_clausify": 178133134,
  "baseline_replay": 301587637, "sliced_replay": 253762924, "fallbacks": 0}
```

`blasted subterms` and `CNF clauses` are sums over the theorem's SAT closes; the
per-close lists are in the JSON (`blast_per_close`, `cnf_per_close`), e.g.
`arm_add_p521`: `1990719 1988795 1928583 1926659` → `1990529 1988603 1928393 1926467`.

## 5. Controls (generated from the same JSON)

| run | task | total | kept / hypotheses | fallbacks | accepted | note |
|---|---|---:|---|---:|---|---|
| `control-droplargest` | `arm_add_p521` | 600904934 | 21/28 21/28 21/28 21/28 / 28 28 28 28 | 2 | yes | S3 over-slice: TEKMERION_SLICE_CONTROL=drop-largest (the largest kept hypothesis, the 1227-subterm range condition, removed) |
| `control-dropnoneq` | `arm_add_p521` | 406875799 | 19/28 19/28 19/28 19/28 / 28 28 28 28 | 2 | yes | S3 over-slice: TEKMERION_SLICE_CONTROL=drop-non-eq (non-equational hypotheses dropped from the component) |
| `control-dropnonoverlap` | `arm_add_p521` | 437398754 | 27/28 27/28 27/28 27/28 / 28 28 28 28 | 0 | yes | diagnostic: TEKMERION_SLICE_CONTROL=drop-nonoverlap (only nonoverlapping_modulo dropped from the full set) |
| `control-droptrue` | `arm_add_p521` | 470293384 | 24/28 24/28 24/28 24/28 / 28 28 28 28 | 0 | yes | diagnostic: TEKMERION_SLICE_CONTROL=drop-true (only `T` hypotheses dropped from the full set) |
| `control-keepnone` | `arm_add_p521` | 501665636 | 0/28 0/28 0/28 0/28 / 28 28 28 28 | 4 | yes | XV.B3 over-slice: TEKMERION_SLICE_CONTROL=keep-none (every hypothesis dropped) |
| `control-onlytrue` | `arm_add_p521` | 501665724 | 4/28 4/28 4/28 4/28 / 28 28 28 28 | 4 | yes | diagnostic: TEKMERION_SLICE_CONTROL=only-true (only `T` hypotheses kept) |
| `control-x86-dropnonoverlap` | `x86_add_p25519` | 120053729 | 16/17 16/17 16/17 16/17 / 17 17 17 17 | 0 | yes | diagnostic: TEKMERION_SLICE_CONTROL=drop-nonoverlap on x86 add_p25519 (the regression source) |
| `determinism` | `x86_neg_sm2` | 15444067 | 3/8 5/8 / 8 8 | 0 | yes | determinism repeat of the sliced x86_neg_sm2 run |

Determinism, verbatim (two independent runs, same task, same binary; the second
is the `determinism` row above):

```text
$ grep -h '^SLICE kept\|^STAMP_INFERENCES' .bench/ix69/determinism/x86_neg_sm2.correct
SLICE kept=3/8 task=x86.bignum_neg_sm2.BIGNUM_NEG_SM2_CORRECT
SLICE kept=5/8 task=x86.bignum_neg_sm2.BIGNUM_NEG_SM2_CORRECT
STAMP_INFERENCES 15444067
$ grep -h '^SLICE kept\|^STAMP_INFERENCES' .bench/ix69/sliced/x86_neg_sm2.correct
SLICE kept=3/8 task=x86.bignum_neg_sm2.BIGNUM_NEG_SM2_CORRECT
SLICE kept=5/8 task=x86.bignum_neg_sm2.BIGNUM_NEG_SM2_CORRECT
STAMP_INFERENCES 15444067
```

The six measured tasks were also re-run after the slice code was frozen (later
edits to `tools/bridge.ml` were comment-only, and were followed by a fixture
re-record) and reproduced every total and every kept count exactly.

## 6. S4: the other 21 tasks (generated from the same JSON)

| task | total | kept / hypotheses | fallbacks | accepted |
|---|---:|---|---:|---|
| `arm_add_p256` | 118896814 | 12/18 12/18 12/18 12/18 / 18 18 18 18 | 0 | yes |
| `arm_add_p256k1` | 115582369 | 15/20 16/20 15/20 16/20 / 20 20 20 20 | 0 | yes |
| `arm_add_sm2` | 108249602 | 12/18 12/18 12/18 12/18 / 18 18 18 18 | 0 | yes |
| `arm_neg_p256` | 42505717 | 7/11 7/11 7/11 7/11 / 11 11 11 11 | 0 | yes |
| `arm_neg_p256k1` | 42560601 | 7/11 7/11 7/11 7/11 / 11 11 11 11 | 0 | yes |
| `arm_neg_p384` | 80582732 | 9/13 9/13 9/13 9/13 / 13 13 13 13 | 0 | yes |
| `arm_neg_p521` | 119718833 | 3/7 3/7 3/7 3/7 / 7 7 7 7 | 0 | yes |
| `arm_neg_sm2` | 42495548 | 7/11 7/11 7/11 7/11 / 11 11 11 11 | 0 | yes |
| `arm_sub_p25519` | 87949506 | 11/17 11/17 11/17 11/17 / 17 17 17 17 | 0 | yes |
| `arm_sub_p256` | 85362540 | 11/17 13/17 11/17 13/17 / 17 17 17 17 | 0 | yes |
| `arm_sub_p256k1` | 88153251 | 11/17 11/17 11/17 11/17 / 17 17 17 17 | 0 | yes |
| `arm_sub_p384` | 163978585 | 15/21 17/21 15/21 17/21 / 21 21 21 21 | 0 | yes |
| `arm_sub_p521` | 177637519 | 20/24 20/24 / 24 24 | 0 | yes |
| `arm_sub_sm2` | 85792796 | 11/17 13/17 11/17 13/17 / 17 17 17 17 | 0 | yes |
| `x86_add_p256k1` | 120713132 | 15/20 15/20 15/20 15/20 / 20 20 20 20 | 0 | yes |
| `x86_add_sm2` | 77654544 | 16/20 16/20 / 20 20 | 0 | yes |
| `x86_neg_p25519` | 16523984 | 4/9 6/9 / 9 9 | 0 | yes |
| `x86_neg_p256k1` | 17753402 | 4/9 6/9 / 9 9 | 0 | yes |
| `x86_sub_p25519` | 49333459 | 11/15 11/15 / 15 15 | 0 | yes |
| `x86_sub_p256k1` | 85391088 | 11/17 13/17 11/17 13/17 / 17 17 17 17 | 0 | yes |
| `x86_sub_sm2` | 50665198 | 11/15 11/15 / 15 15 | 0 | yes |

These 21 have no same-session baseline (the six were chosen because IX.66 has
their split); their fallback rate and acceptance are the measured quantities
here.

## 7. Why 30% is out of reach: the blocker, with numbers

**The saved hypotheses are tiny.** On `arm/add_p521` a close has 28 hypotheses;
22 are kept: the digit equation for `n` and for `m` (1183 subterms each), the
range condition `word_ult …` (1227), and 19 accumulator equations (49–211). The 6
dropped are four `T`, the disconnected boolean `carry_s21` (size 1), and
`nonoverlapping_modulo …` (85). Consequently the sum of bit-blasted subterms over
the four closes moves from **7,834,756 to 7,833,992 (−764)**. Bit-blasting over
the six tasks is **487,167,871 → 486,228,886 (−0.19%)**: no slice of the
hypothesis set can remove what bit-blasting actually costs, because the kept
hypotheses *are* the obligation.

**The kept hypotheses cannot be dropped.** S3 drops the largest kept one (the
range condition) and two of four closes return SAT; `keep-none` fails all four.
`arm/add_p521`'s word goal is `val(ref_add (word p) x y) = val w`; the digit
equations tie `x,y` to the limb digits, the accumulator equations define each
limb of `w`, and the range condition is needed in the branches where it is not
already implied. Dropping them changes the theorem, not just the cost.

**The replay saving is non-monotone.** The whole −4.93% is replay
(**301,587,637 → 253,762,924**, −47,824,713). Isolating the hypotheses shows why
it cannot be extrapolated:

| single change on `arm/add_p521` | total | Δ vs 470,294,392 |
|---|---:|---:|
| drop only `T` (4 hypotheses) | 470,293,384 | −1,008 |
| drop only `nonoverlapping_modulo` | 437,398,754 | −32,895,638 |
| drop the connected component (default) | 422,831,878 | −47,462,514 |

The same single drop on `x86_add_p25519` **adds** 7,414,178
(`112,639,551 → 120,053,729`), which is the entire regression. Removing
redundant clauses can lengthen CaDiCaL's LRAT refutation; the replay is not a
monotone function of the hypothesis set. The default slice is therefore a net win
on the six-task total but not a per-task improvement, and it cannot be made one
by a syntactic criterion.

**The arithmetic.** The target is ≤ ~695M, i.e. removing ~298M. The close is
~987M of the baseline; bitblast is 487.2M and cannot move, so the only remaining
budget is replay (301.6M) plus clausify (178.4M). Slicing removed 47.8M of
replay. To reach 695M one would have to remove essentially all replay, or shrink
bitblast — and S3 shows the hypotheses that would have to go are required. That
is the E4/IX.67 architectural route (replace the SAT close with a parametric
word lemma), not slicing. Per the brief, this is the blocker, stated with numbers.

## 8. Emission, tests and the fixture change

`tools/bridge.ml` changed, so every inlined answer's library text changed. The
tracked answer fixtures under `tests/fixtures/answers/` were **deliberately
re-recorded** with

```text
$ TEKMERION_RECORD_ANSWERS=1 tools/cap.sh 20G -- cargo test --offline --test tools_emit
```

and the digest pins in `tests/tools_receipt.rs` were repinned to the new text
(including the `curve_check` report fixture, now regenerated). The boundary is
recorded in that file next to the digests. `tests/fixtures/emit/bridge_min.ml`
was updated to the new `BRIDGE_CSAT_TAC` anchor. A new test,
`ix69_slice_falls_back_to_the_full_set_and_the_prefix_names_the_task`, pins the
slice-then-fallback order, the two fallback reports, and the task-binding phase
prefix.

```text
$ tools/cap.sh 20G -- cargo test --offline
   40 lines "test result: ok"; 0 lines "test result: FAILED"
$ cargo clippy --offline --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.05s
$ cargo fmt --check
   (exit 0)
$ git ls-files '*.py'
   (no output: 0 tracked Python files)
```

## 9. Environment and commands

```text
$ free -g | head -2
               total        used        free      shared  buff/cache   available
Mem:             125          23          96           0           6         102
$ df -h /mnt/1tb_kingston | tail -1
/dev/nvme1n1p1  1.3T  1.3T  8.1G 100% /mnt/1tb_kingston
```

Every proof build ran sequentially, one target at a time, `make -j1`, behind
`tools/cap.sh 12G -- …`; cargo ran behind `tools/cap.sh 20G`. No run was uncapped
and no two proofs ran at once. Final disk: **8.0 GB free**.

Baseline (same session, unmodified tree, reproduces IX.66 exactly):

```text
$ for t in arm_add_p25519 arm_add_p384 arm_add_p521 arm_neg_p25519 x86_add_p25519 x86_neg_sm2; do
    printf "%-16s " $t; grep '^STAMP_INFERENCES' .bench/ix69/baseline/$t.correct | cut -d' ' -f2; done
arm_add_p25519   119194788
arm_add_p384     241041106
arm_add_p521     470294392
arm_neg_p25519   33981360
x86_add_p25519   112639551
x86_neg_sm2      16422436
```

Sliced (final slice code), totals and kept counts from the same logs:

```text
$ for t in arm_add_p25519 arm_add_p384 arm_add_p521 arm_neg_p25519 x86_add_p25519 x86_neg_sm2; do
    printf "%-16s " $t; grep '^STAMP_INFERENCES' .bench/ix69/sliced/$t.correct | cut -d' ' -f2;
    grep -h '^SLICE kept' .bench/ix69/sliced/$t.correct | cut -d' ' -f2; done
arm_add_p25519   111501987
kept=11/17
kept=11/17
kept=13/17
kept=13/17
arm_add_p384     240835826
kept=23/28
kept=23/28
kept=23/28
kept=23/28
arm_add_p521     422831878
kept=22/28
kept=22/28
kept=22/28
kept=22/28
arm_neg_p25519   33888658
kept=7/11
kept=0/11
kept=7/11
kept=8/11
x86_add_p25519   120053161
kept=12/17
kept=12/17
kept=12/17
kept=12/17
x86_neg_sm2      15444067
kept=3/8
kept=5/8
```

The other 21 tasks ran through `bash .bench/ix69/run_sweep.sh` (same runner,
`-j1`, capped) and finished `ALL_SWEEP_RUNS_DONE` with `rc=0` on every task; the
per-task rows are §6.

## 10. What is not established

- **One host, one kernel, one pin.** All numbers are from this machine and
  `.bench/hol-light-stamp2` (axiom-stamp v2). Nothing here was reproduced on a
  second host or kernel.
- **Statement identity is not `equals_thm` in one session.** It is established by
  construction (`prove` against the frozen `query`) and by byte-identical
  printed `string_of_thm` for `_CORRECT` and `_SUBROUTINE_CORRECT`; see S1.
- **The criterion is syntactic.** Free-variable sharing can keep hypotheses that
  are mathematically irrelevant; the `drop-largest`/`drop-non-eq` controls show
  that `word_ult` is not needed by 2 of the 4 `add_p521` closes, yet the
  criterion keeps it in all 4. So the slice is conservative, and a "smarter"
  per-close slice could save a little more (measured: `drop-non-eq` gives
  406,875,799 on `add_p521`, better than the default 422,831,878, but at the cost
  of two failed attempts; still 13.5% off that task, not 30%).
- **Replay is non-monotone.** The saving is not predictable from the hypothesis
  count or the CNF size (CNF clauses move by 8 on `add_p521`; replay moves by
  47.5M).
- **Only the six have a same-session baseline.** The other 21 were measured for
  acceptance and slicing only.
- **Certificate size (XV.C) is not measured here.**
- **Freeze `v6` is required before any sealed run.** No `v6` tag or freeze was
  created by this task; a sealed evaluation after this change must first create
  and verify the freeze, because `tools/bridge.ml` is inside the frozen runtime.
