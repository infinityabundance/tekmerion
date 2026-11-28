# XXX.STEP — is a per-opcode step-theorem cache feasible for `neg`?

- **Date:** 2026-11-28
- **Branch:** `staging`, HEAD `1747887` (clean; `main` untouched; nothing committed, nothing pushed)
- **Purpose:** establish *how* `arm/neg_p25519`'s simulation is performed, whether one
  generic step theorem per opcode can be derived once from the ARM model and instantiated
  per occurrence, prototype it for one opcode, and project the whole-proof saving.
- **Verdict:** **FEASIBLE AND MEASURED — but it cannot reach the review's target.**
  For `SBCS` the per-occurrence cost falls **7,392 → 1,280** (s6) and **7,418 → 1,280**
  (s9), one-time derivation **20,502** (or **6,715 per opcode** after a shared **13,214**).
  Two cached occurrences cut the measured simulation **528,083 → 515,833** and
  `STAMP_INFERENCES` **2,414,817 → 2,402,567** with `STATEMENT_OK`.
  But the instruction-expansion bucket is only **101,460 (19.21 %)** of the simulation;
  the other **426,623 (80.79 %)** is the framework's per-step state propagation
  (`MAYCHANGE_STATE_UPDATE_TAC` 193,789, `ASSUMPTION_STATE_UPDATE_TAC` 159,456, …).
  So the best possible simulation total from *any* per-opcode step cache is **428,029**,
  not ~100 K.

All numbers are from scratch files under `.bench/xxx-step/` (git-ignored). The shipped
pipeline, `tools/bridge.ml`, `src/` and the fixtures were not changed. The only new tracked
file is this receipt.

---

## 1. Resources, commands, exit codes

### 1.1 Resources

```
free -g   (before the proof builds)
              total  used  free  shared  buff/cache  available
Mem:            125    28    84       0          15         96
Mem:            125    28    81       0          15         97
df -h /mnt/1tb_kingston                                         264G available
```

The cap was never raised and never fired. Highest observed group peak RSS:

```
cap.sh: group=3406839 rc=0 sampled_group_peak_rss_kb=2073532 max_process_vmhwm_kb=1886224
```

### 1.2 One session at a time, capped, sequential

Every build and every run went through `tools/cap.sh 12G --` and was run alone. The proof
product is a `.native` produced by the same recipe the s2n-bignum Makefile's `%.native` rule
uses (`../tools/build-proof.sh`); it is *one* target, built with `-j1` semantics (one
`ocamlopt` invocation, no parallel targets), never raised.

| step | command | rc |
|---|---|---:|
| build scratch proof | `cd .bench/dropin/arm && ../../../tools/cap.sh 12G -- bash ../tools/build-proof.sh proofs/zzz_neg_measured.ml <HOL>/hol.sh ../../xxx-step/neg_measured.native` | 0 |
| run (decomposition) | `cd .bench/dropin && ../../tools/cap.sh 12G -- ../xxx-step/neg_measured.native` | 0 |
| build cache proof | same recipe, `proofs/zzz_neg_cache.ml` → `neg_cache.native` | 0 |
| run measure mode | `STEPCACHE_MODE=measure ../../tools/cap.sh 12G -- ../xxx-step/neg_cache.native` | 0 |
| run cache mode | `STEPCACHE_MODE=cache ../../tools/cap.sh 12G -- ../xxx-step/neg_cache.native` | 0 |

Build wall time 55 s (1.9 GB peak), proof run 25 s (0.17 GB peak). `<HOL>` is
`.bench/hol-light-stamp2`; the scratch `.ml` is a copy of
`.bench/dropin/arm/proofs/bignum_neg_p25519.ml` with only the simulation block and a header
block changed (generators: `.bench/xxx-step/gen_measured.py`, `gen_cache.py`).

### 1.3 Real proof runs

```
.bench/xxx-step/neg_measured.correct        STAMP_INFERENCES 2380410   STATEMENT_OK aconv=1 hyps=0 axioms=3
.bench/xxx-step/neg_cache.measure.correct   STAMP_INFERENCES 2414817   STATEMENT_OK aconv=1 hyps=0 axioms=3
.bench/xxx-step/neg_cache.cache.correct     STAMP_INFERENCES 2402567   STATEMENT_OK aconv=1 hyps=0 axioms=3
```

`neg_measured` is the shipped proof with the simulation block replaced by a byte-faithful
re-implementation of `ARM_XACCSTEP_TAC` plus sub-marks; it differs from the shipped
`2,380,714` by **304** (0.013 %) — marks/glue, no changed inference-producing call.

`neg_cache` adds the generic-theorem derivations in the header (before
`stamp_before_…`). Those derivations shift `STAMP_INF_BEFORE` from `67,649,313` to
`67,669,815` (+20,502 = the derivation cost) and raise the in-proof *library* bucket by
34,407 in **both** modes — a harness artefact of having extra theorems in the session,
identical in the two modes, so it cancels in the measure-vs-cache comparison. The clean
comparison is therefore within the same binary: **2,414,817 → 2,402,567, Δ 12,250**, and
`STATEMENT_OK` in both.

Scratch instrumentation is inference-neutral apart from the marks themselves: the
`STEPCACHE_MODE` switch only selects between two implementations of the same step, and
cache mode produced the same hypothesis-free statement (`aconv=1 hyps=0`).

---

## 2. How simulation is performed

The generated proof performs all 19 steps with **one tactic family, one call per step**
(from `.bench/ix89/gen/arm_neg_p25519.phase.answer`, lines 6389–6407; the assembled proof is
identical apart from the header):

```ocaml
csat_mark "B:step01:plain" THEN ARM_ACCSTEPS_TAC BIGNUM_NEG_P25519_EXEC [4;6;9;12] (1--1) THEN
...
csat_mark "B:step04:acc"   THEN ARM_ACCSTEPS_TAC BIGNUM_NEG_P25519_EXEC [4;6;9;12] (4--4) THEN
...
csat_mark "B:step19:plain" THEN ARM_ACCSTEPS_TAC BIGNUM_NEG_P25519_EXEC [4;6;9;12] (19--19) THEN
```

`[4;6;9;12]` are the accumulate-carry steps (`ACCUMULATEX_ARITH_TAC`). The call chain
(`arm/proofs/arm.ml:579–628`) is:

```
ARM_ACCSTEPS_TAC = ARM_XACCSTEPS_TAC th [`SP`]
  ARM_XACCSTEP_TAC → ARM_SINGLE_STEP_TAC → ARM_VERBOSE_STEP_TAC → ARM_STEP_TAC
    ARM_BASIC_STEP_TAC              (* ARM_CONV: fetch/decode/execute expansion *)
    NONSELFMODIFYING_STATE_UPDATE_TAC (aligned_bytes_loaded_update (length th))
    ASSUMPTION_STATE_UPDATE_TAC      (* rewrite every assumption s→s' *)
    MAYCHANGE_STATE_UPDATE_TAC       (* rewrite the MAYCHANGE assumption *)
    DISCH_THEN (STATE_UPDATE_NEW_RULE uth THEN ASSEMBLER_SIMPLIFY_TAC THEN STRIP_TAC)
    DISCARD_OLDSTATE_TAC; CLARIFY_TAC
  (if accumulate: TRY (ACCUMULATEX_ARITH_TAC [`SP`] THEN CLARIFY_TAC))
```

`ARM_BASIC_STEP_TAC` is the only part that touches the ISA semantics. It calls
`ARM_CONV (decode_ths) (map snd asl) (arm s s')`, which
(1) matches the concrete decode theorem for the current `read PC` (`ARM_THM`,
`arm.ml:339`), then (2) runs `ONCE_DEPTH_CONV ARM_EXEC_CONV` and the component
read/write rewrites (`arm.ml:284–458`). It produces, e.g. for step 6, the equation
(measured in `.bench/xxx-step/probe0.log`):

```
|- arm s s' <=>
   write VF (~(ival (read X7 s) - (ival (read X3 s) + &(bitval (~read CF s))) = ival (word_sub …)))
    (write CF (~(val (read X7 s) < val (read X3 s) + bitval (~read CF s))))
     (write ZF (… = 0)
      (write NF (… < &0)
       (write X3 (word_sub (read X7 s) (word_add (read X3 s) (word (bitval (~read CF s)))))
        (write PC (word (pc + 24)) s))))) = s'
```

### 2.1 The 19 instructions

`bignum_neg_p25519.S` (machinery `[4;6;9;12]` in `SUBS`/`SBCS`/`SBCS`/`SBC`):

| step | instruction | class | step | instruction | class |
|---|---|---|---|---:|---|---|
| 01 | `LDP X2,X3,[X1]` | plain | 11 | `ORR X6,X6,X5` | plain |
| 02 | `MOVN X7,#18` | plain | 12 | `SBC X5,X7,X5` | acc |
| 03 | `ORR X6,X2,X3` | plain | 13 | `CMP X6,XZR` | plain |
| 04 | `SUBS X2,X7,X2` | acc | 14 | `CSEL X2,X2,XZR,NE` | plain |
| 05 | `MOVN X7,#0` | plain | 15 | `CSEL X3,X3,XZR,NE` | plain |
| 06 | `SBCS X3,X7,X3` | acc | 16 | `CSEL X4,X4,XZR,NE` | plain |
| 07 | `LDP X4,X5,[X1,#16]` | plain | 17 | `CSEL X5,X5,XZR,NE` | plain |
| 08 | `ORR X6,X6,X4` | plain | 18 | `STP X2,X3,[X0]` | plain |
| 09 | `SBCS X4,X7,X4` | acc | 19 | `STP X4,X5,[X0,#16]` | plain |
| 10 | `MOVN X7,#0x8000…` | plain | | | |

Nine distinct opcodes (`LDP, MOVN, ORR, SUBS, SBCS, SBC, CMP, CSEL, STP`); `RET` is handled
by the separate subroutine proof. No instruction repeats with the same operands, so *within*
one program there is no per-occurrence reuse at all — the cache is only a reuse mechanism
across the 9 shapes (and across tasks).

### 2.2 Per-step cost (phase marks, shipped run) and the sub-step split

Per-step totals are the shipped figures (`xxx-f-neg-unaccounted.md` §4). The sub-step split
below is measured in `.bench/xxx-step/neg_measured.correct` (`SUBMARK` after each phase;
sum = 528,083, vs 528,862 shipped, difference = inter-step glue not bracketed by `SUBMARK`):

| bucket | inferences | share of simulation |
|---|---:|---:|
| **`ARM_BASIC_STEP_TAC`** (`ARM_CONV`: ISA expansion + step rewrite) | **101,460** | **19.21 %** |
| `NONSELFMODIFYING_STATE_UPDATE_TAC` | 23,409 | 4.43 % |
| `ASSUMPTION_STATE_UPDATE_TAC` | 159,456 | 30.19 % |
| `MAYCHANGE_STATE_UPDATE_TAC` | 193,789 | 36.70 % |
| `STATE_UPDATE_NEW_RULE` + `ASSEMBLER_SIMPLIFY_TAC` | 33,637 | 6.37 % |
| `DISCARD_OLDSTATE_TAC` | 0 | 0.00 % |
| `CLARIFY_TAC` | 2,395 | 0.45 % |
| `ACCUMULATEX_ARITH_TAC` (4 acc steps) | 13,937 | 2.64 % |
| **total** | **528,083** | 100.00 % |

Per step (`basic` column; the rest is the framework's propagation):

| st | total | basic | assume | maychg | newrule | | st | total | basic | assume | maychg | newrule |
|---|---:|---:|---:|---:|---:|---|---|---:|---:|---:|---:|---:|
| s1 | 22,404 | 8,316 | 8,166 | 3,590 | 1,221 | | s11 | 13,293 | 3,022 | 4,403 | 4,437 | 696 |
| s2 | 16,860 | 5,515 | 5,649 | 4,299 | 745 | | s12 | 16,624 | 3,325 | 4,403 | 3,800 | 808 |
| s3 | 15,121 | 2,970 | 6,187 | 4,614 | 688 | | s13 | 27,767 | 6,993 | 8,874 | 8,745 | 1,685 |
| s4 | 52,993 | 5,614 | 14,696 | 22,477 | 5,063 | | s14 | 18,557 | 3,310 | 6,463 | 7,133 | 910 |
| s5 | 20,178 | 5,555 | 6,497 | 6,674 | 745 | | s15 | 16,440 | 3,310 | 5,683 | 5,796 | 910 |
| s6 | 43,720 | 7,392 | 12,301 | 15,491 | 3,493 | | s16 | 13,228 | 3,322 | 4,403 | 3,852 | 910 |
| s7 | 47,148 | 8,657 | 16,133 | 19,854 | 1,301 | | s17 | 13,190 | 3,336 | 4,403 | 3,800 | 910 |
| s8 | 18,477 | 3,163 | 6,618 | 7,275 | 692 | | s18 | 50,146 | 7,013 | 10,865 | 23,528 | 4,192 |
| s9 | 42,407 | 7,418 | 13,143 | 13,158 | 3,493 | | s19 | 61,084 | 7,512 | 14,886 | 29,704 | 4,430 |
| s10 | 18,446 | 5,717 | 5,683 | 5,562 | 745 | | | | | | | |

**The decisive fact:** `ARM_CONV` (the instruction semantics) is **19.2 %** of the
simulation. The two largest buckets, `MAYCHANGE_STATE_UPDATE_TAC` and
`ASSUMPTION_STATE_UPDATE_TAC` (66.9 % combined), are the framework re-deriving the evolving
hypothesis set after each step. They are per-occurrence by construction (they depend on the
accumulated assumptions and on the current update term), and no instruction-semantics
theorem removes them.

---

## 3. The generic step theorem (one derivation per opcode)

### 3.1 What the model exposes

`arm/proofs/arm.ml` exposes exactly the pieces needed:

- `arm_decode s pc inst` is a definition (`?i:int32. aligned_bytes_loaded s pc … /\ decode i = SOME inst`, `arm.ml:17`), so it can be *assumed* rather than discharged;
- `arm_decode_unique` (`arm.ml:38`) makes the decode functional;
- `arm s s' <=> ?instr. arm_decode s (read PC s) instr /\ (PC := … ,, arm_execute instr) s s'` (`arm.ml:158`);
- the ISA semantics are functions `arm_ADCS`, `arm_SBCS`, … (`instruction.ml:809, 1514`) with algebraic forms `arm_SBCS_ALT`, `arm_ADCS_ALT`, … (`instruction.ml:3294–3360`);
- `SEQ : ((c := y) ,, r) s = r (write c y s)` and `assign` (`common/relational.ml:97–102`) flatten the state-update spine;
- `orthogonal_components` / `COMPONENT_READ_OVER_WRITE_CONV` (`common/components.ml:3156, 3227`) move reads past unrelated writes.

A generic step theorem is therefore a genuine theorem, not a schema: quantify the operands,
take `arm_decode` as a hypothesis, and rewrite with the semantic definition. The only
side condition needed is that the PC write does not overlap the registers the instruction
reads (true for every component of the model, but not provable for a *variable* component).

### 3.2 The opcode-independent fetch rule (derived once)

```ocaml
let ARM_STEP_GEN = prove
 (`!s (s':armstate) (pc:num) instr.
     read PC s = word pc /\ arm_decode s (word pc) instr
     ==> (arm s s' <=> (PC := word (pc + 4) ,, instr) s s')`,
  REPEAT GEN_TAC THEN STRIP_TAC THEN
  REWRITE_TAC[arm] THEN ASM_REWRITE_TAC[GSYM WORD_ADD; arm_execute] THEN
  ASM_MESON_TAC[arm_decode_unique]);;
```

Cost **4,517** once per session, for every opcode.

### 3.3 The prototype opcode: `SBCS`

`SBCS` is used twice (`s6` `SBCS X3,X7,X3`, `s9` `SBCS X4,X7,X4`), so one theorem
demonstrates both derivation and reuse. The semantics is unfolded with `arm_SBCS_ALT`,
`let_CONV`, `SEQ`, `assign`; the two reads of the PC-write are moved back with the
orthogonality lemmas. The result (`probe5.log`):

```
|- orthogonal_components PC Rn
   ==> orthogonal_components PC Rm
   ==> orthogonal_components PC CF
   ==> ((PC := word (pc + 4) ,, arm_SBCS Rd Rm Rn) s0 s' <=>
        write VF
         (~(ival (read Rm s0) - (ival (read Rn s0) + &(bitval (~read CF s0))) =
            ival (word_sub (read Rm s0)
                   (word_add (read Rn s0) (word (bitval (~read CF s0)))))))
         (write CF (~(val (read Rm s0) < val (read Rn s0) + bitval (~read CF s0)))
          (write ZF (val (word_sub (read Rm s0)
                          (word_add (read Rn s0) (word (bitval (~read CF s0))))) = 0)
           (write NF (ival (word_sub (read Rm s0)
                           (word_add (read Rn s0) (word (bitval (~read CF s0))))) < &0)
            (write Rd (word_sub (read Rm s0)
                        (word_add (read Rn s0) (word (bitval (~read CF s0)))))
             (write PC (word (pc + 4)) s0))))) = s'))
```

Note `Rd` never appears on a right-hand side (SBCS reads only `Rm`, `Rn`, `CF`), so
`orthogonal_components PC Rd` is not needed and drops out of the theorem.

### 3.4 One-time derivation cost

`DERIVE_STAGE` deltas from `.bench/xxx-step/neg_cache.measure.correct`:

| stage | inferences | reusable |
|---|---:|---|
| `arm_step` (fetch rule) | 4,517 | once per session, all opcodes |
| `orth_pth` (`read c (write d y s) = read c s`) | 298 | once per session |
| `orth_sym` | 8,399 | once per session |
| `sbcs_expand` (semantics → state update) | 6,371 | once per **opcode** |
| `orth_rewrite` (side-conditioned form) | 344 | once per **opcode** |
| **total** | **20,502** | (573 in definitions/`ASSUME`s) |

Amortised: **13,214 shared** + **6,715 per opcode**. (`orth_sym`'s 8,399 is a `MESON_TAC`
that a cheaper proof would remove; it is included as measured.)

### 3.5 Per-occurrence cost vs the current expansion

The cached path is: find the pc and `aligned_bytes_loaded` assumptions exactly as `ARM_CONV`
does, `MATCH_MP` the concrete decode theorem, `INST` the generic theorem
(`s0`, `s'`, `pc`, `Rd`, `Rm`, `Rn`), `discharge_orth` with the (module-cached) concrete
orthogonality facts, `TRANS`, one `ADD_ASSOC`/`NUM_REDUCE` normalisation. It yields a
theorem **aconv-identical** to `ARM_CONV`'s `eth` (`P5 ACONV=true`), so it is a drop-in
replacement for `ARM_BASIC_STEP_TAC`'s expansion; the rest of `ARM_STEP_TAC` is unchanged.

| occurrence | current `basic` (`ARM_CONV` + glue) | cached (`ethbuild` + glue) | saving |
|---|---:|---:|---:|
| s6 `SBCS X3,X7,X3` | 7,392 | 1,206 + 74 = 1,280 | **6,112** |
| s9 `SBCS X4,X7,X4` | 7,418 | 1,206 + 74 = 1,280 | **6,138** |

Measured end to end (same binary, `STEPCACHE_MODE=measure` vs `=cache`, only s6 and s9
switched):

```
simulation (05 play → 06 finalisation)   528,083  →  515,833   (Δ 12,250 = 6,112 + 6,138)
STAMP_INFERENCES                       2,414,817 →  2,402,567   (Δ 12,250)
STATEMENT_OK                                 ok   →       ok   (aconv=1 hyps=0 axioms=3)
```

The cached steps' sub-marks (`s6`: `ethbuild` 1,206, `basic` glue 74; `s9` identical) show
the replacement is cheap and constant; the unchanged framework phases (`nonself`, `assume`,
`maychange`, `newrule`, `clarify`, `acc`) reproduce their measured values to the inference.

---

## 4. Projected saving, and what a general mechanism needs

### 4.1 Projection

Holding the per-occurrence replacement cost at the measured SBCS value (1,280) and keeping
every framework phase:

```
projected saving   = Σ basic − 19 × 1,280 = 101,460 − 24,320 = 77,140
projected simulation = 528,862 − 77,140 = 451,722      (measured harness basis: 450,943)
```

**Hard ceiling.** Even if the cached `eth` were free (0 inferences), only `basic` can be
removed: `528,083 − (101,460 − 19×74) = 428,029`. The review's **529 K → 100 K** needs a
**428,083** saving, i.e. **4.28×** the entire instruction-expansion bucket. It is not
reachable by any per-opcode step theorem, however cheap.

**One-time amortisation.** For `neg` alone the cache is roughly break-even: 9 distinct
opcodes cost `13,214 + 9×6,715 = 73,649` one-time against a 77,140 saving. It only pays
across tasks, where each further `neg`-shaped proof reuses the 9 opcode theorems.

### 4.2 What a general mechanism requires

1. **One semantics→update theorem per opcode shape.** `sbcs_expand` is a mechanical
   conversion (`alt` lemma, `let_CONV`, `SEQ`, `assign`); the same recipe applies to the
   other 8 shapes (`LDP`, `STP`, `MOVN`, `ORR`, `SUBS`, `SBC`, `CMP`, `CSEL`). `LDP`/`STP`
   read/write memory and will need their address/value terms kept symbolic; `CSEL` drags in
   `condition_semantics`. Fox's ~318 figures correspond to full-ISA coverage of this step.
2. **A shared side-condition layer.** The fetch rule (4,517), the read-over-write lemma
   (298) and the orthogonality facts (`ORTHOGONAL_COMPONENTS_CONV` has its own
   `(const,const)` cache, ~149 for the first `PC`/`X3` pair) are opcode independent.
3. **A drop-in `eth` builder.** The occurrence must produce the same equation `ARM_CONV`
   produced, because everything after `ARM_BASIC_STEP_TAC` is syntactic. This prototype
   achieves exact `aconv` equality at 1,280 inferences, so the framework is untouched.
4. **The lever that is actually missing.** To approach 100 K one must attack the 80.8 %
   that is *not* instruction expansion — principally `MAYCHANGE_STATE_UPDATE_TAC`
   (193,789) and `ASSUMPTION_STATE_UPDATE_TAC` (159,456). Both re-derive, at every step,
   the whole evolving hypothesis set from the state-update equation. A step cache does not
   touch them. Candidates are: (a) keep goals in terms of `s0` and compose the updates at
   the end instead of rewriting every assumption forward each step; (b) cache per-opcode
   *update-rule* fragments so `MAYCHANGE` and `STATE_UPDATE_NEW_RULE` are one
   instantiation rather than a spine walk + disjunction canonicalisation. Neither is a step
   theorem; both are framework changes.

---

## 5. Not established

- Other opcodes were **not** derived; their `sbcs_expand`/`ethbuild` costs are assumed
  comparable to `SBCS`'s. The projection is therefore a projection, not a measurement of a
  full 19-step cached proof.
- The 428,029 ceiling is exact only for the measured harness (528,083); on the shipped
  528,862 it is 428,808.
- The +34,407 shift in the in-proof *library* bucket caused by having extra derived theorems
  in the session is an artefact of this harness and was not investigated further; it is
  identical in both modes and cancels in the comparison.
- `orth_sym`'s 8,399 (`MESON_TAC`) would be cheaper as a stored theorem; included as
  measured.

---

## 6. Files and reproduction

Scratch (git-ignored, `.bench/xxx-step/`):
`probe0..5.ml/.log` (model shape, generic derivation, staging), `gen_measured.py`,
`gen_cache.py`, `neg_measured.ml/.correct`, `neg_cache.ml/.native/.correct` (two modes),
`decomp.txt`.

```
cd /mnt/1tb_kingston/Tekmerion
free -g
python3 .bench/xxx-step/gen_cache.py
cp .bench/xxx-step/neg_cache.ml .bench/dropin/arm/proofs/zzz_neg_cache.ml
eval "$(opam env --switch .bench/hol-light-stamp2 --set-switch)"
cd .bench/dropin/arm
../../../tools/cap.sh 12G -- bash ../tools/build-proof.sh proofs/zzz_neg_cache.ml \
  /mnt/1tb_kingston/Tekmerion/.bench/hol-light-stamp2/hol.sh ../../xxx-step/neg_cache.native
cd ..                      # cwd must be .bench/dropin for `needs` and the ELF
STEPCACHE_MODE=measure ../../tools/cap.sh 12G -- ../xxx-step/neg_cache.native
STEPCACHE_MODE=cache   ../../tools/cap.sh 12G -- ../xxx-step/neg_cache.native
```

The `zzz_*.ml` copies placed in `.bench/dropin/arm/proofs/` are untracked and were removed
after the runs. This work modified no tracked file in either repository; the pre-existing
(uncommitted) drops in `.bench/dropin/arm/proofs/*.ml` were left exactly as found.

---

## 7. One-line verdict

```
STEP CACHE: SBCS expansion 7392/7418 -> 1280 per occurrence (derive once 20502, or 6715/opcode after 13214 shared); projected neg simulation 528862 -> 451722 (ceiling 428029) — the ~100K target is unreachable because 80.8% of the simulation is per-step assumption/MAYCHANGE propagation, not ISA expansion
```
