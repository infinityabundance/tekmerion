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
