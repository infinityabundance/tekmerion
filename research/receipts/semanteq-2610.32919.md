# Receipt — SemantEq (arXiv 2610.32919)

- **URL:** <https://arxiv.org/abs/2610.32919>
- **Fetched:** 2026-11-22
- **Submitted:** 2026-10-23 (v1)
- **Type:** primary (author abstract). Full text not read; the *numbers below are the authors'*.
- **Authors:** Xinrui Li, Pawan Sasanka Ammanamanchi, Siddharth Bhat, Rianna Jamdar,
  Yongxi Lin, Stella Biderman

## Exact quote

> SemantEq-Eval, a search without a model, reaches 100% precision and 65.6% recall on the
> ProofNetVerif test split under our adjudicated labels, against 95.8% and 61.8% for BEq+, and
> an 8.9-fold increase in its CPU time adds at most three pairs.

Supporting quotes:

> We introduce SemantEq, a more principled relation that instead requires each statement to turn
> into the other only through a chain of allowed steps, such as logical rearrangement and
> rewriting with the library's own equations. Our programs judge a proof by the proof term Lean
> checks, not by its tactic script.

> SemantEq-ProofAudit audits each model-written proof under a policy derived from the relation
> and accepts none of the 104 non-equivalent pairs, and it raises recall on 407 equivalent pairs
> from 65.4% to 81.1% under the best prompt.

## What it establishes

- The reported figures (100% / 65.6% versus BEq+ 95.8% / 61.8%) reproduce **exactly** as
  stated, on the ProofNetVerif test split, for `SemantEq-Eval`.
- SemantEq is a *relation on statements*, checked on proof terms rather than tactic scripts.
- The direction of the result matters: SemantEq is a **soundness-oriented** relation (high
  precision), and BEq+ is comparable on precision but lower on recall. SemantEq does not solve
  recall on its own; a model plus `ProofAudit` raises it.

## What it does not establish

- Independent evaluation. The labels are **"our adjudicated labels"** — the authors' own
  adjudication, not a third-party gold standard.
- That SemantEq is complete for "faithful translation". It is a conservative *admissibility*
  relation: a statement pair can be rejected and still be equivalent.
- That it applies to object-code specifications, s2n-bignum, or HOL Light. It is a Lean-4
  statement relation.
- That its transforms preserve *proof difficulty* or *meaning for the reader*; only that they
  preserve the statement up to the allowed steps.
