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

- [ ] **C1.** One generic bridge lemma per linear operation shape, from the `num`/`MOD`
  specification to a word-level circuit statement. Modulus-agnostic: proved once.
- [ ] **C2.** `BITBLAST_THEN` to a propositional goal, `CADICAL_PROVE`, and **kernel LRAT
  replay**. `Examples/bitblast.ml` defines `BITBLAST_CADICAL` only inside a commented-out
  block; s2n-bignum uses BDD `BITBLAST` in 55 files and CaDiCaL in none. Connect them.
- [ ] **C3.** Measure close time per theorem against `timeout-map.json` **before** any claim.
  If 256-bit replay is too slow, split per limb with the carry as the interface.
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

- [ ] **D1. Carry-liveness bug in `derive_steps.py`.** `_carry_in_is_live()` stops at a carry
  *reader*; reading does not kill the flag, so an intervening `ADC` can wrongly exclude a later
  `ADCS`. Fix the reaching-definition analysis and add a regression test (`ADDS; ADC; ADCS`).
  Later: condition-specific NZCV dependencies (CS/CC/HI/LS use C; EQ/NE use Z).
- [ ] **D2. `w2_witnesses.py` invariant field is the postcondition**, not an inductive loop
  invariant. Do not advertise loop verification until L-FIT actually synthesises one.
- [ ] **D3. Modulus class as a typed witness field** (NIST/Solinas, pseudo-Mersenne,
  Montgomery-friendly), with one close path per class.
- [ ] **D4. Two held-out protocols:** LOCO *within* a class, and **leave-one-class-out**. The
  second is the one a reviewer will respect.
- [ ] **D5. Residue/quotient certificate (RQ) alternative.** For non-linear families, or where
  bit-blasting is too slow: derive `input = output + p*q` from machine equations and prove
  `0 <= output < p` separately. Untrusted algebra proposes; the kernel checks.
- [ ] **D6. GitHub CI** for the generator and regression tests (currently all Python/OCaml
  checks are manual).
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
