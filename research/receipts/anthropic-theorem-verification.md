# Receipt — Anthropic + Theorem, "LLMs are making formal software verification feasible at a much larger scale"

- **URL:** <https://www.anthropic.com/research/formal-verification>
- **Fetched:** 2026-11-22
- **Published:** 2026-10-22
- **Type:** primary (first-party institutional report). **Not peer-reviewed.**
- **Team:** Anthropic Frontier Red Team, in collaboration with Theorem.

## Exact quotes

**Scope of the campaign.**

> Using Claude, we proved security properties at the assembly code level (RISC-V) in over 100
> real-world open-source software libraries.

> Over the span of a few weeks, **Claude successfully patched the bugs it found and then proved
> that there are no crashes on any input (up to 64 MB) for 239 entry points across 123
> different OSS-Fuzz libraries.**

**The property being proved.**

> To find all the bugs that would've been surfaced by a fuzz test, we turned the fuzz test into
> a spec defined over the compiled RISC-V instructions. The spec says that the fuzz test won't
> crash for _any_ input less than 64 MB.

**Proof volume.**

> These proofs operate on the compiled RISC-V instructions (i.e., executable binaries), and they
> comprise about 180 million lines of Rocq, with a typical proof running to about 600,000 lines
> to provide security guarantees on binaries with a median length of 18,000 RISC-V instructions.

**Mechanism.**

> These proofs are built by translating the RISC-V assembly code to a mathematical model using
> Theorem's RISC-V instruction and Linux process semantics.

**Environment restriction.**

> This means these proofs currently assume that the program runs as a single-threaded static
> binary with a fixed memory budget on a RISC-V CPU using Linux OS.

**Limitations, in their words.**

> **"Formally verified" does not mean "bug-free."** Claude can write a proof showing that a
> compiled program's behavior satisfies the requirements of a formal spec (which is verified by
> an external proof checker). However, a proof checker can't tell you whether your spec matches
> reality.

> We proved that a crash never triggers for any input under 64 MB on many fuzzing entry points
> in Google OSS-Fuzz, including ASan crashes. However, ASan doesn't detect all memory safety
> bugs, and we didn't verify all the entry points for each library.

**Counter-evidence to "no functional correctness" for the programme as a whole.**

> It's common for fuzz tests in real-world code to test for violations of meaningful high-level
> properties, such as encode-decode roundtripping, idempotence, algebraic laws, differential
> testing, self-consistency, or equivalence to reference implementations.

> We asked Claude to write a formal spec of RFC 1950, the technical document that defines the
> zlib compression file format. Then, we had Claude prove that the zlib implementation in the
> Linux kernel correctly implements that spec.

## What it establishes

- 123 libraries / 239 entry points, RISC-V, Rocq, ~180M lines, ~600k lines per median
  18k-instruction binary.
- The 239-entry-point campaign's *specification* is "the fuzz test never crashes" (ASan
  included), under single-threaded static linking, Linux, fixed memory budget.
- The authors say explicitly that this is not bug-freedom and that ASan is incomplete.

## What it does not establish

- That the whole programme proves no functional properties. A separate reported experiment
  proves an x86 binary implements RFC 1950 (zlib), which is functional correctness.
- Anything about constant time: the post neither claims nor excludes it. **Absence of a claim,
  not a claim of absence.**
- Peer review or independent reproduction.
