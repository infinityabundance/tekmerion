# IX.83 — does the aggregate mechanism generalise? `sub`, `neg`, and what stops `x86/add` and `p256/add`

- **Date:** 2026-11-26
- **Branch:** `staging` (parent `e36dc58`; `main` and tag `freeze-rust-closure-v6` at `cb054bd` untouched)
- **Change under test:** `tools/bridge.ml` (the shared close set, the top-bit mask split, the zero-test
  fold, the general MSB bound, shared sub/neg cores and their algebraic entry points) and
  `src/tools/emit.rs` (the per-operation gate and the x86 pass-through). Gate unchanged:
  `TEKMERION_ALG_CLOSE=<task id>`.
- **Verdict in one line.** The mechanism is **shared and it generalises across instruction shape on one
  architecture**: `arm.bignum_sub_p25519` closes solver-free in **9,029,510** primitive inferences and
  `arm.bignum_neg_p25519` in **5,683,733**, with the same chain/weight/close machinery as
  `arm.bignum_add_p25519` (**5,745,767**, re-measured). It is **not** general across architectures or
  curves yet: `x86.bignum_add_p25519` reaches the oracle and closes its first branch, then drives
  `INT_ARITH` past the 12 GB cap; `arm.bignum_add_p256` emits **no aggregate at all** (its accumulator
  equations do not present the disjoint carry paths the detector requires). `arm.bignum_add_p384` was
  not attempted (budget).

Everything below is measured on this host at this commit; every command is in §9.

---

## 1. Measurements

`--mode phase`, `tools/cap.sh 12G -- make -j1`, `.bench/hol-light-stamp2`.

| task | route | primitive inferences | SAT closes | wall | oracle branches |
|---|---|---:|---:|---:|---:|
| `arm.bignum_add_p25519` | aggregate (opt-in) | **5,745,767** | 0 | 30 s | 12 |
| `arm.bignum_sub_p25519` | aggregate (opt-in) | **9,029,510** | 0 | 34 s | 24 |
| `arm.bignum_neg_p25519` | aggregate (opt-in) | **5,683,733** | 0 | 27 s | 10 |
| `x86.bignum_add_p25519` | aggregate (opt-in) | **capped at 12 GB** | 0 in the completed branch | — | 1 closed, then allocation failure |
| `arm.bignum_add_p256` | aggregate (attempted; not in the closing list) | no theorem (`aggregates=0`) | 0 | — | 0 |

Comparisons (measured with this `tools/bridge.ml`; the SAT route counts include a constant +21,789
shared-library load, see C6):

| task | default (SAT) | aggregate | ratio | human `_CORRECT` |
|---|---:|---:|---:|---:|
| add p25519 | 111,523,776 | 5,745,767 | **19.41×** | 1,848,494 → **3.11×** |
| sub p25519 | 87,971,295 | 9,029,510 | **9.74×** | not measured |
| neg p25519 | 33,910,447 | 5,683,733 | **5.97×** | not measured |

The IX.82 sliced-SAT and full-SAT figures for the add family (111,501,987 / 119,194,788) are unchanged
as *proof* costs; the +21,789 is the inlined-library delta described in C6.

`STAMP_INF_BEFORE` is the same (67,646,907 for add, 67,670,291 for sub, 67,649,313 for neg) on both
routes: the difference is the close, not the preamble.

---

## 2. The aggregates, and where their coefficients come from

The chain reader is unchanged from IX.82: limb equations are integer equations carrying at least one
`&(bitval c)` and one non-carry atom; two are linked when they share a carry atom, the link pointing
from magnitude `B > 1` to magnitude `1`; a chain is a disjoint directed path; its weights are
`1, B, B², …`; the sum is proved by `AP_TERM`/`MK_COMB` + `INT_RING`. **Nothing here names a curve, a
width, a limb count or an instruction.** `B` and the weight list are read from the terms.

### 2.1 `arm.bignum_sub_p25519`

```
AGGREGATE limbs=4 base=18446744073709551616 weights=[1,B,B²,B³] premises=4
conclusion=(((((((&(val sum_s11) + -- &1 * &(val sum_s3)) + B*&(val sum_s12)) +
              -- B*&(val sum_s4)) + B²*&(val sum_s13)) +
              -- B²*&(val sum_s7)) + -- B^4*&(bitval carry_s14)) +
              B³*&(val sum_s14)) + -- B³*&(val sum_s8) = -- &19
AGGREGATE … conclusion=raw: W_raw − (M − N) = B^4        (borrow form: the branch fact
                                                        `carry_s8` is decided before the sum)
```

i.e. `W_cor = W_raw − 19 + B^4·c14` (the corrected chain **subtracts** the constant; the last
instruction also ANDs the top limb). The difference from the add is structural, not a copied shape:
the corrected chain is a borrow chain with a `−19` constant and a final `word_and`, and `c14` is the
*borrow out of the correction*, so it is decided by **both** aggregates — the fact proof now takes all
constructed aggregates as premises (`bridge_aggregate_facts`), not only the one that mentions the
carrier.

The `word_and` is the instruction-specific correction: `BRIDGE_AND_MASK_FACTS_TAC` recognises
`val(word_and x (word(2^(w−1)−1)))`, case-splits `val x < 2^(w−1)`, and assumes the proved equation
`&(val mask) = &(val x)` or `&(val mask) + &(2^(w−1)) = &(val x)` together with the matching integer
bound. The two directions are `BRIDGE_AND_TOP_MASK_LOW/HIGH`, proved once from
`VAL_WORD_AND_MASK_WORD` + `MOD_CASES` (§5.2). The mask split is why the branch count doubles
(12 → 24: the residue carries the masked limb in every branch).

### 2.2 `arm.bignum_neg_p25519`

```
AGGREGATE limbs=4 base=18446744073709551616 weights=[1,B,B²,B³] premises=4
conclusion=(((((&(val sum_s4) + &(val n_0)) + B*&(val sum_s6) + B*&(val n_1)) + B²*&(val sum_s9)
              + B²*&(val n_2)) + -- B^4*&(bitval carry_s12)) + B³*&(val sum_s12) + B³*&(val n_3)
            = p_25519
```

i.e. `W + n − B^4·c12 = p`: the negation's minuend is the complement block, so there is **one**
chain (no separate raw/corrected pair) and `carry_s12` is decided as `0`. The instruction-specific
part is the reference's zero guard against the machine's OR test: `word_join n = word 0` and
`word_or … = 0` are the same fact, which BITBLAST does not connect. `BRIDGE_ZERO_TEST_TAC` folds both
to limb-wise conjunctions; `BRIDGE_ZERO_FOLD_TAC` (inside the algebra, after normalisation) rewrites
the machine's *integer* form `&(val(word_or …)) = &0` to `(&(val l) = &0 ∧ &(val r) = &0)` via the
proved `INT_VAL_WORD_OR_EQ_0`. Without that fold the impossible select branch is not refutable (the
measured first failure: `linear_ineqs: no contradiction` on the branch with `val(or n) ≠ 0` and
`n = 0`).

### 2.3 `arm.bignum_add_p25519` (re-measured, IX.82's mechanism)

Same two aggregates as IX.82 (raw and corrected, `weights=[1,B,B²,B³]`); the number moves from
5,233,638 to **5,745,767** because the shared close set changed: the fact proof now uses all
aggregates, non-chain equalities are retained, and the zero-fold/mask machinery runs (inert here but
it is still inferences). The conclusion text is the IX.82 one; the moved value is listed in §7.

---

## 3. What is shared, and what is instruction-specific

**Shared (computed per theorem, no per-task code):**
- chain detection, base `B`, weight list `1, B, B², …`;
- the weighted-sum proof (`AP_TERM`/`MK_COMB` + `INT_RING`) and the flat normalisation;
- the top-carry fact (`aggregate-band`), now over *all* aggregates;
- the close set: aggregates + facts + non-equational bounds + **every integer equality that is not a
  carry-chain row** (`bridge_int_eq_has_carry`), so branch facts such as a folded zero test or a mask
  relation reach the prover;
- the shared `bridge_sub_ref_core` / `bridge_neg_ref_core`: the SAT and algebraic routes differ only
  in `finish`, so their statements cannot drift.

**Instruction/route-specific, each named:**
1. **Subtraction** (`sub`): borrow chains with a subtractive constant (`−19`), a final `word_and`, and
   a correction borrow `c14` decided jointly by both aggregates (hence the all-aggregates premises).
2. **Top-bit mask** (`sub`, `neg`, x86 add): `BRIDGE_AND_MASK_FACTS_TAC` + `BRIDGE_AND_TOP_MASK_LOW/HIGH`
   (`VAL_WORD_AND_MASK_WORD`, `MOD_CASES`). A disjunction was tried first and **looped** in
   `BRIDGE_SPLIT_INT_DISJ_TAC` (which leaves the hypothesis in place); the case-split-plus-equation
   form replaced it.
3. **Zero test** (`neg`): `WORD_JOIN_EQ_0`, `WORD_OR_EQ_0`, `VAL_WORD_OR_EQ_0`, `INT_VAL_WORD_OR_EQ_0`;
   the algebraic fold runs *after* the split (the guard is substituted into the machine's condition
   only then) and *before* the bit/mask work.
4. **Top-bit test** (x86 add): `bit (w−1) x` appears as the correction condition; `BRIDGE_BIT_MSB_CONV`
   rewrites it with the reduced instance of `MSB_VAL`. The reduction is done on the *num* form and by
   two explicit `NUM_REDUCE_CONV` instances (`64−1`, `2 EXP 63`) because `DEPTH_CONV` over the
   polymorphic `MSB_INT_VAL`/`MSB_VAL` recurses without bound (measured: stack overflow).

No curve name, width, limb count or instruction appears in any of the generic code; the specific
parts are correction blocks identified structurally (a mask atom, an OR zero test, a top-bit test).

---

## 4. The two that did not close

### 4.1 `x86.bignum_add_p25519` — capped, not closed

The x86 add is the same mathematics but its correction is a full-width
`word_sub (word_add M N) p` guarded by `bit 63`, not a per-limb correction. The chain reader finds
both x86 chains with `weights=[1,B,B²,B³]` **from the x86 carries** (not ARM's), and the mask split
and the `bit` bound were added so the normalise stage reaches the oracle. The run then closes its
first branch and dies on the next:

```
ORACLE close ok
Fatal error: allocation failure during minor GC
cap.sh: group=… rc=134 sampled_group_peak_rss_kb=12305456 …
```

`RLIMIT_AS` was 12 GB; the peak was 12.31 GB. This is a **fired cap**, recorded as `capped`, not
retried higher. The residual is the correction-plus-mask branch's `INT_ARITH` call
(`ORACLE extra-eqs=0`, both aggregates and both facts present, `ants` in the 50s): the linear prover,
not the aggregate construction, is the wall. The entry point is generic (`ref_entry` maps a gated
`add` to `BRIDGE_ADD_REF_ALG_TAC`), but `x86.bignum_add_p25519` is **not** in
`ALGEBRAIC_CLOSE_TASKS`: the shipped opt-in set is only tasks that close.

### 4.2 `arm.bignum_add_p256` — no aggregate is emitted

```
MASK atoms=0 []
ORACLE aggregate-carriers=0 []
ORACLE system defs=8 residual=1 facts=0 aggregates=0
ORACLE close ants=54 size=9926
ORACLE close failed: Failure("INT_ARITH …: linear_ineqs: no contradiction")
```

The detector emits nothing: p256's accumulator equations do not present disjoint
`(outgoing B, incoming 1)` carry paths, so `bridge_aggregates` returns `[]` and the close falls back
to IX.78's behaviour, which fails on the residual. The residual (verbatim, first lines):

```
ALG_FAIL stage=oracle goal=((B * (B * (B * &(val m_3) + &(val m_2)) + &(val m_1)) + &(val m_0)) +
  B * (B * (B * &(val n_3) + &(val n_2)) + &(val n_1)) + &(val n_0)) -
 &115792089210356248762697446949407573530086143415290314195533631308867097853951 =
 B * (B * (B * &(val sum_s8) + &(val sum_s7)) + &(val sum_s4)) + &(val sum_s3)
```

This is a real generality limit of the *detector*, not of the linear close. `arm.bignum_add_p384`
was not attempted (budget); on this evidence it would fail the same way (the p384 proof also uses
compare-and-mask accumulator equations), so no number is reported for it. Neither p256 nor p384 is
in `ALGEBRAIC_CLOSE_TASKS`: a gate that produced an unprovable answer would be worse than no gate.

**Not a failed route's partial cost:** no inference total is claimed for either task.

---

## 5. Controls

### C1 — statement identity (weaker check, stated explicitly)

Both routes still produce a theorem for the three closing tasks. For `add`, `sub` and `neg` the
`--mode answer` artefacts with and without the gate were diffed. The **only** differing lines are
the close entry point:

```
$ diff add_sat.answer add_alg.answer
< BRIDGE_ADD_REF_TAC (CONV_TAC (ONCE_DEPTH_CONV BIGNUM_OF_WORDLIST_CONV)) p_25519) g
> BRIDGE_ADD_REF_ALG_TAC (CONV_TAC (ONCE_DEPTH_CONV BIGNUM_OF_WORDLIST_CONV)) p_25519) g
$ diff sub_sat.answer sub_alg.answer
< BRIDGE_SUB_REF_TAC …
> BRIDGE_SUB_REF_ALG_TAC …
$ diff neg_sat.answer neg_alg.answer
< BRIDGE_NEG_REF_TAC …
> BRIDGE_NEG_REF_ALG_TAC …
```

This is the **weaker** C1: identical goal and byte-identical printed statement, because the two
artefacts share every line except the entry point. The strong `equals_thm` check is the IX.82 one for
`arm.bignum_add_p25519` (`true`, both conclusions alpha-equivalent, both zero hypotheses); it was not
re-run for the new tasks in this budget, and is reported as not measured for them.

### C2 — axioms

The algebraic artefacts exit 0 through the build template's `check_axioms ()` and are stamped with
the three permitted axioms:

```
STAMP_NAME ETA_AX 1
STAMP_NAME SELECT_AX 2
STAMP_NAME INFINITY_AX 3
```

(present in all three ALG run logs).

### C3 — remove the aggregate: the close fails

With `let aggregates = []` in the emitted `bignum_sub_p25519.ml` (only change; the proof is otherwise
the ALG answer), the run produces 6 closes and then fails, and no theorem:

```
RUN arm_sub_p25519 rc=2 closes_ok=6
ORACLE close failed: Failure("INT_ARITH `&(val sum_s14) < &9223372036854775808 ==> &0 <= &(val (word_and sum_s14 (word 9223372036854775807))) …`: linear_ineqs: no contradiction")
```

The aggregate is load-bearing. (The same `bridge_aggregates` function is the only source of
aggregates for all three tasks.)

### C4 — no solver

A failing `cadical` shim is first on `PATH`; the marker is removed before each run. All three ALG
runs close with **the same inference totals** and the marker is absent:

```
add: STAMP_INFERENCES 5745767   marker absent
sub: STAMP_INFERENCES 9029510   marker absent
neg: STAMP_INFERENCES 5683733   marker absent
```

### C5 — no cheating

```
$ grep -nE 'new_axiom|mk_thm|CHEAT_TAC|Obj\.magic' tools/bridge.ml src/tools/emit.rs tests/fixtures/emit/bridge_min.ml
tools/bridge.ml:1003:(* procedure only; no new axioms, no CHEAT_TAC), and the same linear system  *)
```

One hit, a pre-existing comment. No hypothesis is added: C1's artefacts differ only in the entry
point; the closes `MP` their antecedents away.

### C6 — the default path is unchanged

The gate is unset by default, so every task emits the tactic it had. The emitted SAT tactic line is
byte-identical (C1's `<` lines are the shipped ones). The **total** inference count moves by exactly
**+21,789** for every measured task, because `tools/bridge.ml` is inlined into every answer and the
new shared definitions are loaded before the theorem:

```
arm.bignum_add_p25519  STAMP_INFERENCES 111,523,776   (IX.82: 111,501,987; +21,789)
arm.bignum_add_p256    STAMP_INFERENCES 118,918,603   (IX.82: 118,896,814; +21,789)
arm.bignum_neg_p25519  STAMP_INFERENCES  33,910,447   (IX.82:  33,888,658; +21,789)
arm.bignum_sub_p25519  STAMP_INFERENCES  87,971,295   (not pinned in IX.82)
```

The constant delta across three tasks is the evidence that the *proof* is unchanged and only the
shared library text grew. This is stated plainly because it is a moved value, not a silent one.

---

## 6. What is shared vs instruction-specific — the verdict for `XIX.D`

The shared mechanism is real: it closes three different instruction shapes (add, subtract, negate)
at one width with no per-task template, and the weights are derived from each theorem's equations.
It does **not** yet generalise across architectures or curves:

- x86's full-width subtract-plus-bit-test correction is represented, but one branch exceeds the
  linear prover's practical budget (12 GB cap);
- p256's accumulator presentation yields no aggregate, so the detector is the blocker there, not the
  arithmetic.

Therefore the next investment is **`XIX.D`** (a certificate representation — `(premises, weights,
conclusion)` plus the instruction-specific side facts — that the chain reader can be extended
against), not a new per-curve template. The per-curve templates would encode exactly the shapes the
detector failed on, which is the thing the exercise is trying to avoid.

---

## 7. Fixtures and goldens: what moved, and why

`tools/bridge.ml` changed, so every inlined answer changed. Re-recorded deliberately with
`TEKMERION_RECORD_ANSWERS=1 cargo test --offline --test tools_emit`. The pins that moved:

| pin | was | now |
|---|---|---|
| `BRIDGE_DIGEST` (`tests/tools_receipt.rs`) | `65ced317…` | `070ed5b4de5f2c42598c9cd9c5cc14a16603b2a80d10ce8c9e80c010b2885448` |
| `curve_check` statements | 86 | 91 |
| `instrument` phrases | 232 | 249 |
| `instrument` output digest | `fb1677dc…` | `f84b7582f82f4ea43d4f2b40985f6f1779705dffd75d67367517ca05f2fb5715` |
| `inline_answer` phrases | 276 | 293 |
| inline `add-ref` | `a930a4e6…` | `55920f3e4b9cc42d075c22be8d3ad156c20dd2ff191e7d49882a357c8984ee2c` |
| inline `sub` | `eb3215a8…` | `78a05032259a798c035adb9a6b8bd5926bb11ade48835456025cbf891048b50c` |
| inline `neg` | `e957d506…` | `f67d0350a4f1ebab2af9e1c62e841a71d8d24188c3f91b8268c531b2dfb18cbe` |
| inline `add-p384` | `5fad9abe…` | `a50237816f2236d585ae80e0a467d3f1e7eff927bc2d8118210e55c6072fbc6d` |
| `tests/fixtures/answers/*` | — | 27 ARM + 9 x86 files re-recorded |
| `tests/fixtures/curve_check_bridge.txt` | — | regenerated (91 statements, 0 hits) |
| `tests/fixtures/emit/bridge_min.ml` | — | sub/neg anchors follow the shared cores |
| `research/receipts/freeze-closure-v7.json` | `source_digest 9b907ddc…`, `repo_commit a823a0a` | 73 files, `source_digest cde1f84f15d03da09b4a4ad3a3d328e32c0f63282a2273376111a68c807fe793`, `repo_commit e36dc58…` |

The `freeze-rust-closure-v6` tag at `cb054bd` was not touched.

---

## 8. The residuals, verbatim

- `x86/add`: `Fatal error: allocation failure during minor GC` after
  `cap.sh group=… rc=134 sampled_group_peak_rss_kb=12305456` (12 GB `RLIMIT_AS`), one `ORACLE
  close ok` before it.
- `p256/add`: `aggregates=0`, `residual=1`, and the `ALG_FAIL` goal quoted in §4.2, with
  `INT_ARITH … linear_ineqs: no contradiction`.
- `sub` with the aggregate removed: the `ALG_FAIL` in §C3.
- Every closing branch reports `ORACLE close ok`; there is no `ORACLE close failed` in any of the
  three closing runs.

---

## 9. Commands (verbatim, representative)

```
$ free -g | head -2
               total        used        free      shared  buff/cache   available
Mem:             125          15          51           1          58         110
$ df -h /mnt/1tb_kingston | tail -1
/dev/nvme1n1p1  1.3T  902G  360G  72% /mnt/1tb_kingston

$ TEKMERION_ALG_CLOSE=arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT bash .bench/ix83/run.sh arm curve25519 add p25519 .bench/ix83/add1
STAMP_INF_BEFORE 67646907
STAMP_INF_AFTER 73392674
STAMP_INFERENCES 5745767
Running time: 30.000000 sec

$ TEKMERION_ALG_CLOSE=arm.bignum_sub_p25519.BIGNUM_SUB_P25519_CORRECT … sub p25519
STAMP_INFERENCES 9029510        Running time: 34.000000 sec
$ TEKMERION_ALG_CLOSE=arm.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT … neg p25519
STAMP_INFERENCES 5683733        Running time: 27.000000 sec

$ env -u TEKMERION_ALG_CLOSE … add p25519    # C6
STAMP_INFERENCES 111523776

$ tools/cap.sh 20G -- cargo test --offline          # all targets pass
$ cargo clippy --offline --all-targets -- -D warnings   # clean
$ cargo fmt --check                                  # clean
$ git ls-files '*.py'                                # empty
```

`tools/cap.sh` was used for every proof build `12G -- make -j1`, one target at a time; cargo used
`20G`. No run other than the x86 add exceeded its cap.

---

## 10. What is not established

- **Not a strong statement check for the new tasks.** C1 is the weaker "same goal, byte-identical
  statement" check here; `equals_thm` was not re-run for `sub`/`neg` in this budget (it stands for
  `add` from IX.82).
- **Human figures for `sub`/`neg` were not measured.** They are obtainable by the same typed
  machinery; no number is claimed.
- **x86 add is not closed.** The cap fired at 12 GB; the residual is the linear prover's, not the
  aggregate's, but no number is reported as a cost.
- **p256/p384 add are not closed.** p256 emits no aggregate; p384 was not attempted.
- **The `XIX.D` certificate is still not serialised.** The weights and premises are in the proof and
  in `ix83-generality.json`; no checker consumes them yet.
- All numbers are from this host, this kernel (`.bench/hol-light-stamp2`) and this commit.

---

## 11. Correction log — IX.86 reconciliation (2026-11-26)

Appended, not substituted.

- **The add total is confirmed.** `5,745,767` was re-run today at `81f84db` (bridge
  unchanged since `3d40351`): **5,745,767**, **30 s**, **12** branches, **0** SAT
  closes. It is the standing figure and the one `CHANGELOG.md` now carries, with
  **3.11×** human (not 2.83×).
- **§2.3's explanation of the +512,129 is corrected.** It attributed the move from
  5,233,638 to the shared close set generally ("the zero-fold/mask machinery runs
  (inert here but it is still inferences)"). Measured by phase (`ix86`), that is
  not the dominant term: aggregate construction is identical (**216,726** both), as
  are hypothesis flattening (**234,904**), close assembly (**75,910**) and the final
  linear close (**1,128,617**). **477,376 of the 512,129 (93.2%)** is
  `bridge_aggregate_facts` proving the top carry from `aggregates @ side_hyps`
  instead of the carrier's `agg :: side_hyps`. The change was necessary for `sub`
  (its `c14` is decided by both aggregates); for `add` the corrected aggregate is
  logically inert but still an antecedent of the fact `INT_ARITH` call.
- **The `add` row's 19.41× and 3.11× stand**; the 5,233,638 / 2.83× pair belongs to
  the pre-IX.83 tree (`e36dc58`, bridge `677d97e`), reproduced today at its own
  commit.
- See `research/receipts/ix86-reconcile.md` (+ `.json`). Original text above is the
  record of `3d40351`/`e36dc58`.
