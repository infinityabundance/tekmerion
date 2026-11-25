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

- [x] **C4. Guard conversions.** Implemented in `tools/patch_wrapper.py` (211 conversion wrappers). An injected `BETA_CONV` failure wrote `BETA_CONV.conv.failures.jsonl`; stock and patched success trees are byte-identical (`diff -rq` exit 0).

- [x] **C6. Make `trace/exptrace_failure.ml` standalone.** Done: no `ExportTrace.*`; directory order is explicit setter → `TEKMERION_FAILURE_DIR` → `./tekmerion-failures`. Loaded and exercised in a real HOL Light session (`hol_exit=0`), tactic and conversion journals both written and both re-raised.

- [x] **C7. Fix `research/03-INSTRUMENT.md` §9.** Corrected to `exptrace_dump : string -> unit`. Note: the `exptrace_dump ()` on line 5 is a **verbatim quotation** of TacticTrace's README, so it stays as written.

- [x] **C8. Provide a runner that actually executes proofs.** `tools/run_traces.sh` produces **266** success JSON files for `word_max` and writes journals under `TEKMERION_FAILURE_DIR`. Conformance **0 / 1,569**. TacticTrace restored clean (`git status` empty, wrapper sha `11943794…`).
- [ ] **C9. Conversion-journal retention.** Guarding all 211 conversion wrappers produced ~3.7M lines / ~578 MB for one `word_max` run. Correct but unusable at corpus scale. Add a retention policy (cap per wrapper, keep first N and last N, or record only conversions above a size threshold) without dropping the evidence that a conversion raised. *Acceptance:* one run stays under a declared byte budget with no silent loss.
- [ ] **C10. Wire the runner into `setup.sh`.** It still calls `make build_proofs` and `rm -rf trace-logs`; it never invokes `tools/run_traces.sh`. *Acceptance:* one documented command produces traces, or the README states that tracing requires the runner.

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
- [!] **F4. Reproducing 6.35% and 10.26% is blocked.** A recorded blocker is not a cleared one. No model credentials exist, so no frontier arm has run. This stays `[!]` until an arm actually executes.

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

---

# PART IV — attempt-level tracing, then the first constructed proof (added 2026-11-22)

Freeze N1–N12 and the novelty register until P1–P5 are done. Infrastructure serves the result.

## I. Instrument defects and attempt-level schema

- [ ] **I1. Attempt-level tracing.** One `attempt_id`, one monotonically ordered stream containing
  `attempt_start`, `tactic_ok`, `tactic_raised`, `conv_ok`, `conv_raised`, `attempt_end`, with an
  explicit terminal outcome (`proved` / `failed` / `timeout` / `unknown`). Keep TacticTrace's
  per-tactic JSON for conformance, but the attempt stream becomes the primary artefact.
- [ ] **I2. Bounded conversion retention.** Exact counts and a digest for every event; full terms
  only up to a declared cap per wrapper; a manifest that names what was dropped and why. Sampling
  must never be presented as a complete record.
- [ ] **I3. `--proof PATH` is advertised but not parsed** in `tools/run_traces.sh`. Implement it or
  remove it from usage and error messages.
- [ ] **I4. Conversion guard evaluation order.** `exptrace_guard_conv` takes an eager `term`; stock
  records the input after the conversion succeeds. Change it to take `unit -> term`, forced only on
  failure, and update the patcher and its golden fixture.
- [ ] **I5. JSON escaping.** `String.escaped` is not a valid JSON encoder for all control characters.
  Add a standards-compliant encoder for our own writers. **Do not change the success-record writer**
  that must stay byte-identical to TacticTrace.
- [ ] **I6. Publish a content-addressed G1 reproduction package.** Exact commands, inputs, raw output,
  hashes and assertions, small enough for Git; the large corpus stays out.

## P. From observing proofs to constructing them

- [ ] **P1. Reproduce attempt-level tracing on three routines** of increasing difficulty.
- [ ] **P2. Minimal HOL Light step-level proof controller.**
- [ ] **P3. Generate and kernel-check one CERTCUT intermediate lemma.**
- [ ] **P4. Close an original functional-correctness theorem without its reference proof.**
- [ ] **P5. Matched-budget baseline comparisons.**

Order: I1–I5 → I6 + P1 → P2 → P3 → P4 → P5.

---

# PART V — Programme v3: witness compilation (added 2026-11-22)

Supersedes the ordering in PART II/IV. Keeps their corrections. The unit of search changes from
a tactic script to a small typed **witness**; a deterministic compiler emits the script.

## Ledger corrections from external review

- [x] **F4 is `[!]`, not `[x]`.** A recorded blocker is not a cleared one.
- [ ] **I7. Re-run G1 with the committed module.** The G1 receipt was produced with a modified
  copy; `trace/exptrace_failure.ml` has since been made standalone (C6). Re-run and re-issue the
  receipt so the evidence matches the code in Git.
- [ ] **I8. Package C3 so it is rerunnable.** Full sha256 (not `e5a9f4ce…`), exact commands,
  inputs, raw output and assertions, small enough to commit; the large corpus stays out.
- [ ] **I9. Move C9 onto the critical path.** For conversions, record per-step numeric counters
  (term size, elapsed time, abbreviation count) and keep full goal journals only on failure or at
  sampled steps. The M-E channel lives exactly where the journal currently explodes.

## V3.0 — verify the measurements before citing them

- [x] **Recount FC at the bench pin.** Reproduced at `5e0fc782`, per theorem, not just by
  cardinality: **898 total (463 ARM + 435 x86)**, 898 matched against `.bench/problems.json`,
  0 only-direct, 0 only-corpus. **Corrections to v3:** straight-line is **675 / 898 = 75.17%**
  (v3 said 73%); medians are **ARM 92 / x86 77** (v3 said 91/76); loops are 223 (24.83%) and
  concentrate in the long tail (131 of 164 proofs ≥ 300 lines, 79.88%; 93.7% of 600+).
  **v3's 323 family count is not reproducible**: with name-then-file vocabulary it is 569, with
  v3's listed vocabulary 382, by distinct theorem name 315. v3's 917/473/444 is also not the
  bench rule's total at `0f792fe` (925/478/447). Cite the pin numbers, never the HEAD ones.
  Script: `tools/fcdist.py`; receipt: `research/receipts/v3-recount-5e0fc782.md`.
- [x] **Held-out guard.** `tools/heldout_guard.py`, fail-closed on `25519`, `secp256k1`, `p256k1`,
  `sm2`; resolves symlinks and real paths; refuses missing/directory/outside-root inputs. Verified:
  `secp256k1_jadd.ml` → rc 2, `p256_montjadd.ml` → rc 0.

## W — mechanisms

- [x] **W1. Simulation compiler.** Derivation implemented (`tools/derive_steps.py`: stdlib-only, no
  I/O, no-peek enforced structurally). `add` and `neg` reproduce the human post-simulation goal
  exactly (same SHA-256 and HOL MD5); `sub` matches the conclusion and closes with `HYP 0` but not
  the assumption multiset, because the human uses blanket ranges `(1--8) ∪ (10--17)` which add three
  duplicate carry relations. Gate is therefore **semantic**: identical conclusion plus closure under
  the derived list. Digitize/abbreviate and p384/p521 extension remain open.
- [~] **W2. Witness enumerator** per operation family, **no model**. Implemented for `add`, `sub`,
  `neg`; the congruence term and flag meaning are derived from the postcondition and derived steps.
  The tactic *skeleton* of the close is transcribed from the human proofs and labelled
  `DEVELOPMENT_ONLY` in code, witness and receipt.
- [ ] **W3. Executable-model falsifier** gates every witness and invariant before the kernel.
- [~] **W4. Certified arithmetic closer.** Deterministic closer emits a script the official judge
  accepts for `add`/`sub`/`neg` on P-256 ARM. No external solver was needed in this version; the
  Gröbner/cofactor path remains unexercised.
- [ ] **W5. LLM witness proposer** for what W2 misses and for loop invariants (needs credentials).
- [ ] **W6. DSFB as the L1 scheduler**: per-step term-size series; a slew triggers abbreviation or
  digitization. This is control, not early abort.
- [ ] **W7. Cross-ISA transport of witnesses**, not scripts.
- [ ] **W8. Twin / de-optimisation** as an A/B agent policy only (already re-scoped).
- [ ] **W9. rustc P-256 object against s2n-bignum's spec** (S3 held-out).

## Witness record (the only thing a proposer emits)

`accumulate`, `congruence`, `flag_meaning`, `case_split`, `bounds`, `invariant` — a typed record
of roughly 3–20 terms, not a script.

## Arms

- [ ] **Z** = W1+W2+W3+W4, **zero model calls** (this is K1′).
- [ ] **Z+6** = Z + W6 scheduler.
- [ ] **M** = Z + W5.
- [ ] **A1** = code agent writing full scripts (v2 floor).
- [ ] **M vs A1** at equal budget: do witnesses beat scripts?
- [ ] **L-x** leave-one-out over W1/W3/W4/W6.
- [ ] **B0–B6** from the second review: B0 one-shot, B1 five-attempt, B2 no-model tactic portfolio,
  B3 stateful agent, B4 +invariant generator, B5 +certificates, B6 +residual scheduling.

## Splits and contamination

- [ ] **LOCO** leave-one-curve-out (develop P-256/384/521; evaluate 25519/secp256k1/SM2).
- [ ] **LOIO** leave-one-ISA-out (develop ARM; evaluate x86; report the reverse too under an equal
  compute overlap).
- [ ] **S2** temporal: files added after the compiler is frozen.
- [ ] **S3** openssl-rs rustc objects.
- [ ] Freeze the witness grammar and compiler by digest before evaluation; freeze the self-produced
  lemma bank; no retrieval of human proof text; development examples (`word_max`, `word_clz`,
  `bignum_add`, `bignum_sub`, `bignum_copy`, `montmul_p256`) are **not** blind held-out.

## Axiom-closure caveat (must appear in every claim)

- [ ] The benchmark replaces preceding proofs with `CHEAT_TAC`; an official `OK` is valid under
  benchmark rules but is **not** an axiom-free development. Report the inherited-axiom closure
  separately, and for any production claim replay the generated proof in a complete, genuinely
  proved s2n-bignum context.

## R1 · first generated benchmark `OK` (2026-11-23)

Zero model calls. Pipeline: derive steps → witness → closer → emit → official judge.

| task | verdict |
|---|---|
| `arm.bignum_add_p256.BIGNUM_ADD_P256_CORRECT` | **OK** |
| `arm.bignum_sub_p256.BIGNUM_SUB_P256_CORRECT` | **OK** |
| `arm.bignum_neg_p256.BIGNUM_NEG_P256_CORRECT` | **OK** |

`collect-verdicts.py` → `OK,3`; `CHEATING` = 0. Fresh-process replay: 0 hypotheses, axiom count
unchanged in all three (`REPLAY_AXIOMS_UNCHANGED`). Evidence: `research/receipts/w2-verdicts.csv`,
`w2-witnesses.{md,json}`; judge files under `.bench/eval-20261123_010304/`.

**Scope:** P-256 ARM only, training curve; the close skeleton is `DEVELOPMENT_ONLY`; not held-out;
not a frontier claim. The absolute axiom counts are high because the benchmark inlines preceding
theorems as `CHEAT_TAC`; the invariant that matters is that the proof adds none.

- [~] **R2. Freeze and evaluate LOCO — dry run only.** Frozen `2026-11-23T01:28:28Z` (hashes in
  `research/receipts/r2-freeze-manifest.md`); held-out ARM result **4 accepted / 9**, `CHEATING` 0,
  0 hypotheses, no axiom growth on each accepted proof. Accepted: `add_sm2`, `sub_sm2`, `neg_sm2`,
  `neg_p256k1`. Failed: `add_p25519`, `add_p256k1`, `neg_p25519`, `sub_p25519`, `sub_p256k1`.
  **Not a clean claim, for three reasons:** (1) a pre-guard repository grep exposed digitize lines
  from seven held-out proof files, so the run is not fully blind (disclosed in
  `r2-heldout-eval.md` §8); (2) the denominator is 9, not the published 898, so 4/9 is not
  comparable to 5/898; (3) training generalisation is incomplete — P-384 `add` and the whole P-521
  family fail or emit nothing. Training: P-256 3/3, P-384 `sub`/`neg` OK, P-384 `add` FAIL,
  P-521 FAIL/FAIL/none.
- [ ] **R3. Structural isolation for held-out evaluation.** The path guard is not enough: an
  ordinary repository search still reached held-out proof text. Build a development checkout with
  held-out proof files removed or stubbed, and a search wrapper that cannot traverse them. A run is
  blind only if exposure is structurally impossible, not if it is discouraged.
- [ ] **R4. Closer coverage for the non-P-256 shapes.** P-384 `add` (wrong flag comparison), P-521
  `add` (adds 1 then conditionally subtracts 2^521 or 1), P-521 `sub` (521-bit mask-aware), and the
  special-prime folds in `p25519` (`+19`) and `p256k1` (`+4294968273`). Develop only on training
  curves.
- [ ] **R5. Clean re-run with a new freeze.** After R3 and R4: freeze again, then evaluate once in the
  isolated checkout. This, not R2, is the number that may be called K1′.
  **2026-11-23 result:** frozen at `8dc7a22` + working tree; training `OK,5 / FAIL,3`
  (P-521 neg not emitted); held-out ARM **4/9 accepted** (`add_sm2`, `sub_sm2`,
  `neg_sm2`, `neg_p256k1`), 0 CHEATING; x86 held-out (9 tasks) not evaluated
  (ARM-only pipeline). The 4/9 is **not** comparable to the published 5/898 and
  **not a clean K1′**: a pre-evaluation grep exposed held-out digitize-tactic
  lines. Receipts: `research/receipts/r2-freeze-manifest.md`,
  `r2-heldout-eval.md`, `r2-heldout-verdicts.csv`, `r2-heldout-failures.md`,
  `r2-training-verdicts.csv`. A clean claim needs a re-run by a process that has
  not seen held-out proof text.

## Claims (judgements, not measurements)

- **K1′** Arm Z beats 5/898 FC with zero model calls on LOCO/LOIO (P≈0.65).
- **K2′** M beats A1 at equal budget (P≈0.55).
- **K3′** W6 cuts check time or timeouts (P≈0.4).
- **K4′** One rustc P-256 operation proved against the spec (P≈0.4).

## Order of work (v3)

1. V3.0 recount at the pin + held-out guard; commit the counter.
2. W1 on `add`/`sub`/`neg` mod p, ARM, **training curves only**: reproduce the human simulation
   step exactly (identical post-simulation goal).
3. W4 + W2 on the same family; the closer emits a script the bench marks `OK` on training theorems.
4. W3 falsifier in front of W2.
5. Freeze, then evaluate LOCO. **This is the first claimable number.**
6. Widen family by family, then W6; W5 only once credentials exist.

## Stop doing until step 5 produces a number

- No N1–N12 implementation, no dual-kernel work, no new Rust kernel, no proof-term-hole calculus,
  no GPU replay, no speculative-constant-time model, no expansion of the novelty register.
- No synthetic evidence-lifecycle statistic presented as a frontier argument.

---

# PART VI — Programme v4: the coverage engine (added 2026-11-23)

v3 proved the thesis on straight-line P-256. v4 is about coverage: new modulus classes,
loops, and x86. Nothing here is claimable until a sealed full-set run.

## Method — the R2 split is spent

- [ ] **V4.1.** Treat the nine ARM R2 tasks as **development data**. Fixes motivated by their
  failure traces are development, not held-out evidence — even if the files are later hidden.
- [ ] **V4.2.** Fresh splits only: the nine **x86 counterparts** (never evaluated), operation
  families on untouched curves, and the openssl-rs objects.
- [ ] **V4.3. Freeze hygiene.** Push and **tag** the freeze commit before each held-out run. Run
  each split **once**, in an isolated filesystem containing only ELF objects and
  `problems.json`. Hashes recorded after the fact are not a freeze.

## Wall 1 — C-SAT: close the arithmetic with certified SAT, not templates

- [ ] **C1. Generic bridge lemma — now the measured blocker.** The residual post-simulation goal is
  stated in `num`/`real` with `MOD`, `rem`, `bignum_of_wordlist` and `&` equations; pure SAT cannot
  expand those atoms, so CaDiCaL returns SAT in 3–38 ms at every stage. C-SAT closes word-level
  tautologies only. Implement one generic, modulus-agnostic lemma per linear operation shape that
  relates the `num`/`MOD` specification to the word-level circuit statement, then let C-SAT close
  the remainder. This is the next task, not a re-tuning of the template.
- [x] **C2.** `BITBLAST_THEN` + CaDiCaL + **kernel LRAT replay** is now connected (`tools/csat_lib.ml`,
  `CSAT_TAC`). `Examples/bitblast.ml` indeed defines `BITBLAST_CADICAL` only inside a commented-out
  block. Working binary: CaDiCaL 2.1.2 emitting LRAT natively; `lrat-trim` is absent, which matters
  only if the pinned `CADICAL_PROVE` must be used verbatim.
- [x] **C3.** Measured: 64-bit carry 0.81 s, 128-bit 1.41 s, 256-bit 2.67 s, 256-bit carry 4.67 s,
  320-bit carry 5.86 s, against a 120 s timeout. Replay is **not** the bottleneck; the per-limb split
  is unnecessary.
- [ ] **C4.** Record the trust difference: the LCF kernel replays LRAT, so no compiler enters
  the trusted base (unlike `bv_decide`, which verifies by reflection).
- [ ] **C5.** Scope: linear families only (`add`, `sub`, `neg`, `double`, `half`, `optneg`,
  `triple`, small-constant `cmul`). Multiplicative families need W4's algebraic closer.

## Wall 2 — L-FIT: loop invariants from the postcondition

- [ ] **L1.** Arithmetic core by prefixing the postcondition (`lowdigits`/`highdigits`).
- [ ] **L2.** Fit live registers to affine forms in {i, n, m, pointers} on the **executable
  model**, not on hardware.
- [ ] **L3.** Frame each input's unread suffix by pointer advance.
- [ ] **L4.** Filter every candidate on the model, then check initiation, preservation and exit
  with `ENSURES_WHILE_UP_TAC`; loop body compiled by W1.
- [ ] **L5.** Recount the "282 of 548 invariants use `lowdigits`/`highdigits`" figure **on the FC
  set at the pin** before citing it (the current figure is whole-tree, HEAD, regex-based).

## Wall 3 — x86

- [ ] **X1.** Re-run W1 on the x86 object; map flags through a small ISA-semantics table
  (x86 `sbb` carry vs ARM's inverted no-borrow). Witnesses are value statements, so they
  should transport; everything else is re-derived.
- [ ] **X2.** Evaluate the nine x86 counterparts **once**, after a tagged freeze. Report x86
  conditional on ARM, with order ablated.

## Defects and protocol from review

- [x] **D1. Carry-liveness bug in `derive_steps.py`.** Fixed: only writers ground a definition;
  readers are transparent. Regression tests `ADDS; ADC; ADCS` (derives `[1,2,3]`) and
  `ADDS; CMP; ADCS` (derives `[3]`). W1 gate re-run: derived lists unchanged on P-256, so the old
  bug never fired on those routines.
- [ ] **D2. `w2_witnesses.py` invariant field is the postcondition**, not an inductive loop
  invariant. Do not advertise loop verification until L-FIT actually synthesises one.
- [ ] **D3. Modulus class as a typed witness field** (NIST/Solinas, pseudo-Mersenne,
  Montgomery-friendly), with one close path per class.
- [ ] **D4. Two held-out protocols:** LOCO *within* a class, and **leave-one-class-out**. The
  second is the one a reviewer will respect.
- [ ] **D5. Residue/quotient certificate (RQ) alternative.** For non-linear families, or where
  bit-blasting is too slow: derive `input = output + p*q` from machine equations and prove
  `0 <= output < p` separately. Untrusted algebra proposes; the kernel checks.
- [x] **D6. GitHub CI** for the generator and regression tests. `.github/workflows/ci.yml` runs
  `cargo test`, `clippy -D warnings`, `pytest`, both mutation harnesses and `selftest.sh`; all pass
  locally. Rust pinned to 1.98.0.
- [ ] **D7. Independent proof reconstruction.** Remove the `DEVELOPMENT_ONLY` skeleton
  incrementally; measure which fragments can be selected mechanically from the witness and
  residual goal rather than copied from example proofs.
- [ ] **D8. Axiom closure.** Report benchmark `OK` and full-dependency replay as **two different
  results**. The bench setup inherits thousands of `CHEAT_TAC` axioms; only the second supports
  a security claim.

## The claim v4 is built to win

- [ ] **K1″.** Frozen arm Z (W1 + W2/C-SAT + W4 + L-FIT + W3), **zero model calls**, run once on
  all **898 FC tasks** at pin `5e0fc782`, isolated, after a tagged freeze. Report OK/898,
  `CHEATING` = 0, replay results, per-task close times, and results split between
  development curves/classes and fresh held-out ones. Comparator: 5/898 (poster abstract).
- [ ] **Secondary (cheap, same run).** Generated-vs-human check time on matched theorems;
  draft `bstep` engine (PR #505) as comparator.
- [ ] **Arms:** B0 frozen 0996c32 compiler; B1 + carry-liveness fix; B2 + machine-derived
  quotient; B3 + independent range proof; B4 + counterexample branch refinement; A1 full
  tactic-script agent at equal total budget.

## Order of work (v4)

1. D1 carry-liveness fix + D6 CI.
2. C-SAT on training curves; measure LRAT replay against the timeouts.
3. C-SAT on the spent nine (development), then freeze.
4. X1 x86 W1 + the nine fresh x86 tasks — **the first clean held-out number**.
5. Widen the linear families; freeze; evaluate on fresh curves.
6. L-FIT on `bignum_add`-style loops.
7. The sealed full-set run (K1″).

Everything else stays parked until step 7 produces a number.

---

# PART VII — bridge through a reference function (added 2026-11-23)

## Release blocker

- [x] **V7.1. CI must be green.** Fixed by pinning `runs-on: ubuntu-22.04`.
  **Verified green on the real runner:** run `45125493761`, job `Offline checks (no key, no bench)`,
  conclusion `success`. The two prior runs had failed at `apt-get install ocaml-nox` before any
  check executed.

## The bridge design that keeps the modulus out of the lemmas

- [ ] **V7.2. Bridge through a reference function, not the implementation.** For `add`:
  `ref x y = let s = x + y in if s >= p then s - p else s`, with `p` a word constant. Split:
  1. **One generic lemma per operation, proved once for all `p` and widths:**
     `val x < p /\ val y < p ==> val (ref x y) = (val x + val y) MOD p`.
  2. **One SAT instance per curve:** the post-simulation output words equal `ref x y` at the
     concrete `p`, with `x < p` and `y < p` as bit-level comparator hypotheses *inside* the
     instance. All curve-specific reduction (`+19`, `2^32+977`, masks) lives here, in a purely
     bit-level problem.
  3. **`VAL_JOIN`-style lemmas** turning `bignum_of_wordlist [w0..w3]` into one word.
- [x] **V7.3. Contamination test for the design itself:** if any bridge lemma mentions a specific
  curve constant or reduction shape, class-specific templates have crept back in and the held-out
  failures will return. Enforce mechanically (grep the lemma statements for curve names).
  **Checked 2026-11-23:** `research/receipts/bridge/curve_check.py tools/bridge.ml` → 20 `prove`
  statements, 0 hits (log `research/receipts/bridge/curve_check.log`).
- [x] **V7.4. `bridge.ml` verified.** Fresh HOL Light process: **38/38 bindings `OK`**,
  18 theorems proved, load time 0.384 s, **axioms 3 → 3**. Five real conversion defects were fixed
  (GSYM direction, `dimindex` subterm, `int_mul`/`int_pow` matching, negative shifts, and
  `MATCH_MP_TAC` on a nested implication). Receipt: `research/receipts/bridge/bridge_fresh.log`.
  **Verified 2026-11-23:** all 38 top-level bindings report `BRIDGE_CHECK ... OK` in a fresh
  process; axiom count 3 → 3. Log `research/receipts/bridge/bridge_fresh.log`; harness
  `research/receipts/bridge/instrument.py`.
- [ ] **V7.5. De-hardcode `csat_bridge.ml`.** It still contains P-256 machine-code definitions and
  a hand-written simulation prefix. Acceptable to validate the bridge; the production path must be
  driven by the witness and the machine-code analysis.

## Measurement honesty

- [ ] **V7.6. Per-goal SAT times are not end-to-end proof times.** 0.807–5.858 s is per word goal;
  the receipt also records ~78 s for the probe process and ~167 s for the post-simulation
  experiment. Do not advertise 120 s timeout headroom without full attempt accounting.
- [ ] **V7.7. SATISFIABLE on a partially abstracted formula is not a counterexample.** It means the
  encoding is insufficient. After the bridge, a SAT result *is* a real counterexample on the
  bit-level problem, which makes the same machinery the cheapest falsifier for a wrong reference
  function or wrong preconditions.
- [x] **V7.8. Drop "generated proofs check faster" for linear families.** Human: 1.555 s add,
  1.174 s sub, 0.522 s neg; one 256-bit SAT close is 2.7–4.7 s. Revisit only for long simulations
  where the stepping engine dominates.
  **CORRECTION (XIV.0.1, 2026-11-24):** the 1.555 s / 1.174 s / 0.522 s figures are withdrawn
  as a comparison basis — they measured a single human `prove` call against different scopes.
  The decision to drop the "checks faster" claim stands and is now over-determined. Matched
  whole-file data (`research/receipts/wip/ix58-checktime.partial.json`, 25 of 27 rows, not a
  receipt) indicates 24–26 s human for ARM and 50–53 s for x86.

## Order

- [x] **V7.9. Prove the generic `ref` lemma for add, then P-256 add end to end.**
  **Achieved 2026-11-23.** The judge returns `OK` (23.838 s). The earlier signedness diagnosis was
  **withdrawn** after measurement: `&p` is wordified at 257 bits with the top bit clear (positive),
  and the sums at 259/260 bits. The real causes were (a) `INT_SHIFT_TO_JOIN_CONV` rejecting the
  literal operand the case split substitutes (`&2 pow 64 * &0`), leaving the flag equation opaque,
  and (b) `BRIDGE_CSAT_TAC` never discharging its goal (`MP_TAC` + `ASM_REWRITE_TAC` leaves `w`).
  See `research/receipts/bridge/REPORT.md` follow-up sections 1–5 and `confirm_v8_4.log`.
- [x] **V7.9a. Fix the width/signedness defect.** Withdrawn: there is no width/signedness defect in
  this residue. The measured replacement is the literal-operand conversion gap; fixed in
  `INT_SHIFT_TO_JOIN_CONV`, with the failing-then-passing test `tools/hol_tests/bridge_flag_wordify.ml`.
- [x] **V7.9b. Prove `ADD_REF_VAL`.** Proved for all `pw` and widths in a fresh process and added to
  `tools/bridge.ml` together with `ADD_REF_VAL_WORD`. Fresh process: 49/49 phrases `OK`, axioms 3 -> 3;
  no-curve check 28 statements, 0 hits. Receipts: `bridge_fresh.log`, `curve_check.log`.
- [x] **V7.10. Then `sub` and `neg`** with the same shape. `sub` closes through the bridge
  (94.877 s locally, 0 hypotheses) and is now judge-run: `OK`, 18.544 s, `OK,1`. `neg`
  closes after the V8.13 fix: judge `OK`, 9.204 s.
- [x] **V7.11. Then P-384 addition** — a genuinely different correction structure — as a
  development target. **Achieved 2026-11-23:** judge `OK`, 25.980 s, via
  `BRIDGE_ADD_REF_TAC`; simulation glue is `DEVELOPMENT_ONLY`. See V8.9.
- [ ] **V7.12. Then the nine x86 counterparts**, which are the first clean held-out number.
- [ ] **V7.13. Freeze and compare** on fresh tasks with a matched baseline and full dependency
  replay.

## Architecture

- [ ] **V7.14. Keep the certified HOL tactic/conversion machinery in OCaml.** Use Rust for the typed
  witness compiler, orchestration, immutable evidence and experiment accounting. Do not rewrite the
  trusted proof interface into Rust.

---

# PART VIII — CI hygiene, then finish the reference route (added 2026-11-23)

## Repository hygiene (release blocker)

- [x] **V8.1. Remove the machine-specific solver symlink and the scratch mass.**
  `research/receipts/bridge/bin/cadical` was a committed symlink to an absolute path; `mutate.py`'s
  `copytree` failed on the runner because the target does not exist there. Removed it, removed
  `judge-work/` (1.4 MB) and the 6–7 MB debug logs, kept `REPORT.md` and the decisive logs, and
  narrowed the harness to ignore `research/`, `*.log`, `.bench`, `target` with `symlinks=True`.
  *Acceptance:* mutation harness green locally (16/16 and 5/5) and a green runner run after push.
  **Verified green on the runner:** run `45153255724`, conclusion `success`. The preceding run
  `45153092405` had failed on the symlink.
- [ ] **V8.2. Commit/receipt consistency.** The bridge report says no commit was made, but the work
  was committed. Correct the report. Scratch artefacts never enter Git; receipts are `REPORT.md`
  plus decisive logs, with hashes and compact reproduction instructions.
- [x] **V8.3. 18 of 38 bindings are theorems.** Recorded: the other 20 are conversions and tactics
  that only type-check. Every report must state that distinction; type-checking is not proof of
  behaviour.

## Finish the reference route, not the IWORDIFY conversion

- [x] **V8.4. Confirm the signedness diagnosis before acting on it.** Measured and **withdrawn**:
  `&p` is wordified at 257 bits (top bit clear) and the sums at 259/260 bits, so no value reads
  negative. The alternative is identified: the split substitutes the literal 0/1 into
  `&2^k * &(bitval c)`, and the old shift-to-join conversion rejected a numeral operand, leaving the
  flag equation opaque. Adding the flag equation in wordifiable int form turns the residue from SAT
  (21.6 s) to UNSAT (29.1 s). Failing-then-passing test: `tools/hol_tests/bridge_flag_wordify.ml`
  (before: split_zero FAIL, split_one FAIL, unsplit PASS; after: all PASS).
- [x] **V8.5. Prove `ADD_REF_VAL` for all `p`** in a fresh process and put it in the loaded bridge.
  Done: `ref_add`, `MOD_SUB_ONE`, `SUB_MOD`, `MOD_SUB_REF_A1/A2`, `MOD_KEEP_REF`,
  `ADD_REF_VAL`, `ADD_REF_VAL_WORD` are in `tools/bridge.ml`; all 49 phrases `OK`, axioms 3 -> 3.
  `REWRITE_TAC[ref_add]` also needs `LET_END_DEF`.
- [x] **V8.6. Check `INT_DISTRIB_CONV`.** Not the defect: it distributes correctly. The multiplier
  came from `INT_SHIFT_TO_JOIN_CONV` refusing a *numeral* operand after the case split. Fixed; the
  literal products now become joins. Per-branch times are in `run_add_bridge3.log`.
- [x] **V8.7. Budget the close against the per-task timeout.** Measured end to end: **23.838 s** in
  the official judge (native-compiled answer); interpreted local run 137.344 s over four SAT calls
  (30.804 / 30.008 / 34.967 / 41.266). The category timeout is 7200 s, so the split route fits.
  A one-instance reference route remains the clean long-term fix.
- [x] **V8.8. P-256 add end to end through C-SAT**, then `sub`, then `neg`. Add: judge `OK`
  (`collect-verdicts.py` -> `OK,1`, `.judge.txt` in `.bench/eval-20261123_114817/`). `sub`: local
  close, 0 hypotheses, 94.877 s (not judge-run). `neg`: measured negative, SAT at 24.728 s.
  *Superseded by V8.13–V8.15: all three are judge-`OK` with the final `tools/bridge.ml`.*

- [ ] **V8.16. Judge runs depend on `cadical` being on `PATH`.** The working binary on this
  machine is `/home/one/.elan/toolchains/leanprover--lean4---v4.29.1/bin/cadical`; it is not
  in the repository and not on the non-interactive `PATH`. Every `OK` recorded above was
  produced with that directory prepended to `PATH`. `CSAT_REFUTE_GEN` shells out to `cadical`,
  so a judge run without it fails with exit 127, not with a proof failure. This belongs in the
  environment gate (section F), not in the proof accounting.
- [x] **V8.9. P-384 addition** as the first coverage improvement the old generator cannot reach.
  **Judge `OK` 2026-11-23, 25.980 s**,
  `.bench/eval-20261123_135102/arm.bignum_add_p384.BIGNUM_ADD_P384_CORRECT.judge.txt`,
  `collect-verdicts.py` -> `OK,1`; axioms unchanged, no hypotheses. The close is
  `BRIDGE_ADD_REF_TAC` (curve-generic: 6 limbs, 384-bit operands) over the residue.
  The simulation glue (carry subgoal, `FLAG_FROM_CARRY_LE` comparison, the `X3` mask
  rewrite) is **`DEVELOPMENT_ONLY`**, transcribed from `arm/proofs/bignum_add_p384.ml`.
  Two atoms initially came back unwordifiable, both containing `bignum_of_wordlist`
  (the transcribed carry subgoal and the substituted comparison condition); expanding
  that in the *whole context* (not just the goal) fixed the encoding. Measured: local
  close 145.596 s; judge 25.980 s. This is a *reduction* structure the P-256 route does
  not have (compare-and-mask, not carry-select), so it is real extra coverage.
- [x] **V8.19 (D9). Derive the P-384 simulation facts instead of transcribing them.** The carry
  telescope, the compare-flag meaning and the CSETM mask are now proved from the accumulator
  state by `BRIDGE_CARRY_CONSERVE_TAC` / `BRIDGE_CMP_FLAG_TAC` / `BRIDGE_MASK_SELECT_TAC`
  (`tools/bridge.ml` §9; generic lemmas, 0 curve constants). Judge `OK` again:
  `.bench/eval-20261123_214053/arm.bignum_add_p384.BIGNUM_ADD_P384_CORRECT.judge.txt`,
  `collect-verdicts.py` -> `OK,1`; replay `closed=1 hyps=0`, axiom list unchanged; 61.225 s
  in `bench_run`, 100 s wall (native). Receipt `research/receipts/d9-derived-flags.md`; removed
  glue `research/receipts/d9-removed-glue.diff`. Remaining `DEVELOPMENT_ONLY`: simulation
  grouping, expansion idiom, post bitval rewrites, two-bit CSETM condition shape.

## Scope

- [ ] **V8.10. Do not expand into loops or x86 until P-384 addition passes.** Completing the
  existing bridge is worth more than a new subsystem.
- [ ] **V8.11. Keep CI green before calling anything releasable.**

---

## R3 · first coverage increase (2026-11-23)

All four judge verdicts printed by `collect-verdicts.py`, all with the final `tools/bridge.ml`
(sha256 `3a32e6b0…15d52f`), all 0 hypotheses, axioms 3→3:

| task | route | end-to-end | verdict |
|---|---|---|---|
| `arm.bignum_add_p256` | `BRIDGE_ADD_REF_TAC` | 22.835 s | **OK** |
| `arm.bignum_sub_p256` | `BRIDGE_SUB_TAC` | 18.544 s | **OK** |
| `arm.bignum_neg_p256` | `BRIDGE_NEG_TAC` | 9.204 s | **OK** |
| `arm.bignum_add_p384` | `BRIDGE_ADD_REF_TAC` | 25.980 s | **OK** |

`bridge.ml` fresh load: 60/60 bindings, axioms 3→3, 2.464 s. Curve-constant check: 32 statements,
0 hits. Regression `bridge_flag_wordify.ml`: 3/3.

**Why P-384 matters:** the old template generator failed P-384 addition. It now closes through the
same curve-generic reference tactic, with no P-384 constant in any bridge lemma. That is the first
theorem the previous generator could not produce.

**Scope:** training curves only; the simulation glue and the P-384 carry subgoal are
`DEVELOPMENT_ONLY`; benchmark `OK` is under `CHEAT_TAC` context, not an axiom-free development; not
yet a held-out result.

- [ ] **V8.17.** `BRIDGE_SPLIT_CONDITIONS_TAC` is lossy (rewrites the branch condition to `true`)
  while its docstring claims it keeps the condition. Fix the tactic or the docstring before reuse.
- [x] **V8.18. Write `ref_sub` / `ref_neg`, and route every `add`/`sub`/`neg` task through the
  reference.** Done 2026-11-23. Section 8 of `tools/bridge.ml` defines `ref_sub` (subtract, add
  `pw` back on borrow), `ref_neg` (`0` at zero, else `pw - x`), nine generic lemmas
  (`SUB_REF_VAL`, `SUB_REF_VAL_WORD`, `NEG_REF_VAL`, `NEG_REF_VAL_WORD`, `SUB_INT_REM_MOD`,
  `MOD_ADD_SELF`, `SUB_REARRANGE`, `REF_SUB_KEEP`, `REF_SUB_BORROW`) and the two tactics
  `BRIDGE_SUB_REF_TAC` / `BRIDGE_NEG_REF_TAC`. Fresh process: `ref_check.log` (statements + axioms
  3 -> 3); whole file 81/81 bindings `OK`, axioms 3 -> 3; no-curve check `statements=45 hits=0`.
  Declared 18 re-run unchanged: **16 OK / 18** (was 10/18). `sub` 6/6, `neg` 6/6, `add` 4/6.
  `neg_p521` — which previously emitted no answer — is `OK, 19.527 s` with no carry-chain
  assumption, because a routine with no add/sub chain is simulated with plain `ARM_STEPS_TAC`.
  **Lost `add_p384`** relative to R3: its earlier `OK` came from a transcribed real-arithmetic
  flag subgoal, which the reference route does not carry. Receipt:
  `research/receipts/r4-reference-route.md`. Scope: development, all 18 were visible.
- [x] **V8.22. Encode flag-only `ADCS`/`SBCS XZR` instructions.** **Closed 2026-11-24**
  (`research/receipts/r5-xzr.md`). The generic admission already existed at HEAD
  (`_is_flag_only` + `_carry_out_feeds_chain`); the R4 answers predated it. Re-running the full
  18 ARM tasks in one revision gave **OK 18/18**, `add_p256k1` 22.061 s and `add_p384` 50.168 s.
  No transcribed real-arithmetic flag subgoal was added; `curve_check` stays clean. The description
  below is the historical diagnosis. *Original:* Both select the correction with the **Z flag of a
  chain of flag-only instructions** (`CMN`/`CMP` + `SBCS XZR`/`ADCS XZR`, then `CSEL`/`CSETM NE`).
  `derive_steps` excludes `XZR` destinations and `ACCUMULATE_ARITH_TAC` needs a register read
  equation, so that flag is a free boolean and `cadical` returns exit 10 (SATISFIABLE) on a
  residual instance that is otherwise the pure `output = ref_add` word equation. Needs an
  accumulator relation per flag-only instruction (a `word_carry`-style hypothesis), still
  bit-level and curve-free. Do not "fix" it by reinstating a transcribed real-arithmetic flag
  subgoal.
- [x] **V8.23. `add_p521` headroom.** Measured 2026-11-24: **96.882 s of 120 s (0.81)** in the
  single-revision 18-task run (an earlier run recorded 95.864 s; treat ~96 s as the working
  figure and re-measure on any slower host before a full-898 run). `sub_p521` is 35.216 s. Cheap
  headroom would be a shorter bit-blast for the 576-bit chains, not a longer timeout.
- [x] **V8.19. Freeze and run the whole ARM `add`/`sub`/`neg` family.** Frozen at
  `2026-11-23T13:57:05Z`, base `672cc0b`, ten file hashes matched before and after (combined
  `a6cc2a4d…e2ba9f`). Declared set: 18 tasks = 6 curves × {add, sub, neg}. **Result: 10 OK / 18
  declared** (17 judged; `neg_p521` emitted no answer). Per curve: P-256 3/3, P-384 3/3, SM2 3/3,
  secp256k1 1/3, P-25519 0/3, P-521 0/3. Max per-task 25.656 s vs 120 s; no timeouts. All accepted:
  0 hypotheses, axiom delta 0. **Scope: coverage run, not blind** — all 18 were development-visible;
  the denominator is 18, not 898 or 9. Receipt: `research/receipts/r3-coverage-run.md`.
- The seven failures are all the same class: `BOUNDER_TAC[]` on a derived flag subgoal for curves
  that fold a special-prime correction (`+19`, `2^32+977`, P-521 masking), plus `neg_p25519` failing
  in `REAL_INTEGER_TAC` on the `2^255-19` numeral. This is Wall 1, now measured on a complete family.
- [ ] **V8.20. x86 W1 + the nine x86 counterparts** — the first genuinely fresh held-out number.
- [ ] **V8.21. `cadical` on `PATH` is an environment dependency.** `CSAT_REFUTE_GEN` fails with exit
  127, not a proof failure, when the binary is absent. Pin or document it, and make the failure
  message say so.

## R4 · reference route across the family (2026-11-23)

Same 18 declared tasks, same set (`diff` empty), re-run after `ref_sub`/`ref_neg`:

| curve | add | sub | neg | OK/N | R3 |
|---|---|---|---|---:|---|
| P-25519 | OK | OK | OK | **3/3** | 0/3 |
| P-256 | OK | OK | OK | **3/3** | 3/3 |
| secp256k1 | FAIL | OK | OK | **2/3** | 1/3 |
| P-384 | FAIL | OK | OK | **2/3** | 3/3 |
| P-521 | OK | OK | OK | **3/3** | 0/3 |
| SM2 | OK | OK | OK | **3/3** | 3/3 |
| **total** | **4/6** | **6/6** | **6/6** | **16/18** | 10/18 |

`collect-verdicts.py` → `OK,16 / FAIL,2`; 16 accepted with 0 hypotheses and unchanged axioms; max
95.864 s of the 120 s budget. `neg_p521` is fixed: it is a pure complement with no add/sub chain, so
it needs no accumulating step at all — the old no-answer was generator coverage, not mathematics.

**Measured loss, recorded rather than hidden:** `add_p384` was `OK` in R3 only because the split route
used a **transcribed** real-arithmetic flag subgoal. The pure reference route drops it and now returns
SAT. The flag-only chains (`CMN`/`CMP` + `SBCS/ADCS XZR` + `CSEL/CSETM NE`) select the correction with
the Z flag, and `derive_steps` excludes `XZR` destinations, so that flag is unconstrained.

- [x] **V8.18** closed here: `ref_sub`/`ref_neg` proved for all `p`/widths, both reference tactics in
  use, fresh load axioms 3→3, curve check 0 hits.

## New defects from the 2026-11-23 run

- [x] **V8.12. `BRIDGE_CSAT_TAC` never discharged its goal.** `MP_TAC th THEN ASM_REWRITE_TAC[]`
  removes the antecedents but leaves the conclusion, so four successful SAT refutations left four
  goals. Fixed by chaining `MP` with the assumption theorems and `ACCEPT_TAC` the result.
- [x] **V8.13. `neg` still fails.** `BRIDGE_NEG_TAC` reached a residue CSAT returned SAT
  on (24.728 s). **Diagnosed and fixed 2026-11-23.** The atom dump (V8.4 style) showed
  exactly one unwordifiable atom per branch, the goal itself: `NEG_MOD_NZ` produces
  `&(p - n)` with `&` applied to a *num* subtraction, and `ASM_SIMP_TAC[INT_OF_NUM_SUB]`
  rewrites the *wrong direction* (`&n - &m = &(n - m)`), so nothing changed. `BITBLAST`
  left the atom opaque and the equation came back SAT. Fix: `GSYM INT_OF_NUM_SUB`
  (`&(p - n)` -> `&p - &n`, with `n <= p` the ambient bound). After the fix all 8 atoms
  wordify; local close 59.871 s over 4 SAT calls (13.373 / 13.226 / 15.018 / 18.143),
  0 hypotheses, axioms 3 -> 3. Judge: `OK`, 9.204 s,
  `.bench/eval-20261123_132922/arm.bignum_neg_p256.BIGNUM_NEG_P256_CORRECT.judge.txt`,
  `collect-verdicts.py` -> `OK,1`. Second defect found en route: `BRIDGE_SPLIT_CONDITIONS_TAC`
  rewrites the branch condition to `true` (it reads `|- c` as the rule `c -> T`), so the
  branch fact is lost from the assumptions. Harmless for add/sub (their lemmas restate it
  arithmetically) and fatal for a route that carries the condition in the goal; new
  `BRIDGE_SPLIT_CONDITIONS_KEEP_TAC` / `BRIDGE_FINISH_KEEP_TAC` retain it.
- [x] **V8.14. `sub` through the judge.** Judge: `OK`, **18.544 s**,
  `.bench/eval-20261123_133450/arm.bignum_sub_p256.BIGNUM_SUB_P256_CORRECT.judge.txt`,
  `collect-verdicts.py` -> `OK,1`; axioms unchanged (judge reports `CHEATING` otherwise),
  no hypotheses. `sub_middle` is `DEVELOPMENT_ONLY` (transcribed).
- [x] **V8.15. Wire the reference route.** `BRIDGE_ADD_REF_TAC` built from
  `ADD_REF_VAL_WORD`: the goal `(bx + by) MOD p = W` is rewritten (via the lemma, with
  `val x < p` / `val y < p` discharged from the ambient bounds) to one word equation
  `val(ref_add (word p) x y) = val w`, where `x`, `y`, `w` are the input/output words
  recovered from their digit sums by `BRIDGE_WORD_OF_DIGITS` (a generic flat-sum ->
  nested `word_join` fold; width and limb count read off the terms). The judge answer now
  drives this route. Judge: `OK`, **22.835 s** (split route: 23.838 s);
  `.bench/eval-20261123_133342/arm.bignum_add_p256.BIGNUM_ADD_P256_CORRECT.judge.txt`,
  `collect-verdicts.py` -> `OK,1`. Same interpreted harness: 130.651 s vs 137.344 s.

---

# PART IX — Tekmerion Native Assurance / TNPS (added 2026-11-23)

The project's centre of gravity moves from "automation inside HOL Light" to an independent,
Rust-native verification platform whose native result does **not** depend on HOL Light's runtime,
tactics, search or kernel. HOL Light, OpenTheory, Dedukti, Lean and Rocq become interoperability
targets and independent observers.

## A. Non-negotiable trust boundary

- [ ] **IX.1.** The native verifier declares its own verdict without HOL Light present. HOL Light
  may independently confirm; it may not confer native truth.
- [ ] **IX.2.** No GPU kernel, SAT/SMT/CAS engine, AI model, optimizer, distributed worker or
  adapter can extend the trusted calculus. They propose; the checker accepts.
- [ ] **IX.3.** Exactly one public top grade, **Tekmerion Sealed**, requiring: native check + complete
  approved dependency closure + independent recheck + provenance + applicable policy. Lower internal
  states (`Candidate`, `Native checked`, `Rejected/unresolved`) never carry the same label.
- [ ] **IX.4.** Rigor is the strength of the proven guarantee and the completeness of assumptions,
  not the number of tests or the difficulty of acceptance. Both strong logic and implementation
  resilience are required.

## B. Standards and precedents to study and align with

DO-178C / DO-333 / DO-330; NASA-STD-8739.8B and the 2026-11-05 draft 8739.8C (evolving, not
adopted); ECSS-E-ST-40C Rev.1 and ECSS-Q-ST-80C Rev.2 (2025-04-30); Common Criteria EAL7;
ARP4754B (systems context, not certification); Candle/CakeML; CakeLPR; CakePB; seL4; CompCert;
OpenTheory; Dedukti; SLSA; in-toto; Sigstore; NIST SP 800-193; NIST SSDF; GSN community standard;
NVIDIA CUDA memory-error management and Compute Sanitizer; GPUVerify; Kontroli; verified
compositional proof checking; online LRAT checking.

- [ ] **IX.5.** Write `research/06-TNPS-CHARTER.md` mapping each authority to the requirement it
  supplies, with primary-source receipts. Distinguish normative standards from evolving drafts.

## C. TNPS contracts (four separately versioned documents)

- [ ] **IX.6. Native logic**: types, terms, inference rules, definitions, admitted foundations.
- [ ] **IX.7. Execution semantics**: instructions, memory, control flow, concurrency, arithmetic,
  faults — exactly which program behaviour a proof establishes.
- [ ] **IX.8. Proof certificates**: inference DAGs, local witnesses, premise references, exports.
  Canonical encoding, hash domains, no duplicate-field ambiguity, no implicit coercion, no
  executable payload, no hidden ambient state.
- [ ] **IX.9. Assurance manifests**: artifact/semantics/checker digests, dependencies, provenance,
  independent results, policy version, complete assumption list.
- [ ] **IX.10.** Formal soundness target: `Check_TNPS(E, C, P) = Accept ==> E |= P`. Implementing the
  checker and proving that implication are separate tasks.

## D. The thirteen acceptance laws (normative MUST)

- [ ] **IX.11.** No unchecked inference. **IX.12.** Complete dependency closure (placeholders,
  `CHEAT_TAC`-style substitution and opaque external assertions invalidate full assurance).
- [ ] **IX.13.** Exact theorem identity (typed proposition, assumptions, quantifiers, side
  conditions, scope). **IX.14.** Exact program identity (bytes + ISA + environment digest).
- [ ] **IX.15.** Verified semantic transformations (lowering, bit-blasting, rewrites, abstraction).
- [ ] **IX.16.** Formally established checker soundness; long-term target includes parser and
  executable-level refinement.
- [ ] **IX.17.** Independent acceptance: a substantively independent implementation, not two runs of
  the same library.
- [ ] **IX.18.** Deterministic verdicts across scalar/SIMD/AVX2/AVX-512/multicore/CUDA.
- [ ] **IX.19.** Fail closed on malformed input, missing dependencies, unsupported semantics,
  overflow, resource exhaustion, device error, checker disagreement.
- [ ] **IX.20.** Adversarial resistance: differential, property-based, malformed-input fuzzing,
  mutation testing, negative-proof suites.
- [ ] **IX.21.** Reproducible evidence: checker binary, build provenance, policy, tool versions,
  dependency identities, offline reproduction.
- [ ] **IX.22.** Explicit residual assumptions: hardware, faults, OS, privilege, environment,
  foundations. **IX.23.** Independent assurance governance for trusted-rule changes.

## E. Acceleration (evidence, never authority)

- [ ] **IX.24.** Mandatory portable scalar reference checker; every accelerated path must agree
  with it (equivalence tests, not assertions).
- [ ] **IX.25.** Portable SIMD, then AVX2, then AVX-512, with runtime feature detection **including
  OS-enabled register state**, and scalar-agreement tests.
- [ ] **IX.26.** Blocked multicore replay over the certificate DAG: work-stealing on independent
  nodes, deterministic canonical IDs, immutable checked-theorem handles, checked joins; a worker
  failure or disagreement leaves the proof unaccepted.
- [ ] **IX.27.** CUDA in two modes: (a) GPU computes/elaborates, CPU independently confirms every
  inference; (b) verified accelerated checking only after implementation and translation evidence.
  GPU results are untrusted inputs. Report end-to-end accepted-proof throughput, never kernel-only
  timings.
- [ ] **IX.28.** Fault-injection campaign: corrupt certificate blocks, dependencies, cache entries,
  checker memory, execution results, distributed messages. Measure refuse/recover/improper-accept.

## F. Adapters are observations, not truth

- [ ] **IX.29.** OpenTheory article and HOL Light proof are **export targets**; a foreign acceptance
  is an immutable observation record, never a native theorem.
- [ ] **IX.30.** Translation must preserve meaning: explicit mappings for types, constants,
  definitions, assumptions, rules; verified translator or target-side reconstruction.
- [ ] **IX.31.** Unsupported semantics means refusal, never silent simplification or weakening.
- [ ] **IX.32.** Dedukti and OpenTheory studied as interoperability precedents; no universal
  interoperability is assumed.

## G. Adversarial matrix and assurance case

- [ ] **IX.33.** Implement the attack matrix as conformance tests: hidden invalid inference,
  undeclared assumption, weakened spec, mutated bytes, wrong signedness/width, malformed certificate,
  checker integer/memory error, scheduler inconsistency, GPU fault, tampered binary, compromised
  build, valid proof of the wrong specification.
- [ ] **IX.34.** Machine-readable assurance cases linked to certificates, hazards and source
  artifacts; dependency change triggers impact analysis and revocation of affected claims.
- [ ] **IX.35.** State plainly that a proof cannot establish that a specification reflects the true
  mission requirements; that is a separate assurance activity.

## H. Supply chain and preservation

- [ ] **IX.36.** SLSA-style provenance + in-toto-style signed steps + Sigstore identities; bind the
  **exact checking program**, not a self-reported version.
- [ ] **IX.37.** Reproducible builds with independent rebuild comparison before critical releases.
- [ ] **IX.38.** Offline verification with no server or vendor dependency.
- [ ] **IX.39.** Stable public grammar, canonical serialization, versioned rules, reference test
  vectors, migration requirements. Test: can an independent institution reconstruct the theorem,
  dependencies, semantics and verdict decades later without the original implementation?

## I. Frontier register

- [ ] **IX.40.** `research/07-ASSURANCE-FRONTIER.md`: per dimension, the strongest known guarantee
  and the evidence needed to match or exceed it — kernel soundness (Candle), certificate rigor
  (CakeLPR/CakePB), binary correctness (seL4), compiler semantics (CompCert), parallel checking
  (Kontroli, verified compositional checking, online LRAT), interoperability (OpenTheory/Dedukti),
  lifecycle (DO-333/NASA/ECSS), supply chain (SLSA), fault resilience, autonomous coverage.
- [ ] **IX.41.** Never claim logical priority over all possible checkers; claim audited dominance on
  named dimensions with named evidence.

## J. Milestones (in order)

- [x] **IX.42. Milestone A — native contract + scalar checker.** Implemented under `src/native/`:
  typed word logic with derived content identity, deterministic Tseitin bit-blasting, a strict
  RUP-only LRAT parser/checker, a canonical DAG certificate (`refl`, `sym`, `trans`, `congr`,
  `bitblast`), and `check -> Result<Theorem>` where `Theorem` has no public constructor.
  **Verified natively with no HOL Light, no OCaml and no process spawning at check time:**
  8-bit `x + y = y + x` (CNF 4,247 B, LRAT 9,146 B, cert 27,209 B, check **121 µs**) and 64-bit
  `(x + y) - y = x` (CNF 42,928 B, LRAT 72,822 B, cert 231,932 B, check **1.21 ms**), both in the
  release profile. **19 tamper cases rejected with named errors** (hints, deletions, duplicates,
  truncation, trailing bytes, cycles, unknown rules, dangling refs, altered CNF, transplanted proof,
  out-of-range literals, `-0`, ill-typed terms). `cargo test --offline`: 26 native tests pass.
  Trusted base: SHA-256 for identity, the Rust compiler/runtime, the term constructors, the
  bit-blaster encoding, the RUP checker and five rules. CaDiCaL is **not** trusted; it only produced
  fixtures offline. Receipts: `research/receipts/ix42-native-*`.
  **Not done:** the bit-blaster and checker are tested, not formally verified (Milestone B); RAT is
  refused; the logic is add/sub/equality only — no multiply, shifts, memory or ISA semantics.
- [~] **IX.43. Milestone B — specification, oracle, and the measured gap.** Delivered:
  `research/08-TNPS-SEMANTICS.md` (EBNF for terms/CNF/LRAT/certificates, typing, `eval`, CNF and
  RUP/LRAT relations, DAG well-formedness, acceptance relation, soundness theorem with a per-rule
  argument, assumptions A1–A7, gaps G1–G4); an independent test-only oracle (`src/native/reference.rs`,
  no hints, own parsers, 20-bit search budget); and `research/receipts/ix43-tcb.md`.
  **Differential:** 10 generated certificates agreed; 19 mutations, 18 rejected by both and 1 benign
  reordering accepted by both; 500 term round-trips; 2,000 fuzz inputs and 1,000 bit-flip mutations,
  no panic. `cargo test --offline`: **79 passed**. Trusted base **2,681 lines** across six native
  modules plus the `Digest`/`Canon` core; only dependency `sha2 0.10.9`; no call path from `check` to
  the oracle.
  **Still unproved, and stated as such: G1** bit-blaster equivalence to term semantics, **G2** parser
  refinement, **G3** executable-level refinement, **G4** machine-checked RUP/LRAT lemmas.
  **Metric: 0/4 gaps discharged.** Milestone B is explicitly incomplete; do not describe the checker
  as verified.
  **Progress (2026-11-23, staging, uncommitted):** spec `research/08-TNPS-SEMANTICS.md`,
  test-only oracle `src/native/reference.rs`, differential/conformance suite `tests/native_semantics.rs`,
  and TCB `research/receipts/ix43-tcb.md`. Still open: **0/4 gaps discharged** (bit-blast, parser
  refinement, executable refinement, paper-only lemmas) and independent agreement at **3/64** widths.
  Do not mark done; the metric is in the TCB.
  **2026-11-24:** G4 discharged for the abstract logic (Lean 4, `verification/tnps/Tnps.lean`);
  metric now **1/4 gaps, 3/64 widths**. G1–G3 remain open; the Rust checker is still not
  verified. Parser spec added as §1.7; quantified gap table added as `ix43-tcb.md` §9.
- [ ] **IX.44. Milestone C — accelerated fabric.** Blocked multicore first if it pays, then SIMD,
  AVX2, AVX-512, CUDA; equivalence, fault injection, deterministic verdicts at every step; no relaxed
  rule and no new unverified dependency.
- [ ] **IX.45. Milestone D — evaluated standard.** Native binary-level verification on ARM and x86;
  checked HOL Light/OpenTheory export; adversarial suites; assurance manifests; independently
  replayable public benchmarks; external implementations able to verify TNPS artifacts.
- [ ] **IX.46. Decisive independence test.** A P-384 theorem checked natively on a machine with no
  HOL Light, the same artifact then independently accepted by HOL Light, neither route using
  transcribed intermediate steps, and a mutation rejected by the appropriate checker.

## K. Current gaps this charter exposes

- [ ] **IX.47.** Existing benchmark `OK`s use inherited `CHEAT_TAC` context; they are benchmark
  acceptances, not dependency-closed results. TNPS's top grade must not inherit that ambiguity.
- [ ] **IX.48.** The P-384 answer still contains transcribed simulation glue, a carry equation, a
  comparison lemma and a mask rewrite (`DEVELOPMENT_ONLY`). Automating those is a prerequisite for a
  native, non-transcribed result. *Partially addressed in D9/V8.19: the carry telescope, the
  comparison flag and the mask are now derived by `BRIDGE_CARRY_CONSERVE_TAC` /
  `BRIDGE_CMP_FLAG_TAC` / `BRIDGE_MASK_SELECT_TAC`; what remains transcribed is the simulation
  grouping and the expansion/bitval idiom (`research/receipts/d9-derived-flags.md` §6).*
- [ ] **IX.49.** The Rust engine is an evidence/lifecycle layer, not yet a proof kernel. Treating it
  as one is a real construction task, not a rename.

## Non-claims

- This charter is not aviation, space or security certification, and adopting it certifies nothing.
- No dimension of the frontier register is claimed as surpassed until the named evidence exists.

---

## R5 · P-384 glue derived, not transcribed (2026-11-23)

The three hand-written facts are replaced by generic, kernel-checked derivations, and the judge still
returns `OK`:

| former transcription | replacement |
|---|---|
| `2 EXP 384 * bitval carry_s12 + bignum_of_wordlist […] = m + n` | `BRIDGE_CARRY_CONSERVE_TAC`, folding per-limb carries with `CARRY_CHAIN_ADD_STEP`/`_BASE`; list-generic and reused unchanged on P-256 |
| `FLAG_FROM_CARRY_LE` with a hand-chosen `EXISTS_TAC` | `BRIDGE_CMP_FLAG_TAC`, folding the compare chain with `CARRY_CHAIN_SUB_STEP`/`_BASE` and `SUB_CHAIN_FLAG`/`FLAG_LE` |
| `word_neg(word(bitval(p_384 <= m+n)))` rewrite | `BRIDGE_MASK_SELECT_TAC` from the CSETM assumption with `CSETM_EQ` |

Diff `research/receipts/d9-removed-glue.diff`: **41 lines removed, 4 added**. In the generated answer,
the three transcriptions are absent; `FLAG_FROM_CARRY_LE` appears only as the **definition** of the
lemma at line 16890, not as a driver use. Fresh bridge load: 21 new lemmas, axioms 3→3. Replay:
`closed=1 hyps=0 axioms 2445→2445`; judge `OK`, 61.225 s.

**Still `DEVELOPMENT_ONLY`:** the simulation grouping (`1--13 / 14--22 / [23;24] / 25--38`), the
bignum expansion pass, the post-`REAL_BITVAL_NOT` rewrites, and the fact that the mask tactic
recognises the CSETM two-bit shape without decoding the instruction. The mask is stated over
`carry_s12 \/ (p_384 <= …)`; the link to `m + n` is inside the certified SAT refutation, not a
standalone lemma. `BRIDGE_CMP_FLAG_TAC` fails closed on other compare shapes.

## R6 · x86 development before the sealed run

- [x] **IX.50. x86 simulation + flag mapping.** Implemented: ELF arch detection and an Intel-syntax
  x86 disassembly path (`ENDBR64` handling, entry renumbering from RIP), `X86_SEMANTICS` with an
  explicit borrow-vs-carry mapping (x86 `SUB/SBB/CMP` CF is a borrow; AArch64 `SUBS` C is the
  no-borrow inverse), `X86_ACCSTEPS_TAC`/`X86_STEPS_TAC` generation, and four generic join/zero-test
  lemmas. **Declared development set: x86 `add`/`sub`/`neg` on P-384 and P-521 — 6/6 judge `OK`**
  (36.0 / 62.8 / 19.6 / 36.2 / 6.0 / 7.0 s), replay `hyps=0`, axioms unchanged (2462→2462).
  Two measured blockers found and fixed: a free `NEG` carry, and a zero-test encoding BITBLAST could
  not see (`word_join a b = 0 ==> word_or a b = 0`), now a generic lemma. Sealed nine untouched
  (19 guard refusals, no answer generated). Recorded model: x86 total-memory `x86state`, no MMU, no
  alignment requirement; loaded-entry RIP offset; epilogue not simulated. **Not verified — recorded.**
  **Done 2026-11-24, `staging`, uncommitted.** Declared development set: `add`/`sub`/`neg` on x86
  P-384 and P-521 (six tasks). Judge `eval-20261124_000503`: **OK, 6/6**, 5.997–62.790 s; replay
  `closed=1 hyps=0 axioms 2462→2462`. Two measured encoding defects fixed: the `NEG` carry
  (`WORD_NEG_EQ_0`/`WORD_BITVAL_EQ_0`) and the OR-vs-joined-word zero test (new generic
  `WORD_JOIN_EQ_0` + `BRIDGE_NEG_REF_ZERO_TAC`). Receipt `research/receipts/r6-x86.md`; the nine
  held-out tasks were not answered, opened or disassembled (`r6-x86-sealed-guard.log`).
- [x] **IX.51. Freeze, tag, then run — protocol defect found before any number.** *(Superseded by IX.52/IX.53: two-mode guard adopted; `freeze-x86-sealed-v1` remains as the record of the defect.)* Freeze commit
  `651519c`, tag `freeze-x86-sealed-v1` (pushed, `git describe --exact-match` confirms). Manifest
  hashes re-verified 10/10 before and after. The run produced **no answers at all**: the frozen
  emitter refuses any task whose *name or object path* contains a held-out marker, so all nine
  sealed IDs were rejected in ~0.035 s (`refused: task '…p25519…' carries held-out marker '25519'`).
  The judge therefore saw zero answers; the CSV has no rows. **This is not a 0/9 solve rate — it is
  zero evaluated.** The guard conflates two different things: forbidding *development* from reading
  held-out proof text, and forbidding *evaluation* of held-out objects. Isolation was partial for the
  honest reason that the emitter hardcodes `.bench/problems.json` and `.bench/s2n-bignum`.
- [x] **IX.52. Two guard modes.** `--develop` (default) refuses held-out task IDs, objects and proof
  files. `--sealed-eval` allows the declared sealed task IDs and their objects, while still refusing
  to open any proof `.ml` for those tasks, and refuses unless the freeze tag and manifest match. One
  explicit manifest, no wildcard override. *Implemented at `22fc118`; tested by `tools/test_heldout_modes.py`.*
- [x] **IX.53. Re-freeze and run once.** Done 2026-11-24. Tag `freeze-x86-sealed-v2` (annotated;
  `^{commit}` = `22fc118bb275…` = `manifest.freeze_commit`), manifest
  `research/receipts/ix53-sealed-manifest.json`, judge `eval-20261124_003911` (1 core).
  **OK, 9 of 9** — all nine sealed x86 tasks judged, none refused, none not-evaluated. Seconds:
  add 22.138 / 23.178 / 17.939; sub 9.059 / 15.707 / 9.815; neg 3.320 / 3.651 / 3.102 (sum 107.909,
  slowest 23.178 = 0.193 of the 120 s budget, no timeout). Replay: 9/9 `closed=1 hyps=0 axioms
  2462→2462 unchanged=true`. Receipts: `research/receipts/ix53-sealed-run.md`,
  `ix53-sealed-verdicts.csv`, `ix53-freeze-reconciled.md`. **Caveats that travel with the number:**
  (i) `OK` is under the inherited inlined context (2,462 pre-loaded axioms, `before = after`), not an
  axiom-free dependency-closed result; (ii) isolation is partial — the emitter hardcodes
  `.bench/problems.json` and `.bench/s2n-bignum`, so the sealed set is protected by guard refusal of
  non-manifest IDs/objects/proof files, not filesystem separation; (iii) no sealed proof `.ml` was
  opened and only the nine sealed objects were disassembled.

### R6 · x86 capability (2026-11-23)

| task | verdict | s |
|---|---|---:|
| `x86.bignum_add_p384` | **OK** | 36.019 |
| `x86.bignum_add_p521` | **OK** | 62.790 |
| `x86.bignum_sub_p384` | **OK** | 19.599 |
| `x86.bignum_sub_p521` | **OK** | 36.223 |
| `x86.bignum_neg_p384` | **OK** | 5.997 |
| `x86.bignum_neg_p521` | **OK** | 6.998 |

Fresh replay: `closed=1 hyps=0 axioms 2462→2462` for all six. This is capability, not coverage: the
six are a declared development set.

**Sealed set (must remain untouched until the tagged freeze):**
`x86.bignum_{add,sub,neg}_{p25519,p256k1,sm2}.*_CORRECT` — nine tasks.

---

# PART X — Priority intake, 2026-11-24

Source: the current external review and the programme directive reproduced below. Nothing in it is
dropped; where an existing item already covers a requirement, the mapping is named instead of
duplicated. Items marked **[NEW]** have no earlier counterpart.

Directive, verbatim in substance: *Tekmerion must be an independent, Rust-native verification system
that interoperates with HOL Light; it must own its IR, calculus, certificate format, checker and
acceleration, must exceed every known assurance standard, must research methods never combined or
inverted-combined before, and must beat the frontier empirically and honestly. A negative result
that moves no published number is not a result.*

## X.0 · Freeze reconciliation — RESOLVED (2026-11-24)

- [x] **X.0.** The "tag/manifest disagreement" was a false alarm: `freeze-x86-sealed-v2` is an
  **annotated** tag whose object id is `120360b`, while `git rev-parse --verify
  freeze-x86-sealed-v2^{commit}` — the exact expression `heldout_guard.verify_freeze` uses — yields
  `22fc118bb275…`, equal to `manifest.freeze_commit`. `verify_freeze` and `sealed_policy` both PASS
  against `research/receipts/ix53-sealed-manifest.json`. The manifest commit `a2a4b47` follows the
  tagged code commit, per the adopted convention (manifest records the **code** commit; the manifest
  commit is evidence). The nine sealed objects exist under `.bench/s2n-bignum/x86/{curve25519,
  secp256k1,sm2}/`. No re-tag needed. Receipt: `research/receipts/ix53-freeze-reconciled.md`.
- [x] **X.0b.** Repository hygiene verified: 211 tracked files, 2,584 KB total, no `cadical`, no
  symlinks, no multi-MB logs. The 52k-line debug/binary blob flagged in the earlier review is absent.
  `research/receipts/bridge/REPORT.md` must not claim "no commit made" while committed — fix on next
  touch of that file.

## X.A · Native independence architecture

- [ ] **X.1. [NEW] Typed, architecture-independent verification IR.** A machine-state / control-flow /
  carry / memory IR that lowers ARM, x86-64, RISC-V and (later) LLVM ops into a common semantic form
  only where the behaviours are genuinely equivalent. Bit-exact; no unjustified mathematical
  abstraction. Reads the same object bytes the bench reads.
  *Acceptance:* one IR program lowers to byte-identical machine semantics for a routine already
  proved on two ISAs (start with `bignum_neg_p384`, the cheapest), and the lowering is checked, not
  asserted.
- [ ] **X.2. [NEW] Extract the HOL pipeline's decisions into explicit typed data** so HOL Light becomes
  one backend: machine-state equations, intermediate arithmetic facts, step sets, flag meanings and
  close choices leave OCaml tactic fragments and enter the IR.
  *Acceptance:* a P-384 add proof is generated from the typed record with no tactic string produced
  outside the backend.
- [ ] **X.3. [NEW] Portfolio proof engine + planner.** Independently engineered mechanisms over the IR
  — symbolic execution, equality saturation, SAT-backed refinement, quotient-witness synthesis,
  inductive-invariant synthesis, algebraic decomposition — with a planner that chooses by obligation
  shape, not by routine name.
  *Acceptance:* at least two mechanisms compose on one obligation with the planner's choice logged,
  and removing either mechanism loses the proof.
- [ ] **X.4. [NEW] The trusted checker validates semantics-to-formula, not only arithmetic.** A perfect
  arithmetic certificate must not certify a mistranslated instruction.
  *Acceptance:* a mutated decoder/encoder is rejected by the checker at the semantics boundary.
- [ ] **X.5. [NEW] Constraints on the kernel: machine-verification-specific, deliberately small.** Do
  not spend the programme reimplementing general higher-order logic. Expand rules only when a real
  workload requires it, with a decision record.
- [ ] **X.6. [NEW] Study Aeneas (Rust→prover separation) and Alive2 (semantic comparison) as
  precedents; record what is borrowed and what is not.** No claim of novelty where they already do it.

## X.B · TNPS rigor, standards and the “no stronger standard” requirement

- [ ] **X.7.** `research/06-TNPS-CHARTER.md` (IX.5) must state, per authority, the requirement it
  supplies **and** the residual gap Tekmerion still has. Authorities: DO-178C/DO-333/DO-330;
  NASA-STD-8739.8B + 2026-11-05 draft 8739.8C (evolving); ECSS-E-ST-40C Rev.1 + ECSS-Q-ST-80C
  Rev.2 (2025-04-30); Common Criteria EAL7; ARP4754B (systems context only); Candle/CakeML; CakeLPR;
  CakePB; seL4; CompCert; OpenTheory; Dedukti; SLSA; in-toto; Sigstore; NIST SP 800-193; NIST SSDF;
  GSN community standard; CUDA memory-error management; GPUVerify; Kontroli; verified compositional
  checking; online LRAT checking.
- [ ] **X.8. [NEW] “Match and outmatch” is an audited register, never a superlative.** For every
  dimension in IX.40 there is a row with: strongest known system, its published evidence, the metric
  that would demonstrate an exceedance, and the current Tekmerion value or `unmeasured`. No row may
  say "world-class" without a number and a receipt.
- [ ] **X.9. [NEW] The thirteen laws (IX.11–IX.23) become executable conformance tests,** not prose.
  Each law gets at least one negative test that fails closed.
- [ ] **X.10. [NEW] Machine-readable assurance case (IX.34), linked to certificates, hazards and
  sources; a dependency change revokes affected claims. State plainly (IX.35) that a proof cannot
  establish that the specification reflects mission requirements.
- [ ] **X.11. [NEW] Supply chain binds the exact checking program** (source rev, compiler/linker,
  binary digest, feature set, policy, semantics, cert root, independent results); reproducible
  rebuild comparison before critical releases; offline verification; decade-scale preservability
  (IX.36–IX.39).
- [ ] **X.12. [NEW] Fault model published, never "immune".** Logical soundness and execution
  integrity are separate claims; the fault-injection campaign (IX.28) measures refuse/recover/
  improper-accept and publishes residual risk.

## X.C · Acceleration (evidence, never authority)

- [ ] **X.13.** Portable scalar reference checker (IX.24) — mandatory; every other path agrees with it.
- [ ] **X.14.** Portable SIMD → AVX2 → AVX-512 with OS-enabled register-state detection (IX.25).
- [ ] **X.15.** Blocked multicore DAG replay, deterministic IDs, checked joins, fail-closed on
  disagreement (IX.26).
- [ ] **X.16.** CUDA two-mode: GPU elaborates, CPU confirms every inference; verified GPU checking only
  after implementation + translation evidence; report end-to-end accepted-proof throughput, never
  kernel-only timings (IX.27).
- [ ] **X.17.** Deterministic verdicts across every backend (IX.18); a scheduler, thread count or
  device must never change theorem truth.

## X.D · Interoperability is observation, not truth

- [ ] **X.18. [NEW] The three-level HOL compatibility contract, made explicit:** (i) specification
  compatibility, (ii) proof compatibility — HOL Light *reconstructs and checks* the proof, (iii)
  theory interoperability via OpenTheory where practical. Each level has its own conformance test.
- [ ] **X.19.** OpenTheory articles and HOL Light proofs are export targets; a foreign acceptance is
  an immutable observation record, never a native theorem (IX.29–IX.32). Unsupported semantics means
  refusal, never silent weakening.
- [ ] **X.20. [NEW] Native Tekmerion standard first, adapters second.** The native certificate format
  owns identity, hashing and replay; OpenTheory/Dedukti/Lean/Rocq are adapters with specified
  mappings and target-side reconstruction.

## X.E · Frontier and honesty

- [ ] **X.21. [NEW] No nulls as outputs.** A failed arm is reported beside the positive arm **and**
  reallocates the budget to the next mechanism. "Negative result is pointless" = do not stop at a
  null; identify which mechanism would move a published number and try it.
- [ ] **X.22.** Every claim is scoped to its revision, model, mode and attempt budget (IX.41). Never
  mix corpus revisions (2,301 / 2,284 / 2,622) or Pass@1 with Pass@5.

## X.F · Deep research — un-combined and inverted-combined mechanisms [NEW]

Deliverable `research/09-NOVEL-COMBINATIONS.md`: for each candidate, state the components, the
combination (or inversion), the closest published work with a citation, why the combination is not
already done, the falsifiable experiment, and the cost. Only after that may it be called a candidate.

- [ ] **X.23. Candidate space to investigate (minimum):**
  - **Carry-conservation as a first-class SAT certificate** rather than a tactic fold, so the
    arithmetic close is one checked object across ISAs and limb counts.
  - **Inverted reference route:** instead of proving `machine = ref`, synthesise the *weakest*
    reference function that the machine state implies, then prove `ref = spec` in a small prover —
    inverting which side owns the hard arithmetic.
  - **Equality-saturation over the carry chain** with SAT closing only the residual disequalities
    (e-graph rewrites are untrusted; the SAT refutation is checked).
  - **Portfolio with a cost model learned from measured judge times**, used only to order attempts
    (never to accept).
  - **Specification-relative mutation testing**: mutate the *specification* and require the checker
    to reject the old proof, closing the "valid proof of the wrong spec" gap (IX.33).
  - **Certificate compression with checked replay** (content-addressed dedup of identical inference
    subgraphs) with determinism and equivalence tests.
  - **Invariant synthesis from the postcondition + live machine state** for loops (Wall 2 / L-FIT),
    with the synthesised invariant kernel-checked.
- [ ] **X.24. [NEW] Literature sweep with receipts.** Primary sources only; every "nobody has
  combined X and Y" claim must name the search terms used and the closest hit. Negative novelty
  claims are the easiest to get wrong.
- [ ] **X.25. [NEW] Update `research/04-NOVELTY-REGISTER.md`.** Current status: only N3
  (conformal abstention) is `PLAUSIBLY_NOVEL`; nothing is `DEMONSTRATED_ADVANCE`. A claim moves up
  only with an experiment, never with an argument.

## X.G · Milestone B — logical core and critical parser, gaps stated precisely

- [x] **X.26. [NEW] Formal specification of the parser.** Done 2026-11-24 as §1.7 of
  `research/08-TNPS-SEMANTICS.md`: `parse_A` as a partial function, the four round-trip
  properties (P1)–(P4), the numeric-narrowing refusal rule, A8 (UTF-8 precondition), and the
  fail-closed rejection table. The refinement to the executable is G2 and remains open.
- [x] **X.27. [NEW] The trusted-base gap table, quantified.** Done 2026-11-24: §9 of
  `research/receipts/ix43-tcb.md` gives, for each of G1–G4, the exact proposition, code
  governed, evidence present, evidence missing and discharge route. The metric
  (1/4 gaps, 3/64 widths) stays visible and the checker is still not called verified.
- [x] **X.28. [NEW] Narrow the gap that is cheapest to close first.** Decision 2026-11-24:
  **(a) G4**, machine-checked in Lean 4 (`verification/tnps/Tnps.lean`). The G1/G2/G3 routes
  are costed in `ix43-tcb.md` §9; (b) typed-by-construction parsing is next.
- [ ] **X.29.** Independent acceptance (IX.17): a substantively independent implementation, not two
  runs of the same library. The differential oracle is a start; it is not independent acceptance for
  sealed results until it is used on the sealed artifact.

## X.H · P-384 glue — finish deriving it

- [ ] **X.30.** Remaining `DEVELOPMENT_ONLY` in the P-384 answer (IX.48): the simulation grouping
  (`1--13 / 14--22 / [23;24] / 25--38`), the bignum expansion pass, the post-`REAL_BITVAL_NOT`
  rewrites, and the mask tactic's recognition of the CSETM two-bit shape without decoding the
  instruction. Derive each from the state equations, or record precisely why it cannot be derived yet.
  *Acceptance:* the judged P-384 answer contains none of these as hand-written input; judge still
  `OK`; replay `hyps=0`, axioms unchanged; the removed lines are a receipt.

## X.I · Sealed evaluation

- [x] **X.31.** *(Done — see IX.53.)* Run IX.53 once: nine `x86.bignum_{add,sub,neg}_{p25519,p256k1,sm2}.BIGNUM_*_CORRECT`
  tasks with `--sealed-eval`, the manifest and tag above, one pass, no tuning, no source changes.
  *Acceptance:* `OK/n` printed by the judge, per-task seconds, fresh replay `hyps=0` axioms
  unchanged, and a plain statement that isolation is partial (the emitter hardcodes
  `.bench/problems.json` and `.bench/s2n-bignum`). A failure is reported, not fixed; any fix needs a
  new freeze and a decision record.

## X.J · Immediate order (supersedes earlier orders until done)

1. X.0 freeze reconciliation — **done**.
2. X.31 sealed x86 evaluation — this is the gate number; run while `tools/` is at the frozen commit.
3. X.26/X.27 Milestone B parser specification and quantified TCB gaps (no `tools/` change).
4. X.30 P-384 glue completion (changes `tools/`; after the sealed run).
5. X.23–X.25 research sweep and novelty register.
6. X.13–X.17 acceleration, scalar reference first.
Done in this order because (2) needs an untouched `tools/` tree, (3) does not touch `tools/`, and
(4) dirties it.

---

# PART XI — The Tekmerion Ordeal, Millennium Layer, and Areopagus (intake 2026-11-24)

Source: the external reviews dated 2026-11-24 plus the four standards documents
(Ordeal v1, Millennium Layer, Areopagus, and the millennial architecture). **Nothing is
omitted.** Items carry the identifier they have in the source so a reviewer can diff this queue
against the standard. A clause is `[ ]` until it has a receipt; a clause whose *firing control*
does not exist cannot be marked done.

## XI.0 — Blocking defects (fixed 2026-11-24, `af539ed`)

- [x] **XI.0a. CI red.** `cargo clippy --all-targets -- -D warnings` failed on
  `src/native/reference.rs:178` (`needless_lifetimes`) and then on
  `tests/native_semantics.rs:202` (`manual_contains`). Both fixed. Local gate: 79 Rust tests,
  48 Python tests (1 skip), mutation 16/16, lifecycle 5/5.
- [x] **XI.0b. TNPS soundness argument was wrong.** §6.3 claimed deletion preserves UNSAT;
  false (`{x,¬x}` minus `x` is satisfiable). Replaced with the model-preservation invariant.
  Not yet machine-checked (see XI.G).
- [x] **XI.0c. Leaked absolute paths** in `research/08-TNPS-SEMANTICS.md` and
  `research/receipts/ix43-tcb.md`; `tools/hol_tests/p256_carry_reuse.ml` used an absolute
  `loadt`. All removed; the harness now reads `TEKMERION_ROOT` with a relative fallback.
- [ ] **XI.0d.** Run a repository-wide scan in CI for `/mnt/`, `/home/`, drive letters and
  personal identifiers in tracked files. Firing control: a planted absolute path fails the build.

## XI.A — The Ordeal, O1–O14 (fourteen trials; one public label)

Rule: only a theorem passing **every** trial may be called `TEKMERION-CERTIFIED`. Anything
else is `SCOPED_ACCEPTED [failed: …]`. No partial credit, no rounding up.

- [ ] **XI.A.0. Superset map + firing controls.** Every requirement of every mapped regime
  appears in Annex A with an Ordeal clause at least as strong; every clause has a planted defect
  its check must catch, run on every build. A control that stops firing fails the build.

**O1 — statement fidelity.** [~] O1.1 challenge file written before search, frozen by digest
  (**post-hoc for the nine, `ix-o1-challenges.md`**); [~] O1.2 independent comparator:
  term-identical statement + axiom subset (**statement/object/judge-program binding done; kernel
  terms + axiom whitelist open**); [ ] O1.3 Pollack
consistency/faithfulness with an adversarial corpus (Unicode lookalikes, shadowed constants,
printer that swaps `T`/`F`); [ ] O1.4 two independent formalisations proved equivalent;
[ ] O1.5 non-vacuity: kernel-checked satisfying witness for preconditions, kernel-refuted
violation of each postcondition; [ ] O1.6 spec-mutation sensitivity (dropped bound, `<`→`≤`,
dropped conjunct such as `out < m`).

**O2 — derivation (de Bruijn).** [ ] O2.1 complete proof object exported, no trusted tactics;
[ ] O2.2 ≥2 independent checkers, disjoint implementation and authorship; [ ] O2.3 at least one
verified to machine code for the fragment — **for LRAT this is available now: run `cake_lpr` on
every LRAT certificate**; [ ] O2.4 acceptance originates only from a kernel theorem value, every
acceptance path mutation-controlled (the `BRIDGE_CSAT_TAC` false-success bug is the founding
counterexample).

**O3 — logical diversity.** [ ] O3.1 the full derivation accepted by two *different logics*
(e.g. HOL and a first-order/Metamath-Zero-style checker). Re-implementing one rule set twice does
not count. Control: planted unsound rules and the CVE-2026-72844 definitional class.

**O4 — context closure.** [ ] O4.1 re-check from an empty kernel (base axioms only, every
definition replayed); [~] O4.2 print the axiom set and require equality with the declared
whitelist — **upstream `check_axioms()` passes for all nine sealed x86 tasks** (IX.54
drop-in: only `INFINITY_AX`, `SELECT_AX`, `ETA_AX`); [~] O4.3 upstream drop-in: **nine of
nine built** (`ix54-upstream-dropin.md`, 50–71 s each, mutation control fires). Remaining:
the whole-tree upstream build and a downstream consumer. *Original:* Today's sealed `OK` is under 2,462 inherited axioms, so O4 is **not met**.

**O5 — artifact binding.** [ ] O5.1 digests of object bytes, loader view, ISA model, spec, kernel
binary, checker binaries, certificate engines; [ ] O5.2 theorem is about the *shipped* bytes;
[ ] O5.3 property preservation where a claim moves source→object (by construction here — record
it). Control: flip one byte; binding fails and typed revocation revokes exactly the dependents.

**O6 — machine-model validity.** [ ] O6.1 per instruction used: proof model vs mechanically
derived vendor model (Sail/ASL) vs two-microarchitecture silicon co-simulation, digest-bound.
Any unexplained disagreement blocks every theorem using that instruction.

**O7 — toolchain integrity.** [ ] O7.1 reproducible checker builds on two machines; [ ] O7.2
diverse double-compiling (DDC) for each checker binary not already verified to machine code;
[ ] O7.3 signed tagged releases with recorded binary provenance.

**O8 — execution integrity.** [ ] O8.1 every certified check on two dissimilar platforms
(different ISA *and* vendor), ECC where available; transcript digests must agree. Control: one
bit flip in one transcript must be caught.

**O9 — adversarial robustness.** [~] O9.1 standing hostile corpus (malformed, truncated,
transplanted, ill-typed, definitional exploits, hash-collision padding); [~] O9.2 differential
fuzzing of kernels against each other; [~] O9.3 refusal-site mutation score = 1.0 across the whole
TCB — **Rust 16/16; lifecycle 5/5 + parser 16/16; Python guard 66/71 with the 5 survivors
proved equivalent (`ix-o9-guard-mutation.md`), so 66/66 non-equivalent sites.** Remaining TCB:
the native checker's refusal sites.

**O10 — evaluation integrity.** [~] O10.1 freeze tagged and pushed *before* the run; manifest is
the only authority. **Strengthened 2026-11-24:** `verify_freeze` now also refuses unless
`git diff <freeze_commit> -- tools` is empty, so the code must *be* the frozen code, not merely
clean (`ix-o9-guard-mutation.md` §4a). Consequence: `v3` no longer verifies against the moved
tree; re-verify by checking out `2705f5b`. [~] O10.2 sealed runs in structural
isolation — **benchmark proofs are now unreachable inside the container** (IX.54:
`ix54-sealed-container.md`; O10.2 control shows `No such file or directory` and zero `s2n-bignum`
`.ml` under the container view). Residual limits, stated: the container borrows host userland
(read-only, glibc), and the judge dir was tmpfs so judge files were not preserved; next run must
bind-mount it. [x] O10.3 a held-out split is evaluated once (IX.53/IX.54 use the same nine; IX.54
is a re-run under stronger isolation, so the *number* is the same and the isolation is the new
fact — the split is not being mined for tuning). *Original text:* [ ] O10.2 sealed runs in structural
isolation (container/VM, read-only, no network); [ ] O10.3 a held-out split is evaluated once.

**O11 — independence and review.** [ ] O11.1 every certified statement reviewed by someone who
wrote neither statement nor proof, against a published checklist; [ ] O11.2 external
reproduction by an outsider from the frozen package (`EXTERNAL_REVIEW_PENDING` until then);
[ ] O11.3 every human override of an automated verdict logged and the rate published. This trial
needs **people**, not code; for a solo programme it is budgeted and cannot be waived.

**O12 — honesty of claims.** [x] O12.1 numbers generated from receipts; [x] O12.2 losses beside
wins; [ ] O12.3 certification language exact ("qualification-ready against DO-330 TQL-1
objectives", never "qualified"/"certified"); [x] O12.4 a regression withdraws the claim it
contradicts.

**O13 — revocation.** [ ] O13.1 any change to a bound digest revokes, automatically, with typed
severity; re-certification runs the full Ordeal again. Control: change one ISA-model digest;
exactly the dependent theorems revoke. The engine exists (`src/lib.rs`) but is not wired to
proofs.

**O14 — longevity.** [ ] O14.1 plain documented versioned text for certificates, statements,
checker rules; [ ] O14.2 re-implementation test: someone re-implements the smallest checker from
its written spec and reaches identical verdicts on the whole corpus; [ ] O14.3 frozen packages
archived with persistent identifiers (Software Heritage, Zenodo).

### XI.A.1 Baseline against O1–O14 (the nine sealed x86 proofs, `c4e1eb4`)

| Trial | Status | Cheapest next step |
|---|---|---|
| O1 | not met | challenge files from the bench statements; write the comparator |
| O2 | partly (HOL kernel only) | **`cake_lpr` on every LRAT certificate** |
| O3 | not met | long-term Dedukti/MM0 track |
| O4 | not met (2,462 inherited axioms) | empty-kernel re-check; upstream drop-in |
| O5 | partly (objects named; engine exists) | bind digests into each proof record |
| O6 | not met here (upstream co-sim exists) | import co-sim results per instruction, digest-bound |
| O7 | not met (unsigned) | signed tags; reproducible checker build; DDC |
| O8 | not met | re-run the nine on AArch64, compare transcripts |
| O9 | partly (16/16, 5/5) | extend to the full TCB and the bench harness |
| O10 | partly (tag first; procedural isolation) | container |
| O11 | not met | recruit two statement reviewers and one reproducer |
| O12 | largely met | keep |
| O13 | engine exists, not wired | wire |
| O14 | not met | archive the frozen packages |

**No current theorem is `TEKMERION-CERTIFIED`.** That is the correct result.

## XI.B — Millennium Layer, O15–O22 (1000-year reconstructibility)

Honest frame: nobody can prove a 1000-year property. What can be done is (1) name every
dependency that decays, (2) remove or make it renewable, (3) drill the cold start.

- [ ] **O15.** Eternal/empirical split: mathematics is stated to outlast us; evidence about
  silicon, products and people is dated history, never presented as timeless.
- [ ] **O16.** Closed-world statements: every symbol resolved inside the archive; a closure
  checker walks symbols to a definition or axiom and blocks on a dangling reference.
- [ ] **O17.** Foundation stamp (reverse-mathematics): every theorem carries the weakest base
  system that proves its eternal part, with a checked proof in that system. Target: the
  certified-SAT closes stamped **propositional resolution**; the arithmetic bridge in a weak
  arithmetic (PRA or QF arithmetic with induction); anything stronger reported explicitly.
  Control: a lemma silently using choice must be caught.
- [ ] **O18.** The Stele: the root checker is a **one-page specification**, fully specified,
  with ~50 hand-checkable Rosetta vectors plus invalid vectors with reasons. Translators into
  Stele format are untrusted because the Stele checks their output.
- [ ] **O19.** Cold-start drill: strangers given only the archive implement a checker from
  scratch; every ambiguity is a defect that blocks the next release; record time-to-first-verdict
  and every question asked.
- [ ] **O20.** Lineage ritual: decennial re-check by a freshly implemented Stele checker;
  each re-check appends an archived lineage entry.
- [ ] **O21.** Cryptographic expiry and renewal: ≥3 digests from unrelated families, renewal
  before weakness (RFC 4998 ERS model); succinct arguments are convenience, never the root.
- [ ] **O22.** Custody: plain text; many copies with independent custodians (LOCKSS); durable
  media for the core; a primer teaching the notation from counting upward, in several languages.

## XI.C — Areopagus (the court that administers the Ordeal)

- [ ] **XI.C.1. Certification Theorem.** For every certified `T`, check a second theorem
  `Cert(T)` whose hypotheses are exactly the residual trust assumptions: judges sound (H1),
  translations meaning-preserving (H2), hardware faithful (H3), statement transcribed correctly
  (H4), stored certificate is the checked one (H5). Nothing trusted that is not a named
  hypothesis. Precedent: Milawa; gap: per-result, whole-pipeline.
- [ ] **XI.C.2. Quantified residual risk.** One judge (the Stele) is built to claim *perfection*,
  enabling the Littlewood–Rushby bound for a possibly-perfect channel: `P(false accepted) ≤
  P(rich judge false-accepts) × P(Stele imperfect)`, without assuming independence. Publish the
  evidence for each factor and the epistemic uncertainty; never a bare number.
- [ ] **XI.C.3. Measured diversity.** Run a coincident-failure programme (mutate each judge; record
  which others still catch it) and publish a pairwise diversity matrix. Judges whose failures
  correlate are merged in the risk calculation, not counted twice. Record strategy classes:
  different logics (strongest), different execution technologies, different architectures.
- [ ] **XI.C.4. Checker appliance.** Verified processor (Silver line) + verified compiler, open
  PDK, fabricated at two foundries, imaged against the netlist, air-gapped, one-way media,
  signed verdicts; plus dissimilar commodity execution with bit-identical transcripts; TMR for
  radiation environments.
- [ ] **XI.C.5. Statement tribunal.** Dual formalisation, signed plain-language intent records
  with worked examples, and a paid red cell (technically, managerially, financially independent)
  that hunts for a program satisfying the statement but violating the intent.
- [ ] **XI.C.6. Defeater ledger (Assurance 2.0).** Every assurance case is a ledger of defeaters:
  each eliminated with evidence or printed as open on the certificate.
- [ ] **XI.C.7. Proof Safety Board.** Global registry of soundness incidents across all proof
  systems (root cause, versions, defeating input, regression vectors); confidential near-miss
  reporting (the `BRIDGE_CSAT_TAC` bug is founding entry); standing bounties sized to make
  breaking the court worth more than silence.
- [ ] **XI.C.8. False-theorem heartbeat.** A continuous stream of known-false theorems that every
  judge must reject; one miss halts all certification. Plus a flight recorder: every verdict
  deterministic and replayable forever.
- [ ] **XI.C.9. Two-person integrity for the trusted base.** Quorum of independent custodians
  with independently held keys, plus a mutation control proving the suite would notice removal.
- [ ] **XI.C.10.** Recompute the §XI.C.2 risk bound at every lineage re-check; archive its
  history so confidence changes only by evidence.

## XI.D — Eight-layer architecture, TNPS-0, proof-carrying checkers

Complexity increases outward; trust decreases outward. The smallest component has the most
carefully justified authority.

- [ ] **XI.D.1.** Specify all eight layers with a formal contract, explicit trust boundary and
  conformance standard: (01) Eternal nucleus, (02) Proof calculus, (03) Semantic verification,
  (04) Accelerated computation, (05) Verified execution, (06) Adversarial verification,
  (07) Assurance authority, (08) Continuity.
- [ ] **XI.D.2. TNPS-0.** A finite mathematical language with a precisely specified derivability
  relation — not a Rust/HOL/VM implementation. Investigate LF/Dedukti-style frameworks vs the
  Metamath-style single-rule approach before freezing. Constraints: unambiguous syntax, explicit
  capture-avoiding binding, exact arithmetic (no implicit overflow), precise equality and checked
  conversion, conservative definitions or declared axioms, machine-checked soundness per rule,
  decidable checking for the supported language, complete dependencies, extensions cannot add
  trusted inference silently, interpretation independent of language/hardware/runtime.
- [ ] **XI.D.3.** No arbitrary user rewrite becomes trusted by registration; no unconstrained
  normalisation inside the nucleus — require explicit conversion evidence or a separately verified
  decision procedure whose soundness theorem the nucleus can use.
- [ ] **XI.D.4.** `check(foundation, judgment, certificate, budget) → Result` specified as
  executable mathematics; `Check(F,J,C,R)=Accepted ⇒ F ⊢ J`; separate metatheorem from
  derivability to validity.
- [ ] **XI.D.5. Proof-carrying checker modules.** Every optimised module (SAT, modular arithmetic,
  polynomial identity, GPU) carries its spec, implementation, correctness certificate, resource
  contract, permitted capabilities and dependencies; `Run_M(x,c)=Accept ⇒ Valid_M(x)`, plus a
  proof of how the module's judgment is interpreted by TNPS-0. **The justification must not be
  circular.**

## XI.E — Semantic Constitution, assumptions, anti-vacuity, applicability

- [ ] **XI.E.1. Semantic Constitution** per verification subject: decoding, memory, exceptions,
  alignment, translation, concurrency, interference, initialisation, termination, timing,
  observation model; for physical systems add numerics, units, tolerances, uncertainty, validity
  domain.
- [ ] **XI.E.2. Semantic sensitivity certificates:** which model changes preserve the guarantee
  (proved), which break it (checked counterexample), which are unresolved. Report applicability
  rather than a broad "correct".
- [ ] **XI.E.3. Assumption Observatory:** assumptions are first-class objects with class
  (logical/mathematical/semantic/environmental/physical/implementation/institutional), meaning,
  provenance, scope, justification, dependencies, invalidation conditions. A physical measurement
  must not masquerade as an axiom; an institutional signature must not masquerade as a proof.
- [ ] **XI.E.4. Assumption counterfactuals:** classify each assumption dispensable / necessary
  (with checked countermodel) / unresolved; search for smaller sufficient sets; preserve
  unresolved outcomes. Never claim full minimisation.
- [ ] **XI.E.5. Anti-vacuity engine:** satisfiability of preconditions, reachability, spec
  implementability, over-unconstrained outputs, termination, hazard coverage. A valid vacuous
  theorem is still a valid theorem; a *safety claim* built on it is refused.

## XI.F — Verification fabric, trust cut-sets, hardware, execution

- [ ] **XI.F.1.** ≥3 substantially different checkers (minimal reference, formally verified,
  high-performance Rust, independent research checkers). Acceptance requires **all** mandatory
  checkers to accept the same judgment; any refusal/crash/budget-exhaustion/different semantics
  stops issuance. Majority voting is rejected as the authority.
- [ ] **XI.F.2. Fault-relative soundness:** under the declared bounded fault model, checking
  either yields a correct judgment or refuses issuance. Publish the fault model and residual risk;
  never claim immunity.
- [ ] **XI.F.3. Trust Cut-Set Analyzer:** model the whole infrastructure as a failure-propagation
  graph; publish identified failure paths and minimal cut sets (shared parser, shared bit-blaster,
  same compiler, same machine model, wrong theorem, issuance ignoring failure). Make assurance
  independence machine-analysable and partially formally verifiable.
- [ ] **XI.F.4. Dedicated appliance** per XI.C.4; keep commodity CPU/GPU for throughput.
- [ ] **XI.F.5. Proof-carrying parallel scheduling:** demonstrate every required block was checked
  with its exact dependencies and no unverified block entered the final derivation; optimality is
  not required.
- [ ] **XI.F.6. Resource-bounded reference path** with distinct outcomes: `Accepted | Invalid |
  Unsupported | BudgetExhausted | IntegrityFailure | InternalFailure`. A timeout is not a proof of
  falsity; an unsupported rule is not a counterexample; an internal failure is not a rejection.
- [ ] **XI.F.7.** Scalar → SIMD → AVX2 → AVX-512 → CUDA → verified hardware, each equivalence-tested
  and fault-injected (IX.24–IX.28, X.13–X.17 remain the operative items).

## XI.G — Formal gaps of the native checker (the current science)

- [x] **XI.G.1. G4 (paper lemmas) — machine-check.** **Done 2026-11-24.**
  `verification/tnps/Tnps.lean` (949 lines) compiles with Lean 4 core; `rup_sound`,
  `Lrat.refutation_sound` and `Cert.theorem1` are proved; `blast_sound` is a theorem
  **parameter**, not an axiom; `#print axioms` reports only `propext`, `Classical.choice`,
  `Quot.sound`; no `sorry`. Receipt `research/receipts/ix43-g4-lean.md`. G1–G3 remain open
  and the Rust checker is still **not** verified.
- [ ] **XI.G.2. G1 (bit-blaster equivalence).** Prove `blast` correct for the restricted fragment
  or replace it with a verified generator; state the width-indexed-name caveat. Acceptance:
  machine-checked or an independently generated encoding with a checked certificate.
- [~] **XI.G.3. G2 (parser refinement).** Progress 2026-11-24: **P1 holds by construction** for
  `Term`, `Certificate` and `Cnf::parse_dimacs` (each re-renders and compares; new test
  `dimacs_acceptance_is_exactly_the_canonical_rendering`). Remaining: (P1) for the LRAT reader,
  (P2)/(P3) grammar lemmas, and (P4) totality. Discharge (P1)–(P4) of `research/08-TNPS-SEMANTICS.md`
  §1.7, ideally by construction (typed parse target) plus a grammar/injectivity proof.
- [ ] **XI.G.4. G3 (executable refinement).** Connect the compiled checker to the abstract
  acceptance relation; include the parser and every trusted optimisation.
- [ ] **XI.G.5. Dual acceptance on a real obligation.** The native checker must re-derive its CNF
  **from the HOL statement itself**, not read the file CaDiCaL was given; then check a sealed
  task's certificate natively and in HOL Light. State plainly that the bit-blaster is trusted
  code until XI.G.2 lands.

## XI.H — Capsule, adapters, gates, issuance

- [ ] **XI.H.1. Thousand-Year Proof Capsule**, six layers: (0) reconstruction key, (1) permanent
  foundation, (2) exact judgment, (3) complete derivation, (4) verification evidence,
  (5) assurance history. Bootstrap ladder from physical explanation to full replay. Preserve an
  *interpretation* (compressed derivations + exact decompression rules + checked interpreters +
  their correctness proofs), not a prohibitive full expansion. Alignment with ISO 14721:2025,
  ISO 16363:2025, ISO 16919:2025.
- [ ] **XI.H.2. Adapters are observations.** HOL Light, OpenTheory, Lean, Rocq, Isabelle,
  Metamath, Alethe, LFSC, Dedukti via checked translations; each returns faithful export /
  characterised restricted export / refusal. No external format defines TNPS authority.
- [ ] **XI.H.3. Fifteen issuance gates G01–G15** (identity, grammar, logic, foundations,
  semantics, adequacy, implementation, independence, fault integrity, adversarial scrutiny,
  resource integrity, provenance, preservation, assurance, continuing validity), each with a
  falsifiable evidence requirement and explicit applicability rules.
- [ ] **XI.H.4.** `Issue(C,P) ⇔ ∀g∈G, Satisfied(g,C,P)`; the issuance implementation itself is
  verified against that policy; no score compensates for a failed gate; no one may waive a gate
  and keep the label.

## XI.I — Ten research inventions (order: flagship first)

1. [ ] **XI.I.1. Verified proof-module calculus** (XI.D.5 generalised).
2. [ ] **XI.I.2. Semantic sensitivity certificates** (XI.E.2).
3. [ ] **XI.I.3. Assumption counterfactual analysis** (XI.E.4).
4. [ ] **XI.I.4. Trust cut-set analysis** (XI.F.3).
5. [ ] **XI.I.5. Fault-relative checker soundness** (XI.F.2).
6. [ ] **XI.I.6. Proof-carrying parallel scheduling** (XI.F.5).
7. [ ] **XI.I.7. Proof-carrying assurance cases** (XI.C.6 + XI.H.3).
8. [ ] **XI.I.8. Temporal applicability contracts:** `Applicable(P,t) ⇔ ⋀ Aᵢ(t)`; monitored
   assumptions failing trigger the engineered safe response, never silent reliance.
9. [ ] **XI.I.9. Verified archival resurrection** (O19/O14.2).
10. [ ] **XI.I.10. Cross-era semantic migration:** checked correspondence between historical and
    future versions of the standard.

## XI.J — Benchmark laboratory and Project Millennium

- [ ] **XI.J.1.** Public tracks: logical rigor, implementation rigor, hostile-proof resistance,
  semantic rigor, specification adequacy, hardware resilience, parallel performance, native
  automation, interoperability, preservation. Measure the cost of a *justified* result, not the
  speed of "success".
- [ ] **XI.J.2. Project Millennium:** freeze requirements/theorem/models/artifact → native proof →
  independent verified checkers → semantic correspondence → replay on scalar/blocked/SIMD/AVX2/
  AVX-512/CUDA → verified hardware path → adversarial campaign → HOL/OpenTheory observations →
  assurance case + capsule → **independent clean-room reconstruction**.
- [ ] **XI.J.3.** Effort allocation is a *research* decision, not a forecast: foundations and
  checker metatheory, machine semantics and refinement, independent assurance and adversarial
  research, verified hardware, performance, preservation. Rebalance as bottlenecks move.

## XI.K — Engineering prohibitions and the Constitution

- [ ] **XI.K.1.** Prohibit: trusted inference plugins without formal justification; validity by
  vote among unverified solvers; hidden assumptions or implicit weakening; acceleration that
  changes acceptance semantics; undocumented certificate encodings; history rewritten by upgrade;
  security claims resting on fuzzing alone; assurance scores concealing mandatory failures;
  self-referential certification; indispensable proprietary dependencies; absolute-reliability
  claims; undeclared regulatory certification.
- [ ] **XI.K.2. Constitution, ten articles** as normative text in `research/`: (I) mathematical
  sovereignty, (II) complete justification, (III) verification of verification, (IV) exact
  meaning, (V) adversarial independence, (VI) computational excellence, (VII) intellectual
  integrity, (VIII) permanent reconstructibility, (IX) continuing applicability,
  (X) frontier responsibility.
- [ ] **XI.K.3.** Standards to study at source and add to Annex A before any public "no standard
  goes further" claim: **IEC 61508 SIL 4**, **ISO 26262 ASIL D tool confidence**, **NASA
  software-assurance standards**, **seL4's proof-maintenance practice**, **IEC/IEEE 60880:2026**,
  **IEC 62443**, **NIST SP 800-160**, **ISO/IEC 15408-1:2026**, **ISO 14721:2025**,
  **ISO 16363:2025**, **ISO 16919:2025**, **NASA-STD-7009B**. Distinguish adopted standards from
  drafts (NASA-STD-8739.8C is a 2026-11-05 draft).

## XI.L — Concrete frontier moves from the reviews

- [ ] **XI.L.1. Label the sealed result correctly.** 9/9 is **leave-one-ISA-out**: the ARM
  versions of these curves/operations were development data. It is not leave-one-curve-out. State
  this in the receipt and in every downstream claim.
- [x] **XI.L.2. Fix `XZR` flag accumulation.** **Done 2026-11-24, `r5-xzr.md`.** Correction to
  the diagnosis: the generic mechanism already existed at HEAD (`derive_steps._is_flag_only` +
  `_carry_out_feeds_chain`, landed after the R4 run); the R4 judge files simply predated it, so
  the failures were stale answers, not a missing lemma. All 18 ARM `add`/`sub`/`neg` tasks were
  regenerated and judged in **one** revision (`.bench/eval-20261124_030015`): **OK 18/18**,
  `hyps=0`, axioms `2445→2445` unchanged. This is **development data**, not held-out. New guard
  mode `--spent --spent-manifest` (decision record `ix53-spent-mode.md`) admits only the 18 spent
  ids and still refuses their proof `.ml`; 54 mode/derivation/x86 tests pass; `curve_check` 0
  hits. `add_p521` costs **96.882 s of 120 s (0.81)** — the headroom is thin and is now measured,
  superseding the reviewer's unreceipted 95.864 s. *Original text:* Flag-only `ADCS/SBCS XZR` chains are not encoded;
  the tooling excludes `XZR` destinations, so the correction flag is unconstrained. Fix, then
  re-run the affected `add_p384`/`add_p256k1` tasks.
- [x] **XI.L.3. Upstream drop-in build.** **Done for all nine 2026-11-24**
    (`research/receipts/ix54-upstream-dropin.md`): every generated x86 proof loads in upstream
    s2n-bignum, proves the stated theorem, and **`check_axioms ()` passes** (only the three basic
    axioms). Times 50–71 s. Mutation control: changing the modulus by 1 makes the build fail
    (`CSAT … exit 10`, exit 2). Measured correction: the 2,462-axiom figure is a property of the
    bench's inlined template, not of upstream `base.ml`. Remaining: a whole-tree upstream build
    and a downstream consumer. *Original:* Replace the human s2n-bignum proofs with the generated
    ones and run s2n-bignum's own build. Passing removes the inherited-context caveat; it is the
    cheapest strong upgrade available.
- [x] **XI.L.4. Containerise the sealed run.** Done 2026-11-24, IX.54 and IX.55. IX.54
  (`freeze-x86-sealed-v3`) emitted and judged all nine inside a container
  (`--network none`, `--read-only`, tmpfs `/work`, read-only mounts): **OK 9/9**, proof path
  absent inside. **IX.55** (`freeze-x86-sealed-v4`, `ix55-sealed-container.md`) repeats it with
  a writable `/out` bind mount: **OK 9/9** (`eval-20261124_061439`), the whole judge directory
  preserved, the collector independently re-run (`OK,9`) and the nine challenges re-verified
  (9/9). Evidence: `research/receipts/ix55/` + `eval.sha256` binding the 282 MB directory.
  Residual limit: host userland is borrowed read-only (glibc).
- [x] **XI.L.11. [NEW] Statement provenance from upstream.** Done 2026-11-24:
  `challenge verify --upstream` extracts the goal quotation from the pinned `.ml` and
  compares it to the bench statement. **27/27 PASS**. Decision record
  `research/receipts/ix-o1-upstream-audit.md` states the exception (goal range only, public
  statement, sealed run already frozen). Extractor handles the `let <NAME> = …` shape; a
  corpus-wide audit for other proof shapes is not claimed.
- [ ] **XI.L.12. [NEW] Corpus-wide statement audit.** The `--upstream` extractor handles the
  `let <NAME> = …` shape (the 27 arithmetic proofs). Other entries use `= prove`,
  `= prove_by_refinement`, etc. Extend per shape and audit the full 2,622-task corpus; report
  the shapes that cannot be extracted rather than skipping them silently.
- [ ] **XI.L.5. Full-898 sealed pass.** Freeze arm Z, one pass over all 898 FC tasks at the pin;
  unsupported shapes refused fast and counted as FAIL; report `OK/898`, zero `CHEATING`, split
  development-seen vs sealed. Then **submit the CSV to the leaderboard** so adjudication moves
  outside the programme.
- [x] **XI.L.6. Timeout headroom measured, not assumed.** Measured 2026-11-24 (`r5-xzr.md`):
  `add_p521` **96.882 s** of 120 s (0.81), `add_p384` 50.168 s, `add_p256k1` 22.061 s, all other
  tasks ≤ 35.216 s. The reviewer's 95.864 s figure is consistent in magnitude but was unreceipted;
  96.882 s is the recorded value on this host. A slower host plus another split could time out —
  measure before the full-898 run. *Original text:* A reviewer reports `add_p521` at
  95.864 s of 120 s; that figure has **no receipt here**. Measure every task under the frozen
  emitter on this host and record it; do not cite 95.864 s until reproduced.
- [ ] **XI.L.7. `cake_lpr` on every LRAT certificate**, and on the sealed set. This is O2.3 and
  the cheapest large trust upgrade.
- [ ] **XI.L.8. Divergent hardware.** Re-run the nine on AArch64 and compare transcripts (O8).
- [ ] **XI.L.9. Signed tags.** Sign `freeze-*` tags; record the public key and verification
  command in the receipt.
- [x] **XI.L.10. Challenge files + comparator** for the nine statements (O1.1/O1.2). Done
  2026-11-24, `research/receipts/ix-o1-challenges.md`. `src/tools/challenge.rs` freezes the exact
  statement bytes + corpus/object digests and verifies them against the judge program that ran.
  **27/27 PASS** (nine x86 sealed at `eval-20261124_003911`, eighteen ARM at
  `eval-20261124_030015`). **Honest limit:** all were frozen *after* the run (`pre-search
  false`, `POST_HOC`), so O1.1 is not met for them; `verify --require-pre-search` refuses them.
  Future runs must freeze before `emit`. Kernel-term identity is by construction via
  `prove(query, …)`; the axiom whitelist is now covered by the IX.54 upstream `check_axioms`.
  Eight negative controls pass (including shared-template naming).

## XI.M — Deep research queue (novel / inverted combinations)

Deliverable `research/09-NOVEL-COMBINATIONS.md`: components, combination or inversion, closest
published work with citation, why not already done, falsifiable experiment, cost. Primary sources
only; every "nobody has combined X and Y" names the search terms and the closest hit.

- [ ] **XI.M.1.** Carry-conservation as a **first-class SAT certificate** (one checked object
  across ISAs and limb counts), not a tactic fold.
- [ ] **XI.M.2.** **Inverted reference route:** synthesise the weakest reference function the
  machine state implies, then prove `ref = spec` in a small prover — inverting which side owns the
  hard arithmetic.
- [ ] **XI.M.3.** **Equality saturation over the carry chain** with SAT closing only residual
  disequalities; e-graph rewrites untrusted, the refutation checked.
- [ ] **XI.M.4.** **Portfolio planner with a cost model learned from measured judge times**, used
  only to order attempts, never to accept.
- [ ] **XI.M.5.** **Specification-relative mutation testing:** mutate the spec and require the
  checker to reject the old proof (closes "valid proof of the wrong spec").
- [ ] **XI.M.6.** **Certificate compression with checked replay** (content-addressed dedup of
  identical inference subgraphs), determinism and equivalence tested.
- [ ] **XI.M.7.** **Invariant synthesis from postcondition + live machine state** for loops, the
  synthesised invariant kernel-checked (Wall 2 / L-FIT).
- [ ] **XI.M.8.** Evaluate **foundational proof certificates** and **small-witness** checking for
  the expansion problem in XI.H.1.
- [ ] **XI.M.9.** Update `research/04-NOVELTY-REGISTER.md`: only N3 is `PLAUSIBLY_NOVEL`; nothing
  is `DEMONSTRATED_ADVANCE`. A claim rises only with an experiment.

## XI.N — Order of work (supersedes X.J once XI.0 is done)

1. XI.0 — blockers — **done** (`af539ed`).
2. XI.L.7 (`cake_lpr` on LRAT) + XI.L.10 (challenge files/comparator) — cheapest assurance.
3. XI.G.1 (G4 machine-checked) — **done** (`verification/tnps/Tnps.lean`, receipt `ix43-g4-lean.md`).
4. XI.L.2 (`XZR`), then XI.L.3 (upstream drop-in) — removes the two named caveats.
5. XI.L.4 (container), XI.L.9 (signed tags), XI.L.8 (AArch64).
6. XI.L.5 (full-898 + leaderboard).
7. XI.G.2–XI.G.5, then XI.F.7 acceleration.
8. XI.L.1 label correction in the receipts that predate this intake.

Every step pushes to `staging` on completion. No step is marked done without its receipt and,
where the standard requires it, its firing control.

---

# PART XII — Mentorship findings F1–F9, priorities P0–P7 (intake 2026-11-24)

Source: `tekmerion-mentorship-findings.md` and the accompanying findings text. **Nothing is
omitted.** Findings are ranked by how much each moves Tekmerion toward a result nobody can
argue with. The governing idea: **build a proof compiler, not a proof search engine** —
compile executable behaviour into mathematical evidence.

## XII.0 — Architectural frame (must shape every item below)

- [ ] **XII.0.1. Semantic compiler pipeline, not tactic search.** Executable artifact →
  verified decoding / symbolic execution / memory & flag semantics → mathematical-structure
  discovery (carry chains, polynomial relations, bounds, invariants, equivalences) → certificate
  synthesis (planning, algebra, SAT/SMT, rewriting, composition) → native checker → verified
  theorem. Unfamiliar routines must be proved from their semantics, not from name resemblance.
- [ ] **XII.0.2. Do not let SAT become the ceiling of the math.** Choose the representation that
  makes the obligation easiest: carry → integer conservation; reduction → quotient/residue
  certificates; multiplication → polynomial identities/limb decomposition; Montgomery →
  algebraic cancellation/divisibility; ranges → interval analysis; bitwise → certified SAT;
  nonlinear → polynomial normalisation; loops → inductive invariants + termination; schedules →
  relational equivalence; constant-time → relational trace reasoning.
- [ ] **XII.0.3. Stop measuring progress by code volume.** Track: previously unsolved theorems;
  new families; reusable certified algorithms; unverified trusted assumptions (down);
  task-specific branches (down); human-authored intermediate facts (down); proof-generation cost
  per theorem (down); native replay time (down); declared-semantics coverage (up); surviving
  soundness mutations (down); cross-version proof migration (up). Watch the ratio of new engine
  complexity to newly enabled *capability*.
- [ ] **XII.0.4. Keep architecture docs from outrunning the math.** Every proposed subsystem
  must eventually produce an executable artifact, a checked theorem, or a measured capability
  increase.

## XII.F1 — The proof script is the attack surface (HIGHEST VALUE) [P0/P1]

HOL Light's guarantee is enforced by OCaml's type system; a generated script is arbitrary
OCaml. The bench's `axioms()` comparison catches none of the following. Long-recognised LCF
risks: `Obj.magic`, `Marshal`, mutable strings/state.

- [x] **XII.F1.1 (gate implemented; grammar target open).** `src/tools/script_gate.rs` parses with
  the real OCaml front end through `hol.sh -pp` and refuses `Obj`/`Marshal`/`Unix`/`Sys.command`,
  `external`, and any binding or parameter named after a kernel/judge primitive; `--profile
  strict` also refuses `Sys`, `ref`, `:=` and file I/O. **Measurement: 0/26 answers pass even
  the denylist** (`Unix` 52, `Sys.command` 78, `ref` 260, `:=` 182; zero `Obj`/`Marshal`/
  `external`/rebinding). The emitted proof runs the solver as a subprocess at check time, so F1
  needs the architectural change below, not a syntax tweak. Receipt `ix-f1-script-gate.md`.
- [ ] **XII.F1.1b. Closed proof-term artifact (the actual fix).** Emit only a closed tactic
  term; take the LRAT as a **certificate input**; load the support library from the judge; no
  subprocess and no file I/O at check time. Then require the strict gate to accept all 27.
- [x] **XII.F1.2. Firing controls.** 12 tests (`tests/tools.rs`): `Obj.magic`,
  `Marshal`, `Sys.command`, `external`, shadowed `check_axioms`, rebound `prove`, kernel name as
  a parameter, `ref`/`:=` under strict, `Sys.getenv` under strict; plus the authoritative
  front-end control. All fire.
- [ ] **XII.F1.3. Milestone: one sealed s2n-bignum theorem accepted by Candle.** Candle is a
  fully verified HOL Light clone (every exported fact valid in HOL) parsing the same OCaml
  variant. First obstacle: loading the s2n-bignum libraries. Taking OCaml out of the trust base
  for that result is the highest-value single outcome in the programme.
- [~] **XII.F1.4.** Gate is implemented and tested; it is **not yet wired into the emitter**
  (that would refuse every current answer). Wire it after XII.F1.1b, then require it.

## XII.F2 — Export the derivation, not just the verdict

- [ ] **XII.F2.1. Proof recording on for every accepted theorem.** Export the trace.
  Not started. `ProofTrace/`/`Proofrecording/` are rejected for this purpose (see
  F2.2, which measures the axiom footprint without them): the
  ProofTrace README records 41 GB of theorems / 8 GB of proofs for a fresh
  `hol.ml` context, which does not fit this volume.
- [x] **XII.F2.2. Per-theorem axiom dependencies (foundation stamp O17), measured.**
  Done for all 27 generated theorems (18 ARM + 9 x86) — not just the nine x86.
  Mechanism: an axiom-taint bitset carried in `thm` in a *copy* of the kernel
  (`tools/hol-light/axiom-stamp.diff`, sha256 `f1b8a6db…a6ac`, applied to
  `.bench/hol-light-stamp`); every `new_axiom` allocates an id and every primitive
  rule unions its theorem arguments' masks. Receipt:
  `research/receipts/ix57-axiom-stamp.md` + `ix57-axiom-stamp.json`.
  Result: 27/27 pass `check_axioms`, and each theorem's stamp is exactly
  `{ETA_AX, SELECT_AX, INFINITY_AX}` — no axiom outside the three. Controls: empty
  stamp for `REFL`; an injected `new_axiom` shows up in the stamp and makes
  `check_axioms` fail; false statements still raise; the 27-theorem set is
  identical to the frozen run (wall time 1386 s vs 1398 s).
  Remaining under F2.2: the stamp is axioms only — definitional dependencies are
  not tracked, and the mask is a summary, not a replayable object.
  **Localised (IX.57 §10).** The uniform `{ETA_AX, SELECT_AX, INFINITY_AX}` is
  inherited from the substrate, not earned by the proof: `ARITH_RULE \`2 + 2 = 4\``
  already carries all three, `MESON \`!P. P ==> P\`` carries `{ETA, SELECT}` without
  infinity, and `define_trimmed` carries all three three lines after the ELF decode.
  The mask therefore does **not** settle whether choice is essential to a given
  theorem. Positive datum: `define_assert_from_elf` (bytes → semantics) is
  **axiom-free**, and so are `new_definition` and `REFL \`x:int64\``. Probe source
  `research/receipts/ix57/probe_stamps.ml`.
- [ ] **XII.F2.6. Definitional / choice-essentiality footprint.** New, from IX.57 §10.
  Decide *whether* a theorem needs `SELECT_AX` (not merely whether it inherits it) by
  re-deriving the statement in a context where choice-bearing lemmas are unavailable,
  and by tracking definitional dependencies (`new_definition` /
  `new_basic_type_definition` contribute mask `0` today). This is the analysis that
  turns the stamp from a dependency record into a foundation claim.
- [ ] **XII.F2.3. Re-check outside the OCaml process** (HOLTrace-style replay).
  Open. The IX.57 stamp cannot be replayed by a third party; it is measured by the
  same kernel that produced the theorem.
- [ ] **XII.F2.4. Export to a second logic** (Dedukti via hol2dk) on the small fragment needed.
  Open.
- [ ] **XII.F2.5. Inference-level size/shape baseline** generated vs human proofs.
  Generated side measured: 18,230,539–681,707,202 primitive-rule invocations per
  theorem (`inference_delta` in `ix57-axiom-stamp.json`). Caveat: this counts all
  primitive-rule invocations made during `prove`, including failed tactic search,
  so it is work, not proof-tree size. Human side not measured.

## XII.F3 — Check time is a first-class metric

- [ ] **XII.F3.1. Report check time next to every `OK`**; set the target of parity with the
  human proof on matched theorems. Baseline: P-256 add human 1.555 s vs ~24 s judged; suite
  ~2×10⁵ CPU-s/arch; `bstep` ARM 241,319 → 190,772 s; `add_p521` 96.9/120 s.
  **CORRECTION (XIV.0.1):** "P-256 add human 1.555 s" is withdrawn — it compared a single
  `prove` call against a judge run of a different scope. Use the matched whole-file numbers
  instead (human 24–26 s ARM / 50–53 s x86; generated 30–122 s), and prefer the deterministic
  primitive-inference ratio (`XIV.B2`) over wall clock as the F3 metric.
- [ ] **XII.F3.2. SAT for discovery, induction for reuse.** Use certified SAT to find and
  validate carry/selection facts, then generalise into **width- and modulus-parametric lemmas**
  proved once by induction over limbs and instantiated cheaply. This also lowers the foundation
  needed (arithmetic induction, not a 256-bit circuit) — helps O17.
- [ ] **XII.F3.3. Share the word-level `ref` lemmas across ARM and x86.** The identity
  "outputs equal ref(x,y)" for an (operation, curve) is ISA-independent.
- [ ] **XII.F3.4.** Re-measure the 18 ARM + 9 x86 tasks under the parametric-lemma route.

## XII.F4 — Regeneration is the property nobody else has

- [ ] **XII.F4.1.** Nightly: run the **frozen** compiler against upstream s2n-bignum HEAD and
  publish the survival curve (X% regenerated unchanged and green, no human edits).
- [ ] **XII.F4.2.** Attribute each break: upstream proof-infrastructure change, object-code
  change, ISA-model change.
- [ ] **XII.F4.3.** This is the "proof repair after change" measurement on industrial machine
  code; it is also the strongest argument for AI-driven verification.

## XII.F5 — Let the AI extend the compiler, under sealed evaluation

- [ ] **XII.F5.1.** Give an agent an unseen family (double, half, optneg, triple, then
  `montmul`/`montsqr`) with only object code, spec and compiler interfaces; it extends the
  witness generators and close strategies; freeze; evaluate sealed on unseen curves/ISAs.
- [ ] **XII.F5.2.** Metric: **time-to-coverage for an unseen family**, kernel as sole judge.

## XII.F6 — Statement fidelity, done by machines

- [ ] **XII.F6.1.** Adversarial statement agents (different model families) generate weakened
  mutants, vacuity probes, and intent-violating implementations that satisfy a weakened
  statement; the kernel arbitrates each.
- [ ] **XII.F6.2.** Seed statement defects the agents do not know about; report recall.
- [x] **XII.F6.3 (partial).** Dual extraction: `problems.json` vs pinned upstream `.ml`
  goal quotation, term-for-term. Implemented 2026-11-24 (`challenge verify --upstream`),
  **27/27 PASS**; decision record `ix-o1-upstream-audit.md`. Corpus-wide extension is
  XI.L.12.
- [ ] **XII.F6.4.** Non-vacuity by construction: for every precondition emit a concrete
  satisfying state and run it through the machine model; for every postcondition a concrete
  wrong output it rejects.

## XII.F7 — The upstream tree is the scoreboard

- [ ] **XII.F7.1. Upstream coverage**: the fraction of s2n-bignum's **675 straight-line FC
  theorems** (at the pin) that Tekmerion regenerates green **in s2n-bignum's own build**, by
  family and ISA, with check time. Rerunnable mechanically; needs no benchmark context.
- [x] **XII.F7.2 (partial).** All nine x86 sealed tasks built upstream with `check_axioms`
  passing (IX.54, `ix54-upstream-dropin.md`).
- [x] **XII.F7.3.** ARM 18: **18/18 built** in one forced pass (`make -B`, one core) on
  2026-11-24, `check_axioms` passing; the two stragglers are green too (`neg_p521` 44 s,
  `neg_sm2` 31 s). Receipt: `research/receipts/ix56-upstream-coverage.md`.
- [x] **XII.F7.4.** Whole set (27) in one upstream tree in one pass: **OK 27/27** (18 ARM +
  9 x86), exit 0 each, theorem present, `check_axioms` true; JSON
  `research/receipts/ix56-upstream-coverage.json`. Not a 675-coverage number (27/675 = 4.0%,
  the other ~648 unattempted); ARM answers are spent development data; `axioms()` is
  session-level (XII.F2 open). Timing defect: `p521/add_p521` 122 s against the 120 s budget
  (XII.F9.2).

## XII.F8 — Next coverage blocks, in order of yield

- [ ] **XII.F8.1. Linear families** (double, half, optneg, triple, cmul by small constants):
  same reference-route machinery plus F3 parametric lemmas.
- [ ] **XII.F8.2. Montgomery families** (`montmul`, `montsqr`): use upstream's own `ensures2`
  equivalence route with an agent-synthesised scalar reference program (the de-optimisation
  twin); compose with the reference's correctness as upstream does by hand.
- [ ] **XII.F8.3. Loops** (~25% of FC): postcondition-prefix invariants fitted on the
  executable model (v4 L-FIT), closed per iteration with F3 lemmas.
- [ ] **XII.F8.4. Montgomery reduction as the next research target:** discover `q` with
  `T + q·p ≡ 0 (mod R)`, `u = (T+q·p)/R`, prove `u ≡ T·R⁻¹ (mod p)`, plus a range proof for the
  final correction. The producer finds the witness; the checker validates algebra, divisibility,
  instruction correspondence and ranges. Do not ask SAT to rediscover number theory.

## XII.F9 — Determinism and timing hygiene

- [ ] **XII.F9.1.** Fix core count, memory and HOL Light build for every judged run; record
  them.
- [ ] **XII.F9.2.** Any task above **70% of its timeout is a defect to fix before a sealed
  run**, not a pass (`add_p521` at 96.9/120 s = 0.81 already fails this).
  — **Updated (IX.63):** `add_p521` is now 104/120 s = **0.867**, still above the
  0.70 threshold, so this stays open even though `XIII.E2` (under budget) is met.
  Against the 900 s sweep timeout it is 104/900 = 0.116. Do not conflate the two.
- [ ] **XII.F9.3.** Persist every judge output; no tmpfs-only verdicts. (IX.55 does this for
  the nine; make it the default.)

## XII.P — Immediate engineering mandate (P0–P7)

- [x] **P0. Fix outstanding assurance defects.** Freeze identity now binds `tools/` content
  (O10.1); missing verdict fails and no-`--evaldir` is INCOMPLETE (comparator); mutation CI
  passes offline with synthetic tests. Receipts `ix-o9-guard-mutation.md`, `ix-o1-challenges.md`.
- [ ] **P1. Close G1** — machine-checked bit-blaster soundness, no opaque hypothesis.
- [ ] **P2. Native multi-limb arithmetic** — checked identities beyond the 64-bit term
  fragment (the 256/384-bit obligations are currently outside the native logic).
- [ ] **P3. Multiplication proof generation** — one generic mechanism across unfamiliar
  multiplication routines.
- [ ] **P4. Native machine-code theorem** — complete native verification of the Curve25519
  addition theorem (the two halves united).
- [ ] **P5. Montgomery reduction** — generic proof construction from executable semantics.
- [ ] **P6. Autonomous lemma discovery** — demonstrated improvement on reserved obligations.
- [ ] **P7. Verified optimized checking** — measured acceleration with justified acceptance
  semantics.

## XII.R — Research inventions to pursue (from §13)

- [ ] XII.R.1 Arithmetic Structure Compiler (recover identities from arbitrary supported
  instruction sequences).
- [ ] XII.R.2 Proof Failure Intelligence (distinguish mathematical / semantic / abstraction /
  resource failures; CEGAR-style repair loop; genuine counterexample vs spurious; missing
  carry/range/lemma; unsupported instruction; SAT too expensive; bad induction invariant).
- [ ] XII.R.3 Autonomous Theory Exploration (Hipster/QuickSpec-style; generalise across
  families, measure new capability per discovered theorem).
- [ ] XII.R.4 Semantic Proof Fingerprints (recognise shared behaviour across implementations).
- [ ] XII.R.5 Proof-Cost Optimizer (choose representations/decompositions minimising
  verification cost).
- [ ] XII.R.6 Proof-Preserving Superoptimizer (optimise code and its evidence together; SLOTHY
  interface).
- [ ] XII.R.7 Verified Certificate Module System (modules with checked correctness contracts).
- [ ] XII.R.8 Self-Improving Proof Compiler (convert discoveries into reusable verified
  algorithms; improvement must not depend on the model).
- [ ] XII.R.9 Native Semantic Migration.
- [ ] XII.R.10 Millennial Proof Representation.

## XII.S — Additional technical mandates from the findings

- [ ] **XII.S.1. Proof-planning architecture.** Typed graph of obligations, not an agent
  transcript. Roles: semantic analyst, proof architect, algebraist, invariant synthesiser,
  countermodel analyst, metatheory agent, performance engineer, adversarial agent. None may
  construct an accepted theorem. AI strategy is disposable; evidence is permanent.
- [ ] **XII.S.2. Capability-based Rust kernel API** (`CandidateCertificate` vs private
  `CheckedJudgment` vs `VerifiedArtifact`; `check(expected, candidate, budget)`). Invalid
  states unrepresentable; refinement still required.
- [ ] **XII.S.3. Relational verification early** (constant-time, implementation equivalence;
  transfer a reference theorem to an optimised twin via a proved equivalence; discover
  register/state/memory maps, reorderings, synchronisation cut points). Study the 2025
  relational Hoare logic for realistic machine code.
- [ ] **XII.S.4. Verification-aware optimisation** (optimise executable and proof together;
  proof complexity as part of the objective).
- [ ] **XII.S.5. Profile before accelerating.** Dense clause storage, SIMD bitsets, multicore/CUDA
  batches only where measured; hash-consing; streaming bounded-memory checking; blocked DAG
  execution; content-bound theorem reuse. Targets: wall time, peak memory, certificate size,
  generation cost, replay cost, energy, scaling.
- [ ] **XII.S.6. Benchmark revision discipline.** State the exact dataset revision and
  denominator: the published dataset lists **2,301 tasks / 862 FC**, the pinned corpus here is
  **2,622 / 898 FC**. Never mix them.
- [ ] **XII.S.7. Full-corpus attempt as research data.** Failed and unsupported tasks are
  findings, not noise; each gets a failure class from XII.R.2.

## XII.T — Frontier claims these findings set up (adjudicator in brackets)

- [ ] Generated proofs that cannot forge through the script language (kernel; Candle).
- [ ] A measured foundation per theorem (exported proof object).
- [ ] Check-time parity with human proofs (the clock).
- [ ] Self-maintaining machine-code proofs (upstream HEAD nightly).
- [ ] AI-extended coverage on unseen families (kernel; sealed manifests).
- [ ] Upstream coverage of s2n-bignum FC (s2n-bignum's own build).

## XII.U — Order of execution (supersedes XI.N)

1. XII.F1 grammar gate + firing controls (P0/P1 blocker; do first).
2. ~~XII.F7.3/F7.4 finish the ARM drop-in and publish the 27-task upstream table.~~
   **Done** — `ix56-upstream-coverage.md`, 27/27 forced pass.
3. ~~XII.F2 proof recording + per-theorem axiom stamp on the nine.~~ **Done as the
   axiom stamp** (`ix57-axiom-stamp.md`, 27/27) with the §10 localisation; the rest of
   F2 (exported derivation F2.1/F2.3/F2.4, definitional footprint F2.6) stays open.
4. XII.F3 parametric lemmas (check-time reduction), then XII.F8.1 linear families.
   Note `p521/add_p521` at 122 s exceeds the 120 s budget — F9.2 defect, fix first.
5. P1 (G1) and P2 (multi-limb native arithmetic).
6. XII.F4 nightly regeneration, then XII.F5 sealed agent extension.

---

# XIII — Intake 2026-11-24 (after `ecde4fc`)

Three reviews and two follow-ups. Nothing omitted. `XIII.I` supersedes `XII.U`.
Every item is written so it can be dispatched as one self-contained brief.

## XIII.B — The axiom instrument is not yet correct (do first: trust-critical, cheap)

A reviewer read `axiom-stamp.diff` and found a real defect. Fix it before the stamp is
cited again.

- [x] **XIII.B1.** Fixed in patch v2 (`tools/hol-light/axiom-stamp.diff`).
  `new_basic_type_definition` now returns both results with the witness theorem's
  mask. Receipt: `research/receipts/ix57b-axiom-stamp.md` §1, §4 (N1).
- [ ] **XIII.B2.** Half done, half explicitly open. The **statement-level**
  symbol footprint is implemented (`footprint_of`): constants and type
  constructors in the conclusion, each mapped to the definition or type
  definition that introduced it when the kernel registry knows it. The
  **transitive definitional closure** over the semantic environment is **still
  open**: it needs a recorded proof/derivation DAG, which this patch does not
  produce. Do not describe the footprint as derivation closure.
- [x] **XIII.B3.** N1 in `ix57b-axiom-stamp.md` §4: the same construction prints
  `[]` on `.bench/hol-light-stamp` (v1) and `[4]` on
  `.bench/hol-light-stamp2` (v2), for both `abs_rep` and `rep_abs`.
- [x] **XIII.B4.** Capacity guard: `axiom_capacity () = Sys.word_size - 2` (62
  here). Allocating past it fails with
  `axiom_stamp: capacity exhausted: ... refusing to allocate axiom id 63`.
  N2 in `ix57b-axiom-stamp.md` §4 shows the guard firing and ordinary proofs
  unaffected afterwards.
- [x] **XIII.B5.** Registry entries are complete, fully typed canonical terms
  with no truncation, plus a 32-bit FNV-1a digest; stamp entries are
  `id:<n>#<digest>=|- <canonical>`. `ix57b-axiom-stamp.md` §1, §4 (C1/C2).
- [x] **XIII.B6.** `StampReport::validate` and `TaskResult::green()` require the
  `STAMP` line to name the theorem, unique positive registry ids, every stamp id
  to resolve with a matching digest and conclusion, and `after - before ==
  delta`. Negative tests cover a foreign theorem, a dangling id, a duplicate
  registry id, a bad digest and an inconsistent delta (`src/tools/dropin.rs`).
- [x] **XIII.B7.** Corrections appended to `ix57-axiom-stamp.md` (§10 item 4 and
  a new §11); no historical text rewritten. The upper-bound claim is withdrawn
  for v1, and `define_assert_from_elf` reads "empty *recorded* stamp".
- [x] **XIII.B8.** The surviving claim is unchanged: the session-level
  `check_axioms()` result for the 27 (IX.54/IX.56), plus the corrected
  per-theorem stamps in `ix57b-axiom-stamp.md`.

## XIII.C — The freeze boundary must follow the code into Rust (Critical)

`verify_freeze()` still runs `git diff --name-only <freeze_commit> -- tools`. Since the
migration the producer lives in `src/tools/` and `src/bin/`, so the freeze does not bind
the emitter at all.

- [x] **XIII.C1.** Bind the whole relevant closure: `src/tools/**`, `src/bin/**`,
  `src/native/**`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain*`, the built executable
  digest, the OCaml bridge (`tools/csat_lib.ml`, `tools/bridge.ml`), the benchmark
  inputs, and runtime dependencies (`cadical` path + digest, HOL Light pin).
  — closed by `src/tools/freeze.rs` (source layer walks all of `src/**`, so
  `src/bin/**` and `src/native/**` are covered) plus the runtime layer; no
  `rust-toolchain*` file exists on this tree. Receipt: `research/receipts/ix59-freeze-closure.md`.
- [x] **XIII.C2.** Record that closure in the freeze record; `verify_freeze` must refuse
  on any difference — including a clean checkout whose contents differ. —
  `freeze verify` recomputes every digest; `verify_freeze` enforces the closure when a
  sealed manifest carries `freeze_closure`, and keeps the legacy `tools/` scope (with a
  stderr warning) when it does not.
- [x] **XIII.C3.** Firing controls, each must refuse: modified emitter source; swapped
  binary under the same name; changed `Cargo.lock`; different HOL Light build; a
  different `cadical`. — all six controls refused; verbatim output in IX.59.
- [x] **XIII.C4.** New freeze tag after C1–C3. Freeze v4 stays valid for its own scope;
  it does not cover the Rust producer. — **Done:** `freeze-rust-closure-v5`
  (`51e24fd`, annotated, unsigned), manifest `research/receipts/freeze-closure-v5.json`,
  `source_digest 8a0d2439…b88d` over 65 files; receipt `ix59-freeze-closure.md`.
  Guard mutation 98/98 sites killed (was 58/58). Legacy manifests still verify with a
  warning; IX.53–IX.57 are not retroactively upgraded.
- [ ] **XIII.C5. Verify the sealed state, not the working directory.** `freeze verify`
  hashes the mutable working tree. Observed once by the orchestrator: a verify run on
  the tagged tree refused with `actual 4c409ad6…` for `src/tools/guard.rs`, a digest
  matching no committed file; a rerun passed and `check-tag` and a fresh clone both
  pass. Best explanation is a race with a concurrent writer (the compile emitted an
  `unreachable_code` note that the committed tree does not produce). Not reproduced.
  Fix: hash the closure from the **git object store at `repo_commit`** (and compare the
  working tree against it as a separate, clearly-labelled check), so verification is a
  statement about the sealed commit and cannot be perturbed by concurrent edits. Test:
  dirty the tree and show the committed-state check still passes while the
  working-tree check refuses, naming the path. — annotated unsigned tag `freeze-rust-closure-v5`;
  IX.53–IX.57 manifests still verify under the legacy scope.

## XIII.D — Small honesty defects

- [ ] **XIII.D1.** `dropin` dry-run prints the *planned* count as assembled and exits 0
  even when assembly failed (`src/tools/dropin.rs`, dry-run branch). Report actual
  successes; exit non-zero if a required assembly failed.
- [ ] **XIII.D2.** `src/tools/json.rs` parses numbers through `f64`. Use exact integers
  where integers are expected (ids, counts, seconds), validate the JSON number grammar
  (reject leading zeros), bound recursion depth, add negative tests.
- [ ] **XIII.D3.** Emitter parity tests return early when `.bench` is absent, so green CI
  is not full regression coverage. Add synthetic fixtures so parity runs offline, and
  state the limitation wherever it remains.

## XIII.A — Certificates as data (the architectural step; unlocks three things)

The F1 gate measures **0/26**: every emitted answer spawns `cadical` and does file I/O
while being checked. The kernel still replays the LRAT, so this is not a soundness hole —
it is a trust-surface and reproducibility hole, and it blocks Candle.

> **Measured 2026-11-24 for `arm/curve25519/add_p25519`
> (`research/receipts/ix61-certificates-as-data.md`).** The proof performs **4** SAT
> closes (inside the 8-close budget) but their LRAT texts total **65,657,625 bytes**
> (14.5 MB, 10.0 MB, 26.5 MB, 14.7 MB), an answer of ~66 MB where the current answer is
> 104 KB. Binary caDiCaL LRAT is 29,057,164 bytes; a backward-closure trim that keeps
> only the lemmas the empty clause uses is 22,765,098 bytes. All are ≥2.8× the declared
> 8 MB in-answer budget, so the in-memory `CSAT_REFUTE_CERT` design for `A1–A3` was
> **stopped after the measurement**. No certified answer exists; no control was run; no
> box below is ticked. `A7` is now the selected route (`A8`).

- [ ] **XIII.A1.** Emit a **closed tactic term**: allow-listed combinators, term
  literals, named lemmas. No `Unix`, `Sys`, process spawn, file I/O, references,
  `external`.
- [ ] **XIII.A2.** The SAT run becomes **proof construction**, not checking: the LRAT is
  produced at emission and shipped as a data certificate; the judge loads and replays
  it.
- [ ] **XIII.A3.** The judge supplies the support library (Cadical reader / `csat_lib` /
  `bridge`), so the answer neither inlines nor fetches it.
- [ ] **XIII.A4.** The strict gate must then accept **27/27**, and the gate becomes
  required in the emitter.
- [ ] **XIII.A5.** Deterministic, solver-free re-check: a second machine with no
  `cadical` and no network replays every accepted answer.
- [ ] **XIII.A6.** First **Candle** attempt: one sealed proof re-checked in the verified
  clone. That is the named milestone that takes OCaml out of the trust base.
- [ ] **XIII.A7.** Certificates become content-addressed stored artefacts (needed by
  XIII.F and the Millennium layer).
- [ ] **XIII.A8.** Certificate transport selected by IX.61: the answer carries the
  **digest** and the judge reads one content-addressed certificate file (read-only,
  no writes, no solver), rather than the 65.7 MB LRAT in the answer. Before building
  it, measure a real `lrat-trim` pass at emission; if a trimmed certificate lands
  under ~1–2 MB the in-answer no-I/O design becomes viable again and should win.
  Declare the K2 change explicitly. Blocked on a decision record, not on code.
- [ ] **XIII.A9.** The judge cannot supply a support library to a *compiled* script
  (a runtime `loadt` gives the type-checker no names). Any no-inline route needs the
  sealed-court builder to inject the library text into the template prefix and raise
  `inlined_locations[0][1]` by its line count. Mechanism only; not implemented.

## XIII.E — Proof work reduction (F3: optimisation has become a blocker)

- [ ] **XIII.E1.** **Metric change:** primitive-inference count is the F3 metric
  (deterministic, machine-independent). Baseline 3,697,462,655 across the 27
  (`ix57-axiom-stamp.json`); 18.2M–681.7M per theorem.
- [x] **XIII.E2.** `arm/p521/add_p521`: 681,707,202 inferences, 122 s against a 120 s
  budget. Bring it under budget with the statement unchanged, and report against the
  900 s sweep timeout too so the two budget notions are never conflated again.
  — **Done (IX.63):** the backward-closure LRAT trim inside `CSAT_REFUTE_GEN`
  (`tools/csat_lib.ml`) brings it to **470,294,392 inferences and 104 s** against the
  120 s budget (86.7%, so still above the 70% F9.2 threshold), and **104 s against
  the 900 s sweep timeout** (11.6%). `equals_thm` to the untrimmed theorem is true
  (IX.63 T4); `research/receipts/ix63-trimmed-replay.md`.
- [~] **XIII.E3.** Matched human baseline, but **compare the goals first**: upstream's
  human files prove `BIGNUM_*_SUBROUTINE_CORRECT`, our drop-in files prove
  `BIGNUM_*_CORRECT`. Partial data in `research/receipts/wip/ix58-checktime.partial.json`
  — **25** tasks, not 26 (18 ARM + 7 x86; missing x86 `secp256k1/neg` and `sm2/neg`;
  verified with `grep -c '"dir"'` = 25). It is **not a receipt**; no ratio may be
  published until the statements are compared. See `XIV.B` for the deterministic
  replacement metric. — **Decided 2026-11-24 (IX.60):** all 27 conclusions differ and
  are incomparable (extra return-address pre/postconditions and a broader ABI frame on
  the human side). There is no matched baseline and no ratio; the pair of inference
  counts is published side by side in `research/receipts/ix60-matched-inference.md`.
- [ ] **XIII.E4.** Parametric carry lemmas: proved once over width and modulus by
  induction over limbs, instantiated per theorem; replaces per-theorem SAT replay.
  — **Verdict from data (IX.65):** aimed at the right block, not proved removable.
  On the six mandatory theorems, **98.2%** of primitive inferences are inside
  `BRIDGE_*_REF_TAC` (67.85% bridge-minus-replay, 30.35% certified replay) and
  simulation is **0.77%**, so the warning "the cost is in simulation, which
  parametric lemmas do not touch" does **not** apply. The targeted block is the
  per-theorem proof of `val (ref_add (word p) x y) = val w`, already built on the
  width-parametric `ADD_REF_VAL_WORD`. What IX.65 does **not** show is that a
  parametric lemma removes it: the block also contains `CSAT_PROVE` encoding and
  `BRIDGE_TO_WORD_CONV`, which are not separated here. Next measurement: split the
  bridge into prepare / encoding / replay before building the lemma.
  `research/receipts/ix65-same-obligation-and-phases.md`.
  — **Verdict from data (IX.66):** the block is now split, and the lever is named.
  Across the six mandatory proofs (**993,573,633** primitive inferences): **bitblast
  (`BITBLAST_THEN`) 487,167,871 = 49.03%**, **definitionalise+clausify (`CSAT_PROVE`)
  178,375,918 = 17.95%**, **certified LRAT replay 301,587,637 = 30.35%**, **bridge
  rewrite (`BRIDGE_TO_WORD_CONV` / `ELIM_DIGIT_EQS_TAC` / expand / condition splits)
  412,923 = 0.04%**, **residual 8,155,718 = 0.82%** (the DIMACS write, CaDiCaL and
  the LRAT trim are **0** primitive inferences on all 30 closes; the rest is
  `PROVE_HYP`/`main_rule`/`ACCEPT_TAC`). `add_p521` (470,294,392): bitblast
  228,940,358 (48.68%), defn+clausify 79,017,426 (16.80%), replay 154,535,460
  (32.86%), normalisation 162,873 (0.035%), residual 3,553,893 (0.76%). So the
  target is **not** wordification (0.04% — that hypothesis is dead) and **not** the
  replay alone (30.35%): it is the **bit-blasting of the goal and of the simulator's
  accumulator hypotheses**, a stable 28.2 inferences per bit-blasted subterm
  (26.8–29.8 on every task), with four ~2.0M-subterm instances per `add_p521`.
  **Upper bounds, not predictions:** deleting bitblast would take `add_p521` to
  241,354,034 and the total to 506,405,762; deleting the whole SAT close (a
  parametric rewrite that never builds a CNF) caps at 4,084,382 and 17,873,566.
  Still **not proved** that a width/modulus lemma reaches that cap, and the cost is
  charged to the *hypothesis* set, so a lemma must remove the hypotheses from the
  instance, not only the conclusion. Open.
  `research/receipts/ix66-bridge-split.md`.
- [~] **XIII.E5.** Emit proofs that do not search: separate successful derivations from
  discarded search in the inference count, and drive the discarded part down.
  — **Partial (IX.65):** a phase switch now separates certified replay from the
  bridge's own work, and the certified replay from solver search (cadical's search
  is outside the kernel and produces no primitive inferences). Discarded search
  *inside* a successful HOL block is still mixed into that block; not separated and
  not driven down. `research/receipts/ix65-same-obligation-and-phases.md` §3.
- [ ] **XIII.E6.** Cost observatory: definition dependencies, lemma dependencies,
  successful vs discarded inference nodes, repeated subderivations (sharing),
  inference cost per tactic, certificate size, cost per theory.

## XIII.F — Foundational minimization (new research objective)

- [ ] **XIII.F1.** Separate four objects currently conflated: direct derivational
  dependencies; transitive definition/type dependencies; semantic model assumptions;
  the checker's own foundations. Each gets its own evidence.
- [ ] **XIII.F2.** Do **not** attempt a choice-free context inside HOL Light: `?` is
  defined through `@`, so nothing survives. The honest HOL-level stamp is "HOL, all
  three axioms, inherited from the substrate".
- [ ] **XIII.F3.** Answer sufficiency elsewhere: re-check the certified-SAT core
  (propositional) and the bridge lemmas (arithmetic) in a separate weak-arithmetic
  checker, fed by XIII.A certificates.
- [ ] **XIII.F4.** State the triad at the right strength: dependency (measured),
  sufficiency (constructive — exhibit a weaker derivation), necessity (usually
  undecidable; only ever specific independence results).
- [ ] **XIII.F5.** The decisive experiment: two proofs of one property — the HOL route
  and a native weak-foundation route — compared on foundations, primitive inference
  work, certificate size, check time and generality, with **identical conclusions**.

## XIII.G — Coverage (the frontier claim still rests here)

- [ ] **XIII.G1.** First **new family** (`double` or `half`) under a pre-search freeze,
  landing in the upstream tree with gate + provenance + check time. Every later theorem
  then arrives already gated, provenance-checked and upstream-built.
- [ ] **XIII.G2.** Then `triple`, `optneg`, `cmul` by small constants.
- [ ] **XIII.G3.** Whole-family drop-in name mismatch: **repaired for the correctness
  obligation** (IX.64). Our built `_CORRECT` was shown to be the same statement as upstream's
  `_CORRECT` for 27/27, and assembly now appends upstream's own `_SUBROUTINE_CORRECT`
  statement and derivation verbatim from the pinned `.ml`. All 27 files now prove
  `BIGNUM_<OP>_<FAM>_SUBROUTINE_CORRECT`; `collect-specs.sh` finds 18 ARM + 9 x86 names and
  whole-tree `make -j1` (default target) is green in both arch directories. Still open:
  `_SUBROUTINE_SAFE` and x86 `_WINDOWS` variants are not emitted, and a full `make proofs`
  over the tree (452 ARM + 1758 x86 obligations) has not been run.

## XIII.H — Native checker (unchanged gaps; coverage must not hide them)

- [ ] **XIII.H1.** G1 bit-blaster soundness — the central hypothesis of the abstract
  soundness theorem.
- [ ] **XIII.H2.** G2 parser refinement (canonical rendering is checked; refinement is
  not proved).
- [ ] **XIII.H3.** G3 executable refinement (deployed Rust vs the justified relation).
- [ ] **XIII.H4.** P4: one complete native machine-code theorem — solver-free replay,
  faithful HOL representation. The union of both halves.
- [ ] **XIII.H5.** P2: native multi-limb arithmetic beyond the 64-bit fragment.

## XIII.I — Order of execution

1. **XIII.B** — correct the instrument, then correct the receipts. Cheap, and a wrong
   instrument poisons everything downstream.
2. **XIII.C** — freeze the Rust closure. Nothing sealed is honest until this holds.
3. **XIII.D** — the three small defects.
4. **XIII.A** — certificates as data; strict gate 27/27; then Candle.
5. **XIII.E2/E3** — `add_p521` under budget; equal-goal comparison.
6. **XIII.G1** — first new family under freeze.
7. **XIII.E4/E5** — parametric lemmas; stop searching.
8. **XIII.H** — native foundations, then XIII.F.

---

# XIV — Intake 2026-11-24 (after `f98dd9d` / `1186058`)

Three reviews and two follow-ups. Nothing omitted. `XIV.C` supersedes `XIII.I`. Two
corrections are owed and are recorded here rather than edited out of history.

## XIV.0 — Corrections owed

- [ ] **XIV.0.1. Retract the "10–20×" check-time framing.** `TODO.md` line ~709
  ("Drop 'generated proofs check faster' for linear families. Human: 1.555 s add")
  and line ~2010 ("P-256 add human 1.555 s vs ~24 s judged") compare a single human
  `prove` against a judge run that includes far more. Append a correction at both
  places: the figure is withdrawn; a matched whole-file comparison suggests **1.2–5×**
  on the partial data, pending the goal comparison in `XIII.E3`. Do not delete the
  original lines — they are history.
- [ ] **XIV.0.2. Count correction.** `research/receipts/wip/ix58-checktime.partial.json`
  contains **25** task rows (18 ARM + 7 x86; `x86 secp256k1/neg` and `x86 sm2/neg`
  missing), not the 26 claimed in commit `f98dd9d` and in the earlier `XIII.E3` text.
  Verified with `grep -c '"dir"'` = 25. The commit message cannot be rewritten; this
  entry is the correction of record, and `XIII.E3` now says 25.

## XIV.A — Measurement integrity (defects in the machinery that just landed)

Source-inspection findings on `src/tools/dropin.rs`. The table can currently produce a
ratio, and exit 0, over populations that are not the same theorem.

- [x] **XIV.A1. Comparability must be a typed precondition.** Add
  `compare(human: &MeasuredProof, generated: &MeasuredProof) -> Result<MatchedComparison,
  ComparisonError>`. Constructing `MatchedComparison` must require: exact **theorem
  identity** (not architecture + directory + operation + curve), executable identity,
  identical measurement scope, identical checking configuration, and both judgments
  successful. `checktime_table` must not be able to exit 0 without a `MatchedComparison`
  per row it reports. — **Done (IX.60):** `dropin.rs` has `MeasuredProof`,
  `MeasurementScope`, `MatchedComparison`, `ComparisonError` and `compare`; a unit test
  refuses each variant. `checktime-table` exits 0 only when every row is matched.
- [x] **XIV.A2. `read_timed_tasks()` must retain acceptance.** It currently discards the
  exit status and the axiom-check result, so a failed or rejected measurement can still
  contribute a time. Carry both through and exclude rejected measurements. — **Done
  (IX.60):** `TimedTask` carries `theorem`, `exit`, `check_axioms`, `seconds` and
  `inferences`; `compare` refuses a row whose judgment failed.
- [x] **XIV.A3. Aggregates only over matched pairs.** When a generated time exists and
  its human counterpart does not, the generated time must not enter the numerator of any
  aggregate. Report `n` pairs, list unmatched measurements separately, and label every
  aggregate with its denominator. — **Done (IX.60):** aggregates take a
  `MatchedComparison`; the table prints `n`, the per-metric denominators, and every
  unmatched row with its refusal reason.
- [x] **XIV.A4. Counts recomputed from files, with a test.** The printed task count must
  equal the number of rows parsed; add a test that a pair file with a missing row cannot
  yield a ratio for it. — **Done (IX.60):** the counts section prints human rows parsed,
  generated rows parsed and the union; `a_missing_row_cannot_yield_a_pair_or_a_ratio`
  covers the missing-row case.

## XIV.B — Matched inference counts (the deterministic F3 number)

The reviewer's point, and it is right: wall-clock is the wrong metric and the comparison
already exists in deterministic form.

- [x] **XIV.B1.** Run the **human** tree under the axiom-stamp kernel and fill
  `inference_delta` for the human side. Every human row is `null` today while the
  generated side is counted (3,697,462,655 total; 681,707,202 for `arm/p521/add_p521`).
  — **Done (IX.60):** all 27 human tasks measured at `-j1 --force` under
  `.bench/hol-light-stamp2`; proof-body counts are in
  `research/receipts/ix60-matched-inference.json`.
- [~] **XIV.B2.** Publish per-task human vs generated primitive-inference counts and the
  total ratio. **Counts published, ratio refused** — `ix60-matched-inference.md` reports
  `n = 0 matched pairs` because the 27 obligations differ (`XIV.G1`). Human bodies total
  176,369,762; generated 3,697,462,655. Deterministic, machine-independent, and it needs no
  wall-clock comparison to be publishable. It still requires `XIII.E3`'s goal comparison
  before it is called a comparison of *the same obligation*. — **Done (IX.60):** per-task
  counts and side totals are published. **No ratio is published**: `XIII.E3` decided the
  obligations differ (incomparable), so a quotient would not be a proof-cost ratio.
- [~] **XIV.B3.** Separate **successful derivation** inferences from **discarded search**
  inferences (two counters, or a phase switch), so "derivation size" and "search cost"
  are distinct numbers. Today one counter mixes them (`XIII.E5`). — **Partial (IX.65):**
  a phase switch exists (`csat_mark`, `--mode phase`) and separates certified replay
  (30.35% of the six mandatory proofs) from the bridge's own work (67.85%) and from
  simulation (0.77%); it also confirms cadical's search contributes zero primitive
  inferences. It does **not** separate discarded search inside a successful HOL
  block, so "search cost" is still not a number. `arm/neg_p25519` now reads
  31,832,876 bridge/SAT of 33,981,360 with 2,510,071 replay:
  `research/receipts/ix65-same-obligation-and-phases.md`.
- [x] **XIV.B4.** Report the five costs separately, never as one number: search time;
  successful derivation size; discarded inference work; solver-free replay time; total
  construction + verification cost. Design principle: **expensive discovery is
  acceptable; paying the discovery cost again at verification is not.** — **Done
  (IX.60 §3):** the table reports the two measured costs and marks the other three
  *not measured*; the counter is explicitly labelled work, not proof-tree size.
  — **Five-cost split (IX.66), primitive inferences over the six mandatory proofs
  (total 993,573,633):**
  (1) **solver search time** — CaDiCaL's search costs **0** primitive inferences
  (the `dimacs_of_hol` → solver → LRAT-trim interval is 0 on all 30 closes); it is
  wall time only, still not quoted as a proof cost.
  (2) **successful derivation size** — **691,985,996 = 69.65%** of the total:
  bitblast 487,167,871 (49.03%) + definitionalise+clausify 178,375,918 (17.95%) +
  bridge rewrite 412,923 (0.04%) + residual 8,155,718 (0.82%) + the outer classes
  (initial/play/rewrite/finalisation) 17,873,566 (1.80%).
  (3) **discarded inference work** — still **not measured**: the counter is
  monotone, so retried/abandoned tactics inside `BITBLAST_THEN`'s `REPEAT`/`TRY`
  chain are inside (2). Open.
  (4) **solver-free replay time / size** — **301,587,637 = 30.35%** of the total
  (`LRAT_REFUTE_GEN` alone, 152.9 inferences per CNF clause, 17.5 per bit-blasted
  subterm). This is the cost that is paid again at verification, and it is the
  smallest of the three large classes, not the largest.
  (5) **total construction + verification cost** — unchanged at 993,573,633 for the
  six (aggregate ratio 70.87× the same-obligation human bodies).
  Consequence for the design principle: the replay is 30.35% while *building* the
  instance is 67.0%, so "paying the discovery cost again" is not the dominant term;
  building it is. `research/receipts/ix66-bridge-split.md`.

## XIV.C — Ordering resolution (supersedes `XIII.I`)

One reviewer would bring native foundations forward; the other would put the freeze and
certificates first. Both are satisfied by this order:

1. **Freeze closure — done** (`freeze-rust-closure-v5`). It gates every later claim, so it
   goes first, as the reviewer asks.
2. **XIII.A certificates as data** — strict gate 27/27, solver out of the check path.
3. **XIV.A + XIV.B** — comparability typed, then the matched inference table.
4. **XIII.E2 `add_p521`** under budget.
5. **XIII.G1 first new family** under freeze (and per `XIV.E`, prefer Montgomery or a loop
   over another linear operation).
6. **XIII.H1 (G1 bit-blaster soundness)** — brought forward: native foundations must not
   be starved while the HOL Light side is optimised.
7. `XIII.B3`, `XIII.B7`, `XIII.C5`, `XIII.D*` as small jobs in between.

Rationale for pushing B down: IX.57 §10 settled that the stamp is inherited from the
substrate and cannot discriminate inside HOL Light. Only B3 (negative control) and B7
(receipt correction) still protect something published. The rest matters again only in the
separate weak checker (`XIII.F3`).

## XIV.D — Certificate form is an upstream-acceptability question

- [ ] **XIV.D1.** A proof that needs an external SAT binary at check time is awkward to
  carry inside upstream s2n-bignum. Certificates-as-data (`XIII.A`) is therefore a
  precondition for upstream acceptability, not only a trust-surface improvement.
- [ ] **XIV.D2.** Emitted proofs must replay **without searching**: measure the search
  share of the inference count (`XIV.B3`) and drive it to zero in the emitted path.

## XIV.E — Coverage pressure

- [ ] **XIV.E1.** "Show me `montmul` or a loop." After the first linear family, prefer
  upstream's `ensures2` equivalence route with an agent-written scalar twin
  (`XIII.F8.2`), or a loop family with postcondition-prefix invariants (`XIII.F8.3`).
  Add/sub/neg are the shortest and easiest FC proofs in the library; 4% of the
  straight-line set does not move the frontier claim.
- [ ] **XIV.E2.** `XII.F5` made concrete: an agent extends the compiler to a family it
  has never seen, under a sealed freeze; metric is **time-to-coverage**, kernel sole judge.
- [ ] **XIV.E3.** Never conflate "replaced a core correctness proof" with "full upstream
  subroutine compatibility": `_SUBROUTINE_CORRECT` adds calling convention, return
  behaviour, stack handling and permitted register modifications. Preserve exact theorem
  semantics in every comparison (`XIII.G3`).

## XIV.F — The AI part must become measurable

- [ ] **XIV.F1.** Record plainly that the current 27 wins are compiler-driven with zero
  model calls. That is a differentiator only if the AI contribution is then measured:
  `XIV.E2`/`XII.F5` is that measurement.
- [ ] **XIV.F2.** State the honest comparison point: AlphaProof Code operates at
  libsignal scale; 27 theorems is small. The differentiators to defend are functional
  correctness of *optimised* crypto bignum code, drop-in acceptance by upstream, and the
  measurement discipline — not scale.

## XIV.G — Prove the obligation upstream actually states (new, from IX.60)

IX.60 measured that our drop-in proves `BIGNUM_*_CORRECT` while upstream's human file proves
`BIGNUM_*_SUBROUTINE_CORRECT`, and that all 27 contracts are **incomparable** (stronger
postcondition, weaker precondition, broader ABI frame on the human side). So "27/27 upstream
acceptance" is a per-file build result, not "the theorems upstream states are now proved".

- [x] **XIV.G1.** `_SUBROUTINE_CORRECT` is now **derived**, not re-proved (IX.64).
  `dropin premise` showed our built `_CORRECT` conclusion is the same statement as upstream's
  `_CORRECT` quotation for **27/27** (`PREMISE total=27 same=27`). Assembly now appends the
  upstream statement and derivation **verbatim from the pinned `.ml`**: on ARM
  `ARM_ADD_RETURN_NOSTACK_TAC`, on x86 `X86_PROMOTE_RETURN_NOSTACK_TAC` (NOIBT) then
  `ADD_IBT_RULE` (IBT). `dropin obligation --generated-theorem subroutine` reports
  `OBLIGATIONS total=27 same=27` against `.bench/human-baseline`; `check_axioms` passes. The
  derived step costs 0.33–0.36 s CPU (ARM), 0.39–0.42 s (x86 NOIBT) + 0.0002–0.0004 s (IBT).
  Firing control: replacing the tactic with `ALL_TAC` fails with
  `TAC_PROOF: Unsolved goals`.
- [x] **XIV.G2.** A same-obligation check-time or inference-count comparison is now
  **possible** (the contract is identical for 27/27) but has not been performed under a
  matched instrument: the proof bodies still differ, and IX.60's human timings were taken
  under a different instrument. `XIV.B2`'s ratio remains blocked on running both sides under
  one measurement pass. — **Done (IX.65) for n = 6 of 27:** both sides measured under
  `.bench/hol-light-stamp2` at `-j1 -B`, `_CORRECT` proof body only, typed `compare`
  accepts **6/6** pairs. **Per-task inference ratio median 60.20×, range
  12.93×–109.07×, aggregate 70.87×** (993,573,633 / 14,019,340). Wall seconds median
  106.97× (aggregate 90.59×). The derived `_SUBROUTINE_CORRECT` step (0.33–0.42 s,
  IX.64) is excluded from both sides. Human `_CORRECT` bodies are 0.72M–4.31M, not
  IX.60's 2.7M–17.6M, which was the whole-file body including decode and the wrapper.
  `research/receipts/ix65-same-obligation-and-phases.md`. Remaining 21 tasks not
  measured inside the 90-minute budget. A freeze `v6` is still required before any
  sealed run.
- [x] **XIV.G3.** Whole-tree build: the directory default target (`libs2nbignum.a`, assembly)
  is green under `make -j1` in both `arm/` and `x86/`, and upstream's `collect-specs.sh` now
  finds all 27 names. **Not** done: a full `make proofs` over the whole tree (452 ARM + 1758
  x86 obligations); only the 27 drop-in `.correct` files exist. `count-proofs.sh` reports
  `18/452` and `18/1758` and still lists the missing `_SUBROUTINE_SAFE` / `_WINDOWS` variants.
- [x] **XIV.G4.** Corrected position, now recorded in IX.64 and the addenda to IX.54/IX.56:
  the correctness obligations upstream states are **27 of 27**, not 0 of 27. The claim is
  scoped to `_SUBROUTINE_CORRECT`; safety and x86 Windows variants remain unproved here, and
  “upstream-compatible” is a contract/name/derivation claim, not proof identity. A new freeze
  (`v6`) is required before any sealed run cites the new file shape; none was created.

## XIV.H — Where the inference work actually goes (measured, IX.62)

IX.60 measured, over incomparable obligations: human proof bodies 2.7M–17.6M primitive
inferences per theorem (total 176,369,762), generated 18.2M–681.7M (total 3,697,462,655).
Generated is larger on every row, by 2.4M to 672.9M. Wall clock differs only ~1.2–5×, so the
generated path performs far more primitive work per second — consistent with the extra work
being cheap, uniform, mechanical steps.

- [x] **XIV.H1.** Test the hypothesis that the excess is **certified-SAT replay** (each resolved
  CNF clause becoming a primitive kernel inference) rather than simulation or rewriting. Split
  the counter by phase (simulation / conversion / LRAT replay / arithmetic) and report the
  share. If the hypothesis holds, the target is the replay, not the kernel. — **Done (IX.62):**
  measured on 8 of 27 tasks (all six mandatory plus the three x86 `neg` rows) with
  `inference_count` deltas around `LRAT_REFUTE_GEN`. Replay is **50–54%** of the primitive
  count on the arithmetic-heavy `add` proofs (`arm/add_p521` 365,948,270 of 681,707,202 =
  **53.68%**; `arm/add_p384` 50.28%; `arm/add_p256` 51.78%; `x86/add_p25519` 51.40%) and
  **11–32%** on the `neg` proofs. Across the measured set replay is **731,380,713 of
  1,459,446,909 (50.11%)**; those eight tasks cover 39.47% of the 3,697,462,655 benchmark.
  The hypothesis holds for the arithmetic-heavy rows: there the target is the replay, not the
  kernel. `other` still mixes search (`XIV.B3`).
- [x] **XIV.H2.** The x86 `neg` rows break the pattern (human 17.6M / 8.6M / 8.8M vs generated
  20.0M / 23.4M / 18.2M — nearly equal). Explain that row before generalising anything from
  the other 24. — **Done (IX.62 §4):** the three x86 `neg` rows are measured.

  | row | human | generated | replay | other | replay share |
  |---|---:|---:|---:|---:|---:|
  | `x86/neg_p25519` | 17,589,216 | 19,973,249 | 3,961,554 | 16,011,695 | 19.83% |
  | `x86/neg_p256k1` | 8,632,786 | 23,422,025 | 7,386,030 | 16,035,995 | 31.53% |
  | `x86/neg_sm2` | 8,761,732 | 18,230,539 | 3,478,365 | 14,752,174 | 19.08% |

  Replay is 3.5–7.4M in absolute terms because each `neg` proof makes only 2 small closes;
  non-replay work is flat at 14.8–16.0M, the same order as the human counts. The totals are
  near-equal because the SAT subproblem is small, not because the generated path is cheap.
  The p25519 row is also a human-side outlier: its human count is >2× the other two, while
  generated non-replay (16.01M) is *below* it. This explanation does **not** carry to
  `arm/neg_p25519` (other 31,471,289 vs human 4,077,908; replay share 11.24%) — that row
  needs `XIV.B3`.

## XIV.I — Freeze consequence of XIV.A/B (immediate)

`freeze verify` now **REFUSES** on `src/tools/dropin.rs` because IX.60 changed the producer
after `freeze-rust-closure-v5` was sealed. That is the closure working as designed.

- [ ] **XIV.I1.** Re-seal (`freeze-rust-closure-v6`) before the next sealed run, and state in
  the run record that v5 covers only its own commit.
- [ ] **XIV.I2.** A sealed run must refuse to start when the closure does not verify — check
  that this is enforced in the sealed path, not just available.

## XIV.J — IX.61: certificates-as-data measured, and the in-answer design is infeasible

Measured on `arm/curve25519/add_p25519` with a counting hook on a generated copy of
`csat_lib.ml` (tracked library untouched), reproduced twice:

```
CERTCLOSE n=1 key=4fe73a19… cnf_bytes=1022278 lrat_bytes=14453899 nvars=22018 nclauses=60831
CERTCLOSE n=2 key=6353242d… cnf_bytes=979842  lrat_bytes=9985779  nvars=21249 nclauses=58529
CERTCLOSE n=3 key=251e2a94… cnf_bytes=1022631 lrat_bytes=26518204 nvars=22023 nclauses=60845
CERTCLOSE n=4 key=bb8ee8c8… cnf_bytes=980384  lrat_bytes=14699743 nvars=21254 nclauses=58543
PRODUCER ok=true hyps=0 closes=4 seconds=24.058
lrat_total=65657625 cnf_total=4005135
```

- **4 SAT closes** per proof (within the budget), but **65,657,625 bytes** of LRAT —
  8.2× the 8 MB ceiling. The answer would grow from 103,709 B to ~66.4 MB.
- `clauses_of_term` reproduces `read_dimacs` exactly on all four instances
  (`clauses_equal=true`), so the in-memory CNF is provably the problem the solver saw.
- Encoding probes: binary caDiCaL LRAT 29,057,164 B (3.6×); deletion lines only ~8%
  (163,294 of 729,373); **backward-closure trim 22,765,098 B (2.8×)**. No encoding
  reaches the ceiling.

- [ ] **XIII.A8. Redesigned:** certificates as **content-addressed side artefacts** — the
  answer carries the digest, the judge performs exactly one digest-verified read-only
  read, the kernel still replays every step. This changes K2 from "no file I/O" to
  "no writes, no solver, one verified read", and must be declared as such. Do not
  quietly claim an inert answer under this design. — **Measured (IX.62 §3/§5):** the Rust
  backward-closure trimmer (`src/tools/lrat_trim.rs`) was replayed through the unmodified
  `LRAT_REFUTE_GEN` on 25 closes across 8 tasks: **574,132,604 → 194,382,037 bytes
  (2.95×)**, replay **731,380,713 → 301,972,701 inferences (2.42×)**, replay **68.378 s →
  29.312 s (2.33×)**; every trimmed replay reproduces the same theorem. Per-task trimmed
  floors on the `add` rows: `x86/add_p25519` 23,026,296 B, `arm/add_p256` 27,453,030 B,
  `arm/add_p384` 50,839,817 B, `arm/add_p521` 87,794,247 B — all above 8 MB. Trimming is a
  cheap partial win for a side-artefact; it does **not** make the in-answer design viable.
- [ ] **XIII.A9. Constraint found:** a *compiled* script cannot load a library at run
  time (`loadt` gives the type-checker no names), so any no-inline route requires the
  court to inject the support-library text into the template prefix and raise
  `inlined_locations`. Record that in the sealed-court protocol notes.
- [ ] **XIV.J1.** The 65.6 MB figure is the SAT ceiling in bytes as well as in inferences:
  it is direct evidence for `XIII.E4` (parametric lemmas) and `XIV.H1`, not only for
  packaging work. A 256-bit adder should not need 65 MB of resolution. — **Supported
  (IX.62):** the measured trimmed floor for `arm/add_p521` is 87,794,247 B (11.0× the 8 MB
  gate) and replay is 53.68% of its inferences; eliminating replay would leave
  315,758,932 (−54%), while trimming alone reaches 470,294,392 (−31%). The figure is
  evidence for `XIII.E4`, not for an encoding.

---

# XV — Intake 2026-11-25 (after `e5a8187`): two reviews

Nothing omitted. `XV.H` supersedes `XIV.C` as the order of execution.

## XV.0 — Corrections of record

- [ ] **XV.0.1. The check-time gap was under-stated, twice.** The same-obligation measurement
  (`ix65`) gives an inference ratio with **median 60.20×** (12.93×–109.07×) and a wall-clock
  median of 106.97×. The original "10–20×" was too low; the retraction to "1.2–5×" went the
  wrong way. Both earlier statements are recorded as superseded, not deleted.
- [ ] **XV.0.2. The upstream claim was overstated before IX.64.** "27/27 upstream acceptance"
  meant per-file builds of a *different contract*; upstream coverage in upstream's sense was
  **0/27** until `_SUBROUTINE_CORRECT` was derived (IX.64). That correction is now in
  `ix54-upstream-dropin.md`; keep it there.
- [x] **XV.0.3. IX.65's elapsed-time ratio (90.59×) is withheld** until the scope discrepancy is
  resolved: the human side brackets `_CORRECT`, while the generated seconds look like whole-file
  upstream timings. The **inference** ratio is the one that stands.
  — **Withheld (IX.68):** `TimeScope` is now carried as data, `compare` refuses a pair whose two
  sides' scopes differ, and the table refuses to aggregate mixed or unrecorded scopes by name.
  Re-running IX.65's command prints `SECONDS AGGREGATE REFUSED` and keeps 70.87× / 60.20× for
  inferences. Correction appended to `research/receipts/ix65-same-obligation-and-phases.md` §4;
  evidence in `research/receipts/ix68-ci-and-integrity.md`.
- [x] **XV.0.4. IX.66 close-count denominator.** The receipt says 30 SAT closes; the per-task
  table lists 21 closes across 22 attempts. The inference totals reconcile, but the denominator
  must be corrected and the receipts cross-checked.
  — **Corrected (IX.68):** §6 now reads "21 of 21 closes across 22 CSAT attempts", with an inline
  correction note and a §8 correction log. `src/bin/ix66_table.rs` regenerates the close column
  from the receipt JSON and asserts the printed total equals the sum of the per-task counts
  (`sat_closes == closes.len()` per task). Evidence in `research/receipts/ix68-ci-and-integrity.md`.

## XV.A — Restore CI and fix measurement integrity (blocking; do first)

- [x] **XV.A1. CI is red.** `ix66_markers_are_additive_and_the_normal_answer_is_recovered`
  (`tests/tools_emit.rs:429`) loads library sources that include `.bench/hol-light/Cadical/`,
  which does not exist offline. Fix with a synthetic fixture; the test must run offline and
  still prove `deinstrument ∘ instrument = id`. CI green is a precondition for everything below.
  **Done.** Fixtures under `tests/fixtures/emit/`; the real-source half skips with a printed
  reason. Also fixed the *local* red: the three answer-parity tests compared against git-ignored
  `.bench` references that went stale when the library changed, so the references are now
  tracked at `tests/fixtures/answers/` (27 files, 3.0 MB) and re-recorded deliberately with
  `TEKMERION_RECORD_ANSWERS=1`. Gate: 40 suites green, clippy and fmt clean, every run through
  `tools/cap.sh`.
- [x] **XV.A6. The IX.67 algebraic-close WIP is reverted from the tree** (preserved at
  `0336cb2`). It was unverified, gated to one task, and changed the library that every answer
  inlines, breaking the recorded baselines. It must be redone as a measured experiment with the
  `XV.B3`-style controls, not blessed as-is. The idea stands and is the `XV.D1` fallback.
  — **Fixed (IX.68):** the test now runs three halves — committed fixtures
  (`tests/fixtures/emit/`), the tracked `tools/*.ml` (always present), and the real inlined set
  (skipped, with a printed reason, when `.bench` is absent). Verified by renaming
  `.bench/hol-light/Cadical` for one test run; see `research/receipts/ix68-ci-and-integrity.md`.
- [x] **XV.A2. `MatchedComparison` must not be constructible without validation.** Its fields are
  public. Make them private, expose accessors, and make the only constructor be `compare`. Add a
  compile-fail test or a unit test that the fields are inaccessible outside the module.
  — **Fixed (IX.68):** fields private, `human()`/`generated()`/`theorem()`/`statement_digest()`
  read-only accessors, `compare` the only constructor. A `compile_fail` doctest plus a passing
  `no_run` companion pin the API shape (`cargo test --doc`), and unit tests re-check every
  precondition.
- [x] **XV.A3. Identity, not names.** The comparator matches theorem *names*. It must compare the
  full typed statement (or a canonical digest of it) plus the executable identity, so two
  different theorems with the same name cannot be paired.
  — **Fixed (IX.68):** `MeasuredProof` carries `statement_digest` and `executable`; `compare`
  refuses `StatementIdentityMismatch`, `StatementIdentityUnrecorded` and
  `ExecutableIdentityMismatch`. Tests construct two same-named theorems with different digests
  and require a refusal, both at `compare` and in the rendered table.
- [x] **XV.A4. Fix the trimmer's success reporting.** `src/tools/lrat_trim.rs` can report success
  with negative hints present or a non-empty root. It must refuse (and say why) in both cases;
  its own tests must cover the refusals, not only the success path.
  — **Fixed (IX.68):** `TrimRefusal` names `negative_hint` / `no_empty_clause` (and the rest) in
  the OCaml order and token; unit tests cover the refusals, and
  `tests/csat_trim_agreement.rs` runs the shipped OCaml trimmer on the same table and requires
  the same verdict and byte-identical output.
- [x] **XV.A5. Scope discipline in time comparisons.** Any timing comparison must carry its
  scope (`prove` only, whole file, or judge run) as data, and the table must refuse to aggregate
  rows with different scopes.
  — **Fixed (IX.68):** `TimeScope ∈ {prove_only, whole_file, judge_run, unrecorded}` is read from
  each row; mixed or unrecorded scopes refuse the seconds aggregate with a named row table while
  the inference counts stand. See `XV.0.3`.

## XV.B — Cone-of-influence slicing (the measured lever)

The decisive figure from `ix66`/`ix65`: removing the SAT close entirely caps the six-task total
at **17,873,566** inferences against **14,019,340** for the human proofs. Without the SAT close,
the generated proofs are already within **~1.3×** of hand-written ones. **The entire 60× gap is
the SAT close**, and the close scales with the hypothesis set (~28 inferences per bit-blasted
subterm) rather than with the goal (51 subterms at bridge entry).

- [x] **XV.B1. Slice before blasting.** Compute, from the simulator's dependency information
  (the W1 dataflow analysis already computes per-state dependencies), the cone of influence of
  the output words and the selection flag, and hand `CSAT_TAC` only the hypotheses in that cone.
  Everything else stays as an assumption but is not blasted.
  — **Done (IX.69), with the syntactic criterion the brief specified rather than the simulator
  dataflow:** `BRIDGE_CSAT_TAC` keeps the connected component of the goal under "shares a free
  variable with", transitively, and `MP`s only those hypotheses; the full set is retried on
  failure and reported as `SLICE fallback=… kept=k/n`. All 27 tasks slice; default fallback
  rate **0/27**. See `research/receipts/ix69-slicing.md`.
- [x] **XV.B2. What the slicing is must be** (typed and checkable):
  - **Sound**: a hypothesis may be dropped only if the conclusion's truth cannot depend on it;
  - **Deterministic**: the same input yields the same sliced instance;
  - **Measured**: subterms blasted, CNF clauses, inferences per class, and total, against the
    six-task baseline;
  - **Statement-preserving**: `equals_thm` against the current theorem, `check_axioms` passing.
  — **Done (IX.69):** soundness is the monotonicity argument (a smaller antecedent is stronger;
  the fallback retries the full set, so only incompleteness is possible); determinism is two
  independent runs of `x86_neg_sm2` with identical kept counts and total (15,444,067 twice); the
  measurement is `ix69-slicing.json` (blasted subterms, CNF clauses, bitblast/clausify/replay,
  total); statements are identical by construction (`prove` against the frozen `query`) and by
  byte-identical printed `_CORRECT`/`_SUBROUTINE_CORRECT` statements, `check_axioms` exit 0.
  `equals_thm` itself is not invoked in one session (that would replay both proofs); stated in
  the receipt.
- [x] **XV.B3. Refuse to be vacuous.** Firing controls: a hypothesis the output *does* depend on
  must not be sliced away (construct one deliberately and show it is kept); slicing everything
  must fail, not succeed.
  — **Done (IX.69):** `TEKMERION_SLICE_DEBUG=1` shows the 1227-subterm range condition `word_ult`
  is kept; `TEKMERION_SLICE_CONTROL=drop-largest` and `drop-non-eq` make 2 of 4 `add_p521` closes
  return SAT and fall back; `keep-none` fails all 4. The theorems are still accepted via the
  fallback, which is the point.
- [ ] **XV.B4. Target.** At least 30% off the six-task total (993,573,633) with all six still
  accepted; or a precise blocker with the numbers.
  — **Blocker (IX.69):** six-task total **993,573,633 → 944,555,577 (−4.93%)**, all six accepted
  and all 27 tasks accepted with 0 fallbacks, but far short of 695M. The entire saving is replay
  (301,587,637 → 253,762,924); bitblast moves by −0.19% and clausify by −0.14%, because the kept
  hypotheses are the obligation (two digit equations, the range condition, the 19 accumulator
  equations) and S3 shows dropping them fails the close. Replay is also non-monotone: the same
  `nonoverlapping_modulo` drop saves 32.9M on `add_p521` and costs 7.4M on `x86_add_p25519`.
  Reaching 695M needs the architectural route (XV.D / IX.67), not slicing.

> **Freeze note (do not create here):** `tools/bridge.ml` is inside the frozen runtime, so any
> sealed run after IX.69 requires a **`v6` freeze** — create and verify it first. IX.69 creates
> no tag and no freeze; see `research/receipts/ix69-slicing.md` §10.

## XV.C — Re-measure the certificate after slicing

- [ ] **XV.C1.** Re-measure LRAT bytes for `arm/add_p25519` and `add_p521` on the sliced instance.
  Current: 65,657,625 B and 8.2× the 8 MB budget; trimmed 2.95×.
- [ ] **XV.C2.** Decide the certificate design **from the measurement**, in this order:
  if a proper trim plus slicing gets certificates under ~1–2 MB, prefer the **in-answer,
  no-I/O** design; otherwise use the **content-addressed side artefact** (one digest-verified
  read) and say plainly that the guarantee is weaker.
- [ ] **XV.C3.** Record the decision with its numbers; do not re-open it without new data.

## XV.D — Hybrid closer (fallback if slicing is not enough)

- [ ] **XV.D1.** Where a modulus class has a cheap human-style arithmetic close (the `IX.67`
  route already attempted for `add_p25519`), use it; keep the certified SAT close as the
  fallback for classes the templates have not seen. The class-agnostic guarantee survives; the
  SAT cost is paid only where it buys generality.
  — **IX.70:** the `IX.67` route was recovered from `0336cb2`, repaired, and measured. It is
  sound and reaches a purely linear integer residue with **0** SAT closes, but it **does not
  prove**: `ASM_INT_ARITH_TAC` overflows on the first residual (verbatim goal in
  `ix70-algebraic-close.md`). The same system closes in isolation once the top carry's
  integrality is given (0.48 s), so the route is incomplete at the arithmetic engine, not wrong.
  It is shipped **opt-in only** (`TEKMERION_ALG_CLOSE=<task id>`); the default is the SAT path.
  A viable closer needs `XV.E1`, not a bigger `INT_ARITH` call. No inference total exists.
- [x] **XV.D2.** The choice must be visible in the emitted answer and recorded per theorem, so a
  reader can tell which route proved which theorem.
  — **IX.70:** the emitted tactic names its entry point (`BRIDGE_ADD_REF_TAC` vs
  `BRIDGE_ADD_REF_ALG_TAC`); the switch is read at emission and the assembled proofs differ in
  exactly that token (C1). Since the algebraic route does not prove, the recorded state is: all
  27 tasks on the SAT path by default; the algebraic entry only under the environment switch.

## XV.E — Generic carry-conservation certificate (the deeper target)

- [ ] **XV.E1.** Establish the local radix identity `a_i + b_i + c_i = s_i + B·c_{i+1}` at
  `B = 2^64`, compose it over `n` limbs into
  `Σ B^i(a_i+b_i) = Σ B^i s_i + B^n c_n − c_0`, and prove the composition **once**, parametric
  in width, limb count and modulus.
  — **IX.70 diagnosis (this is the load-bearing step).** The composed identity is already valid
  over the reals (intermediate carries cancel symbolically), but HOL Light's `INT_ARITH` is a
  real-linear procedure: the *only* integrality-dependent atom is the top carry of each chain,
  and supplying it by case split is exponential in the number of carries (all seven: memory
  exhaustion). A parametric composition lemma would make each chain one rewrite, remove the
  per-limb system from the decision procedure, and let `IX.70`'s route close; this is the
  intended replacement for `XV.D1`'s case split.
- [ ] **XV.E2.** Instantiate it cheaply per theorem and show it removes the corresponding
  hypotheses from the blast instance (the `XV.B` slicer and this lemma are complementary).
- [ ] **XV.E3.** Combine with range and modular-correction certificates. This is the route to
  the Arithmetic Structure Compiler, and the first place where Tekmerion would be *inventing*
  reusable mathematics rather than replaying it.

## XV.F — Upstream compatibility: what is and is not established

- [ ] **XV.F1.** Established: same contract, same theorem names, whole-tree default `make` green
  in both arches, one `ALL_TAC` control failing.
- [ ] **XV.F2.** Not established, and must be said wherever the result is cited: the statement
  comparison is **text-based after notation normalisation** (not a kernel-level identity check);
  the assembler **copies upstream's derivation blocks** rather than inventing them; `_SAFE`,
  `_NOIBT_WINDOWS_*` and `_WINDOWS_*` obligations are out of scope; a full `make proofs`
  (452 ARM + 1758 x86 obligations) has never been run.
- [ ] **XV.F3.** Where the comparison can be made kernel-level (both theorems in one session,
  `equals_thm`), do that instead of text comparison — at least for a sample.

## XV.G — Standing open items that must not be lost

- [ ] **XV.G1.** Solver-free replay of a real machine-code proof (still open; `XV.C2` decides
  the design).
- [ ] **XV.G2.** `XIII.H1` G1 bit-blaster soundness (the central hypothesis of the native
  soundness theorem).
- [ ] **XV.G3.** Multiplication / Montgomery coverage (`XIV.E1`), and `XII.F5`'s agent-extends-the-
  compiler experiment.
- [ ] **XV.G4.** Freeze **v6** before any sealed run (`XIV.I1`); the seal is stale because
  `src/tools` and `tools/` moved.
- [ ] **XV.G5.** `XIII.B3` (type-definition negative control) and `XIII.B7` (receipt wording) —
  small, still open from the earlier intake.

## XV.H — Order of execution (supersedes `XIV.C`)

1. **XV.A** — CI green and measurement integrity. Nothing else is trustworthy while CI is red.
2. **XV.B** — cone-of-influence slicing, measured on the six-task baseline.
3. **XV.C** — re-measure certificates, then choose the design from the data.
4. **XV.D** — hybrid closer only if slicing leaves the gap.
5. **XV.E** — carry-conservation certificate, parametric, proved once.
6. **XIV.G1**'s remaining scope → **XV.G3** first new family, then **XV.G2** native G1.
7. Freeze **v6** immediately before the first sealed run, not before.

---

# XVI — Intake 2026-11-25 (after `7f426bc`): two reviews

Nothing omitted. `XVI.G` supersedes `XV.H`. Two reviews: the first is the one already
absorbed as `XV` (slicing, integrity, carry-chain abstraction); the second adds findings
from the IX.69/IX.70 commits.

## XVI.0 — Status of the measurements they correct

- [x] **XVI.0.1.** The check-time gap is **60.20× median in inferences** (12.93×–109.07×), not
  the original "10–20×" nor the retraction's "1.2–5×". Both earlier figures are superseded;
  neither is deleted.
- [x] **XVI.0.2.** Upstream coverage in upstream's sense was **0/27** before IX.64 and is
  **27/27** now (same contract, same theorem names, whole-tree default `make` green, with an
  `ALL_TAC` control failing).
- [x] **XVI.0.3.** Slicing is measured: **49,018,056** inferences removed (4.93% of the six-task
  total), 27/27 accepted, 0 fallbacks — and it **does not** reach the 30% target, for a measured
  reason: the kept hypotheses are the arithmetic obligation. One x86 add proof *regresses* by
  6.58%; smaller instances do not imply shorter proofs.
- [x] **XVI.0.4.** The algebraic route (`BRIDGE_ADD_REF_ALG_TAC`) is **sound but incomplete**:
  0 SAT closes, aborts with `Stack_overflow` in `ASM_INT_ARITH_TAC` at 2,620,473 inferences. The
  2.62M is **not** a proof cost and must never be cited as an improvement.

## XVI.A — CI is still red, and the fix must be real (do first)

Three tests fail in CI because they need `.bench/problems.json` (and the tree around it):
`arm_answers_are_byte_identical`, `x86_answers_are_byte_identical`,
`sealed_freeze_verification_end_to_end`.

- [x] **XVI.A1.** Commit **input** fixtures so the offline suite genuinely exercises what it
  claims: the task entries the emitter needs (`query`, `inlined_locations`, family/op) for the 27
  tasks, in `tests/fixtures/problems.json`, with a test asserting the fixture and the live
  `.bench/problems.json` agree on those 27 entries whenever `.bench` is present.
- [x] **XVI.A2.** No silent early returns. If a fixture is missing, the test must **fail** with a
  message saying which fixture, not skip. Skips are only allowed for things that are genuinely
  machine-specific (e.g. a different HOL Light build), and must print a reason.
- [x] **XVI.A3.** Reproduce the CI condition locally (bench-free copy of the tree) and show
  `cargo test --offline` green there, then push and confirm the workflow.

**Receipt:** `research/receipts/ix72-offline-inputs.md`. The corpus is only the
first missing input: the emitter also needs the 27 ELF objects and the three
Cadical sources, so those are committed too (`tests/fixtures/bench/`). The
override is `TEKMERION_PROBLEMS` (`src/tools/emit.rs`), a fallback that never
shadows a live `.bench/problems.json`; XVI.C1 must bind it once XVI.C has a
variable list (none exists yet).

## XVI.B — Identity-bound comparison must be distinguishable from a diagnostic one

- [x] **XVI.B1.** `comparison_reasons()` is split into `diagnostic_reasons()` (identity evidence that
  is present must agree; missing evidence is recorded) and `identity_reasons()` (missing evidence
  on either side refuses). A pair can no longer be matched on names alone and remain a result.
- [x] **XVI.B2.** `DiagnosticComparison` (legacy, carries `IdentityEvidence` naming exactly what is
  missing) and `IdentityBoundComparison` (statement digest and executable identity present and
  equal on both sides, plus successful judgments and identical scopes). Only the second backs an
  aggregate; tables print the type of every row and the missing evidence; `--diagnostic` prints
  the legacy aggregate with `NOT A RESULT` on every line.
- [x] **XVI.B3.** The `sweep`/`measure` producer now writes `statement_digest` (SHA-256 of
  `canonical_term(extract_conclusion(..))`, the existing canonical typed text) and the executable
  identity (`arch.dir.op`), so the identity-bound path runs on real rows. Exercised against real
  `.bench/ix69` `.correct` files in `a_producer_digest_is_stable_on_real_correct_files`.
- [x] **XVI.B4.** Tests: same name/different statement refused; one-side-missing digest yields a
  diagnostic with the missing evidence while `bind_identity` refuses; a legacy aggregate prints
  `NOT A RESULT`; a fully bound pair yields the identity-bound type.

**Receipt:** `research/receipts/ix74-identity-and-config.md`.

## XVI.C — The freeze must bind the effective execution configuration

`TEKMERION_ALG_CLOSE` and `TEKMERION_SLICE_CONTROL` change the reasoning path without changing any
source file or executable hash.

- [x] **XVI.C1.** The effective configuration is recorded in the `sweep`/`measure` run records and
  in the freeze closure (`configuration.env` for every declared variable, `configuration.flags` for
  `mode`/`stamp`/`force`/`slice`).
- [x] **XVI.C2.** `src/tools/config.rs` declares every `TEKMERION_*` variable read by the binary,
  the OCaml bridge and the fixture recorder; three tests grep those sources and fail if the
  declaration and the source disagree.
- [x] **XVI.C3.** `freeze verify` refuses when the recorded configuration differs from the live
  environment, naming the variable. Old manifests without the field still verify under their own
  scope with a printed warning.
- [x] **XVI.C4.** Firing controls: setting a recorded-unset variable and unsetting a recorded-set
  one both refuse, naming the variable.

**Receipt:** `research/receipts/ix74-identity-and-config.md`; manifest
`research/receipts/freeze-closure-v6.json` (no tag created).

## XVI.D — Resource failure must not trigger a more expensive retry

`BRIDGE_CSAT_TAC` propagates `Out_of_memory` but its general handler retries the full hypothesis
set for other exceptions — potentially including stack overflow, which may itself be a resource
failure.

- [x] **XVI.D1.** Classify failures into distinct outcomes: **mathematical** (closed: false),
  **unsupported input**, **resource exhausted** (OOM, stack overflow, timeout), **internal error**.
  Each has its own message and its own policy.
- [x] **XVI.D2.** Resource exhaustion must **not** fall back to a larger instance. It reports and
  stops; a retry at a larger budget is a decision, recorded.
- [x] **XVI.D3.** Tests for each class, including a synthetic stack-overflow-shaped failure.

## XVI.E — `cap.sh` is per-process; say so and close the gap

`RLIMIT_AS` caps one process, not a tree.

- [x] **XVI.E1.** State the limitation in `AGENTS.md` §7a and in `tools/cap.sh`'s header.
- [x] **XVI.E2.** Add the aggregate guarantee that is actually available here: run the capped
  command in its own process group, kill the group on overrun, and report peak RSS for the tree
  (`/usr/bin/time -v` or `wait4` rusage) into the receipt. Where a cgroup is enforceable, use it;
  where it is not (this host: `MemoryMax` is ignored), say so and fall back to the group kill plus
  the documented per-process cap.
- [x] **XVI.E3.** A control showing the tree is killed, not just the leaf: a parent that spawns a
  child allocating past the cap must end with both gone.

## XVI.F — The carry-conservation certificate compiler (the real target)

The reviewers converge: stop optimising SAT preprocessing; eliminate intermediate variables
**before** the solver, with three certificate types.

- [~] **XVI.F1. Conservation certificate.** Local step `a_i + b_i + c_i = s_i + B·c_{i+1}` at
  radix `B = 2^k`; composed by induction over the limb list to
  `Σ_{i<n} B^i(a_i+b_i) = Σ_{i<n} B^i s_i + B^n c_n − c_0`. Proved **once**, parametric in radix
  and limb count. Intermediate carries cancel exactly; that cancellation is the point.
  *IX.78: the elimination is implemented and per-theorem (`bridge_gauss_jordan`); it produces the
  corrected chain's aggregate but destroys the raw one when a `2^256`-coefficient carry is a
  pivot. Composing the raw aggregate as a projection is the remaining step.*
- [~] **XVI.F2. Range certificate.** Bounds on inputs, outputs and carries — including the
  integrality `c_i ∈ {0,1}` that `INT_ARITH` cannot infer, which is exactly what made IX.70
  overflow.
  *IX.78: the exact rational interval relaxation (`bridge_carry_value`) decides the top carry's
  integrality per theorem and the fact is emitted and proved (`&(bitval carry_s8) = &0`); the
  other six carries are correctly reported undetermined. A `BOUNDER`-style engine that emits the
  bounds rather than re-deriving each by `INT_ARITH` is still missing.*
- [ ] **XVI.F3. Selection certificate.** That the conditional correction yields the canonical
  residue (the `word_ult`/mask/csel step), as its own certificate rather than a blast.
- [~] **XVI.F4. Reconstruction.** Recover the premises of (F1)–(F3) automatically from the
  simulator's accumulator relationships, for ARM and x86, without per-curve or per-width lemmas.
  *IX.78: the linearisation, elimination target and interval facts are read from each theorem's
  own terms; no curve, width or limb count is named. Not yet run on x86 or on sub/neg.*
- [ ] **XVI.F5. First complete solver-free proof.** `arm.bignum_add_p25519`, same theorem,
  **0 SAT closes**, `check_axioms` passing, with the inference number reported against
  111,501,987 (sliced SAT) and 1,848,494 (human `_CORRECT`).
  *IX.78 attempted it: 0 SAT closes, 0 solver invocations, the statement unchanged, and the
  oracle emits the correct fact — but the close still fails (`linear_ineqs: no contradiction`)
  because the raw chain aggregate is absent. No inference total. See
  `research/receipts/ix78-oracle-arithmetic-close.md` §6 for the exact missing mechanism.*
- [ ] **XVI.F6. Generalise to the nine additions**, then the same representation for
  multiplication, squaring and Montgomery reduction.
- [ ] **XVI.F7. One representation, two checkers.** The same certificate objects must be
  consumable by the native TNPS checker, so the HOL producer and the native kernel converge
  (`XV.G2`, `XIII.H`).
- [ ] **XVI.F8.** A lemma whose omission does not break the proof is not being used: every
  certificate type needs its own firing control.

## XVI.G — Order of execution (supersedes `XV.H`)

1. **XVI.A** — CI green with real input fixtures. Nothing else is credible while CI is red.
2. **XVI.D + XVI.E** — failure taxonomy and the cap's documented scope. Small, and they protect
   every later run.
3. **XVI.B + XVI.C** — identity-bound comparison and configuration-bound freeze. These decide
   which numbers may be called results.
4. **XVI.F1–F5** — conservation certificate, range certificate, selection certificate, automatic
   premise reconstruction, first solver-free proof.
5. **XVI.F6–F8** — generalise across the nine additions, then multiplication/Montgomery, then the
   TNPS representation.
6. Freeze **v6** immediately before the first sealed run that cites any of this, with the
   configuration of `XVI.C` bound.

---

# XVII — Intake 2026-11-25 (after `2b38c37`): decision, review, kernel charter, inversions

Nothing omitted. `XVII.D` supersedes `XVI.G`.

**Decision recorded:** Tekmerion builds its own kernel (TNK) — a logical framework with verified
computation at its core, in which HOL is the *first guest theory*, not the ceiling. The charter is
a project document; this section is the queue it implies.

## XVII.0 — Corrections and standing negatives

- [x] **XVII.0.1.** Slicing was my recommendation and it was wrong: **4.93%**, not 30%. All of the
  saving is replay; the kept hypotheses *are* the obligation. The non-monotonicity (one dropped
  hypothesis saves 32.9M on `add_p521`, costs 7.4M on `x86_add_p25519`) is recorded so nobody
  tunes the SAT path further without knowing it.
- [x] **XVII.0.2.** The algebraic route is sound, needs **0 SAT closes**, and fails at
  `Stack_overflow` in `ASM_INT_ARITH_TAC` because `INT_ARITH` is real-linear and cannot use carry
  integrality. The isolated probe closed the same system in **0.48 s** once the top carry was
  given as `0 \/ 1`. That is the most informative number of the week.
- [x] **XVII.0.3.** A partial carry-range attempt is preserved at branch
  `wip/xvi-f2-carry-range` (tests red, library text changed, fixtures not re-recorded). It is not
  cited anywhere. Staging stays green.
- [ ] **XVII.0.4.** The reviewer's allocation is noted as guidance, not measurement: 40% arithmetic
  structure compiler, 30% native foundations, 20% new-family coverage, 10% infrastructure.

## XVII.A — Integrity defects that block any claim (do first)

- [x] **XVII.A1. CI is red again**: three ARM tests fail because the runner's `objdump` cannot
  disassemble AArch64 (`can't disassemble for architecture UNKNOWN`). `find_objdump()` in
  `src/tools/disasm.rs` takes the first executable found without checking capability.
  - Provision `binutils-aarch64-linux-gnu` in CI;
  - **and** fix the architecture: probe the candidate disassembler on a known instruction from a
    committed fixture object, accept only a tool that decodes it, and bind the tool's version and
    digest into the evaluation environment record.
  - Do not weaken the tests.
- [x] **XVII.A2. Executable identity must be digests, not a label.** `dropin.rs` currently writes
  `format!("{arch}.{dir}.{op}")`, so two different objects with the same tuple compare equal.
  Introduce `ExecutableIdentity { architecture, object_sha256, code_sha256, semantics_sha256 }` and
  require it in `IdentityBoundComparison`.
- [x] **XVII.A3. The sealed-run guard must enforce the configuration check.** `freeze verify` does;
  `guard::verify_freeze` does not. Integrated it, with a firing control on the sealed emit
  (`research/receipts/ix77-enforcement.md` §1).
- [x] **XVII.A4. Freeze tag validation must bind the whole committed manifest**, not selected
  fields: a tag whose committed manifest differs anywhere now refuses, naming the first differing
  field (`research/receipts/ix77-enforcement.md` §2).
- [x] **XVII.A5. Only recognised mathematical outcomes may retry.** Generic OCaml `Failure` is no
  longer assumed mathematical; an explicit allow-list gates the retry and everything else is
  `internal` (`research/receipts/ix77-enforcement.md` §3).
- [ ] **XVII.A6. Statement identity from typed terms, not printed text.** Introduce a canonical
  serialization of the actual kernel term (structure, types, sharing-independent) and use it for
  `statement_digest`; keep the printed form only for display.
- [ ] **XVII.A7.** Regenerate `freeze-closure-v6.json` after A3–A6, then seal `freeze-rust-
  closure-v6` as a recorded decision.

## XVII.B — The Tekmerion kernel charter (TNK)

The design: framework core + guest theories + verified computation primitives + content-addressed
proof objects + a kernel that certifies its own binary.

- [ ] **XVII.B1. Proof-object import from HOL Light** — the 27 generated and the matching human
  proofs. This is the differential corpus; nothing downstream means anything without it.
- [ ] **XVII.B2. Framework core + HOL guest in Rust.** Replay every imported object; agree on every
  conclusion with HOL Light.
- [ ] **XVII.B3. `CIRCUIT` (promote the existing native LRAT checker) + `LIA`** (Farkas + cuts).
  Re-check the six tasks and measure against the **14,019,340** human-step figure.
- [ ] **XVII.B4. `EVAL` + `STEP` on AArch64.** One routine proved end to end by evaluation, not
  rewriting.
- [ ] **XVII.B5. Lean soundness per rule**, bit-blaster first (this is `G1`).
- [ ] **XVII.B6. Finitary arithmetic guest + checked translation**; the first foundation stamp that
  discriminates (in HOL Light everything carries all three axioms — measured).
- [ ] **XVII.B7. `IDEAL`** (ideal-membership certificates) and the first Montgomery-family theorem.
- [ ] **XVII.B8. Aeneas refinement of the rules**, then self-certification of the kernel binary by
  Tekmerion's own machine-code method — the first kernel whose shipped bytes are proved directly,
  with no compiler in the trust base.
- [ ] **XVII.B9. Kernel targets, all measured on statements HOL Light also accepts**: six-task
  steps **< 14.0M**; all 27 accepted by both from the same statement bytes; corpus re-check faster
  than HOL Light's rebuild, DAG-parallel and deterministic; checks/second for agents published;
  discriminating stamps; unproved trusted-base lines fewer than HOL Light + OCaml.
- [ ] **XVII.B10. Agent-native API**: structured rejection data (failing rule, failing subterm) as
  a first-class channel, millions of checks per second.
- [ ] **XVII.B11.** DAG-parallel, deterministic checking.
- [ ] **XVII.B12. Certificates carry five identities**: judgment, derivation, theory, subject,
  attestation. A changed checker executable renews the attestation; it does not make a theorem
  false.

## XVII.C — Inversion programme (13 items; ★ = chase first)

- [ ] **XVII.C1 ★ Assembly as hardware.** Extract the straight-line routine's dataflow graph as a
  circuit and verify it algebraically against the spec polynomial (Gröbner / polynomial-calculus
  certificates, the Biere–Kauers–Kaufmann line, verified far beyond 256 bits), checked by `IDEAL`.
  Symbolic simulation drops out and Montgomery multiplication becomes a circuit problem. **Prior-art
  sweep first** — this has neighbours in the literature.
- [ ] **XVII.C2. Infer the spec from the binary, then diff it against the stated spec.** Every
  disagreement is a spec bug, a hidden precondition or a documented quirk. Verification normally
  runs spec → code; this audits the spec.
- [ ] **XVII.C3. Prove the mutants wrong.** Generate one-edit mutants per routine (flipped carry,
  wrong limb, off-by-one shift) and require a concrete counterexample theorem for each. Publish a
  **proof mutation score** per routine. Prior-art sweep first.
- [ ] **XVII.C4. Prove ARM ≡ x86** by normalising both dataflow graphs; one spec proof then covers
  both ISAs and divergence is caught directly.
- [ ] **XVII.C5 ★ Prover as optimiser.** Combine SLOTHY/CryptOpt-style search with proofs to emit
  *faster* assembly than s2n-bignum's hand-tuned routines, proved. The claim is measured in cycles
  against AWS's own code. Verify the optimiser's move set, not each output.
- [ ] **XVII.C6 ★ Universal second judge.** Import Lean proof objects and re-check other people's
  proofs, starting with AlphaProof Code's libsignal work. An independent kernel re-checking
  DeepMind's Signal proofs is a frontier event and makes dual acceptance a public utility.
- [ ] **XVII.C7. ZK-wrapped verdicts.** STARK of the kernel's execution so a vendor can prove a
  closed-source binary meets a public spec without revealing it. The proof object stays the
  authority; the ZK layer only lets a verdict travel.
- [ ] **XVII.C8. Proof cost as a change detector.** Nightly regeneration with per-theorem
  inference-count time series: a jump on a new binary version means its semantics or structure
  changed even when the proof still passes.
- [ ] **XVII.C9. Mine lemmas from kernel traces, not tactics.** 3.7 billion inferences on record;
  find repeated sub-derivations and promote them to lemmas. Prior-art sweep first.
- [ ] **XVII.C10. Failed LLM proofs as a fuzz corpus.** Realistic malformed inputs, unlike random
  fuzzing.
- [ ] **XVII.C11. Depend on instruction *properties*, not the whole ISA model.** Record which
  properties each proof used, so a model revision revokes only the proofs whose properties changed.
- [ ] **XVII.C12. Publish each proof's shadow.** Generate what the theorem does *not* guarantee
  (unconstrained registers, frame gaps, timing) mechanically.
- [ ] **XVII.C13. Proof shape as a constant-time screen.** Shape invariance is not a proof, but it
  is a cheap filter for which routines deserve a relational proof.

## XVII.D — Order of execution

1. **XVII.A** — CI and the six integrity defects, then v6 regenerated and sealed. Nothing is
   citable until the identity and configuration guarantees are enforced rather than recorded.
2. **The decisive experiment** (from `XVI.F`, still the gate): oracle-guided arithmetic close on
   `arm.bignum_add_p25519`. SAT is used **offline at emission time** to identify which carries are
   undetermined and which bounds pin the rest; each needed fact is emitted as a small lemma proved
   by bounds plus integrality (the `REAL_INTEGER_TAC`/`BOUNDER_TAC` idiom); the residual goes to
   linear arithmetic with those facts supplied, so nothing case-splits. Measure inferences against
   111,501,987 (current) and 1,848,494 (human). If it lands near the human figure, that is also how
   double, half and the rest come in cheaply.
   *IX.78 outcome: built, sound, **not closing**. An exact rational oracle (linearise → eliminate
   the non-goal carries by integer-preserving Gauss–Jordan → interval relaxation with one
   relational hypothesis → emit and prove the decided fact) runs per theorem, performs 0 SAT
   closes and 0 solver invocations, and correctly decides `&(bitval carry_s8) = 0` while leaving
   the other six carries undetermined. The close still fails in 0.65 s with
   `linear_ineqs: no contradiction`: the **raw chain aggregate** (`M + N = W_raw + 2^256·c8`) is
   missing, because Gauss–Jordan destroys it when the top carry is a pivot with a `2^256`
   coefficient. Next step: project that definition onto the goal's atoms and emit *both* chain
   aggregates (the `GEN_DECARRY_RULE` step), then close — not a larger `INT_ARITH` call. Details:
   `research/receipts/ix78-oracle-arithmetic-close.md`.*
3. **XVII.B1–B3** — proof-object import and the framework core, with `CIRCUIT` + `LIA`.
4. **XVII.C1** after its prior-art sweep — the route to the Montgomery families.
5. **XVII.C5, XVII.C6** — the two claims engineers cannot argue with.
6. **XVII.B4–B8** — `EVAL`/`STEP`, Lean rules, finitary guest, `IDEAL`, self-certification.
7. **XVII.C2, C3, C9, C11–C13** in whatever order the measurements favour.

## XVII.E — Follow-ups from IX.78 (2026-11-25)

The oracle-guided close was built and is sound; it fails at one precisely identified step. Two
items come out of it, one of them a process defect that matters more than the mathematics.

- [ ] **XVII.E1. Gate that an emitted answer parses.** XVI.D's classifier returned polymorphic
  variants, which `camlp5r` cannot parse, so **no emitted answer built at all** — and every test
  passed, because no test elaborates a real answer with the real toolchain. Add a cheap gate: for
  one representative answer, run the bench's own syntax/type check (`ocamlc -i` on the inlined
  answer, as `combine-answer-and-setup.py` does) in CI. It needs OCaml, not HOL Light, so it can be
  offline. This class of regression must be caught by the suite, not by a 49-second proof.
- [ ] **XVII.E2. The missing step is the second aggregate.** The residual carries the corrected
  chain and `c8 = 0`, but not the raw chain aggregate `M + N = W_raw + 2^256·c8`; `c8`'s definition
  encodes it with accumulated `2^256` coefficients, and handing those to `INT_ARITH` fired the 12 GB
  cap (recorded, not retried). The next attempt is the `GEN_DECARRY_RULE`-style projection: project
  `carry_s8`'s definition onto the goal's atoms and emit **both** aggregates, so the residual is
  linear without the coefficients. Then re-measure against 111,501,987 and 1,848,494.
- [ ] **XVII.E3. A certificate-producing bounder.** The oracle currently *proposes* facts proved by
  `INT_ARITH` from the definition's cone; for the generic mechanism the bound must come with a
  checkable certificate (the `XVI.F2` range certificate), not from a decision procedure's success.
