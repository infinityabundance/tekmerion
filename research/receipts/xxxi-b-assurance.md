# XXXI.B — upstream hashes, A/B hypothesis identity, backend selector

**Date:** 2026-11-28
**Branch:** `staging` (working tree; not committed by this task).
**Scope:** automatic assurance only.  No change to `tools/bridge.ml`, the
fixtures, or the proof tactic.  One small emitter change (backend selector),
three new files, and the `run_gen.sh` wiring.

## 1. What was added

| file | role |
|---|---|
| `src/tools/emit.rs` | `TEKMERION_SIM_BACKEND` (`tekmerion` default / `upstream`); `SimBackend`; pure unit tests |
| `src/tools/config.rs` | declare `TEKMERION_SIM_BACKEND` in `ENV_VARS`/`ALL_ENV_VARS` (the existing contract test requires every source-read variable to be declared) |
| `tests/tools_freeze.rs`, `tests/ix77_enforcement_controls.sh` | include the new declared variable in the freeze/control fixtures |
| `tests/ix89/framework_check.sh` | `hashes` and `ab <task-id>` subcommands |
| `tests/ix89/upstream_manifest.txt` | the pinned SHA-256 manifest (data, not a constant) |
| `tests/ix89/ab_lib.ml` | in-session `aconv` recorder/comparator and per-backend statement check |
| `tests/ix89/ab_gen.pl` | builds the two-backend A/B proof from the matched-session split |
| `tests/ix89/run_gen.sh` | prints `UPSTREAM_HASH`/`UPSTREAM_UNCHANGED` every run and enforces them; `TEKMERION_AB=1` runs the A/B mode |
| `tests/ix89/ASSURANCE.md` | usage and the three-baseline procedure |

## 2. Resources

`free -g` before the runs: `total=125 used=33 free=79 available=92`.
Every build: `tools/cap.sh 12G -- make -j1`, one at a time, `-j1`; no cap fired.

A/B sessions (`cap.sh` summary, stderr):

```
neg: group=3818708 rc=0 sampled_group_peak_rss_kb=1914976 max_process_vmhwm_kb=1699480
add: group=3812285 rc=0 sampled_group_peak_rss_kb=1795260 max_process_vmhwm_kb=1704724
```

`make` rc 0; proof `Running time` 26 s (`neg`) and 32 s (`add`).

## 3. Upstream hash pin and printed lines (`XXXI.B1`)

`tests/ix89/upstream_manifest.txt`:

```
893b5b1cc8589fb4024555952951e269db4b9a1438d1f5e55ef462d3be155857  common/components.ml
1f83d69177b0625928943394f4d4686f6bc8e8e90398078a802a1295a9ddefce  common/relational.ml
6ff6d009d873ac8a0c00b492ab965a0d0a14f67ed92d36fa168e246f39f9bff9  arm/proofs/arm.ml
4e08e32c6aa3afe27c89a500a9ef5b0d4e357d5d44ad525c9a82abaebaa0a5d0  arm/proofs/base.ml
```

`tests/ix89/framework_check.sh hashes` printed:

```
UPSTREAM_HASH common/components.ml 893b5b1cc8589fb4024555952951e269db4b9a1438d1f5e55ef462d3be155857
EFFECTIVE_HASH common/components.ml 893b5b1cc8589fb4024555952951e269db4b9a1438d1f5e55ef462d3be155857
UPSTREAM_HASH common/relational.ml 1f83d69177b0625928943394f4d4686f6bc8e8e90398078a802a1295a9ddefce
EFFECTIVE_HASH common/relational.ml 1f83d69177b0625928943394f4d4686f6bc8e8e90398078a802a1295a9ddefce
UPSTREAM_HASH arm/proofs/arm.ml 6ff6d009d873ac8a0c00b492ab965a0d0a14f67ed92d36fa168e246f39f9bff9
EFFECTIVE_HASH arm/proofs/arm.ml 6ff6d009d873ac8a0c00b492ab965a0d0a14f67ed92d36fa168e246f39f9bff9
UPSTREAM_HASH arm/proofs/base.ml 4e08e32c6aa3afe27c89a500a9ef5b0d4e357d5d44ad525c9a82abaebaa0a5d0
EFFECTIVE_HASH arm/proofs/base.ml 4e08e32c6aa3afe27c89a500a9ef5b0d4e357d5d44ad525c9a82abaebaa0a5d0
UPSTREAM_UNCHANGED 1
```

`run_gen.sh` prints these before every build and exits non-zero on
`UPSTREAM_UNCHANGED 0`; the pristine baseline (`human-baseline`) and the
effective build tree (`.bench/dropin`) are both compared to the pin.

## 4. A/B identity, `TEKMERION_AB=1` (`XXXI.B2`)

Both backends run in one HOL session with the support library loaded once.
The comparison is `aconv` over the goal, every assumption conclusion (frame
predicates included and counted) and the bridge hypotheses.

### neg (`arm.bignum_neg_p25519`)

```
AB_BACKEND_SELECT ... tekmerion_calls=1 upstream_calls=1 tekmerion_definition=1
AB_L7 1496597
AB_INFERENCES backend=tekmerion 779424
AB_STRICT backend=tekmerion 2276021
AB_INFERENCES backend=upstream 799648
AB_STRICT backend=upstream 2296245
AB_COMPARE task=arm.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT states=39 assumptions=831 frames=38 bridge_hyps=7
AB_IDENTICAL task=arm.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT states=39 assumptions=831 frames=38 bridge_hyps=7
STATEMENT_OK task=arm.bignum_neg_p25519 aconv=1 hyps=0 axioms=3
STATEMENT_OK_UP backend=upstream task=arm.bignum_neg_p25519 aconv=1 hyps=0 axioms=3
AB_VERDICT ... identical=1 statement=1 tekmerion=779424 upstream=799648
```

### add (`arm.bignum_add_p25519`)

```
AB_L7 1496597
AB_INFERENCES backend=tekmerion 2252460
AB_STRICT backend=tekmerion 3749057
AB_INFERENCES backend=upstream 2256318
AB_STRICT backend=upstream 3752915
AB_COMPARE task=arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT states=39 assumptions=1189 frames=38 bridge_hyps=13
AB_IDENTICAL task=arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT states=39 assumptions=1189 frames=38 bridge_hyps=13
STATEMENT_OK task=arm.bignum_add_p25519 aconv=1 hyps=0 axioms=3
STATEMENT_OK_UP backend=upstream task=arm.bignum_add_p25519 aconv=1 hyps=0 axioms=3
AB_VERDICT ... identical=1 statement=1 tekmerion=2252460 upstream=2256318
```

The Tekmerion strict totals reproduce the published IX.104 figures exactly
(`neg 2,276,021`; `add 3,749,057`).  The upstream A/B column is measured
**second in the same session**, so it is cache-warmed; it is not the fresh
upstream baseline (see §5).  `states = 2 × steps + 1`; `frames` counts frame
assumptions over those states; `bridge_hyps` is the bridge-entry assumption
count (7 on `neg`, matching IX.104; 13 on `add`).

## 5. Backend selector and the fresh upstream baseline (`XXXI.B3`)

```
TEKMERION_SIM_BACKEND=upstream tests/ix89/run_gen.sh plain neg
```

printed `STATEMENT_OK task=arm.bignum_neg_p25519 aconv=1 hyps=0 axioms=3` and

```
STAMP_INFERENCES 2380714
```

which is exactly the upstream baseline recorded in
`research/receipts/ix103-supersede-state-update.md` (`neg 2,380,714`).  The
default (`tekmerion`) run of the same task printed
`STAMP_INFERENCES 2276021`.  The two answers differ only by the simulation call
name and the inlined definition: the upstream answer contains exactly one
`ARM_ACCSTEPS_TAC` call and no `TEKMERION_*` name.

Three baselines (`XXXI.F1`), commands:

```
tests/ix89/run_gen.sh plain <op>                              # generated (Tekmerion)
TEKMERION_SIM_BACKEND=upstream tests/ix89/run_gen.sh plain <op> # original backend
TEKMERION_AB=1 tests/ix89/run_gen.sh plain <op>                # hashes + A/B identity
```

Baseline (1) compares the generated strict total to the recorded human total;
baseline (2) replaces only the simulation call in the human proof; baseline (3)
is the two selector rows above.  Details in `tests/ix89/ASSURANCE.md`.

## 6. Not established

- The A/B mode proves intermediate-state identity and the final theorem
  `aconv`-identity; correctness of the inlined tactics still rests on the
  kernel accepting every step, not on a separate soundness proof.
- The upstream A/B column is same-session-second; the fresh upstream baseline
  must come from the selector run.
- The statement cache (and therefore the `STATEMENT_OK` part of A/B) is defined
  for the `p25519` tasks; the identity comparison runs for any ARM
  add/sub/neg task, and other families print `AB_STATEMENT_SKIPPED`.
