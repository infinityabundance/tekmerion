# IX.70 — the recovered algebraic close: sound, incomplete, does not prove

- **Date:** 2026-11-25
- **Branch:** `staging`
- **Recovered from:** `0336cb2c03dff08b21fdc448dee04f4ec8dc42ca` (`tools/bridge.ml`,
  `src/tools/emit.rs`), the IX.67 WIP that `a1546d8` reverted as unverified.
- **Change under test:** `BRIDGE_ALGEBRA_TAC` / `BRIDGE_ADD_REF_ALG_TAC` in `tools/bridge.ml`,
  and an **opt-in** emitter switch (`TEKMERION_ALG_CLOSE=<task id>`) in `src/tools/emit.rs`.
- **Verdict: it does not prove `arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT`.** Two focused
  attempts were made (§2). The residual is the same in both: HOL Light's `ASM_INT_ARITH_TAC`
  does not discharge the word-residue linear system — `Stack_overflow` on the first residual,
  after **0** SAT closes. The mathematics is sound and closes in isolation (§6); the blocker is
  the decision procedure/encoding, not a missing lemma. **No inference total exists.**
- **Not a hard gate.** The WIP gated the theorem to the algebraic entry point at emission. That
  would make the theorem unprovable in the shipped tree, so the recovered gate was changed to
  opt-in: unset, every task emits exactly the tactic it had (verified: the sliced SAT total
  reproduces, §3). The full reasoning for not blessing it is in §2 and `TODO.md` XV.D1.

---

## 1. Recovery and composition with IX.69

`git show 0336cb2 -- tools/bridge.ml src/tools/emit.rs` was applied by hand. The two changes do
not overlap: IX.69 lives inside `BRIDGE_CSAT_TAC` (the SAT path); the recovered algebraic close is
a separate tactic that **never calls `BRIDGE_CSAT_TAC`**. Composition therefore keeps the slice in
the SAT path and the algebraic tactic as a separate entry point, exactly as the brief asks. There
was no textual or semantic conflict.

`git show 0336cb2` also contains `AGENTS.md` and `tools/cap.sh`; both are already on `staging`
(they are that commit's other half), so only the two code files were recovered.

Nothing in the recovered code is unsound: it uses rewrite rules, library lemmas and HOL Light's
proof-producing `INT_ARITH`; there is no `CHEAT_TAC`, no `new_axiom`, no `mk_thm` (the only
occurrences of those strings are in a comment saying so). A failed close can only fail; it cannot
produce a false theorem.

## 2. Two focused attempts

### Attempt 1 — recovered as-is (plus the minimum to run)

The WIP was first made to load and reach the residue. Three things had to be fixed before it got
past name resolution:

1. `INST_TYPE` was applied to a *term*; it is a theorem rule. The per-width upper-bound fact now
   uses the library lemma `INT_VAL_BOUND` instantiated at the width.
2. The bitval rewrite used `prove` with an `ASSUME`; `prove` refuses a result with hypotheses. It
   is now a contextual rewrite carrying the branch fact (`BITVAL_CLAUSES` only; `bitval` itself
   is not unfolded, so undecided carries stay atoms).
3. `at`-wrapped stage reporting for the residual (prints only in `--mode phase`).

Result: `Stack overflow` in `ASM_INT_ARITH_TAC` on the first residual, with the goal still
containing `word_sub`/`word_add` and `word_ult`.

### Attempt 2 — normalise, bound, and supply integrality

Three genuine gaps were found and closed (all generic; none names p25519, a width or a limb
count):

- **`[IX.70-a]` unsigned comparison and machine add/sub were never rewritten.** The correction
  condition stayed `word_ult s x \/ word_ule pw s` (opaque to the arithmetic), and the goal kept
  `&(val (word_add ...))` opaque, so `INT_VAL_WORD_ADD_CASES`/`INT_VAL_WORD_SUB_CASES` could not
  expose the wrap. `BRIDGE_ALG_NORM_TAC` now rewrites `WORD_ULT`, `WORD_ULE`, the two CASES
  lemmas, the joins and the `val` numerals, on the goal **and every assumption**.
- **`[IX.70-b]` the carry atoms were only bounded below.** `0 <= &(val w)` is not enough; without
  `&(val w) < &2^dimindex` the accumulator's top carry is not pinned and the overflow branches
  are satisfiable. `BRIDGE_ATOM_BOUNDS_TAC` adds both bounds.
- **`[IX.70-c]` integrality.** `INT_ARITH`/`ARITH_RULE` reduce to a *real*-linear procedure
  (`linear_ineqs: no contradiction`). `0 <= c <= 1` does not make a carry integral, so the
  procedure cannot conclude `c = 0` from `0 <= W < 2^256` and `W = M + N - 2^256 c`.
  `BRIDGE_SPLIT_TOP_CARRY_TAC` case-splits the structural carry — the one on the side of the
  hypothesis that contains the highest-weighted input atom (weight computed from the nested
  2^k·(…) form), i.e. the carry out of the most-significant limb's addition. Every branch here
  already fixes the *corrected* carry (the selection flag) through a branch fact.

Result: the route now reaches a fully linear integer equation with `0` SAT closes, and then
`ASM_INT_ARITH_TAC` **overflows the stack** (not a wrong subgoal). The residual is §3.

## 3. Measurements (`emit --mode phase`, `tools/cap.sh 12G`, `make -j1`)

All numbers below are printed by `tests/ix70_report.sh` from the `.correct` logs into
`research/receipts/ix70-algebraic-close.json`.

| route | total primitive inferences | SAT closes | bitblast | defn+clausify | replay | wall |
|---|---:|---:|---:|---:|---:|---:|
| SAT, full (pre-IX.69) hypothesis set | **119,194,788** | 4 | 53,397,616 | 21,500,946 | 40,808,833 | 55 s |
| SAT, IX.69 slice (shipped default) | **111,501,987** | 4 | 53,396,030 | 21,500,774 | 33,117,926 | 53 s |
| algebraic close (opt-in) | **none — aborts** | **0** | 0 | 0 | 0 | abort after 11.2 s CPU |

Algebraic run, from the marks: `bridge:algebra` at 70,151,488; `bridge:algebra-arith` at
70,267,380; pre-theorem counter 67,646,907, so **2,620,473 inferences were spent before the
abort**. There is no `STAMP_INFERENCES` line and no theorem. `bitblast`, `defn_clausify`,
`encode`, `replay` and `post_solve` are all zero because no SAT close is entered.

The failing residual, verbatim (`ALG_FAIL` block; the first branch is the top carry fixed to 1):

```text
ALG_FAIL stage=arith goal=((&18446744073709551616 *
  (&18446744073709551616 * (&18446744073709551616 * &(val m_3) + &(val m_2)) +
   &(val m_1)) +
  &(val m_0)) +
 &18446744073709551616 *
 (&18446744073709551616 * (&18446744073709551616 * &(val n_3) + &(val n_2)) +
  &(val n_1)) +
 &(val n_0)) -
&57896044618658097711785492504343953926634992332820282019728792003956564819949 =
&18446744073709551616 *
(&18446744073709551616 *
 (&18446744073709551616 * &(val sum_s8) + &(val sum_s7)) +
 &(val sum_s4)) +
&(val sum_s3)
...
ALG_HYP 48 &57896044618658097711785492504343953926634992332820282019728792003956564819949 <=
 (&18446744073709551616 * (... m ...) + &18446744073709551616 * (... n ...))
ALG_HYP 50 (&18446744073709551616 * (... m ...) + &18446744073709551616 * (... n ...)) <
&115792089237316195423570985008687907853269984665640564039457584007913129639936
ALG_HYP 54 ~carry_s13
ALG_HYP 56 &(val n_0) + (&18446744073709551616 * &(val n_1) + &0) + ...
       < &57896044618658097711785492504343953926634992332820282019728792003956564819949
ALG_HYP 57 &(val m_0) + (&18446744073709551616 * &(val m_1) + &0) + ...
       < &57896044618658097711785492504343953926634992332820282019728792003956564819949
ALG_HYP 58 &0 + &(val sum_s13) = &(val sum_s8) + &9223372036854775808 + &(bitval carry_s12)
ALG_HYP 59 (&18446744073709551616 * &(bitval carry_s12) + &0) + &(val sum_s12) =
       &(val sum_s7) + &(bitval carry_s11)
ALG_HYP 60 (&18446744073709551616 * &(bitval carry_s11) + &0) + &(val sum_s11) =
       &(val sum_s4) + &(bitval carry_s10)
ALG_HYP 61 (&18446744073709551616 * &(bitval carry_s10) + &0) + &(val sum_s10) =
       &(val sum_s3) + &19
ALG_HYP 62 (&18446744073709551616 * &1 + &0) + &(val sum_s8) =
       &(val m_3) + &(val n_3) + &(bitval carry_s7)
ALG_HYP 63 (&18446744073709551616 * &(bitval carry_s7) + &0) + &(val sum_s7) =
       &(val m_2) + &(val n_2) + &(bitval carry_s4)
ALG_HYP 64 (&18446744073709551616 * &(bitval carry_s4) + &0) + &(val sum_s4) =
       &(val m_1) + &(val n_1) + &(bitval carry_s3)
ALG_HYP 65 (&18446744073709551616 * &(bitval carry_s3) + &0) + &(val sum_s3) =
       &(val m_0) + &(val n_0)
ALG_HYP 68 nonoverlapping_modulo (2 EXP 64) (pc,80) (val z,8 * 4)
ANSWER_EXCEPTION: Stack overflow
Failed after (user) CPU time of 11.199575: Stack overflow
```

`ALG_HYP 1`–`47` are the `&0 <= …` / `… < &2^64` bounds (one pair per `&(val *)` and `&(bitval *)`
atom) plus `true`; the four condition branches are visible as `48` (`p <= M+N`), `50`
(`M+N < 2^256`), `54` (`~carry_s13`) and the branch's substituted top carry in `62` (`&1`).

The subgoal is arithmetically inconsistent: `62`–`65` say
`W_raw = M + N - 2^256·1`, while `W_raw >= 0` (the sum bounds) and `M+N < 2^256` (`50`) make it
negative. So the failure is **not** that the goal is unprovable.

## 4. Controls

### C1 — the statement is the same obligation (weaker check, stated)

`equals_thm` needs two live theorems, and the algebraic path produces none, so it cannot be run.
The weaker check is exact and was run: emit the arm `add_p25519` answer in `--mode answer` with the
switch off and on, assemble both with `dropin assemble --stamp`, and diff the two scripts.

```text
$ ./target/release/emit --task <tid> --mode answer --out .bench/ix70/c1-sat.answer --spent \
    --spent-manifest .bench/ix66/spent-manifest.json
$ TEKMERION_ALG_CLOSE=<tid> ./target/release/emit --task <tid> --mode answer \
    --out .bench/ix70/c1-alg.answer --spent --spent-manifest .bench/ix66/spent-manifest.json
$ ./target/release/dropin assemble --task <tid> --answer .bench/ix70/c1-sat.answer --stamp
$ ./target/release/dropin assemble --task <tid> --answer .bench/ix70/c1-alg.answer --stamp
$ diff .bench/ix70/c1-sat.ml .bench/ix70/c1-alg.ml
2956c2956
< BRIDGE_ADD_REF_TAC (CONV_TAC (ONCE_DEPTH_CONV BIGNUM_OF_WORDLIST_CONV)) p_25519) g
---
> BRIDGE_ADD_REF_ALG_TAC (CONV_TAC (ONCE_DEPTH_CONV BIGNUM_OF_WORDLIST_CONV)) p_25519) g
```

The `time prove (…, …)` invocation, the frozen query and the postcondition are byte-identical;
only the close entry point differs. This is identity of the statement and goal term, not
`equals_thm`, and is weaker.

### C2 — axioms: established for the SAT control, not for the algebraic path

The SAT runs exit 0, which the driver treats as `check_axioms = true`. The stamp of the sliced
run is the expected three:

```text
$ grep -E '^STAMP_NAME' .bench/ix70/sliced/arm_add_p25519.correct
STAMP_NAME ETA_AX 1
STAMP_NAME SELECT_AX 2
STAMP_NAME INFINITY_AX 3
```

The algebraic path produces no theorem, so its stamp does not exist. The machinery it adds is
definitions and tactics over library lemmas; it introduces no axiom (`grep` for
`CHEAT|new_axiom|mk_thm` finds only the comment that says it introduces none).

### C3 — firing control: not obtainable in the intended form

There is no success to weaken: the algebraic close fails. The honest statement is that the
intended C3 (break a working close and watch the proof fail) is **inapplicable**. Two negative
controls were run on the isolated system instead (§6): the system closes with the top carry as a
disjunction (`OK 0.482 s`) and fails when the corrected accumulator chain is removed
(`allocation failure`). The in-tree failure itself is the evidence that the route is not vacuous
and not silently accepting anything: it proves nothing.

### C4 — no solver on the algebraic path

A failing `cadical` shim is placed **before** the Lean toolchain bin in `PATH`; it appends to a
marker and exits 1. The SAT path invokes it (positive control); the algebraic path does not.

```text
$ bash .bench/ix70/run_task_shim.sh arm curve25519 add p25519 .bench/ix70/c4-sat   # default gate
RUN arm_add_p25519 rc=2 ...
$ wc -l .bench/ix70/c4-sat/cadical-invocations.txt
2
$ cadical-invoked --no-binary --lrat /tmp/csathol6c17d6.cnf /tmp/csathol06458c.lrat
$ cadical-invoked --no-binary --lrat /tmp/csatholc108a5.cnf /tmp/csathol7e69e5.lrat

$ TEKMERION_ALG_CLOSE=arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT \
    bash .bench/ix70/run_task_shim.sh arm curve25519 add p25519 .bench/ix70/c4-alg
RUN arm_add_p25519 rc=2 ...
$ grep -c 'bridge:prove-entry' .bench/ix70/c4-alg/arm_add_p25519.correct
0
$ cat .bench/ix70/c4-alg/cadical-invocations.txt
0
```

So the algebraic path performs **zero** `cadical` invocations for this theorem (and enters zero
SAT closes). The shim is proven capable of firing by the control above.

### C5 — scope: one task, others unchanged

The switch is off by default, so the shipped emitter is unchanged for every task. Three other
tasks were re-run on the final tree; each reproduces its IX.69 sliced total exactly:

```text
arm_add_p256      STAMP_INFERENCES 118896814   (IX.69: 118,896,814)
arm_neg_p25519    STAMP_INFERENCES  33888658   (IX.69:  33,888,658)
x86_neg_sm2       STAMP_INFERENCES  15444067   (IX.69:  15,444,067)
```

`arm_add_p25519` itself is unchanged by default: `111,501,987` (above). The gate is one task and
requires an explicit environment switch to select it.

### C6 — curve specificity

**Generic:** the tactic names no modulus, width, limb count or transducer. It is driven by
`WORD_ULT`/`WORD_ULE`, `INT_VAL_WORD_ADD_CASES`/`SUB_CASES` (generic in `:N`), `VAL_WORD_JOIN_SIMPLE`,
`VAL_BOUND`/`INT_VAL_BOUND`, `VAL_WORD_EQ`, and `BITVAL_CLAUSES`. The join expansion recurses on
the nesting; the coefficients are read from the goal. The only per-task datum is the modulus
numeral, supplied by the caller exactly as in the SAT path.

**Specific to this instance:** (i) the *gate* is one task id; (ii) the intermediate carries cancel
symbolically only because the chain is a plain base-2^k accumulator with the correction added as
a constant, which is the shape of all six `bignum_add_*` transducers, not only p25519; (iii) the
one integrality-critical atom is the top carry of the raw add, again a property of the chain, not
the curve. Nothing keys on 25519, 64, or 4.

**The generic form of the missing step** is the carry-conservation lemma of `XV.E1`: prove once,
parametric in width/limb count, that a chain `b_i c_i + s_i = a_i + b_{i-1}` with bounded limbs
composes to `W + B^n c_n = Σ B^i (a_i + b_i) + …`. Then each chain becomes a single aggregate
identity by rewriting, and the decision procedure never sees a 16-variable integer system. That
is a mechanism; the case split of §2 is a workaround that does not scale (it is exponential in
the number of undecided carries — measured: splitting all seven exhausts memory).

## 5. Wall-clock and resource record

`free -g` recorded before the long runs: `total 125, used 25, free 85 (available 100)` at the
first check, and `used 25, free 77 (available 100)` later; available never fell below 100 GB.
Every proof run used `tools/cap.sh 12G -- make -j1`, one at a time. Disk before and after is in
the report.

## 6. Is the approach wrong or merely incomplete?

**Merely incomplete**, at the level of the arithmetic engine. The evidence is a probe of the
abstracted close-1 system (plain `int` variables for `&(val w)`, `&(bitval c)`; the same
equations, bounds and side conditions), in `.bench/ix70/arith_probe.ml`:

- with the top carry given as `c8 = 0 \/ c8 = 1`, `INT_ARITH` proves the close-1 obligation in
  **0.482 s**;
- with `c8` fixed to 1 it also proves it (0.388 s);
- removing the corrected accumulator chain makes it fail (`allocation failure`).

`research/receipts/ix70-probe.log` holds these lines. So the mathematics of the residue closes;
what fails in the tree is handing the 60+-hypothesis `&(val ·)`/`&(bitval ·)` encoding to HOL
Light's real-linear procedure. That is why the missing piece is an aggregate/structured reduction
(`XV.E1`), or a genuinely integer-linear procedure — not another rewrite.

## 7. What is not established

- **No inference total, no phase split, no SAT-close reduction**: the theorem is not proved. The
  "~4.08M without the SAT close" reference point is untouched; `ix65`'s 1.85M is untouched.
- **C2/C3 for the algebraic path**: no theorem, so no axiom stamp and no success to weaken.
- **The isolated probe is an abstraction**, not the in-tree term; it shows the system is
  arithmetically consistent, not that any particular encoding will be discharged by `INT_ARITH`.
- **One host, one kernel, one pin**: all numbers are from this machine and
  `.bench/hol-light-stamp2`.
- **Freeze `v6` is required before any sealed run.** No `v6` tag or freeze was created by this
  task; `tools/bridge.ml` is inside the frozen runtime, so a sealed evaluation after this change
  must first create and verify the freeze (`XV.G4`).

## 8. Commands (verbatim)

```text
$ git show 0336cb2 -- tools/bridge.ml src/tools/emit.rs     # recovered diff
$ free -g | head -2
total        used        free      shared  buff/cache   available
Mem:             125          25          77           0          24         100
$ df -h /mnt/1tb_kingston | tail -1
/dev/nvme1n1p1  1.3T  1.3T  8.0G 100% /mnt/1tb_kingston

$ TEKMERION_SLICE_CONTROL=full bash .bench/ix69/run_task.sh arm curve25519 add p25519 .bench/ix70/prefull
SLICE kept=17/17 task=arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT   (x4)
STAMP_INFERENCES 119194788
Running time: 55.000000 sec

$ bash .bench/ix69/run_task.sh arm curve25519 add p25519 .bench/ix70/sliced
SLICE kept=11/17 ... (x2), SLICE kept=13/17 ... (x2)
STAMP_INFERENCES 111501987
Running time: 53.000000 sec

$ TEKMERION_ALG_CLOSE=arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT \
    bash .bench/ix69/run_task.sh arm curve25519 add p25519 .bench/ix70/alg
RUN arm_add_p25519 rc=2 phase_marks=15
ALG_FAIL stage=arith goal=(... as quoted in §3 ...)
ANSWER_EXCEPTION: Stack overflow
Failed after (user) CPU time of 11.199575: Stack overflow

$ bash tests/ix70_report.sh .bench/ix70 > research/receipts/ix70-algebraic-close.json
$ tools/cap.sh 20G -- cargo test --offline
$ cargo clippy --offline --all-targets -- -D warnings
$ cargo fmt --check
$ git ls-files '*.py'
```

**Fixtures:** `tools/bridge.ml` changed, so the tracked answers were re-recorded deliberately
(`TEKMERION_RECORD_ANSWERS=1 tools/cap.sh 20G -- cargo test --offline --test tools_emit`, 27
files). The receipt-digest and phrase-count assertions in `tests/tools_receipt.rs` were updated
in the same change, with the IX.69 digests kept as the boundary.
