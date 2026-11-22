# 05 — Frontier facts: verification record

**Date of all fetches and measurements:** 2026-11-22.
**Rule applied:** a claim is `VERIFIED` only if a primary source says it, in words I can quote.
Secondary coverage is labelled as such. Where I did not look, I say so; "not found" is recorded
as "not found, searched X", and the queries are logged in
`research/receipts/n-mechanism-search-log.md`.

Status vocabulary is the one fixed by the brief: `VERIFIED`, `PARTIAL`, `NOT_FOUND`,
`CONTRADICTED`.

## Summary

| # | Claim | Status |
|---|---|---|
| 1 | AlphaProof Code: ~6.3M Lean lines, compiled `libsignal`, machine code, explicit assumptions, mutation-based spec testing | **VERIFIED** (two caveats recorded) |
| 2 | Aria: 4,257 Iris + 318 `reglang` lemmas, whole-lemma agent, kernel-closing harness, no fixed strategy | **VERIFIED** |
| 3 | Anthropic + Theorem: 123 libraries / 239 entry points, RISC-V, Rocq, no-crash only, ~600k lines per ~18k-instruction binary | **PARTIAL** |
| 4 | s2n-bignum equivalence decomposition; P-256/384/521 Montgomery; `equiv.ml` 2,392 lines; 11 ARM proof files | **VERIFIED** |
| 5 | Prior art for N1, N3, N4, N5, N6, N11, N12 | **PARTIAL** (N4, N12 occupied; N3, N11 not found; N1, N5, N6 partially occupied) |
| 6 | SemantEq numbers; LemmaNet (ASE 2026); CoCo-Prover; "Predicting the Next State Is Not Enough" | **VERIFIED** (with scope caveats) |

---

## 1. AlphaProof Code — VERIFIED, with two caveats

**Source:** <https://google-deepmind.github.io/code-verification/> — **primary** (authors'
technical report, dated 2026-11-10). Not peer-reviewed.
Receipt: `research/receipts/alphaproof-code.md`.

> We introduce AlphaProof Code, a multi-agent AI framework for formal software specification and
> verification that we used to prove key security properties of `libsignal`, the Signal
> protocol's reference implementation, at the machine-code level.

> Applied to `libsignal`, our agents generated about 6.3 million lines of machine-checked Lean
> proof code, resting on about 27,000 lines of trusted specifications and AArch64 semantics.

Each element of the claim is present:

| Element | Holding |
|---|---|
| ~6.3M lines | "about 6.3 million lines of machine-checked Lean proof code" |
| `libsignal` executable | `libsignal-cli`, a harness linking unmodified `libsignal` v0.105.4, compiled with stock `rustc` 1.98.1 to AArch64 |
| machine-code level | semantics over AArch64 ELF bytes; "verifying the linked executable directly down to its system calls" |
| explicit assumptions | Table 1 "Assumptions"; §5.3 trusted computing base; "the Rust compiler is _not_ trusted" |
| mutation-based spec testing | "synthetic mutants apply mutation testing to specifications" |

**Caveat 1 — the line count includes comments and blanks.**

> The repository contains over 6.3 million lines of Lean, counting comments and blank lines.

**Caveat 2 — the mutation suite is weaker than the phrase "mutation-based specification testing"
suggests.** The published strength is 16 of 17 mutants, **at the specification level, one witness
trace each**:

> ...(so far for 16 of 17 mutants, each for one witness trace under a pinned concrete primitive
> interpretation)

and the full binary disproofs are not done:

> ...their 17 full binary disproof obligations (against the placed helper) remain open `sorry`
> stubs in the non-default library.

Do not cite this as "end-to-end mutation testing". Cite it as "16/17 specification-level mutant
refutations on one witness trace each; full binary refutations are `sorry`".

**What this means for the plan:** the headline (AI-generated machine-checked proofs about stock
`rustc` output, compiler outside the TCB) is **real and already published**. Any claim in M-F or
CERTCUT that "nobody has verified `rustc` output at machine-code level" is false as of
2026-11-10.

---

## 2. Aria (arXiv 2607.06341) — VERIFIED

**Source:** <https://arxiv.org/abs/2607.06341>, "Harnessing Code Agents for Automatic Software
Verification", Kan, Kan, Ertel; submitted 2026-07-07 — **primary** (author abstract).
Receipt: `research/receipts/aria-harness-2607.06341.md`.

> (1) Core logic: on Iris, the state-of-the-art separation logic for concurrent and
> memory-manipulating programs, Aria proves all 4,257 lemmas of the four core modules and the
> 217 lemmas verifying Rust's standard libraries built on it, fully automatically. (2)
> Comparison with prior LLM provers: on reglang, where prior provers manage barely one in
> eight, Aria proves all 318.

> Handing the whole lemma to a general LLM code agent (for example, Claude Code), free to choose
> its own approach, and wrapping it in a verification harness is both simpler and more effective,
> achieving full coverage: every targeted lemma proved, with no failures and no Coq expert
> intervention.

The harness design is explicitly three-part:

> ...keep each one sound (accepted only when the prover's kernel closes it), complete (no
> obligation left unproved or silently dropped), and terminating (no divergent tactics).

So all four elements of the claim hold: 4,257; 318; whole lemma handed over; kernel-closing
harness; no fixed strategy. Two numbers the brief omits and should carry: **217** Rust-stdlib
lemmas and **72** `iris-lean` lemmas.

**Limit:** abstract-level only. The paper reports coverage, not sample efficiency; a "free to
choose" agent that burns unbounded attempts is consistent with these numbers.

---

## 3. Anthropic + Theorem (2026-10-22) — PARTIAL

**Source:** <https://www.anthropic.com/research/formal-verification> — **primary** (first-party
institutional report). Not peer-reviewed.
Receipt: `research/receipts/anthropic-theorem-verification.md`.

Verified components:

> Claude successfully patched the bugs it found and then proved that there are no crashes on any
> input (up to 64 MB) for 239 entry points across 123 different OSS-Fuzz libraries.

> These proofs operate on the compiled RISC-V instructions (i.e., executable binaries), and they
> comprise about 180 million lines of Rocq, with a typical proof running to about 600,000 lines
> to provide security guarantees on binaries with a median length of 18,000 RISC-V instructions.

> This means these proofs currently assume that the program runs as a single-threaded static
> binary with a fixed memory budget on a RISC-V CPU using Linux OS.

The property is the fuzz test never crashing, ASan included:

> The spec says that the fuzz test won't crash for _any_ input less than 64 MB.

**Why the claim is only PARTIAL.** The bullet "no functional correctness; no constant time" is
a characterisation of the *programme*, and it is too strong in one direction and unsupported in
the other:

1. **"No functional correctness" is contradicted for the programme as a whole.** The same post
   reports a separate experiment proving that the Linux kernel's zlib implements RFC 1950,
   at the x86 binary level. That is functional correctness. The post also says fuzz specs often
   encode round-tripping, idempotence, algebraic laws and equivalence to a reference:
   "proving the test never crashes is equivalent to proving the property itself."
   For the **239-entry-point campaign** the characterisation is fair; for the programme it is not.
2. **"No constant time" is not stated anywhere in the source.** The post neither claims nor
   excludes constant-time properties. Writing "no constant time" as *their* claim attributes to
   them something they did not say. Record it as "not addressed in this report", or drop it.

Correct formulation: *the 239-entry-point campaign proves ASan-inclusive crash-freedom on
RISC-V binaries under a single-threaded, static, fixed-memory, Linux model; the report explicitly
says this is not bug-freedom, because ASan is incomplete and not every entry point was verified.*

---

## 4. s2n-bignum equivalence — VERIFIED (measured)

**Source:** the repository itself at commit
`1d097f630f0ca91cc2271491b9dd57dc9c9023a5` (2026-11-21), plus
`doc/program_equivalence.md` — **primary artifact and first-party doc**.
Receipt: `research/receipts/s2n-bignum-program-equivalence.md`.

Provenance: I reused the existing clone at `/tmp/s2n` (present before this task) rather than
re-cloning. I verified `HEAD` equals the pin already in `research/sources.md` and that
`git status --porcelain` is empty. Measurements:

| Measurement | Result |
|---|---|
| `git rev-parse HEAD` | `1d097f630f0ca91cc2271491b9dd57dc9c9023a5` |
| working tree | clean (0 modified paths) |
| `wc -l arm/proofs/equiv.ml` | **2,392** |
| non-blank, non-comment lines in `equiv.ml` | 1,880 |
| proofs files loading `arm/proofs/equiv.ml` | **11** (all under `arm/proofs`; none under `x86/proofs`) |
| `doc/program_equivalence.md` | present, 42,950 bytes |

The 11 files include `bignum_montmul_p256/p384/p521.ml` and `bignum_montsqr_p256/p384/p521.ml`,
plus `bignum_mul_p521.ml`, `bignum_sqr_p521.ml`, `bignum_mul_8_16.ml`, `bignum_sqr_8_16.ml`,
`bignum_emontredc_8n_cdiff.ml`.

The doc states the decomposition exactly:

> Each of these composes (1) NEON vectorization followed by (2) SLOTHY instruction reordering.
> Each step proves a separate `ensures2` equivalence theorem; the two are then composed into a
> single scalar↔optimized equivalence; and the composed equivalence is combined with the
> pre-existing `ensures`-style functional correctness of the scalar reference to yield
> functional correctness of the optimized program.

> **P384/P521.** The structure is identical. Same three-versions layout
> (`…_unopt_core_mc`, `…_interm1_core_mc`, `…_core_mc`), same `eqin`/`eqout`

**One wording correction.** "`equiv.ml` is ~2,392 lines **across 11 ARM proof files**" conflates
two measurements. `equiv.ml` is 2,392 lines *itself*; it is *loaded by* 11 files. Write it as
"`equiv.ml` = 2,392 lines (1,880 code); loaded by 11 ARM proof files".

**Consequence the plan must absorb:** the "de-optimisation / proof by twin" idea (M-A) is the
*existing* s2n-bignum methodology, with the twin's functional correctness and the twin ≡ shipped
equivalence both already proved, for exactly the P-256/P-384/P-521 Montgomery routines.

---

## 5. Prior art for the N-mechanisms — PARTIAL

Search basis and its limits are in `research/receipts/n-mechanism-search-log.md`. In short:
arXiv title/abstract index only; 15 queries; two intended queries returned HTTP 400 and are
marked as not completed. Absence at title/abstract level is **not** evidence of novelty.

| N | Nearest work found | What remains | Status |
|---|---|---|---|
| **N1** partial proof term with typed holes as state | Typed holes as a first-class synthesis context: Hazelnut Live (arXiv 1805.00155); LLMs prompted on a hole's typing context (arXiv 2409.00921, OOPSLA 2024). Goal-count heuristics in Aesop/`auto`; proof-graph goal selection in CoCo-Prover (2610.09681) | The residual triple — hole count, hole **type complexity**, **unification-failure pair** — used as the search signal, compared against goal-size pruning at equal kernel-call budget. **Feasibility blocker:** HOL Light exposes no dependent typed holes; it is LCF goalstack + tactic scripts. N1 presupposes an elaborator (Lean/Coq/Agda), so the substrate must change or an elaborator must be built | **PARTIAL** |
| **N3** conformal / distribution-free abstention in proof search | Conformal prediction (Angelopoulos & Bates, 2107.07511); learnable conformal abstention for LLMs/VLMs (2502.06884) | No work found applying conformal prediction, coverage guarantees or selective abstention to *proof search* (3 queries, no hit). Also unaddressed: exchangeability between calibration and temporal-hold-out tasks | **NOT_FOUND** (searched) |
| **N4** dense kernel-grounded reward | Verifier-in-the-loop per-step feedback (2503.09730); step-aware Lean verification as reward (2506.04592); hybrid predicted+real reward on GPU (2610.21881); RLPAF and intrinsic reward in MCTS (2408.08152); outcome reward from the Lean compiler (2504.06122) | The exact residual `r_t = -ΔGoalComplexity - λ·KernelCalls + μ·CertificateProgress`, on **object-code** obligations, at an equal kernel-call budget. The mechanism family is occupied | **CONTRADICTED** as a novel mechanism; formula unverified |
| **N5** proof repair by transport along a semantics-preserving diff | PUMPKIN Pi (2010.00774): proof-term transport across type equivalences, zero search, no extra axioms. s2n-bignum's own `ensures2` composition | Transport along a **code** diff whose equivalence is a machine-checked object-code theorem, with the theorem statement **guarded** against weakening. PUMPKIN Pi changes types, so it changes statements by design | **PARTIAL** |
| **N6** check a translation certificate, not just the exported theorem | Lean `comparator` + AlphaProof Code gatekeeper: statement **and every definition it depends on**, held-out copy, standard axioms only. hol2dk → Dedukti and OpenTheory: translation with target-side checking. ProofCert framework (Chihani/Miller/Renaud) — **secondary, not fetched** | A certificate *for the translation itself*, with planted bug classes (binder capture, definition shadowing, implicit-argument mismatch) as controls. Statement-identity checking is not translation-certificate checking | **PARTIAL** |
| **N11** GPU-parallel proof-certificate checking | CPU-parallel proof checking: Isabelle/PIDE multi-core checking (1207.3441). Verified sequential checkers: `cake_lpr`, `Par` — **secondary, not fetched**. GPU *prediction* of Lean verdicts (2610.21881) is a surrogate, not a checker | No work found on GPU certificate checking (2 queries; 6 hits, none relevant). Structural obstacle: LCF derivations have sequential dependencies, so the parallel unit is *independent obligations*, not one derivation | **NOT_FOUND** (searched); feasibility bounded |
| **N12** semantics-preserving transforms as memorisation probe | CodeMorph (2506.17627, 26 transforms, −24.67% accuracy); Dynamic Benchmarking (2503.06643, "dynamic benchmarks can resist against the data contamination problem"); RustMizan (2607.04729, "semantics-preserving code mutants for contamination testing"); Haroon et al. (2504.04372, SPMs, 78% failure) | SemantEq-admissible transforms on **Lean statements**, applied to a **proof-search** benchmark, with kernel-accepted completion as endpoint. The mechanism is occupied four times over | **CONTRADICTED** as a novel mechanism; instantiation unverified |

---

## 6. SemantEq, LemmaNet, CoCo-Prover, JEPA — VERIFIED

### 6.1 SemantEq — arXiv 2610.32919 (2026-10-23)

<https://arxiv.org/abs/2610.32919> — **primary** (author abstract). Receipt:
`research/receipts/semanteq-2610.32919.md`.

> SemantEq-Eval, a search without a model, reaches 100% precision and 65.6% recall on the
> ProofNetVerif test split under our adjudicated labels, against 95.8% and 61.8% for BEq+, and
> an 8.9-fold increase in its CPU time adds at most three pairs.

The numbers match the brief exactly: **100% / 65.6% vs BEq+ 95.8% / 61.8%**. Two scope notes
belong in every citation: (a) "under our adjudicated labels" — the authors' own adjudication;
(b) `SemantEq-Eval` is "a search without a model", so it is the relation's ceiling, not a
model's score. The paper also reports that under two of four prompts a model proves both
directions of about two in five non-equivalent pairs — i.e. a stronger prover increases the
*false-acceptance* risk, which is why `SemantEq-ProofAudit` exists.

### 6.2 LemmaNet — arXiv 2603.22114 (ASE 2026)

<https://arxiv.org/abs/2603.22114> — **primary** (author abstract + metadata). Receipt:
`research/receipts/lemma-net-ase26.md`.

> We implement LemmaNet on top of an existing proof agent AutoRocq for Rocq and the Frama-C
> ecosystem, and evaluate it on SV-COMP and established real-world subjects, including modules of
> the Linux kernel, Contiki OS, standard C++ library, and X.509 parser.

Page metadata: "13 pages. **To appear in ASE 26**", DOI `10.1145/3832783.3834356`. So "LemmaNet
(ASE 2026)" is correct. No numeric results are in the abstract ("significantly outperforms"), so
do not quote a number for it without reading the body.

### 6.3 CoCo-Prover — arXiv 2610.09681 (2026-10-07)

<https://arxiv.org/abs/2610.09681> — **primary**. Receipt:
`research/receipts/coco-prover-2610.09681.md`.

> On five program verification benchmarks in Lean 4 including function-level CLEVER, VERINA, and
> AlgoVeri, and repository-level NTP4VC and Vero, we show that CoCo-Prover achieves a better
> success-vs-cost frontier than baselines ... it achieves the best solve rate on every benchmark
> and up to 100% on two benchmarks. It also reduces cost by up to 30.9% compared to the strongest
> baseline with the strongest LLM in our evaluation.

Relevant to the plan because it already does two-level proof-graph goal selection plus
per-action cost pricing, in Lean 4 — i.e. the "which goal, which action, at what cost" problem.

### 6.4 "Predicting the Next State Is Not Enough" — arXiv 2609.32908 (2026-09-26)

<https://arxiv.org/abs/2609.32908> — **primary**. Receipt:
`research/receipts/jepa-next-state-2609.32908.md`. Accepted to MathNLP @ EMNLP 2026.

> JEPA achieves higher Top-1 than matched InfoNCE on a same-theorem ranking diagnostic (50.18%
> versus 31.55%), but averages 282.3 of 987 solved theorems across three seeds versus 308 for
> proposer ordering, while requiring more tactic checks.

> In this setting, accurate one-step transition ranking is therefore insufficient as a
> long-horizon search value.

This is a **negative control the plan must cite**. It is the cleanest published demonstration
that improving a next-state/forecast diagnostic can *lose* on kernel-checked completion. Any
residual-reward claim (N4, M-E, the older R3 forecast residual) must be pre-registered against
this result.

---

## 7. What changed in the plan as a result

1. **M-A is the existing s2n-bignum method**, not a new proof strategy (measured above).
2. **M-F's headline is pre-empted** by AlphaProof Code (stock `rustc`, AArch64, machine code,
   compiler outside the TCB). What remains is a narrower, still-useful claim: HOL Light, the
   *existing* s2n-bignum specification, a pinned digest, a contamination-free held-out set.
3. **N12 and N4 are occupied** at mechanism level; N3 and N11 have no prior art I could find,
   but N11 has a structural ceiling (derivation-level sequentiality).
4. **N1 may be unimplementable on HOL Light** without building an elaborator, because HOL Light
   has no dependent typed holes.
5. Any "no functional correctness / no constant time" language about the Anthropic + Theorem
   work must be corrected: functional correctness *was* reported in a separate experiment, and
   constant time is simply not discussed.
