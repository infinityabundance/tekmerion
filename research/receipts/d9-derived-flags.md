# D9 — derived (not transcribed) P-384 add simulation facts

Branch `staging`, working tree only; nothing committed or pushed. This note records the
three facts that used to be transcribed in
`research/receipts/bridge/make_inlined_answer.py` (`TACTIC_ADD_P384`), how they are now
derived, and the judge verdict with the artifacts that support it.

## 1. What was derived

Three things were transcribed from `arm/proofs/bignum_add_p384.ml` and are now proved from
the accumulator state emitted by `ARM_ACCSTEPS_TAC`:

| # | was (hand-written) | now |
|---|---|---|
| 1 | `2 EXP 384 * bitval carry_s12 + bignum_of_wordlist [sum_s3;…;sum_s12] = m + n` | `BRIDGE_CARRY_CONSERVE_TAC` folds the per-limb equations with `CARRY_CHAIN_ADD_STEP` / `CARRY_CHAIN_ADD_BASE` and `ASSUME_TAC`s the telescope |
| 2 | `carry_s22 <=> p_384 <= bignum_of_wordlist [sum_s3;…;sum_s12]` via `FLAG_FROM_CARRY_LE` with a hand-chosen `384` | `BRIDGE_CMP_FLAG_TAC p_384` folds the compare chain with `CARRY_CHAIN_SUB_STEP` / `CARRY_CHAIN_SUB_BASE`, evaluates the subtrahend limbs to `p_384`, and applies `SUB_CHAIN_FLAG` + `FLAG_LE` |
| 3 | `read X3 s = word_neg(word(bitval(p_384 <= m + n)))` | `BRIDGE_MASK_SELECT_TAC` reads the CSETM assumption, rewrites its two-bit condition with `VAL_WORD_ADD_BITVAL` / `BITVAL_ADD_ZERO`, and applies `CSETM_EQ` |

The generic lemmas are in `tools/bridge.ml` §9. They are parameterised by the limb words and
carries; **no curve constant occurs in any lemma or tactic body** (§5). The modulus enters only
as the `pth` argument of `BRIDGE_CMP_FLAG_TAC`, and only as an evaluated instance
(`bignum_of_wordlist ys = p_384`). The mask fact produced in this run is

```
read X3 s24 =
  word_neg (word (bitval (carry_s12 \/ p_384 <= bignum_of_wordlist
    [sum_s3; sum_s4; sum_s7; sum_s8; sum_s11; sum_s12])))
```

(the `carry_s22` in the condition was already rewritten to its derived meaning by the
accumulator machinery after `BRIDGE_CMP_FLAG_TAC`, so the mask is stated over the two
carries the routine actually tests).

The new tactic sequence in `TACTIC_ADD_P384` is

```
ARM_ACCSTEPS_TAC … (1--13) (1--13) THEN BRIDGE_CARRY_CONSERVE_TAC THEN
ARM_ACCSTEPS_TAC … (14--22) (14--22) THEN BRIDGE_CMP_FLAG_TAC p_384 THEN
ARM_STEPS_TAC … [23;24] THEN BRIDGE_MASK_SELECT_TAC THEN
ARM_ACCSTEPS_TAC … (25--38) (25--38) THEN …
```

## 2. Removed glue (diff)

`research/receipts/d9-removed-glue.diff` (verbatim `git diff` of
`research/receipts/bridge/make_inlined_answer.py`, 56 lines). The removed block is the whole
`TACTIC_ADD_P384` body between digitisation and the post-processing; the three `SUBGOAL_THEN`s,
the `MATCH_MP_TAC FLAG_FROM_CARRY_LE THEN EXISTS_TAC \`384\``, the `SPEC
\`word_neg(word(bitval(p_384 <= m + n)))\`` and the `BOOL_CASES_TAC \`carry_s12\`` word
arithmetic are all gone. In the generated answer:

```
FLAG_FROM_CARRY_LE                            0
word_neg(word(bitval(p_384 <= m + n)))        0
2 EXP 384 * bitval carry_s12                  0
```

and the three derived calls are present at lines 2119/2121/2123 of the answer.

## 3. Fresh-process check of the lemmas

Command (from `.bench/s2n-bignum`):

```
cat research/receipts/bridge/check_fresh_d9.ml | ../hol-light/hol.sh
```

`research/receipts/bridge/check_fresh_d9.summary.txt` (verbatim excerpts; the full log was
truncated to these lines to keep the receipt small):

```
AXIOMS before=3 after=3
HAVE … 21 lemmas …, all hyps=0
BRIDGE_CHECK_DONE seconds=3.096
```

## 4. Judge

```
python3 combine-answer-and-setup.py d9-answers 1      # syntactic check PASSED → eval-20261123_214053
bash r3-run.sh eval-20261123_214053                   # RUN_WALL_SECONDS=100
python3 collect-verdicts.py eval-20261123_214053 d9-verdicts.csv
```

```
PROFILE arm.bignum_add_p384.BIGNUM_ADD_P384_CORRECT 61.225 OK
Problem,Verdict
arm.bignum_add_p384.BIGNUM_ADD_P384_CORRECT,OK
--,
SUMMARY,
OK,1
```

- Verdict: **OK**, printed by `collect-verdicts.py`.
- `.judge.txt`: `.bench/eval-20261123_214053/arm.bignum_add_p384.BIGNUM_ADD_P384_CORRECT.judge.txt`
- End-to-end: **100 s** wall for the judge run (compile + run); **61.225 s** measure inside
  `bench_run`. (Interpreted local closes: 339.852 s with carry+flag+mask; 268.498 s with the
  carry fact only; 274.964 s with no facts. The judge is native, hence the smaller number.)

Replay with the final answer (`r3_replay.py`, diagnostic only):

```
REPLAY arm.bignum_add_p384.BIGNUM_ADD_P384_CORRECT closed=1 hyps=0
       axioms_before=2445 axioms_after=2445 unchanged=true seconds=61.191
```

## 5. Curve-constant check

On the new generic section of `tools/bridge.ml` (§9, lines 1091–1510):

```
p_ names (p_256|p_384|p_521|p_25519|p_256k1|p_sm2):  0
p_384 decimal literal:                                0
```

## 6. Remaining `DEVELOPMENT_ONLY`

Still hand-chosen / transcribed, and left labelled:

1. **Simulation call schedule.** The splits `(1--13)`, `(14--22)`, `[23;24]`, `(25--38)` are the
   routine's instruction grouping, chosen by hand. The *facts* are derived from whatever those
   steps emit, but the grouping is not derived from the disassembly.
2. **Expansion idiom.** `BIGNUM_EXPAND_CONV` / `BIGNUM_LEXPAND_CONV` with the `LAND_CONV`/
   `RAND_CONV` traversal, `DISCARD_STATE_TAC`, and the `RULE_ASSUM_TAC (CONV_RULE
   (ONCE_DEPTH_CONV BIGNUM_OF_WORDLIST_CONV))` pre-pass are the human expansion idiom.
3. **Post-processing bitval rewrites.** `RULE_ASSUM_TAC(REWRITE_RULE [REAL_BITVAL_NOT;
   ADD_CLAUSES; VAL_WORD_BITVAL])` is copied from the human add closes.
4. **Mask condition shape.** `BRIDGE_MASK_SELECT_TAC` recognises the two-bit CSETM condition
   (`word_add (word (0 + bitval a)) (word (bitval b)) = 0`). It is mechanical over the bit
   names and state but does not decode the CSETM/ADCS instructions itself.
5. `tools/csat_bridge.ml`'s `sub_middle` (`FLAG_FROM_CARRY_LT`) is unchanged; this task covered
   the P-384 add facts only.

## 7a. Reuse on another curve (P-256)

`tools/hol_tests/p256_carry_reuse.ml` runs the same `BRIDGE_CARRY_CONSERVE_TAC` on the P-256
addition chain (4 limbs, `sum_s3;sum_s4;sum_s7;sum_s8`, `carry_s8`). It derives (verbatim from
`.bench/bridge-dev/p256_carry_reuse.log`):

```
bignum_of_wordlist [m_0; m_1; m_2; m_3] +
bignum_of_wordlist [n_0; n_1; n_2; n_3] =
bignum_of_wordlist [sum_s3; sum_s4; sum_s7; sum_s8] +
2 EXP (64 * SUC (SUC (SUC (SUC 0)))) * bitval carry_s8
```

No P-256 constant appears in the lemma; only the limbs and carries the simulator emitted are
used. P-521's addition is a 9-limb plain add with the same per-limb shape, so the list lemma
applies unchanged, but its `add` task is still failing in the bridge for the separate modular
correction and was not rewired here.

## 7. What is still not generic / could be wrong

- `BRIDGE_CMP_FLAG_TAC` assumes the compare chain's final out-flag is a syntactic negation
  (`~carry`), which is the `ADCS XZR, x, XZR` all-ones-limb shape. A final *positive* carry
  flag would need the dual lemma; the tactic fails closed rather than guessing.
- The tactical chain extraction follows carries and refuses on ambiguity. It does not prove
  that the limb list it finds is the only one; it selects the limb words that survive in
  registers and follows the outgoing carries through them.
- The mask fact is stated over `carry_s12 \/ (p_384 <= bignum …)`; the final
  `p_384 <= m + n` form is not emitted as a separate theorem. The bridge's SAT instance uses
  the derived carry telescope to relate `bignum m + bignum n` to that term. That relation is
  kernel-checked only through the certified SAT refutation, not as a standalone lemma.
- P-256 was not rewired: its add route had no transcribed carry fact to remove. P-521 add is
  still failing (pre-existing; out of scope here).

---

## Addendum (Slice D): ported to Rust

Ported to Rust in Slice D on `staging` (base commit `b28016c`).
`research/receipts/bridge/make_inlined_answer.py` is deleted. Reproduction of the
`add-p384` tactic is now
`cargo run --release --bin inline_answer -- <out-answer.txt> add-p384`; the output is
byte-identical to the Python original, sha256
`83d90067847d411d3eee7fb7b6ca6b532d790ab262341e6ba5d248d54db15044`. See
`research/receipts/slice-d-rust-receipts.md`. Historical text above is untouched.
