# XXIX.A — statement-harness hardening

**Date:** 2026-11-27
**Branch:** `staging`; files `tests/ix89/stmt_check.sh`, `stmt_lib.ml`, `run_gen.sh`,
`stmt_check_selftest.sh` (new).

The check itself was correct (IX.94–IX.98: `aconv`, `hyp = []`, `axioms()`); this closes the ways the
**harness carrying it** could be fooled. Nothing about the proofs changed.

## Fixes

1. **Cache validation.** `ensure` requires **exactly one** `REPARSE 1` and validates every mandatory
   field (`TASK`, `THEOREM`, `SOURCE`, `STMT_LEN`, `STMT`, the provenance fields). A `REPARSE 0`, a
   duplicate marker, a missing field, a malformed length or an unknown key is rejected with a named
   reason (`reparse-not-1`, `stmt-length-field`, `stmt-length-mismatch`, `unknown-field`, ...);
   `inject` refuses rather than using an invalid cache.
2. **Provenance binding.** The cache stores and verifies `UPSTREAM_COMMIT`, `SOURCE_DIGEST`
   (upstream proof file), `THEORY_DIGEST` (transitive `needs` closure: upstream proof + `base.ml` +
   HOL `Library/*.ml`) and `OBJECT_DIGEST` (the task's ELF). A mismatch rejects with
   `*-changed`. Extraction also refuses if the human-tree ELF differs from the recorded upstream
   object.
3. **Verdict validation.** `verdict` requires the expected task, **exactly one** canonical
   `STATEMENT_OK`, exactly one `STATEMENT_CHECK_COMPLETE` marker, and **no** `STATEMENT_FAIL`/
   `STMT_EXTRACT_FAIL`/`STATEMENT_CHECK_ABORT` anywhere. Named reasons for each violation.
4. **Portability.** Paths resolve from the repository root; no absolute project-path literals remain.

## Mutation controls (`stmt_check_selftest.sh`)

```
SELFTEST_BASELINE_VALID
SELFTEST_CASE name=missing-completion-marker rc=1 ... result=rejected
SELFTEST_CASE name=wrong-task                 rc=1 ... result=rejected
SELFTEST_CASE name=cache-reparse-0            rc=91 reason=reparse-not-1 result=rejected
SELFTEST_CASE name=duplicate-STATEMENT_OK     rc=1 ... result=rejected
SELFTEST_CASE name=stray-STATEMENT_FAIL       rc=1 ... result=rejected
SELFTEST_SUMMARY cases=5 rejected=5 bad=0 distinct_reasons=5
SELFTEST_EXIT=0
```

Additional direct probes (duplicate `REPARSE`, missing/wrong `STMT_LEN`, each provenance change,
extra key) all reject with distinct reasons.

## Real runs (unchanged)

| run | verdict | `STAMP_INFERENCES` |
|---|---|---:|
| neg | `STATEMENT_OK task=arm.bignum_neg_p25519 aconv=1 hyps=0 axioms=3` | 2,941,309 |
| add | `STATEMENT_OK task=arm.bignum_add_p25519 aconv=1 hyps=0 axioms=3` | 4,310,114 |
| sub | `STATEMENT_OK task=arm.bignum_sub_p25519 aconv=1 hyps=0 axioms=3` | 5,795,082 |

All three `statement_rc=0`, one proof at a time under `tools/cap.sh 12G -- make -j1`.

## Scope

This establishes the harness rejects the five hostile cases and that valid runs still pass with the
same totals. It does not establish cross-session equivalence beyond `aconv` in the shared environment;
the provenance fields bind the upstream revision, theory closure and object bytes, which is the
environment the comparison relies on.
