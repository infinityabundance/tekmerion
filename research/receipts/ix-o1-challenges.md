# Receipt — O1.1/O1.2 statement challenges and comparator

- **Date:** 2026-11-24
- **Branch:** `staging`
- **Artefacts:** `tools/challenge.py`, `tools/test_challenge.py`,
  `research/challenges/*.challenge` (twenty-seven files: nine x86 sealed, eighteen ARM)
- **Corpus:** `.bench/problems.json` sha256 `74f314445fa5f579d18b03e73c167ea0ca56e0581e26add8a11d8d077f7ea8d4`
- **s2n-bignum:** `5e0fc782ae6d587c2ae40faf2c2c4d0b637e6240`

## 1. What was built

`tools/challenge.py` has two subcommands and no third-party dependencies.

* `freeze --task ID --out PATH [--post-hoc REASON]` writes a plain-text challenge:
  the exact statement bytes from `problems.json`, its sha256 and length, the
  corpus digest, the object digest, the s2n-bignum commit, and a
  `pre-search true|false` field.
* `verify --challenge PATH [--evaldir DIR] [--require-pre-search]` re-derives every
  digest from the live files and checks that the **judge program that actually ran**
  embeds the statement byte-for-byte (`bench_run "<task>" (<query>) (fun () ->`).

The comparator is deliberately independent of `emit_ref_proof*.py` and of
`heldout_guard.py`: it re-implements the object lookup instead of importing the
emitters, so a defect in the producer cannot hide in the checker of the producer.

## 2. Result — twenty-seven of twenty-seven PASS

The nine sealed x86 tasks verify against `.bench/eval-20261124_003911`, and the eighteen
ARM `add`/`sub`/`neg` tasks verify against `.bench/eval-20261124_030015`. For every task
`verify` reports PASS on: `statement-sha256`, `statement-bytes`, `task-present`,
`statement==problems.json`, `category`, `object-exists`, `object-sha256`,
`judge-program-scanned`, `judge-embeds-statement`, `judge-verdict-ok`.

That binds, for each task: the intended statement → the corpus → the object →
the exact judge program → the `OK` verdict file.

One comparator defect was found and fixed while doing this: the first version guessed the
judge-program filename from the task id, but some tasks are inlined into a
*differently named* shared template (`arm.bignum_sub_p256` lives in
`ml_files_arm_p256_montjdouble.ml`). `verify` now scans every `ml_files_*.ml` and requires
exactly one to embed the statement. Regression test:
`test_judge_program_named_for_a_shared_template`.

## 3. What this does **not** establish

* **Kernel-term identity is not independently compared.** It does not need to be
  re-established for these runs: the harness calls `prove(query, tac)` and HOL's
  `prove` fails unless the tactic proves that exact goal, so the accepted
  theorem's conclusion *is* the query. What was missing was an independent record
  of the intended statement; that is what this adds. Comparing exported proof
  objects is the stronger form and is still O2 work.
* **The axiom whitelist is not checked** by this comparator. The harness compares axiom
  *counts* (`axioms_before = axioms_after`), which detects cheating but permits inherited
  context. The whitelist is now covered separately for all nine by the upstream drop-in
  (`ix54-upstream-dropin.md`, `check_axioms`).
* **Statement provenance is now checked** by `verify --upstream`: the statement is compared
  with the goal quotation in the pinned upstream `.ml` (27/27). See
  `ix-o1-upstream-audit.md`, which also records the sealed-goal-reading exception.
* **O1.1 is not met for these twenty-seven.** The challenge files were written *after* the
  judged run, so each carries `pre-search false` and
  `frozen-at POST_HOC: judged at eval-20261124_003911`. `verify
  --require-pre-search` refuses them, and certification must. For future runs,
  `freeze` must be run before `emit`; then the field is `true`.

## 4. Firing controls (the clause must be able to fail)

`python3 -m pytest tools/test_challenge.py -q` → **7 passed**. Each control plants
a defect and requires rejection:

| control | required result |
|---|---|
| one byte changed inside the statement | `FAIL statement-sha256`, `FAIL statement==problems.json` |
| one byte changed in the object | `FAIL object-sha256` |
| judge program embeds a different statement | `FAIL judge-embeds-statement` |
| challenge frozen post hoc, `--require-pre-search` | `FAIL pre-search` |
| duplicated header key | refusal (exit 2) |
| task moved out of the corpus | `FAIL task-present` |
| a byte changed in the corpus | `FAIL problems-json-sha256` |
| verdict file missing | `FAIL judge-verdict-ok` (`missing verdict file`) |
| no `--evaldir` at all | `RESULT INCOMPLETE` (exit 3), never PASS |
| judge program named for a shared template | `PASS` when it embeds the statement |
| upstream goal differs | `FAIL statement==upstream-goal` |
| clean round trip | `RESULT PASS` |

## 5. Commands

```
python3 tools/challenge.py freeze --task <ID> --out research/challenges/<op>_<family>.challenge \
    --post-hoc "judged at eval-20261124_003911; challenge recorded after the run"
python3 tools/challenge.py verify --challenge research/challenges/<op>_<family>.challenge \
    --evaldir .bench/eval-20261124_003911
python3 -m pytest tools/test_challenge.py -q
```
