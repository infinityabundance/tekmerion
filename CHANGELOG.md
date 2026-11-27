# Changelog

All notable changes to Tekmerion. Numbers in this file are from committed receipts
under `research/receipts/`; each line names its receipt and commit, because totals
are only comparable within a commit (the inlined support library grows, which shifts
per-answer totals by a constant).

## [Unreleased]

### XXIII.B1 — per-carrier aggregate premise fast path (`ix91`, `cdd673f`)

- `bridge_aggregate_facts` now tries only the aggregate(s) that mention a carrier before
  falling back to the full list. Strict self-contained totals: `add` 5,745,767 →
  **5,268,391** (−477,376, the IX.86 inert-premise figure exactly; 2.85× human), `sub`
  9,029,510 → **8,217,594** (−811,916; 6.40×), `neg` unchanged at 5,683,733 (7.88×; one
  aggregate, so the selection is structurally a no-op). `plain == phase` with `diff 0`;
  every phase except the aggregate-fact phase is bit-identical.
- Fixtures re-recorded deliberately: 27 answers, `curve_check_bridge.txt` (line numbers
  only), `BRIDGE_DIGEST` (`895a5105…`), the instrument sha and the four inline-answer digests.
- Not established: `equals_thm` re-check, matched-session accounting, timing medians.

### XXV.E — silent-acceptance mutation operator (`a62b3c4`)

- A second mutation operator (`accept`, replacing a refusal with `Ok(Default::default())`)
  measures whether tests **assert** a refusal rather than merely reach it: 61/61 valid
  mutants killed; 40 sites are invalid because their `Ok` payload has no `Default`.
  Reachability stays 101/101. The accept score is not yet wired into CI.

## [0.2.0] — 2026-11-26

The first release in which the arithmetic reasoning is Tekmerion's own.

### Solver-free machine-code proofs

- **ARM Curve25519 add, sub and neg close with no SAT solver** (`ix82`, `ix83`;
  reconciled and re-measured in `ix86`). `add`: 5,745,767 primitive inferences
  against 111,523,776 on the sliced SAT route (19.41× fewer) and 1,848,494 for the
  human proof body (3.11×). `sub`: 9,029,510 vs 87,971,295 (9.7×). `neg`:
  5,683,733 vs 33,910,447 (6.0×). Zero solver invocations, verified with a failing
  `cadical` shim on `PATH` and an absent marker file. (The pre-IX.83 tree measured
  5,233,638 / 2.83× for `add`; `ix86` reconciles the two figures and the reason for
  the difference.)
- The mechanism constructs the **chain aggregate as a weighted sum**
  (`Σ 2^(64i)·eqᵢ`) instead of recovering it after variable elimination, with the
  weights read from the proof rather than hard-coded.
- Controls: removing the aggregates makes the close fail; `equals_thm` against the
  SAT-route theorem returns true in one session, both with zero hypotheses.
- Known limits: `x86/add` exhausts memory (12.31 GB at a 12 G cap, 24.90 GB at 24 G)
  and does not close; `arm/add_p256` emits no aggregate because its accumulator
  presentation lacks disjoint carry paths.

### Proof objects

- First **export and replay** of a real HOL Light derivation by an independent Rust
  checker: `ADD_SYM`, 314 recorded nodes → 227 reachable, 13 imports, 7,039 bytes,
  ~24 ms replay. Labelled provisional: the checker is well-formed, not proved sound.
- Hostile-input refusals: truncated object, spliced object, wrong conclusion,
  unjustified import, unauthorised axiom.
- The real replay found a defect no test had: substitution pairs were applied in the
  wrong order relative to HOL Light. Fixed and regression-tested.

### Assurance

- Freeze `v6` sealed (`freeze-rust-closure-v6`, annotated, unsigned, `cb054bd`):
  72 source files, 10 runtime digests, 6 bound `TEKMERION_*` variables. Later work is
  a `v7` delta; a seal is a snapshot, not a permanent precondition.
- Identity is digests, not labels: `object_sha256`, `code_sha256`, `semantics_sha256`.
- Two comparison types: `IdentityBoundComparison` may back a claim;
  `DiagnosticComparison` prints what evidence is missing and is never a result.
- Failure taxonomy: only allow-listed mathematical failures retry; resource
  exhaustion stops rather than retrying with a larger instance.
- `tools/cap.sh` caps every heavy run (`RLIMIT_AS`, verified to bite; per-process,
  documented as such).
- Offline CI fixtures for the 27 answers, their objects and corpus entries; an
  `ocamlc` gate that fails rather than skips.

### Corrections of record

- The "10–20× slower" check-time figure was withdrawn, then the retraction was itself
  corrected: the same-obligation inference ratio is a 60.2× median.
- "27/27 upstream acceptance" was overstated: the drop-in proved a *different*
  contract. Upstream's `_SUBROUTINE_CORRECT` is now derived for all 27, whole-tree
  `make` is green, and the earlier claim is corrected in place.

## [0.1.0] — 2025-11-22

- Typed, content-addressed revocation for kernel-checked proof evidence.
- Failure-trace scaffold, self-test, and release workflow.
