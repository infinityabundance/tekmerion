# Tekmerion

A proof-verification programme aimed at beating the published frontier on measured,
externally adjudicated criteria — without overstating what has been established.

**Start here: [`research/00-VERDICT.md`](research/00-VERDICT.md).**

---

## Release 0.2.0 (2026-11-26)

Published on crates.io as `tekmerion 0.2.0`. What is actually demonstrated, and
what is not, is summarised in [`CHANGELOG.md`](CHANGELOG.md). In brief:

- **Solver-free machine-code proofs**, *opt-in and currently 3 of the 27 tasks* (the other 24
  still use the certified-SAT route): ARM Curve25519 add 5,745,767, sub 9,029,510, neg
  5,683,733 primitive inferences, against 111,523,776 / 87,971,295 / 33,910,447 on the sliced
  SAT route (19.4×, 9.7×, 6.0×) and 1,848,494 for the human `add` proof (3.1×). Zero solver
  invocations, checked with a failing `cadical` shim on `PATH`.
  *Measured at `c6f5106` (`sub`/`neg` re-run today; `add` at `81f84db`, same bridge). Totals are
  only comparable within a commit — the inlined support library grows, which shifts every answer
  by a constant.*
- **Later measurement (append-only, 2026-11-27, `cdd673f`, receipt `ix91`):** the XXIII.B1
  per-carrier aggregate premise fast path reduces the strict self-contained totals to **add
  5,268,391** (2.85× human), **sub 8,217,594** (6.40×), **neg 5,683,733** (7.88×, unchanged —
  its proof has one aggregate, so the selection is a no-op). The published crates.io 0.2.0
  artefact predates this; these are repository measurements, not a crate release.
- **Latest measurement (append-only, 2026-11-28, `9983f9c`, receipts `ix92`–`ix101`):** the slim
  close, the refutations, the complement/identity/Farkas certificates, the hoisted aggregate fact,
  the width-independent carry-bound certificate, the entry hoist and canonical-row replay bring the
  strict totals to **add 3,885,488** (**2.10×** human), **sub 4,831,144** (**3.77×**), **neg
  2,728,198** (**3.78×**). Cumulative since IX.89: add −32.4 %, sub −46.5 %, neg −52.0 %. Neg's
  close is **2,057** (certificate-only) and its fact phase **1,511,696 → 13,572**. All certificates
  are derived-rule kernel proofs. **Statement-checked in-run** (`aconv=1 hyps=0 axioms=3`), with a
  mutated-statement negative control and a hostile harness self-test (5/5).
- **Measured matched-session (append-only, 2026-11-27, `7acf887`, receipt `xxx-e`):** with the support
  library preloaded and kernel-checked before the counter (L7 = **1,496,597**), the task proofs cost
  **add 2,591,777 (1.40× human) · sub 3,472,959 (2.71×) · neg 1,351,423 (1.87×)**. `L7 + matched =
  strict` exactly on all three; every run is `STATEMENT_OK`, and a no-library control fails to build.
- **27/27 generated proofs** accepted by upstream s2n-bignum's own build, including
  upstream's `_SUBROUTINE_CORRECT` derived from ours (same contract, same names,
  whole-tree `make` green).
- **First proof-object export and independent Rust replay**: `ADD_SYM`, 227
  reachable nodes, 7,039 bytes. Provisional — the checker is well-formed, not proved
  sound.
- **Freeze `v6` sealed** (`freeze-rust-closure-v6`): source and runtime closure plus
  the bound `TEKMERION_*` configuration.

Not demonstrated, and stated as such: no native machine-code theorem yet, G1–G3
soundness obligations open, `x86/add` exhausts memory, `arm/add_p256` emits no
aggregate, and coverage is still the same 27 routines.

---

## The short version

The brief's target — beat 6.35% on `s2n-bignum-bench` — is not the frontier. The same
published abstract reports 10.26% for a five-attempt feedback loop, and all original
baselines solve **0 of 864** functional-correctness tasks. The real opening is that
category, not the aggregate. Three of the brief's "no published number" rows are already
occupied, and its 2,633-line figure does not reproduce against the current tree.

What is left, once those are removed, is a smaller and better programme:

| | Claim | Adjudicated by | Status |
|---|---|---|---|
| **C1** | Lossless **failure-side** trajectories on an industrial machine-code proof corpus | Conformance against `TacticTrace`; replay | Specified (`research/03-INSTRUMENT.md`) |
| **C2** | First replay-checked solve rate on the **functional-correctness** category | Public leaderboard, `CHEATING` check | Preregistered (`research/02-EXPERIMENTS.md`) |
| **C3** | **Typed revocation** of proof evidence, measured | Reproducible harness | Implemented; E3 is the claim-bearing experiment |
| **E4** | Goal-state forecasting (R3) from fields the harness already records | Structural distance vs observed goals | Preregistered |

C1 is the substrate: `TacticTrace` records `goal_before` and `goals_after` for successes and
explicitly writes nothing for failures. That blind spot is exactly the programme's thesis.
E4 is cheap but not free: `TacticTrace` samples at most 20 records per tactic and prefers the
shortest goals, so forecast residuals must be computed on C1's unsampled stream.

---

## Evidence revocation

Content-addressed proof evidence with **typed** automatic revocation.

The third invariant of the Tekmerion programme says a proof is revocable evidence: it names
its dependencies, and any dependency change revokes it. Implemented naively, that invariant
is useless — it revokes a theorem when a file is reformatted, and it cannot tell "the theorem
is no longer true" from "the theorem is still true but is now about different bytes".

This crate implements the three distinctions that make revocation usable.

| Edge kind | Means | Revoked by |
|---|---|---|
| `Semantic` | the dependent uses the dependency's *meaning* | a change of semantic digest only |
| `Subject` | the dependent is *about* the dependency's *bytes* | any byte change |
| `Attestation` | the dependency only *accepted* the dependent | any byte change, to `AttestationStale` |

Revocation reports a severity per affected object:

- `SemanticsChanged` — the proof must be redone;
- `SubjectChanged` — the statement is unchanged, but the proof no longer covers the shipped
  artefact;
- `AttestationStale` — the proof stands; the record of who checked it is stale, so replay is
  enough. This severity does not propagate.

A `Revocation` is the **delta of one update**, not the object's current status: an event
reports only what it made worse. `Store::status` recomputes effective severity from the
current head map over the transitive pin closure, and admission reads that. Without the
distinction, an artefact change followed by a checker rebuild would report only the
rebuild's attestation delta while the subject mismatch remained unresolved.

Keeping two digests per object — one for meaning, one for bytes — is the whole trick. It is
also what closes the gap s2n-bignum names in its own `SOUNDNESS.md`: nothing checks that a
downstream reassembly produced the bytes the proofs were about.

### What is enforced

1. Identity is **derived** from content. A caller cannot assert an object's digest.
2. Construction is monotone: a node may only pin an already-declared unit at its current
   digest, so the graph is acyclic by construction.
3. Every unrepresentable or ambiguous input is **refused**, never coerced. Refusal sites are
   tagged `R01`–`R16`.

### What it does not establish

It does not decide whether any theorem is true. Revocation is a graph property of *recorded*
dependencies; validity is decided only by a proof kernel. If a dependency is omitted from the
graph, the graph cannot revoke on it, and hashing cannot fix that. Hashing establishes
identity, never validity.

---

## Measured — model conformance, not a frontier result

These numbers show that the implementation matches its stated model. They are not evidence of
any advance: the ground truth is definitional. The claim-bearing experiment is E3, which runs
real reassembled object bytes through the same path.

Measured on 2026-11-22, Rust 1.98.0, this machine:

| Measurement | Value | How |
|---|---|---|
| Tests | 46 passing | `cargo test` |
| Refusal sites | 16 | `R01`–`R16` |
| Valid mutants | 16 (no non-compiling mutants) | `cargo run --bin mutate` |
| Mutation score | **16/16 = 1.000** | every disabled refusal is caught |
| Lifecycle fault sites | 5 | `L01`–`L05`, feature `fault-injection` |
| Lifecycle mutation score | **5/5 = 1.000** | every lifecycle control is caught |
| Synthetic corpus | 310 objects, 40 changes | `tests/semantics.rs` |
| Naive revocation | 240 objects needing reproof | untyped closure over changed digests |
| Typed revocation | 150 affected: 60 semantic, 60 subject, 30 attestation | this engine |
| Reproof work removed | **75.0%** of the naive total | arithmetic over the above |

The synthetic corpus number is a **model conformance test**, not empirical evidence: the
ground truth is definitional, so the improvement follows from the model rather than being
discovered by it. It guards the implementation against regression. The empirical version is
experiment E3 in `research/02-EXPERIMENTS.md`, which puts real object bytes from
`s2n-bignum` through the same path.

### The mutation harnesses

`cargo run --bin mutate` (source: `src/tools/mutate.rs`) implements invariant 5 — every
instrument proves its sensitivity. For each
refusal site it replaces `refuse!` with the no-op `skip_refusal!` and runs the suite. A
mutant that fails to compile is excluded from the score rather than counted as killed; a
site whose removal no test notices is reported as `SURVIVED` and the script exits non-zero.

The technique is not ours. It is published as refusal-site mutation testing
(arXiv 2608.26183, August 2026), which also reports 112 refusal sites of which 75 could be
deleted with a green suite. Cite it; do not claim it.

`cargo run --bin mutate_lifecycle` (source: `src/tools/mutate_lifecycle.rs`) covers the
lifecycle rather than the wire format. The crate has
an off-by-default `fault-injection` feature and a `fault(code)` switch driven by
`TEKMERION_FAULT`. Five guarded sites (`L01`–`L05`) disable, respectively, pin handling in
effective status, the pins comparison in `classify`, closure propagation in `status`,
attestation edges in `status`, and retention of the superseded node in `update`. The
harness runs the suite once per code and exits non-zero on any survivor.

### Wire format

```text
TEKMERION-EVIDENCE v2
node<TAB>kind<TAB>unit<TAB>semantic<TAB>bytes<TAB>pins
head<TAB>unit<TAB>hex-digest
```

A document is an **immutable node table plus a head map**. Each `node` line declares one
node and its identity is derived from content, so the line carries no digest. A unit may be
declared by several `node` lines; its `head` line names the current one. `semantic` and
`bytes` are `-` or 64 hex characters. `pins` is empty or a comma-separated list of
`m:unit@hex` (semantic), `s:unit@hex` (subject), `a:unit@hex` (attestation).

Pins must reference a `(unit, digest)` node declared earlier, and heads come after every
node, so the table is a topological order. Superseded nodes are emitted too: that is what
lets a dependent keep its stale pin and makes `parse(render(store))` succeed after any
update. Rendering emits the node table, then the head map.

---

## What is in this repository

```
README.md              this file
Cargo.toml             crate `tekmerion`, published as 0.2.0
src/                   Rust tooling: emitters, guard, freeze, proof objects; `src/native/` is the
                       independent certificate checker
src/tools/mutate.rs    refusal-site mutation harness (`--bin mutate`)
src/tools/mutate_lifecycle.rs  lifecycle fault-injection harness (`--bin mutate_lifecycle`)
tests/                 46 tests, including one per refusal site and lifecycle status tests
examples/              reassembly.rs: the gap s2n-bignum names, step by step
trace/                 failure-side extension to TacticTrace
  exptrace_failure.ml  write-through failure capture, `exptrace_guard`
  fixtures/            wrapper golden file and conformance record pairs
src/bin/patch_wrapper.rs  rewrites a generated kernel_wrapper.ml, fail-closed
src/bin/trace_conformance.rs  stock vs patched success records, field by field
tests/write_through.rs proves a failure reaches disk before any dump
tools/selftest.sh      runs every check above in one command
mutation-report.json   last measured parser mutation score
lifecycle-report.json  last measured lifecycle mutation score
research/
  TEKMERION-FRONTIER-PROMPT.md   the brief this work audits
  00-VERDICT.md        the verdict, eleven corrections, the three-claim programme
  01-PRIOR-ART.md      mechanism-by-mechanism audit with novelty classifications
  02-EXPERIMENTS.md    E1–E5: hypotheses, arms, endpoints, kill criteria
  03-INSTRUMENT.md     failure-side trace schema, hook point, conformance test
  sources.md           pinned commits, commands, fetched documents, unverified items
```

## Reproduce

```sh
bash tools/selftest.sh                     # everything below, one command
cargo test --offline                       # 46 tests
cargo test --offline --features fault-injection  # same suite, lifecycle faults armed
cargo run --offline --bin mutate            # refusal-site mutation score
cargo run --offline --bin mutate_lifecycle  # lifecycle mutation score
cargo run --offline --example reassembly   # the reassembly gap, step by step
```

The failure-trace scaffold is verified offline: the OCaml module is syntax-checked and
type-checked against stubs, the wrapper patcher is checked against a golden file, the
conformance checker is checked against matching and deliberately diverging fixtures, and a
test proves a failure record reaches disk before any dump call. It has **not** been run
inside HOL Light; that is the first CI job when the benchmark environment exists.

## Branches and releasing

- `main` — every published state. Nothing is on `main` that was not published.
- `staging` — all work. Committed and pushed continuously so nothing is lost.

Releasing: merge `staging` into `main`, tag `vX.Y.Z`, `cargo publish`, push the tag.

```sh
git checkout main && git merge --ff-only staging
cargo publish
git tag -a vX.Y.Z -m "tekmerion X.Y.Z" && git push origin main vX.Y.Z
```

## Non-claims

Nothing here is a frontier claim. It is a prior-art audit, a corrected ledger, and a
measured conformance test of one component. The programme's first claim begins at C1, and
no result is `INDEPENDENTLY_REPRODUCED` until someone outside the programme reruns the
frozen package.

## Licence

Apache-2.0 OR MIT.
