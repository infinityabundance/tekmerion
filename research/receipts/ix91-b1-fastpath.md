# IX.91 — XXIII.B1: the per-carrier aggregate premise fast path

**Date:** 2026-11-27
**Branch:** `wip/xxiii-b-fastpath` (measured tree: `78a4433` plus a test-only digest edit;
`tools/bridge.ml` sha256 `895a51057fa1fe1f0def697ae4a60bcb34851507b7f533e49e09073b71be93f6`)
**Host:** same host as IX.89; `free -g` available 91 GB at start, 84 GB at end; 250 GB disk free.
**Counter:** `hol-light-stamp2`, same as IX.89. Each proof built with
`tools/cap.sh 12G -- make -j1`, one at a time; no cap was raised and none fired.

## 1. What changed

`bridge_aggregate_facts` in `tools/bridge.ml` proved each carrier's fact from **all** aggregates
plus the side hypotheses. IX.86 §4 traced 477,376 inferences on `add` to handing the fact proof one
logically inert aggregate — 93.2 % of the 512,129 regression introduced by IX.83's generalisation —
and §11 recorded that the fast path was **not implemented**.

This change implements it: a carrier's fact is proved from the aggregate(s) that **mention** that
carrier, with an automatic fallback to the full list when the fast path fails. Both paths are real
kernel proofs; the fallback makes the change behaviour-preserving whenever the fast path is wrong.

```
let aggs_mentioning v = List.filter (... mentions v ...) aggregates
let premise_sets v =
  let local = aggs_mentioning v in
  if local = [] || List.length local >= List.length aggregates then [aggregates @ side_hyps]
  else [local @ side_hyps; aggregates @ side_hyps]
```

## 2. The measurement

Strict self-contained accounting (`XXIII.0` rule 3), same phase boundaries as IX.89. `plain` and
`phase` modes agree exactly (`diff 0` on every total), so the instrumentation is inert.

| task | IX.89 total | IX.91 total | Δ | ratio to human |
|---|---:|---:|---:|---:|
| add | 5,745,767 | **5,268,391** | **−477,376 (−8.31 %)** | **2.85×** |
| sub | 9,029,510 | **8,217,594** | **−811,916 (−8.99 %)** | **6.40×** |
| neg | 5,683,733 | 5,683,733 | 0 | 7.88× |

Human baselines (IX.89): 1,848,494 / 1,283,174 / 721,473.

The entire reduction is in the aggregate-fact phase; every other phase is **bit-identical**:

| phase | add before → after | sub before → after | neg before → after |
|---|---:|---:|---:|
| PRE support library | 1,498,061 → 1,498,061 | 1,497,979 → 1,497,979 | 1,497,352 → 1,497,352 |
| A init + digitise | 1,795 → 1,795 | 1,805 → 1,805 | 1,076 → 1,076 |
| B simulation | 787,891 → 787,891 | 674,249 → 674,249 | 528,862 → 528,862 |
| C aggregate fact | 1,178,682 → **701,306** | 2,155,782 → **1,343,866** | 1,511,696 → 1,511,696 |
| D linear close | 1,128,617 → 1,128,617 | 2,786,328 → 2,786,328 | 1,563,214 → 1,563,214 |
| E other | 1,150,721 → 1,150,721 | 1,913,367 → 1,913,367 | 581,533 → 581,533 |
| **total** | 5,745,767 → **5,268,391** | 9,029,510 → **8,217,594** | 5,683,733 → 5,683,733 |

`Δtotal = ΔC` exactly: add 1,178,682 − 701,306 = 477,376; sub 2,155,782 − 1,343,866 = 811,916.
The `add` recovery is the IX.86 figure to the inference.

## 3. Why `neg` is unchanged (measured, not assumed)

In phase mode the instrumenter prints `FACT_PREM hyps=<n> size=<n> aggs=<n>` at each
`bridge_aggregate_facts` call. On the `neg` run **every** call reports `aggs=1` (for example
`FACT_PREM hyps=29 size=3009 aggs=1`), while `add` and `sub` report `aggs=2`. With a single
aggregate, `List.length local >= List.length aggregates` is always true, so `premise_sets` returns
the full list — byte-identical to the old behaviour. The optimisation is structurally a no-op for a
proof with one aggregate. A confirmation run of `neg` reproduced `STAMP_INFERENCES 5683733` exactly.

**This is a miss against the pre-registered `XXV.D1` target** ("on `sub`/`neg` at least 20 %"):
`sub` met it (9.0 % of the total, but 37.7 % of the aggregate-fact phase); `neg` did not move at
all. Published as a miss, with the structural reason above. `neg` needs a different lever
(`XXV.C` steps 3–4), not this one.

## 4. Pre-registered criteria (`XXV.D1`)

| criterion | target | result |
|---|---|---|
| `add` aggregate-fact phase drop ≥ IX.86's 477,376 | ≤ 701,306 | **701,306 — met exactly** |
| `sub` aggregate-fact phase drop ≥ 20 % | ≤ 1,724,626 | **1,343,866 — met (37.7 %)** |
| `neg` aggregate-fact phase drop ≥ 20 % | ≤ 1,209,357 | **1,511,696 — missed (0 %; structural)** |

## 5. Commands (decisive)

```
$ sha256sum tools/bridge.ml
895a51057fa1fe1f0def697ae4a60bcb34851507b7f533e49e09073b71be93f6

$ bash tests/ix89/run_gen.sh plain add    # then phase add
GEN_RUN arm_add_p25519.plain rc=0 phase_marks=101
STAMP_INFERENCES 5268391
$ bash tests/ix89/run_gen.sh plain sub    # then phase sub
GEN_RUN arm_sub_p25519.plain rc=0 phase_marks=184
STAMP_INFERENCES 8217594
$ bash tests/ix89/run_gen.sh plain neg    # then phase neg
GEN_RUN arm_neg_p25519.plain rc=0 phase_marks=85
STAMP_INFERENCES 5683733

$ awk -f tests/ix89/parse_gen.awk .bench/ix89/gen/arm_add_p25519.phase.correct
# PHASE PRE 1498061 A 1795 B 787891 C 701306 D 1128617 E 1150721 ; diff 0
$ awk -f tests/ix89/parse_gen.awk .bench/ix89/gen/arm_sub_p25519.phase.correct
# PHASE PRE 1497979 A 1805 B 674249 C 1343866 D 2786328 E 1913367 ; diff 0
$ awk -f tests/ix89/parse_gen.awk .bench/ix89/gen/arm_neg_p25519.phase.correct
# PHASE PRE 1497352 A 1076 B 528862 C 1511696 D 1563214 E 581533 ; diff 0
```

All six proofs ran under `tools/cap.sh 12G -- make -j1`, sequentially; every run `rc=0`, wall time
27–32 s.

## 6. Fixtures and suite

The bridge text moved, so all 27 answer fixtures, `tests/fixtures/curve_check_bridge.txt` (line
numbers only), `BRIDGE_DIGEST`, the instrument `sha256` and the four inline-answer digests were
re-recorded deliberately. Final state: `cargo test --offline --no-fail-fast` **396 passed, 0 failed**
across 45 targets; `cargo clippy --offline --all-targets -- -D warnings` exits 0;
`curve_check` still reports `statements=91 hits=0`.

## 7. What is not established

- **No `equals_thm` re-check in this run.** The generated answer was assembled and built by the
  harness (the `_CORRECT` theorem was produced), but the explicit `equals_thm` comparison against the
  upstream theorem was not re-run here. It is inherited from IX.89 and must be re-run before any
  victory claim.
- **One accounting mode only.** These are strict self-contained totals. No matched-session number
  (`XXIII.P3`) was measured.
- **No timing claim.** Wall times (27–32 s) are recorded as observations, not as medians of five.
- **No soundness change claimed.** Both proof paths are kernel proofs; the fallback preserves the
  old behaviour. The face validity of that argument is not a formal proof of equivalence.
- **`neg` unchanged**, and therefore still 7.88× human. This experiment does not address the hardest
  race.
- **L9 (proof amplification) is not measurable yet** — there is no certificate backend.
