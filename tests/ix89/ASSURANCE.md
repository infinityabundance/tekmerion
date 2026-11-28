# XXXI.B assurance: upstream hashes, A/B identity, backend selector

This directory hosts the automatic assurance that must hold before the next
Tekmerion claim ("first win", `XXXI.0.3`).  Nothing here edits the shared
s2n-bignum framework, `tools/bridge.ml` or the emitted fixtures.

## 1. Upstream framework hashes (`XXXI.B1`)

**Pin:** `tests/ix89/upstream_manifest.txt` — `<sha256>  <path>` per file.
The pin is data, not a mutable in-code constant.

**Files:** `common/components.ml`, `common/relational.ml` and
`arm/proofs/arm.ml` are what `TEKMERION_ARM_TACTIC` copies from or calls into
(`STATE_UPDATE_RULE`, `MAYCHANGE_CANON_CONV`, `ASSIGNS_SWAP_CONV`, `SEQ_ASSOC`,
the step tactics).  `arm/proofs/base.ml` is the `needs` entry the generated
proof names, so pinning it also pins the closure declaration.

**Command:**

```
tests/ix89/framework_check.sh hashes
```

It prints, for every pinned file, the pristine-baseline digest
(`UPSTREAM_HASH <rel> <sha256>`), the effective build-tree digest
(`EFFECTIVE_HASH <rel> <sha256>`, the tree the proof's `needs` resolves
against), any drift line, and one verdict:

```
UPSTREAM_UNCHANGED 1
```

`UPSTREAM_UNCHANGED 0` (missing file, drifted pristine copy, or drifted
effective tree) is a failing run.  `tests/ix89/run_gen.sh` calls this before
every build and exits non-zero on `0`, so "baseline untouched" is a checked
fact on every run receipt, not a claim.

## 2. A/B hypothesis identity (`XXXI.B2`)

```
tests/ix89/framework_check.sh ab <task-id>
```

The task id is the full emitted id, e.g.
`arm.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT`.  The mode works for any ARM
bignum add/sub/neg task; when a `p25519` statement cache exists it also runs
the statement/axiom check on both theorems.

**How it runs.**  Re-loading the inlined support library in one HOL session
would redefine constants, so the proof loads the library once (the
`matched_session.sh` split) and runs the task-only tactic twice: once with
`ARM_XACCSTEP_TAC` (the shared framework) and once with
`TEKMERION_ARM_XACCSTEP_TAC` (Tekmerion-owned).  `tests/ix89/ab_lib.ml` spys
every simulation step's **entry goal and assumption conclusions**, the **goal
it leaves behind**, and the **bridge entry**, then compares each upstream
recording with the Tekmerion one using `aconv` (not string equality):

* the goal of every state,
* **every** assumption conclusion, in order (the frame predicates are included
  in this comparison and also counted),
* the bridge hypotheses (the assumptions at `BRIDGE_*_TAC`).

A single difference prints `AB_MISMATCH task=... element=... tag=...` and makes
the run fail.  Success prints:

```
AB_COMPARE task=... states=N assumptions=M frames=F bridge_hyps=B
AB_IDENTICAL task=... states=N assumptions=M frames=F bridge_hyps=B
STATEMENT_OK task=... aconv=1 hyps=0 axioms=3            (Tekmerion backend)
STATEMENT_OK_UP backend=upstream task=... aconv=1 hyps=0 axioms=3
AB_INFERENCES backend=tekmerion <task-only>
AB_STRICT backend=tekmerion <L7 + task-only>
AB_INFERENCES backend=upstream <task-only>
AB_STRICT backend=upstream <L7 + task-only>
AB_VERDICT task=... identical=1 statement=1 ...
```

`states` counts recorded goals (`2 × steps + 1` bridge); `frames` counts
assumption conclusions mentioning `MAYCHANGE`/`ASSIGNS` across those states;
`bridge_hyps` is the assumption count at the bridge entry.

**Ordering caveat.**  The Tekmerion task runs first and is the clean figure
(`AB_STRICT backend=tekmerion` reproduces the published strict total exactly).
The upstream task runs second in the same session, so its figure is
cache-warmed and is *not* the fresh upstream baseline.  Use baseline #3 below
for that number.

## 3. Backend selector (`XXXI.B3`)

`TEKMERION_SIM_BACKEND` selects the simulation backend of the emitted answer:

| value | emitted call | inlined Tekmerion tactic |
|---|---|---|
| `tekmerion` (default, unset/empty) | `TEKMERION_ARM_ACCSTEPS_TAC` | yes |
| `upstream` | `ARM_ACCSTEPS_TAC` | no |

An unrecognised value is an emitter error, never a silent fallback.  The two
answers differ only by the call name (and the presence of the Tekmerion tactic
definition), so the published baselines are reproduced without editing
fixtures.

## 4. The three-baseline procedure (`XXXI.F1`)

Run one task at a time, capped, `-j1` (`AGENTS.md §7a`).

**(1) Generated vs the original human proof — system superiority.**
The generated strict total is `STAMP_INFERENCES` from

```
tests/ix89/run_gen.sh plain <add|sub|neg>
```

and the human `_CORRECT` total is the one recorded by the statement-cache
extraction (`research/receipts/ix60-matched-inference.md`, refreshed in the
IX.10x receipts).  Do not present a simulation gain as an arithmetic gain.

**(2) Generated vs the human proof on the same optimised backend — arithmetic
compiler superiority.**  Replace only the simulation call in the human proof
with the Tekmerion call (the same substitution the generated answer makes) and
rebuild it in the same capped session shape.  The difference is the arithmetic
close, not the state engine.

**(3) Tekmerion backend vs the original backend — state-engine superiority.**
Two fresh runs of the same generated task:

```
TEKMERION_SIM_BACKEND=tekmerion tests/ix89/run_gen.sh plain <op>
TEKMERION_SIM_BACKEND=upstream  tests/ix89/run_gen.sh plain <op>
```

Compare `STAMP_INFERENCES`; both must print `STATEMENT_OK`, and
`TEKMERION_AB=1` on the same task proves the two runs' intermediate states are
`aconv`-identical.  With the default backend this is also the normal run, so
`TEKMERION_AB=1 tests/ix89/run_gen.sh plain <op>` prints the hashes, the
Tekmerion strict total, and the A/B verdict in one receipt.

## 5. Resource discipline

Every HOL build in this directory runs through
`tools/cap.sh 12G -- make -j1`, one at a time.  `framework_check.sh ab` prints
`FREE_G` before the build.  A cap that fires is a result, not a retry.
