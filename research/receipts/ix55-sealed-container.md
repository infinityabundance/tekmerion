# IX.55 — sealed x86 re-run with the judge directory preserved

- **Date:** 2026-11-24
- **Branch:** `staging`
- **Freeze:** manifest `research/receipts/ix55-sealed-manifest.json`, tag
  `freeze-x86-sealed-v4`, `freeze_commit` `b8c2b6fb28f7ebba65327136d33b046eaa8873a2`
- **Judge directory:** `eval-20261124_061439` (bind-mounted to the host, still present under
  `.bench/out-v4/`)
- **Verdict: OK 9/9.** This closes limitation 1 of IX.54: the `*.judge.txt` files now survive
  the container and can be re-collected.

This is the same nine tasks under a fresh freeze, not a new split. It re-establishes the
IX.54 result with stronger evidence handling.

## 1. What changed since IX.54

- New freeze `v4` (IX.54's `v3` was retired by the freeze-binding change: the guard now
  refuses unless `git diff <freeze_commit> -- tools` is empty).
- The container has a writable `/out` bind mount; the run copies the **whole** judge
  directory, the emitted answers, the verdicts and all logs there.
- The guard inside the container verifies `freeze-x86-sealed-v4` on every emitter call.

## 2. Isolation (structural, for the proof data)

From the container log:

```
HEAD=61fe3680b3a0235d7a932851c3afbd1a997ed306        (manifest commit)
FREEZE=b8c2b6fb28f7ebba65327136d33b046eaa8873a2      (tag commit)
s2n_bignum_ml_lines=0
TEST_E_ABSENT
cat_exit=1                                            (proof path does not exist)
cat_hostpath_exit=1                                   (host path is not mounted)
```

The nine benchmark templates are present only as prefixes truncated at each task's
insertion line (verified byte-identical to the prefix the harness consumes in IX.54).
No `s2n-bignum` `.ml` file exists anywhere in the container's view.

Limits unchanged and stated: the container borrows host `/usr`, `/lib64`, `/etc`
read-only because the OCaml switch needs glibc 2.44 while the local images have ≤ 2.41.
It is the *data* that is isolated, not the userland.

## 3. Result

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

Per-task seconds (`PROFILE`): add 22.141 / 23.337 / 18.164; sub 9.111 / 16.072 / 9.884;
neg 3.344 / 3.652 / 3.176. Max 23.337 s of 120 s. No timeout.

## 4. Independent re-collection (the point of this run)

Re-run by the orchestrator against the preserved directory:

```
$ python3 .bench/collect-verdicts.py <preserved eval dir> /tmp/recollect.csv
COLLECT_EXIT=0
... SUMMARY, OK,9
```

All nine `*.judge.txt` files exist and contain `OK`.

The challenge comparator was also re-run against the preserved directory:

```
$ python3 tools/challenge.py verify --challenge research/challenges/<task>.challenge \
      --evaldir <preserved eval dir>
x86 challenges: VERIFIED 9 of 9; failures 0
```

Each verify re-checks the statement bytes, the corpus digest, the object digest, the s2n
commit, and that exactly one preserved judge program embeds the statement.

## 5. Preserved evidence

Tracked under `research/receipts/ix55/`: `container.log`, `verdicts.csv`, the nine
`*.judge.txt`, `emit.log`, and `eval.sha256` (SHA-256 of every file in the preserved judge
directory, so the full 282 MB directory on disk is bound by content). The directory itself
stays under git-ignored `.bench/out-v4/`.

## 6. What this does and does not establish

**Establishes:** the nine sealed x86 tasks were generated and judged inside a
proof-data-isolated container, and the judgment evidence is now independently re-collectable.

**Does not establish:** a new split, a coverage claim, or an axiom-free result. `OK` is
still under the bench's inherited inlined context; the axiom-closed result is the separate
upstream drop-in (`ix54-upstream-dropin.md`, all nine, `check_axioms`).

## 7. Commands

```
python3 .bench/build-sealed-court-v4.py
bash .bench/run-sealed-court-v4.sh full
python3 .bench/collect-verdicts.py .bench/out-v4/eval-20261124_061439 /tmp/recollect.csv
python3 tools/challenge.py verify --challenge <challenge> --evaldir .bench/out-v4/eval-20261124_061439
```
