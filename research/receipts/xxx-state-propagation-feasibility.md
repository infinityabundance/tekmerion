# XXX.PROP — is demand-driven state propagation cheaper for `neg`?

- **Date:** 2026-11-28
- **Branch:** `staging`, HEAD `e41df24` (clean apart from a pre-existing ` M TODO.md`,
  which was left untouched; `main` untouched; nothing committed, nothing pushed)
- **Question:** `research/receipts/xxx-step-cache-feasibility.md` showed that 80.8 % of
  `neg`'s 528,083-inference simulation is state propagation
  (`ASSUMPTION_STATE_UPDATE_TAC` 159,456 + `MAYCHANGE_STATE_UPDATE_TAC` 193,789), not ISA
  expansion (101,460). This receipt asks whether a **demand-driven / incremental** state
  update (cf. `XXIII.D2`) can remove that 353,245, and prototypes three variants on the
  whole 19-step proof.
- **Verdict:** **PARTLY — but the big two are not available.**
  - Deferring the *memory* components **fails** (proof rc=2): memory facts are the ground that
    makes loaded-register facts state-free; without them `STATE_UPDATE_NEW_RULE`'s facts keep a
    stale state read and `DISCARD_OLDSTATE_TAC` deletes them, so the accumulator has no
    `read Xk s` fact left to consume.
  - Deferring *`MAYCHANGE` canonicalisation* is **sound but a net loss**: simulation
    528,083 → **337,726** (−190,357) but `ENSURES_FINAL_STATE_TAC` 142,511 → **409,555**
    (+267,044), total **2,380,410 → 2,457,097 (+76,687)**. The shipped per-write absorption
    keeps the accumulated list at 12–14 groups over 61 writes; deferral makes it one entry per
    write and the single final subsumption pays a quadratic merge.
  - What *does* work is the increment on the assumption side: **do not re-derive an assumption
    about a component the step overwrites.** Measured whole-proof:
    `ASSUMPTION_STATE_UPDATE_TAC` **159,456 → 121,843** (−37,613), `STAMP_INFERENCES`
    **2,380,410 → 2,345,520** (−34,890), `STATEMENT_OK aconv=1 hyps=0 axioms=3`.
    Adding a (program-specific stand-in for a) liveness rule that drops the input-memory facts
    after the last load: **→ 2,315,554 (−64,856)**, statement check unchanged.
  - Combined with the step cache's 77,140 (a disjoint phase), projected simulation
    **528,083 → 386,647**; the review's ~100 K remains **3.9× away** and needs a different
    representation of the frame, not a lazier tactic.

## 1. Resources, commands, exit codes

### 1.1 Resources

```
free -g   (before the builds)          free -g   (after)
 total used free shared buff/cache avail        total used free shared buff/cache avail
   125   28   82    0       14      96           125   29   80    0       16      96
df -h /mnt/1tb_kingston: 264G available
```

Every build and run went through `tools/cap.sh 12G --`, one at a time, never raised.
The cap fired **once**, on a bug in my own prototype (an infinitely recursing term walk):

```
cap.sh: group=3455114 rc=134 sampled_group_peak_rss_kb=12317612 max_process_vmhwm_kb=12317612
        (allocation failure during minor GC)
```

That is recorded as a `capped` result, not retried: the bug was fixed (the walk recursed on the
same node), rebuilt, and the reruns are below. All other runs:

```
cap.sh: group=3449403 rc=0  sampled_group_peak_rss_kb=1876832  (build, neg_state)
cap.sh: group=3465203 rc=0  sampled_group_peak_rss_kb=1901908  (build, neg_demand)
cap.sh: group=3450721 rc=0  sampled_group_peak_rss_kb=167880   (run, probe)
cap.sh: group=3466527 rc=0  sampled_group_peak_rss_kb=168108   (run, shipped)
cap.sh: group=3467876 rc=0  sampled_group_peak_rss_kb=167440   (run, supersede)
cap.sh: group=3469210 rc=0  sampled_group_peak_rss_kb=170964   (run, deadmem)
```

### 1.2 Commands and exit codes

`<HOL>` = `.bench/hol-light-stamp2`; builds use the drop-in tree's own recipe,
`.bench/dropin/tools/build-proof.sh`, one `ocamlopt` target, from `.bench/dropin/arm`.

| # | command | rc |
|---|---|---:|
| 1 | `python3 .bench/xxx-prop/gen_probe_state.py` → `neg_state.ml` | 0 |
| 2 | `cp .../neg_state.ml .bench/dropin/arm/proofs/zzz_neg_state.ml` | 0 |
| 3 | `cd .bench/dropin/arm && ../../../tools/cap.sh 12G -- bash ../tools/build-proof.sh proofs/zzz_neg_state.ml <HOL>/hol.sh ../../xxx-prop/neg_state.native` | 0 |
| 4 | `cd .bench/dropin && ../../tools/cap.sh 12G -- ../xxx-prop/neg_state.native` | 0 |
| 5 | `python3 .bench/xxx-prop/gen_demand.py` → `neg_demand.ml` | 0 |
| 6 | same build recipe for `proofs/zzz_neg_demand.ml` → `neg_demand.native` | 0 |
| 7 | `PROP_MODE=shipped   ../../tools/cap.sh 12G -- ../xxx-prop/neg_demand.native` | 0 |
| 8 | `PROP_MODE=supersede ../../tools/cap.sh 12G -- ../xxx-prop/neg_demand.native` | 0 |
| 9 | `PROP_MODE=deadmem   ../../tools/cap.sh 12G -- ../xxx-prop/neg_demand.native` | 0 |
| 10 | `PROP_MODE=maychg    ../../tools/cap.sh 12G -- ../xxx-prop/neg_demand.native` | 0 |
| 11 | `PROP_MODE=mcfast    ../../tools/cap.sh 12G -- ../xxx-prop/neg_demand.native` | 0 |
| 12 | `PROP_MODE=assume` / `PROP_MODE=demand` (defer-memory prototype) | **2** |

Build wall time ≈ 55 s (1.9 GB peak), run ≈ 25 s (0.17 GB peak). `neg_state` reproduces the
`xxx-step` measurement harness total exactly (`STAMP_INFERENCES 2380410`; the upstream shipped
proof is 2,380,714 — the 304 difference is that harness's marks, as in the previous receipt),
and the per-phase decomposition reproduces `xxx-step-cache-feasibility.md` exactly: simulation
**528,083** with `basic` 101,460 / `nonself` 23,409 / `assume` 159,456 / `maychange` 193,789 /
`newrule` 33,637 / `clarify` 2,395 / `acc` 13,937. So the instrumentation is faithful, and all
comparisons below are within one binary with only `PROP_MODE` changed.

## 2. What the two tactics rewrite, and why 8–10 K per step

### 2.1 `ASSUMPTION_STATE_UPDATE_TAC` (`common/components.ml:3341`)

```ocaml
let ASSUMPTION_STATE_UPDATE_TAC =
  let rec memorable tm = ... (* mentions Const "memory" *) in
  fun gl -> (DISCH_THEN(fun uth ->
   let utm = lhand(concl uth)              (* the write spine:  write c1 y1 (... s) *)
   and thl = map snd (fst gl) in
   let s = repeat rand utm in              (* the old state variable *)
   let newthms =
     if memorable utm then
       let thy,thn = partition (vfree_in s o concl) thl in
       let cxt = (NONOVERLAPPING_DRIVERS thn,FILTER_CANONIZE_ASSUMPTIONS thn) in
       mapfilter (fun thi -> STATE_UPDATE_RULE cxt uth thi) thy
     else
       let thy = filter (vfree_in s o concl) thl in
       mapfilter (fun thi -> STATE_UPDATE_RULE ([],[]) uth thi) thy in
   MP_TAC uth THEN MAP_EVERY ASSUME_TAC newthms)) gl
```

with the rule it applies (`components.ml:3310`):

```ocaml
let STATE_UPDATE_RULE cxt uth th =
  let utm,s' = dest_eq(concl uth) and tm = concl th in
  let s = repeat rand utm in
  if not(vfree_in s tm) then failwith "STATE_UPDATE_RULE" else
  let th0 = SUBS_CONV[uth] (vsubst[utm,s] (concl th)) in
  let th1 = CONV_RULE(LAND_CONV
    (COMPONENTS_READ_OVER_WRITE_ORTHOGONAL_CONV cxt)) th0 in
  EQ_MP th1 th
```

**What it rewrites, after every step.** Every hypothesis that mentions the old state variable
`s` is pulled forward: `uth` replaces `s` by `spine s` (`SUBS_CONV`), then every `read c (…)`
in the hypothesis is pushed back through the spine with
`COMPONENTS_READ_OVER_WRITE_ORTHOGONAL_CONV`, discharging one orthogonality/non-overlap
obligation per `(read, write)` pair, and the result is a hypothesis about `s'`.

**Why it costs ~8.4 K per step** (measured): it *attempts* ~16 hypotheses per step
(303 attempts / 19 steps), of which ~12.5 succeed. Mean cost per successful propagation is
**488** inferences. Across the 19 steps there are 61 writes and 924 `(attempt, write)` pairs,
against 159,437 spent: ≈ **173 inferences per (fact, write) pair** — the price of
`NONOVERLAPPING_DRIVERS`/`COMPONENTS_READ_OVER_WRITE_ORTHOGONAL_CONV` per write. Mean cost per
*step* is 159,456/19 = **8,392**.

### 2.2 `MAYCHANGE_STATE_UPDATE_TAC` (`common/relational.ml:1006`)

```ocaml
let MAYCHANGE_UPDATE_RULE =
  let JUSTADD_CONV = GEN_REWRITE_CONV I [MAYCHANGE_LID] ORELSEC
                     GEN_REWRITE_CONV TOP_DEPTH_CONV [GSYM SEQ_ASSOC] in
  let MAYCHANGE_SIMPLE_ABSORB_CONV tm =
    match tm with
      Comb(Comb(Const(",,",_) as cop,ltm),rtm) ->
          let lth = MAYCHANGE_CANON_CONV ltm        (* whole accumulated prefix *)
          and rth = MAYCHANGE_CANON_CONV rtm in
          let tm' = ... in
          let n,eth = ASSIGNS_MOVE_BACK_CONV ASSIGNS_IDEMPOT_CONV tm' in
          if n <> 1 then JUSTADD_CONV tm else ...     (* absorb, or just append *)
    | _ -> failwith "MAYCHANGE_SIMPLE_ABSORB_CONV" in
  let rec BASIC_MAYCHANGE_UPDATE_RULE tm th =
      match tm with
        Comb(Comb(Comb(Const("write",_),c),y),s) ->
           let th1 = BASIC_MAYCHANGE_UPDATE_RULE s th in
           let th2 = ISPECL [c;y] (MATCH_MP MAYCHANGE_WRITE th1) in
           CONV_RULE(RATOR_CONV(RATOR_CONV MAYCHANGE_SIMPLE_ABSORB_CONV)) th2
      | _ -> th in
  fun uth th -> ... EQ_MP (AP_TERM (rator(concl th')) uth) th'

let MAYCHANGE_STATE_UPDATE_TAC =
  DISCH_THEN(fun uth -> MP_TAC uth THEN
    FIRST_X_ASSUM(ASSUME_TAC o MAYCHANGE_UPDATE_RULE uth o
           check(maychange_term o concl)))
```

**What it rewrites.** The frame assumption `MAYCHANGE [..] ,, .. s0 s` is extended, one
`MAYCHANGE_WRITE` (`R s0 s1 ==> (R ,, MAYCHANGE [c]) s0 (write c y s1)`) per written
component, and after each extension `MAYCHANGE_SIMPLE_ABSORB_CONV` **canonicalises the entire
accumulated prefix** (`MAYCHANGE_CANON_CONV ltm`), then moves the new component back through
the list (`ASSIGNS_MOVE_BACK_CONV`) merging it if already present (`ASSIGNS_IDEMPOT_CONV`).
Start state is `MAYCHANGE [] s0 s0` (from `MAYCHANGE_STARTER`, added by `ENSURES_INIT_TAC`).

**Why it costs ~10.2 K per step** (measured): 61 writes over 19 steps, 193,789 inferences,
~3,177 per write. The accumulated assumption grows from 1 to 14 `MAYCHANGE` nodes
(12 distinct components) and the cost tracks the sink steps:

| step | MAYCHANGE nodes | cost | note |
|---|---:|---:|---|
| s1 | 1 | 3,590 | 4 writes |
| s4 | 6 | 22,477 | 6 writes (SUBS: VF,CF,ZF,NF,X2,PC) |
| s7 | 10 | 19,854 | LDP: X4,X5,PC + events |
| s9 | 12 | 13,158 | SBCS, 6 writes |
| s18 | 12 | 23,528 | STP: **memory :> bignum(z,4)** + 2 regs + PC |
| s19 | 14 | 29,704 | STP + memory again |
| total | | **193,789** | |

The two store steps alone are 53,232 (27.5 %) of the bucket: moving the large
`memory :> bignum(z,4)` component back through the list is the expensive comparison.

### 2.3 Where the 159,456 goes (measured per assumption)

`neg_state` wraps `STATE_UPDATE_RULE` per hypothesis (`.bench/xxx-prop/run_state.log`):

| category | attempts | success cost | mean | notes |
|---|---:|---:|---:|---|
| memory reads (`read (memory :> bytes…) s = n_i`) | 74 | 59,595 (70 ok) | 851 | 4 facts carried for 17 steps |
| register reads | 131 | 44,414 (119 ok) | 373 | |
| flag reads | 60 | 11,644 (48 ok) | 242 | |
| `read PC` | 19 | **0** (19 fail) | — | always superseded |
| the `MAYCHANGE` fact itself | 19 | **0** (19 fail) | 23 each | `EQ_MP` failure |
| **failures, all categories** | **66** | **42,600** | 645 | **26.7 % of the bucket is failed work** |

Every failing term is an assumption about a component *the current step writes*:

```
read PC s0 = word pc                 (step 1 writes PC)
read X2 s3 = n_0                     (step 4 writes X2)
read CF s5 <=> ~carry_s4             (step 6 writes the flags)
read X7 s4 = word 18446744073709551597   (step 5 writes X7)
read (memory :> bytes64 x) s17 = n_0 (step 18 writes memory(z,4))
```

i.e. the framework spends 42,600 inferences proving that a fact about a component it is about
to overwrite cannot be carried through the overwrite. That is the demand-driven opening.

## 3. The three prototypes (whole 19 steps, one binary, `PROP_MODE`)

All three replace only the per-step tactic in a scratch copy of the assembled proof; the
framework files are untouched. Each produces the same hypothesis set at the final algebra
bridge (`diff` of the 7 final hypotheses: identical) and passes `STATEMENT_OK aconv=1
hyps=0 axioms=3`, so the final theorem is aconv-identical.

### 3.1 P1 — defer the memory components. **Fails.**

Rule tried: in `ASSUMPTION_STATE_UPDATE_TAC`, if the step's update record contains no `memory`
write, do not re-derive memory assumptions (an unwritten component cannot have changed).

```
PROP_MODE=assume   rc=2   STATEMENT_OK absent
ANSWER_EXCEPTION in BRIDGE_NEG_REF_ALG_TAC: REAL_ARITH … linear_ineqs: no contradiction
```

**Why it is not merely unprovable but *breaks the invariant*.** Evidence
(`.bench/xxx-prop/run_assume.log` vs `run_shipped.log`):

- `NEWRULE` lines are byte-identical in both modes: the same antecedent, the same 2–6 new
  facts per step. So the new facts *are* created.
- Yet at `s9:preacc` the assumptions `read X4 s9 = …`, `read X5 s9 = n_3`,
  `read X6 s9 = word_or …` and the four flag facts are **absent**; at the final bridge 5 of 7
  hypotheses remain, and the missing two are the carry equations for `s9` and `s12`.
- Mechanism: a load's fact is `read X5 s7 = read (memory :> bytes64 (…x…)) s7`. It survives to
  the next step only because the propagated memory fact `… = n_3` rewrites it to a
  **state-free** `read X5 s7 = n_3`. `DISCARD_OLDSTATE_TAC` keeps a hypothesis only when its
  reads mention nothing but the current state (`us = []` or `us = [v]`); without the memory
  facts the fact keeps a stale `read … s8` inside it, so `DISCARD` deletes it at the next step.
  (`ACCUMULATEX_ARITH_TAC`'s `pth_sbc`/`pth_adc` do accept ungrounded operands — `x`,`y` are
  variables — so the failure is purely the missing fact, not pattern strictness.)

So the memory facts are load-bearing — they are the only thing that turns a load into a numeral
— and the tempting 59,595 saving is not available: **the memory component cannot be deferred in
isolation**, because it is the ground of the register facts. It does not follow that the whole
80.8 % is irreducible; it follows that per-component deferral is the wrong axis (see §4.1).

*Related measurement:* the memory facts are re-derived at essentially every step until the
stores (70 successful propagations over steps 1–18), not once at the end, precisely because
each new load fact needs re-grounding. For these facts "resolve on demand" coincides with
"resolve every step".

### 3.2 P2 — defer `MAYCHANGE` canonicalisation. **Sound, slower.**

`MAYCHANGE_WRITE` is one `ISPECL`+`MATCH_MP` per write; drop the `CONV_RULE` entirely and let
the single already-required final subsumption (`MONOTONE_MAYCHANGE_TAC` inside
`ENSURES_FINAL_STATE_TAC`) do all the merging once.

| mode | sim | maychange | ENSURES_FINAL | **STAMP_INFERENCES** | statement |
|---|---:|---:|---:|---:|---|
| shipped | 528,083 | 193,789 | 142,511 | **2,380,410** | ok |
| `maychg` (no absorb at all) | **337,726** | **1,967** | **409,555** | **2,457,097** | ok |
| `mcfast` (absorb new item, don't canon the prefix) | 474,331 | 138,572 | 409,120 | 2,593,267 | ok |

**Why deferral loses.** The shipped incremental absorption keeps the accumulated predicate short
and canonical enough: the measured node count stays at 1 → **14 groups** (12 distinct
components) for 61 writes, so each write costs an `ASSIGNS_MOVE_BACK` over ≤ 14 items. Removing
it lets the list become the raw 61-write history; the one-shot merge at the end must then
canonicalise and idempotence-check a 61-entry chain (`MAYCHANGE_IDEMPOT_TAC` →
`ASSIGNS_MOVE_BACK_CONV`), which is quadratic. Measured: −190,357 in simulation, **+267,044**
at the end. The `mcfast` hybrid shows the same: keeping the prefix un-canonicalised saves 55 K
in simulation and costs the same 267 K at the end.

This is the direct answer to "keep the state as an unevaluated update record and resolve only at
the end" **for the frame**: the shipped tactic already *is* the cheap incremental strategy, and
the expensive final merge is the price of not normalising per step.

### 3.3 P3 — do not re-derive what the step overwrites. **Works.**

Rule (the "supersede" rule): compute the written component set from the update record
(`write c …` spine) and skip any hypothesis whose conclusion reads one of those components with
the old state. Such a hypothesis cannot be carried (the shipped tactic already fails on it) and
is superseded by the `STATE_UPDATE_NEW_RULE` fact for the same component.

```ocaml
let prop_superseded uth s thi =
  let ws = written components of (lhand (concl uth)) in
  let rs = components c with a subterm `read c s` in (concl thi) in
  exists (fun r -> exists (aconv r) ws) rs;;
(* used as a filter in both branches of ASSUMPTION_STATE_UPDATE_TAC *)
```

Whole-proof, same binary:

```
shipped    STAMP_INFERENCES 2380410   sim 528083   assume 159456   STATEMENT_OK aconv=1 hyps=0 axioms=3
supersede  STAMP_INFERENCES 2345520   sim 493193   assume 121843   STATEMENT_OK aconv=1 hyps=0 axioms=3
```

**−34,890 inferences (1.47 % of the proof, 6.6 % of the simulation).** Per step:

| st | assume shipped | assume supersede | Δ | st | assume shipped | assume supersede | Δ |
|---|---:|---:|---:|---|---:|---:|---:|
| s1 | 8,166 | 7,375 | 791 | s11 | 4,403 | 2,547 | 1,856 |
| s2 | 5,649 | 5,046 | 603 | s12 | 4,403 | 2,547 | 1,856 |
| s3 | 6,187 | 5,584 | 603 | s13 | 8,874 | 5,576 | 3,298 |
| s4 | 14,696 | 11,536 | 3,160 | s14 | 6,463 | 4,607 | 1,856 |
| s5 | 6,497 | 4,641 | 1,856 | s15 | 5,683 | 3,827 | 1,856 |
| s6 | 12,301 | 7,561 | 4,740 | s16 | 4,403 | 2,547 | 1,856 |
| s7 | 16,133 | 15,371 | 762 | s17 | 4,403 | 2,547 | 1,856 |
| s8 | 6,618 | 4,762 | 1,856 | s18 | 10,865 | 10,077 | 788 |
| s9 | 13,143 | 7,767 | 5,376 | s19 | 14,886 | 14,098 | 788 |
| s10 | 5,683 | 3,827 | 1,856 | **Σ** | **159,456** | **121,843** | **37,613** |

(The `MAYCHANGE` phase moves the other way by +2,723 total; cause not established, it is
concentrated on the same steps where the assumption set shrinks. Net of the two is −34,890.)

### 3.4 P3b — plus a liveness stand-in for the input-memory facts

A *general* liveness rule needs a read-set analysis over the decoded instruction sequence. To
size that win I used a program-specific stand-in: after the last load from `x` has executed
(step 7), drop all memory assumptions at steps 8–17. Sound (dropping hypotheses is never
unsound), but not a mechanism.

```
deadmem    STAMP_INFERENCES 2315554   sim 463787   assume 93165   STATEMENT_OK aconv=1 hyps=0 axioms=3
```

Per-step `assume` after the memory facts are gone: s9 2,035, s10 1,887, s11–s17 **607** each.
607 is essentially the single `read PC s = word (…+4)` fact (584) that the next fetch needs.
So the residual for a full liveness rule is ≈ 19 × 607 ≈ 11,500, i.e. a further ≈ 80,000 below
the measured 93,165 — the largest remaining lever on the assumption side.

## 4. Projection

Measured, this receipt (state propagation only, whole 19-step proof):

```
simulation   528,083 -> 463,787   (-64,296, -12.2%)
STAMP      2,380,410 -> 2,315,554 (-64,856,  -2.7%)
```

The step cache from `xxx-step-cache-feasibility.md` touches a disjoint phase (`basic`,
101,460 → 19 × 1,280 = 24,320, saving 77,140 measured there). Adding it:

```
projected simulation = 463,787 - 77,140 = 386,647
```

with the caveat that the cache's one-time derivation (13,214 shared + 6,715 × 9 opcodes =
73,649) makes it ~break-even on `neg` alone; it pays only across tasks.

With a full per-component liveness rule (projected from the 607/step floor): `assume` ≈
11,500, so simulation ≈ **382,000**; adding the cache again ≈ **305,000**.

Remaining buckets at the best measured point: `maychange` 196,512 (no reduction found),
`basic` 101,460 (cache → 24,320), `assume` 93,165 (liveness → ~11.5 K), `newrule` 33,637,
`nonself` 23,233, `acc`+`clarify` 15,780. The review's 529 K → 100 K needs the `MAYCHANGE`
bucket gone, and nothing in this receipt or the previous one removes it.

### 4.1 What a general mechanism needs

1. **A read-set / write-set analysis over the decoded sequence.** The supersede rule is the
   zero-analysis special case (a component written *now* is definitely superseded). The full
   win (ceiling −148 K on `assume`: 159,456 → ~11,500) needs *last-use* information so dead
   register and flag facts stop being carried; on this proof the measured floor is 607/step,
   one fact.
2. **Relax `DISCARD_OLDSTATE_TAC`.** It keeps a hypothesis only if its reads mention nothing
   but the current state, which is why facts must be re-grounded every step. Any deferral
   scheme must either allow "pure read of a preserved region" atoms to survive at an older
   state or ground them eagerly (as the shipped memory facts do).
3. **A frame representation that does not merge incrementally.** `193,789` is the cost of
   keeping a duplicate-free `MAYCHANGE` predicate by `ASSIGNS_MOVE_BACK`; deferring the merge
   costs more (267 K). A mechanism must carry the written-component set out-of-band (an ML
   ref, as the accumulator does) and prove `MAYCHANGE Frame s0 s19` once by a **linear**
   composition over the 61 writes, with the duplicate elimination factored out of the proof
   (e.g. a component-set lemma proved once per frame). The 53,232 spent on the two store steps
   says the `memory :> bignum(z,4)` comparison is where to start.
4. **Resolve reads by table lookup, not by proof.** 173 inferences per `(fact, write)` pair
   (924 pairs for 159,437) is the real "demand-driven" lever: a component-indexed substitution
   (a map from component to the last write in the spine) would make each pair a handful of
   inferences. That is a rewrite of how
   `COMPONENTS_READ_OVER_WRITE_ORTHOGONAL_CONV` is driven, not a tactic substitution.

### 4.2 The change that is ready to ship (if wanted)

The supersede rule is a 6-line addition to `ASSUMPTION_STATE_UPDATE_TAC` in
`common/components.ml` (build the written set from `utm`, filter both branches). It is
inference-neutral in output (identical hypothesis set at the final bridge, identical statement)
and saves 37,613 on `neg`. It should be generalised with a read-set parameter before being
trusted on other proofs — on a program where a step writes a component it later needs the
*old* value of, dropping is still sound but could lose the proof.

## 5. Not established

- The liveness win is measured only through a program-specific stand-in (`deadmem`); the
  general rule (last-use per component) was **not** implemented. The 607/step floor is a
  measured *bound*, not an achieved figure.
- The `+2,723` in the `MAYCHANGE` phase under the new assumption modes was not explained.
  It is 0.11 % of the proof and is included in every total quoted.
- Only `neg` was exercised. Whether the supersede rule generalises (e.g. to a loop, where a
  written component is read again later) is untested; on this straight-line program every
  overwritten fact is genuinely dead.
- The projection that adds the step cache (77,140) assumes the two savings are additive. They
  touch disjoint phases, but this was not measured in one binary.

## 6. Files and reproduction

Scratch, git-ignored (`.bench/xxx-prop/`):
`gen_probe_state.py` → `neg_state.ml/.native/.correct`, `run_state.log`;
`gen_demand.py` → `neg_demand.ml/.native`, `run_shipped.log`, `run_supersede.log`,
`run_deadmem.log`, `run_maychg.log`, `run_mcfast.log`, `run_assume.log`, `run_demand.log`,
`build_state.log`, `build_demand.log`.

```
cd /mnt/1tb_kingston/Tekmerion
free -g
python3 .bench/xxx-prop/gen_probe_state.py
cp .bench/xxx-prop/neg_state.ml .bench/dropin/arm/proofs/zzz_neg_state.ml
cd .bench/dropin/arm
../../../tools/cap.sh 12G -- bash ../tools/build-proof.sh proofs/zzz_neg_state.ml \
  /mnt/1tb_kingston/Tekmerion/.bench/hol-light-stamp2/hol.sh ../../xxx-prop/neg_state.native
cd ..                                     # cwd must be .bench/dropin (`needs` + the ELF)
../../tools/cap.sh 12G -- ../xxx-prop/neg_state.native

python3 /mnt/1tb_kingston/Tekmerion/.bench/xxx-prop/gen_demand.py
cp /mnt/1tb_kingston/Tekmerion/.bench/xxx-prop/neg_demand.ml \
   arm/proofs/zzz_neg_demand.ml
cd arm
../../../tools/cap.sh 12G -- bash ../tools/build-proof.sh proofs/zzz_neg_demand.ml \
  /mnt/1tb_kingston/Tekmerion/.bench/hol-light-stamp2/hol.sh ../../xxx-prop/neg_demand.native
cd ..
for m in shipped supersede deadmem maychg mcfast; do
  PROP_MODE=$m ../../tools/cap.sh 12G -- ../xxx-prop/neg_demand.native
done
```

The `zzz_*.ml` copies placed in `.bench/dropin/arm/proofs/` are untracked and were removed after
the runs. No tracked file in the repository was modified except this receipt; the framework
(`.bench/dropin/common/*.ml`, `.bench/dropin/arm/proofs/arm.ml`), the shipped pipeline,
`tools/bridge.ml` and `src/` were not changed. The pre-existing ` M TODO.md` was left as found.

## 7. One-line verdict

```
STATE PROP: ASSUMPTION_STATE_UPDATE_TAC 8392 -> 6413 per step (supersede rule: skip facts the step overwrites; measured whole-19-step STAMP 2380410 -> 2345520, STATEMENT_OK), 4903 with a liveness stand-in (2315554); MAYCHANGE_STATE_UPDATE_TAC 10199/step NOT reducible (deferring the canonicalisation is sound but costs 267044 at the end, net +76687); projected neg simulation 528083 -> 386647 with the step cache — the ~100K target stays blocked: 80.8% of the simulation is per-step propagation, whose frame bucket cannot be deferred and whose assumption bucket shrinks only 12% (supersede) to 42% (liveness)
```
