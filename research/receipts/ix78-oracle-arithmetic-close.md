# IX.78 — the oracle-guided arithmetic close: built, sound, still incomplete

- **Date:** 2026-11-25
- **Branch:** `staging`
- **Change under test:** `tools/bridge.ml` §6c (`BRIDGE_ADD_REF_ALG_TAC` → the new
  `BRIDGE_ALGEBRA_TAC`, which runs `BRIDGE_ORACLE_TAC`), and the phase marker anchor in
  `src/tools/emit.rs`. The gate is unchanged: `TEKMERION_ALG_CLOSE=<task id>`.
- **Verdict: it still does not prove `arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT`.**
  The oracle is built, it is driven per theorem, it emits the correct fact
  (`&(bitval carry_s8) = &0`) and it invokes **no solver**. The residual is no longer a stack
  overflow but a small, verbatim linear-system failure (`linear_ineqs: no contradiction`): one
  chain relation is missing. §6 names the missing mechanism. **No inference total exists.**
- **A regression that blocked the whole tree was fixed first (§1).** On `staging` before this
  change, *no* emitted answer parsed at all.

---

## 1. Prerequisite: the emitted answer did not parse

`XVI.D`'s failure classifier (`dec8480`) returned polymorphic variants (`` `Resource (…) ``).
The emitted answer is preprocessed by HOL Light's **camlp5r**, where a backtick starts a term
quotation, so the block was a parse error and *every* task failed to build. This was found by
building, not by reading. The fix is the representation `wip/xvi-f2-carry-range` used, adapted to
the allow-list classifier that is actually on `staging`:

- `bridge_failure_class` returns a portable `(kind, detail, limit)` **string tuple**;
- `bridge_slice_retry` matches the tuple;
- `bridge_slice_note` is passed as `~note:(fun s -> bridge_slice_note "%s" s)` (its format type
  does not unify with `string -> unit`).

Verified by `make`ing an answer (it is the only reason any proof could run at all). The
standalone block is still compiled by plain OCaml in `tests/bridge_failure_selftest.sh`, which
passes.

## 2. What the oracle does (and why each step is forced)

The residue after `BRIDGE_ALG_NORM_TAC` / `BRIDGE_SPLIT_CONDITIONS_KEEP_TAC` is a set of integer
equations and bounds over atoms `&(val w)` and `&(bitval b)`, plus the branch condition. The
IX.70 receipt established that `INT_ARITH`/`REAL_ARITH` is a **rational** linear prover
(`realarith.ml`, `linear_ineqs`), so a carry bounded by `0 <= c <= 1` is not integral to it.
The human proof (`s2n-bignum/.bench/human-baseline/arm/proofs/bignum_add_p25519.ml`) does three
things this section reconstructs:

1. **Linearise exactly.** Each integer hypothesis becomes a sparse rational form
   `Σ k_i·a_i + c = 0` or `>= 0`/`> 0`. Opaque terms (`&(val …)`, `&(bitval …)`) are atoms;
   anything non-linear is dropped (`bridge_lf_of_term`).
2. **Eliminate what the goal does not mention.** `bridge_gauss_jordan` eliminates the *carry*
   atoms (the only internal atoms of a chain) by exact Gauss–Jordan. Elimination is
   `k2·row2 − k1·row1`, so **integer coefficients are preserved** — no division is introduced.
   What remains is one *residual* relation among the goal's own atoms and the *definitions* of
   the carries. For this theorem: `defs=7 residual=1`.
3. **Decide the carries by the linear relaxation, exactly.** `bridge_carry_value` computes, for
   each carry definition `k·c + rest = 0`, a rational interval for `rest` — from one-atom bounds
   and **one relational hypothesis used per bound** (`bridge_rel_hi`): that is what `M + N < 2^256`
   is. It *proposes* `c = 0` only when the interval suggests it. The interval engine is a
   heuristic: it can be over-tight (it proposed `carry_s3 = 0`, which is false), so a proposal is
   never trusted on its own.
4. **Prove the proposed fact, and let the procedure decide.** `bridge_try_value` proves `c = &k`
   from the definition, the hypotheses whose atoms all occur in it (the *cone*), and the integer
   discreteness that `INT_ARITH` encodes. A proposal the procedure cannot prove is reported
   `undetermined` and **not** emitted — that is the soundness-critical step, and it is what makes
   the wrong `carry_s3` proposal harmless.
5. **Flatten and close.** `bridge_flat_conv` rewrites every integer subterm to the canonical flat
   form `B^3·a + B^2·b + B·c + d` (each rewrite proved by `INT_RING`). This is load-bearing: the
   same 59-hypothesis system that closes in **0.205 s** in flat form exhausted 12 GB in the nested
   `B*(B*(B*a+b)+c)+d` form (`probe3.ml`, `A_flat_val` 0.235 s vs `A_nested` cap fired).

Every emitted fact is derived from the machine's own equations and its `bitval` semantics; none
is asserted, and no hypothesis is added to the theorem.

## 3. The oracle's decisions per close

From the run (`.bench/ix78/final-alg.log`, `--mode phase`), for the first bridge branch:

```
ORACLE undetermined &(bitval carry_s3) why=not-proved-from-cone
ORACLE undetermined &(bitval carry_s4) why=not-proved-from-cone
ORACLE undetermined &(bitval carry_s7) why=not-proved-from-cone
ORACLE fact &(bitval carry_s8) = 0 why=relaxation
ORACLE undetermined &(bitval carry_s10) why=relaxation
ORACLE undetermined &(bitval carry_s11) why=relaxation
ORACLE undetermined &(bitval carry_s12) why=relaxation
ORACLE system defs=7 residual=1 facts=1
```

**Emitted facts (one line each):**

```
&(bitval carry_s8) = &0
```

That is the right fact: the top carry out of the raw add is 0 because `M + N < 2^256`, and with
it the branch residual is decided in `A_zero` (0.205 s) instead of exhausting memory (`A_all`).
The other six carries are correctly *not* pinned: `carry_s3/4/7` may be 1, and
`carry_s10/11/12` are the corrected chain's carries.

## 4. Measurements

`emit --mode phase`, `tools/cap.sh 12G -- make -j1`, `.bench/hol-light-stamp2`.

| route | total primitive inferences | SAT closes | wall |
|---|---:|---:|---:|
| SAT, full hypothesis set (IX.70 baseline) | **119,194,788** | 4 | 55 s |
| SAT, IX.69 slice (shipped default) | **111,501,987** | 4 | 49 s |
| human `_CORRECT` body (IX.65, cited) | **1,848,494** | — | — |
| oracle close (opt-in) | **none — fails** | **0** | 0.65 s |

Algebraic run, from the phase marks: pre-theorem `67,646,907`; `bridge:algebra` `70,151,488`;
`bridge:algebra-arith` `70,260,806`. So **2,613,899 inferences** are spent between entering the
algebraic route and the residual, and then it stops. `bitblast`, `defn_clausify`, `replay` are
all zero: no SAT close is entered. There is no `STAMP_INFERENCES` line.

The IX.70 figure **2,620,473** in the earlier receipt is a *cost of an abort*, not a result; it is
not cited here as a cost of a proof. It is not a cost at all.

## 5. The residual, verbatim

```
ANSWER_EXCEPTION: Failure("INT_ARITH `(((((&(val sum_s11) + -- &1 * &(val sum_s4)) + -- &1 * &(bitval carry_s10)) +
   &18446744073709551616 * &(val sum_s12)) +
  -- &18446744073709551616 * &(val sum_s7)) +
 &340282366920938463463374607431768211456 * &(val sum_s13)) +
-- &340282366920938463463374607431768211456 * &(val sum_s8) =
&3138550867693340381917894711603833208051177722232017256448
==> (((-- &1 * &(val sum_s12) + &(val sum_s7)) + &(bitval carry_s11)) +
     -- &18446744073709551616 * &(val sum_s13)) +
    &18446744073709551616 * &(val sum_s8) =
    -- &170141183460469231731687303715884105728
==> (&(val sum_s13) + -- &1 * &(val sum_s8)) + -- &1 * &(bitval carry_s12) =
    &9223372036854775808
==> ((((((-- &1 * &(val sum_s10) + &(val sum_s3)) +
         -- &18446744073709551616 * &(val sum_s11)) +
        &18446744073709551616 * &(val sum_s4)) +
       -- &340282366920938463463374607431768211456 * &(val sum_s12)) +
      &340282366920938463463374607431768211456 * &(val sum_s7)) +
     -- &6277101735386680763835789423207666416102355444464034512896 *
     &(val sum_s13)) +
    &6277101735386680763835789423207666416102355444464034512896 *
    &(val sum_s8) =
    -- &57896044618658097711785492504343953926634992332820282019728792003956564819987
==> &(bitval carry_s8) = &0
==> &57896044618658097711785492504343953926634992332820282019728792003956564819949 <=
    (… M + N, flat …)
==> (… M + N, flat …) <
    &115792089237316195423570985008687907853269984665640564039457584007913129639936
==> (… n = &(val n_0) + B·&(val n_1) + …, flat …) < p
==> (… m, flat …) < p
==> (… M + N − p = W_raw, flat …)`: linear_ineqs: no contradiction")
```

The `(…)` elisions are only the flat spellings of the same terms; the full text is in
`.bench/ix78/final-alg.log` and `.bench/ix78/c4-alg/arm_add_p25519.correct`. The antecedents are:
the three corrected-chain relations, the single residual aggregate, `&(bitval carry_s8) = &0`,
and the four relational hypotheses.

## 6. Diagnosis: exactly what is missing

The residual contains the **corrected** chain's relations and the emitted carry fact, but it does
**not** contain the **raw** chain's aggregate

```
M + N = W_raw + 2^256 · &(bitval carry_s8).
```

That relation is the one the definition of `carry_s8` encodes, and it is what, together with
`c8 = 0`, gives `M + N = W_raw`; the corrected chain gives `W_cor = W_raw + (2^256 − p)`, and the
contradiction then costs one subtraction. Without it the residual is genuinely not a consequence
of the given antecedents — the failure is semantic, not a search failure.

The missing mechanism is therefore **the aggregate/decarray step of the human proof**:

- `GEN_DECARRY_RULE` (`.bench/human-baseline/common/interval.ml:614`) composes the accumulator
  equations into exactly these aggregates and, using
  `BOUNDER_TAC`/`BOUNDER_RULE` (immediate-interval bounds, `REAL_INTEGER_TAC` for the integrality),
  simultaneously discharges each carry's range. The human proof gives it
  `FLAG_FROM_CARRY_LE` (the carry as a *flag over the specification's own comparison`) and then
  one `REAL_ARITH_TAC`.
- Here, Gauss–Jordan eliminates the carries but destroys the aggregate rows: a definition whose
  pivot is `carry_s8` has accumulated `2^256` coefficients, and handing eight such definitions to
  `INT_ARITH` drives its equality elimination into rational coefficient blow-up (measured: a
  12 GB cap firing, recorded in the work log, not retried). Supplying the *residual + small
  definitions* (the current close, 9 antecedents) avoids the blow-up but omits the raw aggregate.

So the remaining work is precise: emit **both** chain aggregates as relations over the goal's
atoms (the raw one is `M + N − W_raw − 2^256·c8 = 0`, which is a *projection* of `carry_s8`'s
definition, not the definition itself), plus the range fact, and close. That is
`XVII.B`/`XVI.F1–F3`, and it is a small, well-specified addition — not another call to
`INT_ARITH`.

## 7. Controls, verbatim

### D1 — the statement is the same obligation (weaker check, stated)

`equals_thm` needs two live theorems; the algebraic path produces none, so it cannot be run.
Emit the same answer with the switch off and on, assemble both, diff:

```
$ diff .bench/ix78/c1-sat.ml .bench/ix78/c1-alg.ml
3755c3755
< BRIDGE_ADD_REF_TAC (CONV_TAC (ONCE_DEPTH_CONV BIGNUM_OF_WORDLIST_CONV)) p_25519) g
---
> BRIDGE_ADD_REF_ALG_TAC (CONV_TAC (ONCE_DEPTH_CONV BIGNUM_OF_WORDLIST_CONV)) p_25519) g
```

The `time prove`, the frozen query and the postcondition are byte-identical; only the close
entry point differs. This is identity of the printed statement and goal term, **not**
`equals_thm`, and is weaker.

### D2 — axioms

The SAT run exits 0 (`check_axioms`) and is stamped with the three permitted axioms:

```
$ grep -E '^STAMP_NAME' .bench/ix78/sat/arm_add_p25519.correct
STAMP_NAME ETA_AX 1
STAMP_NAME SELECT_AX 2
STAMP_NAME INFINITY_AX 3
```

The algebraic path produces no theorem, so no stamp exists for it. It adds only definitions,
rewrites and proof-producing tactic calls over library lemmas.

### D3 — firing: the fact is load-bearing

The oracle reports `&(bitval carry_s8) = &0`; the close with it has **9** antecedents, without it
**8**, and the fact disappears from the residual:

```
$ # facts' replaced by [] in the assembled proof (scratch, re-assembled afterwards)
ORACLE close ants=8 size=11296
ORACLE close failed: Failure("INT_ARITH `(((((&(val sum_s11) + …`: linear_ineqs: no contradiction")
$ grep -c 'bitval carry_s8) = &0' .bench/ix78/d3-nofact.log   # in the antecedent list
0
```

The isolated system shows the same polarity in a decidable setting: the branch closes in
**0.205 s** with the fact (`A_zero`), the same system without it fails (`A_all`), and with only
the integrality *disjunction* it takes **38.4 s** (`A_disj`). Since the in-tree proof fails
anyway, the honest formulation is: the fact is necessary for the one branch that is
arithmetically decidable, and the in-tree evidence is the antecedent removed from the residual.

### D4 — no solver on this theorem

A failing `cadical` shim is placed before the toolchain; it appends to a marker and exits 1.

```
$ TEKMERION_ALG_CLOSE=arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT \
    bash .bench/ix70/run_task_shim.sh arm curve25519 add p25519 .bench/ix78/c4-alg
RUN arm_add_p25519 rc=2 phase_marks=15
$ wc -l .bench/ix78/c4-alg/cadical-invocations.txt
0
```

Zero invocations. The algebraic tactic never calls `BRIDGE_CSAT_TAC`; the run enters zero SAT
closes (no `bridge:prove-entry` mark). The same shim fires twice on the default SAT path
(IX.70 §4, control C4).

### D5 — no cheating

```
$ grep -nE 'new_axiom|mk_thm|CHEAT_TAC|Obj\.magic' tools/bridge.ml
1003:(* procedure only; no new axioms, no CHEAT_TAC), and the same linear system  *)
```

One hit, a comment. No hypothesis is added to the statement (D1).

### D6 — the default path is unchanged

The switch is unset by default, so every task emits exactly the tactic it had. Re-measured today:

```
arm.bignum_add_p25519  STAMP_INFERENCES 111501987   (IX.69: 111,501,987)
arm.bignum_neg_p25519  STAMP_INFERENCES  33888658   (IX.69:  33,888,658)
arm.bignum_add_p256    STAMP_INFERENCES 118896814   (IX.69: 118,896,814)
x86.bignum_neg_sm2     STAMP_INFERENCES  15444067   (IX.69:  15,444,067)
```

### D7 — generality: what is and is not generic

**Generic, computed per theorem:**
- the lineariser (any integer add/sub/numeral-multiple term);
- the elimination target (atoms *not* occurring in the goal — read from this goal);
- the interval relaxation (exact rationals, one-atom bounds plus one relational hypothesis);
- the fact proof (the definition's own cone);
- the flat normalisation (structural, proved by `INT_RING`);
- the close (residual rows + emitted facts + relational hypotheses).

Nothing names `p25519`, `64`, `4`, a limb count or an instruction. The oracle is driven by *this*
theorem's equations and bounds: it decides `carry_s8 = 0` and reports the other six carries
undetermined. It is not a per-class template.

**Not generic / missing (the honest list):**
1. **The second aggregate.** Only *one* residual row survives elimination; the raw aggregate is
   destroyed by the `carry_s8` definition. The fix is to project the definition onto the goal's
   atoms (or to eliminate carries per chain), not to add a template.
2. **The interval engine is a filter, not a certificate, and it errs.** It combines exact
   one-atom bounds with one relational hypothesis; that one-shot decomposition can be over-tight
   (observed: it proposed `carry_s3 = 0`, which is false — the note is
   `why=not-proved-from-cone`). This is *tolerable* only because every proposed value is re-proved
   by `INT_ARITH` from the definition and its cone: a wrong proposal simply fails to prove and is
   not emitted. It is nevertheless a soundness-critical order: the procedure, not the interval
   arithmetic, decides. A `BOUNDER`-style engine that returns a *certificate* would remove the
   gap and the second call.
3. **The close set is a heuristic.** It excludes carry definitions with accumulated `B^n`
   coefficients to avoid a measured blow-up. A principled close is the aggregate-level one.
4. **One theorem, one architecture.** Nothing here has been run on `sub`/`neg`, on x86, or on the
   nine additions.

## 8. Environment

`free -g` before the runs: `total 125, used 33, free 84 (available 92)`; after: `used 38, free 77,
available 86`. `df -h /mnt/1tb_kingston`: `1.3T 901G 360G 72%`. Every proof run used
`tools/cap.sh 12G -- make -j1`, one target at a time. The one cap that fired during the work was
the nested-form probe (§2 item 5); it is recorded as a result and was not retried.

## 9. Commands (verbatim)

```
$ free -g | head -2
               total        used        free      shared  buff/cache   available
Mem:             125          38          77           0           8          86
$ df -h /mnt/1tb_kingston | tail -1
/dev/nvme1n1p1  1.3T  901G  360G  72% /mnt/1tb_kingston

$ bash .bench/ix69/run_task.sh arm curve25519 add p25519 .bench/ix78/sat
STAMP_INF_BEFORE 67646907
STAMP_INFERENCES 111501987
Running time: 49.000000 sec

$ TEKMERION_ALG_CLOSE=arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT \
    ./target/release/emit --task … --mode phase --out .bench/ix78/run.answer …
$ ./target/release/dropin assemble --task … --answer .bench/ix78/run.answer --stamp
$ make -j1 -B HOLDIR=.bench/hol-light-stamp2 curve25519/bignum_add_p25519.native
$ tools/cap.sh 12G -- ./arm/curve25519/bignum_add_p25519.native
PHASE_MARK bridge:algebra 70151488
PHASE_MARK bridge:algebra-arith 70260806
ORACLE system defs=7 residual=1 facts=1
ORACLE close ants=9 size=11309
ORACLE close failed: Failure("INT_ARITH `…`: linear_ineqs: no contradiction")
Failed after (user) CPU time of 0.646108: …

$ tools/cap.sh 20G -- cargo test --offline
… all targets pass
$ cargo clippy --offline --all-targets -- -D warnings
$ cargo fmt --check
$ git ls-files '*.py'     # empty
$ git status --short
```

## 10. What is not established

- **No theorem, no inference total, no phase split for the algebraic route.**
- **D2/D3 for the algebraic route**: no theorem, so no axiom stamp and no success to weaken; D3
  is demonstrated by the antecedent change and by the isolated probe, not by a working proof.
- **The missing aggregate is a diagnosis, not an implemented step.** The next attempt should
  start from `carry_s8`'s definition projected onto the goal's atoms, not from a larger
  `INT_ARITH` call.
- `tools/bridge.ml` and `src/tools/emit.rs` are in the frozen source closure; the goldens/fixtures
  were re-recorded deliberately (§11) and `freeze-closure-v6.json` was regenerated; **no tag was
  created**.
- All numbers are from this host, this kernel (`.bench/hol-light-stamp2`) and this commit.

## 11. Fixtures and goldens: what moved, and why

`tools/bridge.ml` changed (§1–§2), so every inlined answer changed. Re-recorded deliberately with
`TEKMERION_RECORD_ANSWERS=1 cargo test --offline --test tools_emit` (27 files under
`tests/fixtures/answers/`).

`tests/tools_receipt.rs` pins the library text; the values that moved, and are updated in the same
commit, are:

| pin | was | now |
|---|---|---|
| `BRIDGE_DIGEST` | `ace0b868…` (XVII.A5) | `c3a1e199a481e17daa1fe03f79c48f4e3239e1e5b1444b941337b6f722a8385c` |
| `curve_check` statements | 76 | 33 |
| `instrument` phrases | 160 | 217 |
| `inline_answer` phrases | 204 | 261 |
| inline `add-ref` | `4363d594…` | `c26c0080c3db52e5f62ba0aa4175d74ef461d50f4de9e575f5daa5923925f1f7` |
| inline `sub` | `eecd7548…` | `f600e09a4dd9be44cc935ce1386e4613436816cd403a73aba89a746514d01546` |
| inline `neg` | `ee40990a…` | `821f71d16ad4c8b0620804f5451ffd188a9a5671ed94bf60215f2ecdc8add5ac` |
| inline `add-p384` | `82a52246…` | `91083c8163cca271005806aea20d9a7147adbbf247db33f39c93994412a3c640` |
| `curve_check` report fixture | — | regenerated (`tests/fixtures/curve_check_bridge.txt`) |

The `curve_check` statement count *fell* (76 → 33) because its scanner pairs each `prove` with the
next two backticks, and the new helper functions use **more** backtick quotations between
`prove`s, so its pairing shifts; `hits = 0` is unchanged, which is the property that matters (no
curve name or 5-digit numeral in a lemma statement). The `tests/fixtures/emit/bridge_min.ml`
anchor was updated from `at "arith" ASM_INT_ARITH_TAC` to `at "oracle" BRIDGE_ORACLE_TAC`.
