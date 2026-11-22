# Receipt — Semantics-preserving transforms as contamination probes (N12 prior art)

N12 proposes "SemantEq-preserving transforms as a contamination probe; original-minus-transform
gap is the memorisation score." The *mechanism* — semantic-preserving program transformation as a
contamination/robustness probe — is already published in at least four places. These receipts
exist to prevent N12 being described as new.

## 1. CodeMorph — arXiv 2506.17627 (ICSE 2025, Industry Challenge Track)

- **URL:** <https://arxiv.org/abs/2506.17627>
- **Fetched:** 2026-11-22, submitted 2025-06-21
- **Type:** primary (author abstract). Authors: Hongzhou Rao, Yanjie Zhao, Wenjie Zhu, Ling Xiao,
  Meizhen Wang, Haoyu Wang.

> Consequently, generating new datasets through code perturbation has become essential.

> The first component employs 26 semantic-preserving transformation methods to iteratively
> perturb code, generating diverse variations while ensuring that the modified code remains
> compilable.

> Experimental results demonstrate that after applying CodeMorph, the accuracy of the LLM on code
> completion tasks across five programming languages decreased by an average of 24.67%, with
> Python showing the most significant reduction at 45%.

**Establishes:** 26 semantics-preserving transforms, used to measure the drop in model accuracy —
i.e. exactly the "original minus transform" gap — for code benchmarks.
**Does not establish:** transform admissibility defined by a proof-assistant equivalence relation,
or application to proof-search tasks.

## 2. "Is Your Benchmark Still Useful? Dynamic Benchmarking for Code Language Models" — arXiv 2503.06643

- **URL:** <https://arxiv.org/abs/2503.06643>
- **Fetched:** 2026-11-22, submitted 2025-03-09
- **Type:** primary (author abstract). Authors: Batu Guan, Xiao Wu, Yuanyuan Yuan, Shaohua Li.

> Given a code understanding or reasoning benchmark, our framework dynamically transforms each
> input, i.e., programs, with various semantic-preserving mutations to build a syntactically new
> while semantically identical benchmark.

> Our evaluation reveals several interesting or surprising findings: (1) all models perform
> significantly worse than before, (2) the ranking between some models shifts dramatically, and
> (3) dynamic benchmarks can resist against the data contamination problem.

**Establishes:** the explicit claim that semantic-preserving mutation resists contamination —
the same thesis as N12.

## 3. RustMizan — arXiv 2607.04729

- **URL:** <https://arxiv.org/abs/2607.04729>
- **Fetched:** 2026-11-22, submitted 2026-07-06
- **Type:** primary (author abstract). Authors: Tarek Elsayed et al.

> A paired mutation framework produces semantics-preserving code mutants for contamination
> testing and robustness probing.

**Establishes:** even the *phrase* "semantics-preserving mutants for contamination testing" is
in print (July 2026).

## 4. Haroon et al. — arXiv 2504.04372

- **URL:** <https://arxiv.org/abs/2504.04372>
- **Fetched:** 2026-11-22, submitted 2025-04-06
- **Type:** primary (author abstract).

> For each successfully localized program, we apply semantic-preserving mutations (SPMs) and
> rerun localization to assess robustness and determine whether LLM reasoning relies on syntactic
> cues rather than semantics.

**Establishes:** SPMs as a robustness/contamination probe across many tasks; 78% of previously
localized faults fail after SPMs.

## Consolidated implication

N12 as stated is **EXISTING** in mechanism. What is not claimed anywhere I found is the specific
instantiation: transforms whose admissibility is decided by **SemantEq on Lean statements**, and
the gap measured on a **proof-search** benchmark whose endpoint is a kernel-accepted proof. That
residual is a combination, not a mechanism, and must be described as such.
