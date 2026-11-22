# AGENTS.md — operating rules for Tekmerion

These rules bind the orchestrator and every subagent. They override convenience.

## 1. The orchestrator delegates. It does not do the work itself.

The orchestrator's job is judgement, sequencing, the claim ledger, git, and publishing.
It must **not** spend its context on:

- long builds or proof runs,
- tailing logs or polling processes,
- running test/fix loops,
- bulk edits across many files,
- literature sweeps that a subagent can do and summarise.

If a task can be delegated, it is delegated.

## 2. Exactly one subagent at a time. Always.

Never spawn a second subagent while one is running. Queue the rest in `TODO.md`.
Sequential is the rule even when two tasks look independent.

## 3. Every delegation is self-contained.

A subagent does not see the conversation. Each brief must include:

- absolute repository path and the branch to work on,
- the exact defect or deliverable,
- file paths it may write, and files it must not touch,
- resource limits (disk, RAM, no network, no 20 GB build),
- the commands it must run and report verbatim,
- acceptance criteria and the stop condition.

A brief that lets the agent guess is a bug in the brief.

## 4. No claim without a receipt. No hand-typed numbers.

- Every number in a document is derived from a file, a command output, or a cited source
  stored under `research/receipts/`.
- A subagent must report **actual** command output. "It should pass" is not a report.
- If something cannot be verified, the report says so explicitly.
- Never delete a receipted claim because a reviewer looked at a different document. Cite the
  receipt, name the document difference, and keep the claim. (Current example: the 10.26% and
  Grok lines are in the NeurIPS poster abstract, not in the arXiv paper or leaderboard data.)

## 5. Push to `staging` before and after every delegated task.

- `main` holds only published states.
- Work is committed and pushed to `staging` continuously so nothing is lost.
- Release: `git checkout main && git merge --ff-only staging`, bump the version, tag,
  `cargo publish`, push.

## 6. Verification on return

When a subagent returns:

1. Confirm the claimed files exist.
2. Re-run the decisive command yourself, or ask the next subagent to, before marking done.
3. Mark `TODO.md` only after that check.
4. If the report and the tree disagree, treat the tree as truth and re-delegate.

## 7. Resource discipline

- `.bench/` is git-ignored. Never commit benchmark artefacts, opam switches, or object files.
- Do not rebuild HOL Light unless a change requires it; reuse with `--skip-hol` when possible.
- Keep at least 3 GB free on the project volume. Stop and report before filling it.
- Each s2n-bignum proof can use ~5 GB RAM. Size parallelism as `RAM_GB / 6`.

## 8. Honesty over momentum

- Report blockers plainly: missing model credentials, missing assembler, insufficient disk.
- Negative results are reported next to positive ones; they are never dressed up as wins.
- If no arm can clear the published bar, the answer is "stop", not a reframed null.
- A benchmark result is scoped to its version, model and budget. Never mix revisions or
  attempt counts.

## 9. Stop and escalate

- Kernel disagreement, a sensitivity control that stops firing, a detected contamination, or a
  claimed result that fails reproduction: stop the affected work and record why.
- The trusted base never grows without a decision record.

## 10. The queue

`TODO.md` is the single queue. The orchestrator dispatches one item, waits, verifies, marks it,
and only then dispatches the next. Items are never worked in parallel.
