# XXV — Search log

Session date 2026-11-27, branch `staging` (read-only). Sequence numbers are chronological within
this session; no wall-clock timestamps were recorded. Every `fetch` is listed, including failures
and one that returned a different paper than intended.

Interfaces used:
- `fetch` on a direct URL (publisher page, author page, arxiv abs page, raw GitHub file, GitHub
  REST API, text-extraction proxy `r.jina.ai`).
- `fetch` on `https://arxiv.org/search/?query=...&searchtype=all` (strict AND over metadata).
- `fetch` on `https://export.arxiv.org/api/query?search_query=...` (arXiv API).
- `fetch` on `https://api.semanticscholar.org/graph/v1/...` (bibliographic lookup).

Known interface limits, so the negatives are bounded:
- Semantic Scholar returned **HTTP 429** on both mutation-testing queries; no retry succeeded.
- `www0.cs.ucl.ac.uk` **refused the connection** (transport error, not 404).
- `api.github.com/repos/jrh13/hol-light/contents/tests` returned 404 — that path does not exist,
  which does not prove the repository has no test code elsewhere.
- arXiv site search is a strict AND over title/abstract/comments/keywords; `all:"proof
  amplification"` is a term collision with signal/PCP literature, so its 357 hits are not evidence.
- Direct PDF fetch of `sos.pdf` returned undecoded PDF bytes; the proxy fetch (row 15) supplied the
  text. `groebner.pdf` / `grobner.pdf` were 404 on both guesses.

| # | query or URL | interface | result |
|---|---|---|---|
| 1 | `https://link.springer.com/chapter/10.1007/978-3-031-37703-7_11` | fetch | **OK** — open access, full text (CoqCryptoLine); source of the "30 lines of annotations"/"hints" quotes and the `lift`/`gbarith` sentence |
| 2 | `https://adam.chlipala.net/papers/FiatCryptoSP19/` | fetch | **OK** — abstract page only (Fiat Cryptography) |
| 3 | `https://www.cs.ru.nl/~freek/notes/factor.pdf` | fetch | **FAILED** — 404 |
| 4 | `https://www.cs.ru.nl/~freek/notes/` | fetch | **OK** — notes index; "The De Bruijn Factor" abstract with the "approximately equal to four" sentence |
| 5 | `https://doi.org/10.1109/TSE.2011.41` | fetch (DOI) | **FAILED** — "no textual content found" |
| 6 | `https://arxiv.org/search/?query=proof+amplification&searchtype=all` | arXiv site | **OK — 357 results**, all unrelated senses (privacy/gap/signal amplification); **no relevant hit** |
| 7 | `https://export.arxiv.org/api/query?search_query=all:"witness generation"&max_results=10` | arXiv API | **OK — 10 entries**; JSON-Schema, ZK-SNARK, WCET, entanglement witnesses; **none on arithmetic proof witnesses** |
| 8 | `https://www.cl.cam.ac.uk/~jrh13/papers/` | fetch | **OK** — Harrison publications list; confirms SOS (TPHOLs 2007), Gröbner (CADE 2007), proof-producing real arithmetic (CADE-20 2005), *Theorem Proving with the Real Numbers* (1998) |
| 9 | `https://homes.cs.washington.edu/~rjust/publ/valid_substitute_fse_2014.pdf` | fetch | **FAILED** — 404 (guessed filename) |
| 10 | `https://www0.cs.ucl.ac.uk/staff/y.jia/Publications.html` | fetch | **FAILED** — transport error (host refused) |
| 11 | `https://www.cl.cam.ac.uk/~jrh13/papers/sos.pdf` | fetch | **FAILED** — undecoded PDF byte stream, no extractable text |
| 12 | `https://www.cl.cam.ac.uk/~jrh13/papers/groebner.pdf` | fetch | **FAILED** — 404 |
| 13 | `https://homes.cs.washington.edu/~rjust/publ/` | fetch | **OK** — publication list; confirms Just et al., "Are mutants a valid substitute…", FSE 2014, pp. 654–665 |
| 14 | `https://api.semanticscholar.org/graph/v1/paper/DOI:10.1109/TSE.2011.41?fields=title,abstract,year,authors,venue` | S2 API | **OK but WRONG PAPER** — resolved to "How We Refactor, and How We Know It"; the DOI is not Jia & Harman |
| 15 | `https://r.jina.ai/https://www.cl.cam.ac.uk/~jrh13/papers/sos.pdf` | fetch (proxy) | **OK** — full 17-page text; source of the "unverified external programs"/"merely needs to verify" quote, the Positivstellensatz cofactor formulation, and the 0.06 s / 1.25 s / ~1 s timings |
| 16 | `https://api.semanticscholar.org/graph/v1/paper/search?query=An+Analysis+and+Survey+of+the+Development+of+Mutation+Testing&fields=...` | S2 API | **FAILED** — HTTP 429 |
| 17 | `https://api.semanticscholar.org/graph/v1/paper/search?query=Are+mutants+a+valid+substitute+for+real+faults+in+software+testing&fields=...` | S2 API | **FAILED** — HTTP 429 |
| 18 | `https://api.github.com/repos/ufmg-smite/carcara/contents/tests` | GitHub API | **OK** — `elaboration/`, `regression/`, `rules/`, `test_*.rs` |
| 19 | `https://api.github.com/repos/jrh13/hol-light/contents/tests` | GitHub API | **FAILED** — 404 (no root `tests/`) |
| 20 | `https://arxiv.org/abs/2211.10665` | fetch | **OK** — CryptOpt abstract ("randomized search through the space of assembly programs…") |
| 21 | `https://raw.githubusercontent.com/awslabs/s2n-bignum/main/README.md` | fetch | **OK** — "accompanied by a machine-checked formal proof in HOL-Light"; no idiom-recognition claim |
| 22 | `https://raw.githubusercontent.com/ufmg-smite/carcara/main/tests/test_regression.rs` | fetch | **OK** — "Cases in `valid/` must be accepted, and cases in `invalid/` must be rejected. No case may panic."; `known-bug` ignored tests |
| 23 | `https://api.github.com/repos/tanyongkiam/cake_lpr/contents/` | GitHub API | **OK** — `example.cnf`/`example.lpr`, `experiments/`, no root reject-suite |
| 24 | `https://pitest.org/quickstart/mutators/` | fetch | **OK** — operator table (Conditionals Boundary, Negate Conditionals, Return Values, Remove Conditionals, Primitive/True/False/Null/Empty returns, Void Method Calls) |
| 25 | `https://www.cs.gmu.edu/~offutt/softwaretest/papers/mutation-2000.pdf` | fetch | **FAILED** — redirected to the GMU CS department page, no paper text |
| 26 | `https://export.arxiv.org/api/query?search_query=all:"Barrett reduction" AND all:verified&max_results=8` | arXiv API | **OK — 3 entries** (2604.15249, 2605.02856, 2604.25878); all masking/side-channel verification, **no correctness-proof or witness-derivation work** |
| 27 | `https://export.arxiv.org/api/query?search_query=all:"primitive inferences"&max_results=10` | arXiv API | **OK — 10 entries**, all NLP/DNN "primitive inference patterns"; **no ITP/machine-code hit** |
| 28 | `https://cs.gmu.edu/~offutt/softwaretest/papers/mutation-2000.pdf` | fetch | **FAILED** — same department page redirect |
| 29 | `https://homes.cs.washington.edu/~mernst/pubs/mutation-testing-fse2014.pdf` | fetch | **FAILED** — 404 (guessed filename) |
| 30 | `https://r.jina.ai/https://adam.chlipala.net/papers/FiatCryptoSP19/FiatCryptoSP19.pdf` | fetch (proxy) | **OK** — full 18-page text; source of the Montgomery `q` sentence and the OpenSSL #1953 `q`/`r` bug quote |
| 31 | `https://www.cl.cam.ac.uk/~jrh13/papers/grobner.pdf` | fetch | **FAILED** — 404 |

Totals: 31 rows — **17 usable OK** (1, 2, 4, 6, 7, 8, 13, 15, 18, 20, 21, 22, 23, 24, 26, 27, 30;
rows 6 and 27 are OK-but-negative), **13 FAILED** (3, 5, 9, 10, 11, 12, 16, 17, 19, 25, 28, 29, 31),
and **1 OK-but-wrong-paper** (14).

## Searches that returned nothing relevant

- `all:"proof amplification"` (arXiv site, 357 hits) — term occupied by unrelated amplification
  notions; **no proof-engineering hit**. Do not read this as "no such metric exists"; read it as
  "the phrase is not indexed in this sense".
- `all:"witness generation"` (arXiv API, 10 hits) — JSON Schema, ZK-SNARK, WCET, LLM bug witnesses;
  **no arithmetic `q`/`r` witness derivation**.
- `all:"Barrett reduction" AND all:verified` (arXiv API, 3 hits) — masked-hardware side-channel
  analyses only; **no canonical-residue automation**.
- `all:"primitive inferences"` (arXiv API, 10 hits) — machine-learning sense only; **no inference
  counting in machine-code verification**.
- Mutation-testing scholarly sources: **no query succeeded** (429 twice, host refusal, three 404s
  on guessed PDF paths). The taxonomy in the receipt rests on PIT's documentation plus the
  confirmed bibliographic existence of Jia & Harman (TSE 2011) and Just et al. (FSE 2014).
