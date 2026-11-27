# XXIV — Structured literature extraction (certificate-based arithmetic closes)

**Date:** 2026-11-27. **Branch:** `staging`. **Type:** primary-source extraction.
Companion search log: `research/receipts/xxiv-search-log.md`.

## Method and its limits

- Every block below is built **only** from a page I fetched in this session. The URL I fetched is
  named in each block; where I also used a proxy to reach a PDF, the proxy URL is in the log.
- Quotes are verbatim and ≤ 40 words. Where a fetch returned binary PDF bytes with no extractable
  text, I say so; I did not reconstruct text from memory.
- Queries are logged with interface and hit count. Two search engines were unavailable: DuckDuckGo
  began returning a bot challenge after two successful queries, and arXiv's site search is a strict
  AND over title/abstract/keywords, so multi-term queries returned 0 with the reason shown.
- "Certified?" is answered for the artefact the paper produces, not for the paper's argument.

---

## 1. Fiat Cryptography

- **Citation:** Erbsen, Philipoom, Gross, Sloan, Chlipala. *Simple High-Level Code For Cryptographic
  Arithmetic — With Proofs, Without Compromises.* IEEE S&P 2019. Fetched
  `https://adam.chlipala.net/papers/FiatCryptoSP19/` (abstract page; PDF link not downloaded).
- **Mechanism:** Write field arithmetic as short, generic high-level code in Coq and rely on
  *partial evaluation* of that code to produce the fast C-like output, so one proof covers all
  instantiations.
- **What it operates on:** Source-level functional programs (Coq) lowered to a C-like IR. Not
  assembly, not bit-vectors, not integer constraints.
- **Exact passage:** "We further demonstrate that simple partial evaluation is sufficient to
  transform into the fastest-known C code, breaking the decades-old pattern" —
  `https://adam.chlipala.net/papers/FiatCryptoSP19/`
- **What it explicitly does NOT do:** Nothing on machine code; no carry-chain reasoning; no
  certificate emitted for an external checker. **No stated limitation** in the fetched abstract;
  the scope is visible only from the artefacts named. Assessment: no stated limitation.
- **Nearest Tekmerion mechanism:** symbolic execution (partial evaluation is a form of it).
- **Distance:** Would need (a) a bit-accurate ISA semantics, (b) per-limb carry obligations, and
  (c) an emitted certificate, none of which the pipeline has.
- **Certified?** Yes, but by Coq's kernel on the whole translation; no separately checkable
  certificate object.

## 2. CoqCryptoLine

- **Citation:** Tsai, Fu, Liu, Shi, Wang, Yang. *CoqCryptoLine: A Verified Model Checker with
  Certified Results.* CAV 2023, LNCS 13965, pp. 227–240. Fetched
  `https://link.springer.com/chapter/10.1007/978-3-031-37703-7_11` (open access, full text).
- **Mechanism:** Two verified reductions — algebraic (polynomial equalities → root entailment) and
  bit-vector (ranges → SMT QF_BV) — with untrusted external solvers returning *certificates* that a
  Coq-verified checker validates.
- **What it operates on:** CryptoLine models of **assembly programs** (mov/add/adc/sub/sbb/mul/mull,
  with explicit carry and borrow flags), plus modular polynomial equations and QF_BV predicates.
- **Exact passage:** "The witness found is further validated by CoqCryptoLine, which relies on the
  polynomial normalization procedure norm_subst in Coq to check if Eq. 1 holds." —
  `https://link.springer.com/chapter/10.1007/978-3-031-37703-7_11`
- **What it explicitly does NOT do:** The search is not a machine-code proof in an LCF kernel; it is
  a Coq development with extracted OCaml. Scope limit it does state: "the algebraic reduction in
  SSA2ZSSA is sound but not complete due to the abstraction of bit-accurate semantics into
  (modular) polynomial equations over integers".
- **Nearest Tekmerion mechanism:** carry-chain aggregate + certificate language.
- **Distance:** Swap Coq+MathComp for HOL Light, and replace the CAS-called Singular with a
  multiplier-emitting procedure whose output is replayed as kernel inferences. CoqCryptoLine also
  needs carry annotations: the p256 example needs "30 lines of annotations" of which the carry/flag
  zero asserts are the load-bearing ones.
- **Certified?** Yes: ideal-membership cofactors checked by `norm_subst`; QF_BV by CoqQFBV; SAT by
  Grat. Trusted base is stated explicitly as parser, text interface, Isabelle (inside Grat), Coq.

## 3. Myreen, Decompilation into Logic

- **Citation:** Myreen (with Gordon, Slind). *Decompilation into Logic — Improved.* Fetched
  `https://www.cl.cam.ac.uk/~mom22/decompilation/` (project page + abstract).
- **Mechanism:** A proof-producing decompiler that turns concrete machine code into functional
  programs in HOL4 logic, proving a certificate theorem that the extracted functions have exactly
  the machine code's behaviour.
- **What it operates on:** Machine code against an ISA model; output is HOL functions.
- **Exact passage:** "proof-certified translation of concrete machine code into functional programs
  defined in the logic of a theorem prover." — `https://www.cl.cam.ac.uk/~mom22/decompilation/`
- **What it explicitly does NOT do:** No arithmetic decision procedure, no aggregation of limb
  equations, no certificate format. **No stated limitation** in the fetched abstract.
- **Nearest Tekmerion mechanism:** symbolic execution.
- **Distance:** Decompilation gets you the functional program; it says nothing about how the
  arithmetic postcondition is discharged, which is exactly Tekmerion's cost centre.
- **Certified?** Yes, in-kernel (HOL4 LCF derivations).

## 4. Myreen & Curello, Proof Pearl: verified bignum in x86-64

- **Citation:** Myreen, Curello. *Proof Pearl: A Verified Bignum Implementation in x86-64 Machine
  Code.* CPP 2013, LNCS 8307, pp. 66–81. Fetched `https://www.cl.cam.ac.uk/~mom22/cpp13/`.
- **Mechanism:** Case study: build arbitrary-precision integer arithmetic in x86-64 and verify it
  using the decompiler plus a proof-producing compiler, keeping reasoning at a high level.
- **What it operates on:** 800 lines of verified 64-bit x86 machine code for bignum operations.
- **Exact passage:** "our proofs are shorter and, more importantly, the reasoning is at a more
  convenient high level of abstraction" — `https://www.cl.cam.ac.uk/~mom22/cpp13/`
- **What it explicitly does NOT do:** No inference-count target, no certificate, no sparse
  elimination. **No stated limitation.** It reports proof length in lines, not primitive inferences.
- **Nearest Tekmerion mechanism:** symbolic execution + support library.
- **Distance:** This is the historical shape of the problem Tekmerion automates; it does not attempt
  to beat a hand proof on inference count, and it is HOL4, not HOL Light.
- **Certified?** Yes, in-kernel (HOL4).

## 5. Proof Synthesis and Reflection for Linear Arithmetic (Chaieb & Nipkow)

- **Citation:** Chaieb, Nipkow. *Proof synthesis and reflection for linear arithmetic.* J. Automated
  Reasoning 41(1):33–59, 2008. Fetched
  `https://portal.fis.tum.de/en/publications/proof-synthesis-and-reflection-for-linear-arithmetic/`.
- **Mechanism:** Cooper (for Z) and Ferrante–Rackoff (for R) quantifier elimination implemented twice
  — once as a proof-producing tactic, once by reflection (computation inside the logic).
- **What it operates on:** Quantifier-free/quantified linear arithmetic formulas over Z and R.
- **Exact passage:** "An implementation in Isabelle/HOL shows that the reflective approach is between
  one and two orders of magnitude faster." —
  `https://portal.fis.tum.de/en/publications/proof-synthesis-and-reflection-for-linear-arithmetic/`
- **What it explicitly does NOT do:** Nothing about machine code, carry chains or sparse
  elimination. **No stated limitation** in the fetched abstract.
- **Nearest Tekmerion mechanism:** `INT_ARITH_TAC` close.
- **Distance:** This is the direct ancestor of the close Tekmerion uses: it tells you reflection is
  the cheaper of the two, but its input is a formula over integers, not per-limb machine-state
  equations, and its output is a decision, not a small certificate.
- **Certified?** Yes, in-kernel (Isabelle LCF-style kernel); the reflective variant is a computation
  inside the logic.

## 6. Checking Linear Integer Arithmetic Proofs in Lambdapi

- **Citation:** Coltellacci, Merz. *Checking Linear Integer Arithmetic Proofs in Lambdapi.*
  FroCoS 2025, LNCS/LNAI 15979, pp. 367–385. Fetched
  `https://link.springer.com/chapter/10.1007/978-3-032-04167-8_20` (open access, full text).
- **Mechanism:** Reconstruct Alethe `la_generic` / `lia_generic` steps in Lambdapi by proof by
  reflection over a canonical-form linear-polynomial algebra, using AC rewriting.
- **What it operates on:** Alethe SMT proof traces (linear integer/real arithmetic steps) → Lambdapi
  terms.
- **Exact passage:** "Proof by reflection is a technique for writing certified procedures for
  automated reasoning. It reduces the validity of a logical statement to a symbolic computation." —
  `https://link.springer.com/chapter/10.1007/978-3-032-04167-8_20`
- **What it explicitly does NOT do:** It states its scalability limit: "the current implementation
  does not yet scale well in arithmetic-heavy cases", naming `rings` where `la_generic` steps carry
  "hundreds of uninterpreted variables". No LCF kernel anywhere in the pipeline.
- **Nearest Tekmerion mechanism:** `INT_ARITH_TAC` close + certificate language.
- **Distance:** Input is SMT clauses, not limb equations; the checker is Lambdapi's own λΠ-modulo
  type checker, not a HOL Light kernel; no integer-multiplier witness is produced for sparse
  elimination.
- **Certified?** Yes, by Lambdapi's kernel via reflection. Note for Q5: Lambdapi *can* export to
  Rocq/Lean/HOL-Light, but the arithmetic check happens before export, in Lambdapi.

## 7. VeriPB / CakePB (CP 2025)

- **Citation:** Koops, Le Berre, Myreen, Nordström, Oertel, Tan, Vinyals. *Practically Feasible
  Proof Logging for Pseudo-Boolean Optimization.* CP 2025, LIPIcs 340, 21:1–21:27. Fetched
  `https://drops.dagstuhl.de/entities/document/10.4230/LIPIcs.CP.2025.21`.
- **Mechanism:** Efficient pseudo-Boolean proof logging for RoundingSat and Sat4j, covering
  core-guided search and LP integration with Farkas certificates and cut generation, checked by
  VeriPB (unverified reference) and CakePB (CakeML-verified).
- **What it operates on:** 0-1 integer linear constraints (pseudo-Boolean), i.e. ILP-style
  inequalities over Boolean variables.
- **Exact passage:** "we present an efficient toolchain based on VeriPB and CakePB for formally
  verified pseudo-Boolean optimization" —
  `https://drops.dagstuhl.de/entities/document/10.4230/LIPIcs.CP.2025.21`
- **What it explicitly does NOT do:** No machine code, no HOL/LCF kernel; CakePB is a standalone
  verified executable. No stated limitation beyond the scope of PB optimization.
- **Nearest Tekmerion mechanism:** certificate language (cutting-planes/Farkas lines).
- **Distance:** Tekmerion's premises are equations between machine-word expressions, not 0-1
  inequalities; a translation from limb equations to PB constraints would lose the integer-scalar
  structure that makes the close cheap.
- **Certified?** Yes — but by CakePB, a CakeML-extracted binary. That is a verified checker, **not**
  an LCF kernel: CakePB emits an answer, not a kernel derivation.

## 8. CryptOpt

- **Citation:** Kuepper, Erbsen, Gross, Conoly, Sun, Tian, Wu, Chlipala, Chuengsatiansup, Genkin,
  Wagner, Yarom. *CryptOpt: Verified Compilation with Randomized Program Search for Cryptographic
  Primitives.* arXiv:2211.10665 (v3, 2023). Fetched `https://arxiv.org/abs/2211.10665`.
- **Mechanism:** Randomized search over assembly programs with repeated benchmarking, plus a new
  verified program-equivalence checker connected to Fiat Cryptography, so the final Coq theorem
  mentions only the functional program and x86-64 semantics.
- **What it operates on:** Functional cryptographic programs → x86-64 assembly; equivalence is
  checked with "a modest subset of known features of SMT solvers and symbolic-execution engines".
- **Exact passage:** "we connect to the Fiat Cryptography framework (which translates functional
  programs into C-like IR code) and extend it with a new formally verified program-equivalence
  checker" — `https://arxiv.org/abs/2211.10665`
- **What it explicitly does NOT do:** It does **not** invert the direction: it never recovers
  algebraic structure from an existing shipped implementation. Its input is always a high-level
  functional program. No stated limitation; the direction is fixed by construction.
- **Nearest Tekmerion mechanism:** symbolic execution (equivalence checker) + sparse premise
  selection.
- **Distance:** To serve Tekmerion it would have to start from an object file and *infer* the digit
  decomposition and the limb weights, which is the inverse problem and is not attempted.
- **Certified?** Yes, by Coq's kernel on the equivalence proof.

## 9. AMulet2 (and the column-wise paper)

- **Citation (a):** Kaufmann, Biere. *Improving AMulet2 for verifying multiplier circuits using SAT
  solving and computer algebra.* Int. J. Softw. Tools Technol. Transfer 25:133–144, 2023. Fetched
  `https://link.springer.com/article/10.1007/s10009-022-00688-6` (open access, full text).
- **Citation (b):** Kaufmann, Biere, Kauers. *Incremental column-wise verification of arithmetic
  circuits using computer algebra.* Form. Methods Syst. Des. 56(1):22–54, 2020. Fetched
  `https://pmc.ncbi.nlm.nih.gov/articles/PMC7691315/` (full text).
- **Mechanism:** Model an AIG as polynomials over Z[X] with Boolean value constraints; the gate
  polynomials form a D-Gröbner basis under a reverse-topological order, so verification is
  substitution (D-reduction) of the specification polynomial. AMulet2 adds adder substitution, XOR
  slicing, monomial sharing; the column-wise paper adds incremental per-column slices with explicit
  **carry recurrence relations**.
- **What it operates on:** Gate-level AIG circuits (bit-level multipliers). Not machine code.
- **Exact passage (a):** "However, proof checking consists of calculating one huge linear
  combination, making it nearly impossible to locate a possible error in the certificate." —
  `https://link.springer.com/article/10.1007/s10009-022-00688-6`
  **Exact passage (b):** "we present a novel incremental column-wise technique to verify gate-level
  multipliers" — `https://pmc.ncbi.nlm.nih.gov/articles/PMC7691315/`
- **What it explicitly does NOT do:** (b) states the scalability boundary: "more complex and
  optimized multipliers require more sophisticated techniques" and, in the experiments, synthesized
  and technology-mapped multipliers fare worse — the paper reports "Even an 8-bit multiplier cannot
  be verified any more". No instruction semantics, no LCF kernel.
- **Nearest Tekmerion mechanism:** carry-chain aggregate (b's carry recurrence
  `-C_i + 2C_{i+1} + s_i - P_i ∈ I(C)` is the closest published relative of the weighted aggregate)
  + certificate language (PAC / Nullstellensatz / LPAC).
- **Distance:** Tekmerion's aggregate is over *instruction-produced* limb equations with weights
  `1, B, B², …`; AMulet's is over *gate* polynomials with weights `2^i` and its conclusion is a
  Gröbner-basis remainder. To port: replace D-reduction with a kernel-checked linear combination and
  supply the ISA semantics; also accept that AMulet's soundness argument is circuit-shaped.
- **Certified?** Yes: PAC (step-wise), Nullstellensatz (single linear combination with explicit
  integer/polynomial cofactors) and LPAC; checked by Pacheck/Pastèque, external checkers. The
  Nullstellensatz form is *exactly* "explicit integer multipliers summing to the goal" — but for
  circuits and outside any LCF kernel.

## 10. Small Proofs from Congruence Closure

- **Citation:** Flatt, Coward, Willsey, Tatlock, Panchekha. *Small Proofs from Congruence Closure.*
  arXiv:2209.03398 (2022). Fetched `https://arxiv.org/abs/2209.03398`.
- **Mechanism:** An O(n^5) optimal-proof algorithm under a relaxed "proof tree size" metric, plus an
  O(n log n) greedy variant, implemented in the `egg` equality-saturation toolkit; the first
  certifying equality-saturation engine.
- **What it operates on:** E-graph congruence-closure proof certificates (proof terms/dags internal
  to `egg`); benchmarked against Z3 on 3760 problems.
- **Exact passage:** "we introduce an O(n^5) time algorithm which generates optimal proofs under a
  new relaxed "proof tree size" metric that directly bounds proof size" —
  `https://arxiv.org/abs/2209.03398`
- **What it explicitly does NOT do:** No LCF kernel, no HOL, no arithmetic; the object minimised is
  an e-graph derivation, and the motivation is conflict clauses and certificate size, not kernel
  inference count. **No stated limitation** beyond NP-completeness of minimal size.
- **Nearest Tekmerion mechanism:** proof-object replay (minimisation of a derivation).
- **Distance:** Would need (a) a representation of a HOL Light kernel derivation as a shareable DAG
  with explicit rule nodes, and (b) a cost model matching primitive inferences. Neither exists in
  the paper.
- **Certified?** Yes, but self-certifying inside `egg`; there is no independent kernel.

## 11. Carcara

- **Citation:** Andreotti, Lachnitt, Barbosa. *Carcara: An Efficient Proof Checker and Elaborator for
  SMT Proofs in the Alethe Format.* TACAS 2023, LNCS 13993, pp. 367–386. Fetched
  `https://link.springer.com/chapter/10.1007/978-3-031-30823-9_19` (open access, full text).
- **Mechanism:** A standalone Rust checker for Alethe proofs (90 rule checkers) plus an elaborator
  that replaces coarse steps (implicit equality reordering, `lia_generic`) with fine-grained ones.
- **What it operates on:** Alethe SMT proof traces (first-order clauses + theory lemmas).
- **Exact passage:** "Both rules conclude a clause of negated linear inequalities, which is valid due
  to Farkas' lemma" — `https://link.springer.com/chapter/10.1007/978-3-031-30823-9_19`
- **What it explicitly does NOT do:** It states the limit on the arithmetic step: `lia_generic`
  "essentially requiring the arithmetic solving to be repeated in the checker", so such steps are
  treated as holes. No LCF kernel; the checker is the oracle.
- **Nearest Tekmerion mechanism:** certificate language + sparse premise selection.
- **Distance:** Alethe's `la_generic` carries exactly the coefficient vector Tekmerion wants to emit,
  but the consumer is a Rust checker, and the clauses are SMT atoms, not limb equations.
- **Certified?** Yes; checking is the certification. Emphatically not a kernel: a bug in Carcara is a
  soundness bug for the whole pipeline.

## 12. Candle (extended)

- **Citation:** Abrahamsson, Myreen, Kumar, Sewell. *Candle: A Verified Implementation of HOL Light
  (Extended Version).* J. Automated Reasoning 69, art. 32, 2025. Fetched
  `https://doi.org/10.1007/s10817-025-09743-8` (open access, full text).
- **Mechanism:** A HOL Light clone running on CakeML, with an end-to-end soundness theorem from the
  machine code up: anything printed on the kernel FFI channel is a HOL theorem.
- **What it operates on:** The prover implementation itself (source → CakeML → machine code).
- **Exact passage:** "every exported fact produced by this machine-code program is valid in
  higher-order logic" — `https://doi.org/10.1007/s10817-025-09743-8`
- **What it explicitly does NOT do:** No decision-procedure or tactic optimisation; it reports that
  Candle is slower than HOL Light at start-up (`#use "hol.ml"` "takes approximately 50% longer").
  It also flags the opening it creates: "This opens up the possibility to implement various decision
  procedures, e.g. for linear arithmetic, in the logic and run them faster".
- **Nearest Tekmerion mechanism:** support library / proof-object replay (the trusted base for
  anything Tekmerion replays).
- **Distance:** Candle gives a place to *put* a fast in-kernel arithmetic procedure; it does not
  provide one. That sentence is the clearest published statement that such a procedure is missing.
- **Certified?** Yes, in-kernel, and down to machine code.

## 13. Fast, Verified Computation for Candle

- **Citation:** Abrahamsson, Myreen. *Fast, Verified Computation for Candle.* ITP 2023, LIPIcs 268,
  4:1–4:17. Fetched `https://doi.org/10.4230/LIPIcs.ITP.2023.4`.
- **Mechanism:** A new kernel function for fast in-logic computation (compiled evaluation), added to
  the Candle kernel and proved sound.
- **What it operates on:** In-logic terms to be normalised/evaluated; produces theorems by
  computation rather than by primitive inference sequences.
- **Exact passage:** "Experiments show that the new computation function is able to speed up certain
  evaluation proofs by several orders of magnitude." — `https://doi.org/10.4230/LIPIcs.ITP.2023.4`
- **What it explicitly does NOT do:** It is computation, not decision: no linear arithmetic, no
  Farkas, no certificates, no machine code. No stated limitation.
- **Nearest Tekmerion mechanism:** proof-object replay (cheap kernel-produced derivation).
- **Distance:** This is the strongest published precedent for "replace a tactic with an in-kernel
  fast path", and the natural host for a carry-chain decision procedure. The missing piece is the
  arithmetic procedure itself.
- **Certified?** Yes — the new function is proved sound in the Candle kernel.

## 14. Fox, Improved Tool Support for Machine-Code Decompilation in HOL4

- **Citation:** Fox. *Improved Tool Support for Machine-Code Decompilation in HOL4.* ITP 2015,
  LNCS 9236, pp. 187–202. Fetched `https://link.springer.com/chapter/10.1007/978-3-319-22102-1_12`
  (abstract, paywalled) and the author PDF via `https://r.jina.ai/https://acjf3.github.io/papers/itp15.pdf`
  (full text).
- **Mechanism:** Replace per-opcode symbolic evaluation with a database of pre-computed **generic
  instruction-semantics theorems** (derived by MATCH_MP from fetch/decode/run/next-state lemmas),
  cached and specialised to concrete opcodes.
- **What it operates on:** ARM/ARMv8/x86-64/MIPS/CHERI ISA models in L3 → HOL4; machine-code
  verifications (including a 1024-bit multiword addition and a 256-bit Skein example).
- **Exact passage:** "Rather than generate Hoare triple theorems from scratch for every machine-code
  opcode (which is expensive), we can instead dynamically build up a database of generic triples" —
  `https://r.jina.ai/https://acjf3.github.io/papers/itp15.pdf`
  Count claim, verbatim: "The ARM model is complex and, at the time of writing, there are 318 of
  these theorems." (same URL)
- **What it explicitly does NOT do:** No arithmetic decision procedure; no certificate; the cache is
  of *semantics* theorems, not of arithmetic lemmas. It does not optimise proof size — it optimises
  time. **No stated limitation** on inference counts, because it does not target them.
- **Nearest Tekmerion mechanism:** step-theorem cache.
- **Distance:** The cache exists and is measured, but the measured quantity Tekmerion cares about
  (primitive inferences of the arithmetic close) is untouched; and the ISA is HOL4, not HOL Light.
- **Certified?** Yes, in-kernel (HOL4). **Note for Q9:** Tables 2 and 3 of this paper are headed
  "decompilation time (and inferences)" and give figures such as `142.5 s (73,039 i)`,
  `7.8 s (1.5 Mi)`, `70 s (1,029,685 i)`, `37 s (8.9 Mi)`. Primitive inference counts for
  machine-code proofs therefore **do** exist in print; the brief's premise that Fox measured only
  wall time is not correct.

## 15. Lean `ite_congr` / contextual if-then-else blow-up

- **Citation:** Lean core/metaprogramming documentation, `Lean.Meta.Sym.Simp.SimpM` ("Structural
  Simplifier for Symbolic Simulation"), Lean FRO / Lean community docs mirror. Fetched
  `https://teorth.github.io/expdb/docs/Lean/Meta/Sym/Simp/SimpM.html`. **Status: primary Lean source
  documentation, not a peer-reviewed paper.** The claim is documented, not measured in a paper.
- **Mechanism:** A specialised simplifier for symbolic simulation that avoids the standard
  congruence-based treatment of `ite` and instead simplifies only the condition, keeping `ite`
  symbolic so the simulator can case-split.
- **What it operates on:** Lean expressions during symbolic execution of programs.
- **Exact passage:** "The standard `ite_congr` theorem adds hypotheses when entering branches,
  invalidating the cache and causing O(2^n) behavior on conditional trees." —
  `https://teorth.github.io/expdb/docs/Lean/Meta/Sym/Simp/SimpM.html`
- **What it explicitly does NOT do:** It offers a workaround (non-contextual `ite`), not a proof
  technique; no certificate, no kernel-level accounting, no machine code. No stated limitation
  beyond naming the blow-up.
- **Nearest Tekmerion mechanism:** symbolic execution.
- **Distance:** Directly relevant as a *negative control*: any branch-containing instruction sequence
  risks the same blow-up, and the fix (keep the conditional symbolic, defer case-split) is cheap to
  adopt. It says nothing about arithmetic closes.
- **Certified?** Not applicable: this is a tactic/simplifier, not a certifying procedure. Proof terms
  are still produced by Lean's elaborator.

## 16. MM0 (supplementary, for Q8)

- **Citation:** Carneiro. *Metamath Zero: The Cartesian Theorem Prover.* arXiv:1910.10703 (v3,
  2020). Fetched `https://arxiv.org/abs/1910.10703` (abstract).
- **Mechanism:** Multi-sorted first-order logic with a binary proof format designed for essentially
  linear-time verification.
- **What it operates on:** Proof objects in the `mm0` binary format; `set.mm` and 71 of Wiedijk's 100.
- **Exact passage:** "Metamath Zero currently holds the record for fastest verification of the
  `set.mm` Metamath library of proofs in ZFC … at less than 200 ms." —
  `https://arxiv.org/abs/1910.10703`
- **What it explicitly does NOT do:** Not an LCF kernel; logic is deliberately minimal; no
  arithmetic decision procedures. No stated limitation.
- **Nearest Tekmerion mechanism:** proof-object replay.
- **Distance:** A throughput reference point only.
- **Certified?** Yes, by `mm0-c`; the paper's stated goal is to verify the verifier down to binary.

## 17. cake_lpr (supplementary, for Q8)

- **Citation:** Tan, Heule, Myreen. *cake_lpr: Verified Propagation Redundancy Checking in CakeML.*
  TACAS 2021, LNCS 12652, pp. 223–241. Fetched
  `https://link.springer.com/chapter/10.1007/978-3-030-72013-1_12` (abstract only).
- **Mechanism:** LPR proof format plus a CakeML-verified checker for propagation redundancy.
- **What it operates on:** Clausal (LPR/LRAT) SAT proof certificates.
- **Exact passage:** "we experimentally show that LPR provides efficiency gains over existing proof
  formats and that the strong correctness guarantees are obtained without significant sacrifice in
  the performance of the verified executable" —
  `https://link.springer.com/chapter/10.1007/978-3-030-72013-1_12`
- **What it explicitly does NOT do:** Not an LCF kernel; no arithmetic; no machine code. No stated
  limitation.
- **Nearest Tekmerion mechanism:** certificate language.
- **Distance:** Relevant only as evidence that verified *standalone* checkers are fast; not a model
  for kernel replay.
- **Certified?** Yes, by a verified CakeML binary.

---

## Answers to Q1–Q9

**Q1. Farkas/cutting-plane certificates applied to carry-chain aggregation in machine code?**
**No.** Farkas is used for SMT linear arithmetic (Carcara §3.1; Lambdapi LIA §2.2, §5) and for
pseudo-Boolean LP integration (VeriPB abstract: "linear programming integration with Farkas
certificates and cut generation"), over SMT clauses / 0-1 constraints respectively. Carry chains are
handled algebraically, as Gröbner reduction over *circuits* (AMulet column-wise, carry recurrence
`-C_i + 2C_{i+1} + s_i - P_i ∈ I(C)`). No work found that applies Farkas certificates to per-limb
equations arising from instruction semantics. arXiv query `"Farkas" "machine code" certificate`
returned 0 results.

**Q2. Certified sparse elimination (integer λ with λᵀAᵢ = 0) inside a proof compiler?**
**Not as such.** Two constituents exist. CoqCryptoLine checks ideal-membership witnesses
`q = Σ cᵢpᵢ` with `norm_subst` and states that "Elimination of ideal generators through variable
substitution is an efficient approach to simplify an ideal membership problem", but the elimination
is a verified *transformation*, not a per-step checked λ-certificate. AMulet's Nullstellensatz format
supplies explicit integer/polynomial cofactors and is checked by Pacheck, but the elimination is
tool-internal preprocessing and the certificate is the final combination. I found no paper where the
proof compiler *emits* the multipliers that cancel internal variables and the kernel checks that
cancellation. (query `proof minimization`+kernel and `congruence closure`+kernel both 0 on arXiv;
this specific query was not runnable — see "returned nothing relevant" in the log.)

**Q3. Congruence-closure proof minimisation combined with LCF/HOL Light proof objects?**
**No.** The only minimisation work found is in `egg` (Flatt et al., arXiv:2209.03398), which
minimises an e-graph derivation and self-certifies; the applied work is SAT/resolution (Gorzny et al.
2018; Sidorov et al. 2024), DNN-verifier proofs (Isac et al. 2025) and equational rewriting
(Krympa 2026). None mentions a kernel, HOL Light, or an LCF derivation DAG.

**Q4. Is VeriPB/CakePB or PB proof logging integrated with an LCF kernel?**
**No — always a separate checker.** CakePB is a CakeML-verified binary; the paper's claim is that
performance is now "quite close to the level of SAT solving", not that it produces kernel
derivations. CakeML is a compiler/runtime project, not an LCF kernel, even though Candle is built on
it.

**Q5. Does Carcara/Lambdapi-style certificate elaboration feed an LCF kernel?**
**No.** Carcara is a standalone Rust checker; its elaborator rewrites Alethe into easier-to-check
Alethe. Lambdapi is a λΠ-modulo proof assistant with its own type checker; it can export to Rocq,
Lean and HOL Light, but the arithmetic steps are discharged by reflection *inside* Lambdapi before
any export. Neither hands inferences to a HOL-Light-style kernel.

**Q6. Prior art for inverting Fiat Cryptography (recover structure from an implementation and
specialise generic proofs)?**
**None found.** The direction of travel is uniform: Fiat Crypto (spec → C), CryptOpt (functional
program → x86-64 with a verified equivalence checker), Jasmin ("game hopping", reference
implementation → optimised implementation, both in the Jasmin source language; the compiler is
verified, arXiv:1904.04606 and arXiv:2511.11292). The nearest thing to going from assembly *up* to
algebra is CoqCryptoLine/CryptoLine, which checks a hand-written algebraic model *of* an assembly
routine — but the model is supplied by the user (230 lines, 30 of them hints for `mul_montx`), not
recovered. No "lifting assembly to algebraic semantics" work appeared in queries
`Jasmin cryptography compiler` (2 hits, both source→assembly) or the CryptOpt abstract.

**Q7. Proof supercompilation / proof-DAG optimisation across a whole development?**
**No.** "Proof compression" on arXiv (14 hits) is proof-theoretic (subatomic logic, guarded
substitutions; Pi_2-cut introduction; tree-to-DAG compression for complexity results) or per-proof
engineering (partial regularization of resolution proofs; branch-and-bound minimal resolution
proofs; Krympa). Every engineering result is per-proof and outside a kernel. Nothing optimises a
whole LCF development, and nothing uses kernel inference count as the objective.

**Q8. Measured checker throughputs.**
- **MM0 (`mm0-c`):** `set.mm` (ZFC, incl. 71 of Wiedijk's 100) verified in "less than 200 ms" —
  arXiv:1910.10703 abstract. **Hardware not stated in the fetched abstract.** Units: whole-library
  wall time.
- **CakeML / Candle:** **Not found.** cake_lpr's abstract claims efficiency gains "without
  significant sacrifice in the performance of the verified executable" but states no rate; Candle
  (JAR 2025) gives only a relative figure ("approximately 50% longer" than HOL Light to load
  `hol.ml`), and Candle/ITP 2023 gives "several orders of magnitude" speedup on evaluation proofs
  with no absolute number. I did not fetch the paywalled bodies of these two.
- **Carcara:** corpus-scale figures only: 39,229 veriT proofs totalling 92 GB, largest 4.5 GB;
  checking faster than solving for 81.61% of proofs; run on AWS Graviton2 2.5 GHz with 4 GB per job.
  Absolute proofs/second is in Table 1, which did not render in the fetched HTML. **Rate not
  obtained.**

**Q9. Precomputed instruction-semantics step theorems *and* primitive inference counts?**
**Yes — and this corrects the premise.** Fox ITP 2015 precomputes instruction-semantics theorems
("there are 318 of these theorems" for ARM) and reports inference counts alongside time in both
performance tables, headed "decompilation time (and inferences)", with rows such as
`142.5 s (73,039 i)` / model evaluation `7.8 s (1.5 Mi)`, `70 s (1,029,685 i)` / `37 s (8.9 Mi)`,
and after the caching work warm-state figures such as `0.02 s (15,521 i)` and `4.0 s (2.8 Mi)`.
(Caveat: the table columns in the extracted PDF are misaligned, so I do **not** attribute a
particular figure to a particular example; the presence of inference counts is unambiguous, the
per-row mapping is not.) So inference counts for machine-code proofs are published, on a
step-theorem-cached pipeline, alongside a bignum addition example. What is *not* published
is (a) an arithmetic-close inference count separated from semantics/decompilation, or (b) any
attempt to *minimise* the count rather than the time.

---

## Provisional gap map

| combination / inversion | constituents | nearest neighbour found | appears untried? |
|---|---|---|---|
| Farkas/cutting-plane certificate for a carry-chain aggregate over **instruction traces** | Farkas (Carcara, Lambdapi); cutting planes (VeriPB); carry recurrence (AMulet col-wise) | AMulet column-wise — but over circuit gates, via Gröbner reduction | yes |
| Sparse elimination emitted as integer multipliers and checked **in HOL Light** | CoqCryptoLine cofactor checking; AMulet Nullstellensatz cofactors | CoqCryptoLine (Coq + external CAS) | unverified |
| Equivalence-graph / congruence-closure proof minimisation applied to an **LCF kernel derivation DAG** | Flatt et al. (egg); resolution/DNN proof minimisation | none for LCF | yes |
| Pseudo-Boolean proof logging (VeriPB/CakePB) as the certificate layer under a HOL-Light arithmetic close | VeriPB, CakePB, Candle | CakePB — standalone verified checker | unverified |
| Elaborated certificate (Carcara-style) replayed into an LCF kernel instead of a standalone checker | Carcara elaborator; Lambdapi LIA reflection | Carcara (standalone) | yes |
| Inversion of Fiat Crypto: infer digit structure + limb weights from a shipped object and specialise a generic proof | CryptOpt; Jasmin; CryptoLine assembly models | CoqCryptoLine's hand-written assembly models | unverified |
| Proof-DAG supercompilation across a whole development, not one proof | proof compression / partial regularization / Krympa | per-proof only | yes |
| In-kernel verified computation as a replacement for the decision-procedure close | Candle fast computation (ITP 2023) | Candle | unverified |
| Optimising a step-theorem cache for **inference count** rather than wall time | Fox ITP 2015 (318 theorems; "(and inferences)" tables) | Fox | unverified |
| Digit weights `1, B, B², …` per limb used as a *certificate* rather than as a reduction order | AMulet column-wise (`2^i` weights) | AMulet | yes |
