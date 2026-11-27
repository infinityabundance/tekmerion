# 04 — Novelty register

**Date:** 2026-11-22. Companion to `05-FRONTIER-FACTS.md`; evidence in `research/receipts/`.

## How to read this

Taxonomy is exactly: `EXISTING` · `KNOWN_COMBINATION` · `UNVERIFIED_COMBINATION` ·
`PLAUSIBLY_NOVEL` · `DEMONSTRATED_ADVANCE`.

Novelty increases left to right. Nothing here is `DEMONSTRATED_ADVANCE`: that requires a result,
and this register records designs. `EXISTING` means the mechanism as stated is published;
`KNOWN_COMBINATION` means the combination is published; `UNVERIFIED_COMBINATION` means the
constituents exist and I could not verify that anyone composed them; `PLAUSIBLY_NOVEL` means I
searched and found nothing close, which is not the same as nothing existing.

### Evidence markers (applied in every row)

- **[P]** quotation or measurement from a primary source, recorded in a receipt.
- **[S]** a dedicated search was run and logged (`receipts/n-mechanism-search-log.md`).
- **[†]** **no dedicated search was run.** The nearest neighbour is named from the repository,
  the prior-art audit, or general knowledge. Treat the classification as provisional.
- **[2°]** the neighbour is cited from secondary coverage and was not fetched here.

### The discipline I applied

If a constituent technique exists, the row cannot exceed `UNVERIFIED_COMBINATION`. I therefore
downgraded two rows that the earlier audit (`01-PRIOR-ART.md`) had marked more favourably: M-B
and M-F, whose *constituents* are known but whose stated compositions are not published.

---

## Table A — v2 mechanisms

| Mechanism | Nearest neighbour(s) with citation | What remains unclaimed | Class | The single experiment that would settle it |
|---|---|---|---|---|
| **M-A** De-optimisation / proof by twin: prove a simple twin, discharge twin ≡ shipped with `ensures2`, compose | **Published in-tree.** s2n-bignum `doc/program_equivalence.md` §2: prove `unopt` functional correctness, prove `unopt ↔ interm1 ↔ final` by `ensures2`, compose by transitivity, derive `ensures` on the optimised core — for `bignum_montmul`/`montsqr` p256/p384/p521 **[P]**. Foundation: Mazzucato et al., relational Hoare logic `L2`, arXiv 2505.14348 **[P]** | Only the **agent policy**: an agent choosing to *invent* a de-optimised twin as a proof move, and the measured FC solve-rate delta at equal budget. The scalar reference already exists in-tree (`arm/*/unopt/*.o`), so "the twin" is not the new part | **EXISTING** | Two-arm A/B on the same 40 FC tasks at fixed token and kernel-call budget: arm T may introduce a twin, arm C may not. Endpoint: kernel-accepted `ensures` proofs of the **shipped** object. Pre-register both arms and the identical attempt count |
| **M-B** External certified algebra (Gröbner / ideal-membership cofactors, in-kernel ring check, LRAT for bit-width side conditions) into HOL Light **object-code** proofs | `bv_decide` in Lean; Jazzline inside Jasmin; Cadical/LRAT in HOL Light; HOL Light's own integer ring/ideal machinery **[2°]**; AlphaProof Code already proves bit-vector side conditions "in Lean's kernel" **[P]** | Certified algebra *beneath* s2n-bignum's `ensures` goals at object-code scale: emitting cofactors and LRAT so that the hand-written bit-vector block shrinks, with the same theorem accepted | **UNVERIFIED_COMBINATION** | Replace the hand-written bit-vector side-condition block in one `bignum_montmul_p256` proof with an emitted certificate. Endpoint: the same theorem accepted by the pinned kernel, with fewer proof-script lines and reported checker time. Reject if the certificate checker must be trusted |
| **M-C** Executable ISA model as a cheap falsifier: run each proposed invariant on the machine model before the kernel | s2n-bignum CI co-simulation (`SOUNDNESS.md` B1); LNSym, Islaris, arm-tv **[2°]**. AlphaProof Code: "Agents also try to falsify a goal before work on it starts, to avoid wasting resources on unprovable subgoals" **[P]** | The *role* in the loop and the two metrics: kernel calls per accepted proof, and false invariants reaching the kernel. Falsify-before-prove is already practice; per-invariant model execution with those metrics is not published | **UNVERIFIED_COMBINATION** | Instrument the loop so every candidate invariant is executed on the model first. Endpoint: ratio of kernel invocations to kernel-accepted proofs, gate on vs off, plus the count of invariants the model refutes that the ungated loop would have sent to the kernel |
| **M-D** Cross-ISA proof transport from the system's own proved lemmas; cache holds no human proof text | Per-architecture proof developments: s2n-bignum has both `arm/proofs/bignum_montmul_p256.ml` and `x86/proofs/bignum_montmul_p256.ml`, each stating its own `*_SUBROUTINE_CORRECT` about its own object; they share libraries (`common/bignum.ml`) but **not proof terms** (measured: 0 files in either proofs tree load a shared spec module). Proof-term transport across equivalences: PUMPKIN Pi, arXiv 2010.00774 **[P]**. Query "cross-ISA" returned 8 hits, all binary translation or build repair **[S]** | Transporting *proof terms* (not specifications) between ISAs, through a machine-checked relation between the two ISA models. No near neighbour found, but no dedicated query on "cross-architecture proof reuse" was run **[†]** | **UNVERIFIED_COMBINATION** | Ablate the cache: measure conditional x86 solve rate given ARM-proved lemmas versus unconditional, at equal budget, with cache order ablated. Reject if the transport requires the x86 proof text |
| **M-E** DSFB on symbolic-simulation traces: per-step term size, abbreviation count, open side conditions; typed early-cutoff reason instead of timeout | Branch-and-bound / early cutoff in ATP; learned search guidance; timeout prediction **[2°]**. AlphaProof Code's falsify-before-work gate **[P]**. Negative control: one-step transition ranking is "insufficient as a long-horizon search value", 282.3 vs 308 of 987 solved, arXiv 2609.32908 **[P]** | The typed residual vector (term size, abbreviation count, open side conditions) as an *early cutoff with a reason*, never a timeout label. The only nearby thing is an untyped falsification gate | **UNVERIFIED_COMBINATION** | On recorded symbolic-simulation traces, compare accepted proofs per kernel call for: no cutoff, timeout-only cutoff, DSFB typed cutoff. Endpoint is kernel-accepted proofs per kernel call; no timeout may be used as a training label |
| **M-F** `rustc` output verified against **s2n-bignum's own specification**; no new ISA model; held-out by construction | **AlphaProof Code**: stock `rustc` 1.98.1 → AArch64, verified at machine level, "the Rust compiler is _not_ trusted", arXiv site 2026-11-10 **[P]**. Aeneas / RefinedRust / hax / Corten: source-level, trust `rustc` **[2°]**. Translation validation: seL4, arm-tv **[2°]** | The pairing, not the headline: **HOL Light**, **s2n-bignum's existing `ensures` spec**, a pinned object digest, and a contamination-free held-out set. "Nobody has verified `rustc` output at machine level" is now false | **UNVERIFIED_COMBINATION** | Compile one openssl-rs P-256 kernel with a pinned `rustc`, record the object's SHA-256, prove s2n-bignum's `ensures` statement about it in HOL Light. Endpoint: kernel-accepted or not, with version and digest in the claim. Then repeat on a held-out sibling |

---

## Table B — N1–N12

| Mechanism | Nearest neighbour(s) with citation | What remains unclaimed | Class | The single experiment that would settle it |
|---|---|---|---|---|
| **N1** Partial proof term with typed holes as the search state; residual = hole count, hole type complexity, unification-failure pair | Typed holes as a first-class synthesis context: Hazelnut Live, arXiv 1805.00155; LLMs prompted on a hole's typing context, arXiv 2409.00921 (OOPSLA 2024) **[S]**. Goal-count heuristics in Aesop/`auto`; proof-graph goal selection in CoCo-Prover, arXiv 2610.09681 **[P]** | The three-part residual and the equal-budget comparison against goal-size pruning. **Blocking feasibility issue:** HOL Light has no dependent typed holes or evars — it is an LCF goalstack. N1 needs Lean/Coq/Agda, or a bespoke elaborator | **UNVERIFIED_COMBINATION** | In a term-level prover, log the triple at every step; fit a ranker; endpoint = nodes expanded to first kernel-accepted proof at equal kernel-call budget versus a goal-count-only baseline |
| **N2** CEGIS for lemmas: SMT counterexample → generalised lemma → kernel proof; count only lemmas reused in another proof | CEGIS / invariant synthesis (SyGuS, Sketch lineage); theory exploration (QuickSpec, HipSpec, AProS) **[2°]**; LemmaNet, arXiv 2603.22114 (ASE 2026), discovers and adapts helper lemmas from program comprehension **[P]** | The **reuse-in-a-different-proof** metric as the only countable yield, and generalisation of the counterexample rather than of the source code | **UNVERIFIED_COMBINATION** | On 20 failing obligations, run the counterexample → lemma → kernel loop. Endpoint: number of kernel-checked lemmas that are used in a *different* proof, with the reuse witnessed by a dependency check |
| **N3** Conformal prediction / distribution-free abstention for doomed branches; preregistered coverage, fail closed | Conformal prediction: Angelopoulos & Bates, arXiv 2107.07511 **[P]**. Learnable conformal abstention for LLMs/VLMs, arXiv 2502.06884 **[P]**. Queries "conformal prediction theorem proving" (15 hits, none relevant), "… proof search" (0), "… code generation abstention" (1, LLM abstention only) **[S]** | Applying conformal coverage to *proof-search abstention*: an unclaimed combination. Risk to state explicitly: the guarantee needs exchangeability, which a temporal hold-out (S2) may violate; and the "label" is a kernel timeout, not a clean ground truth | **PLAUSIBLY_NOVEL** (not demonstrated) | Preregister 90% coverage of "doomed" labels on a calibration split. Endpoint: empirical coverage and false-abstention rate on a temporal hold-out. Fail closed if coverage is below the bound minus slack |
| **N4** Dense kernel-grounded reward `r_t = -ΔGoalComplexity - λ·KernelCalls + μ·CertificateProgress`; sparse vs dense at equal budget | Verifier-in-the-loop per-step feedback, arXiv 2503.09730; step-aware Lean verification as reward, arXiv 2506.04592; hybrid predicted+real reward on GPU, arXiv 2610.21881; RLPAF and intrinsic MCTS reward, arXiv 2408.08152; outcome reward from the Lean compiler, arXiv 2504.06122 **[S]** | The exact residual and the **kernel-call** budget as the comparison axis. "Dense beats sparse" is not the claim to make — it is already studied and mixed | **EXISTING** (mechanism) | Sparse vs dense at equal kernel-call *and* token budget on the same task set. Endpoint: kernel-accepted proofs. Include a negative control (constant reward, same compute). Pre-register the exact reward formula |
| **N5** Proof repair by transporting proof terms along a semantics-preserving diff; zero-search transport, statement guarded | PUMPKIN Pi, arXiv 2010.00774: "transport across equivalences", zero search, "does not rely on axioms beyond those Coq assumes" **[P]**. s2n-bignum `ensures2` composition **[P]**. CAPRI / VerusWeave / TopoVerus for contract-aware repair **[2°]** | Transport along a **code** diff whose equivalence is a machine-checked object-code theorem, plus a **guard that the theorem statement is unchanged**. PUMPKIN Pi changes types, so statements change by construction | **UNVERIFIED_COMBINATION** | For K kernels whose optimisation is covered by a proved `ensures2`, measure the fraction of old proof terms that transport with **zero search** and pass a statement-identity check. Endpoint: zero-search transport rate with zero statement changes |
| **N6** Dual-logic acceptance with a translation certificate; planted bugs (binder capture, definition shadowing, implicit-argument mismatch) must be caught | Lean `comparator` + AlphaProof Code gatekeeper: "the statement **and every definition it depends on**", held-out copy, standard axioms only **[P]**. hol2dk → Dedukti; OpenTheory; ProofCert framework **[2°]** | A certificate for the **translation itself**, with the three planted bug classes as controls. Statement-identity checking is a different, weaker check | **UNVERIFIED_COMBINATION** | Plant one bug from each class into the HOL Light → Lean translator. Endpoint: detection at zero false alarms over a corpus of N correctly translated theorems |
| **N7** Slice ISA equivalence over instructions the shipped kernel actually executes; bind to the slice digest; mutation controls must fire | s2n-bignum CI co-simulation (`SOUNDNESS.md` B1) and per-instruction coverage accounting; LNSym/Islaris **[2°]** | Slice extraction from *actual traces*, digest binding so a model change revokes proofs, and mutation controls that are required to fire. The prior audit found no object-byte digest checks in `tools/` or the Makefiles (prior audit, `01-PRIOR-ART.md`; not re-measured here) | **UNVERIFIED_COMBINATION** | Extract the executed-instruction slice for one kernel. Endpoint: the slice digest changes when one executed instruction's semantics is mutated, **and** the proof fails under that mutation. If the control does not fire, the claim is dead |
| **N8** Relational leakage by transformation: prove the transformer once; a leaky control must be distinguished or the build fails | Jasmin (speculative CT at source + one compiler pass); s2n-bignum sequential-only CT (`SOUNDNESS.md` B3 excludes speculation) **[2°]** | Proving the transformation once and making the **negative control** a build gate, rather than a paper observation | **UNVERIFIED_COMBINATION** | Implement a leaky variant (data-dependent branch) and a non-leaky twin. Endpoint: the check rejects the leaky one and accepts the non-leaky one, both reported. A check that passes both is not a check |
| **N9** Cross-logic failure intersection (HOL Light vs Lean) as a diagnosis channel feeding N2 | Differential testing; cross-prover lemma reuse; proof mining **[2°]**. No dedicated search run **[†]** | Using the *intersection of failing subgoals* as a precision-weighted diagnosis signal, and measuring whether the lemmas it yields unblock the original proof | **UNVERIFIED_COMBINATION** | Take N obligations that fail in both logics. Endpoint: fraction of intersected subgoals whose synthesised lemma (N2) unblocks the original proof; report the base rate from non-intersected subgoals |
| **N10** Proof difficulty as a specification-health metric; detect a weakened postcondition (e.g. Montgomery `out < m` dropped) | Mutation testing of specifications: AlphaProof Code's 17 synthetic mutants, 16 with specification-level refutations **[P]**. NACT, arXiv 2503.01538 **[2°]**. Weakened assurance theorem found by audit, arXiv 2608.18822 **[2°]** | Difficulty *collapse* as the signal, combined with mutation sensitivity, rather than either alone. Requires a difficulty metric that is stable under irrelevant edits | **UNVERIFIED_COMBINATION** | Delete `out < m` from the Montgomery postcondition. Endpoint: the difficulty metric collapses **and** a mutation control fires; restore and require both to reverse. Calibrate first that the metric does not move on semantics-preserving edits |
| **N11** GPU-parallel certificate checking with CPU aggregation and recheck; target 2× replay at identical accepted theorems | CPU-parallel proof checking: Isabelle/PIDE multi-core checking, arXiv 1207.3441 **[P]**. Verified sequential checkers: `cake_lpr`, `Par` **[2°]**. GPU *prediction* of Lean verdicts (4.2× throughput on one H100) is a surrogate, not a checker, arXiv 2610.21881 **[P]**. Queries "GPU proof checking parallel" (4 hits, none relevant), "parallel proof checking Coq" (2, neither a checker) **[S]** | GPU certificate checking itself: nothing found. **Structural ceiling:** LCF derivations are sequentially dependent, so the parallel unit is *independent obligations*, not a single derivation. A 2× target must be stated per-obligation, not per-derivation | **UNVERIFIED_COMBINATION** | Check M independent obligations with GPU workers, CPU aggregation and a full recheck. Endpoint: wall-clock speedup at an **identical set of accepted theorems** (not identical proofs). Report the independence fraction — it is the hard upper bound on any speedup |
| **N12** SemantEq-preserving transforms as a contamination probe; original-minus-transform gap is the memorisation score | CodeMorph, arXiv 2506.17627: "26 semantic-preserving transformation methods", −24.67% accuracy **[P]**. Dynamic Benchmarking, arXiv 2503.06643: "dynamic benchmarks can resist against the data contamination problem" **[P]**. RustMizan, arXiv 2607.04729: "semantics-preserving code mutants for contamination testing" **[P]**. Haroon et al., arXiv 2504.04372: SPMs cause failure in 78% of previously localised faults **[P]** | Only the instantiations: admissibility decided by **SemantEq on Lean statements**, applied to a **proof-search** benchmark, with kernel-accepted completion as endpoint. The mechanism is occupied four times over | **EXISTING** (mechanism) | On the pinned FC task set, generate SemantEq-admissible transforms. Endpoint: solve-rate drop for a model whose training cut-off precedes the tasks, versus a temporal hold-out. The difference is the memorisation score; report it with a confidence interval |

---

## Feasibility warnings (read before dispatching)

1. **N1 presupposes an elaborator.** HOL Light's kernel gives you `thm` values from tactics; it
   does not expose a partial proof term with typed holes. N1 on HOL Light requires either a
   substrate change (Lean/Coq) or building an elaborator — a project, not a task. This is the
   single largest unstated dependency in the N-list.
2. **N11 is bounded by independence, not by hardware.** In an LCF kernel each inference depends
   on earlier ones. GPU parallelism buys throughput across *independent obligations*; it does
   not accelerate one derivation. State the target as "2× per-obligation throughput over a task
   batch", not "2× replay".
3. **N3's guarantee needs exchangeability.** A temporal hold-out is not an exchangeable sample.
   The conformal bound holds on the calibration distribution; on a shifted test distribution it
   does not, which is exactly why the "fail closed if calibration degrades" clause is part of
   the mechanism and not an extra.
4. **N10 needs a difficulty metric that is invariant to irrelevant edits.** Without that control,
   "difficulty collapse" will fire on every refactor and the signal is worthless. The control is
   part of the experiment, not an appendix.

## What this register does not claim

- It does not claim N1–N12 are new. Fourteen of eighteen rows are `UNVERIFIED_COMBINATION`,
  which is a statement about the literature I could search, not about the world.
- It does not claim any of these works. No row is `DEMONSTRATED_ADVANCE`, and none can become so
  without a kernel-accepted result at a pinned version.
- Where a row rests on secondary coverage, it is marked `[2°]`; where no dedicated search was
  run, it is marked `[†]`. Those rows are the first to re-check before any external claim.

---

## Table C — XXV extraction (2026-11-27)

Evidence: `research/receipts/xxv-literature.md` and `research/receipts/xxv-search-log.md`.
This table is deliberately more conservative than the review that prompted it. Classifications
follow the discipline above; `PLAUSIBLY_NOVEL` is used only where the logged search found nothing
close, and is marked weak where the search was narrow.

### Correction of record

- **"Replay a certificate through the LCF kernel" is `EXISTING`.** Harrison's SOS work builds
  Positivstellensatz/ideal-cofactor certificates externally and replays them in HOL Light; the
  kernel checks each step. The mechanism is not new and must never be claimed as such. What may be
  unclaimed is the *application* to instruction-derived machine-code arithmetic (§ probe 1).
- **de Bruijn factor is not proof amplification.** It is text-bytes ÷ text-bytes; it has no
  certificate-operation denominator. Do not conflate them.

| Mechanism | Nearest neighbour(s), with evidence | What remains unclaimed | Class | The single experiment that settles it |
|---|---|---|---|---|
| **W1** Derive canonical-residue witnesses `q`, `r` (`x − r = q·p`, `0 ≤ r < p`) from the instruction-derived equations and replay them in HOL Light | CoqCryptoLine hand-writes the assembly model and supplies carry/flag hints, then checks CAS-produced ideal cofactors in Coq **[P]**; Fiat computes the Montgomery quotient as a compile-time constant of a hand-written algorithm **[P]**; Harrison SOS replays cofactor certificates in HOL Light **[P]** | Deriving `q`/`r` **from the lifted machine-code equations** (not from a human model) and closing the canonical-residue obligation by kernel replay | **UNVERIFIED_COMBINATION** | The negation prototype (`XXV.C` step 3): produce `q = [a>0]`, `r = q·p − a` from the instruction equations; `equals_thm`; human proof text not read |
| **W2** Premise selection by a certificate's own coefficient support (`λᵢ ≠ 0`) inside an LCF proof search | Sparse linear-combination/Farkas practice; CoqCryptoLine's "elimination of ideal generators through variable substitution" **[P]**; no support-driven selection inside an LCF search found | Selecting the *antecedents* of a kernel arithmetic close from the certificate's non-zero coefficients, so unneeded normalisations never happen | **UNVERIFIED_COMBINATION** | Instrument the close's premise set; ablate `λᵢ ≠ 0` selection vs the current set; endpoint is the close phase in absolute inferences |
| **W3** Proof amplification `A = I_expanded / N_certificate_operations` | de Bruijn factor (different units) **[P]**; proof-compression work reports nodes/bytes, not certificate ops; `all:"proof amplification"` = 357 term-colliding hits | A reported ratio whose denominator is certificate steps, beside the expanded-inference count | **PLAUSIBLY_NOVEL (weak)** | Only after a certificate backend exists; report `A` with `I_expanded`, `N_ops`, checking time and bytes together |
| **W4** Silent-acceptance mutation of a verified checker's refusal sites | PIT `TRUE_RETURNS`/`RETURN_VALS` operators **[P]**; Carcara ships `tests/regression/invalid` with the rule "cases in `invalid/` must be rejected. No case may panic." **[P]** | Applying the *operator* (refusal → plausible `Ok`) to a verified checker's refusal sites and reporting an **assertion-strength** score separate from reachability | **UNVERIFIED_COMBINATION** | `XXV.E`: run operator 2 on `guard.rs`/`freeze.rs`/`lib.rs`/`wire.rs`; a mutant survives unless a test asserts the refusal; report two scores |
| **W5** Column-wise polynomial reduction (AMulet) over machine-instruction partial products, replayed in an LCF kernel | AMulet2 column-wise over circuits/C source with Coq/Frama-C **[P]**; Fiat associational→columns in Coq **[P]**; CryptOpt asm + Coq equivalence **[P]** | The three-way composition over *machine-code* partial products with kernel replay | **UNVERIFIED_COMBINATION** | A multiplication prototype (`XXIII.F4`): recover `c_k + Σ_{i+j=k} aᵢbⱼ = r_k + B·c_{k+1}` from the instructions; replay in HOL Light |

### Negative results worth keeping

- **No prior art found** that derives modular witnesses from an implementation's arithmetic
  structure (Q1), recognises the conditional-subtract/conditional-add idiom automatically (Q6), or
  reports primitive inference counts for machine-code verification other than Fox ITP 2015 (Q7).
  Each is "not found after query X", not "does not exist"; the search log names the queries.
- **No published inference-count comparison** of `INT_ARITH_TAC` vs a hand-built linear combination
  in HOL Light exists (Q8). The Isabelle reflection ratio (1–2 orders) is the only nearby number.
