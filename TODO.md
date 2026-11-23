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
- [ ] **V8.22. Encode flag-only `ADCS`/`SBCS XZR` instructions.** This is the whole remaining
  failure class (`add_p256k1`, `add_p384`): both select the correction with the **Z flag of a
  chain of flag-only instructions** (`CMN`/`CMP` + `SBCS XZR`/`ADCS XZR`, then `CSEL`/`CSETM NE`).
  `derive_steps` excludes `XZR` destinations and `ACCUMULATE_ARITH_TAC` needs a register read
  equation, so that flag is a free boolean and `cadical` returns exit 10 (SATISFIABLE) on a
  residual instance that is otherwise the pure `output = ref_add` word equation. Needs an
  accumulator relation per flag-only instruction (a `word_carry`-style hypothesis), still
  bit-level and curve-free. Do not "fix" it by reinstating a transcribed real-arithmetic flag
  subgoal.
- [ ] **V8.23. `add_p521` headroom.** 95.864 s of the 120 s per-task budget (0.80) on the
  reference route; `sub_p521` is 35.088 s. Cheap headroom would be a shorter bit-blast for the
  576-bit chains, not a longer timeout.
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

- [ ] **IX.50. x86 simulation + flag mapping.** Re-run the step derivation on x86 objects and map
  flags (x86 `sbb` carry vs ARM's inverted no-borrow). Develop only on x86 tasks that are **not** the
  nine held-out counterparts (e.g. x86 P-384/P-521 and other families).
- [ ] **IX.51. Freeze, tag, then run.** After IX.50: write the freeze manifest, commit, **tag and push**
  the tag, then evaluate the nine untouched x86 counterparts **once** in an isolated filesystem.
  Tag-after-run does not count as a freeze.
