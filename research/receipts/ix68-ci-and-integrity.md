# IX.68 — Restoring CI and repairing measurement integrity (XV.A1–XV.A5, XV.0.3, XV.0.4)

- **Date:** 2026-11-25
- **Branch:** `staging` (one commit)
- **Scope:** `tests/*`, `tests/fixtures/emit/*`, `src/tools/dropin.rs`,
  `src/tools/lrat_trim.rs`, `src/bin/ix66_table.rs` (new), this receipt, and
  appended corrections in `ix65-same-obligation-and-phases.md` (§4) and
  `ix66-bridge-split.md` (§6 inline + §8). No `tools/*.ml`, no `src/native/`, no
  `verification/`, no other receipt, no `.bench` write, no tag.
- **Environment:** rustc 1.98.0 / cargo 1.98.0 / rustfmt 1.9.0-stable, OCaml
  5.5.0 (`ocaml`, `ocamlc`), one host.
- **Resource check before any run** (AGENTS.md §7a; cap 12 G, so the 3× rule
  needs ≥ 36 G available):

  ```text
  $ free -g | head -2
                 total        used        free      shared  buff/cache   available
  Mem:             125          24          98           0           3         101
  $ df -h /mnt/1tb_kingston | tail -1
  /dev/nvme1n1p1  1.3T  1.3T  8.3G 100% /mnt/1tb_kingston
  ```

  Available memory (101 G) is 8.4× the 12 G cap; 8.3 G free on the volume is
  above the 3 G floor. Every build and test run below went through
  `tools/cap.sh 12G --`; no proof build (HOL Light) was run at all.

- **Answer, one line:** CI's one offline failure is fixed with committed
  fixtures; `MatchedComparison` is now unconstructible outside `compare` and
  identity is statement-based; the trimmer refuses `negative_hint` /
  `no_empty_clause` in the same token as OCaml, verified against the shipped
  OCaml trimmer; timings carry a `TimeScope` and mixed/unrecorded scopes refuse
  the seconds aggregate by name; IX.65's 90.59× is marked withheld and IX.66's
  close count is corrected to 21 of 22 with a generator assertion.

| # | defect | fix | test | status |
|---|---|---|---|---|
| A1 | `ix66_markers_are_additive_…` read `.bench/hol-light/Cadical/` unconditionally | three halves: committed fixtures, tracked `tools/*.ml`, real set (skipped with a printed reason) | `tests/tools_emit.rs::ix66_markers_are_additive_and_the_normal_answer_is_recovered` | fixed, offline-verified |
| A2 | `MatchedComparison` fields public | fields private, read-only accessors, `compare` the only constructor | `compile_fail` + `no_run` doctests, `matched_comparison_is_read_only_and_compare_is_the_only_constructor` | fixed |
| A3 | rows paired on theorem *name* | `statement_digest` + `executable` identity; three new refusals | `compare_refuses_two_different_statements_with_the_same_name`, `…_a_different_executable_identity`, `a_same_named_theorem_with_a_different_statement_is_refused_in_the_table` | fixed |
| A4 | trimmer could report success with negative hints or a non-empty root | `TrimRefusal` with OCaml tokens, OCaml check order | 4 new unit tests + `tests/csat_trim_agreement.rs` (runs the shipped OCaml trimmer) | fixed, agreement verified |
| A5 | timings carried no scope | `TimeScope` per row; `compare` refuses mixed pairs; table refuses mixed/unrecorded rows by name | `compare_refuses_a_time_scope_difference`, `checktime_table_refuses_to_aggregate_mixed_time_scopes`, `checktime_table_withholds_seconds_when_the_scope_is_unrecorded`, `checktime_table_publishes_seconds_only_for_one_recorded_scope` | fixed; IX.65 corrected |
| A6 | IX.66 said "30 of 30 closes"; data says 21 of 22 | sentence corrected in place + correction log; generator asserts the total | `src/bin/ix66_table.rs` unit tests | fixed |

---

## 1. A1 — the offline failure, and why a fixture is the right fix

The old test mapped over `emit::ANSWER_SOURCES` and called
`fs::read_to_string(...).expect("library source")` on every entry. Three of the
five entries live under `.bench/hol-light/Cadical/`, which does not exist in a
CI checkout, so the test panicked before asserting anything. The reviewer is
right that this is the only offline failure: it is the one test in the suite
whose *inputs* are not skippable.

The property is `deinstrument ∘ instrument = id` and the identity of the phase
answer with the normal answer. Neither needs HOL Light: the instrumented set is
`tools/csat_lib.ml` and `tools/bridge.ml`, and the Cadical files are inlined but
never edited (`_ => continue` in the old loop). The test now has three halves:

1. **Committed fixtures** (`tests/fixtures/emit/csat_lib_min.ml`,
   `bridge_min.ml`, `cadical_stub.ml`): each marker anchor occurs exactly once,
   in the tracked library's lexical form. Instrument → deinstrument → byte
   identity, plus the assembled answer restored from the phase copy. Always
   runs.
2. **The tracked library texts** (`tools/csat_lib.ml`, `tools/bridge.ml`): in
   git, so always present; this is the half that fails if an anchor drifts in
   the real library. Always runs.
3. **The real inlined set** including `.bench/hol-light/Cadical/`: runs when the
   bench is there and prints why it was skipped when it is not.

Offline proof (the exact fixture path the old test loaded, renamed for one test
run and restored immediately; `.bench` itself was not moved):

```text
$ mv .bench/hol-light/Cadical .bench/hol-light/Cadical.offline-xv-a1
$ tools/cap.sh 12G -- cargo test --offline --test tools_emit \
      ix66_markers_are_additive_and_the_normal_answer_is_recovered -- --nocapture
   Compiling tekmerion v0.1.0 (/mnt/1tb_kingston/Tekmerion)
    Finished `test` profile [optimized + debuginfo] target(s) in 6.84s
     Running tests/tools_emit.rs (target/debug/deps/tools_emit-ad9ff4d32527f8d5)

running 1 test
skipping the real-source half: /mnt/1tb_kingston/Tekmerion/.bench/hol-light/Cadical is absent (offline CI). The fixture and tracked-library halves above proved `deinstrument ∘ instrument = id`.
test ix66_markers_are_additive_and_the_normal_answer_is_recovered ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.01s
$ mv .bench/hol-light/Cadical.offline-xv-a1 .bench/hol-light/Cadical
restored: .bench/hol-light/Cadical
```

The full suite with `.bench` absent (a byte copy of the tree at
`/tmp/ci-tree-ix68`, `rsync --exclude .bench --exclude target`) is green; see §8.

## 2. A2 — `MatchedComparison` cannot be forged

Fields are now private and the only constructor is `compare`:

```rust
pub struct MatchedComparison { human: MeasuredProof, generated: MeasuredProof }
impl MatchedComparison {
    pub fn human(&self) -> &MeasuredProof
    pub fn generated(&self) -> &MeasuredProof
    pub fn theorem(&self) -> &str
    pub fn statement_digest(&self) -> Option<&str>
    pub fn seconds_ratio(&self) -> Option<f64>   // None unless the scope is recorded
    pub fn inference_ratio(&self) -> Option<f64>
}
```

`trybuild` is not in `Cargo.lock` and the gate is offline, so the compile-fail
half is a `compile_fail` doctest on the type, with a companion `no_run` doctest
that constructs the type the supported way. If the imports or paths were wrong,
the `compile_fail` case would fail for the wrong reason and the `no_run` case
would fail outright, so the pair cannot pass spuriously:

```text
$ tools/cap.sh 12G -- cargo test --offline --doc
    Finished `test` profile [optimized + debuginfo] target(s) in 1.39s
   Doc-tests tekmerion

running 2 tests
test src/tools/dropin.rs - tools::dropin::MatchedComparison (line 2447) - compile ... ok
test src/tools/dropin.rs - tools::dropin::MatchedComparison (line 2437) - compile fail ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
```

`matched_comparison_is_read_only_and_compare_is_the_only_constructor` additionally
re-checks each precondition at run time (failed judgment, scope difference,
statement difference all refuse), so the property is covered behaviourally as
well as at compile time.

## 3. A3 — identity is the typed statement plus the executable

`MeasuredProof` (and the rows it is built from) now carries:

- `statement_digest: Option<String>` — a canonical digest of the theorem's
  conclusion, read from the receipt row's `statement_digest`;
- `executable: Option<String>` — the `arch.dir.op` the theorem was judged for,
  read from `executable`.

`comparison_reasons` appends three refusals (the existing ones keep their order,
so `compare`'s first-error contract is unchanged):

- `StatementIdentityMismatch { human, generated }` — names match, digest differs;
- `StatementIdentityUnrecorded { side }` — one side has no digest at all;
- `ExecutableIdentityMismatch { human, generated }`.

The names are still carried and still compared (`TheoremIdentityMismatch`), but
they are display identity only: two same-named theorems with different digests
are refused, at `compare` and in the rendered table
(`refused: statement identity`).

**Limit, stated plainly:** no current receipt records a conclusion digest, so a
row from `.bench/ix65/*.json` reads `statement_digest: None`. Two `None`s fall
back to the name (that is the legacy-receipt path, unchanged from before), and a
mixed present/absent pair is refused. The digest is only an identity once a
producer writes it; this receipt adds the field, the refusal and the tests, and
does not claim any existing receipt has statement identity.

## 4. A4 — the trimmer refuses, in the OCaml trimmer's words

`src/tools/lrat_trim.rs` now returns `Result<_, TrimRefusal>`, and
`TrimRefusal::reason()` returns exactly the token `tools/csat_lib.ml` prints
after `CSAT_TRIM skipped=`:

| Rust refusal | `reason()` |
|---|---|
| `NegativeHint { count }` | `negative_hint` |
| `NoEmptyClause { root }` | `no_empty_clause` |
| `Malformed { .. }` | `malformed` |
| `EmptyLrat` | `empty_lrat` |
| `NoAdditions` | `no_additions` |
| `DuplicateIds { .. }` | `duplicate_id` |
| `Io { .. }` | `io_error` |

Two behaviour changes make the two implementations actually agree, not just
share a vocabulary:

1. **The checks run in the OCaml order** — malformed, `empty_lrat`,
   `no_additions`, `no_empty_clause`, `negative_hint`, `duplicate_id`. So a file
   that fails two of them reports the same reason on both sides
   (`duplicate_ids_and_non_empty_root` → `no_empty_clause` in both).
2. **Any negative hint refuses, reachable or not.** The old Rust code counted
   negatives only while walking the backward closure; OCaml scans every hint.
   The Rust side now scans the whole file too (`a_negative_hint_anywhere_refuses_even_if_unreachable`).
   `TrimStats.negative_hints` therefore reads 0 on every success; the doc comment
   says so.

Agreement is not asserted from a shared table of expectations alone:
`tests/csat_trim_agreement.rs` extracts the `BEGIN/END CSAT_TRIM_STANDALONE`
block from `tools/csat_lib.ml` verbatim, appends a three-line driver, runs
`ocaml`, and requires (a) the same verdict token and (b) byte-identical trimmed
output for every success case, over 13 inputs including both refusal cases the
defect names:

```text
$ tools/cap.sh 12G -- cargo test --offline --test csat_trim_agreement
running 2 tests
test a_negative_hint_refusal_names_the_ocaml_reason ... ok
test the_rust_and_ocaml_trimmers_reach_the_same_verdict_on_every_case ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.97s
```

If `ocaml` is absent the OCaml half prints a skip line and the Rust half (which
pins that no refusal is reported as a success) still runs.

The shipped OCaml selftest is unchanged and still passes:

```text
$ bash tests/csat_trim_selftest.sh
...
CSAT_TRIM skipped=negative_hint
OK negative_hint_skips
CSAT_TRIM skipped=no_empty_clause
OK no_empty_clause_skips
...
PASS csat_trim_selftest cases=11
OK: csat_trim_selftest
```

## 5. A5 — a second is only a second if its scope is stated

`TimeScope ∈ {prove_only, whole_file, judge_run, unrecorded}` is now read from
each receipt row (`time_scope`, falling back to the file-level key, else
`Unrecorded`; an unrecognised token is `Unrecorded`, never a guess).

- `compare` refuses a pair whose two sides' scopes differ
  (`TimeScopeMismatch`), so a human `prove_only` number cannot be paired with a
  generated `whole_file` number at all.
- `MatchedComparison::seconds_ratio()` returns `None` unless the shared scope is
  recorded, so the ratio is withheld at the typed layer, not only in prose.
- Both renderers (`checktime-table`, `matched-table`) refuse the seconds
  aggregate when the matched rows do not share one recorded scope, print
  `SECONDS AGGREGATE REFUSED` plus a table naming **every row** and its scope,
  and keep the inference totals, which are unaffected.

Demonstration on IX.65's own inputs (read-only; outputs to the terminal):

```text
$ tools/cap.sh 12G -- cargo run --offline --quiet --bin dropin -- matched-table \
    --human .bench/ix65/human.json --generated .bench/ix65/generated.json \
    --human-kernel hol-light-stamp2 --generated-kernel hol-light-stamp2
### Matched totals (denominator: n matched pairs)

| metric | value |
|---|---:|
| matched pairs (n) | 6 |
| human inferences (n=6) | 14019340 |
| generated inferences (n=6) | 993573633 |
| aggregate inference ratio (n=6) | 70.87 |
| per-task inference ratio median (n=6) | 60.20 |
| human seconds | withheld: time scope not aggregatable |
| generated seconds | withheld: time scope not aggregatable |
| aggregate seconds ratio | withheld: time scope not aggregatable |

**SECONDS AGGREGATE REFUSED.** Every matched row records `unrecorded`, which is not one of `prove_only`, `whole_file` or `judge_run`, so the span the seconds were measured over is not established. No total or ratio over these rows is published; the inference counts are unaffected.

| row | time scope |
|---|---|
| arm curve25519/add_p25519 | unrecorded |
| arm p384/add_p384 | unrecorded |
| arm p521/add_p521 | unrecorded |
| arm curve25519/neg_p25519 | unrecorded |
| x86 curve25519/add_p25519 | unrecorded |
| x86 sm2/neg_sm2 | unrecorded |

MATCHED 6/6 PAIRS; UNMATCHED 0; HUMAN ROWS 6; GENERATED ROWS 6
```

The 90.59× second ratio is therefore withheld as a comparison, the inference
ratio stands, and the correction is appended to IX.65 §4 (the original table is
left in place, marked withheld).

## 6. A6 — IX.66's close count

From `research/receipts/ix66-bridge-split.json`, per task:

| task | `sat_closes` | `csat_attempts` |
|---|---:|---:|
| `arm_add_p25519` | 4 | 4 |
| `arm_add_p384` | 4 | 4 |
| `arm_add_p521` | 4 | 4 |
| `arm_neg_p25519` | 3 | 4 |
| `x86_add_p25519` | 4 | 4 |
| `x86_neg_sm2` | 2 | 2 |
| **total** | **21** | **22** |

§6's "30 of 30 closes" is corrected in place with a visible correction note, and
a §8 correction log records what changed and why. The original table generator
(`.bench/ix66/build66.awk`) is git-ignored and out of this task's write scope, so
the assertion lives in a checked-in generator, `src/bin/ix66_table.rs`, which
regenerates the close column from the receipt JSON and asserts:

- per task, `sat_closes == closes.len()` and
  `csat_attempts == word_goal_sizes.len()` (a printed count must be backed by
  entries, not typed);
- the total **re-parsed from its own rendered table** equals the sum of the
  per-task counts;
- a unit test pins this section: the receipt totals 21 / 22 and the prose states
  `21 of 21 closes across 22 CSAT attempts`.

```text
$ tools/cap.sh 12G -- cargo run --offline --quiet --bin ix66_table
| task | SAT closes / CSAT attempts |
|---|---:|
| `arm_add_p25519` | 4 / 4 |
| `arm_add_p384` | 4 / 4 |
| `arm_add_p521` | 4 / 4 |
| `arm_neg_p25519` | 3 / 4 |
| `x86_add_p25519` | 4 / 4 |
| `x86_neg_sm2` | 2 / 2 |

CLOSES total=21 attempts=22 tasks=6
```

---

## 7. Gate steps, verbatim

Two runs are reported, because they answer different questions.

- **Local tree** (`/mnt/1tb_kingston/Tekmerion`, `.bench` present): exactly what
  `cargo test --offline` does here today.
- **CI-faithful tree** (`/tmp/ci-tree-ix68`: `rsync -a --exclude .bench
  --exclude target`, built with `--offline`): the CI checkout, which has no
  `.bench` at all. `.bench` itself was never moved for a suite run.

### 7.1 `free -g | head -2`

```text
               total        used        free      shared  buff/cache   available
Mem:             125          24          98           0           3         101
```

### 7.2 `tools/cap.sh 12G -- cargo test --offline` — local tree

Exit status 101. Every suite passes except `tests/tools_emit.rs`, where three
tests compare against git-ignored reference answers; see §8.

```text
$ tools/cap.sh 12G -- cargo test --offline
...
     Running tests/tools_emit.rs (target/debug/deps/tools_emit-ad9ff4d32527f8d5)
test result: FAILED. 8 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.40s
error: test failed, to rerun pass `--test tools_emit`
```

### 7.3 `tools/cap.sh 12G -- cargo test --offline` — CI-faithful tree

Exit status 0. The bench-dependent tests skip themselves; the new tests run.

```text
$ ( cd /tmp/ci-tree-ix68 && tools/cap.sh 12G -- cargo test --offline > /tmp/ix68-cargo-test-ci.txt 2>&1; echo "rc=$?" )
rc=0
$ grep -E "^(     Running|   Doc-tests|test result)" /tmp/ix68-cargo-test-ci.txt
     Running unittests src/lib.rs (target/debug/deps/tekmerion-28c175528fb66e7d)
test result: ok. 104 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/bin/attempt_id.rs (target/debug/deps/attempt_id-6a18dfac6bb46eae)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/bin/attempt_lifecycle.rs (target/debug/deps/attempt_lifecycle-68032f8a5d28c11f)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/bin/challenge.rs (target/debug/deps/challenge-a1784f0282b80b0d)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/bin/curve_check.rs (target/debug/deps/curve_check-02bca025f411625c)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/bin/dropin.rs (target/debug/deps/dropin-ce5cac345504b603)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/bin/emit.rs (target/debug/deps/emit-be12028ea777c938)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/bin/emit_x86.rs (target/debug/deps/emit_x86-1a3cd3d92d2091bb)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/bin/fcdist.rs (target/debug/deps/fcdist-212cd246ab88a7d5)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/bin/freeze.rs (target/debug/deps/freeze-b3c9cad8ee48ee4e)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/bin/guard.rs (target/debug/deps/guard-908195ca0742b712)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/bin/inline_answer.rs (target/debug/deps/inline_answer-4b92729a94f04772)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/bin/instrument.rs (target/debug/deps/instrument-2fe171057a82c0a8)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/bin/ix66_table.rs (target/debug/deps/ix66_table-db48cd66db171e22)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/bin/lrat_trim.rs (target/debug/deps/lrat_trim-5039189361000176)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/bin/mutate.rs (target/debug/deps/mutate-af6ca9f5c32824e5)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/bin/mutate_guard.rs (target/debug/deps/mutate_guard-01af4f072e77a063)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/bin/mutate_lifecycle.rs (target/debug/deps/mutate_lifecycle-e2bd890bdf3e9e11)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/bin/patch_wrapper.rs (target/debug/deps/patch_wrapper-9ccd5310c926d79a)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/bin/run_traces_inject.rs (target/debug/deps/run_traces_inject-ad597b15a4fe4b5d)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/bin/script_gate.rs (target/debug/deps/script_gate-b60ef04c7c151336)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/bin/sealed_court.rs (target/debug/deps/sealed_court-76826c1190981ea3)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/bin/trace_conformance.rs (target/debug/deps/trace_conformance-cdb8b907313fae04)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/csat_trim_agreement.rs (target/debug/deps/csat_trim_agreement-7332f952124d726e)
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.97s
     Running tests/native.rs (target/debug/deps/native-466c05518b443e43)
test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/native_semantics.rs (target/debug/deps/native_semantics-854b82106dd74c1b)
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/refusals.rs (target/debug/deps/refusals-cb9e5e47513a329e)
test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/semantics.rs (target/debug/deps/semantics-12a2efb7ad911f91)
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/tools.rs (target/debug/deps/tools-f9f19812a21f7af7)
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/tools_attempt.rs (target/debug/deps/tools_attempt-b5f0fbe826fd1fe1)
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/tools_derive.rs (target/debug/deps/tools_derive-813a366d6ab91c98)
test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/tools_emit.rs (target/debug/deps/tools_emit-ad9ff4d32527f8d5)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/tools_fcdist.rs (target/debug/deps/tools_fcdist-79ae6e7dd4268b8e)
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/tools_freeze.rs (target/debug/deps/tools_freeze-55768064e9a041de)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s
     Running tests/tools_guard.rs (target/debug/deps/tools_guard-60d5b249bcbf993c)
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/tools_patch_wrapper.rs (target/debug/deps/tools_patch_wrapper-ce6c6ef7fadddf7c)
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/tools_receipt.rs (target/debug/deps/tools_receipt-0af53b2acaa06bf3)
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/tools_trace_conformance.rs (target/debug/deps/tools_trace_conformance-8388f3201945b388)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/write_through.rs (target/debug/deps/write_through-b3f963ee2ac58f6c)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
   Doc-tests tekmerion
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
```

### 7.4 `cargo clippy --offline --all-targets -- -D warnings`

Local tree:

```text
$ tools/cap.sh 12G -- cargo clippy --offline --all-targets -- -D warnings
    Checking tekmerion v0.1.0 (/mnt/1tb_kingston/Tekmerion)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.53s
clippy_rc=0
```

CI-faithful tree:

```text
$ ( cd /tmp/ci-tree-ix68 && tools/cap.sh 12G -- cargo clippy --offline --all-targets -- -D warnings )
   Compiling generic-array v0.14.7
    Checking typenum v1.20.1
    Checking cpufeatures v0.2.17
    Checking cfg-if v1.0.5
    Checking crypto-common v0.1.7
    Checking block-buffer v0.10.4
    Checking digest v0.10.7
    Checking sha2 v0.10.9
    Checking tekmerion v0.1.0 (/tmp/ci-tree-ix68)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.23s
clippy_rc=0
```

### 7.5 `cargo fmt --check`

```text
$ cargo fmt --check; echo "fmt_rc=$?"
fmt_rc=0
```

(Same in the CI-faithful tree: `fmt_rc=0`.)

### 7.6 Refusal-site mutation harness (offline)

`src/bin/mutate.rs` mutates `src/tools/guard.rs` refusal sites.

```text
$ tools/cap.sh 12G -- cargo run --offline --quiet --bin mutate
R01: KILLED
R02: KILLED
R03: KILLED
R04: KILLED
R05: KILLED
R06: KILLED
R07: KILLED
R08: KILLED
R09: KILLED
R10: KILLED
R11: KILLED
R12: KILLED
R13: KILLED
R14: KILLED
R15: KILLED
R16: KILLED

refusal sites: 16
valid mutants: 16  killed: 16  survived: 0
mutation score: 16/16 = 1.000
mutate_rc=0
```

### 7.7 Lifecycle mutation harness (offline)

**Local tree fails**, and the reason is the same one as §7.2: the harness runs
the *whole* suite as its baseline, and that suite contains the three
`.bench`-reference tests.

```text
$ tools/cap.sh 12G -- cargo run --offline --quiet --bin mutate_lifecycle
baseline suite failed with fault-injection enabled
...
arm_answers_are_byte_identical --- FAILED
sealed_freeze_verification_end_to_end --- FAILED
x86_answers_are_byte_identical --- FAILED
...
mutate_lifecycle_rc=2
```

CI-faithful tree (the CI condition; the section banners and `OK` lines are
verbatim, the per-section test chatter is omitted):

```text
$ ( cd /tmp/ci-tree-ix68 && tools/cap.sh 12G -- cargo run --offline --quiet --bin mutate_lifecycle )
L05 (update forgets the superseded node): KILLED

lifecycle sites: 5
valid mutants: 5  killed: 5  survived: 0
lifecycle mutation score: 5/5 = 1.000
parser mutation score: 16/16 = 1.000
mutate_lifecycle_rc=0
```

(L01–L04 are elided; each prints `KILLED`.)

### 7.8 Guard refusal-site mutation harness (Rust, offline)

```text
$ tools/cap.sh 12G -- cargo run --offline --quiet --bin mutate_guard

src/tools/guard.rs refusal sites: 60 (0 invalid)
killed: 60  survived: 0
mutation score: 60/60 = 1.000

src/tools/freeze.rs refusal sites: 38 (0 invalid)
killed: 38  survived: 0
mutation score: 38/38 = 1.000

all targets: 98 refusal sites (0 invalid), killed 98, survived 0
mutation score: 98/98 = 1.000
all survivors are recorded equivalent mutants
mutate_guard_rc=0
```

### 7.9 Scaffold self-test (offline)

CI-faithful tree (local tree fails at 6/7a for the §8 reason):

```text
$ ( cd /tmp/ci-tree-ix68 && tools/cap.sh 12G -- bash tools/selftest.sh )
=== 1/7 OCaml syntax check (syntax only; type-checking needs HOL Light) ===
OK: trace/exptrace_failure.ml parses.
=== 2/7 runner script syntax ===
OK: tools/run_traces.sh parses
OK: run_traces_inject self-test passed
=== 2b/7 attempt_id self-test ===
OK: attempt_id self-test passed
=== 2c/7 attempt_lifecycle self-test ===
OK: attempt_lifecycle self-test passed
=== 2d/7 run_traces.sh argument parsing ===
OK: an unknown option is rejected
OK: --proof is parsed and a missing path is reported
OK: --proof and a theorem name are mutually exclusive
=== 3/7 patch_wrapper --self-test ===
OK: patch_wrapper self-test passed
=== 4/7 trace_conformance --self-test ===
OK: trace_conformance self-test passed
=== 5/7 write-through self-test (module compiled against stubs, no dump) ===
OK: write-through self-test passed
=== 6/7a existing Rust suite (cargo test --offline) ===
OK: cargo test --offline passed
=== 6/7b existing mutation harness (cargo run --bin mutate) ===
OK: cargo run --bin mutate passed
=== 7/7 lifecycle mutation harness (cargo run --bin mutate_lifecycle) ===
OK: cargo run --bin mutate_lifecycle passed
selftest_rc=0
```

### 7.10 Workflow steps that are environment, not checks

- `Install Rust 1.98.0 with clippy`: `rustc 1.98.0 (88d9e12ae 2026-08-18)`,
  `cargo 1.98.0 (797e8a9bc 2026-08-05)` — matches the pin.
- `Install OCaml (syntax check only)`: `ocamlc -version` → `5.5.0`.
- `Prime crate cache`: `cargo fetch --locked --offline` → rc 0 (already primed).
- `Set up Python`: nothing in the workflow's checks uses Python; the language
  ban is enforced by `git ls-files '*.py'` below.

### 7.11 No Python

```text
$ git ls-files '*.py'
py_files=0
```

### 7.12 Working tree

```text
$ git --no-optional-locks status --short
 M TODO.md
 M research/receipts/ix65-same-obligation-and-phases.md
 M research/receipts/ix66-bridge-split.md
 M src/tools/dropin.rs
 M src/tools/lrat_trim.rs
 M tests/tools_emit.rs
?? src/bin/ix66_table.rs
?? tests/csat_trim_agreement.rs
?? tests/fixtures/emit/
```

---

## 8. The three local failures, and the proof they are not mine

`tests/tools_emit.rs::{arm_answers_are_byte_identical, x86_answers_are_byte_identical,
sealed_freeze_verification_end_to_end}` compare `--mode answer` output against
`.bench/dropin-gen/answers{,-arm}/*.answer.txt`. The reference files are 112,111
bytes; the current answer for `arm.bignum_add_p25519` is 119,727 bytes, and the
first difference is at byte 68,743 — the reference carries
`(* everything that precedes the SAT call, on the goal *)` (which is
`tools/bridge.ml:804`) where the current answer carries a different comment. So
a tracked library changed after those reference answers were written. Nothing in
this task's change set touches `tools/*.ml`, `src/tools/emit.rs`, or the
reference files, and CI cannot see the failure because `.bench` is absent there
and all three tests return early.

**Proof it is pre-existing.** All five files I modified were copied to
`/tmp/tek-backup/`, reverted with `git checkout --`, and the same test target was
re-run; the panic is byte-for-byte the same:

```text
$ git checkout -- src/tools/dropin.rs src/tools/lrat_trim.rs tests/tools_emit.rs \
      research/receipts/ix65-same-obligation-and-phases.md research/receipts/ix66-bridge-split.md
$ tools/cap.sh 12G -- cargo test --offline --test tools_emit
---- arm_answers_are_byte_identical stdout ----

thread 'arm_answers_are_byte_identical' (1614765) panicked at tests/tools_emit.rs:140:5:
arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT: first difference at byte 68743 (got 119727 bytes, want 112111 bytes)
  got:  "ths in\n    ACCEPT_TAC th' (asl,w)\nin\n(* ----------------------------------------"
  want: "ths in\n    ACCEPT_TAC th' (asl,w)\nin\n(* everything that precedes the SAT call, o"

...
failures:
    arm_answers_are_byte_identical
    sealed_freeze_verification_end_to_end
    x86_answers_are_byte_identical

test result: FAILED. 8 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.72s
error: test failed, to rerun pass `--test tools_emit`
$ # files restored from /tmp/tek-backup/
```

Regenerating those answers needs a HOL Light run against the pinned tree, which
this task explicitly does not do (no proof builds), and `.bench` is out of the
write scope. It is recorded here, **not fixed**, and does not affect CI.

---

## 9. What could not be verified

- **The CI runner itself.** No GitHub Actions run was observed. The CI condition
  is reproduced by a bench-free copy of the tree (§7.3–7.9), not by the service.
- **`StatementIdentityMismatch` on real receipts.** No producer writes
  `statement_digest` yet, so A3 is verified on constructed proofs and receipts
  only; every existing receipt reads `None` (see §3's limit).
- **IX.65's true generated-side time scope.** The generated `seconds` may be
  whole-file or prove-only; the receipt does not say. The number is withheld
  rather than guessed, and re-measuring under one scope is future work.
- **A fresh HOL Light re-run** of any task. None was attempted (AGENTS.md §7a:
  no proof build was needed for these six defects).
- **The three pre-existing reference-answer failures.** Diagnosed and proven
  pre-existing (§8); regenerating the references is out of scope.

## 10. Freeze note

`src/tools/dropin.rs`, `src/tools/lrat_trim.rs`, `tests/tools_emit.rs` and the
new `src/bin/ix66_table.rs` / `tests/csat_trim_agreement.rs` moved, so any
existing freeze identity that covers them is stale for this commit. This receipt
creates no freeze tag and claims no sealed run.
