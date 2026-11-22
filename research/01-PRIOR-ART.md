# Prior-art audit

**Method.** I cloned `awslabs/s2n-bignum`, `kings-crown/s2n-bignum-bench`,
`hol-light-family/TacticTrace`, and fetched `jrh13/hol-light/fusion.ml` over the network,
then measured rather than trusted. Repository facts are at the commits listed in
`sources.md`. Literature facts are from the arXiv API and the pages cited there. Where I
could not check something, it says so.

The rule I applied: a mechanism is only `PLAUSIBLY_NOVEL` if I searched for its nearest
neighbours and can name them. "I did not find it" is not evidence; "here is what I found
instead, and why it is different" is.

---

## A. Verified repository facts

| Fact | Value | How checked |
|---|---|---|
| HOL Light kernel file | `fusion.ml` = 680 lines, **400** non-blank non-comment | `wc -l`, `grep -vE '^\s*$|^\s*\(\*'` |
| Kernel primitives | 10: `REFL TRANS MK_COMB ABS BETA ASSUME EQ_MP DEDUCT_ANTISYM_RULE INST_TYPE INST` | `grep -nE '^\s*(let\|and)\s+(...)'` |
| Axiom introducer | `new_axiom` (l. 575), `new_basic_type_definition` (l. 617) | grep |
| s2n-bignum kernel claim | "~400 lines ... 10 primitive inference rules and 3 axioms" | `SOUNDNESS.md` |
| `bignum_inv_p25519` proof | `arm` 3,624 lines / 3,346 code; `x86` 4,617 / 4,090 (commit `1d097f6`) | `wc -l`, comment-stripped count |
| Its theorem structure | correctness at l. 3315; `_SAFE` at 3540; `_SUBROUTINE_SAFE` at 3590 | grep |
| Proof corpus size | 723 `.ml` proof files (arm + x86 proofs dirs) | `ls | wc -l` |
| Object-byte digest checks | **none** in `tools/`, `Makefile`, `arm/Makefile` | `grep -rniE 'sha256|md5|digest|checksum|watermark'` |
| Benchmark tasks | v1.0 2,301; v1.1 2,622 | leaderboard `README.md` |
| Category split (v1.1) | generic 686, program_state 601, FC-arm 463, FC-x86 435, bit_vector 437 | leaderboard `README.md` |
| Cheating oracle | `axioms()` before vs after; verdicts `OK/FAIL/CHEATING/ERROR/TIMEOUT/SYNTAX_ERROR` | `assess_answer.py`, `combine-answer-and-setup.py` |
| Baseline prompt | single query, THEN-form, 3 few-shot examples, no feedback | `prompt-basic.txt` |
| Trace record fields | `{definition_line_number, user_line_number, goal_before, goals_after, num_subgoals}` | `exportTrace.ml` |
| Trace failure policy | "a proof that fails writes no traces" | `TacticTrace/README.md` |
| Trace sampling | 20 records, 5 buckets by subgoal count, longest-first tie-break | `exportTrace.ml` |
| Leaderboard state | 0 issues, 0 forks, no `results.json` on `master`, last push 2026-09-25 | GitHub API |

Three of these contradict the brief and are material: the FC floor of zero, the absent
failure traces, and the `2,633` line count that does not reproduce.

---

## B. Benchmark results as published

| Arm | Result | Source |
|---|---|---|
| GPT-5.3-Codex (xhigh), one attempt, query-only | **6.35%** of 2,301 | NeurIPS 2026 abstract |
| GPT-5.3-Codex (high), five attempts + execution feedback | **10.26%** | same |
| All original baselines, functional correctness | **0 / 864** | same |
| v1.1 (2,622), functional correctness | **5** successes, Grok 4.6, all smallest tasks | same |

Unverified by me: the per-category leaderboard rows, because `results.json` is not on
`master` and the site renders client-side.

---

## C. Mechanism-by-mechanism audit of the brief

Classification follows the brief's own taxonomy.

### M1 · Cross-logic acceptance — `KNOWN_COMBINATION`

Prior art: OpenTheory (HOL-family interchange), hol2dk (HOL Light → Dedukti → Rocq/Matita),
Candle (verified HOL kernel), cake_lpr (verified LRAT checker), Metamath's multiple
checkers. The CVE does not make cross-logic acceptance new; it makes *rule-level diversity*
non-optional.

What is unverified: coupling dual acceptance to **artifact digests with automatic
revocation**, and reporting it as an acceptance *criterion* rather than an audit. That is
`UNVERIFIED_COMBINATION`. The bridge is also a trust surface — if it is unverified code it
can be the attack, so it must emit a checkable interpretation or be verified.

### M2 · Differential kernel fuzzing — `PLAUSIBLY_NOVEL` (narrowly)

Prior art: differential testing is routine for compilers (arm-tv found 40 LLVM AArch64
miscompilations); Metamath checkers are differential-tested informally.

The specific, defensible target: **definitional** differential fuzzing — ill-typed nested
inductive declarations and projection-mismatch values — scored by **refusal-site mutation
score**. The CVE's PoC is Docker-reproducible, so the positive control already exists. I
found no published definitional differential fuzzer for proof kernels.

Correction to the brief: fuzzing *inference rules* would have missed this bug. The bug was
in `add_inductive`.

### M3 · ISA model court — `EXISTING` (co-simulation), `UNVERIFIED_COMBINATION` (the rest)

s2n-bignum's CI already co-simulates the formal model against an independent backend,
covering reg/reg forms and several addressing modes. LNSym co-simulates Arm; Islaris checks
against Sail models; arm-tv compares hand and derived semantics.

Genuine deltas: per-instruction **digest binding** so a model change revokes proofs;
a **vendor-derived** (Sail/ASL) third leg; **mutation controls** proving the court can fail;
and coverage accounting restricted to instructions the target kernels use. Do not claim the
court.

### M4 · Certified algebra at object-code scale — `KNOWN_COMBINATION`

Cadical/LRAT in HOL Light, Jazzline inside Jasmin, `bv_decide` in Lean. The delta is
applying it beneath object-code proofs to shrink hand scripts. Legitimate, not novel as a
mechanism.

### M5 · Forecast-residual calibration (R3) — `UNVERIFIED_COMBINATION`, and cheap

The field predicts the next *tactic* (ReProver, HTPS, tactic-prediction benchmarks) or
scores a *state* with a value function. I found no system that asks a proposer to predict
the **resulting goal state** and scores the structural error against the observed one.

This is the sharpest unexploited idea in the brief because `TacticTrace` already emits
`goal_before` and `goals_after`. R3 costs a prompt and a diff.

### M6 · Envelope pruning / doomed branches — `KNOWN_COMBINATION` (algorithm), `PLAUSIBLY_NOVEL` (labels)

Prior art: learned search guidance, clause selection (ENIGMA), value functions, timeout
prediction. The algorithm is not novel.

The novel artefact is a **ground-truth doomed-branch label from kernel failure events** on
an industrial machine-code corpus. That label is exact, not heuristic, and it does not exist
publicly.

### M7 · Per-motif trust fusion (HRET) — `EXISTING`

Portfolio and algorithm-selection literature (SATzilla, MaLARea, ENIGMA) already weights
provers by predicted performance per problem class. "Per motif" is a refactor of the
conditioning variable. Low novelty; useful engineering.

### M8 · Retroactive credit (TMTR) — `EXISTING`

Credit assignment and hindsight relabelling are standard in RL and in MCTS backup. Re-scoring
near-misses is a known technique. Do not lead with this.

### M9 · Causal lemma synthesis (DSCD) — `EXISTING` (mechanism), delta in conditioning

Prior art: QuickSpec (conjecture generation with testing, in the Haskell/Isabelle
ecosystem), HR, CVC, AProS, template-based conjecturing. Lemma *yield* is not a new
scoreboard.

The delta is selecting which lemma to conjecture from a **residual signature**, and
measuring reuse. That is worth doing; it is not a new field.

### M10 · Endoductive scheduling — `KNOWN_COMBINATION`

Explore/amplify/discriminate/falsify is bandit-style experiment selection with new names.
The measurable claim — beating best-first and MCTS at equal kernel-call budget — is fine
and falsifiable. Just do not call the classes novel.

### M11 · Residual-driven proof repair — heavily occupied

CAPRI (Isabelle, 2026) measures contract-aware repair with an independent edit-contract
checker: 138/180 valid repairs, 32/36 frozen iterative, and it *catches* proof repairs that
modified protected text. VerusWeave (75–85% focused, 29–47% module-wide). TopoVerus
(61.6/72.2 Safe-Pass@1/@3). APRIL (260k supervised tuples). Baldur (whole-proof generation
plus repair). Ringer et al. for type-level repair.

Re-scope to *ISA-model-induced* breakage of machine-code proofs, which I did not find
measured.

### M12 · Per-artefact translation validation for Rust — `EXISTING` pieces, hard target

seL4 for gcc; arm-tv for LLVM's AArch64 backend; Alive2; RefinedRust; Aeneas; Corten. The
unclosed step is landing rustc output as a **kernel-checked** refinement theorem at a named
release digest. Real, valuable, expensive. Do not put it in a gate.

### M13 · Speculative constant time on object code — `PLAUSIBLY_NOVEL`, expensive

s2n-bignum proves sequential constant time on object code and explicitly excludes
speculation (`SOUNDNESS.md` B3). Jasmin proves speculative CT at source plus one compiler
pass. The gap is real. The blocker is that a speculative non-interference proof needs a
formal hardware–software contract; that is a research programme, not a phase.

### M14 · Proofs as revocable evidence — `KNOWN_COMBINATION` (idea), `PLAUSIBLY_NOVEL` (measurement)

Necula's proof-carrying code binds a certificate to a binary; SBOM/SLSA bind attestations to
artefacts; ETP binds test evidence. Binding a *derivation tree* to digests and revoking on
change is a composition of known ideas.

What I did not find published: **revocation precision/recall under semantically-neutral
versus semantic change**, and the typed distinction between "no longer true", "no longer
about these bytes", and "accepted by a different checker binary". See the crate in `src/`.

### M15 · Self-application — `KNOWN_COMBINATION`

Refusal-site mutation testing is published (arXiv 2608.26183). Its most valuable content is
the self-critique: every number self-measured over a closed loop of the author's own
repositories, stated as the paper's credibility rather than a caveat. Tekmerion will face
the same objection and should answer it the same way — by external adjudication.

---

## D. Prior art that must be cited or the work is dismissed

| Work | Why it matters here |
|---|---|
| `TacticTrace` | Already does success-side trajectory capture, better than a reimplementation would |
| `SOUNDNESS.md` | Names B1 (ISA model), A2/D3 (no C-level link), B3 (no speculation), the reassembly gap, and the unchecked stack-discipline property |
| CAPRI / VerusWeave / TopoVerus / APRIL / Baldur | Proof repair is measured; "no published number" is false |
| QuickSpec, HR, CVC, AProS | Lemma discovery is a field |
| Necula, PCC | Proof-carrying code already binds certificates to binaries |
| arXiv 2608.26183 | Refusal-site mutation testing, and the self-measurement warning |
| SemantEq (Oct 2026) | A valid proof can establish the wrong statement; audits transformations, not fidelity to intent |
| Islaris, LNSym, arm-tv | ISA-model trust and translation validation |
| Candle, cake_lpr, bv_decide, Jazzline | Verified kernels/checkers and certified algebra |
| Aeneas, RefinedRust, Corten | Rust verification stack |
| CVE-2026-72844 advisory + PoC | The definitional bug class and a reproducible positive control |

---

## E. What I could not verify

- The `2,633`-line figure. It does not match the current tree under any counting rule I
  tried; it may refer to an older commit, a specific architecture, or a subset. **Unverified.
  Do not cite until pinned.**
- Per-category leaderboard numbers (client-side rendering; no `results.json` on `master`).
- That the leaderboard is empty in the strong sense. I infer it from zero issues, zero
  forks, no results file, and a submission-by-issue process. Inference, not observation.
- Candle's exact supported fragment of the HOL Light libraries s2n-bignum uses. I assess
  whole-corpus dual checking as infeasible now, but I did not measure it.
- The claim that Metamath Zero checks `set.mm` in <200 ms, and hol2dk's ~4 min. Taken from
  the brief; not independently measured.
- Any 2026 paper's internal numbers beyond its abstract, unless quoted above.
