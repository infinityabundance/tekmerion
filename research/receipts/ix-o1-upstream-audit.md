# Decision record — reading goal quotations from pinned upstream `.ml` files

- **Date:** 2026-11-24
- **Trigger:** review defect: the challenge comparator reads the same `problems.json` the
  producer used, so it cannot catch an error in the corpus. The independent source is the
  pinned upstream s2n-bignum proof file.
- **Implementation:** `src/tools/challenge.rs` `verify --upstream` (helper `upstream_goal`).
- **Result:** 27/27 challenges pass `statement==upstream-goal` (9 x86 sealed at
  `eval-20261124_061439`, 18 ARM at `eval-20261124_030015`).

## What is read and what is not

For a task, the comparator:

1. takes `problems.json[task]["json"]["filename"]` and `["theorem_name"]`;
2. opens the pinned `.ml`, finds `let <theorem_name> =`;
3. reads the **first backtick-delimited HOL quotation** after it;
4. normalises whitespace and compares it to the challenge statement (stripping the outer
   backticks the corpus stores).

The proof body after that quotation is never parsed or used. The extractor was first
validated on the six non-held-out arithmetic tasks (`p256`, `p384`, `p521` × `add/sub/neg`
on ARM) and all nine matched.

## The exception, stated plainly

For the nine sealed x86 tasks this reads the **goal range** of a sealed proof file. The
programme's guard forbids opening sealed proof text during development. This audit is
permitted on the following basis, and only on this basis:

- the statement is **not secret**: it is already in `problems.json`, and the upstream
  s2n-bignum repository is public;
- only the goal quotation is read, never the proof body;
- the sealed evaluation was **completed and frozen** (IX.53, IX.54, IX.55) before this audit,
  so no verdict can be influenced by it;
- the audit is an additional check on the *benchmark's* extraction, not a source of proof
  material.

This is recorded rather than silently allowed because it changes what "not reading sealed
files" means in practice. If the programme later re-uses these nine as a fresh held-out set,
the audit must not be run on them until after the new evaluation.

## Scope and residuals

- The extractor handles the `let <NAME> = …` shape used by the 27 arithmetic proofs. Other
  corpus entries use different shapes (`= prove`, `= prove_by_refinement`, …) and were not
  audited; a corpus-wide audit needs per-shape extraction and is not claimed.
- The comparison is whitespace-normalised text, not a kernel term comparison. It checks that
  the benchmark copied the intended statement faithfully; it does not re-typecheck it.
- The audit does not change any verdict and does not add a truth channel.

## Commands

```
cargo run --offline -q --bin challenge -- verify --challenge research/challenges/<task>.challenge \
    --evaldir <judge dir> --upstream
```

## Result

```
RESULT-x86 9/9
RESULT-arm 18/18
```
