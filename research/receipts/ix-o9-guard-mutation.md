# Receipt — guard refusal-site mutation coverage (Ordeal O9.3)

- **Date:** 2026-11-24
- **Branch:** `staging`
- **Artefacts:** `tools/mutate_guard.py`, `tools/test_guard_refusals.py`,
  `.github/workflows/ci.yml`
- **Subject:** `tools/heldout_guard.py` (the Python guard that decides which tasks and
  objects may be opened). `tools/mutate.py` mutates the *Rust* refusal sites only; the
  guard was not covered by it.

## 1. Why this matters

The guard is on the acceptance path: it decides whether a held-out task, object or proof
file may be read. A guard refusal that no test notices is a vacuous check, so the guard
gets the same treatment as the Rust engine: disable each refusal in turn and require the
suite to notice.

## 2. Method

`tools/mutate_guard.py` parses `heldout_guard.py` with `ast`, finds every
`raise HeldOutRefused(...)`, and for each one replaces the whole statement with
`pass  # MUTATED`. It runs `pytest -x -q` over
`tools/test_heldout_modes.py tools/test_guard_refusals.py tools/test_challenge.py`
and records `KILLED` (suite fails) or `SURVIVED` (suite passes). The file is restored
after every mutant and once more in `finally`, with a SHA-256 check; a stale `.orig`
backup makes the tool refuse to start.

AST-based rewriting is used so multi-line raises and `... from exc` are handled without
regex fragility.

## 3. Measured

First measurement, before adding tests:

```
guard refusal sites: 71
killed: 22  survived: 49
mutation score: 22/71 = 0.310
```

After `tools/test_guard_refusals.py` (one test per reachable site) and the freeze-binding
check added on the same day:

```
guard refusal sites: 72
killed: 66  survived: 6
mutation score: 66/72 = 0.917
```

The first score is the honest datum: the guard's admission refusals were **not** covered
by the existing mode tests.

## 4. The six survivors are equivalent mutants, with a reason

Each survivor was inspected. In every case the mutant still *refuses*, because a downstream
check raises the same exception:

| line | site | why removing it does not change accept/refuse |
|---|---|---|
| 271 | `check_path`: empty path | falls through to `_check_regular("")`, which raises `HeldOutRefused("empty path")` |
| 520 | `verify_freeze`: dirty `tools/` | a dirty tree also differs from the freeze commit, so the new `git diff` check refuses |
| 607 | `check_object`: empty path | falls through to `check_path("")` → line 271 path → same refusal |
| 632 | `check_proof`: empty path | falls through to `check_path("")` → same refusal |
| 640 | `check_proof`: held-out marker (develop) | falls through to `check_path(path, …)`, which raises the same marker refusal (message differs) |
| 647 | `check_path`: empty path | falls through to the same `check_path("")` refusal |

Five are exactly behaviour-preserving; the sixth (the held-out marker in `check_proof`)
preserves accept/refuse and differs only in the diagnostic string. No survivor turns a
refusal into an acceptance.

This is recorded rather than "fixed" by deleting the defensive early returns, because the
early returns give the more specific diagnostic.

## 4a. New freeze-binding check (2026-11-24)

The harness exposed a real weakness: `verify_freeze` checked that `tools/` was *clean*, not
that it *was* the frozen code. A later commit could change the emitters and still pass an
older tag. It now also runs

```
git diff --name-only <freeze_commit> -- tools
```

and refuses if anything is listed. Firing controls: `test_sealed_refuses_tools_that_moved_since_the_freeze`
uses a *valid but different* commit (so the tag check cannot mask the diff check) and also
tests the tag-resolves-to-a-different-commit path.

Consequence: `freeze-x86-sealed-v3` no longer verifies against the current tree, because
`tools/` has moved. That is the intended behaviour; re-verifying IX.54 requires checking out
`2705f5b`.

Effective coverage of non-equivalent sites is **66/66**.

## 5. Firing controls

`python3 -m pytest tools/test_guard_refusals.py -q` → **49 passed**. Every test asserts a
specific `HeldOutRefused` message, so a mutated guard that stops refusing fails the test.

The harness itself is falsifiable: every survivor must either be killed or appear in the
line-anchored `EQUIVALENT` table in `tools/mutate_guard.py`. Any new unlisted survivor makes
the tool exit 1 and fails CI; and because the table is keyed by `(line, source)`,
an edit that shifts a listed site also fails until it is re-reviewed.

## 5a. CI exit behaviour

The tool exits 0 only when every survivor is a declared equivalent mutant, and 1 otherwise.
The current run ends with `all survivors are recorded equivalent mutants`.

## 6. CI

`.github/workflows/ci.yml` gains:

```
- name: Guard refusal-site mutation harness (offline)
  run: python3 tools/mutate_guard.py
```

The step is not `continue-on-error`.

## 7. Scope

This establishes that the guard's refusals are *tested*, not that the guard is *correct*.
The guard is still procedural isolation (O10.2 open); this work does not change that.
