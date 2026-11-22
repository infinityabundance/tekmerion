# Receipt — Aria / "Harnessing Code Agents for Automatic Software Verification"

- **URL:** <https://arxiv.org/abs/2607.06341>
- **Fetched:** 2026-11-22
- **Submitted:** 2026-07-07 (v1)
- **Type:** primary (author abstract; full text not read)
- **Authors:** Shuangxiang Kan, Shuanglong Kan, Sebastian Ertel

## Exact quotes

**The design claim (no fixed strategy).**

> We show that imposing such a strategy is unnecessary and limiting. Handing the whole lemma to
> a general LLM code agent (for example, Claude Code), free to choose its own approach, and
> wrapping it in a verification harness is both simpler and more effective, achieving full
> coverage: every targeted lemma proved, with no failures and no Coq expert intervention.

**The harness contract (sound / complete / terminating).**

> The agent writes the proofs under feedback and hard constraints from the harness that keep
> each one sound (accepted only when the prover's kernel closes it), complete (no obligation
> left unproved or silently dropped), and terminating (no divergent tactics).

**The numbers.**

> (1) Core logic: on Iris, the state-of-the-art separation logic for concurrent and
> memory-manipulating programs, Aria proves all 4,257 lemmas of the four core modules and the
> 217 lemmas verifying Rust's standard libraries built on it, fully automatically. (2)
> Comparison with prior LLM provers: on reglang, where prior provers manage barely one in
> eight, Aria proves all 318. (3) Generality: on iris-lean, the unfinished Lean 4 port of Iris,
> it proves 72 not-yet-ported lemmas.

## What it establishes

- 4,257 Iris core-module lemmas and 318 `reglang` lemmas, all proved, no failures.
- The harness is *kernel-closing*: acceptance requires the kernel to close the proof; a
  proof obligation may not be dropped; divergent tactics are rejected.
- No fixed proof strategy is imposed on the agent.

## What it does not establish

- That the agent is more sample-efficient, only more successful at the stated coverage.
- Anything about object code, machine-code semantics, or HOL Light.
- Independent reproduction. The counts are the authors' own.
- Two numbers the brief omits: 217 Rust-stdlib lemmas and 72 `iris-lean` lemmas.
