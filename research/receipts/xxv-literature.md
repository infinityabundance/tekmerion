# XXV — Witness compilation, proof amplification, mutation, and kernel replay

**Date:** 2026-11-27. **Branch:** `staging` (read-only). **Type:** primary-source extraction.
Companion search log: `research/receipts/xxv-search-log.md`.

Purpose: evidence for (a) a witness compiler for the negation race (`x − r = q·p`,
`0 ≤ r < p`) and (b) an honest novelty register for the planned mechanisms.

## Method and limits

- Every quote below comes from a page fetched **in this session**; the URL is named in the block.
  Nothing is reconstructed from memory.
- Two PDFs returned undecoded bytes on direct fetch (`sos.pdf`) or 404 (`groebner.pdf`); the
  former was recovered through the `r.jina.ai` text proxy, the latter was not obtained.
- Semantic Scholar's API rate-limited (HTTP 429) on both mutation-testing queries. The UCL host
  for Jia & Harman (`www0.cs.ucl.ac.uk`) refused the connection. Consequently **the two canonical
  mutation-testing papers carry no quote here**; I mark them `QUOTE UNAVAILABLE` and rely on PIT's
  operator documentation (primary for the tool, `[secondary]` for the scholarly taxonomy).
- `api.github.com` was used to inspect the *test suites* of Carcara, cake_lpr, and HOL Light. This
  is primary evidence of what those repositories contain, not of any paper's claims.
- arXiv search is a strict AND over metadata; multi-term queries are unreliable negatives. The
  query `all:"proof amplification"` returned 357 hits in unrelated fields (signal/noise, PCP gap
  amplification), which is a term collision, not prior art.
- Repository receipts from earlier sessions (`xxiv-literature.md`, `ix70-algebraic-close.md`) are
  cited as `[repo receipt]` and are **not** re-quoted as primary evidence here.

---

## Q1 — Witness computation for modular arithmetic

**Answer (short):** No fetched work *derives* `q`/`r` from the arithmetic structure of a machine-code
implementation. Prior art splits into three shapes, none of which is Tekmerion's:

1. **CryptoLine / CoqCryptoLine**: the assembly model is **hand-written**, the carry/flag hints are
   **user-supplied**, and the only "witness" is an ideal-membership cofactor vector
   `c₀…cₙ` with `q = Σ cᵢpᵢ`, produced by an external CAS from the *human-authored polynomial
   system* and then validated in Coq.
   - Source: `https://link.springer.com/chapter/10.1007/978-3-031-37703-7_11` (CAV 2023, open access).
   - Quote: "The remaining 30 lines of annotations are hints to CoqCryptoLine, which then verifies
     the post-condition in 30 s with 24 threads." (23 words)
   - Quote: "hints only tell CoqCryptoLine what, not why properties should hold." (12 words)
   - Quote: "To compute the witness, bvCryptoLine relies on gbarith [32], where new variables are
     introduced." (17 words) — CoqCryptoLine instead uses Singular's `lift`.
   - **Refutation confirmed for the hand-supplied part:** models, pre/post-conditions, and the
     load-bearing carry annotations are all written by the user; the tool does not read them off
     the instruction stream.
   - Certification: the cofactors are checked by `norm_subst` inside Coq. The worksheet is a
     *polynomial* entailment, not a division theorem with a canonical residue.

2. **Fiat Cryptography**: it *does* compute the Montgomery quotient — but as part of a
   **hand-written high-level algorithm**, at Coq-compile time, from the algorithm's algebraic form.
   - Source: `https://r.jina.ai/https://adam.chlipala.net/papers/FiatCryptoSP19/FiatCryptoSP19.pdf`
     (IEEE S&P 2019 full text).
   - Quote: "which is very cheap to compute: the first factor is a compile-time constant, and S mod
     R is simply the limbs of S below R" (24 words)
   - The input is a Coq functional program (`wwmm_step`), specialised by partial evaluation to C.
     The quotient is not derived from a disassembly, and there is no LCF replay: the whole
     translation is inside Coq's kernel.
   - Bonus evidence that the canonical-residue obligation is a real failure mode: "an operation that
     produces a q and r s.t. x = qm + r but does not guarantee that r is the smallest possible"
     (24 words) — from the paper's survey of real crypto bugs (OpenSSL #1953).

3. **Barrett/Montgomery verified-reduction and "canonical residue" automation**: **not found.**
   - Query: `all:"Barrett reduction" AND all:verified` (arXiv API). 3 hits, all side-channel /
     masking verification of PQC hardware (arXiv 2604.15249, 2605.02856, 2604.25878). None proves
     a modular-reduction *correctness* theorem or derives a residue witness. `[secondary]`
   - No fetched paper reports automating the "conditional subtract if `x ≥ p`" residue
     normalisation for machine code.

4. **SMT/decision-procedure witnesses (Cooper, Omega)**: not established this session. The nearest
   fetched facts are (i) CoqCryptoLine's witnesses are CAS-produced cofactors, checked after the
   fact; (ii) Harrison's SOS pipeline has the same shape (external solver proposes, kernel checks).
   Classic Cooper/Omega witness production is *internal* to the decision procedure; proof-producing
   variants export a **proof**, not a `q`/`r` data witness. This last sentence is an **inference**,
   flagged as such; no Omega primary source was fetched.

**Nearest Tekmerion mechanism:** the premise fast path plus the unbuilt witness compiler.
**Distance:** Tekmerion would derive the division identity from instruction-derived equations and
replay it in an LCF kernel. Both halves exist separately (§Q5); the combination does not appear in
the fetched literature.

---

## Q2 — Proof amplification / certificate-operation counts

**Answer (short):** **No paper found** reports a ratio whose denominator is *certificate
(high-level) operations*. The canonical "de Bruijn factor" is a **text-size** ratio and is a
different quantity; it must not be cited as proof amplification.

- **de Bruijn factor — different thing.** Source: `https://www.cs.ru.nl/~freek/notes/` (Wiedijk's
  notes index, entry "The De Bruijn Factor", 5 pp.).
  - Quote: "This factor is determined by a combination of the amount of detail present in the
    original text and the expressivity of the system used to do the formalization." (28 words)
  - Quote: "For three specific examples this factor turns out to be approximately equal to four."
    (14 words)
  - Units: **bytes of informal text ÷ bytes of formal text** (a loss/compression factor between two
    *writings*). It is not expanded primitive inferences, and not certificate operations. The direct
    PDF (`.../notes/factor.pdf`) returned 404; the abstract above is from the index page.
- **"proof amplification"**: query returned 357 arXiv hits, all in unrelated senses (privacy
  amplification, gap amplification, signal amplification). **No relevant hit.** The term is
  unoccupied in proof-engineering, which is a mild positive for novelty but a strong negative for
  prior-art support.
- **"proof compression ratio" / "certificate steps versus proof steps"**: `[repo receipt]`
  `xxiv-search-log.md` rows 27–33 already logged this ground: proof-compression work measures DAG
  nodes, bytes, or resolution length, not per-certificate-operation ratios. Not re-searched.
- **Verdict:** no fetched source gives `A = I_expanded / N_certificate_operations`. If Tekmerion
  reports it, it should define it locally and say so; it cannot claim an established metric.

---

## Q3 — Mutation operators for checkers

**Answer (short):** The standard taxonomy exists and is stable; PIT's documentation is the fetched
primary-for-tool list. Substituting `todo!()` is **not** a standard operator, and mutating a
*checker's acceptance result* is standard in spirit but not standard practice for proof checkers.

**Taxonomy (fetched):** `https://pitest.org/quickstart/mutators/`
- Value replacement: "Replaces int, short, long, char, float and double return values with 0."
  (15 words) — see `PRIMITIVE_RETURNS`; `RETURN_VALS` maps `0→1`, otherwise `→0`, `Object→null`.
- Condition negation: "The negate conditionals mutator will mutate all conditionals found according
  to the replacement table below." (16 words) — `==`↔`!=`, `<`↔`>=`, etc.
- Guard deletion / forced branch: "The remove conditionals mutator will remove all conditionals
  statements such that the guarded statements always execute" (17 words) — and the documented
  `..._ELSE` specialisations force the branch **never** to execute.
- Statement deletion: `VOID_METHOD_CALLS` removes calls to void methods; `REMOVE_INCREMENTS`
  removes local increments.
- "Return a plausible value": `RETURN_VALS`, `EMPTY_RETURNS`, `FALSE_RETURNS`, `TRUE_RETURNS`,
  `NULL_RETURNS`, `PRIMITIVE_RETURNS`.
- Boundary shift: "The conditionals boundary mutator replaces the relational operators `<, <=, >, >=`
  with their boundary counterpart" (16 words).

**Scholarly anchors (both quote-unavailable):**
- Jia & Harman, *An Analysis and Survey of the Development of Mutation Testing*, IEEE TSE 37(5), 2011.
  `QUOTE UNAVAILABLE — page not fetched` (UCL host refused connection; Semantic Scholar 429).
- Just, Jalali, Inozemtseva, Ernst, Holmes, Fraser, *Are mutants a valid substitute for real faults
  in software testing?*, FSE 2014. Bibliographic existence confirmed on the author's publication
  page `https://homes.cs.washington.edu/~rjust/publ/`; `QUOTE UNAVAILABLE — PDF 404`.

**Panic substitution (`todo!()`) — answered as a judgement, not a citation:**
- It is **not** a standard named operator. It is closest to **statement replacement with an aborting
  expression**; if the abort is observable as a test failure, its kill condition is "some test
  reaches the site", i.e. **reachability / coverage**, not the strength of any assertion at that
  site. A checker that returns a *wrong verdict without aborting* is not detected by it.
- The operator that **does** probe assertion strength at a refusal site is a *silent-acceptance*
  mutant: force the boolean result to "accepted" (a `TRUE_RETURNS`-style mutation on the accept
  predicate). That is the mutation Tekmerion should use if the claim is about soundness, not
  coverage. This is my inference from the operator semantics; no fetched paper states it.

---

## Q4 — Adversarial negative testing of verified checkers

**Answer (short):** At least one concrete, maintained "should reject" suite exists (Carcara). The
pattern has no single standard name; "negative testing" / "regression tests for rejection" /
"soundness fuzzing" are all used loosely.

**Concrete, fetched:**
- **Carcara** — `https://raw.githubusercontent.com/ufmg-smite/carcara/main/tests/test_regression.rs`
  - Quote: "Cases in `valid/` must be accepted, and cases in `invalid/` must be rejected. No case
    may panic." (17 words)
  - Directory layout confirmed via `api.github.com/repos/ufmg-smite/carcara/contents/tests`:
    `elaboration/`, `regression/`, `rules/`, plus `test_*.rs`. The harness also has
    `known-bug` extensions that are `#[ignore]`d — i.e. acknowledged false-accept bugs are tracked
    explicitly. This is the strongest example found: a *reject* corpus, not just an accept corpus.
- **cake_lpr** — `https://api.github.com/repos/tanyongkiam/cake_lpr/contents/` shows
  `example.cnf` + `example.lpr` (accept case), `experiments/`, and no `tests/invalid` analogue at
  the repository root. **No "should reject" suite located**; absence in the root listing is not
  proof of absence in submodules (`cakeml/`, `HOL/` are submodules).
- **HOL Light** — `api.github.com/repos/jrh13/hol-light/contents/tests` returns **404**: there is no
  `tests/` directory at the repository root. **No kernel "should reject" suite located.**
- **MM0 / mm0-c** — not inspected this session. **Not established.**

**Naming:** the literature term that comes closest is *negative testing* (tests that must fail); for
checkers the security-flavoured term *soundness fuzzing* is used informally. I found no fetched paper
that standardises a name for "adversarial reject corpus for a verified checker". Record the pattern;
do not claim to have named it.

---

## Q5 — Sparse certificate replay in an LCF kernel

**Answer (short):** Yes for algebraic certificates replayed through HOL Light primitive rules —
Harrison's SOS work is exactly **(b)**, and it is the closest structural precedent for the witness
compiler's replay step. It is **not** about machine code.

**(b) Certificate replayed by kernel inference — fetched:**
- Harrison, *Verifying nonlinear real formulas via sums of squares*, TPHOLs 2007.
  `https://r.jina.ai/https://www.cl.cam.ac.uk/~jrh13/papers/sos.pdf` (full text, 17 pp.).
  - Quote: "the 'difficult' part, finding the certificates, can be done using highly optimized,
    unverified external programs. The theorem prover merely needs to verify the certificate"
    (27 words)
  - Quote: "we extract the SOS decomposition and prove the resulting identity in HOL" (13 words)
  - Certificate shape: Positivstellensatz / Nullstellensatz — `P + Q + R² = 0` with `P` in the
    **ideal generated by the hypotheses**, i.e. a sum of *ideal-membership cofactors* times
    generators. This is the same algebraic object as CoqCryptoLine's `c₀…cₙ`, but here it is
    replayed inside HOL Light's kernel rather than checked by a separate procedure.
  - Cost data reported are wall-clock, not inferences: "around one second", "0.06 seconds",
    "1.25 seconds"; the deep example is a "38 ×38 matrix … parametrized by 143 variables".
- Harrison, *Automating elementary number-theoretic proofs using Gröbner bases*, CADE 2007: title
  and venue confirmed on `https://www.cl.cam.ac.uk/~jrh13/papers/`; **PDF not obtained**
  (`groebner.pdf`, `grobner.pdf` both 404). Its likely relevance (Gröbner cofactors → HOL Light
  proof) is stated on the title alone and is **not** quoted.
- Isabelle `linarith` (Chaieb & Nipkow 2008) is proof-producing, hence **(b)**, `[repo receipt]`
  `xxiv-literature.md` §5.

**(a) Certificate checked by a separate/verified checker — fetched or `[repo receipt]`:**
- CoqCryptoLine: cofactors validated by `norm_subst` in Coq (a verified checker, not LCF primitive
  rules). Q1 source above.
- Carcara (Alethe), cake_lpr (LPR), VeriPB/CakePB: standalone checkers with their own proof formats.
  Carcara's test harness is quoted in Q4; the others are `[repo receipt]` `xxiv-*`.

**Distinction that matters for the register:** Tekmerion's planned replay is **(b)** on
*machine-instruction-derived* linear equations. Harrison's is **(b)** on *mathematical* polynomial
hypotheses. CoqCryptoLine is **(a)** on *machine-code models*. No fetched work is **(b) + machine
code**.

---

## Q6 — Negation / conditional-subtraction idioms

**Answer (short):** **Not found** — no fetched work *recognises* "conditional subtract `p` if
`x ≥ p`" or "conditional add `p` if borrow" in machine code and generates the proof
automatically. Every neighbour supplies the pattern by hand at a higher level.

- **Fiat Cryptography**: the conditional add of a multiple of the modulus is written by the human as
  part of `wwmm_step` / the Montgomery algorithm, then specialised. Quote (Q1 source):
  "which is very cheap to compute: the first factor is a compile-time constant" — the *structure* is
  design knowledge, not a recognised idiom. Also, its saturated/modular-reduction text notes the
  manual workaround for negative coefficients and limb underflow.
- **s2n-bignum**: each function has a separate, human-written HOL Light script; no idiom matcher.
  Quote: "each function is accompanied by a machine-checked formal proof in HOL-Light that its
  mathematical result is correct" (18 words)
  — `https://raw.githubusercontent.com/awslabs/s2n-bignum/main/README.md`.
  The README describes automatic *optimisation* (SLOTHY) but no automatic *proof/idiom* recognition.
- **CryptOpt**: randomised search over assembly plus a verified equivalence checker; it *synthesises*
  instruction sequences, it does not recognise a human idiom and prove it.
  Quote: "we apply randomized search through the space of assembly programs, with repeated automatic
  benchmarking on target CPUs" (18 words) — `https://arxiv.org/abs/2211.10665`.
- **HACL\*, Jasmin, Vale**: not fetched this session; from the Fiat related-work text they are
  source-level or handwritten-assembly verification lines, not idiom recognisers. `[secondary]`

**Consequence for the negation race:** "the negated operand is canonicalised by a conditional
subtract" is exactly the fact a witness compiler needs; the literature gives it as a hand-written
lemma/algorithm step, not as an inferred one.

---

## Q7 — Step-theorem caches and inference counts

**Answer (short):** **Not found** beyond Fox ITP 2015 (as the brief already states). No fetched
machine-code verification paper reports *primitive inference counts* for symbolic simulation or
decompilation.

- Query `all:"primitive inferences"` (arXiv API): 10 hits, all NLP/DNN "primitive inference
  patterns". **No machine-code or ITP hit.**
- `[repo receipt]` `xxiv-literature.md` §3–4 records that Myreen/Decompilation-into-Logic and the
  CPP 2013 bignum pearl report proof length in **lines**, not inferences.
- HOL Light's own output does not carry a global primitive-inference counter by default, so a
  "matched inference" metric has to be instrumented locally `[repo receipt]`
  `ix60-matched-inference.md`.
- Harrison's SOS paper reports seconds, not inferences (Q5/Q8).

**Verdict:** Fox ITP 2015 remains the only named precedent for inference-count tables in this
neighbourhood. Treat "inference counts" as a deliberate, unusual reporting choice, not a norm.

---

## Q8 — Cheapest sound close for linear integer goals in HOL Light

**Answer (short):** No published inference-count comparison was found. Published cost data are
wall-clock and are for *nonlinear* SOS or for *reflection vs proof production*, not for
`INT_ARITH_TAC` vs a hand-built linear combination.

- **HOL Light's linear procedures** (identification only, from the fetched publications list at
  `https://www.cl.cam.ac.uk/~jrh13/papers/`): "A HOL decision procedure for elementary real algebra"
  (1993), "A Proof-Producing Decision Procedure for Real Arithmetic" (CADE-20, 2005), and *Theorem
  Proving with the Real Numbers* (1998). These establish that the family is **proof-producing**
  (hence kernel-inference heavy). The internals (`int_arith` is Cooper-style quantifier elimination)
  are **an inference from the paper titles and standard HOL Light practice**; I did not fetch the
  implementations. Flagged.
- **Measured ratio, different procedures:** Chaieb & Nipkow report reflection is "between one and
  two orders of magnitude faster" than proof-producing linear arithmetic (Isabelle).
  `[repo receipt]` `xxiv-literature.md` §5. Units: wall-clock ratio; **no inference counts**.
- **Measured wall-clock, nonlinear:** Harrison's `REAL_SOS` — 0.06 s for a quadratic bound, 1.25 s
  for a cubic lemma, "around one second" for the 143-variable example. Units: seconds; machine
  unspecified beyond contemporaneous hardware.
- **Local evidence, no numbers:** `[repo receipt]` `ix70-algebraic-close.md` — `ASM_INT_ARITH_TAC`
  `Stack_overflow` on the word-residue system after 0 SAT closes; **no inference total exists**.
  That is a failure datum, not a cost measurement.

**Practical reading (inference, clearly labelled):** the literature supports "reflection beats
proof-producing QE by 1–2 orders of magnitude in time" and supports "linear proof production is
inference-heavy"; it does **not** supply a number for hand-built linear combination vs
`INT_ARITH_TAC`. Tekmerion must measure its own `I_expanded` for both arms on the same goal.

---

## Novelty / inversion probes

Classification vocabulary: `EXISTING` (found, same thing), `KNOWN_COMBINATION` (both halves
independently published and the combination is routine), `UNVERIFIED_COMBINATION` (halves exist,
combination not found, searches thin), `PLAUSIBLY_NOVEL` (logged search found nothing close).

| # | Probe | Nearest neighbour(s) | Class |
|---|-------|----------------------|-------|
| 1 | Derive canonical-residue witnesses from instruction-derived equations, replay in an LCF kernel, for machine code | CoqCryptoLine (machine-code model + CAS cofactors + Coq checker); Harrison SOS (LCF replay of ideal-cofactor certificates, but over mathematical polynomials) | **UNVERIFIED_COMBINATION** |
| 2 | Per-carrier premise selection by a certificate's own coefficient support (`λᵢ ≠ 0`) inside LCF search | CoqCryptoLine "Elimination of ideal generators through variable substitution"; sparse linear-combination/Farkas practice; no support-driven selection inside an LCF search found | **UNVERIFIED_COMBINATION** |
| 3 | "Proof amplification" `A = I_expanded / N_certificate_operations` reported with inference counts | de Bruijn factor (bytes÷bytes); proof-compression work (nodes/bytes) | **PLAUSIBLY_NOVEL** (weak: one term-colliding engine query, de Bruijn confirmed different) |
| 4 | Silent-acceptance mutation applied to a *verified checker's* refusal sites | PIT `TRUE_RETURNS`/`RETURN_VALS` (mutation testing); Carcara `invalid/` suite (reject corpus, not mutation) | **UNVERIFIED_COMBINATION** |
| 5 | Column-wise polynomial reduction (AMulet) + machine-instruction partial products + LCF replay | AMulet2 (column-wise, Coq/Frama-C, C source); Fiat (associational→columns, Coq only); CryptOpt (asm + Coq equivalence); Harrison (LCF replay) | **UNVERIFIED_COMBINATION** |

Notes on the strongest five untried combinations (for the register):
1. Instruction-derived `q`/`r` + LCF replay (probe 1) — the negation mechanism itself.
2. Support-driven premise selection keyed to `λᵢ ≠ 0` (probe 2) — cheap, testable, no neighbour.
3. Amplification metric with a certificate-operation denominator (probe 3) — define locally.
4. Reject-corpus mutation aimed at accept-sites, not panics (probe 4) — soundness, not coverage.
5. Column-wise reduction over machine partial products with kernel replay (probe 5) — the only
   probe where all three components are individually well developed.

Do **not** describe probes 1, 2, 4, 5 as novel on the strength of this session: the searches were
narrow (one general engine, one strict-AND arXiv interface), and each half is published.

---

## What I could not establish

- **Jia & Harman and Just et al. verbatim text.** DOI was mis-remembered on the first attempt (the
  DOI resolved to *How We Refactor, and How We Know It*), the UCL host refused connection, and
  Semantic Scholar returned 429. The taxonomy is therefore supported by PIT's documentation and by
  the papers' bibliographic existence, not by their prose.
- **Harrison's Gröbner-basis paper body.** Title/venue only. The claim that it replays ideal
  cofactors through HOL Light rules is plausible from the SOS paper's references and the title, but
  unverified. Do not cite it for that.
- **MM0/mm0-c reject tests, CakeML/Candle kernel reject tests, HOL4 kernel test suites.** Not
  inspected. HOL Light has no root `tests/` directory (404), which is *absence at one path*, not
  proof of no test suite elsewhere.
- **Whether any Barrett/Montgomery correctness proof automates residue canonicalisation.** The
  arXiv query surfaced only side-channel work. A dedicated search of the verified-crypto
  literature (HACL\*, fiat-crypto, Jasmin, Vale) would be needed to call this a firm negative.
- **Any paper reporting primitive inference counts for machine-code verification other than Fox
  ITP 2015.** One arXiv query, one negative; treat as "not found", not "does not exist".
- **A primary source for Cooper/Omega witness extraction.** The internal-vs-exported distinction is
  an inference from the fetched CoqCryptoLine and SOS material.
- **Cost of `INT_ARITH_TAC` vs a hand-built linear combination in HOL Light.** No published number
  in any unit found; only the Isabelle 1–2 order-of-magnitude reflection ratio.
