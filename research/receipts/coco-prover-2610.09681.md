# Receipt — CoCo-Prover (arXiv 2610.09681)

- **URL:** <https://arxiv.org/abs/2610.09681>
- **Fetched:** 2026-11-22
- **Submitted:** 2026-10-07 (v1)
- **Type:** primary (author abstract)
- **Authors:** Shuangjie Yao, Nikolaus Holzer, Mark Paul Santolucito, Baishakhi Ray,
  Suman Jana, Dongdong She

## Exact quotes

> Almost all existing provers, however, pursue pass rates alone at whatever sampling or search
> budget it takes, and overlook the success-vs-cost frontier; yet real software often carries
> hundreds of interdependent proof obligations, so what matters at scale is not whether one
> theorem can be proved, but how many can be proved economically.

> We introduce CoCo-Prover, which formalizes cost-efficient program proving as metalevel
> decision-making under cost, grounded on two-level proof graphs: an AND/OR proof hypergraph
> within each declaration is joined to a lemma-dependency graph across declarations; and at each
> step, it answers two questions: which open goals to select, and which actions to purchase on
> these goals.

> Selection stays symbolic as a topological pass over the proof graphs. Action choice is agent
> orchestration via metalevel decision-making: an agentic router treats every bounded specialist
> invocation as a separately priced, best-effort computation, matching heterogeneous specialist
> agents together with configurations, under evolved routing rules as evidence accumulates.

> On five program verification benchmarks in Lean 4 including function-level CLEVER, VERINA, and
> AlgoVeri, and repository-level NTP4VC and Vero, we show that CoCo-Prover achieves a better
> success-vs-cost frontier than baselines including frontier coding agents and state-of-the-art
> LLM-based provers: it achieves the best solve rate on every benchmark and up to 100% on two
> benchmarks. It also reduces cost by up to 30.9% compared to the strongest baseline with the
  strongest LLM in our evaluation.

## What it establishes

- CoCo-Prover exists and targets exactly the "success at what cost" axis, in Lean 4, over
  program-verification benchmarks.
- Two-level proof graphs (intra-declaration AND/OR + inter-declaration lemma dependency) are
  already used for goal selection and action pricing.
- Up to 100% on two benchmarks; up to 30.9% cost reduction versus the strongest baseline.

## What it does not establish

- That the same cost frontier holds on object-code proofs or HOL Light; the substrate is Lean 4.
- That its routing rules transfer; they are "evolved" per evidence, i.e. fitted.
- Anything about kernel-call budgets. Cost is not kernel calls.
- That goal-selection-by-proof-graph is novel; it is thereby occupied, and N1/N2 must
  differentiate from it.
