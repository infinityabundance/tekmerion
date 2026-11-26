# IX.82 — construct the aggregate, don't recover it: `arm.bignum_add_p25519` closes with no SAT

- **Date:** 2026-11-25
- **Branch:** `staging` (parent `a823a0a`; `main` and tag `freeze-rust-closure-v6` at `cb054bd` untouched)
- **Change under test:** `tools/bridge.ml` §6c.3c (the weighted-sum aggregate) and the close
  in `BRIDGE_ORACLE_TAC`. Gate unchanged: `TEKMERION_ALG_CLOSE=<task id>`.
- **Verdict: it proves.** `arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT` closes in
  **5,233,638 primitive inferences**, **28.0 s**, **0 SAT closes**, over **12 oracle branches**.
  That is **21.30×** below the sliced SAT route, **22.77×** below the full SAT route, and
  **2.83×** the human `_CORRECT` body. The 60.35× SAT gap for this family is closed without the
  native kernel.
- **Not attempted:** `sub`, `neg`, `x86/add`. There is no algebraic entry point for them (only
  `BRIDGE_ADD_REF_ALG_TAC` exists), so no number is reported for a route that was not run.

---

## 1. What IX.78 left, and what was wrong with recovering it

IX.78's oracle eliminated the carry atoms with exact Gauss–Jordan and handed the residual to
`INT_ARITH`. It was sound, made no solver call, and emitted the right fact, but it did not close:
the elimination gives `carry_s8` a pivot with a `2^256` coefficient and the raw relation

```
M + N = W_raw + 2^256 · c8
```

never reaches the residual. IX.78 said the fix is to *project* the definition, or to eliminate
per chain. That is what this change does, and it does not solve for anything.

## 2. The construction: aggregate, weighted by the digit positions

The human proof sums the per-limb equations; the internal carries cancel because consecutive
limbs see them with coefficients `+1` and `−B`. The same sum is built here, as a *proof*.

### 2.1 The two chains, read from this proof's hypotheses

Raw chain (hypotheses 61–64 of the residue), `B = 2^64`:

```
(B·&(bitval carry_s3) + &0) + &(val sum_s3) = &(val m_0) + &(val n_0)
(B·&(bitval carry_s4) + &0) + &(val sum_s4) = &(val m_1) + &(val n_1) + &(bitval carry_s3)
(B·&(bitval carry_s7) + &0) + &(val sum_s7) = &(val m_2) + &(val n_2) + &(bitval carry_s4)
(B·&(bitval carry_s8) + &0) + &(val sum_s8) = &(val m_3) + &(val n_3) + &(bitval carry_s7)
```

Corrected chain (hypotheses 57–60):

```
(B·&(bitval carry_s10) + &0) + &(val sum_s10) = &(val sum_s3) + &19
(B·&(bitval carry_s11) + &0) + &(val sum_s11) = &(val sum_s4) + &(bitval carry_s10)
(B·&(bitval carry_s12) + &0) + &(val sum_s12) = &(val sum_s7) + &(bitval carry_s11)
&0 + &(val sum_s13) = &(val sum_s8) + &9223372036854775808 + &(bitval carry_s12)
```

The last constant is `2^63` at limb 3, so the corrected chain adds `19 + 2^255 = 2^256 − p`.

### 2.2 The coefficients, and how they were obtained

**Raw combination:** `1·eq64 + B·eq63 + B²·eq62 + B³·eq61`.
**Corrected combination:** `1·eq60 + B·eq59 + B²·eq58 + B³·eq57`.

The weight list printed in phase mode is

```
base = 18446744073709551616                  (B = 2^64)
weights = [1,
           18446744073709551616,              (B)
           340282366920938463463374607431768211456,                          (B²)
           6277101735386680763835789423207666416102355444464034512896]       (B³)
```

The list is **derived, not hard-coded** (`XIX.C7`): a *limb equation* is an integer equation
with at least one `&(bitval c)` atom and one non-carry atom; two limb equations are linked when
they share a carry atom, and the link points from the equation where that atom has coefficient
magnitude `B > 1` (outgoing) to the equation where it has magnitude `1` (incoming). A chain is a
connected component that is a directed path; its root has in-degree 0 and its weights are
`w(next) = B·w(previous)`. `B` is read from the outgoing coefficient, the limb count from the
path length, and the term structure from this theorem's assumptions. If the links are not
disjoint `(outgoing B, incoming 1)` paths, **no aggregate is emitted** and the close falls back to
IX.78's behaviour — no regression.

### 2.3 The emitted equations (verbatim, flat)

Raw (`AGGREGATE limbs=4 base=18446744073709551616 premises=4`):

```
(((((((((((&(val sum_s3) + -- &1 * &(val m_0)) + -- &1 * &(val n_0)) +
          &18446744073709551616 * &(val sum_s4)) +
         -- &18446744073709551616 * &(val m_1)) +
        -- &18446744073709551616 * &(val n_1)) +
       &340282366920938463463374607431768211456 * &(val sum_s7)) +
      -- &340282366920938463463374607431768211456 * &(val m_2)) +
     -- &340282366920938463463374607431768211456 * &(val n_2)) +
    &115792089237316195423570985008687907853269984665640564039457584007913129639936 *
    &(bitval carry_s8)) +
   &6277101735386680763835789423207666416102355444464034512896 * &(val sum_s8)) +
  -- &6277101735386680763835789423207666416102355444464034512896 * &(val m_3)) +
 -- &6277101735386680763835789423207666416102355444464034512896 * &(val n_3) = &0
```

i.e. `W_raw + 2^256·c8 − (M + N) = 0`.

Corrected:

```
((((((&(val sum_s10) + -- &1 * &(val sum_s3)) +
      &18446744073709551616 * &(val sum_s11)) +
     -- &18446744073709551616 * &(val sum_s4)) +
    &340282366920938463463374607431768211456 * &(val sum_s12)) +
   -- &340282366920938463463374607431768211456 * &(val sum_s7)) +
  &6277101735386680763835789423207666416102355444464034512896 * &(val sum_s13)) +
 -- &6277101735386680763835789423207666416102355444464034512896 * &(val sum_s8) =
 &57896044618658097711785492504343953926634992332820282019728792003956564819987
```

i.e. `W_cor − W_raw = 2^256 − p`. The right-hand literal is `2^256 − p_25519`.

### 2.4 How the combination is proved

Each premise is flattened (`bridge_flat_th`, every rewrite by `INT_RING`), scaled by
`AP_TERM (( * ) k)`, and the equations are added by `MK_COMB`; the sum is normalised to the flat
canonical form by a `INT_RING` identity. The integer weights are therefore part of the proof,
not a solver's trace: the certificate for `XIX.D1` is `(premises, weights, conclusion)` and is
printed as the `AGGREGATE` line above.

The aggregate is **constructed** from the equations; it is not another `INT_ARITH` call, and it
does not eliminate anything.

## 3. The fact, and why the aggregate does not close by itself

The top carry is decided from the aggregate, not from the relaxation:

```
ORACLE fact &(bitval carry_s8) = &0 why=aggregate-band
```

The proof is: `INT_ARITH` derives `&(bitval carry_s8) < &1` from the raw aggregate, `M+N < 2^256`
and the `0 <= &(val …)` bounds (pure real-linear); then a `bitval` case split turns `< &1` into
`= &0`. Both the bound and the case split are proof-producing, and the premise set is the
aggregate plus the integer bounds — not the 60-hypothesis system that exhausted the stack in
IX.70.

The bound matters: `p ≤ M+N` (a branch condition), `M + N = W_raw` (raw aggregate plus `c8 = 0`)
and `W_cor − W_raw = 2^256 − p` (corrected aggregate) together give `M+N < p`, because
`W_cor` is a sum of four digits each `< B` and therefore `W_cor < B^4 = 2^256`. The final
`INT_ARITH` needs the digit bounds for that last step, so the close set is

```
2 aggregates + 1 fact + the non-equational integer hypotheses (digit bounds, carry ranges,
the relational bounds)   = 53 or 54 antecedents per branch
```

It excludes the Gauss–Jordan carry definitions and the residual; those carry the accumulated
`B^n` coefficients that drove IX.78's coefficient blow-up.

**The IX.81 change did matter, and did not matter.** After the interval fix, the relaxation no
longer proposes `carry_s8 = 0` (nor a false value for `carry_s3/4/7`): all seven carries report
`undetermined … why=relaxation`. The fact is now derived from the aggregate
(`why=aggregate-band`). The old proposal path was a lucky consequence of the join bug; the
aggregate does not depend on it. This is why the close is robust to the correction rather than a
casualty of it.

## 4. Measurements

`emit --mode phase`, `tools/cap.sh 12G -- make -j1`, `.bench/hol-light-stamp2`.

| route | total primitive inferences | SAT closes | wall |
|---|---:|---:|---:|
| SAT, full hypothesis set (IX.70) | **119,194,788** | 4 | 55 s |
| SAT, sliced (shipped default; re-measured today) | **111,501,987** | 4 | 49 s |
| human `_CORRECT` body (IX.65, cited) | **1,848,494** | — | — |
| **aggregate close (opt-in)** | **5,233,638** | **0** | **28.0 s** |

- vs sliced SAT: **21.30×** fewer; vs full SAT: **22.77×** fewer; vs human: **2.83×** more.
- Pre-theorem: `67,646,907`. First `bridge:algebra`: `70,151,488`. Last
  `bridge:algebra-arith`: `72,660,028`. `STAMP_INF_AFTER`: `72,880,545`. So
  **2,504,581** inferences are spent before the first oracle entry and **2,729,057** in the
  aggregate/close region (12 branches).
- `bridge:pre-bitblast`, `bridge:sat-done`, `bridge:pre-replay`: **0 each**. `bitblast`,
  `defn_clausify`, `replay`: 0. The run enters **12** oracle closes, all `ORACLE close ok`.
- Antecedents per close: six branches with 53, six with 54.
- `AGGREGATE` lines: 24 (two per branch), all `limbs=4`, same base and weight list.

**Is the 60× gap for the linear families closed without the new kernel?** For this theorem, yes:
the 60.35× sliced-SAT gap becomes 2.83×, with no solver and no native kernel. It is not 1×: the
remaining factor over the human proof is the 12-branch case split and the normalisation before
each close, not SAT.

## 5. Controls, verbatim

### C1 — `equals_thm` against the SAT-route theorem, in one session

Both routes now produce theorems, so the strong check runs. The two `--mode answer` artefacts are
byte-identical except for the close entry point:

```
$ diff .bench/ix82/c1-sat.ml .bench/ix82/c1-alg.ml
4100c4100
< BRIDGE_ADD_REF_TAC (CONV_TAC (ONCE_DEPTH_CONV BIGNUM_OF_WORDLIST_CONV)) p_25519) g
---
> BRIDGE_ADD_REF_ALG_TAC (CONV_TAC (ONCE_DEPTH_CONV BIGNUM_OF_WORDLIST_CONV)) p_25519) g
```

In one HOL session (both files loaded after `arm/proofs/base.ml`; the shared `p_25519` and
`bignum_add_p25519_mc` definitions are loaded once, so the terms are the same constants):

```
$ #load "unix.cma";; needs "arm/proofs/base.ml";;
$ loadt "arm/proofs/c1_alg.ml";; let c1_alg_th = BIGNUM_ADD_P25519_CORRECT;;
$ loadt "arm/proofs/c1_sat.ml";; let c1_sat_th = BIGNUM_ADD_P25519_CORRECT;;
C1_ALG_CONCL_HASH 1021733992
C1_SAT_CONCL_HASH 1021733992
C1_ACONV_CONCL true
C1_EQUALS_THM true
C1_HYPS_ALG 0
C1_HYPS_SAT 0
```

`equals_thm c1_alg_th c1_sat_th = true`, both conclusions are alpha-equivalent, and **both
theorems have zero hypotheses**. No hypothesis was added.

### C2 — axioms

The algebraic artifact exits 0 through `check_axioms ()` (the build template calls it) and is
stamped with the three permitted axioms, measured with `.bench/hol-light-stamp2`:

```
$ grep -E '^STAMP_NAME' .bench/ix82/alg/arm_add_p25519.correct
STAMP_NAME ETA_AX 1
STAMP_NAME SELECT_AX 2
STAMP_NAME INFINITY_AX 3
```

### C3 — remove the aggregate: the proof fails

With `let aggregates = []` in `BRIDGE_ORACLE_TAC` (only change; reverted):

```
RUN arm_add_p25519 rc=2 phase_marks=15
ORACLE system defs=7 residual=1 facts=0 aggregates=0
ORACLE close ants=50 size=9736
ORACLE close failed: Failure("INT_ARITH `&0 <= &(val sum_s8)
==> &(val sum_s8) < &18446744073709551616
==> …`: linear_ineqs: no contradiction")
ALG_FAIL stage=oracle goal=((&18446744073709551616 * …
```

No theorem, no inference total. The aggregate is load-bearing.

### C4 — zero `cadical` invocations

A failing `cadical` shim is first on `PATH` (`.bench/ix70/shim`, appends a marker and exits 1):

```
$ rm -f .bench/ix70/cadical-invoked.txt
$ TEKMERION_ALG_CLOSE=arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT \
    bash .bench/ix70/run_task_shim.sh arm curve25519 add p25519 .bench/ix82/c4
RUN arm_add_p25519 rc=0 phase_marks=29
STAMP_INF_BEFORE 67646907
STAMP_INFERENCES 5233638
$ ls .bench/ix70/cadical-invoked.txt
ls: cannot access '.bench/ix70/cadical-invoked.txt': No such file or directory
```

**Zero invocations.** The marker is absent; the proof still closes (12 `ORACLE close ok`) and
its inference total is the same 5,233,638 as without the shim.

### C5 — no cheating

```
$ grep -nE 'new_axiom|mk_thm|CHEAT_TAC|Obj\.magic' tools/bridge.ml
1003:(* procedure only; no new axioms, no CHEAT_TAC), and the same linear system  *)
```

One hit, a pre-existing comment. No hypothesis is added: C1 reports `hyp = 0` for both routes.

### C6 — the default path is unchanged

The switch is unset by default, so every task emits exactly the tactic it had. Re-measured today
with the new `tools/bridge.ml`:

```
arm.bignum_add_p25519  STAMP_INFERENCES 111501987   (IX.69/IX.78: 111,501,987)
arm.bignum_add_p256    STAMP_INFERENCES 118896814   (IX.69: 118,896,814)
arm.bignum_neg_p25519  STAMP_INFERENCES  33888658   (IX.69:  33,888,658)
x86.bignum_neg_sm2     STAMP_INFERENCES  15444067   (IX.69:  15,444,067)
```

All four match exactly.

### C7 — generality: what is specific, what is generic

**Generic, computed per theorem:**
- the chain detection (limb equations and their shared carry atoms);
- the base `B` (from the outgoing coefficient magnitude) and the weight list (`1, B, B², …`);
- the weighted-sum proof (`AP_TERM`/`MK_COMB` plus `INT_RING`);
- the flat normalisation before every `INT_ARITH` call;
- the top-carry fact (strict linear bound plus a `bitval` case split);
- the close (aggregates + fact + the theorem's own bounds).

Nothing names `p25519`, `64`, `4`, a limb count or an instruction. The weights are printed as
`[1, B, B², B³]` derived from the equations above.

**Specific to this instance:**
- the corrected chain's constant (`2^256 − p` here) is whatever the proof's equations contain —
  `19` at limb 0 and `2^63` at limb 3; it is read, not assumed;
- the instruction sequence is the four `ADDS`/`ADCS` plus the `ADDS`/`ADCS` correction block and
  the `CSEL` select; a different sequence would produce different hypotheses, and the chain
  reader would have to find the same path structure;
- **one theorem, one architecture.** `sub`, `neg` and `x86/add` were not attempted: there is no
  algebraic entry point for them. The generality claim is about the algorithm, not about a
  measured second family.

**Not a general certificate engine yet:** the aggregate detector requires disjoint
`(outgoing B, incoming 1)` carry paths; a proof that presents the chain differently emits no
aggregate (and falls back, correctly, to the IX.78 close). The `XIX.D1` certificate exists as
`(premises, weights, conclusion)` in the proof and in the JSON, not as a serialised
`LinearCombinationCertificate` consumed by a checker.

## 6. Environment

```
$ free -g | head -2
               total        used        free      shared  buff/cache   available
Mem:             125          12          62           0          49         112
$ df -h /mnt/1tb_kingston | tail -1
/dev/nvme1n1p1  1.3T  902G  360G  72% /mnt/1tb_kingston
```

Every proof run used `tools/cap.sh 12G -- make -j1`, one target at a time; `cargo` runs used
`tools/cap.sh 20G`. No cap fired.

## 7. Fixtures and goldens: what moved, and why

`tools/bridge.ml` changed, so every inlined answer changed. Re-recorded deliberately with
`TEKMERION_RECORD_ANSWERS=1 cargo test --offline --test tools_emit` (27 files under
`tests/fixtures/answers/`). The pins that moved, and are updated in the same commit:

| pin | was | now |
|---|---|---|
| `BRIDGE_DIGEST` | `c4b52cc7…` (XIX.A) | `65ced317f254a0122117e3800b07a331e03ed98d3efb1e0f4aa12bbceff9b6ca` |
| `curve_check` statements | 81 | 86 |
| `instrument` phrases | 221 | 232 |
| `instrument` output digest | `61cded3e…` | `fb1677dc3e7785cae102b118b6d8b4731c90951ef02d6fc8c5d9b9402ca2bcf5` |
| `inline_answer` phrases | 265 | 276 |
| inline `add-ref` | `e2ca43d5…` | `a930a4e6466d3f4db7becfddbddb73b07e626fef546ea39406e5a89660137029` |
| inline `sub` | `45daa0a6…` | `eb3215a8be9965309c357d98a4ccb57deb8efec65617bb62aa98c98d149eac71` |
| inline `neg` | `0c9706a2…` | `e957d506e1e2d8044d3b1f9b9df66e359b162059fbb5b6c956e1eb0c45833fdd` |
| inline `add-p384` | `b5a74287…` | `5fad9abe89f876e841404c414bb12110afd17f9c9042cda20c9e8eb6106bf5ee` |
| `curve_check` report fixture | — | regenerated (`tests/fixtures/curve_check_bridge.txt`) |

`research/receipts/freeze-closure-v7.json` was regenerated: 73 source files,
`source_digest 9b907ddcb1e2e010ba75386a12b1fca55bd21a46c38814bab9a8288b4c64d5a9`,
`repo_commit a823a0a…` (the parent; the same delta convention the previous v7 used). The
`freeze-rust-closure-v6` tag at `cb054bd` was not touched.

## 8. Commands (verbatim)

```
$ TEKMERION_ALG_CLOSE=arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT \
    bash .bench/ix69/run_task.sh arm curve25519 add p25519 .bench/ix82/alg
RUN arm_add_p25519 rc=0 phase_marks=29
STAMP_INF_BEFORE 67646907
STAMP_INFERENCES 5233638
Running time: 28.000000 sec

$ grep -c 'ORACLE close ok' .bench/ix82/alg/arm_add_p25519.correct      # 12
$ grep -c 'bridge:pre-bitblast' .bench/ix82/alg/arm_add_p25519.correct  # 0
$ grep -c 'bridge:sat-done' .bench/ix82/alg/arm_add_p25519.correct      # 0

$ tools/cap.sh 20G -- cargo test --offline            # all targets pass
$ cargo clippy --offline --all-targets -- -D warnings # clean
$ cargo fmt --check                                   # clean
$ git ls-files '*.py'                                 # empty
```

## 9. What is not established

- **One theorem, one architecture.** `sub`, `neg` and `x86/add` have no algebraic entry point and
  were not run. No number is reported for them.
- **The aggregate detector is structural, not general.** It requires the chain equations to be
  disjoint `(outgoing B, incoming 1)` paths. A different presentation emits no aggregate.
- **The `XIX.D1` certificate is not serialised.** The weights and premises are in the proof and
  in `ix82-aggregate.json`; no checker consumes them yet.
- **The interval relaxation still proposes nothing** for these carries. The fact no longer
  depends on it (it comes from the aggregate), but the discovery engine is still a filter, not a
  certificate.
- All numbers are from this host, this kernel (`.bench/hol-light-stamp2`) and this commit.

---

## 10. Correction log — IX.86 reconciliation (2026-11-26)

Appended, not substituted: the text above is the record of the tree it measured.

- **The number above is right for its tree.** `5,233,638` was reproduced today at the
  IX.82 tree (`e36dc58`; `tools/bridge.ml` identical to `677d97e`), rebuilt from
  `git archive` in `.bench/ix86/old-src` and run with the same harness: **5,233,638**,
  **28 s**, **12** oracle branches, **0** SAT closes. So this receipt's measurement
  is not withdrawn.
- **It is not the current commit's number.** At `81f84db` (bridge unchanged since
  `3d40351`) the same route measures **5,745,767**, **30 s**, **12** branches, **0**
  SAT closes. `ix83`'s figure is therefore the standing one; the 2.83× in §4 and in
  the verdict is **scoped to `e36dc58`**, and the current figure is **3.11×**.
- **Why the 512,129 difference.** `ix86` splits it by phase: **477,376 (93.2%)** is
  the aggregate **fact proof**, **22,909** the preamble, **11,568** the
  normalisation, **276** the branch-return/tail. The aggregate construction is
  identical on both trees (**216,726**), the two `AGGREGATE` equations and weights
  are identical, and the final linear close is identical (**1,128,617**). The fact
  proof changed from `agg :: side_hyps` (the carrier's aggregate) to
  `aggregates @ side_hyps` (all aggregates) in `3d40351`, because `sub`'s correction
  borrow needs both; for `add` the second aggregate is extra weight. The suspicion
  that the generalised detector emits a more expensive aggregate is **not** borne
  out: the detector's output is the same.
- See `research/receipts/ix86-reconcile.md` (+ `.json`) for both runs, both phase
  splits and the verbatim commands. `CHANGELOG.md` was corrected to the current
  route's 5,745,767 / 3.11×.
