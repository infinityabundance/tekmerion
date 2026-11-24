# Receipt — IX.43 / Milestone B trusted-base report

- **Repository:** the `tekmerion` source tree (paths are repository-relative).
- **Branch:** `staging` (committed; the original draft said "not committed, not pushed" — that was true when written and is corrected here).
- **Date:** 2026-11-23 (status lines added 2026-11-24).
- **Toolchain:** `rustc 1.98.0`, `cargo 1.98.0`
- **New crates:** none. `Cargo.toml` still has exactly one direct dependency,
  `sha2 = "0.10"` (locked 0.10.9).
- **Verdict claimed:** *not verified*. The abstract relation is specified and
  sound on paper; the executable refines it only under G1–G4 below.

## 1. Trusted files

Line counts are from `wc -l` on the working tree; the verbatim output is
`research/receipts/ix43-native-tcb-sizes.txt`. "Trusted" means the file can
change the checker's verdict.

| file | lines | why it is on the verdict path |
| --- | ---: | --- |
| `src/native/mod.rs` | 253 | error taxonomy, module wiring; no decision logic |
| `src/native/logic.rs` | 609 | term grammar, typing, parser/render, digests, evaluator |
| `src/native/bitblast.rs` | 401 | canonical CNF, Tseitin gates, strict DIMACS |
| `src/native/lrat.rs` | 458 | RUP, hint discipline, deletions, empty-clause rule |
| `src/native/cert.rs` | 628 | canonical container, DAG/cycle check, blob binding |
| `src/native/check.rs` | 217 | the five rules and the only `Theorem` constructor |
| `src/lib.rs` lines 205–319 | 115 | `Digest` parsing/hex and `Canon` domain-separated hashing |
| **total** | **2,681** | |

`src/lib.rs` is 1,121 lines; only the digest/hasher slice above is on the
verdict path. The store/revocation code in the same file is not consulted by
`native::check`, but it is linked into the same crate.

## 2. Not trusted (compiled, but cannot affect a verdict)

| file | lines | status |
| --- | ---: | --- |
| `src/native/reference.rs` | 726 | `#[doc(hidden)]` test oracle; see §5 |
| `tests/native.rs` | 672 | Milestone A conformance suite |
| `tests/native_semantics.rs` | 1,109 | Milestone B differential suite |
| `tests/refusals.rs`, `tests/semantics.rs` | — | unrelated suites |

The oracle is reachable from integration tests only through its public module.
The test `reference_oracle_is_not_on_the_trusted_path` enforces the separation
textually. Verbatim scan (`research/receipts/ix43-oracle-scan.txt`):

```text
src/native/check.rs:0
src/native/cert.rs:0
src/native/lrat.rs:0
src/native/bitblast.rs:0
src/native/logic.rs:0
src/native/mod.rs:1
```

`mod.rs:1` is the declaration `pub mod reference;` itself; `mod.rs` contains no
`reference::` call, and none of the five decision modules mentions it. The
imports of `check.rs` are only `std::collections`, `std::fmt`, `bitblast`, `cert`,
`lrat`, and `crate::{Canon, Digest}`. There is no path from `check` to
`reference`.

## 3. External dependencies

Direct: `sha2 0.10.9` (locked). Transitive (from `Cargo.lock`): `cfg-if`,
`cpufeatures`, `libc`, `block-buffer`, `crypto-common`, `digest`,
`generic-array`, `typenum`, `version_check`. All are inside assumption A3. No
other crate, no `build.rs`, no FFI, no subprocess, no file or network access in
the checker modules.

## 4. Assumed but unproved

- **A1** SHA-256 collision resistance (content identity only; not needed for
  `Accept ⇒ valid`).
- **A2** Rust semantics and the compiled binary; no UB; panic on exhausted
  budgets is fail-closed. There is no explicit recursion-depth bound: a deeply
  nested term can exhaust the stack and abort (fail-closed, but a denial of
  service).
- **A3** `sha2` and its transitive graph compute SHA-256 as specified.
- **A4** Parser faithfulness to the canonical grammar (gap G2).
- **A5** Bit-blaster equivalence to `⟦·⟧` (gap G1).
- **A6** Executable refinement of the abstract relations (gap G3).
- **A7** The paper lemmas (RUP, LRAT, Theorem 1) are not machine-checked (G4).

## 5. The oracle and its limits

`src/native/reference.rs` re-implements rule matching and LRAT/DIMACS parsing,
runs RUP by unit propagation to a fixpoint over the whole live clause set
(ignoring hint order; hint *references* are still validated), and for at most 20
free bits evaluates the proposition under every assignment with a separate
evaluator. It is not hardened and must never be used in production. It shares
the `Term` type/equality and `bitblast` with the signed path; those are inside
G1/G3 and are not independently validated.

Known strictness difference: a proof with a well-referenced but order-invalid
hint can be refused by the signed checker and accepted by the oracle. That is a
strictness difference, not unsoundness. In the other direction, on tiny widths
the oracle's exhaustive evaluator can refuse a certificate the signed checker
accepts if the bit-blaster has a semantic bug. That is the intended detection
channel. No such divergence appears in the tested mutation families (§6).

## 6. Verification actually run

```console
$ cargo test --offline
```

Result (verbatim from `research/receipts/ix43-native-semantics-tests.log`):

```text
tests/native.rs            26 passed
tests/native_semantics.rs   7 passed
tests/refusals.rs          25 passed
tests/semantics.rs         21 passed
lib/doc tests               0 passed, 0 failed
```

Differential counts (verbatim from
`research/receipts/ix43-native-semantics-counts.txt`):

```text
term round-trip cases: 500
certificate round-trip cases: 6
differential mutation cases: 19
differential acceptance cases: 10
```

- Acceptance: widths `1,2,3` × `add_comm`, `sub_add`, `add_zero`, plus
  `add_assoc` at width 2 = **10** independently generated certificates; signed
  checker and oracle both accept.
- Mutation: **19** cases; 18 must be refused by both (hint corruption, negative
  hint, deletion of a needed clause, dangling premise, unknown rule, duplicate
  step/blob, truncation, trailing bytes, CNF flip/non-canonical bytes, LRAT
  garbage/range/no-empty, transplanted proof, bad `sym`), 1 benign literal
  reordering must be accepted by both. All agreed.
- Parser: 500 random well-typed term round trips, 6 certificate round trips,
  18 malformed terms, 8 malformed certificates, and 4 non-canonical DIMACS
  inputs refused.
- Fuzz: 2,000 random inputs through `Term::parse`, `Certificate::parse`,
  `Cnf::parse_dimacs`, `check_lrat`, and 100 of them also through the oracle's
  LRAT path; 1,000 bit-flip mutations of a valid certificate. No panic.
- False identity: `x + 1 = x` at width 2 is false under all 4 assignments, its
  bit-blast is SAT, and forged/transplanted/swapped certificates are refused by
  both checkers.

Not run: the oracle on the 8-bit and 64-bit fixtures (its semantic budget is 20
free bits; its no-hint propagation would be quadratic-to-cubic on those sizes).
Those fixtures are accepted by the signed checker only (`tests/native.rs`).

## 7. The exact incompleteness metric

Milestone B is complete when **G1–G4 are discharged** and independent
generation plus oracle agreement covers **every width `1..=64`**.

| quantity | value | target |
| --- | ---: | ---: |
| gaps discharged | **0 / 4** | 4 / 4 |
| widths with independent generation + agreement | **3 / 64** | 64 / 64 |
| widths with signed-only fixtures | 2 (`8, 64`) | — |
| oracle semantic budget | 20 free bits | no decline |
| differential acceptance cases | 10 | — |
| differential mutation cases | 19 | — |
| surviving divergences in tested families | 0 | 0 |

`0/4` and `3/64` are the metric. Any statement that Milestone B is complete
must first move both to their targets.

## 8. Resource notes

- `.bench/` untouched. No benchmark artefacts written.
- Disk after the run: 24 GB free (threshold 3 GB not approached).
- No new processes are spawned by the checker; the no-process scan in
  `tests/native.rs` still passes over all six modules.
- No network, no new crate, no fixture regeneration.
