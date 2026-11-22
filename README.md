# Tekmerion

A proof-verification programme aimed at beating the published frontier on measured,
externally adjudicated criteria — without overstating what has been established.

**Start here: [`research/00-VERDICT.md`](research/00-VERDICT.md).**

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
| **C3** | **Typed revocation** of proof evidence, measured | Reproducible harness | Implemented + measured here |
| **E4** | Goal-state forecasting (R3) from fields the harness already records | Structural distance vs observed goals | Preregistered |

C1 is the substrate: `TacticTrace` records `goal_before` and `goals_after` for successes and
explicitly writes nothing for failures. That blind spot is exactly the programme's thesis.
E4 is the cheapest scientific result available, because R3 needs no new instrumentation at
all.

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

## Measured

Measured on 2026-11-22, Rust 1.98.0, this machine:

| Measurement | Value | How |
|---|---|---|
| Tests | 30 passing | `cargo test` |
| Refusal sites | 16 | `R01`–`R16` |
| Valid mutants | 16 (no non-compiling mutants) | `tools/mutate.py` |
| Mutation score | **16/16 = 1.000** | every disabled refusal is caught |
| Synthetic corpus | 310 objects, 40 changes | `tests/semantics.rs` |
| Naive revocation | 240 objects needing reproof | untyped closure over changed digests |
| Typed revocation | 150 affected: 60 semantic, 60 subject, 30 attestation | this engine |
| Reproof work removed | **75.0%** of the naive total | arithmetic over the above |

The synthetic corpus number is a **model conformance test**, not empirical evidence: the
ground truth is definitional, so the improvement follows from the model rather than being
discovered by it. It guards the implementation against regression. The empirical version is
experiment E3 in `research/02-EXPERIMENTS.md`, which puts real object bytes from
`s2n-bignum` through the same path.

### The mutation harness

`tools/mutate.py` implements invariant 5 — every instrument proves its sensitivity. For each
refusal site it replaces `refuse!` with the no-op `skip_refusal!` and runs the suite. A
mutant that fails to compile is excluded from the score rather than counted as killed; a
site whose removal no test notices is reported as `SURVIVED` and the script exits non-zero.

The technique is not ours. It is published as refusal-site mutation testing
(arXiv 2608.26183, August 2026), which also reports 112 refusal sites of which 75 could be
deleted with a green suite. Cite it; do not claim it.

### Wire format

```text
TEKMERION-EVIDENCE v1
kind<TAB>unit<TAB>semantic<TAB>bytes<TAB>pins
```

`semantic` and `bytes` are `-` or 64 hex characters. `pins` is empty or a comma-separated
list of `m:unit@hex` (semantic), `s:unit@hex` (subject), `a:unit@hex` (attestation).
Documents are order-sensitive: a pin may only reference a unit declared above it. Rendering
therefore emits a topological order.

---

## What is in this repository

```
README.md              this file
Cargo.toml             crate `tekmerion`
src/                   typed revocation engine
tests/                 30 tests, including one per refusal site
examples/              reassembly.rs: the gap s2n-bignum names, step by step
tools/mutate.py        refusal-site mutation harness
mutation-report.json   last measured mutation score
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
cargo test --offline                       # 30 tests
python3 tools/mutate.py                    # refusal-site mutation score
cargo run --offline --example reassembly   # the reassembly gap, step by step
```

## Non-claims

Nothing here is a frontier claim. It is a prior-art audit, a corrected ledger, and a
measured conformance test of one component. The programme's first claim begins at C1, and
no result is `INDEPENDENTLY_REPRODUCED` until someone outside the programme reruns the
frozen package.

## Licence

Apache-2.0 OR MIT.
