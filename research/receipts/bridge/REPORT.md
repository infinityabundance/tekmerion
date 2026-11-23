# Bridge + C‑SAT on P‑256 add — measured result (2026‑11‑23, branch `staging`)

**Bottom line: no `OK`.** The official judge returns `FAIL` on P‑256 add because
CaDiCaL reports the post‑simulation residue SATISFIABLE. The bridge lemmas
themselves all prove in a fresh process; the gap is in how the SAT instance
encodes the branch condition/selection flag, not in the arithmetic lemmas.
Everything below is copied from the logs named next to it. No commit or push
was made.

- Source: `tools/bridge.ml` sha256 `79c5d0d77b7f3bf98a3cdeab9e6d22c3bdcc5b1b1fdfcb02dfd2a8e219cf8346`, 455 lines.
- `tools/csat_lib.ml` sha256 `70439c92d58046a065312fc4faad9e06d53ddd64f89f8e93f676b815cc83e5a8`.
- Working tree: `tools/bridge.ml` and `TODO.md` modified (queue status only); new receipts under
  `research/receipts/bridge/`. Nothing committed or pushed.

## 1. Fresh-process verification of `tools/bridge.ml` (Task 1)

`research/receipts/bridge/instrument.py` mechanically wraps every top-level
`let` in `tools/bridge.ml` in a per-phrase reporter. It does not change any
statement or tactic. The generated file is
`research/receipts/bridge/bridge_instrumented.ml` (sha256 of the source recorded
in its header).

Command (from `.bench/s2n-bignum`):

```
cat research/receipts/bridge/check_fresh.ml | ../hol-light/hol.sh
```

Fresh process: `base.ml`, then `Cadical/make.ml`, then `tools/csat_lib.ml`, then
the instrumented bridge. Log: `research/receipts/bridge/bridge_fresh.log`.

- **All 38 top-level bindings report `BRIDGE_CHECK ... OK`; none fail.**
- Load/prove time for the bridge alone: **0.384 s** (`BRIDGE_CHECK_DONE seconds=0.384`).
- Axiom count: **before = 3, after = 3** (`AXIOMS before=3 after=3`). No new axioms.

The 18 actual theorems (all proved):

```
ADD_MOD_LT ADD_MOD_GE ADD_GE_SUM ADD_MOD_SPLIT
SUB_REM_LT SUB_REM_GE SUB_LT_SUM SUB_GE_SUM SUB_REM_SPLIT
NEG_MOD_ZERO NEG_MOD_NZ NEG_NZ_SUM NEG_MOD_SPLIT
EXP2_NZ LT_EXP_MULT VAL_JOIN_ZERO INT_JOIN_ZERO BITVAL_AS_VAL
```

The remaining bindings are `BRIDGE_SPEC_LEMMAS` plus conversions/tactics:
`REAL_TO_INT_CONV`, `NUM_TO_INT_CONV`, `INT_DISTRIB_CONV`,
`INT_JOIN_ZERO_TH`, `INT_SHIFT_TO_JOIN_CONV`,
`BRIDGE_SHIFTS_TO_JOINS_CONV`, `BRIDGE_TO_WORD_CONV`, `bridge_conditions`,
`BRIDGE_SPLIT_CONDITIONS_TAC`, `BRIDGE_HAS_VAL`, `ELIM_DIGIT_EQS_TAC`,
`BRIDGE_CSAT_TAC`, `BRIDGE_PREPARE_TAC`, `BRIDGE_FINISH_TAC`,
`BRIDGE_ZERO_LT_TAC`, `BRIDGE_REWRITE_MODULUS`, `BRIDGE_ADD_TAC`,
`BRIDGE_SUB_TAC`, `BRIDGE_NEG_TAC`. All type-check; this is not a proof of
their behaviour, only that the declared bindings are well-formed.

### Defects found and fixed in `tools/bridge.ml`

The interrupted file did not previously produce any shift-to-join conversion.
Four conversion defects and one application defect were measured and fixed;
each is a conversion/application fix, not a change to a lemma statement.

1. `NUM_TO_INT_CONV` used `INT_OF_NUM_ADD/MUL/POW` forwards. Those theorems are
   `&m + &n = &(m+n)`; the code needed `GSYM` to push `&` inside. Before the fix
   the whole bignum stayed under one `&`, so no join was ever formed
   (`research/receipts/bridge/conv_test3.log`).
2. `INT_JOIN_ZERO_TH` reduced `dimindex(:N)` with `LAND_CONV(RAND_CONV ...)`,
   which reaches the second operand `&(val x)` and raises `DIMINDEX_CONV`;
   `DEPTH_CONV` swallowed the failure. Fixed to
   `LAND_CONV(LAND_CONV(RAND_CONV DIMINDEX_CONV))`
   (`conv_test4.log`, `conv_test5.log`).
3. `INT_SHIFT_TO_JOIN_CONV` matched `Const("*")`/`Const("pow")`; the actual
   constants are `int_mul`/`int_pow`. Fixed (`conv_test6.log`, `conv_test7.log`).
4. The same conversion assumed positive shifts and rewrote `bitval` at the
   wrong level. Accumulator equations carry negated shifts
   (`-- &(2 EXP 64) * &(bitval carry)`); both are now handled
   (`conv_test7.log`: `SHIFTBIT` and `ACC` convert).
5. `BRIDGE_CSAT_TAC` applied a nested implication with `MATCH_MP_TAC`, which
   peels one implication only (`MATCH_MP_TAC: No match`). Fixed to
   `MP_TAC th THEN ASM_REWRITE_TAC[]` (`debug_add_bridge.log`).

## 2. The generic `ref` lemma (Task 2) — not proved

The intended definition and statement are:

```
ref_add (pw:W word) (x:W word) (y:W word) =
  let s = word_add x y in
  if word_ult s x \/ word_ule pw s then word_sub s pw else s

ADD_REF_VAL:
  !(pw:W word) (x:W word) (y:W word).
    0 < val pw /\ val x < val pw /\ val y < val pw
    ==> val(ref_add pw x y) = (val x + val y) MOD val pw

ADD_REF_VAL_WORD:
  !p (x:W word) (y:W word).
    0 < p /\ p < 2 EXP dimindex(:W) /\ val x < p /\ val y < p
    ==> val(ref_add (word p : W word) x y) = (val x + val y) MOD p
```

The carry is folded into the condition (`word_ult s x` detects overflow of
`x+y`; `word_ule pw s` then tests `p <= s`). This is the shape that keeps all
curve-specific reduction out of the generic lemma.

**Status: not proved.** It is not in `tools/bridge.ml`. The attempt is
`research/receipts/bridge/ref_proof3.ml` (log `ref_proof3.log`). It proves
`SUB_MOVE`, but `SUB_CANCEL`, `MOD_SUB_ONE`, `MOD_ADD_GE_VAR`, `SUB_MOD` and
`ADD_REF_VAL` did not close. The failures are HOL tactic plumbing around
`2 EXP dimindex(:W)` under truncated subtraction (`ARITH_RULE` treats the
exponent as an unconstrained atom); they are **not** a counterexample to the
statement. The library facts needed are `VAL_WORD_ADD_CASES`,
`VAL_WORD_SUB_CASES`, `MOD_ADD_CASES`, `MOD_LT`, `MOD_UNIQ`. Do not cite this
lemma as proved.

## 3. Mechanical no-curve-constant check (V7.3)

`research/receipts/bridge/curve_check.py tools/bridge.ml` strips comments,
extracts every `prove` statement, and flags curve names, large numerals/hex and
known reduction shapes.

- Result: **`statements=20 hits=0`**, exit 0. Log: `research/receipts/bridge/curve_check.log`.
- No `p_256`, `p_384`, `p_521`, `nistp`, `25519`, `secp256k1`, `p256k1`, `sm2`,
  no 5+ digit numeral, no `0x`, no `+ 19`/`2 EXP 255`/`2 EXP 521` shape.

So the current bridge lemmas are class-agnostic. The curve-specific material is
only in the driver (machine code, `p_256`, simulation prefix), not in the lemma
statements.

## 4. Official judge on P‑256 add only (Task 4)

Problem: `arm.bignum_add_p256.BIGNUM_ADD_P256_CORRECT`.

Pipeline used: `retrieve-problem.py`, then `combine-answer-and-setup.py <dir> 1`
(one core, as required on Python 3.14), then `run-answers.sh <eval> 1`, then
`collect-verdicts.py`.

- Judge directory: `.bench/eval-20261123_091448/`
- Judge file: `.bench/eval-20261123_091448/arm.bignum_add_p256.BIGNUM_ADD_P256_CORRECT.judge.txt` = `FAIL`
- `collect-verdicts.py` output: `research/receipts/bridge/judge-verdicts.csv` → `FAIL,1`
- Runtime output: `.bench/eval-20261123_091448/ml_files_arm_bignum_add_p256.run.outlog`

```
ANSWER_EXCEPTION: Failure("CSAT: cadical did not report UNSAT, exit 10")
PROFILE arm.bignum_add_p256.BIGNUM_ADD_P256_CORRECT 8.765 FAIL
```

- **End-to-end judge time: 8.765 s.** No theorem is produced, so there are no
  hypotheses and no axiom delta for the judge run (the tactic raised before
  `prove` returned). The fresh bridge load itself has axiom delta 0 (§1).
- CaDiCaL returned exit 10 (SATISFIABLE), so the kernel correctly refused it.

The answer had to be statically inlined (Cadical `dimacs.ml`/`cnf.ml`/
`ldrup.ml`, `csat_lib.ml`, `bridge.ml`) because the syntax gate runs
`ocamlc -i` and `loadt`-introduced names are not statically visible. The inlined
answer is generated by `research/receipts/bridge/make_inlined_answer.py` into
`research/receipts/bridge/judge-work/.../answer.txt`. This is packaging, not a
change to the tactics.

### Per-goal timing (debug harness, same bridge via `loadt`)

`research/receipts/bridge/debug_add_bridge.ml`, log `debug_add_bridge.log`:

| residue | result | seconds | CaDiCaL |
|---|---|---|---|
| LT branch, `carry_s16 = true` | **closed (UNSAT)** | 29.862 | UNSAT |
| LT branch, `carry_s16 = false` | **not closed** | 22.188 | SAT (exit 10) |

The judge failed in 8.765 s on the first residue it reached; SAT run time is not
stable across clause orderings, so the judge total is not the sum of these two
debug numbers. It is not a timeout: the exception is CaDiCaL exit 10.

## 5. What does not follow, precisely

The post-simulation residual is (from `research/receipts/bridge/probe_add.log`):

```
(m + n) MOD p_256 =
bignum_of_wordlist
  [if carry_s16 then sum_s3 else sum_s10;
   if carry_s16 then sum_s4 else sum_s12;
   if carry_s16 then sum_s7 else sum_s13;
   if carry_s16 then sum_s8 else sum_s15]
```

After the verified bridge conversion, the `carry_s16 = false` LT residue is the
implication printed in `debug_add_bridge.log` (`DEBUG_CSAT_2_START ... END`).
Its first antecedent is

```
&(val m_0) + &(val (word_join m_1 (word 0))) + ...
  + &(val n_0) + ...  <  &p
```

i.e. `&(m + n) < &p` over the joined inputs, and the conclusion is the
256-bit-limb sum identity with the corrected limbs `sum_s10, sum_s12, sum_s13,
sum_s15`.

CaDiCaL says this is satisfiable. Two encoding facts are consistent with that,
and the first is measured from the library:

1. **The 257-bit comparison is wordified as a signed `ival` comparison.**
   `IWORDIFY_CONV` chooses `pty = max(widths) + 1 = 257` for
   `(256-bit) + (256-bit)`, while the true sum can be `2^257 - 2`. Values
   `>= 2^256` then have the top bit set and `ival` (via `IVAL_LT`, used by
   `IWORDIFY_ATOM_CONV`) reads them as negative. The LT branch hypothesis is
   therefore not the intended unsigned comparison; SAT can satisfy the residue
   with a true sum outside `[0, p)`.
2. **The selection flag is only linked to the corrected limbs.** The final
   accumulator equation has `sum_s16` free enough that the flag can be set on
   the “wrong” side; the machine’s flag/limb pairing is not forced by the
   per-equation encoding alone.

We did not isolate which fact is sufficient; the measured SAT result is the
receipt, and the width/signedness argument is the concrete reason the LT branch
is not faithfully encoded. This is why the reference-function formulation is
the right next step: its condition is a 256-bit `word_ult`/`word_ule` circuit,
not a 257-bit `num` comparison.

## 6. `DEVELOPMENT_ONLY` list

- The P-256 machine-code words and `define_assert_from_elf` in
  `probe_add.ml`, `debug_add_bridge.ml`, and the judge answer: transcribed from
  `tools/csat_bridge.ml` / `tools/w1_hol_capture.ml` / the s2n-bignum proof.
- The simulation prefix `BIGNUM_DIGITIZE_TAC`, `ARM_ACCSTEPS_TAC
  BIGNUM_ADD_P256_EXEC [3;4;7;8;10;12;13;15;16] (1--22)`,
  `ENSURES_FINAL_STATE_TAC`, `DISCARD_STATE_TAC "s22"`: hand-written /
  transcribed, not derived from a witness.
- The `p_256` numeral in the driver: a curve constant, deliberately outside the
  bridge lemma statements.
- The inlined judge answer: a packaging step to satisfy the static syntax gate.

No statement weakening and no new axioms (axiom count 3 → 3). The bridge lemmas
were not proved by `CHEAT_TAC`.

## 7. Still wrong / next action

- **No `OK`.** The acceptance is the clean localised negative above.
- `ADD_REF_VAL` is unproved and is not in the loaded path.
- The split-based SAT close is incomplete; do not advertise it as a closer.
- Before any production claim, the reference-function route still needs the
  generic lemma proved and a SAT instance that proves the machine output equals
  `ref_add` with a 256-bit condition. The current `bridge.ml` is only a verified
  set of arithmetic bridges plus a SAT conversion that reaches, but does not
  close, the P-256 add residue.

---

# Follow-up 2026-11-23, branch `staging` — diagnosis withdrawn, judge `OK`

This section supersedes §2, §4, §5 and §7 above. The earlier signedness
hypothesis is **withdrawn**; two concrete defects were measured and fixed, and
the official judge now returns `OK` on P-256 add.

- `tools/bridge.ml` sha256 `894c05f41f8211d78de5af44edbead23992abb6a28a16e53cc94e513a3052c38`, 49 phrases.
- `tools/csat_bridge.ml` now uses `top_thm()` in `run_op` (was `b()`, a
  type error that meant `sub`/`neg` had never run).
- New regression test: `tools/hol_tests/bridge_flag_wordify.ml`.

## 1. The signedness hypothesis is false (V8.4)

The failing residue is the carry_s16=false sub-branch of the *true* arithmetic
branch (`m+n < p`). The old debug harness called it "the false branch" and did
not notice that both printed residues carry `m+n < p`.

Measured operand widths (`research/receipts/bridge/confirm_v8_4.log`, from
`v8_4_confirm2.ml`):

```
VALWIDTH word_join (word (bitval carry)) (word 0) : (65)word
VALWIDTH sum_* : (64)word
VALWIDTH word_join m_1 (word 0) : (128)word ; m_2:192 ; m_3:256
```

`IWORDIFY_CONV` chooses **257** bits for `&p` (top bit clear, so `ival` is
positive) and the eight-way sum is wordified at **bits 259/260**. The claim
that the sum is 257 bits and reads negative is therefore false; the
Comparison is faithful in width and sign.

Controlled experiment on the exact residue (`v8_4_confirm3.ml`; int terms built
from the residue's own subterms, because `&` in a quotation is `real_of_num`):

| variant | result | seconds |
|---|---|---|
| V0 residue as produced | **SAT** | 21.634 |
| V2 residue + `bitval c8 = bitval c15 + val s16` | **UNSAT** | 29.080 |
| V4 V2's flag equation + equations ==> `B + p = A_total` | **UNSAT** | 30.815 |

So the missing fact is the flag equation, not a signed comparison.

### Root cause A: the split substitutes a numeral and the shift conversion rejected it

`BRIDGE_SPLIT_CONDITIONS_TAC` replaces `bitval carry_s16` by the literal 0 or 1,
so the accumulator equation becomes `--2^64 * &0 + val s16 = ...`.
`INT_SHIFT_TO_JOIN_CONV` only accepted `&(val x)` and `&(bitval b)` operands, so
that equation could not be wordified. BITBLAST left it as an opaque int atom;
the flag it constrains was therefore free and the false sub-branch was
satisfiable.

Fix: `INT_SHIFT_TO_JOIN_CONV` now also accepts a numeral operand. `bits_of_num`
places `&n` in a word narrow enough that `val(word n)` is exact, and the
product becomes a join (`2^64 * &0` becomes the zero join), never a multiplier.

### Root cause B: `BRIDGE_CSAT_TAC` never discharged its goal

The old tail was `MP_TAC th THEN ASM_REWRITE_TAC[]`. That rewrites the
antecedents away but leaves `w` itself as the goal, so the tactic reported
success while closing nothing. Measured before the fix: four SAT refutations
all succeeded and **four goals remained**. Fix: chain `MP` with the assumption
theorems and `ACCEPT_TAC` the result.

## 2. Failing-then-passing regression test

`tools/hol_tests/bridge_flag_wordify.ml` proves, for the split flag equation,
the consequence each literal forces: `c16=0 ==> val s16 <= 1` and
`c16=1 ==> 2^64-2 <= val s16`, plus the unsplit disjunction. It runs without the
ARM base and mentions no curve constant.

| run | split_zero | split_one | unsplit_bitval |
|---|---|---|---|
| before the fix (`regression_before_fix.log`) | FAIL (SAT) | FAIL (SAT) | PASS |
| after the fix (`regression_after_fix.log`) | PASS | PASS | PASS |

This is the measured distinction the old conversion had: the bitval form was
encoded, the literal form was not.

## 3. `ADD_REF_VAL` proved and loaded (V8.5)

`tools/bridge.ml` now contains `ref_add` and proves, in a fresh base:

```
ADD_REF_VAL : !pw x y. 0 < val pw /\ val x < val pw /\ val y < val pw
              ==> val(ref_add pw x y) = (val x + val y) MOD val pw
ADD_REF_VAL_WORD : for pw = word p, with p < 2^dimindex(:W)
```

`ref_add pw x y = let s = word_add x y in
                  if word_ult s x \/ word_ule pw s then word_sub s pw else s`.

Fresh process (`bridge_fresh.log`): **49/49 phrases `OK`** (25 `prove`
results, `ref_add` is a definition, the rest are conversions/tactics/lists),
`ADD_REF_VAL OK`, `ADD_REF_VAL_WORD OK`, **axioms 3 -> 3**, load 2.249 s.
No-curve check (`curve_check.log`): **statements=28 hits=0**. The lemma
statement mentions no curve name and no 5+ digit numeral.

One proof detail: `REWRITE_TAC[ref_add]` leaves `LET_END`; the proof needs
`LET_END_DEF` as well as `LET_DEF`. This is the only reason the earlier draft
stalled at `MATCH_MP_TAC`.

## 4. `INT_DISTRIB_CONV` (V8.6)

`INT_DISTRIB_CONV` is not the defect: it distributes `+` over `*` correctly.
The multiplier circuit came from `INT_SHIFT_TO_JOIN_CONV` refusing the literal
operand produced by the case split. With that fixed, `2^64 * &0` and
`2^64 * &1` are joins, not multipliers. Measured SAT times are unchanged for
the sub-branches (below); the *failing* branch is now UNSAT rather than SAT.

## 5. Official judge on P-256 add (V8.8)

```
retrieve-problem.py retrieve --outputdir work-add-p256 --name arm.bignum_add_p256.BIGNUM_ADD_P256_CORRECT
make_inlined_answer.py .../answer.txt          # inlines the final bridge
combine-answer-and-setup.py work-add-p256 1    # syntactic check PASSED
run-answers.sh eval-20261123_114817 1
collect-verdicts.py eval-20261123_114817 judge-verdicts-after-fix.csv
```

- Judge file `.bench/eval-20261123_114817/arm.bignum_add_p256.BIGNUM_ADD_P256_CORRECT.judge.txt` = **`OK`**.
- `collect-verdicts.py` output: `OK,1` (`judge-verdicts-after-fix.csv`).
- **End-to-end judge time: 23.838 s** (`PROFILE ... 23.838 OK`).

Local bridge run (`run_add_bridge3.log`), no incremental compilation:

| sub-branch | seconds |
|---|---|
| `m+n < p`, carry true | 30.804 |
| `m+n < p`, carry false | 30.008 |
| `p <= m+n`, first split | 34.967 |
| `p <= m+n`, second split | 41.266 |
| **total, 4 SAT calls** | **137.344** |

`ADD_CLOSED hyps=0 axioms_before=3 axioms_after=3`. The judge is faster because
the answer is native-compiled (`ocamlopt`), while this table is the interpreted
HOL process.

## 6. `sub` and `neg` (after the `OK`)

`tools/csat_bridge.ml` (`csat_bridge_subneg.log`):

| op | result | hypotheses | seconds |
|---|---|---|---|
| add | closed | 0 | 137.003 |
| sub | closed | 0 | 94.877 |
| neg | **CSAT SAT (exit 10)** | - | 24.728 |

`sub` is a measured local success (not yet judge-run). `neg` is a measured
negative: its residual still has an unencoded branch. It is not fixed here and
is recorded as the next defect.

## 7. `DEVELOPMENT_ONLY` (this follow-up)

- P-256 machine-code words, goals and simulation prefixes in
  `research/receipts/bridge/*.ml`, `tools/csat_bridge.ml` and the judge answer:
  transcribed, not derived from a witness.
- `sub_middle` (`FLAG_FROM_CARRY_LT`, `DECARRY_RULE`, `BOUNDER_TAC`): transcribed
  from the human proof; development only.
- The `p_256` numeral in drivers: a curve constant, outside every bridge lemma.
- The inlined judge answer: packaging to satisfy the static syntax gate.

## 8. Still wrong

- `neg` does not close (SAT on the post-simulation residue).
- The reference route is proved and loaded but the judge answer still drives
  `BRIDGE_ADD_TAC` (the now-correct split); `ADD_REF_VAL` is not yet wired into
  a `BRIDGE_*_REF_TAC`.
- Benchmark `OK` is valid under the bench rules (`CHEAT_TAC` context); it is
  **not** an axiom-free development. Replay in a full s2n-bignum context is a
  separate result (D8).
