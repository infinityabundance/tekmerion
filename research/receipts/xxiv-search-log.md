# XXIV — Search log

Session date 2026-11-27, branch `staging`. Sequence numbers are chronological within this session;
no wall-clock timestamps were recorded. Every `fetch` call is listed, including those that failed.

Interfaces used:
- `fetch` on a direct URL (publisher page, author page, DOI redirect, arXiv abs page).
- `fetch` on `https://arxiv.org/search/?query=...&searchtype=all` (strict AND over metadata).
- `fetch` on `https://export.arxiv.org/api/query?search_query=...` (arXiv API).
- `fetch` on `https://duckduckgo.com/html/?q=...` (general web).
- `fetch` on `https://r.jina.ai/<pdf-url>` (text extraction proxy), used once, after the direct PDF
  fetch returned undecoded binary.

Known interface limits (so the negatives are bounded):
- arXiv site search matches title/abstract/comments/journal-ref/keywords only, and is a strict AND.
  Multi-term queries therefore return 0 even when the constituent terms are abundant. Rows 27–29 are
  artefacts of this, not evidence of absence.
- DuckDuckGo began returning a bot challenge at row 21 and did not recover during the session.
- Springer pages marked "abstract only" are paywalled; I did not obtain their bodies.

| # | query or URL | interface | result |
|---|---|---|---|
| 1 | `https://adam.chlipala.net/papers/FiatCryptoSP19/` | fetch | **OK** — paper page + full abstract; PDF not downloaded |
| 2 | `https://link.springer.com/chapter/10.1007/978-3-031-37703-7_11` | fetch | **OK** — open access, full text (CoqCryptoLine) |
| 3 | `https://www.cl.cam.ac.uk/~mom22/decompilation/` | fetch | **OK** — project page + abstract (Decompilation into Logic) |
| 4 | `https://www.cl.cam.ac.uk/~mom22/cpp13/` | fetch | **OK** — abstract + script links, no body text |
| 5 | `https://portal.fis.tum.de/en/publications/proof-synthesis-and-reflection-for-linear-arithmetic/` | fetch | **OK** — abstract + bibliographic metadata |
| 6 | `https://link.springer.com/chapter/10.1007/978-3-032-04167-8_20` | fetch | **OK** — open access, full text (Lambdapi LIA) |
| 7 | `https://drops.dagstuhl.de/entities/document/10.4230/LIPIcs.CP.2025.21` | fetch | **OK** — abstract + metadata + reference list (VeriPB/CakePB) |
| 8 | `https://arxiv.org/abs/2211.10665` | fetch | **OK** — abstract (CryptOpt) |
| 9 | `https://link.springer.com/article/10.1007/s10009-022-00688-6` | fetch | **OK** — open access, full text (AMulet2) |
| 10 | `https://pmc.ncbi.nlm.nih.gov/articles/PMC7691315/` | fetch | **OK** — full text (AMulet column-wise, FMSD 2020) |
| 11 | `https://arxiv.org/abs/2209.03398` | fetch | **OK** — abstract (Small Proofs from Congruence Closure) |
| 12 | `https://link.springer.com/chapter/10.1007/978-3-031-30823-9_19` | fetch | **OK** — open access, full text (Carcara) |
| 13 | `https://doi.org/10.1007/s10817-025-09743-8` | fetch (DOI) | **OK** — redirect to Springer, full text (Candle extended) |
| 14 | `https://doi.org/10.4230/LIPIcs.ITP.2023.4` | fetch (DOI) | **OK** — abstract + metadata (Fast, Verified Computation for Candle) |
| 15 | DDG `Fox "Improved Tool Support for Machine-Code Decompilation in HOL4" ITP 2015` | DDG | **OK** — 10 results; located Springer chapter, author PDF `acjf3.github.io/papers/itp15.pdf`, dblp, semanticscholar |
| 16 | DDG `Lean ite_congr contextual if-then-else exponential blowup symbolic simulation` | DDG | **OK** — 10 results; relevant hit: `teorth.github.io/expdb/docs/Lean/Meta/Sym/Simp/SimpM.html`; also a Lean issue (#7525) about an `ite_congr` simp loop, not the asymptotic claim |
| 17 | `https://teorth.github.io/expdb/docs/Lean/Meta/Sym/Simp/SimpM.html` | fetch | **OK** — Lean metaprogramming doc; contains the O(2^n) `ite_congr` statement verbatim |
| 18 | `https://acjf3.github.io/papers/itp15.pdf` | fetch | **FAILED** — response was undecoded PDF stream bytes; no text could be read, so no quote taken from it |
| 19 | `https://link.springer.com/chapter/10.1007/978-3-319-22102-1_12` | fetch | **OK (partial)** — abstract only (paywalled); endnote 10 mentions run times vs caching, which corroborated the PDF |
| 20 | `https://www.semanticscholar.org/paper/Improved-Tool-Support-for-Machine-Code-in-HOL4-Fox/fcc206b37ba29607dd47ac494851f19fbe97780b` | fetch | **FAILED** — "no textual content found" |
| 21 | DDG `cake_lpr verified checker clauses per second throughput LRAT` | DDG | **FAILED** — bot challenge ("select all squares containing a duck"), no results |
| 22 | DDG `mm0-c Metamath Zero checker verification speed theorems per second` | DDG | **FAILED** — same bot challenge |
| 23 | `https://r.jina.ai/https://acjf3.github.io/papers/itp15.pdf` | fetch (proxy) | **OK** — full 16-page text extracted; source of the 318-theorem quote and Tables 2–3 with inference counts |
| 24 | `https://arxiv.org/abs/1910.10703` | fetch | **OK** — abstract (Metamath Zero); gives the "<200 ms" `set.mm` figure |
| 25 | `https://export.arxiv.org/api/query?search_query=all:%22cake_lpr%22&max_results=5` | arXiv API | **OK** — 2 entries (2610.26465, 2607.23766); both *use* cake_lpr as a DRAT checker; neither reports throughput |
| 26 | `https://link.springer.com/chapter/10.1007/978-3-030-72013-1_12` | fetch | **OK (abstract only)** — cake_lpr TACAS 2021; abstract states no rate |
| 27 | arXiv site `proof compression proof minimisation proof DAG` | arXiv site search | **OK — 0 results** (strict-AND artefact; see limits) |
| 28 | arXiv site `"Farkas" "machine code" certificate` | arXiv site search | **OK — 0 results** |
| 29 | arXiv site `congruence closure proof minimisation kernel` | arXiv site search | **OK — 0 results** (strict-AND artefact) |
| 30 | arXiv site `"proof compression"` | arXiv site search | **OK — 14 results**, none on LCF/kernel proof-DAG optimisation; nearest: 2505.20009 (subatomic logic), 2212.12150, 1804.06531 (first-order partial regularization), 2609.38384 (LeanPolish, Lean *proof-text* compression for LLM supervision) |
| 31 | `https://export.arxiv.org/api/query?search_query=all:Carcara&max_results=10` | arXiv API | **OK but unhelpful** — payload header reported 10/0/0 with no entry list rendered; treated as no usable hit |
| 32 | arXiv site `Jasmin cryptography compiler` | arXiv site search | **OK — 2 results**: 2511.11292 (KEM-IND-CCA-preserving compilation of Jasmin ML-KEM) and 1904.04606 (The Last Mile). Both are source→assembly; neither inverts |
| 33 | arXiv site `"proof minimization"` | arXiv site search | **OK — 9 results**: 2209.03398 (congruence closure), 2411.07955 (resolution length minimisation), 2511.08198 (DNN proofs), 2605.21200 (Krympa), 1805.12181 (clausal proof minimisation), plus 4 irrelevant (math.GR / math.HO) |

Not fetched, and why:
- MIT DSpace record for Fiat Cryptography — the author page carried the abstract; the PDF was not
  needed for any block, so not fetched.
- CakePB's own paper and the CakeML/STTT journal version of cake_lpr — the TACAS abstract sufficed
  to establish "no rate stated"; the bodies are paywalled and I chose not to spend the fetch budget.
- `01-PRIOR-ART.md`, `04-NOVELTY-REGISTER.md` — read for terminology only; no facts taken from them.

## Searches that returned nothing relevant

1. **`proof compression` + `proof minimisation` + `proof DAG` (row 27).** Returned 0. The three
   terms must all appear in metadata; the literature uses "proof compression" or "proof
   minimization" but not the "DAG optimisation" phrasing. Re-run as row 30/33 with single phrases,
   which produced the useful set. Conclusion drawn: engineering minimisation exists per proof
   (resolution, DNN, equational, e-graph); nothing on whole-development LCF DAGs.
2. **`"Farkas" "machine code" certificate` (row 28).** 0 hits. Farkas work is indexed under SMT
   (Alethe/Carcara), pseudo-Boolean (VeriPB) and integer programming, not under machine code. This is
   the direct evidence for Q1 = no.
3. **`congruence closure proof minimisation kernel` (row 29).** 0 hits, partly a strict-AND artefact,
   but row 11 (the only congruence-closure minimisation paper, fetched in full abstract) confirms it
   targets `egg`/Z3, not a kernel.
4. **DuckDuckGo rows 21–22.** Not "no relevant results" but "no results at all" — the engine refused
   service. Both were throughput queries (cake_lpr, mm0-c). Consequence: Q8 is answered for MM0 from
   the arXiv abstract, and left as "not found" for CakeML/Candle.
5. **Row 31 (`all:Carcara`).** The API response rendered a header with no entries; not usable.
   Carcara coverage rests on the fetched TACAS full text instead.
6. **Row 19 vs row 23.** The Springer abstract alone would have yielded only "one to two orders of
   magnitude faster"; the 318-theorem count and the inference-count tables exist only in the PDF,
   obtainable only through the proxy at row 23. Recording this because a future reader relying on the
   publisher page would wrongly conclude Fox reports no inference counts.
