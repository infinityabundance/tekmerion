# Receipt — AlphaProof Code (Google DeepMind), 2026-11-10

- **URL:** <https://google-deepmind.github.io/code-verification/>
- **Fetched:** 2026-11-22
- **Date on page:** 2026-11-10
- **Type:** primary (authors' technical report and artifact site). **Not peer-reviewed.**
- **Authors on page:** Swarat Chaudhuri, Andrew Ferraiuolo, Anton Kovsharov, Tej Chajed,
  Sam Schlesinger, Karn Seth, Jonathan Katz, Charles Sutton, Pushmeet Kohli
  (Google DeepMind; Google).

## Exact quotes

**Scope and headline.**

> We introduce AlphaProof Code, a multi-agent AI framework for formal software specification
> and verification that we used to prove key security properties of `libsignal`, the Signal
> protocol's reference implementation, at the machine-code level.

**Volume.**

> Applied to `libsignal`, our agents generated about 6.3 million lines of machine-checked Lean
> proof code, resting on about 27,000 lines of trusted specifications and AArch64 semantics.

**Counting rule.**

> The repository contains over 6.3 million lines of Lean, counting comments and blank lines.

**Executable and compiler.**

> We link `libsignal` into a small command-line harness and compile it with the stock `rustc`
> compiler to AArch64 Linux machine code, verifying the linked executable directly down to its
> system calls.

> Notably, however, the Rust compiler is _not_ trusted.

**Mutation-based specification testing.**

> Third, _synthetic mutants_ apply mutation testing [47, 64] to specifications: agents inject
> small bugs into the code, such as skipping ciphertext MAC verification in Signal, record the
> resulting concrete execution traces, and prove in Lean that the specification rejects those
> traces (so far for 16 of 17 mutants, each for one witness trace under a pinned concrete
> primitive interpretation).

**The limit of that testing (§6).**

> ...their 17 full binary disproof obligations (against the placed helper) remain open `sorry`
> stubs in the non-default library.

**Statement/tcb checking (§2.4).**

> Inspecting the theorem statement alone is not enough to check this, since a statement can be
> weakened indirectly by changing a definition it depends on.

> The gatekeeper accepts a proof only if `comparator` confirms that the statement and every
> definition it depends on are identical to this copy, that the proof uses only Lean's standard
> axioms, and that Lean's kernel accepts the proof.

## Additional quotes used by the novelty register

**Falsify before proving (§2.3) — cited for M-C and M-E.**

> Agents also try to falsify a goal before work on it starts, to avoid wasting resources on
> unprovable subgoals.

**Bit-vector side conditions in the kernel (§C.3 proof outline) — cited for M-B.**

> Contracts are proved by symbolic execution with a weakest-precondition calculus (callee
> contracts, loop invariants, the frame rule, and bit-vector side conditions proved in Lean's
> kernel).

## What it establishes

1. ~6.3M lines of machine-checked Lean about compiled AArch64 code of a `libsignal`-based CLI,
   with cryptographic, allocator, formatting and I/O routines linked and verified, not hooked
   (18 panic entry points overridden).
2. Explicit assumptions are published (Table 1 "Assumptions"; §5.3 trusted computing base).
3. Mutation-based specification testing exists in the workflow, and its **exact** current
   strength is 16/17 mutants at the specification level, one witness trace each.

## What it does not establish

- That the 6.3M figure is non-comment code. The report says it counts comments and blank lines.
- That the mutant suite is backed by full binary refutations. Those are `sorry` stubs in a
  non-default library that CI does not build and `comparator` does not replay.
- Peer review, or independent reproduction of the build.
- Anything about s2n-bignum, HOL Light, or x86.
