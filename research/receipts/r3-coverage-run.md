# R3 coverage run — ARM `add`/`sub`/`neg`, 18 declared tasks (V8.19)

Run date: **2026-11-23** (UTC), branch `staging`, base commit
`672cc0b45270f262de9bf76773e2fa8d0c0f9304`. Nothing was committed or pushed.
Freeze receipt: `research/receipts/r3-freeze-manifest.md`.

## 1. Declared set and headline

Declared before generation from `.bench/problems.json`: every
`functional_correctness_arm` task whose id matches
`arm.bignum_(add|sub|neg)_<family>.` — **N = 18** (6 curves × 3 operations).
Taskset digest `54de4367fd72ddce0618ea9c10499e9be91901fc7943f4088b0b1bee3ccb2cab`.
No task was added or removed after this declaration.

| | count |
|---|---:|
| declared | **18** |
| answer emitted | 17 |
| judged | 17 |
| **OK** | **10** |
| FAIL | 7 |
| no answer emitted | 1 |

- **Overall: `OK 10 / 18` declared** (10 / 17 of the tasks that were emitted and
  judged; one task could not be judged because no answer was produced).
- `collect-verdicts.py` printed exactly `OK,10` and `FAIL,7`
  (`research/receipts/r3-verdicts.csv`, console copy `r3-collect.log`). No
  verdict below is claimed unless it appears in that CSV.

**Denominator scope.** "Across all curves" is read as the per-curve bignum
routines. Two further ARM FC tasks carry an `add`/`sub` token but no curve
family and a variable-length signature: `arm.bignum_add.BIGNUM_ADD_CORRECT`
and `arm.bignum_sub.BIGNUM_SUB_CORRECT` (generic `val p`-limb routines,
`C_RETURN` carry). They are outside this declared set and were not run. This
report's denominator is 18, not 20, and not the 898-task FC set.

## 2. Freeze verified before and after

The ten pipeline files were hashed at `2026-11-23T13:57:05Z` before any answer
was generated and re-hashed after the run. All ten individual hashes and the
combined digest match the freeze manifest:

```
combined pre/post: a6cc2a4dac8088b01b4503695ec74d94f556bf4ebeed02a16e009a6d70e2ba9f
tools/bridge.ml                    3a32e6b0e5e7f36fd8d49e9a79cac930668beca031416c6c9b992b227115d52f
tools/csat_bridge.ml               8c1dc799f508d3e4f2df5e0b549ec2cd9f36bbda6e85f723464596ee31e2e726
tools/csat_lib.ml                  70439c92d58046a065312fc4faad9e06d53ddd64f89f8e93f676b815cc83e5a8
tools/emit_proof.py                92c4f8964bc1477ecb4cec54e863e4205a092a679769223bf0d3f0b27eabfd65
tools/derive_steps.py              894ef23f25bc64769b5471b242d9ae6e5d8b9b723e988ed2e24566335baf9ee8
tools/disasm_steps.py              fe8c9d2502e2d033868e850178153f5f4892158ff4b7d99d4be8651dcf78ed50
research/…/make_inlined_answer.py  9093b086998748cbc76cade2b71cb4ae66a424b3364b4ca18398ec177b65e6ae
tools/w2_witnesses.py              4b1e9c605fe1d79a16fc283d2398f8c7db32f37059e578987bd755a659413d03
tools/w4_close.py                  71eeaa353452345cf1a56f973bf66d6edea58e64bfa71b64c1c9eba2d54d9cc6
tools/witness.py                   156ac91e615f133237a56a080d6f4c7e281fefd15bac6000e9a1bf8b2b5f6f8b
```

The three inlined HOL Light sources (`Cadical/{dimacs,cnf,ldrup}.ml`) also
re-hash unchanged. `git status` shows no modified tracked file; the only new
tracked-directory files are this report and its logs.

## 3. Dispatch (declared before the run)

| route | generator | tasks |
|---|---|---|
| C-SAT bridge | `make_inlined_answer.py … add-ref` | `add_p256`, `add_p384` |
| C-SAT bridge | `make_inlined_answer.py … sub` | `sub_p256` |
| C-SAT bridge | `make_inlined_answer.py … neg` | `neg_p256` |
| witness | `tools/emit_proof.py …` | the other 14 |

`--allow-heldout` was passed only for the six `p25519`/`p256k1`/`sm2` tasks.

## 4. Per-task verdicts

Judge directory: `.bench/eval-20261123_135737`. "seconds" is the harness
`PROFILE` elapsed (`bench_run`, i.e. from entering `prove` to verdict).
"hyps" is measured by the diagnostic replay in §6, not assumed.

| task | route | emitted | verdict | seconds | hyps |
|---|---|---|---|---:|---:|
| `arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT` | witness | yes | FAIL | 0.380 | n/a |
| `arm.bignum_add_p256.BIGNUM_ADD_P256_CORRECT` | bridge | yes | **OK** | 22.785 | 0 |
| `arm.bignum_add_p256k1.BIGNUM_ADD_P256K1_CORRECT` | witness | yes | FAIL | 0.459 | n/a |
| `arm.bignum_add_p384.BIGNUM_ADD_P384_CORRECT` | bridge | yes | **OK** | 25.656 | 0 |
| `arm.bignum_add_p521.BIGNUM_ADD_P521_CORRECT` | witness | yes | FAIL | 1.786 | n/a |
| `arm.bignum_add_sm2.BIGNUM_ADD_SM2_CORRECT` | witness | yes | **OK** | 0.488 | 0 |
| `arm.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT` | witness | yes | FAIL | 0.181 | n/a |
| `arm.bignum_neg_p256.BIGNUM_NEG_P256_CORRECT` | bridge | yes | **OK** | 9.171 | 0 |
| `arm.bignum_neg_p256k1.BIGNUM_NEG_P256K1_CORRECT` | witness | yes | **OK** | 0.201 | 0 |
| `arm.bignum_neg_p384.BIGNUM_NEG_P384_CORRECT` | witness | yes | **OK** | 0.339 | 0 |
| `arm.bignum_neg_p521.BIGNUM_NEG_P521_CORRECT` | witness | **no** | **no answer emitted** | — | — |
| `arm.bignum_neg_sm2.BIGNUM_NEG_SM2_CORRECT` | witness | yes | **OK** | 0.194 | 0 |
| `arm.bignum_sub_p25519.BIGNUM_SUB_P25519_CORRECT` | witness | yes | FAIL | 0.291 | n/a |
| `arm.bignum_sub_p256.BIGNUM_SUB_P256_CORRECT` | bridge | yes | **OK** | 18.431 | 0 |
| `arm.bignum_sub_p256k1.BIGNUM_SUB_P256K1_CORRECT` | witness | yes | FAIL | 0.292 | n/a |
| `arm.bignum_sub_p384.BIGNUM_SUB_P384_CORRECT` | witness | yes | **OK** | 0.616 | 0 |
| `arm.bignum_sub_p521.BIGNUM_SUB_P521_CORRECT` | witness | yes | FAIL | 4.164 | n/a |
| `arm.bignum_sub_sm2.BIGNUM_SUB_SM2_CORRECT` | witness | yes | **OK** | 0.317 | 0 |

Machine-readable: `research/receipts/r3-results.tsv`.

### Per-curve

| curve | add | sub | neg | OK/N |
|---|---|---|---|---:|
| P-25519 (`p25519`) | FAIL | FAIL | FAIL | **0/3** |
| P-256 (`p256`) | OK | OK | OK | **3/3** |
| secp256k1 (`p256k1`) | FAIL | FAIL | OK | **1/3** |
| P-384 (`p384`) | OK | OK | OK | **3/3** |
| P-521 (`p521`) | FAIL | FAIL | no answer | **0/3** (0/2 judged) |
| SM2 (`sm2`) | OK | OK | OK | **3/3** |
| **total** | | | | **10/18** |

P-256 and P-384 are closed on all three operations, but P-256 uses the bridge
for all three and P-384 only for `add`; P-384 `sub`/`neg` are witness-route.

## 5. Every failure, with the exact failing tactic

All diagnoses below are from the diagnostic replay and step logger in §6
(run on the same answer bytes, same judge setup). No answer was edited or
re-run after the verdict.

| task | failing tactic | exception | generated subgoal |
|---|---|---|---|
| `add_p25519` | `BOUNDER_TAC[]` | `Failure("NO_TAC")` | `carry_s13 <=> (m + n) < p_25519` (via `FLAG_FROM_CARRY_LT`, `EXISTS_TAC \`320\``) |
| `add_p256k1` | `BOUNDER_TAC[]` | `Failure("NO_TAC")` | `carry_s20 <=> (m + n) < p_256k1` (`EXISTS_TAC \`320\``) |
| `add_p521` | `BOUNDER_TAC[]` | `Failure("NO_TAC")` | `carry_s31 <=> (m + n) < p_521` (`EXISTS_TAC \`640\``) |
| `sub_p25519` | `BOUNDER_TAC[]` | `Failure("NO_TAC")` | `carry_s14 <=> m < n` (`EXISTS_TAC \`256\``) |
| `sub_p256k1` | `BOUNDER_TAC[]` | `Failure("NO_TAC")` | `carry_s15 <=> m < n` (`EXISTS_TAC \`256\``) |
| `sub_p521` | `BOUNDER_TAC[]` | `Failure("NO_TAC")` | `carry_s27 <=> m < n` (`EXISTS_TAC \`576\``) |
| `neg_p25519` | `REAL_INTEGER_TAC` | `Failure("ACCEPT_TAC")` | `integer (… / &2 pow 256 + &0)` with the `2^255 − 19` numeral in the numerator |

`NO_TAC` is `BOUNDER_TAC`'s failure path: `common/interval.ml` ends its chain
with `… THEN CONV_TAC REAL_RAT_REDUCE_CONV THEN NO_TAC`, so it raises exactly
when the arithmetic bound is not closed. The step logger confirms the last
successful top-level step and that the next one is the `THENL` whose first
branch ends in `BOUNDER_TAC[]` (add/sub) or the final `REAL_INTEGER_TAC` (neg).

**Cause (same for all six add/sub failures):** the derived flag meaning is
`(m + n) < modulus` (add) or `m < n` (sub). That is the P-256 routine's single
carry-select shape. The `p25519`, `p256k1` and `p521` routines fold a
special-prime correction or a mask-driven conditional subtraction into the
carry chain, so the decisive carry flag is not that comparison; the real
inequality left for `BOUNDER_TAC` is not provable.

**Cause (`neg_p25519`):** the simulation and case split are produced, but the
final `integer` side condition still contains the `2^255 − 19` reduction and
the quotient by `&2 pow 256`; `REAL_INTEGER_TAC` cannot discharge it and raises
`ACCEPT_TAC`.

## 6. No answer emitted

`arm.bignum_neg_p521.BIGNUM_NEG_P521_CORRECT` — `tools/emit_proof.py` exited
rc 1 before writing an answer:

```
ValueError: derive_steps found no accumulating steps
  (tools/w2_witnesses.py:248)
```

The P-521 `neg` routine has no add/sub carry chain (it is a complement with a
masked top word), so the witness generator has nothing to derive. This is a
generator-coverage limit, **not** a judge `FAIL`; it is excluded from the
judged denominator and reported separately.

## 7. Hypotheses and axiom delta

The judge's `prove` rejects any theorem with additional assumptions
(`hol_lib_inlined.ml`: `if asl <> [] then failwith "prove: additional
assumptions in result"`), so an `OK` verdict already implies zero hypotheses.
The diagnostic replay measured it directly for all ten `OK` tasks:

```
REPLAY … closed=1 hyps=0 axioms_before=N axioms_after=N unchanged=true
```

Axioms were `2445 → 2445` for every task except `sub_p256`, where the
`p256_montjdouble` template loads a larger context: `6519 → 6519`. In every
case `axioms_before = axioms_after`; the harness would have written `CHEATING`
otherwise, and it wrote `OK`. No failure introduced an axiom either (all
failures leave `2445 → 2445`).

Full replay lines: `research/receipts/r3-replay.log` and
`.bench/r3-replay-all.log`.

## 8. Environment and resources

- `cadical` **was present**: `/home/one/.elan/toolchains/leanprover--lean4---v4.29.1/bin/cadical`,
  version `2.1.2`, exported on `PATH` for the run. It is not on the default
  `PATH`. No exit-127 environment failure occurred.
- One core for `combine-answer-and-setup.py` (Python 3.14.10). All 17 emitted
  answers passed the `ocamlc -i` syntactic gate; none was dropped as a compile
  error.
- Whole judged execution wall time: **733 s** (compiling and running 17
  programs, one at a time). Per-task `PROFILE` times are in §4; the maximum is
  **25.656 s** against a 120 s per-task timeout, so nothing timed out.
- Host at run time: 16 cores, ~67 GB RAM free. Peak RAM was not instrumented;
  no OOM or swap exhaustion occurred.
- Disk: 27 GB free before, 26 GB free after (the eval directory, replay copies
  and logs total ~1.2 GB). Stayed above the 3 GB floor.
- No network. `.bench/s2n-bignum` sources, `objfiles/` contents and
  `toplevel-thms/` were not modified (the harness runs with `objfiles/` as the
  working directory; it writes only into the eval and replay directories).

## 9. Files created

Frozen/receipts (tracked directory):
`research/receipts/r3-freeze-manifest.md`, `r3-coverage-run.md` (this file),
`r3-verdicts.csv`, `r3-results.tsv`, `r3-profile.txt`, `r3-generate.log`,
`r3-combine.log`, `r3-run.log`, `r3-collect.log`, `r3-replay.log`,
`r3-logged.log`.

Working artefacts (git-ignored under `.bench/`): `r3-answers/`,
`r3-receipts/`, `eval-20261123_135737/`, `r3-replay/`,
`r3-replay-log/`, `r3-generate.sh`, `r3-run.sh`, `r3_replay.py`,
`r3-replay-run.sh`, `r3_logged.py`, `r3-logged-run.sh`.

## 10. What this number supports — and what it does not

**This is a coverage run, not a blind held-out claim.** All 18 tasks had prior
visible outcomes in this programme:

- the nine R2 tasks — `p25519`, `p256k1`, `sm2` × `add`/`sub`/`neg`;
- the four tasks developed for the C-SAT bridge — `p256` `add`/`sub`/`neg`
  and `p384` `add`;
- the five R2 *training* tasks — `p384` `sub`/`neg` and `p521`
  `add`/`sub`/`neg`.

The nine R2 tasks are development-visible; so are the other nine. The only
task with a previously unjudged outcome is `neg_p521`, and it again emitted no
answer. Nothing here is held out.

What it supports:

- With the frozen pipeline at `672cc0b`, one evaluation accepted **10 of the 18**
  declared ARM `add`/`sub`/`neg` tasks: all of P-256, P-384 (add via the
  bridge; sub/neg via the witness closer) and SM2, plus `neg_p256k1`.
- The C-SAT bridge closes `p256` `add`/`sub`/`neg` and `p384` `add`; the
  witness closer closes `sm2` `add`/`sub`/`neg`, `neg_p256k1`, and `p384`
  `sub`/`neg`.
- The witness closer's failure boundary is the special-prime / mask-corrected
  routines (`p25519`, `p256k1`, `p521` add and sub) and `neg_p25519`.

What it does **not** support:

- A held-out or blind-generalisation claim, for the reason above.
- A rate on the 898-task functional-correctness set. **This report's
  denominator is 18 and must not be compared with 5/898 or 4/9.** Different
  denominators, task selection, system and budget.
- An axiom-free development. Benchmark `OK` is under the harness `CHEAT_TAC`
  context: the preceding lemmas are already available as theorems, and the
  absolute axiom count (2445, or 6519 for the `sub_p256` template) reflects
  that setup. The only invariant shown is that an answer adds no axiom.
- A statement that `10/18` is better or worse than any other system's number.

## 11. Labelled diagnostics (not part of the frozen pipeline)

To obtain measured hypotheses and exception messages, copies of the judge
programs were instrumented under `.bench/r3-replay*`:

- `r3_replay.py` replaces the harness `bench_run` with a prover that reports
  the hypothesis count on success and the propagating exception on failure;
- `r3_logged.py` splits each answer at depth-0 `THEN` and inserts
  `ALL_TAC`-with-print steps, so the last successful step and the next
  (failing) step are identifiable.

These are separate copies. They do not change the frozen files, the answers,
or any verdict, and they are not used to claim an `OK`. The failing-step labels
in §5 come from them.

---

## Addendum (Slice D): ported to Rust

Ported to Rust in Slice D on `staging` (base commit `b28016c`).
`research/receipts/bridge/make_inlined_answer.py` is deleted. Reproduction is now
`cargo run --release --bin inline_answer -- <out-answer.txt> <mode>`; all four modes are
byte-identical to the Python original (`add-ref` sha256
`39fd3d64c65063410c02f0a879d505632b4d82042ac3a64f26c137426e23264f`; the other three
are tabulated in `research/receipts/slice-d-rust-receipts.md`). Historical text above is
untouched.
