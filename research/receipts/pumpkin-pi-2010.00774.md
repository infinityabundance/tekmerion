# Receipt — Proof repair by transport across equivalences (PUMPKIN Pi), arXiv 2010.00774

- **URL:** <https://arxiv.org/abs/2010.00774>
- **Fetched:** 2026-11-22
- **Submitted:** 2020-10-02 (v1); v2 2021-05-11
- **Type:** primary (author abstract)
- **Authors:** Talia Ringer, RanDair Porter, Nathaniel Yazdani, John Leo, Dan Grossman
- **Venue metadata on page:** DOI `10.1145/3453483.3454033` (PLDI 2021)

## Exact quotes

> We describe a new approach to automatically repairing broken proofs in the Coq proof assistant
> in response to changes in types.

> Our approach combines a configurable proof term transformation with a decompiler from proof
> terms to tactic scripts. The proof term transformation implements transport across equivalences
> in a way that removes references to the old version of the changed type and does not rely on
> axioms beyond those Coq assumes.

> We demonstrate PUMPKIN Pi's flexibility on eight case studies, including supporting a benchmark
> from a user study, easing development with dependent types, porting functions and proofs
> between unary and binary numbers, and supporting an industrial proof engineer to interoperate
> between Coq and other verification tools more easily.

## What it establishes

- **Proof-term transport along an equivalence, with zero proof search**, is a published,
  implemented mechanism (PUMPKIN Pi). N5's core operation therefore exists.
- It removes references to the old type and adds no axioms beyond Coq's.

## What it does not establish

- Transport along a *code* diff whose equivalence is a machine-checked object-code theorem
  (`ensures2` in s2n-bignum), rather than a type equivalence.
- Transport between *logics* (Coq only here) or between *ISAs*.
- Any guarantee that the transported proof's *statement* is unchanged; PUMPKIN Pi changes types,
  which changes statements by construction. N5's additional requirement — a guard that the
  theorem statement is unchanged — is not addressed.
