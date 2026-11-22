# Receipt — Prior-art search log for N1, N3, N4, N5, N6, N11, N12

- **Searches run:** 2026-11-22
- **Type:** primary (search results observed directly). This receipt records *what was searched*,
  so that "not found" is bounded by the query and the index.

## Method and its limits

Two interfaces were used:

1. arXiv **site search** (`https://arxiv.org/search/?query=...&searchtype=all`).
2. The arXiv **API** (`http://export.arxiv.org/api/query?search_query=...`).

Limits, stated so the negative results are not over-read:

- Site search with `searchtype=all` matches **title, abstract, comments, journal ref and
  metadata**, not the full text. A zero-hit query means "no title/abstract match", not "no such
  work exists".
- The API silently fails on `all:"phrase" AND all:"phrase"` conjunctions (returned
  `totalResults 0` for queries that the site search shows are non-empty). Those API results were
  discarded and the site search was used instead.
- Later in the session the site-search endpoint began returning HTTP 400 (rate limiting). Three
  intended queries on N6/N1 were *not* completed; the affected rows are marked in the novelty
  register as having no dedicated search.
- Coverage is arXiv-only. Journals, ITP/CPP/POPL/PLDI proceedings and technical reports not on
  arXiv are not searched. `ProofCert` and `Par` are cited from prior knowledge and are marked
  secondary in the register.

## Query log

| # | Query | Interface | Result |
|---|---|---|---|
| 1 | `conformal prediction` AND `theorem proving` | API | `totalResults 0` (rejected as unreliable, see above) |
| 2 | `conformal prediction theorem proving` | site | 15 hits; **none** on proof search or prover abstention |
| 3 | `conformal prediction` AND `proof search` | API | `totalResults 0` (unreliable) |
| 4 | `conformal prediction code generation abstention` | site | 1 hit: arXiv 2502.06884 (learnable conformal abstention for LLMs/VLMs) |
| 5 | `GPU proof checking parallel` | site | 4 hits; none is GPU proof-**certificate** checking (hits: 2610.24548 GPU-kernel verification in Lean/VerCors; 2610.05683 RESOLVE; 2609.17380 OPEN-1B; 2603.25414 Decidable By Construction) |
| 6 | `parallel proof checking Coq` | site | 2 hits; neither is a parallel proof checker (1910.10848 LLFP; 1207.3441 Isabelle/jEdit, which mentions multi-core parallelism) |
| 7 | `proof repair type equivalence transport` | site | 1 hit: arXiv 2010.00774 PUMPKIN Pi |
| 8 | `contamination code benchmark semantics-preserving` | site | 5 hits, incl. 2607.04729 RustMizan, 2506.17627 CodeMorph, 2503.06643 Dynamic Benchmarking, 2504.04372 Haroon et al. |
| 9 | `process reward model theorem proving Lean` | site | 5 hits, incl. 2610.21881, 2506.04592 Safe, 2504.06122 Leanabell-Prover, 2503.09730 verifier-in-the-loop, 2408.08152 DeepSeek-Prover-V1.5 |
| 10 | `dense reward theorem proving kernel` | site | 0 hits |
| 11 | `ProofCert` / `proof certificate translation checking` | site | **HTTP 400 (not completed)** |
| 12 | `metavariable holes proof search tactic` | site | 0 hits |
| 13 | `typed holes` (API) | API | 18 total; shown: 1805.00155 Hazelnut Live, 2409.00921 LLMs + typed holes, 2110.04461 Liquid Haskell holes |
| 14 | `cross-ISA` (API) | API | 8 total; all binary translation / build-repair benchmarks; **none** on proof transport |
| 15 | `verified LRAT checker CakeML` | site | **HTTP 400 (not completed)** |

## Quoted hits for the two `[P]` conformal citations (N3)

Both abstracts were returned verbatim by the queries below; the quotes are reproduced so the
register's `[P]` markers have a receipt.

**arXiv 2107.07511** (site query 2), Angelopoulos & Bates, *A Gentle Introduction to Conformal
Prediction and Distribution-Free Uncertainty Quantification*:

> Conformal prediction is a user-friendly paradigm for creating statistically rigorous
> uncertainty sets/intervals for the predictions of a model. Critically, the sets are valid in a
> distribution-free sense: they possess explicit, non-asymptotic guarantees even without
> distributional assumptions or model assumptions.

**arXiv 2502.06884** (site query 4), Tayebati et al., *Learning Conformal Abstention Policies
for Adaptive Risk Management in Large Language and Vision-Language Models*:

> Conformal prediction (CP), a leading UQ method, provides statistical guarantees but relies on
> static thresholds, which fail to adapt to task complexity and evolving data distributions,
> leading to suboptimal trade-offs in accuracy, coverage, and informativeness. To address this,
> we propose learnable conformal abstention, integrating reinforcement learning (RL) with CP to
> optimize abstention thresholds dynamically.

What these establish: distribution-free coverage is a mature, formal guarantee, and conformal
*abstention* is already applied to LLMs/VLMs. What they do not establish: any application to
proof search, branch pruning, or theorem provers.

## What the log supports

- N3: no prior work found at title/abstract level applying conformal prediction or
  distribution-free abstention to *proof search*. Bounded by queries 1–4.
- N11: no prior work found on GPU proof-certificate checking. Bounded by queries 5–6.
- N1: typed holes as a *synthesis context* for LLMs is published (2409.00921), but no work was
  found on hole/type-complexity/unification-failure *residuals* as a search signal (queries
  12–13).
- N5: PUMPKIN Pi is the closest work found (query 7).
- N12: mechanism is occupied four times over (query 8).
- N4: dense, step-level, verifier-grounded reward is occupied (query 9).

## What the log does **not** support

- Any claim of novelty. Zero hits in a title/abstract index is not evidence of absence.
- Any claim about N6 or N11's LRAT literature; queries 11 and 15 did not complete.
