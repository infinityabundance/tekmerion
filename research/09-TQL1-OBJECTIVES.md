# Assurance case: DO-330 TQL-1 objectives for Tekmerion

**Status:** an objective list with gaps marked. This is **not** a claim of qualification or
certification, and it is not a claim that the applicable level for a tool such as this is TQL-1.

## What is being claimed, precisely

DO-330 assigns a tool qualification level by what a tool's failure can do, and the applicable level
depends on the intended use, the software's assurance level, and whether the tool's output is used
to *justify eliminating* other verification or development activity (DO-330/ED-215 criteria,
including Criterion 2). A tool that can only fail to detect an error is not automatically confined
to TQL-5.

So the objective, stated at the strength the evidence supports:

> **Develop an assurance case against the applicable DO-330 TQL-1 objectives, supplemented by formal
> kernel soundness, executable refinement, and mathematical evidence preservation.**

TQL-1 is set at roughly the rigour of Level A software, so several of its objectives fall on
Tekmerion's own code and processes, not on the proofs it emits. Those are the expensive ones.

## Objective map

Legend: **met** = evidence exists and is cited · **partial** = something exists, named gap ·
**gap** = nothing yet.

| # | TQL-1 objective (paraphrased) | Tekmerion artefact | Status |
|---|---|---|---|
| 1 | Tool requirements traceable to the system's needs | `TODO.md` (C1/C2/C3/E4 then XVII–XX), receipts per claim | **partial** — requirements are prose; no traceability matrix to a single baseline |
| 2 | Tool requirements are complete and correct | Claim ledger in `research/00-VERDICT.md`; corrections of record | **partial** — no independent review of completeness |
| 3 | Tool design is documented and traceable to requirements | `src/tools/*` module docs; `IX.59`–`IX.77` design receipts; charter `verification/tnps/` | **partial** — design→requirement links are implicit |
| 4 | Tool is implemented per its low-level requirements | 45 test suites; mutation harness (guard 60/60, freeze 39/41 — **2 survivors**) | **partial** — two freeze-parser mutants survive; CI red on them |
| 5 | **Tool verification includes structural coverage of the tool's own code** | none | **gap** — this is the large item. TQL-1 expects MC/DC-like coverage of the qualification-relevant code; Tekmerion has line/branch-by-test but no coverage measurement at all |
| 6 | Tool source is under configuration management | git; `freeze-rust-closure-v6` (72 source files, source + runtime closure) | **met** for the sealed scope; `v7` delta outstanding |
| 7 | Tool accomplishment is traceable and repeatable | receipts with commands and verbatim output; `tool`s re-run by a second party on request | **partial** — one host, one pin; no reproducibility claim across machines |
| 8 | Tool life-cycle data are controlled | `TODO.md`, CHANGELOG, receipts, `freeze-*` tags | **partial** — no review/approval records |
| 9 | **Tool executable object code is verified to its source** | none | **gap** — this is the charter's self-certifying kernel (`XVII.B8`): prove the shipped bytes by Tekmerion's own machine-code method. Nothing yet |
| 10 | Tool is robust to invalid input and resource exhaustion | failure taxonomy (5 classes, allow-list retry), `tools/cap.sh` (`RLIMIT_AS`, group kill), streaming limits | **partial** — budgets apply after parsing in the proof-object path; TOCTOU between digest scan and replay (`XXI.B1`) |
| 11 | Tool qualification data are summarised in a plan | this file | **partial** — no plan, no approval, no schedule |
| 12 | **Soundness of the mathematical judgments the tool issues** | Lean: `rup_sound`, `Lrat.refutation_sound`, `Cert.theorem1` (G4); `blast_sound` a hypothesis | **partial** — G1 (bit-blaster), G2 (parser), G3 (executable) open. The strongest objective, and the least complete |
| 13 | Independence of the verification from the tool under test | generated proofs re-checked by upstream s2n-bignum's own build; independent Rust checker (provisional) | **partial** — the Rust checker is not proved sound; `HOLTrace`/`Candle` cross-check not yet run (`XXI.C`) |

## The two that dominate the effort

- **#5 structural coverage of Tekmerion's own code.** Nothing exists. Measuring MC/DC over the
  qualification-relevant Rust (guard, freeze, emit, proof_object) is tractable with existing
  tooling, but it will surface untested branches that the current 45-suite pass count hides.
- **#9 executable object code verification.** This is the charter's self-certification step, and it
  is the one no other tool in this space has attempted for a *kernel*. It is also the one with the
  clearest fallback: prove the checker's relation in Lean, refine Rust to it, then prove the binary
  by the machine-code pipeline that already exists.

## Honest position today

Tekmerion can show: a sealed source/runtime closure, a failure taxonomy with firing controls, a
mutation harness with two known survivors, and machine-checked abstract soundness with three open
gaps. It cannot show structural coverage, executable refinement, or an approved qualification plan.

Anything stronger than that is not claimed.
