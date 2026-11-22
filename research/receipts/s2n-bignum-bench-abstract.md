# Receipt — s2n-bignum-bench benchmark claims

- **Source:** <https://neurips.cc/virtual/2026/poster/139651> (NeurIPS 2026 poster abstract)
- **Fetched:** 2026-11-22
- **Page size:** 88,679 bytes
- **SHA-256 of fetched page:** compute with `curl -sL <url> | sha256sum` when re-checking.

## Verbatim sentences

> Under query-only prompting, GPT-5.3-Codex (xhigh reasoning effort) solves **6.35%** of the
> original 2,301 tasks in a single attempt, while GPT-5.3-Codex (high effort) solves **10.26%**
> within five sequential attempts with execution feedback. **None of the original baselines
> solves any of the 864 functional-correctness tasks.** A later evaluation on the refreshed
> v1.1 snapshot (2,622 tasks) records the first *five functional-correctness successes*
> (**Grok 4.6**), all among the smallest tasks, leaving substantial headroom.

## What this receipt establishes

1. 6.35% — one attempt, query-only, 2,301 tasks.
2. 10.26% — five sequential attempts **with execution feedback**, 2,301 tasks.
3. 0 / 864 — functional correctness, all original baselines.
4. v1.1 (2,622 tasks): the first five functional-correctness successes, Grok 4.6, all smallest.

## Scope caveats — read before citing

- The **arXiv paper** abstract (`arXiv:2603.14628`, checked the same day) does **not** contain
  items 2, 3 or 4. Cite the **poster abstract**, not the paper, for those.
- The leaderboard's embedded `results.json` / `index.html` is a **different artefact**. A
  reviewer did not find the Grok line there. Do not attribute item 4 to the leaderboard data.
- The poster abstract is a conference artefact. It is citable, but it is not a peer-reviewed
  proceedings statement; label it as such in any claim.

## Consequence for the programme

The comparative bars are: 6.35% (Pass@1) and 10.26% (Pass@5 with feedback), both on 2,301
tasks, and 0/864 on functional correctness. Never mix the 2,301 and 2,622 revisions, and never
mix one-shot with five-attempt numbers.
