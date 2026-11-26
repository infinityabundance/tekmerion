# IX.86 — reconciling two published `arm/add_p25519` totals

- **Date:** 2026-11-26
- **Branch:** `staging` at `81f84db` (`main` and the tags `freeze-rust-closure-v6` /
  `v0.2.0` untouched; `main` happens to sit on the same commit).
- **Question:** `ix82` publishes **5,233,638** primitive inferences for
  `arm.bignum_add_p25519`; `ix83` publishes **5,745,767** for the same theorem. The
  inlined support library grew by a constant `+21,789`; that does not explain a
  **+512,129** gap. Was the generalised detector emitting a more expensive aggregate?
- **Verdict, one line:** **No.** There is one aggregate route at the current commit.
  Both numbers reproduce exactly, but on different trees: `5,233,638` is the
  pre-IX.83 tree (`e36dc58`, `tools/bridge.ml` identical to `677d97e`) and
  `5,745,767` is the current tree (`81f84db`, bridge unchanged since `3d40351`).
  The aggregates themselves are **identical** (same two equations, same weights,
  same premises; construction costs **216,726** inferences on both trees). **93.2%**
  of the delta (`477,376`) is one line of the IX.83 generalisation: the aggregate
  **fact proof** changed from the carrier's aggregate (`agg :: side_hyps`) to **all**
  aggregates (`aggregates @ side_hyps`). The final linear close is bit-for-bit the
  same cost (**1,128,617** on both). The standing figure is **5,745,767 / 3.11×
  human** at `81f84db`.
- **Part B:** **not attempted, and the precondition failed.** The delta is not the
  12 branches: the branch count is 12 on both trees and the close itself is
  identical. Per-branch oracle work is 2,834,839 of 5,745,767 (49.3%); the measured
  breakdown and an optimistic fold bound are in §6, but no fold was built and no
  folded number is claimed.

---

## 1. What "both routes" means at this commit

`git log -- tools/bridge.ml` ends at `3d40351` (IX.83). IX.83 **generalised the
shared close in place**; it did not add a second entry point. At `81f84db`:

- `src/tools/emit.rs` has one gate, `ALGEBRAIC_CLOSE_TASKS =
  ["arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT"]`, and one entry point,
  `BRIDGE_ADD_REF_ALG_TAC`;
- there is no specialised detector function and no separate specialised gate to
  select;
- the last bridge change is `3d40351`; `git diff 3d40351 HEAD -- tools/bridge.ml`
  is empty.

So "both paths" are **the same path at two trees**: the specialised-era tree
`e36dc58` (the IX.82 verification commit; its `tools/bridge.ml` is identical to the
aggregate commit `677d97e`) and the current general tree `81f84db`. Both were run
today with the same harness, kernel and cap.

## 2. The two runs

`--mode phase`, `.bench/hol-light-stamp2`, `tools/cap.sh 12G -- make -j1 -B`, one at
a time. The current-tree runs use `.bench/ix69/run_task.sh`; the historical tree was
extracted with `git archive e36dc58` and run with `.bench/ix86/run_old.sh` (identical
steps, rooted in the reconstructed tree).

| tree | bridge | route | total primitive inferences | SAT closes | oracle branches | wall |
|---|---|---|---:|---:|---:|---:|
| `e36dc58` (specialised-era) | `677d97e` tree | aggregate close (opt-in) | **5,233,638** | 0 | 12 | 28 s |
| `81f84db` (general) | `3d40351` tree | aggregate close (opt-in) | **5,745,767** | 0 | 12 | 30 s |

Both reproduce the published numbers **exactly**, and both have zero
`bridge:pre-bitblast`, zero `bridge:sat-done`, zero `bridge:pre-replay` marks.
`STAMP_INF_BEFORE` is `67,646,907` on both: the preamble is the same proof; the
difference is entirely in the close.

## 3. Phase split, side by side

Phase markers: IX.66's `bridge:algebra` (entry to `BRIDGE_ALGEBRA_TAC`) and
`bridge:algebra-arith` (entry to `BRIDGE_ORACLE_TAC`), plus IX.86's four
measurement-only markers inside the oracle (`pre-agg`, `post-agg`, `post-flat`,
`post-facts`, `pre-close`, `post-close`). All are inert: the totals are unchanged
with and without them (5,233,638 and 5,745,767 both re-measured).

| segment | what it contains | `e36dc58` | `81f84db` | Δ |
|---|---|---:|---:|---:|
| preamble | before the first `bridge:algebra` | 2,504,581 | 2,527,490 | **+22,909** |
| normalisation + split | `algebra → algebra-arith`, 4 invocations | 346,004 | 357,572 | **+11,568** |
| oracle entry | `algebra-arith → pre-agg` | 0 | 0 | 0 |
| aggregate construction | `pre-agg → post-agg` (both aggregates) | 216,726 | 216,726 | **0** |
| flatten integer hyps | `post-agg → post-flat` | 234,904 | 234,904 | **0** |
| **aggregate fact proof** | `post-flat → post-facts` | **701,306** | **1,178,682** | **+477,376** |
| close assembly | `post-facts → pre-close` | 75,910 | 75,910 | **0** |
| linear close | `pre-close → post-close` | 1,128,617 | 1,128,617 | **0** |
| return to next branch | `post-close → algebra` | 6,976 | 7,020 | +44 |
| tail | last `post-close → STAMP_INF_AFTER` | 18,614 | 18,846 | +232 |
| **total** | | **5,233,638** | **5,745,767** | **+512,129** |

Every segment sums to the stamped total (parsers check this; diff 0). The
attribution of the +512,129:

| cause | inferences | share |
|---|---:|---:|
| aggregate fact proof takes all aggregates instead of the carrier's | **477,376** | **93.21%** |
| preamble (shared-library/close-set growth) | 22,909 | 4.47% |
| normalisation | 11,568 | 2.26% |
| branch return + tail | 276 | 0.05% |

The answer to "branch count, bit-blast/clausify, replay, or normalisation?" is
**none of those**: there are 12 oracle branches on both trees; bit-blast,
clausification and replay are **0** on both (no SAT close); the final linear close
is **1,128,617 on both**; and normalisation accounts for only 2.3% of the delta.

The antecedent sets are also identical: 53/54 antecedents per close, with sizes
`15005, 17494, 15531, 18544, 17497, 18547` repeated twice on both trees. The
aggregates printed are the same two equations with the same weights and the same
four premises on both trees (`AGGREGATE limbs=4 base=18446744073709551616
weights=[1,B,B²,B³] premises=4`, twice per branch, 24 times).

## 4. Where the 477,376 goes, exactly

`bridge_aggregate_facts` proves each top carry from a premise list. In the IX.82
tree that list was **one** aggregate — the one mentioning the carrier:

```
-  let prove_zero_lt (v, agg) =
-    let premises = agg :: side_hyps in
```

IX.83 changed it to **all** aggregates, because `sub`'s correction borrow `c14` is
decided by both the raw and the corrected aggregate:

```
+  let prove_zero_lt v =
+    let premises = aggregates @ side_hyps in
```

For `add`, the carrier is `carry_s8`, which only needs the raw aggregate; the
corrected aggregate is inert logically, but it is a large integer equation and it
is now an antecedent of the `INT_ARITH` fact proof in every branch. That is the
whole cost.

Two measurements make this attribution checkable rather than a story:

1. **The premise sets are identical.** A phase-only print of the fact premises
   (`FACT_PREM hyps=… size=… aggs=…`) gives, on both trees and in the same order:
   `50/9736, 51/12225, 50/10262, 51/13275, 50/12228, 50/13278` (then repeated), with
   `aggs=2` in both. So `side_hyps` did not change; only the aggregate list in
   front of it did (one entry vs two).
2. **The aggregate construction did not change.** `pre-agg → post-agg` is
   **216,726** on both trees; the `AGGREGATE` lines are identical; the weights are
   derived, not hard-coded, and are the same. The suspicion that the generalised
   detector emits a more expensive aggregate is therefore **false**: the detector's
   output is the same. The cost is in the fact proof's antecedent list.

The remaining `+22,909` preamble is the inlined support-library growth plus the
normalisation additions (`GSYM VAL_EQ_0`, `BRIDGE_BIT_MSB_CONV`) measured in the
same tree; it is the same order as the `+21,789` constant `ix83` identified.

## 5. Reconciliation: the number that stands

- **Standing at `81f84db` (route: general aggregate close, gate
  `TEKMERION_ALG_CLOSE=arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT`):
  5,745,767 primitive inferences, 0 SAT closes, 12 branches, 30 s.
  That is 19.41× below the current default sliced SAT route (111,523,776) and
  **3.11×** the human `_CORRECT` body (1,848,494).**
- **5,233,638 is the `e36dc58` figure for the same route before IX.83's fact-premise
  change**, 21.30× below the `e36dc58` default and **2.83×** human. It is not the
  current figure and must not be quoted as it.
- Both receipts were *right for their commits*; the defect was quoting them as if
  they were the same tree. `CHANGELOG.md` quoted the `e36dc58` figure for a release
  whose `add` theorem is produced by the `3d40351` route; it is corrected to
  5,745,767 / 19.41× / 3.11×.

The corrections are appended to `ix82-aggregate.md` and `ix83-generality.md`; the
original text is not rewritten. Documents outside the write set (`ix82-verify.md`,
`ix84-export-and-cap.md`, the `XIX.C*` lines of `TODO.md`) still quote 5,233,638 /
2.83×; they are scoped to `e36dc58` and are **not** corrected here (write set).

## 6. Part B — the fold: precondition failed, evidence recorded

Part B was conditional on Part A answering "branches are the cost". Part A says the
opposite: the 512,129 discrepancy is the fact-premise list; the branch count is 12
on both trees and the linear close is identical. **No selector-bit fold was built
and no folded number is claimed.**

The measurement that decides this is the per-branch decomposition of the standing
route. Of 5,745,767:

| bucket | inferences | per branch (÷12) |
|---|---:|---:|
| once-only (preamble + normalisation + tail + returns) | 2,910,928 | — |
| aggregate construction | 216,726 | 18,061 |
| flatten hypotheses | 234,904 | 19,575 |
| fact proof | 1,178,682 | 98,224 |
| close assembly | 75,910 | 6,326 |
| linear close | 1,128,617 | 94,051 |
| **per-branch oracle subtotal** | **2,834,839** | **236,237** |

An **optimistic bound** for a fold that collapsed 12 branches to one at the same
unit cost is `2,910,928 + 236,237 = 3,147,165`, i.e. **1.70×** human. This is a
bound on the saving, not a folded proof: a real single branch would combine the
twelve systems and its close would be larger, and the once-only 2.53 M preamble is
untouched. It is reported as the evidence Part B.4 asks for; it is **not** a
"matches human" result and is not used as one. The fold was stopped because its
precondition is false and because the cheapest available lever is elsewhere: on
the `add` route, reverting the fact premises to the carrier's aggregate recovers
the full 477,376 (that is a specialisation, not a fold; not implemented here — see
§11).

## 7. Controls

The folded route does not exist, so C1–C5 are reported as **not applicable to a
fold**. The controls that apply to the standing route and to the only change in
this receipt (phase-only markers) are:

- **C2 (axioms):** the `.correct` run prints
  `STAMP_NAME ETA_AX 1`, `STAMP_NAME SELECT_AX 2`, `STAMP_NAME INFINITY_AX 3` —
  the theorem's stamp is `{ETA_AX, SELECT_AX, INFINITY_AX}`, measured on today's
  run. The `_SUBROUTINE_CORRECT` derivation also runs (the `.correct` target exits
  0).
- **C4 (no solver):** zero `bridge:pre-bitblast`, zero `bridge:sat-done`, zero
  `bridge:pre-replay`, zero `ORACLE close failed`; 12 `ORACLE close ok`. No
  `cadical` invocation is reachable from the aggregate route (the shim control is
  IX.82's, not repeated here).
- **C5 (no cheating):** the only source change in this receipt is the marker table
  in `src/tools/emit.rs` and the synthetic `tests/fixtures/emit/bridge_min.ml`
  anchor block. The added OCaml is `csat_mark_at "…";` calls and one guarded
  `Printf.printf`; there is no `new_axiom`, `mk_thm`, `CHEAT_TAC`, `Obj.magic`, and
  no hypothesis was added. `deinstrument ∘ instrument = id` is re-proved by
  `cargo test`.
- **C6 (default path unchanged):** with `TEKMERION_ALG_CLOSE` unset, the same task
  measures **111,523,776** — exactly IX.83's C6 value, i.e. the default route did
  not move. The 27 ARM + 9 x86 normal answers are byte-identical in
  `cargo test --test tools_emit`.
- **C1 / C3:** inherited, not re-run. C1 (`equals_thm` against the SAT-route
  theorem) stands for `e36dc58` from IX.82; the gate changes the tactic, not the
  goal, and the current run proves the same kernel-checked statement. C3 (remove
  the aggregate → fail) is IX.82/IX.83's; no aggregate was removed here.

## 8. Instrumentation, fixtures and freeze (what moved)

`tools/bridge.ml` is **unchanged**. `src/tools/emit.rs` gained four
`MarkerEdit`s that apply only to the `--mode phase` copy:

| marker | anchor |
|---|---|
| `bridge:pre-agg` / `post-agg` | `let aggregates = bridge_aggregates eqs in` |
| `bridge:post-flat` | `let all_hyps = List.map … in` |
| `bridge:post-facts` | `let agg_facts = … in` |
| `bridge:pre-close` / `post-close` | before the final `prove`, before `ORACLE close ok` |
| `FACT_PREM …` (a guarded print) | after the `side_hyps` filter in `bridge_aggregate_facts` |

`tests/fixtures/emit/bridge_min.ml` gained the same anchors in the tracked
library's lexical form, so the table's "every anchor occurs exactly once" contract
still holds on the synthetic fixture. `--mode answer` is untouched; the answer
fixtures, `BRIDGE_DIGEST` and the `curve_check` / `instrument` pins did **not**
move. `research/receipts/freeze-closure-v7.json` was regenerated because
`src/tools/emit.rs` is in the freeze scope (75 source files, 10 runtime digests;
new digest `8990dbe33910f5d394267d473ae20bcbabfd29fca520f1fc607c3bc2033ef9da`);
no tag was moved. The committed manifest named `repo_commit c276f62` and already
had the 75-file scope (ix83's "73" is the earlier `e36dc58` regeneration), so the
only source digest change caused by IX.86 is `src/tools/emit.rs`.

The committed manifest was **stale** at HEAD: the regenerated diff also records the
`Cargo.toml` / `Cargo.lock` digest change from the 0.2.0 release bump and the
`src/tools/proof_object.rs` digest from IX.84 (`cfa9322`). Those files were not
edited by IX.86 — `git status` shows no change to them, and `git diff HEAD --
  src/tools/proof_object.rs` is empty. Recording the true HEAD tree is the point of
the regeneration; the alternative was to leave a manifest that does not describe the
tree it ships with.

## 9. Environment

```
$ free -g | head -2
               total        used        free      shared  buff/cache   available
Mem:             125          18          26           1          80         107
$ df -h /mnt/1tb_kingston | tail -1
/dev/nvme1n1p1  1.3T  916G  346G  73% /mnt/1tb_kingston
```

Every proof build used `tools/cap.sh 12G -- make -j1 -B`, one at a time; every
`cargo` used `tools/cap.sh 20G`. No cap fired.

## 10. Commands (verbatim, decisive)

```
$ TEKMERION_ALG_CLOSE=arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT \
    bash .bench/ix69/run_task.sh arm curve25519 add p25519 .bench/ix86/head-alg
RUN arm_add_p25519 rc=0 phase_marks=29
STAMP_INF_BEFORE 67646907
STAMP_INFERENCES 5745767
Running time: 30.000000 sec

$ TEKMERION_ALG_CLOSE=arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT \
    bash .bench/ix69/run_task.sh arm curve25519 add p25519 .bench/ix86/head-fine2
RUN arm_add_p25519 rc=0 phase_marks=101
STAMP_INFERENCES 5745767          # markers inert
Running time: 29.000000 sec

$ git archive e36dc58 | tar -x -C .bench/ix86/old-src
$ ln -s /mnt/1tb_kingston/Tekmerion/.bench .bench/ix86/old-src/.bench
$ ( cd .bench/ix86/old-src && tools/cap.sh 20G -- cargo build --release --offline )
$ TEKMERION_ALG_CLOSE=arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT \
    bash .bench/ix86/run_old.sh arm curve25519 add p25519 .bench/ix86/old-alg
RUN arm_add_p25519 rc=0 phase_marks=29
STAMP_INF_BEFORE 67646907
STAMP_INFERENCES 5233638
Running time: 28.000000 sec

$ TEKMERION_ALG_CLOSE=arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT \
    bash .bench/ix86/run_old.sh arm curve25519 add p25519 .bench/ix86/old-fine2
RUN arm_add_p25519 rc=0 phase_marks=101
STAMP_INFERENCES 5233638          # markers inert
Running time: 28.000000 sec

$ env -u TEKMERION_ALG_CLOSE \
    bash .bench/ix69/run_task.sh arm curve25519 add p25519 .bench/ix86/c6-default
RUN arm_add_p25519 rc=0 phase_marks=37
STAMP_INFERENCES 111523776        # C6, unchanged default path
Running time: 42.000000 sec

$ awk -f .bench/ix86/parse_fine2.awk .bench/ix86/head-fine2/arm_add_p25519.correct
# the §3 table; check=5745767 (total 5745767, diff 0)
$ awk -f .bench/ix86/parse_fine2.awk .bench/ix86/old-fine2/arm_add_p25519.correct
# the §3 table; check=5233638 (total 5233638, diff 0)

$ tools/cap.sh 4G -- cargo run --offline --quiet --bin freeze -- manifest \
    --runtime --hol .bench/hol-light-stamp2 \
    --out research/receipts/freeze-closure-v7.json --tag freeze-rust-closure-v7
wrote research/receipts/freeze-closure-v7.json: 75 source files, source_digest 8990dbe33910f5d394267d473ae20bcbabfd29fca520f1fc607c3bc2033ef9da, 10 runtime digests, repo_commit 81f84db
$ tools/cap.sh 4G -- cargo run --offline --quiet --bin freeze -- verify \
    --manifest research/receipts/freeze-closure-v7.json
OK: source closure verified: 75 files, source_digest 8990dbe33910f5d394267d473ae20bcbabfd29fca520f1fc607c3bc2033ef9da
OK: effective configuration verified: 6 TEKMERION_* variables
```

## 11. What is not established

- **No folded proof.** Part B was stopped on its precondition; 3,147,165 is an
  optimistic bound, not a measurement of a fold.
- **No fast path was added.** The 477,376 is recoverable in principle by proving
  the fact from the aggregate(s) that mention the carrier and falling back to all
  aggregates when that fails; it was not implemented, so no claim is made about
  `sub`/`neg` under it.
- **C1 is inherited, not re-run** for the current tree; it is a statement check and
  the statement is unchanged.
- **The `sub`/`neg` totals were not re-measured** here; they are IX.83's numbers at
  `3d40351` and are untouched by this change (no bridge change).
- **Stale figures elsewhere.** `ix82-verify.md`, `ix84-export-and-cap.md` and the
  `XIX.C*` TODO lines still quote 5,233,638 / 2.83×; they are scoped to `e36dc58`
  and are outside this write set.
- All numbers are from this host, `.bench/hol-light-stamp2`, and the commits named
  in the tables.
