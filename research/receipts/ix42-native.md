# Receipt — IX.42 / Milestone A native checker

- **Repository:** `/mnt/1tb_kingston/Tekmerion`
- **Branch:** `staging` (not committed, per instruction)
- **Date:** 2026-11-23
- **Crate:** `tekmerion` (unchanged name; no new crates)

## What was run

```console
$ cargo test --offline
...
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 26 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Full log: `research/receipts/ix42-native-tests.log`.

```console
$ /home/one/.elan/toolchains/leanprover--lean4---v4.29.1/bin/cadical --version
2.1.2
```

Fixture proposal command (generation only, not check time):

```console
$ cadical --lrat --binary=false --plain <identity>.cnf <identity>.lrat
```

CaDiCaL 2.1.2's LRAT output annotates deletions as `<id> d <ids> 0`; the checker
accepts that form only when `<id>` is the most recently added clause id.

## Fixture sizes

Verbatim from `research/receipts/ix42-native-sizes.txt`:

```text
  4247 tests/fixtures/native/add_comm_8.cnf
  9146 tests/fixtures/native/add_comm_8.lrat
 27209 tests/fixtures/native/add_comm_8.cert
 42928 tests/fixtures/native/sub_add_64.cnf
 72822 tests/fixtures/native/sub_add_64.lrat
231932 tests/fixtures/native/sub_add_64.cert
388284 total
   208 tests/fixtures/native/add_comm_8.lrat
  1251 tests/fixtures/native/sub_add_64.lrat
  1459 total
```

SHA-256 of every fixture is in the same file.

## Check times

`check(&Certificate)` only; certificate parsing excluded. Commands and logs:

```console
$ cargo run --release --offline --example native_fixtures -- check tests/fixtures/native/add_comm_8.cert 1000
checks: 1000 total: 121.469879ms mean: 121.469µs

$ cargo run --release --offline --example native_fixtures -- check tests/fixtures/native/sub_add_64.cert 1000
checks: 1000 total: 1.210260639s mean: 1.21026ms
```

Logs: `ix42-native-time-add8.txt`, `ix42-native-time-sub64.txt`.

Unoptimized debug profile (profile `test` uses `opt-level = 1`):

```console
$ cargo run --offline --example native_fixtures -- check tests/fixtures/native/add_comm_8.cert 100
checks: 100 total: 115.828657ms mean: 1.158286ms

$ cargo run --offline --example native_fixtures -- check tests/fixtures/native/sub_add_64.cert 100
checks: 100 total: 868.671976ms mean: 8.686719ms
```

Logs: `ix42-native-time-add8-debug.txt`, `ix42-native-time-sub64-debug.txt`.

## Theorem digests

```text
add_comm_8: 41f6be858771cacfc227a079c45c28fff5cb273c1211bc087e123323f39f89fb
sub_add_64: 7c4653d3e3b63525644ca8d1e01688058c63f338b982769fb1a67ff297113854
```

## No-process scan

Command and result from `ix42-native-cadical.txt`:

```console
$ grep -nE 'Command|std::process|process::|spawn\(' \
    src/native/check.rs src/native/lrat.rs src/native/cert.rs
grep_exit=1
```

`grep_exit=1` means no matches. The integration test
`checker_path_contains_no_process_spawning` repeats this over all six modules
(`mod.rs`, `logic.rs`, `bitblast.rs`, `lrat.rs`, `cert.rs`, `check.rs`) and passes.

## Trusted base for this milestone

1. SHA-256 from the existing `sha2` dependency (identity only).
2. The Rust compiler and runtime executing `src/native`.
3. `logic.rs`: seven term constructors and the canonical digest encoding.
4. `bitblast.rs`: ripple-carry adder, two's-complement subtractor, XNOR equality,
   and Tseitin gates. Tested against exhaustive evaluation at small widths and
   against a pure-Rust DPLL SAT check for one false identity (`x + 1 = x` stays
   SAT) and valid identities (UNSAT).
5. `lrat.rs`: RUP only, deletions honoured, RAT refused, empty clause required last.
6. `cert.rs` + `check.rs`: rules `refl`, `sym`, `trans`, `congr`, and
   `bitblast(cnf_digest, lrat_digest)`.

Not in the trusted base: CaDiCaL, the fixture generator, HOL Light, OCaml, and any
external process at check time. The checker recomputes the canonical CNF from the
claimed proposition and refuses a mismatched CNF (`Error::CnfMismatch`).

## Stated side condition for the 64-bit identity

`(x + y) - y = x` over `bv64`. **Side condition: none.** `add` and `sub` are
modulo `2^64`, so the carry out is discarded by the semantics. The theorem and the
LRAT refutation cover carry-out and carry-free assignments alike. If an
integer-level identity were intended, the side condition `x + y < 2^64` would be
required; that is not what is proved here, and the statement was not weakened to
make checking easier.
