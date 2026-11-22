# Receipt — GPU-accelerated Lean feedback and dense verifier-in-the-loop reward

Three papers are grouped because each is cited in the novelty register for N4 (dense
kernel-grounded reward) and/or N11 (GPU-parallel checking). They are **different** things and
must not be conflated.

## 1. "Learning Lean Feedback for Efficient Proof-Candidate Evaluation on GPUs" — arXiv 2610.21881

- **URL:** <https://arxiv.org/abs/2610.21881>
- **Fetched:** 2026-11-22, submitted 2026-10-15
- **Type:** primary (author abstract). Authors: Lazar Milikic, Etienne Bamas, Emmanuel Abbe,
  Viktor Kunčak. NeurIPS 2026 Workshop on VeriCodeGen.

> We train neural evaluators to predict Lean 4 verdicts and compiler feedback on GPUs using
> 4.84M labelled programs.

> A two-stage evaluator that screens candidates with a classifier and scores its positive
> predictions with the feedback model reaches 87.8% accuracy in predicting whether Lean accepts a
> candidate proof, evaluated on a 68,932-program benchmark collected from public provers.

> The combined evaluator processes 3.5 candidates per second on one H100, 4.2x the throughput of
> our 64-worker Lean baseline with environment reuse.

> Finally, we combine our evaluator with Lean verdicts to obtain a hybrid reward system that
> shortens GRPO training steps by approximately 31%. After one GRPO epoch with hybrid rewards,
> the prover achieves nearly the same miniF2F pass@1 as with Lean-only rewards (62.2% versus
> 62.4%) and a pass@32 of 72.4% versus 71.3%.

**Establishes:** GPU-side *prediction* of Lean verdicts at 87.8% accuracy and 4.2x throughput;
a hybrid (predicted + real) reward with ~31% fewer GRPO steps at equal final pass@1.
**Does not establish:** GPU *certificate checking*. This is a learned surrogate whose errors are
tolerated by mixing in real verdicts; it cannot certify a theorem.

## 2. "Local Look-Ahead Guidance via Verifier-in-the-Loop for Automated Theorem Proving" — arXiv 2503.09730

- **URL:** <https://arxiv.org/abs/2503.09730>
- **Fetched:** 2026-11-22, submitted 2025-03-12
- **Type:** primary (author abstract). Authors: Sara Rajaee, Kumar Pratik, Gabriele Cesa,
  Arash Behboodi. Findings of ACL 2025.

> In this work, we focus on the Automatic Theorem Proving (ATP) task and propose a novel
> verifier-in-the-loop design, which, unlike existing approaches that leverage feedback on the
> entire reasoning trajectory, employs an automated verifier to give intermediate feedback at each
> step of the reasoning process.

**Establishes:** dense, verifier-grounded, per-step reward for ATP is published. N4's mechanism
family is occupied.
**Does not establish:** the specific residual formula
`r_t = -ΔGoalComplexity - λ·KernelCalls + μ·CertificateProgress`, nor an object-code substrate.

## 3. "Safe: Enhancing Mathematical Reasoning ... via Retrospective Step-aware Formal Verification" — arXiv 2506.04592

- **URL:** <https://arxiv.org/abs/2506.04592>
- **Fetched:** 2026-11-22, submitted 2025-06-04
- **Type:** primary (author abstract). Authors: Chengwu Liu et al. ACL 2025.

> Rather than assigning arbitrary scores, we strive to articulate mathematical claims in formal
> mathematical language Lean 4 at each reasoning step and provide formal proofs to identify
> hallucinations.

**Establishes:** step-level formal (Lean) verification as a reward/signal is published.
**Does not establish:** search-time reward shaping for a prover; the object here is
chain-of-thought verification.

## Consolidated implication

For N4, "dense kernel-grounded reward" is **not** a novel mechanism. The unclaimed residual is
the exact residual definition, applied to machine-code (object-code) proof obligations in HOL
Light, compared against sparse reward at an equal *kernel-call* budget. For N11, none of these
papers checks a certificate on a GPU; the GPU work is either prediction (2610.21881) or kernel
execution for candidate evaluation (RESOLVE-style), not proof-certificate checking.
