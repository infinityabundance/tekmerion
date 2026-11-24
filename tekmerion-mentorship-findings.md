# Tekmerion — mentorship findings (F1–F9) and the P0–P7 mandate

**Date:** 2026-11-24
**Basis:** repo state at `6536e5f` (sealed x86 9/9 in a proof-free container; ARM 18/18
development; first upstream drop-in passing `check_axioms`; challenge comparator; refusal-site
mutation harness), the Ordeal, and the literature cited at the end.

This file is the source for `TODO.md` Part XII. Findings are ranked by how much each moves
Tekmerion toward a result the field cannot argue with.

## Governing frame

Build a **proof compiler**, not a proof search engine: compile executable behaviour into
mathematical evidence. Executable artifact → verified decoding / symbolic execution / memory
and flag semantics → mathematical-structure discovery → certificate synthesis → native
checker → theorem.

Do not let SAT become the ceiling of the mathematics. Choose the representation that makes
the obligation easiest (carry → integer conservation; reduction → quotient/residue; multiply
→ polynomial identities; Montgomery → algebraic cancellation; ranges → interval analysis;
bitwise → certified SAT; loops → inductive invariants; schedules → relational equivalence).

Stop measuring progress by code volume. The metric is newly enabled capability per unit of
new engine complexity.

## F1 — The proof script is the attack surface (highest value)

HOL Light's guarantee is enforced by OCaml's type system; a generated script is arbitrary
OCaml. A producer can shadow `prove`, `axioms`, `check_axioms` or a tactic name, use
`Obj.magic`/`Marshal`, or mutate global state. The bench's axiom-count check catches none of
these. Unsafe OCaml is a long-recognised LCF risk.

1. **Closed tactic grammar.** Typed, closed language (allow-listed combinators, term
   literals, named lemmas) with a printer to OCaml. Acceptance parses the OCaml with the
   front end (compiler-libs) and refuses any AST that is not exactly the printer's output:
   no rebinding of kernel/judge names, no `Obj`, `Marshal`, `Sys`, `Unix`, no references, no
   `external`.
2. **Firing controls:** `Obj.magic` forgery; shadowed `check_axioms`; rebinding `prove`;
   mutable-state trick. Each refused before HOL Light runs.
3. **Candle milestone:** one sealed s2n-bignum theorem re-checked in Candle (verified HOL
   clone, same OCaml variant), removing OCaml from the trust base for that result. First
   obstacle: loading the s2n-bignum libraries.

## F2 — Export the derivation, not just the verdict

Turn on proof recording (ProofTrace) for every accepted theorem. Gives: true per-theorem
axiom dependencies (foundation stamp, measured); re-checking outside the OCaml process
(HOLTrace); export to a second logic (Dedukti/hol2dk); inference-level size/shape baseline.

## F3 — Check time is a first-class metric

Linear-family proofs are ~10× slower than the human ones (P-256 add ~24 s vs 1.555 s;
`add_p521` 96.9/120 s; suite ~2×10⁵ CPU-s/arch; `bstep` ARM 241,319 → 190,772 s).

- SAT for **discovery**, induction for **reuse**: find and validate carry/selection facts,
  then generalise into width- and modulus-parametric lemmas proved once by induction over
  limbs and instantiated cheaply. Lowers the foundation too (arithmetic induction, not a
  256-bit circuit).
- Share the word-level `ref` lemmas across ARM and x86.
- Report check time next to every `OK`; target parity with the human proof.

## F4 — Regeneration is the property nobody else has

Human machine-code proofs are repaired by hand. Tekmerion's are compiler outputs. Run the
**frozen** compiler nightly against upstream HEAD and publish the survival curve (X%
regenerated unchanged and green, no human edits), attributing each break to proof
infrastructure, object code, or ISA model. This is "proof repair after change" measured on
industrial machine code, and the strongest argument for AI-driven verification.

## F5 — Let the AI extend the compiler, under sealed evaluation

Give an agent an unseen family (double, half, optneg, triple, then `montmul`/`montsqr`) with
only object code, spec and interfaces; it extends the generators; freeze; evaluate sealed on
curves/ISAs it never saw. Metric: **time-to-coverage for an unseen family**, kernel as sole
judge.

## F6 — Statement fidelity, done by machines

- Adversarial statement agents from different model families generate weakened mutants,
  vacuity probes, and intent-violating implementations that satisfy a weakened statement.
- Seed unknown defects; report recall.
- Extract statements twice (bench `problems.json` and pinned upstream `.ml` goal quotation)
  and require term-for-term equality.
- Non-vacuity by construction: concrete satisfying state run on the machine model; concrete
  wrong output rejected for each postcondition.

## F7 — The upstream tree is the scoreboard

Publish **upstream coverage**: the fraction of s2n-bignum's **675 straight-line FC theorems**
(at the pin) regenerated green in s2n-bignum's own build, by family and ISA, with check time.
Rerunnable mechanically; the same harness measures F3 and F4 for free.

## F8 — Coverage order

1. Linear families (double, half, optneg, triple, cmul).
2. Montgomery families via upstream's own `ensures2` equivalence route with an agent-written
   scalar twin (de-optimisation), composed with the reference's correctness.
3. Loops (~25% of FC): postcondition-prefix invariants fitted on the executable model.

Montgomery reduction is the next research target: discover `q` with `T + q·p ≡ 0 (mod R)`,
`u = (T+q·p)/R`, prove `u ≡ T·R⁻¹ (mod p)`, plus a range proof. The producer finds the
witness; the checker validates algebra, divisibility, instruction correspondence and ranges.

## F9 — Determinism and timing hygiene

Fix core count, memory and HOL build for every judged run and record them. Any task above
**70% of its timeout is a defect to fix before a sealed run** (so `add_p521` at 0.81 already
fails). Persist every judge output; no tmpfs-only verdicts.

## P0–P7 mandate

| Priority | Deliverable | Done when |
|---|---|---|
| P0 | Fix outstanding assurance defects | freeze identity, mandatory verdict evidence, mutation CI correct |
| P1 | Close G1 | machine-checked bit-blaster soundness, no opaque hypothesis |
| P2 | Native multi-limb arithmetic | checked identities beyond the 64-bit term fragment |
| P3 | Multiplication proof generation | one generic mechanism across unfamiliar routines |
| P4 | Native machine-code theorem | Curve25519 addition verified natively |
| P5 | Montgomery reduction | generic construction from executable semantics |
| P6 | Autonomous lemma discovery | demonstrated improvement on reserved obligations |
| P7 | Verified optimized checking | measured acceleration with justified acceptance semantics |

## Frontier claims these findings set up

| Claim | Evidence | Adjudicator |
|---|---|---|
| Proofs that cannot forge through the script language | F1 gate + firing controls; one proof in Candle | Kernel; Candle |
| A measured foundation per theorem | F2 per-theorem axiom dependencies | Exported proof object |
| Check-time parity with human proofs | F3 measurements | The clock |
| Self-maintaining machine-code proofs | F4 nightly survival curve | Upstream HEAD |
| AI-extended coverage on unseen families | F5 sealed time-to-coverage | Kernel; manifests |
| Upstream coverage of s2n-bignum FC | F7 green upstream builds | s2n-bignum's own build |

## Sources

- Candle (ITP 2022; JAR 2025): verified HOL Light clone; same OCaml variant
- LCF soundness risks including `Obj.magic` and mutable strings (HOL Zero discussions,
  isabelle-users)
- HOList (arXiv 1904.03241); HOLTrace (holtrace.cr.yp.to)
- s2n-bignum draft PR #505 (`bstep`) CPU-time figures
- CoqCryptoLine; Lean `bv_decide`; CakeLPR; AlphaProof Code; SLOTHY; OpenTheory
- Relational Hoare Logic for Realistically Modelled Machine Code (2025)
- Hipster / QuickSpec theory exploration; CEGAR (Clarke et al.)
