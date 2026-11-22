# TODO — Tekmerion

Status: `[ ]` todo · `[~]` in progress · `[x]` done · `[!]` blocked

Rule: an item is done when its acceptance criterion is met with a receipt, not when it looks finished.
Every item below came from an external review or from the programme's own audit. Nothing is dropped.

---

## A. Claim corrections — blocking, do before any new claim

- [x] **A1. Fix the attribution of 10.26% and 0/864.**
  The verdict attributes both to "the same abstract". A reviewer traced them to the leaderboard's
  `results.json` embedded in `index.html` and says the NeurIPS abstract gives only 6.35%.
  *Acceptance:* fetch both sources, store verbatim receipts under `research/receipts/`, quote the exact
  sentence in each, and cite the precise document. If a number appears only in the leaderboard data,
  say so. No claim cites an unspecified "abstract".

- [x] **A2. Source or delete the Grok 4.6 / five-v1.1-FC-successes line.**
  A reviewer could not find it in the leaderboard data, its README, or the paper.
  *Acceptance:* exact quote and URL in `research/receipts/`, or the sentence is deleted. If deleted, FC
  is still exactly zero, which strengthens the case.

- [x] **A3. Narrow E4's novelty claim.**
  arXiv 2410.11133 predicts a tactic's success likelihood, execution time, and the resulting proof state
  or error message; it cites Xin et al. (2024), which used successful environment responses as an
  auxiliary objective. "Learning Proof Transformations..." (2023) learns proof-state transformations for
  tactic prediction.
  *Acceptance:* E4 states the narrower hypothesis — using forecast error *online, per region*, as a trust
  and calibration signal, evaluated on an industrial machine-code corpus — and cites all three.

- [x] **A4. Withdraw "R3 for free".**
  `TacticTrace` keeps at most 20 records per tactic, prefers the shortest goals, and records no step order.
  Forecast data drawn from it is biased toward the easiest goals.
  *Acceptance:* `00-VERDICT.md`, `02-EXPERIMENTS.md` and `03-INSTRUMENT.md` all state that E4 runs on
  C1's unsampled corpus, and why.

- [x] **A5. Rewrite C2's mechanism.**
  `collect-problems.py` replaces *every* proof in the inlined file with `CHEAT_TAC`, so all preceding lemmas
  are already theorems in each task's context. "A single-shot model cannot recall library lemmas" is ruled
  out by the harness. FC answers mostly never reach the checker (33 and 42 evaluated on ARM FC, zero OK).
  *Acceptance:* C2's mechanism is step-level construction, not recall; the old explanation is removed.

- [x] **A6. Reposition C3.**
  C3 is infrastructure, not a frontier result. Its 75% figure follows from the model by definition and
  must not occupy the only "Measured" slot. E3, on real reassembled bytes, is the part that counts.
  *Acceptance:* README and verdict label C3 infrastructure; the synthetic number is labelled a model
  conformance test; E3 is named as the claim-bearing experiment.

- [x] **A7. Rewrite the negative-results section.**
  It reads defensively. Nulls are published, but the programme must be designed so the expected outcome
  is a win.
  *Acceptance:* the section is replaced with: pick arms whose prior of clearing the bar is high; exit
  losers to reallocate budget; if no arm can clear a bar, stop — and say so.

- [x] **A8. Fix the comparator table.**
  146/2,301 zero-shot and 236/2,301 within five feedback attempts are the exact comparators.
  Benchmark versions and attempt budgets must never be mixed.
  *Acceptance:* a single comparator table in `02-EXPERIMENTS.md` with version, attempt budget, and mode
  for every number.

---

## B. Revocation-engine lifecycle hardening (external code review)

- [x] **B1. Round-trip after update.**
  Done: wire v2 is an immutable node table plus head map; pins name nodes declared earlier
  and rendering emits superseded nodes too. `parse(render(s))` is tested after semantic,
  subject, attestation and re-pin updates.

- [x] **B2. Shared validated constructor.**
  Done: `Node::validate` is the single path used by `wire::parse`, `Store::insert` and
  `Store::update`; a test asserts the API refuses exactly what the parser refuses.

- [x] **B3. Separate event delta from effective status.**
  Done: added `ChangeEvent`, `AffectedByEvent` and `Store::status` over the transitive pin
  closure; `classify` treats a re-pin as semantic; admission uses `is_admissible`/`status`.

- [x] **B4. Mutation controls for lifecycle, not just parsing.**
  Done: off-by-default `fault-injection` feature with `L01`–`L05` guarded sites and
  `tools/mutate_lifecycle.py`; measured 5/5 killed next to the parser's 16/16.

---

## C. Instrument — lossless trajectories (C1)

- [~] **C1. Persist successes and failures together, unsampled.** Write-through works: a terminal failure persisted 30 journals and a recovered failure persisted 1 line before any dump. Successes are still sampled by `TacticTrace`, and no runner exists that executes proof binaries.
  Failure records are write-through, but surrounding successful transitions are still subject to
  `TacticTrace`'s sampling and end-of-proof dump. An aborted proof can leave failures without the
  transitions needed to reconstruct the trajectory.
  *Acceptance:* one stream per attempt containing ordered success and failure events, attempt identity,
  and a terminal outcome; no sampling on this stream.

- [ ] **C2. Fix E1's hypothesis.**
  A tactic exception can be caught by `TRY`/`ORELSE`; it does not mean the branch is doomed. E1 currently
  promises exact doomed-branch labels.
  *Acceptance:* distinguish *failed tactic*, *failed attempt*, and *search-exhausted branch*; E1 claims only
  the first two as labels, with the third derived separately.

- [~] **C3. One fully replayable failed trajectory, published.** Achieved in `.bench/G1/` (byte-identical replay, same `goal_before`, same sha256) but not yet packaged as a tracked, content-addressed artefact.
  *Acceptance:* a real HOL Light run produces a failed attempt whose trajectory replays to the same
  exception and goal state, archived and content-addressed.

- [ ] **C4. Guard conversions.**
  A conversion that raises currently writes no failure record (211 conversion wrappers; 288 conv trace files skipped).
  *Acceptance:* conversion path instrumented, or the omission stated in every report that uses the corpus.

- [ ] **C6. Make `trace/exptrace_failure.ml` standalone.** It calls `ExportTrace.effective_output_dir`
  and `get_sampling_config`, which do not exist at pinned TacticTrace `7ea3808`. Use an explicit
  directory (`TEKMERION_FAILURE_DIR` or a setter) so it compiles against the pin.
  *Acceptance:* the tracked module compiles against `7ea3808`; the selftest uses the tracked file, not a copy.

- [ ] **C7. Fix `research/03-INSTRUMENT.md` §9.** It documents `exptrace_dump ()`; the real function is
  `string -> unit`. *Acceptance:* the doc matches the source.

- [ ] **C8. Provide a runner that actually executes proofs.** `setup.sh` runs `make build_proofs`, which
  builds binaries but never runs them, so `.bench/trace-logs` stays empty; `build-proof.sh:88` also
  pre-creates the dump directory that `exportTrace.ml:192` then `Sys.mkdir`s.
  *Acceptance:* one command produces success traces on a named theorem, with the pre-created directory resolved.

---

## D. The functional-correctness pivot (C2)

- [ ] **D1. Step-level harness on HOL Light's goalstack.**
  Built on the benchmark's checkpoint workflow. The model fills creative parts (invariants, bound lemmas,
  case splits); a fixed skeleton drives symbolic simulation.
  *Acceptance:* a separate arm with its own budget, reported as step-level (not Pass@1) and never merged
  with the one-shot numbers. The abstract says "future step-level proof construction workflows"; nobody
  has submitted one.

- [ ] **D2. ARM↔x86 sibling transfer.**
  Prove a routine on one architecture, then adapt the kernel-accepted proof to the other. The arithmetic
  closing is often nearly identical.
  *Acceptance:* measured transfer rate; every transferred proof re-checked by HOL Light on the target
  architecture.

- [ ] **D3. Smallest-first, one architecture before the other.**
  *Acceptance:* preregister the architecture order and the size ordering; report both.

- [ ] **D4. Put DSFB on the per-step simulation channel.**
  Per step: term size, carry-chain depth, open side conditions, subgoal count. This is a real time series,
  unlike motif labels, and matches drift/slew.
  *Acceptance:* an early-cutoff policy evaluated on saved compute at fixed kernel-accepted output.

- [ ] **D5. Primary endpoint.**
  Kernel-accepted functional-correctness proofs per fixed compute budget. Forecast accuracy, motif
  precision and trajectory fidelity are diagnostics only.

---

## E. Contamination and evaluation discipline

- [ ] **E1. No retrieval over s2n-bignum proof text.** Preregistered exclusion.
- [ ] **E2. Lemma cache holds only proofs the system produced itself.**
- [ ] **E3. Ablation over which architecture is solved first.**
- [ ] **E4. Freeze the lemma bank before held-out evaluation,** or classify the experiment as online
  cross-task learning. Otherwise the result measures accumulated evaluation information.
- [ ] **E5. Memorisation probe per model; verbatim and obfuscated query variants both reported.**

---

## F. Benchmark environment gate

- [x] **F1. Build HOL Light + TacticTrace + s2n-bignum.** ARM exit 0 (6,331 s, 348 objects). x86 terminated at the 3 GB disk floor after 335/342 binaries; corpus unaffected. See `research/receipts/g1-failure-trace.md`.
- [x] **F2. `collect-problems.py`; record actual counts by category.** **2,622 confirmed** at this pin: generic 686, program_state 601, FC-arm 463, bit_vector 437, FC-x86 435. 2,301 and 2,284 are not produced by this commit.
- [x] **F3. Harness validation without a model.** Real proof → `OK` ×4; `CHEAT_TAC` → `CHEATING`; `ALL_TAC` → `FAIL`. **Correction: this revision has no `SYNTAX_ERROR` verdict**; malformed answers produce a compile error and no judge file. Use `ERROR`/pre-check failure instead.
- [x] **F4. Reproducing 6.35% and 10.26% is blocked.** No model credentials in this environment. Stated the blocker; no model was substituted.

---

## G. Literature and novelty register

- [ ] **G1. Create `research/04-NOVELTY-REGISTER.md`.** One row per mechanism: nearest neighbour(s),
  what remains, classification (`EXISTING` / `KNOWN_COMBINATION` / `UNVERIFIED_COMBINATION` /
  `PLAUSIBLY_NOVEL` / `DEMONSTRATED_ADVANCE`).
- [ ] **G2. Prior-art search** on: step-level proof construction for machine-code verification;
  ARM↔x86 proof porting and transfer; per-step residual scheduling in provers; forecast-error-based trust.
- [ ] **G3. Tighten the forecast-residual hypothesis** to: a model's prediction error about a tactic's
  effect identifies when its future recommendations are unreliable, and using that signal raises
  kernel-accepted proofs at equal compute.

---

## H. Repository hygiene

- [x] **H1. `main` and `staging` on `github.com/infinityabundance/tekmerion`; `staging` pushed continuously.**
- [x] **H2. Published `tekmerion 0.1.0` to crates.io; tag `v0.1.0`.**
- [ ] **H3. Keep `.bench/` out of git; never commit benchmark artefacts.**
- [ ] **H4. Every reported number derived from receipts by code; raw series exported.**

---

## Immediate order of work

1. A1–A4 (claims traceable to receipts; these are cheap and they protect everything else).
2. B1–B4 (make the engine survive the lifecycle it claims to model).
3. C1–C3 (lossless trajectories; one replayable failed attempt).
4. F1–F3 (environment gate as far as credentials allow).
5. D1–D4 (the pivot; the only route to beating the bar).
6. A5–A8, E, G (rewrite the plan around the pivot; keep the novelty register current).

---

# PART II — v2 programme, CERTCUT, N1–N12 (added 2026-11-22)

Supersedes the C1–C3 ordering. Keeps the corrections in PART I except where `X1`/`X2` say
otherwise. Nothing below is dropped; each item is one dispatchable unit.

## V. Frontier facts to verify and freeze (before any claim)

- [x] **V1.** Verify AlphaProof Code (DeepMind, 2026-11-10): ~6.3M Lean lines, compiled
  `libsignal`, machine-code level, stated assumptions. Store a receipt.
- [x] **V2.** Verify Aria (arXiv 2607.06341): 4,257 Iris lemmas, 318 reglang lemmas, code
  agent + kernel harness, no fixed tactic strategy.
- [x] **V3.** Verify Anthropic + Theorem (2026-10-22): 123 libraries / 239 entry points,
  RISC-V, no-crash properties only, no functional correctness, no constant time.
- [x] **V4.** Verify `s2n-bignum/doc/program_equivalence.md` and `arm/proofs/equiv.ml`
  (2,392 lines, 11 ARM proof files).
- [ ] **V5.** Resolve the corpus-count conflict: paper 2,284 vs poster 2,301 vs leaderboard
  2,622. Pin one SHA, one count, one attempt budget per claim.
- [ ] **V6.** Record that no agent entry exists on the leaderboard as of 2026-11-22.

## M. v2 mechanisms

- [ ] **M-A.** De-optimisation as a proof strategy: agent writes a simple twin; prove twin;
  discharge twin ≡ shipped with `ensures2`; compose. Metric: FC solve with vs without twin.
- [ ] **M-B.** External certified algebra into HOL Light object-code proofs: Gröbner /
  ideal-membership cofactors, in-kernel ring check, LRAT for bit-width side conditions.
- [ ] **M-C.** Executable ISA model as a cheap falsifier: run every proposed invariant on the
  machine model before the kernel. Metric: kernel calls per accepted proof, false invariants
  reaching the kernel.
- [ ] **M-D.** Cross-ISA proof transport from the system's own proved lemmas; cache holds no
  human proof text. Metric: conditional x86 solve rate vs unconditional, order ablated.
- [ ] **M-E.** DSFB on symbolic-simulation traces: per-step term size, abbreviation count,
  open side conditions; typed early-cutoff reason instead of timeout.
- [ ] **M-F.** rustc-generated P-256 objects against s2n-bignum's own specification; no new
  ISA model; contamination-free held-out set by construction.

## N. N1–N12

- [ ] **N1.** Proof-term residuals: partial proof term with typed holes is the state; residual
  is hole count, hole type complexity, unification failure pair. Compare against goal-size
  pruning at equal budget.
- [ ] **N2.** CEGIS for lemmas: SMT counterexample → generalized lemma → kernel proof; count
  only lemmas reused in another proof.
- [ ] **N3.** Conformal abstention for doomed branches: preregistered coverage; fail closed if
  calibration degrades.
- [ ] **N4.** Dense residual reward `r_t = -ΔGoalComplexity - λ·KernelCalls +
  μ·CertificateProgress`; sparse vs dense at equal budget.
- [ ] **N5.** Proof repair as semantic transport along a SemantEq-admissible diff; zero-search
  transport rate, with the theorem statement guarded.
- [ ] **N6.** Dual-logic acceptance with a translation certificate; planted translation bugs
  (binder capture, definition shadowing, implicit-argument mismatch) must be caught.
- [ ] **N7.** Slice ISA equivalence over the instructions the shipped kernel actually executes;
  bind to the slice digest; mutation controls must fire.
- [ ] **N8.** Relational leakage by transformation: prove the transformer once; leaky control
  variant must be distinguished or the build fails.
- [ ] **N9.** Cross-logic failure intersection (HOL Light vs Lean) as a diagnosis channel that
  feeds N2; precision = intersected subgoals that unblock the original proof.
- [ ] **N10.** Proof difficulty as a specification-health metric; detect weakened postconditions
  (e.g. Montgomery `out < m` dropped) by proof-difficulty collapse plus mutation sensitivity.
- [ ] **N11.** GPU-parallel certificate checking with CPU aggregation and recheck; target 2×
  replay at identical accepted theorems.
- [ ] **N12.** SemantEq-preserving transforms as a contamination probe; original-minus-transform
  gap is the memorisation score.

## C. CERTCUT (RFC execution)

- [ ] **C.1.** Adopt CERTCUT as the primary research bet: automatic cutpoints and arithmetic
  bridge lemmas, every one kernel-checked, original theorem unchanged.
- [ ] **C.2.** Gates G0 (frozen comparator) → G1 (real end-to-end tracing) → G2 (first certified
  cutpoint) → G3 (generalised lemma reused on a held-out family) → G4 (benchmark
  confrontation) → G5 (production-relevant theorem).
- [ ] **C.3.** Arms A0–A6 plus leave-one-out L-x, same per-task budget in tokens, wall clock,
  kernel calls, hardware.
- [ ] **C.4.** Splits S1 (public FC, obfuscated variant), S2 (temporal hold-out by file-add
  date), S3 (openssl-rs objects).
- [ ] **C.5.** Types: `AttemptOutcome`, `Intervention`; `KernelAccepted` constructible only
  from a verified response of the pinned HOL Light process.
- [ ] **C.6.** Modules: harness, trace, obligation, certcut, strategy, evidence, frontier.
  Do not start with fourteen crates.
- [ ] **C.7.** Reject list: new Rust kernel as first deliverable; forecast accuracy as a
  frontier claim; 6.35% vs 10.26% as a like-for-like comparison; counting near-duplicate
  proofs independently; calling a replay a security theorem; speculative constant time from a
  sequential model without a hardware contract.

## X. Conflicts to resolve (do not silently "fix" either way)

- [ ] **X1.** v2 §7 says 10.26%/0-864 come from the leaderboard, not the abstract. The receipt
  (`research/receipts/s2n-bignum-bench-abstract.md`) shows the **poster abstract** contains
  them and the arXiv paper does not. Keep the poster citation; record the difference.
- [ ] **X2.** v2 §7 says the Grok 4.6 line is absent everywhere. The same receipt shows it in
  the poster abstract. Keep it, cite the poster, and note the reviewer checked other documents.
- [ ] **X3.** Reconcile "supersedes C1–C3": C1 and C3 remain infrastructure; C2's mechanism is
  replaced by M-A/M-D/CERTCUT.
- [x] **X4.** Resolved for this pin: `timeout-map.json` = arm 1,106 + common 809 + x86 707 = 2,622; ARM+common collected = 1,915. 2,301 and 2,284 are not reproduced by this commit and must not be quoted without a commit that produces them.

## R. Residual channels and the DSFB split

- [ ] **R7.** Proof-term channel: hole structure, hole types, proof-term size, certificate
  progress.
- [ ] **R8.** Specification channel: SemantEq distance between candidate specs, proof-difficulty
  delta.
- [ ] **R9.** Cross-logic channel: failure intersection, translation diff, kernel disagreement.
- [ ] **R10.** Separate numeric residuals (subtraction meaningful: drift/slew/envelopes) from
  typed residuals (no subtraction: typed transitions, grammar, motifs). Never mix them.

## Immediate order (v2)

1. X1–X4, then V1–V4 (verify the frontier facts and store receipts).
2. F1–F3 (environment gate; assembler shim already validated).
3. G0/G1: reproduce A0, build the agent harness, land lossless tracing.
4. Pilot A1 on 40 FC tasks, smallest first; record every simulation trace.
5. M-C and M-A pilots; then M-B and M-D; then preregister the ladder.
6. M-F on one openssl-rs P-256 kernel.

---

# PART III — research findings (2026-11-22)

All claims below have receipts under `research/receipts/`; full tables in
`research/05-FRONTIER-FACTS.md` and `research/04-NOVELTY-REGISTER.md`.

## Verified, with caveats that change what we may say

- **AlphaProof Code — VERIFIED, three caveats.** ~6.3M Lean lines for compiled `libsignal` at
  machine-code level under stated assumptions, **but** the line count includes comments and
  blank lines; mutation testing is **16/17 at specification level with one witness trace each**;
  the 17 full binary disproofs are `sorry` stubs. Never cite its mutation suite as end-to-end.
- **Aria — VERIFIED.** 4,257 Iris lemmas and 318 `reglang` lemmas with a kernel-closing harness
  and no fixed strategy. It also proved 217 Rust-stdlib and 72 `iris-lean` lemmas, which the v2
  brief omitted.
- **Anthropic + Theorem — PARTIAL.** Numbers verified verbatim. "No functional correctness" is
  **contradicted for the programme**: they separately prove at x86 binary level that kernel zlib
  implements RFC 1950. "No constant time" is an absence they never claim. Assert neither.
- **s2n-bignum equivalence — VERIFIED by measurement** at `1d097f6`: `arm/proofs/equiv.ml` =
  2,392 lines (1,880 code), loaded by **11** ARM proof files. (My own grep matched 12 files
  containing the string, possibly including `equiv.ml` itself; quote 11 until disambiguated.)
  Wording fix: 2,392 is `equiv.ml`'s own size, not a total "across 11 files".

## Mechanism corrections — these change the plan

- **M-A must be re-scoped.** De-optimisation is not a new proof strategy; it is the existing
  s2n-bignum method, prescribed in `doc/program_equivalence.md` with scalar `unopt` objects
  already in-tree. Only the *agent policy* of inventing a twin is unclaimed. Make M-A an A/B on
  agent behaviour, not a mechanism claim.
- **M-F's headline is pre-empted.** AlphaProof Code already verifies stock-`rustc` AArch64 output
  at machine-code level with the compiler outside the TCB. Delete "nobody has verified rustc
  output at machine level". What survives: HOL Light, s2n-bignum's own specification, a pinned
  object digest, and a contamination-free held-out set.
- **N1 cannot run on HOL Light as written.** LCF exposes `thm` values, not a dependent proof term
  with typed holes. N1 presupposes an elaborator (Lean/Coq/Agda) or a bespoke term-level search.
  Treat as a substrate decision before any implementation.
- **N4 is CONTRADICTED as novel.** Dense verifier-grounded reward is published, and
  arXiv 2609.32908 is a clean negative control: better one-step ranking, worse kernel-completed
  solves (282.3 vs 308 of 987).
- **N12 is CONTRADICTED as novel.** Semantics-preserving mutants as contamination probes appear
  in CodeMorph, Dynamic Benchmarking, RustMizan and Haroon et al.
- **N3 is the only `PLAUSIBLY_NOVEL` row.** Conformal abstention for doomed branches. Preregister
  its caveat: coverage is marginal and exchangeability-dependent, and a temporal hold-out is not
  an exchangeable sample — which is why the fail-closed control is part of the mechanism.
- **N6 and N11 are PARTIAL** and rest on secondary sources; two arXiv queries returned HTTP 400
  and are logged as incomplete.
