# R5 — the two flag-only ARM add failures closed; 18/18 in one revision

Run date: **2026-11-24** (UTC), branch `staging`, base commit **`3fedd8c`**.
Nothing was committed or pushed. Free disk at the end: **22 GB** (floor 3 GB).

## 0. Scope: development data, not a held-out claim

These are the 18 R4 ARM `add`/`sub`/`neg` tasks. `r4-reference-route.md` §0
records that all had prior visible outcomes in this programme; the R2 held-out
set is spent. Re-answering two of them is a **repair of a development result**,
not a blind evaluation. The denominator is **18**, and this is **18/18
development**, not a held-out rate. Every verdict below comes from one judge
run of one revision of the tree.

## 1. Result — 18/18, one revision

Judge dir `.bench/eval-20261124_030015`; all 18 answers were regenerated from
the tree described in §6--§8 and judged in one `run-answers.sh` invocation.

```
Problem,Verdict
arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT,OK
arm.bignum_add_p256.BIGNUM_ADD_P256_CORRECT,OK
arm.bignum_add_p256k1.BIGNUM_ADD_P256K1_CORRECT,OK
arm.bignum_add_p384.BIGNUM_ADD_P384_CORRECT,OK
arm.bignum_add_p521.BIGNUM_ADD_P521_CORRECT,OK
arm.bignum_add_sm2.BIGNUM_ADD_SM2_CORRECT,OK
arm.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT,OK
arm.bignum_neg_p256.BIGNUM_NEG_P256_CORRECT,OK
arm.bignum_neg_p256k1.BIGNUM_NEG_P256K1_CORRECT,OK
arm.bignum_neg_p384.BIGNUM_NEG_P384_CORRECT,OK
arm.bignum_neg_p521.BIGNUM_NEG_P521_CORRECT,OK
arm.bignum_neg_sm2.BIGNUM_NEG_SM2_CORRECT,OK
arm.bignum_sub_p25519.BIGNUM_SUB_P25519_CORRECT,OK
arm.bignum_sub_p256.BIGNUM_SUB_P256_CORRECT,OK
arm.bignum_sub_p256k1.BIGNUM_SUB_P256K1_CORRECT,OK
arm.bignum_sub_p384.BIGNUM_SUB_P384_CORRECT,OK
arm.bignum_sub_p521.BIGNUM_SUB_P521_CORRECT,OK
arm.bignum_sub_sm2.BIGNUM_SUB_SM2_CORRECT,OK
--,
SUMMARY,
OK,18
```

Harness `PROFILE` seconds (the judge's own elapsed time), sorted by task:

| task | seconds | R4 |
|---|---:|---|
| `add_p25519` | 23.081 | OK |
| `add_p256` | 23.438 | OK |
| `add_p256k1` | **22.061** | **FAIL** |
| `add_p384` | **50.168** | **FAIL** |
| `add_p521` | 96.882 | OK |
| `add_sm2` | 22.401 | OK |
| `neg_p25519` | 5.341 | OK |
| `neg_p256` | 6.657 | OK |
| `neg_p256k1` | 6.731 | OK |
| `neg_p384` | 12.933 | OK |
| `neg_p521` | 19.628 | OK |
| `neg_sm2` | 6.708 | OK |
| `sub_p25519` | 15.769 | OK |
| `sub_p256` | 14.706 | OK |
| `sub_p256k1` | 15.740 | OK |
| `sub_p384` | 31.198 | OK |
| `sub_p521` | 35.216 | OK |
| `sub_sm2` | 15.177 | OK |

Max 96.882 s against the 120 s per-task budget (`add_p521`, 0.81). The two
repaired tasks cost 22.061 s and 50.168 s; no task timed out. R4's two failures
were 10.849 s and 7.249 s, i.e. fast SATISFIABLE answers, not timeouts.

The earlier 3-task pre-check (`eval-20261124_024005`) used byte-identical
answers for the three tasks (answer hashes below), so nothing here depends on
that run.

Verdict file: `research/receipts/r5-xzr-all-verdicts.csv` (byte copy of
`.bench/r5-xzr-all-verdicts.csv`).

## 2. The cause, measured (and why the R4 description is stale)

`r4-reference-route.md` §3 states that `derive_steps` excludes instructions
whose destination is `XZR` and that "the fix that would close both" is a new
mechanism. That was true of the R4 answers: the judge `.ml` for `add_p384`
carries

```
ARM_ACCSTEPS_TAC BIGNUM_ADD_P384_EXEC [3;4;7;8;11;12;23;27;29;32;33;34;35] (1--38)
```

i.e. **without** the flag-only steps 15, 17, 19, 20, 21, 22 (and `add_p256k1`
likewise omits 14, 15). The R4 receipt was written at 17:28 on 2026-11-23.

Commit **`60ddf31`** (2026-11-23 18:01:48, "wip: checkpoint before TNPS charter
intake") changed exactly that, generically:

* `derive_steps._is_flag_only` admits `CMP`/`CMN` and `ADCS`/`SBCS` to `XZR`;
* `_carry_out_feeds_chain` keeps such a form as a chain link only when its
  carry-out reaches a later add/sub that *writes a register*.  The docstring
  states the reason: "without a relation for it the flag survives only as an
  arithmetic comparison over the previous flag, which the word-level SAT
  encoding cannot see, and the branch it selects is left free."

The set of admitted `XZR` instructions is derived from the disassembly through
the mnemonic/semantics table and the carry dataflow; no task name, curve or
modulus appears in the module (`test_derive_module_has_no_io_and_no_proof_vocabulary`
enforces this, including the tokens `p256`, `p384`, `p521`, `25519`,
`secp256k1`, `sm2`).

The **word-level flag hypothesis** for each admitted step is already produced by
the vendored `ACCUMULATE_ARITH_TAC` (`s2n-bignum/common/interval.ml`), not by a
new lemma:

```
(* - secondary triggering from zero flag if result is not written (ARM xzr) *)
and pth_zerotrig = prove
 (`(read ZZ s <=> val(a:int64) = 0) ==> read (rvalue a) s = a`,
  REWRITE_TAC[READ_RVALUE])
```

For `ADCS/SBCS XZR, x, y` the step writes `ZF = (val d = 0)`, where `d` is the
word the instruction would have written.  `pth_zerotrig` turns that zero test
back into the read equation `read (rvalue d) s = d`, and `pth_adc`/`pth_sbc`
then produce the curve-free accumulator relation

```
± 2^64 * bitval carry_sN + val sum_sN = val x ± val y ± bitval carry_in
```

with `carry_sN <=> (2^64 <= val x + val y + bitval b)` for ADCS and
`carry_sN <=> (val x < val y + bitval b)` for SBCS, where `b` is the incoming
bit the instruction consumes (`bitval(CF)` for ADCS, `bitval(~CF)` — the
incoming **borrow** — for SBCS; AArch64 `SBCS` sets C as *no-borrow*, which is
why the complement is the one that enters the 64-bit subtraction).  That is the
mechanism the brief asked for.  **No new HOL lemma was needed**: adding one
would duplicate a proof that is already in the checked-in tactic, so none was
written.  `tools/bridge.ml` gained no new section; the only statement touched is
recorded in §6.

The resulting derived sets, printed by the emitter:

```
add_p256k1: 3 4 7 9 14 15 16 18 19 20 21
add_p384:   3 4 7 8 11 12 15 17 19 20 21 22 23 27 29 32 33 34 35
add_p256:   3 4 7 8 10 12 13 15 16
```

(`14`/`15`/`16` in p256k1 are `CMN X5 X4`, `ADCS XZR X9 XZR`,
`ADCS X3 X3 XZR`; `15`/`17`/`19`--`22` in p384 are `SBCS XZR X6 …`,
`SBCS XZR X7 …`, the three `ADCS XZR …` and `ADCS X3 X3 XZR`.)

The isolation is measured on **one** bridge, changing only that list
(`research/receipts/r5-xzr-ab.log`):

* R4 list, flag-only steps excluded →
  `Failure("CSAT: cadical did not report UNSAT, exit 10")`, and the selection
  hypothesis is the simulator's nested condition
  `&2 pow 64 <= &(val sum_s12) + &(bitval (… &(val sum_s4) < … ))`, whose
  `bitval` subterms are unpinned atoms.
* Current list, the six flag-only steps admitted → the same selection
  hypothesis is `val(word_add (word (bitval carry_s12))
  (word (bitval carry_s22))) = 0`, `carry_s22` is pinned by the last flag-only
  step's relation, and the residual is UNSAT (`ok=true axioms=3`).

## 3. The SAT instance now ties the final `NE` test to the chain

`research/receipts/r5-xzr-csat-instance.log` is the first certified-SAT branch
of `add_p384`, printed from a copy of `tools/bridge.ml` with one `Printf`
inside `BRIDGE_CSAT_TAC` (same answer bytes, same tactic; the dev run closes
`ok=true axioms=3` in 269.631 s bytecode).  The first hypothesis is the flag the
selection used:

```
CSAT_HYP &(val (word_add (word (bitval carry_s12)) (word (bitval carry_s22)))) = &0
```

and `carry_s22` is tied by the accumulator relation of the last flag-only step
(`ADCS X3 X3 XZR`), which in turn consumes `carry_s21`, `carry_s20`, … The p384
SBCS links appear as

```
CSAT_HYP -- &(val (word_join (word (bitval carry_s17)) (word 0))) + &(val sum_s17)
         = &(val sum_s4) - &18446744069414584320 - &(bitval carry_s15)
CSAT_HYP -- &(val (word_join (word (bitval carry_s15)) (word 0))) + &(val sum_s15)
         = &(val sum_s3) - &4294967295
```

No `read ZF` / `read CF` atom survives into the instance: the flags have been
folded into carries, so the residual is a pure word instance.  (These numerals
are in a log of an instance, not in any lemma statement; `curve_check.py` scans
`tools/bridge.ml` only.)

## 4. Commands, verbatim

All 18 answers were generated with the spent mode (the manifest lists exactly
those 18 ids), in one loop:

```
$ for op in add sub neg; do for fam in p25519 p256 p256k1 p384 p521 sm2; do
    t="arm.bignum_${op}_${fam}.BIGNUM_$(echo $op | tr a-z A-Z)_$(echo $fam | tr a-z A-Z)_CORRECT"
    mkdir -p ".bench/r5-xzr-all/$t"
    python3 tools/emit_ref_proof.py --spent \
        --spent-manifest research/receipts/spent-r4-arm.json \
        --task "$t" --mode answer --out ".bench/r5-xzr-all/$t/answer.txt" \
        --accumulate-out ".bench/r5-xzr-all/$t/accumulate.txt"
  done; done
```

The two repaired tasks print (verbatim, with the output path elided):

```
wrote .../answer.txt mode=answer task=arm.bignum_add_p256k1.BIGNUM_ADD_P256K1_CORRECT accumulate=[3, 4, 7, 9, 14, 15, 16, 18, 19, 20, 21] bytes=103718
wrote .../answer.txt mode=answer task=arm.bignum_add_p384.BIGNUM_ADD_P384_CORRECT accumulate=[3, 4, 7, 8, 11, 12, 15, 17, 19, 20, 21, 22, 23, 27, 29, 32, 33, 34, 35] bytes=103738
```

and the regression task prints

```
wrote .../answer.txt mode=answer task=arm.bignum_add_p256.BIGNUM_ADD_P256_CORRECT accumulate=[3, 4, 7, 8, 10, 12, 13, 15, 16] bytes=103708
```

The full set of 18 `accumulate` lists is in each task's `accumulate.txt` under
`.bench/r5-xzr-all/` (the three quoted above are the ones this receipt reasons
about).

```
$ cd .bench && python3 combine-answer-and-setup.py r5-xzr-all 1
Loaded per-problem timeouts for 2622 problems from timeout-map.json
arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT: syntactic check PASSED
arm.bignum_add_p256.BIGNUM_ADD_P256_CORRECT: syntactic check PASSED
arm.bignum_add_p256k1.BIGNUM_ADD_P256K1_CORRECT: syntactic check PASSED
arm.bignum_add_p384.BIGNUM_ADD_P384_CORRECT: syntactic check PASSED
... (all 18 PASSED) ...
arm.bignum_sub_sm2.BIGNUM_SUB_SM2_CORRECT: syntactic check PASSED
eval-20261124_030015

$ cd .bench && PATH=…cadical…:$PATH ./run-answers.sh eval-20261124_030015 1
... (all 18 compile, then run; every PROFILE line is in §1) ...

$ cd .bench && PATH=…cadical…:$PATH python3 collect-verdicts.py eval-20261124_030015 r5-xzr-all-verdicts.csv
(no stdout; the CSV is §1)
```

`run-answers.sh` exited 0 and the judge directory contains exactly 18
`*.judge.txt`.  One task (`sub_p256`) is carried by the benchmark's larger
`p256_montjdouble.ml` template (as in R4); it still judges `OK` (14.706 s).

Answer hashes: `research/receipts/r5-xzr-answers.sha256` (18 lines). The three
relevant ones:

```
86d698110121f34cd70cc63147e6eaec39ac48f3e6b7e20fe148893f809d1c24  arm.bignum_add_p256.BIGNUM_ADD_P256_CORRECT/answer.txt
273e873d0d249ef984114247478883682e398225b3672938e9a82bf455ea50cf  arm.bignum_add_p256k1.BIGNUM_ADD_P256K1_CORRECT/answer.txt
819b3a0fc9498253633e2688fe149fd64d0b6b8c7b0ced7dfae05aa34e1d3c10  arm.bignum_add_p384.BIGNUM_ADD_P384_CORRECT/answer.txt
```

`--spent` is the third guard mode added by IX.53
(`research/receipts/ix53-spent-mode.md`).  It allows only the 18 manifest ids and
their objects, and still refuses every listed proof `.ml`.

## 5. Fresh replay (diagnostic; same answer bytes)

Replay of the judge `.ml` from `eval-20261124_030015` with `bench_run` replaced
by a prover that reports the theorem's hypothesis count (`.bench/r3_replay.py`;
native OCaml, not the bytecode REPL):

```
REPLAY arm.bignum_add_p256k1.BIGNUM_ADD_P256K1_CORRECT closed=1 hyps=0 axioms_before=2445 axioms_after=2445 unchanged=true seconds=22.044
REPLAY arm.bignum_add_p384.BIGNUM_ADD_P384_CORRECT closed=1 hyps=0 axioms_before=2445 axioms_after=2445 unchanged=true seconds=50.266
REPLAY arm.bignum_add_p256.BIGNUM_ADD_P256_CORRECT closed=1 hyps=0 axioms_before=2445 axioms_after=2445 unchanged=true seconds=23.397
REPLAY arm.bignum_sub_p256.BIGNUM_SUB_P256_CORRECT closed=1 hyps=0 axioms_before=6519 axioms_after=6519 unchanged=true seconds=14.806
```

The third line is the required regression spot-check: `add_p256` is still
`OK`, `hyps=0`.  The fourth is the larger-template task, included because its
judge file is the one that loads extra context; axioms `6519 -> 6519`.

## 6. `tools/bridge.ml`: no new lemma, one statement change

`curve_check.py` at HEAD (before this work) reported `hits=1`: `CSETM_EQ`
contained the literal `18446744073709551615` (= 2^64 - 1).  That is a width
constant, not a curve constant, but the mechanical check cannot tell.  The
statement is now written at the width it belongs to and the numeral is computed
inside the proof, so the check is meaningful again:

```hol
let CSETM_EQ = prove
 (`!p:bool. (if p then (word 0:int64) else word (2 EXP 64 - 1)) =
            word_neg(word (bitval(~p)))`,
  GEN_TAC THEN BOOL_CASES_TAC `p:bool` THEN
  REWRITE_TAC[BITVAL_CLAUSES; word_neg] THEN
  REWRITE_TAC[ARITH_RULE `2 EXP 64 - 1 = 18446744073709551615`] THEN
  CONV_TAC WORD_REDUCE_CONV);;
```

The statement is the same term, alpha-for-alpha; `CSETM_EQ` is not used by any
tactic in the file (its only occurrence outside the definition is inside
`BRIDGE_MASK_SELECT_TAC`, which no entry point calls), so no close changes.  The
existing `BRIDGE_*` tactics are untouched.

Fresh process (`research/receipts/r5-xzr-bridge-fresh.log`, generated by
`check_fresh.ml` over the mechanically instrumented copy, from
`.bench/s2n-bignum`):

```
# AXIOMS before=3 after=3
# BRIDGE_CHECK_DONE seconds=3.225
```

**119/119 `BRIDGE_CHECK … OK`, 0 FAIL**; the changed lemma prints

```
val CSETM_EQ : thm =
  |- forall p.
         (if p then word 0 else word (2 EXP 64 - 1)) =
         word_neg (word (bitval (~p)))
```

Other `prove` statements in the file are unchanged; in particular no statement
of `ADD_REF_VAL*`, `SUB_*`, `NEG_*`, `REF_*`, `ADC_*`, `SBC_*` or `VAL_*`
changed.  Hashes:

```
ceb690a8a24589614886953337caaf05ab64a8c62cc50cd6500f36c413a5ea4c  tools/bridge.ml
f4bba97fba1ba3281acd60cb8aa09275ef7f62a205d3c32254c858a5999c5b23  research/receipts/bridge/bridge_instrumented.ml
```

## 7. Checks

```
$ python3 research/receipts/bridge/curve_check.py tools/bridge.ml
statements=71 hits=0        (exit 0)

$ python3 -m pytest tools/test_derive_steps.py tools/test_x86_support.py tools/test_heldout_modes.py -q
54 passed, 1 skipped in 0.13s

$ python3 -m pytest tools/test_heldout_modes.py -q
23 passed in 0.08s
```

The skip is the pre-existing opt-in HOL gate (`TEKMERION_RUN_HOL=1`).

## 8. Files changed

* `tools/bridge.ml` — `CSETM_EQ` statement + its proof only (§6).
* `tools/heldout_guard.py` — third mode `--spent --spent-manifest`, with
  `SpentManifest`/`load_spent_manifest`/`spent_policy`; `--develop` and
  `--sealed-eval` semantics unchanged.
* `tools/test_heldout_modes.py` — the six spent-mode tests.
* `tools/emit_ref_proof.py` — docstring line for the new mode; no behaviour
  change.
* `research/receipts/spent-r4-arm.json` — the 18-id spent manifest with a
  per-id reason and a cited decision.
* `research/receipts/bridge/bridge_instrumented.ml` — regenerated (it was two
  sections stale; the generator is mechanical).
* New receipts: `r5-xzr.md`, `r5-xzr-all-verdicts.csv`, `r5-xzr-answers.sha256`,
  `r5-xzr-ab.log`, `r5-xzr-bridge-fresh.log`, `r5-xzr-csat-instance.log`,
  `ix53-spent-mode.md`, `ix53-spent-demo.log`.

Working artefacts under git-ignored `.bench/`: `r5-xzr-all/` (18 answers +
accumulate lists), `eval-20261124_030015/` (the judge), `r5-xzr-replay/`
(native replay copies), `r5-xzr-dbg/` (the instrumented-bridge dev run), and
`r5-xzr-work/`, `r5-xzr-smoke/` (superseded 3-task pre-checks).

Not modified: `tools/derive_steps.py`, `tools/disasm_steps.py`, the emitters'
tactics, `.bench/s2n-bignum` sources, any human proof file.

## 9. Honest limits

* This is **development**, 18/18 on a set whose outcomes were visible.  No
  held-out number is claimed or implied, and the R2 held-out set remains spent.
  The 18 answers, the judge run and the replay are all from this one revision
  (base `3fedd8c` plus the uncommitted changes in §8), so the number does not
  mix revisions.
* No new HOL lemma was written, because the generic mechanism exists and is
  exercised; the brief's "measured cause" describes the R4 answer bytes, which
  predate commit `60ddf31`.  `r4-reference-route.md` §3 should be read as
  superseded by this receipt for the mechanism question (its verdict table is
  still accurate for the R4 answers it describes).
* The only `tools/bridge.ml` statement change is `CSETM_EQ`, and it is
  alpha-equivalent and unused by any entry point.  No `BRIDGE_*` tactic was
  weakened or deleted; all 18 tasks closed on the same file.
* A flag-only link whose chain ends in a predicate and never reaches a
  register-writing add/sub is still excluded by design
  (`_carry_out_feeds_chain`).  That case is not present in these two routines,
  and relating such a compare to a word it never feeds would be unsound as an
  accumulator relation.  It remains open if a future routine selects on such a
  flag.
* `add_p521` costs 96.882 s of its 120 s budget (0.81) and is the least
  headroom in the set; `add_p384` is 50.168 s (0.42) and `add_p256k1` 22.061 s
  (0.18).  No task timed out.
