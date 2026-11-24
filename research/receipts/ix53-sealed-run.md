# IX.53 — sealed x86 evaluation, one pass

Status: **measurement run only.** Branch `staging`, no commit, no push, no source
change, no re-freeze. Nine sealed tasks, emitted once, judged once, replayed once.
The orchestrator handles git.

Environment: Python 3.14.10; `cadical` 2.1.2 at
`/home/one/.elan/toolchains/leanprover--lean4---v4.29.1/bin/cadical`; opam switch
`.bench/hol-light`; HEAD `b2c20523c11acec97bf37a893d03e7f18d35359d`. Judge
directory `.bench/eval-20261124_003911`, 1 core.

## Headline

**OK, 9 of 9** (`N = 9`, every sealed task was judged; none refused, none
"not evaluated").

Caveat that travels with that number: this `OK` is the harness verdict under the
**inherited inlined context** of the benchmark program (thousands of axioms
already loaded, `axioms_before = axioms_after = 2462`), **not** an axiom-free,
dependency-closed result. See §8.

## 1. Freeze verification

```
$ python3 -c "import sys; sys.path.insert(0,'tools'); import heldout_guard as g; m=g.load_sealed_manifest('research/receipts/ix53-sealed-manifest.json'); g.verify_freeze(m,'freeze-x86-sealed-v2'); print('FREEZE PASS', m.freeze_commit[:12], len(m.sealed_ids))"
FREEZE PASS 22fc118bb275 9
```

Manifest `research/receipts/ix53-sealed-manifest.json`, tag `freeze-x86-sealed-v2`,
`freeze_commit` `22fc118bb275cc12fe54123b035dc2406edb1511`. Nine ids listed.

## 2. Emit (one pass, `.bench/x86-sealed-work/`)

Emitter stdout, verbatim, one line per task. All nine exited 0; none printed
`refused:`.

```
wrote .bench/x86-sealed-work/x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT/answer.txt mode=answer task=x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT entry=0 last=25 neg=False accumulate=[2, 4, 6, 8, 13, 14, 15, 16] bytes=103646
wrote .bench/x86-sealed-work/x86.bignum_sub_p25519.BIGNUM_SUB_P25519_CORRECT/answer.txt mode=answer task=x86.bignum_sub_p25519.BIGNUM_SUB_P25519_CORRECT entry=0 last=20 neg=False accumulate=[2, 4, 6, 8, 12, 14, 16, 18] bytes=103551
wrote .bench/x86-sealed-work/x86.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT/answer.txt mode=answer task=x86.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT entry=0 last=23 neg=True accumulate=[13, 16, 19, 21] bytes=103516
wrote .bench/x86-sealed-work/x86.bignum_add_p256k1.BIGNUM_ADD_P256K1_CORRECT/answer.txt mode=answer task=x86.bignum_add_p256k1.BIGNUM_ADD_P256K1_CORRECT entry=0 last=26 neg=False accumulate=[2, 4, 6, 8, 14, 15, 16, 19, 21, 23, 25] bytes=103655
wrote .bench/x86-sealed-work/x86.bignum_sub_p256k1.BIGNUM_SUB_P256K1_CORRECT/answer.txt mode=answer task=x86.bignum_sub_p256k1.BIGNUM_SUB_P256K1_CORRECT entry=0 last=19 neg=False accumulate=[3, 5, 7, 9, 12, 14, 16, 18] bytes=103551
wrote .bench/x86-sealed-work/x86.bignum_neg_p256k1.BIGNUM_NEG_P256K1_CORRECT/answer.txt mode=answer task=x86.bignum_neg_p256k1.BIGNUM_NEG_P256K1_CORRECT entry=0 last=22 neg=True accumulate=[13, 16, 19, 21] bytes=103516
wrote .bench/x86-sealed-work/x86.bignum_add_sm2.BIGNUM_ADD_SM2_CORRECT/answer.txt mode=answer task=x86.bignum_add_sm2.BIGNUM_ADD_SM2_CORRECT entry=0 last=27 neg=False accumulate=[3, 5, 7, 9, 11, 13, 14, 16, 17, 20, 22, 24, 26] bytes=103656
wrote .bench/x86-sealed-work/x86.bignum_sub_sm2.BIGNUM_SUB_SM2_CORRECT/answer.txt mode=answer task=x86.bignum_sub_sm2.BIGNUM_SUB_SM2_CORRECT entry=0 last=21 neg=False accumulate=[2, 4, 6, 8, 14, 16, 18, 20] bytes=103546
wrote .bench/x86-sealed-work/x86.bignum_neg_sm2.BIGNUM_NEG_SM2_CORRECT/answer.txt mode=answer task=x86.bignum_neg_sm2.BIGNUM_NEG_SM2_CORRECT entry=0 last=23 neg=True accumulate=[17, 18, 19] bytes=103508
```

Answer-tape hashes (`sha256sum .bench/x86-sealed-work/*/answer.txt`):

```
bf2a851ab6b7ac749ca1ad92f257e26c6182512d5ccbde22a3948e933cd76e26  x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT/answer.txt
e31e3bed2523bdb5b6b044424e68392fc27e72768c7f25c884bc2ab4a221d480  x86.bignum_add_p256k1.BIGNUM_ADD_P256K1_CORRECT/answer.txt
439948ca47584ea3c7a3a756b8b068506b7df1efa4c81236c6fb628f0defb48a  x86.bignum_add_sm2.BIGNUM_ADD_SM2_CORRECT/answer.txt
3e51c7db633e22d337bbf32b255754a2fd2c3b9f5ea6a9bddb925f6dfc207c3b  x86.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT/answer.txt
6e06c9abd118d402ea7585051eb78a5036a964d957a4e6b23aa84a1f04a67441  x86.bignum_neg_p256k1.BIGNUM_NEG_P256K1_CORRECT/answer.txt
c6a393eb3755ee277b5e743d339350d327d024fb949c291c4a8b97aee7d5b04a  x86.bignum_neg_sm2.BIGNUM_NEG_SM2_CORRECT/answer.txt
800886769fcb77fd2a1a301749cdcd18d22008dc3be813129bebae297e55261f  x86.bignum_sub_p25519.BIGNUM_SUB_P25519_CORRECT/answer.txt
037e21efa1e38db5085c76a93948a8e540a25f78bed18ad8bfb9a01c566ec938  x86.bignum_sub_p256k1.BIGNUM_SUB_P256K1_CORRECT/answer.txt
3b2e2ce97ce1b325104e0ebf3b2fb930e1ae8fb2dd6a555ae7c84380e206a8bb  x86.bignum_sub_sm2.BIGNUM_SUB_SM2_CORRECT/answer.txt
```

(The invocation used in §2 of the brief was repeated once per task with the
`<ID>` substituted; see §10 for the exact command shape.)

## 3. Combine + syntactic gate

```
$ cd .bench && python3 combine-answer-and-setup.py x86-sealed-work 1
Loaded per-problem timeouts for 2622 problems from timeout-map.json
x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT: syntactic check PASSED
x86.bignum_add_p256k1.BIGNUM_ADD_P256K1_CORRECT: syntactic check PASSED
x86.bignum_add_sm2.BIGNUM_ADD_SM2_CORRECT: syntactic check PASSED
x86.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT: syntactic check PASSED
x86.bignum_neg_p256k1.BIGNUM_NEG_P256K1_CORRECT: syntactic check PASSED
x86.bignum_neg_sm2.BIGNUM_NEG_SM2_CORRECT: syntactic check PASSED
x86.bignum_sub_p25519.BIGNUM_SUB_P25519_CORRECT: syntactic check PASSED
x86.bignum_sub_p256k1.BIGNUM_SUB_P256K1_CORRECT: syntactic check PASSED
x86.bignum_sub_sm2.BIGNUM_SUB_SM2_CORRECT: syntactic check PASSED
eval-20261124_003911
```

Judge programs produced: **9** (`eval-20261124_003911/ml_files_x86_*.ml`), one
per task, consistent with a full run.

## 4. Judge verdicts

`collect-verdicts.py` output CSV, verbatim (`research/receipts/ix53-sealed-verdicts.csv`):

```
Problem,Verdict
x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT,OK
x86.bignum_add_p256k1.BIGNUM_ADD_P256K1_CORRECT,OK
x86.bignum_add_sm2.BIGNUM_ADD_SM2_CORRECT,OK
x86.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT,OK
x86.bignum_neg_p256k1.BIGNUM_NEG_P256K1_CORRECT,OK
x86.bignum_neg_sm2.BIGNUM_NEG_SM2_CORRECT,OK
x86.bignum_sub_p25519.BIGNUM_SUB_P25519_CORRECT,OK
x86.bignum_sub_p256k1.BIGNUM_SUB_P256K1_CORRECT,OK
x86.bignum_sub_sm2.BIGNUM_SUB_SM2_CORRECT,OK
--,
SUMMARY,
OK,9
```

Every `*.judge.txt` (content, one line each):

```
x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT.judge.txt: OK
x86.bignum_add_p256k1.BIGNUM_ADD_P256K1_CORRECT.judge.txt: OK
x86.bignum_add_sm2.BIGNUM_ADD_SM2_CORRECT.judge.txt: OK
x86.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT.judge.txt: OK
x86.bignum_neg_p256k1.BIGNUM_NEG_P256K1_CORRECT.judge.txt: OK
x86.bignum_neg_sm2.BIGNUM_NEG_SM2_CORRECT.judge.txt: OK
x86.bignum_sub_p25519.BIGNUM_SUB_P25519_CORRECT.judge.txt: OK
x86.bignum_sub_p256k1.BIGNUM_SUB_P256K1_CORRECT.judge.txt: OK
x86.bignum_sub_sm2.BIGNUM_SUB_SM2_CORRECT.judge.txt: OK
```

All nine `.run.errlog` files are 0 bytes. No `CSAT` line appears in any
`.run.outlog`.

## 5. Per-task seconds (judge, `PROFILE` lines)

Each task's budget from `timeout-map.json` is 120 s.

| task | operator | verdict | seconds (PROFILE) | budget | `.run.outlog` |
|---|---|---|---:|---:|---|
| `x86.bignum_add_p25519` | add | OK | 22.138 | 120 | `eval-20261124_003911/ml_files_x86_bignum_add_p25519.run.outlog` |
| `x86.bignum_add_p256k1` | add | OK | 23.178 | 120 | `eval-20261124_003911/ml_files_x86_bignum_add_p256k1.run.outlog` |
| `x86.bignum_add_sm2` | add | OK | 17.939 | 120 | `eval-20261124_003911/ml_files_x86_bignum_add_sm2.run.outlog` |
| `x86.bignum_sub_p25519` | sub | OK | 9.059 | 120 | `eval-20261124_003911/ml_files_x86_bignum_sub_p25519.run.outlog` |
| `x86.bignum_sub_p256k1` | sub | OK | 15.707 | 120 | `eval-20261124_003911/ml_files_x86_bignum_sub_p256k1.run.outlog` |
| `x86.bignum_sub_sm2` | sub | OK | 9.815 | 120 | `eval-20261124_003911/ml_files_x86_bignum_sub_sm2.run.outlog` |
| `x86.bignum_neg_p25519` | neg | OK | 3.320 | 120 | `eval-20261124_003911/ml_files_x86_bignum_neg_p25519.run.outlog` |
| `x86.bignum_neg_p256k1` | neg | OK | 3.651 | 120 | `eval-20261124_003911/ml_files_x86_bignum_neg_p256k1.run.outlog` |
| `x86.bignum_neg_sm2` | neg | OK | 3.102 | 120 | `eval-20261124_003911/ml_files_x86_bignum_neg_sm2.run.outlog` |

Raw `PROFILE` lines:

```
PROFILE x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT 22.138 OK
PROFILE x86.bignum_add_p256k1.BIGNUM_ADD_P256K1_CORRECT 23.178 OK
PROFILE x86.bignum_add_sm2.BIGNUM_ADD_SM2_CORRECT 17.939 OK
PROFILE x86.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT 3.320 OK
PROFILE x86.bignum_neg_p256k1.BIGNUM_NEG_P256K1_CORRECT 3.651 OK
PROFILE x86.bignum_neg_sm2.BIGNUM_NEG_SM2_CORRECT 3.102 OK
PROFILE x86.bignum_sub_p25519.BIGNUM_SUB_P25519_CORRECT 9.059 OK
PROFILE x86.bignum_sub_p256k1.BIGNUM_SUB_P256K1_CORRECT 15.707 OK
PROFILE x86.bignum_sub_sm2.BIGNUM_SUB_SM2_CORRECT 9.815 OK
```

Sum of the nine `PROFILE` seconds = 107.909 s. Slowest = 23.178 s
(`add_p256k1`), 0.193 of its 120 s budget. No timeout.

## 6. Fresh replay (hypotheses / axiom delta)

Script `.bench/x86-sealed-replay-run.sh` is `.bench/x86-replay-run.sh` with only
lines 10–11 changed (`EVAL=eval-20261124_003911`, `OUT=x86-sealed-replay`);
`diff` shows exactly those two lines. It rebuilds the same answer bytes with the
instrumented `bench_run` from `r3_replay.py` (verdict path untouched).

REPLAY lines, verbatim:

```
REPLAY x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT closed=1 hyps=0 axioms_before=2462 axioms_after=2462 unchanged=true seconds=22.445
REPLAY x86.bignum_add_p256k1.BIGNUM_ADD_P256K1_CORRECT closed=1 hyps=0 axioms_before=2462 axioms_after=2462 unchanged=true seconds=23.239
REPLAY x86.bignum_add_sm2.BIGNUM_ADD_SM2_CORRECT closed=1 hyps=0 axioms_before=2462 axioms_after=2462 unchanged=true seconds=17.896
REPLAY x86.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT closed=1 hyps=0 axioms_before=2462 axioms_after=2462 unchanged=true seconds=3.351
REPLAY x86.bignum_neg_p256k1.BIGNUM_NEG_P256K1_CORRECT closed=1 hyps=0 axioms_before=2462 axioms_after=2462 unchanged=true seconds=3.597
REPLAY x86.bignum_neg_sm2.BIGNUM_NEG_SM2_CORRECT closed=1 hyps=0 axioms_before=2462 axioms_after=2462 unchanged=true seconds=3.089
REPLAY x86.bignum_sub_p25519.BIGNUM_SUB_P25519_CORRECT closed=1 hyps=0 axioms_before=2462 axioms_after=2462 unchanged=true seconds=9.101
REPLAY x86.bignum_sub_p256k1.BIGNUM_SUB_P256K1_CORRECT closed=1 hyps=0 axioms_before=2462 axioms_after=2462 unchanged=true seconds=15.871
REPLAY x86.bignum_sub_sm2.BIGNUM_SUB_SM2_CORRECT closed=1 hyps=0 axioms_before=2462 axioms_after=2462 unchanged=true seconds=9.588
```

Nine of nine: `closed=1`, `hyps=0`, axiom set unchanged (2462 → 2462), no
`RESIDUAL` line. Replay seconds sum = 108.177 s.

## 7. Isolation is partial (recorded plainly)

The sealed isolation here is **partial**, and the protection is procedural, not
filesystem separation:

* `tools/emit_ref_proof_x86.py` hardcodes `BENCH = REPO / ".bench"`, reading
  `BENCH / "problems.json"` for the query and `S2N = BENCH / "s2n-bignum"` for
  the ELF objects. There is no separate sealed tree and no namespace/container
  boundary. The sealed manifest is the **only** authority that widens the gate.
* Therefore the sealed set is protected by `tools/heldout_guard.py`'s refusal of
  the non-manifest task ids / object paths / proof-file reads, not by the sealed
  artefacts living somewhere the emitter cannot see. `--sealed-eval` allows
  exactly the nine manifest-listed ids and their objects and still refuses to
  open any sealed proof `.ml`.
* This run opened none of `.bench/s2n-bignum/x86/proofs/*.ml`, and disassembled
  no x86 object other than the nine sealed objects (the emitter's own
  `disasm_steps.disassemble` on those nine). Nothing under `tools/` was written.

A future run should not read the `OK` in §4 as evidence of strong isolation. It
is evidence that the guard, on this tree at this commit, permitted exactly the
nine ids and no proof text was read.

## 8. What `OK` means here (inherited inlined context)

The benchmark harness in `.bench/combine-answer-and-setup.py` splices each answer
into a 1.57 MB inlined program that has already loaded the s2n-bignum x86 base
(`needs "x86/proofs/base.ml"`, definitions, lemmas). The judge's `bench_run`
compares `axioms()` before and after the proof:

```
let axioms_before = axioms() in
... prove(query, tac) ...
let axioms_after = axioms() in
if axioms_before = axioms_after then "OK" else "CHEATING"
```

That comparison is against **2462 already-loaded axioms** (the replay reports the
same 2462 before and after). So:

* `OK` means the tactic closed the goal and **added no new axiom** relative to the
  inherited context.
* `OK` is **not** an axiom-free, dependency-closed result. The proof depends on
  the thousands of theorems and axioms already present in the loaded x86 base.
  No closure over dependencies is claimed, and no independent re-check from an
  empty kernel was performed.

Both facts (inherited context; no axiom delta) must be quoted together; the
`OK/N` in §Headline is the first without being the second.

## 9. Hygiene

```
$ git --no-optional-locks status --porcelain -- tools
<empty>
```

Nothing under `tools/` is modified, created or deleted. The only tracked writes
from this run are this receipt and `research/receipts/ix53-sealed-verdicts.csv`.
All scratch lives under git-ignored `.bench/`
(`.bench/x86-sealed-work/`, `.bench/eval-20261124_003911/`,
`.bench/x86-sealed-replay/`, `.bench/x86-sealed-verdicts.csv`). Pre-existing
`.bench/x86-work/` and `.bench/sealed-run/` were not touched.

Disk: 23 GB free before the run, above the 3 GB floor; no large tree was copied.

## 10. Commands run (verbatim)

Freeze:

```
cd /mnt/1tb_kingston/Tekmerion
python3 -c "import sys; sys.path.insert(0,'tools'); import heldout_guard as g; m=g.load_sealed_manifest('research/receipts/ix53-sealed-manifest.json'); g.verify_freeze(m,'freeze-x86-sealed-v2'); print('FREEZE PASS', m.freeze_commit[:12], len(m.sealed_ids))"
```

Emit (once per `<ID>`, nine times; `<ID>` one of the nine sealed ids):

```
cd /mnt/1tb_kingston/Tekmerion
python3 tools/emit_ref_proof_x86.py \
  --sealed-eval \
  --sealed-manifest research/receipts/ix53-sealed-manifest.json \
  --freeze-tag freeze-x86-sealed-v2 \
  --task <ID> --mode answer \
  --out .bench/x86-sealed-work/<ID>/answer.txt \
  --accumulate-out .bench/x86-sealed-work/<ID>/accumulate.txt
```

Combine and judge:

```
cd /mnt/1tb_kingston/Tekmerion/.bench
python3 combine-answer-and-setup.py x86-sealed-work 1

cd /mnt/1tb_kingston/Tekmerion/.bench
export PATH=/home/one/.elan/toolchains/leanprover--lean4---v4.29.1/bin:$PATH
./run-answers.sh eval-20261124_003911 1

cd /mnt/1tb_kingston/Tekmerion/.bench
export PATH=/home/one/.elan/toolchains/leanprover--lean4---v4.29.1/bin:$PATH
python3 collect-verdicts.py eval-20261124_003911 x86-sealed-verdicts.csv
```

Replay:

```
cd /mnt/1tb_kingston/Tekmerion/.bench
sed -e "s/^EVAL=.*/EVAL=eval-20261124_003911/" \
    -e "s/^OUT=.*/OUT=x86-sealed-replay/" \
    x86-replay-run.sh > x86-sealed-replay-run.sh
chmod +x x86-sealed-replay-run.sh
export PATH=/home/one/.elan/toolchains/leanprover--lean4---v4.29.1/bin:$PATH
./x86-sealed-replay-run.sh
```

Seconds extraction:

```
cd /mnt/1tb_kingston/Tekmerion/.bench
grep -H '^PROFILE' eval-20261124_003911/*.run.outlog
```

## 11. Artefacts

| path | what |
|---|---|
| `.bench/x86-sealed-work/<ID>/answer.txt` | the nine answer tapes (hashes in §2) |
| `.bench/x86-sealed-work/<ID>/accumulate.txt` | derived accumulate set per task |
| `.bench/eval-20261124_003911/` | judge programs, `*.run.outlog`, `*.run.errlog`, `*.judge.txt` |
| `.bench/x86-sealed-verdicts.csv` | CSV from `collect-verdicts.py` |
| `.bench/x86-sealed-replay/` | replay programs and `*.replay.log` |
| `.bench/x86-sealed-replay-all.log` | replay `REPLAY` lines |
| `research/receipts/ix53-sealed-verdicts.csv` | copy of the verdict CSV |
| `research/receipts/ix53-sealed-run.md` | this receipt |

## 12. Stop condition

Stopped after this receipt. No failure was investigated, no code was modified,
and nothing was re-run. There were no refusals and no infrastructure errors.
