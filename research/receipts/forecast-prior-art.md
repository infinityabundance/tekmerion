# Receipt — prior art on predicting post-tactic state

- **Source:** <https://arxiv.org/abs/2410.11133> (3D-Prover), fetched 2026-11-22.
- **Title:** *3D-Prover: Diversity Driven Theorem Proving With Determinantal Point Processes*

## Verbatim

> We first demonstrate that it is possible to generate semantically aware tactic
> representations which capture the **effect on the proving environment, likelihood of
> success, and execution time**.

## What this establishes

Predicting a tactic's effect on the proof state — not merely the next tactic — is published.
Therefore:

- E4 may **not** claim novelty for "predicting the post-tactic goal state".
- The only defensible novelty is narrower: using the **forecast error online, per region, as a
  trust and calibration signal**, and showing it raises kernel-accepted proofs at equal compute
  on an industrial machine-code corpus.

## Other prior art that must be cited alongside it

- **Xin et al. (2024)** — used successful environment responses as an auxiliary training
  objective. Reported by an external reviewer; not independently fetched here.
- **"Learning Proof Transformations and Its Applications in Interactive Theorem Proving"**
  (2023, Springer) — learned structural descriptions of proof-state transformations for tactic
  prediction and proof shortening. Reported by an external reviewer; not independently accessed.

## Consequence

Rewrite the forecast-residual hypothesis as:

> Does a model's prediction error about a tactic's effect identify when its future
> recommendations are unreliable, and does using that signal increase kernel-accepted proofs
> under equal compute?

That is the claim to test. Anything broader is already occupied.
