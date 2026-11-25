# IX.74 — identity-bound comparison and configuration-bound freeze (XVI.B, XVI.C)

Branch `staging`. Repository `/mnt/1tb_kingston/Tekmerion`.
No tag was created. No file under `.bench/`, `tools/*.ml`, `src/native/`,
`verification/` or another receipt was modified.

This receipt records two changes:

* **XVI.B** — a comparison can no longer be "matched on names alone". There are
  two types and only `IdentityBoundComparison` may back a performance result.
* **XVI.C** — the freeze binds the effective execution configuration, not just
  file hashes, and `freeze verify` refuses when the live environment differs
  from the recorded one.

---

## 0. Environment at the start

```
$ free -g | head -2
               total        used        free      shared  buff/cache   available
Mem:             125          26          93           0           5          98

$ df -h /mnt/1tb_kingston | tail -1
/dev/nvme1n1p1  1.3T  901G  361G  72% /mnt/1tb_kingston
```

Every command that can allocate more than a couple of GB ran through
`tools/cap.sh` (`cargo test`, `cargo clippy`, `cargo build --release`,
`freeze manifest`, `freeze verify`). The full suite ran once, sequentially, with
`-j` left at cargo's default inside that one capped command; no two proof
builds were started. Peak sampled group RSS was 2 885 824 KB (2.75 GB), well
under the 20 GB cap; the cap did not fire.

---

## 1. XVI.B — two comparison types

### 1.1 The types (`src/tools/dropin.rs`)

`MatchedComparison` and `compare()` are gone. In their place:

| type | constructor | evidence rules |
|---|---|---|
| `DiagnosticComparison` | `diagnose(h, g)` | theorem names equal; **both** judgments successful (`exit == 0`, `check_axioms`, theorem proved); human/generated tree kinds correct; measurement scopes equal; identical `TimeScope`; a statement digest **that is present** must be equal; an executable identity **that is present** must be equal. Missing identity evidence is allowed and recorded in `IdentityEvidence`. |
| `IdentityBoundComparison` | `bind_identity(h, g)` | everything the diagnostic requires **plus**: statement digest present and equal on both sides, executable identity present and equal on both sides. |

Both types have private fields and one constructor each; the two
`compile_fail` doctests on the types and
`comparison_types_are_read_only_and_their_constructors_are_the_only_way_in`
pin that.

`IdentityEvidence` records the four inputs:

```text
human_statement_digest, generated_statement_digest,
human_executable,        generated_executable
```

and prints exactly what is missing, e.g.

```text
statement digest missing on generated; executable identity missing on generated
```

`comparison_reasons()` was the defect. It is replaced by two functions:

* `diagnostic_reasons(h, g)` — the base rules above, with missing evidence
  **not** a reason;
* `identity_reasons(h, g)` — `diagnostic_reasons` plus
  `StatementIdentityUnrecorded { side }` and
  `ExecutableIdentityUnrecorded { side }` for every absent piece.

### 1.2 Tables and the `--diagnostic` flag

`dropin checktime-table` and `dropin matched-table` print the type of every row:

* `identity-bound`
* `diagnostic: <exactly what is missing>`
* `refused: <first refusal code>`

Aggregates (`ratio`, `median`, `total`) are computed over identity-bound rows
only. The legacy aggregate is printed only with `--diagnostic`, under the
heading `### Legacy aggregate (diagnostic types included) — NOT A RESULT`, and
**every line carries `NOT A RESULT`**. Example from a two-row fixture: one
fully bound row and one row whose generated side has no statement digest:

```
$ target/release/dropin checktime-table --human /tmp/ix74-human.json \
      --generated /tmp/ix74-generated.json --diagnostic
...
| arm | p521/add_p521 | BIGNUM_ADD_P521_CORRECT | BIGNUM_ADD_P521_CORRECT | 3.000 | 12.000 | identity-bound |
| arm | p521/sub_p521 | BIGNUM_SUB_P521_CORRECT | BIGNUM_SUB_P521_CORRECT | 4.000 | 8.000 | diagnostic: statement digest missing on generated |
...
| identity-bound pairs (n) | 1 |
...
### Legacy aggregate (diagnostic types included) — NOT A RESULT

| metric | value |
|---|---:|
| legacy compared pairs (n) | 2 NOT A RESULT |
| legacy human total s (n=2) | 7.000 NOT A RESULT |
| legacy generated total s (n=2) | 20.000 NOT A RESULT |
| legacy seconds ratio (n=2) | 2.86 NOT A RESULT |
| legacy seconds ratio median (n=2) | 3.00 NOT A RESULT |
| legacy human inferences (n=2) | 3000 NOT A RESULT |
| legacy generated inferences (n=2) | 12000 NOT A RESULT |
| legacy inference ratio (n=2) | 4.00 NOT A RESULT |
| legacy inference ratio median (n=2) | 4.00 NOT A RESULT |
...
IDENTITY-BOUND 1/2; DIAGNOSTIC 1; REFUSED 0; HUMAN ROWS 2; GENERATED ROWS 2
```

Without `--diagnostic`, the same run prints the identity-bound row, the
diagnostic row with its missing evidence, `identity-bound pairs (n) | 1`, and
**no** `NOT A RESULT` line. The CLI exits 0 only when every row is
identity-bound (the run above exits 1).

### 1.3 The digest scheme (B3)

The producer (`sweep`/`measure`, `build_logged`) writes two fields per task:

* `statement_digest` — `sha256_hex(canonical_term(extract_conclusion(text, theorem)))`,
  where `text` is the task's `.correct` file. This deliberately reuses the two
  existing normalisations: `extract_conclusion` reads the conclusion after
  `val <theorem> : thm = |- `, and `canonical_term` is the same canonical typed
  text used by the premise and axiom-registry comparisons. No second
  normalisation was introduced. An absent or empty conclusion yields `null`,
  never the digest of `""`.
* `executable` — `arch.dir.op` (the task key), the executable the theorem
  belongs to.

`read_timed_tasks` already read both fields; the pipeline is now exercised end
to end. The real-data test
`a_producer_digest_is_stable_on_real_correct_files` reads
`.bench/ix69/baseline/arm_add_p25519.correct` and
`.bench/ix69/sliced/arm_add_p25519.correct`, confirms both prove
`BIGNUM_ADD_P25519_CORRECT`, confirms the canonical digests are equal, and
constructs a real `IdentityBoundComparison` from them. Both artifacts are
emitter products; in that test they are the two comparison *sides*, so it
exercises identity, not human/generated provenance. A human `_CORRECT`
artifact for the six IX.65 measured tasks is not currently present in
`.bench` (the human tree prints `_SUBROUTINE_CORRECT`), so a real
human-vs-generated identity-bound pair was not produced without re-running
those proofs; that is stated rather than simulated.

### 1.4 The four B tests, verbatim

```
$ tools/cap.sh 20G -- cargo test --offline --lib -- --exact \
    tools::dropin::tests::a_same_named_theorem_with_a_different_statement_is_refused_in_the_table \
    tools::dropin::tests::a_missing_digest_on_one_side_is_diagnostic_and_never_identity_bound \
    tools::dropin::tests::a_legacy_aggregate_is_visibly_not_a_result \
    tools::dropin::tests::a_fully_bound_pair_yields_the_identity_bound_type
...
running 4 tests
test tools::dropin::tests::a_fully_bound_pair_yields_the_identity_bound_type ... ok
test tools::dropin::tests::a_missing_digest_on_one_side_is_diagnostic_and_never_identity_bound ... ok
test tools::dropin::tests::a_legacy_aggregate_is_visibly_not_a_result ... ok
test tools::dropin::tests::a_same_named_theorem_with_a_different_statement_is_refused_in_the_table ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 113 filtered out
```

Mapping to the four required cases:

* (a) two different statements under the same theorem name —
  `a_same_named_theorem_with_a_different_statement_is_refused_in_the_table`
  (both `diagnose` and `bind_identity` refuse with
  `StatementIdentityMismatch`; the table shows `refused: statement identity`).
* (b) missing digest on one side — 
  `a_missing_digest_on_one_side_is_diagnostic_and_never_identity_bound`
  (`diagnose` succeeds, `missing_evidence()` names the generated side;
  `bind_identity` refuses with `StatementIdentityUnrecorded { side: "generated" }`).
* (c) a legacy aggregate prints `NOT A RESULT` —
  `a_legacy_aggregate_is_visibly_not_a_result` (default render contains no
  `NOT A RESULT` and no legacy aggregate; `--diagnostic` contains both, and the
  identity-bound aggregate is still `n=0`).
* (d) a fully bound pair — `a_fully_bound_pair_yields_the_identity_bound_type`
  (equal digests and equal executables on both sides yield
  `IdentityBoundComparison`; `evidence().is_complete()`).

---

## 2. XVI.C — the freeze binds the effective configuration

### 2.1 Every variable, found by grep

```
$ grep -rn 'env::var\|env::var_os' src/ --include=*.rs
src/tools/disasm.rs:94:    let path = std::env::var_os("PATH")?;
src/tools/emit.rs:1288:    match std::env::var_os(PROBLEMS_ENV) {
src/tools/emit.rs:1438:    std::env::var("TEKMERION_ALG_CLOSE")
src/tools/config.rs:12://! * [`ENV_VARS`] — read by the Rust binary (`std::env::var`/`var_os` in
src/tools/config.rs:155:    std::env::var_os(name).map(|value| value.to_string_lossy().into_owned())
src/tools/config.rs:313:    /// The argument text of `env::var(..)` / `env::var_os(..)` on one line.
src/tools/script_gate.rs:162:    let mut env: Vec<(String, String)> = std::env::vars().collect();
src/lib.rs:78:        std::env::var("TEKMERION_FAULT")
```

Resolving `PROBLEMS_ENV` (`src/tools/emit.rs:1281`,
`const PROBLEMS_ENV: &str = "TEKMERION_PROBLEMS"`) and ignoring non-`TEKMERION_`
reads (`PATH`, `env::vars()`), the binary reads:

```
TEKMERION_ALG_CLOSE
TEKMERION_FAULT
TEKMERION_PROBLEMS
```

The reasoning path is also changed by two variables read outside the Rust
binary, by the OCaml bridge, and one read by the fixture recorder:

```
$ grep -n 'Sys.getenv "TEKMERION_' tools/bridge.ml
729:  try Sys.getenv "TEKMERION_SLICE_CONTROL" with Not_found -> "";;
732:  try Sys.getenv "TEKMERION_SLICE_DEBUG" = "1" with Not_found -> false;;

$ grep -rn 'TEKMERION_RECORD_ANSWERS' tests/ --include=*.rs
tests/tools_emit.rs:132:/// Set `TEKMERION_RECORD_ANSWERS=1` to (re-)record the fixtures instead of
tests/tools_emit.rs:135:    std::env::var_os("TEKMERION_RECORD_ANSWERS").is_some()
tests/tools_emit.rs:145:         TEKMERION_RECORD_ANSWERS=1 (see research/receipts/ix68-ci-and-integrity.md)",
```

### 2.2 The declaration and its guard test (`src/tools/config.rs`)

```rust
pub const ENV_VARS: &[&str] = &[
    "TEKMERION_ALG_CLOSE",
    "TEKMERION_FAULT",
    "TEKMERION_PROBLEMS",
];
pub const BRIDGE_ENV_VARS: &[&str] = &["TEKMERION_SLICE_CONTROL", "TEKMERION_SLICE_DEBUG"];
pub const TEST_ENV_VARS: &[&str] = &["TEKMERION_RECORD_ANSWERS"];
pub const ALL_ENV_VARS: &[&str] = &[
    "TEKMERION_ALG_CLOSE",
    "TEKMERION_FAULT",
    "TEKMERION_PROBLEMS",
    "TEKMERION_RECORD_ANSWERS",
    "TEKMERION_SLICE_CONTROL",
    "TEKMERION_SLICE_DEBUG",
];
```

Three tests walk the sources and fail if the declaration and the source
disagree:

* `declared_binary_vars_match_the_source` — scans `src/**/*.rs` for
  `env::var`/`env::var_os`, resolves a literal or a `const ... = "TEKMERION_…"`
  identifier, and asserts the found set equals `ENV_VARS`.
* `declared_bridge_vars_match_the_bridge` — scans `tools/bridge.ml` for
  `Sys.getenv "TEKMERION_…"` and asserts the set equals `BRIDGE_ENV_VARS`.
* `declared_test_vars_match_the_recorder` — scans `tests/**/*.rs` and asserts
  the set equals `TEST_ENV_VARS`.

A new variable therefore cannot appear unnoticed: the first test fails until it
is declared.

### 2.3 Where the configuration is recorded

* **Run record** — `sweep` and `measure` JSON now carry
  `"configuration": {"env": {...}, "flags": {...}}` (all six variables, with
  the literal `unset` for an unset one; `mode`, `stamp`, `force`, `slice`).
  `--mode` is `unspecified` for the drop-in producer because dropin has no such
  flag; it is recorded as `unspecified`, never guessed.
* **Freeze closure** — the same object as `closure.configuration`. A manifest
  without the field parses and verifies under its own scope, with the printed
  warning below.

Example from `freeze-closure-v6.json` (line 81):

```json
"configuration": {"env": {"TEKMERION_ALG_CLOSE": "unset", "TEKMERION_FAULT": "unset", "TEKMERION_PROBLEMS": "unset", "TEKMERION_RECORD_ANSWERS": "unset", "TEKMERION_SLICE_CONTROL": "unset", "TEKMERION_SLICE_DEBUG": "unset"}, "flags": {"mode": "unspecified", "stamp": "unspecified", "force": "unspecified", "slice": "off"}},
```

### 2.4 `freeze verify` refuses on a configuration mismatch

The manifest and verify commands (the canonical regeneration command is the
same one, wrapped by `cap.sh`):

```
$ tools/cap.sh 20G -- cargo run --release --offline --bin freeze -- manifest \
      --out research/receipts/freeze-closure-v6.json \
      --tag freeze-rust-closure-v6 --hol .bench/hol-light-stamp2 --runtime
wrote research/receipts/freeze-closure-v6.json: 72 source files,
source_digest 66168775f1a7c6317782b78e392488640ed0d459ed0dfae05c58c1daf3b0cb88,
10 runtime digests, repo_commit 97514a44f541ee0db1b8baa6fd7479367d6801fb

$ tools/cap.sh 20G -- cargo run --release --offline --bin freeze -- verify \
      --manifest research/receipts/freeze-closure-v6.json
OK: source closure verified: 72 files, source_digest 66168775f1a7c6317782b78e392488640ed0d459ed0dfae05c58c1daf3b0cb88
OK: effective configuration verified: 6 TEKMERION_* variables
```

With the runtime layer as well (extra check, same manifest):

```
$ target/release/freeze verify --manifest research/receipts/freeze-closure-v6.json --runtime
OK: source closure verified: 72 files, source_digest 66168775f1a7c6317782b78e392488640ed0d459ed0dfae05c58c1daf3b0cb88
OK: effective configuration verified: 6 TEKMERION_* variables
OK: runtime closure verified against the recorded build
```

### 2.5 The C firing controls, verbatim

**(1) Flip a recorded-unset variable.** `TEKMERION_ALG_CLOSE` was recorded
`unset`; setting it must refuse and name it.

```
$ TEKMERION_ALG_CLOSE=arm.bignum_add_p25519 tools/cap.sh 20G -- \
      cargo run --release --offline --bin freeze -- verify \
      --manifest research/receipts/freeze-closure-v6.json
OK: source closure verified: 72 files, source_digest 66168775f1a7c6317782b78e392488640ed0d459ed0dfae05c58c1daf3b0cb88
REFUSED: configuration mismatch: TEKMERION_ALG_CLOSE recorded unset, live "arm.bignum_add_p25519"
exit=2
```

**(2) Unset a recorded-set variable.** A scratch manifest records
`TEKMERION_PROBLEMS=/mnt/1tb_kingston/Tekmerion/tests/fixtures/problems.json`;
it verifies while the variable is set, and refuses when it is unset.

```
$ TEKMERION_PROBLEMS=/mnt/1tb_kingston/Tekmerion/tests/fixtures/problems.json \
      tools/cap.sh 20G -- cargo run --release --offline --bin freeze -- manifest \
      --out /tmp/ix74-config-control.json --tag freeze-rust-closure-v6 \
      --hol .bench/hol-light-stamp2
wrote /tmp/ix74-config-control.json: 72 source files,
source_digest 66168775f1a7c6317782b78e392488640ed0d459ed0dfae05c58c1daf3b0cb88,
0 runtime digests, repo_commit 97514a44f541ee0db1b8baa6fd7479367d6801fb

$ TEKMERION_PROBLEMS=/mnt/1tb_kingston/Tekmerion/tests/fixtures/problems.json \
      tools/cap.sh 20G -- cargo run --release --offline --bin freeze -- verify \
      --manifest /tmp/ix74-config-control.json
OK: source closure verified: 72 files, source_digest 66168775f1a7c6317782b78e392488640ed0d459ed0dfae05c58c1daf3b0cb88
OK: effective configuration verified: 6 TEKMERION_* variables
exit=0

$ env -u TEKMERION_PROBLEMS tools/cap.sh 20G -- \
      cargo run --release --offline --bin freeze -- verify \
      --manifest /tmp/ix74-config-control.json
OK: source closure verified: 72 files, source_digest 66168775f1a7c6317782b78e392488640ed0d459ed0dfae05c58c1daf3b0cb88
REFUSED: configuration mismatch: TEKMERION_PROBLEMS recorded "/mnt/1tb_kingston/Tekmerion/tests/fixtures/problems.json", live unset
exit=2
```

### 2.6 Compatibility: an old manifest still verifies, with a warning

The configuration line was removed from a copy of v6 (the source list and
`source_digest` are untouched, so the manifest is exactly an old-style record):

```
$ grep -v '"configuration"' research/receipts/freeze-closure-v6.json > /tmp/ix74-legacy-scope.json
$ tools/cap.sh 20G -- cargo run --release --offline --bin freeze -- verify \
      --manifest /tmp/ix74-legacy-scope.json
OK: source closure verified: 72 files, source_digest 66168775f1a7c6317782b78e392488640ed0d459ed0dfae05c58c1daf3b0cb88
WARNING: manifest records no effective configuration; the TEKMERION_* variables that change the reasoning path are not bound (legacy scope).
exit=0
```

The schema string is unchanged (`tekmerion-freeze-closure/1`); the new field is
optional, so old manifests parse under their own scope and are never silently
upgraded. `a_manifest_without_configuration_still_parses_and_warns` in
`tests/tools_freeze.rs` pins this.

---

## 3. The v6 manifest

* Path: `research/receipts/freeze-closure-v6.json`
* `source_digest`: `66168775f1a7c6317782b78e392488640ed0d459ed0dfae05c58c1daf3b0cb88`
* `repo_commit`: `97514a44f541ee0db1b8baa6fd7479367d6801fb` (the code commit the
  manifest was generated against; the commit that adds this receipt is a
  descendant that changes no source-closure file, so the same digest holds)
* `tag`: `freeze-rust-closure-v6` **named only — no tag was created**
* 72 source files, 10 runtime digests
* runtime layer: `target/release/emit`
  (`c198dace0dc9fba50a1c4837d6348ab1038c89271361704d3cbdac9c9e10bff4`),
  `rustc 1.98.0 (88d9e12ae 2026-08-18)`, HOL Light pin `d30e823` with
  `hol_lib.cmxa`, cadical `2.1.2`, the six `tools/*.ml` bridges, and
  `.bench/problems.json`.

Regeneration command:

```
tools/cap.sh 20G -- cargo run --release --offline --bin freeze -- manifest \
  --out research/receipts/freeze-closure-v6.json \
  --tag freeze-rust-closure-v6 --hol .bench/hol-light-stamp2 --runtime
```

(`--hol` is required because the default `.bench/hol-light-stamp` no longer
exists on this host; the live stamped kernel is `.bench/hol-light-stamp2`.)

---

## 4. Validation

```
$ tools/cap.sh 20G -- cargo test --offline
...
running 117 tests          (lib)
test result: ok. 117 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
... every integration suite: test result: ok. N passed; 0 failed ...
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out   (doc-tests)
cap.sh: group=2243660 rc=0 sampled_group_peak_rss_kb=2885824 max_process_vmhwm_kb=713624
exit=0
```

```
$ tools/cap.sh 20G -- cargo clippy --offline --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.02s
cap.sh: group=2247907 rc=0 sampled_group_peak_rss_kb=76260 max_process_vmhwm_kb=54924
exit=0

$ cargo fmt --check
exit=0   (no output)

$ git ls-files '*.py'
(no output)
```

Disk after all runs: `/dev/nvme1n1p1  1.3T  901G  361G  72% /mnt/1tb_kingston`
(361 GB free, no cap fired, no OOM).

---

## 5. What is still not bound (honest list)

* **Environment outside `TEKMERION_*`.** `PATH` is read by `disasm.rs`; locale,
  `TZ`, CPU count, frequency scaling, memory pressure, other tenants and the
  wall-clock at which a timing ran are not recorded. Timings are scoped to the
  machine and the run (as `research/receipts/` already says) and are not
  portable.
* **`mode`/`stamp`/`force` flags are recorded, not verified.** `freeze verify`
  compares the environment only: there is no live flag state to compare. The
  manifest records them as `unspecified` unless the caller passed the flags.
* **The guard path.** `guard::verify_freeze` (the sealed-run guard) calls
  `freeze::verify_source` and `freeze::verify_runtime`; it does **not** yet call
  `config::verify_live`, because `src/tools/guard.rs` was outside this task's
  write set. `freeze verify` does refuse. Closing the guard gap is one call in
  `guard.rs` and should be a separate, reviewed change.
* **Non-UTF-8 values.** A variable's value is recorded lossily
  (`to_string_lossy`); a non-UTF-8 value on this host is not byte-preserved.
  Every declared variable carries a path, task id or flag token, so this was
  not observed.
* **The literal `unset`.** A variable actually set to the string `unset` is
  indistinguishable from an unset one in the recorded form.
* **No real human-vs-generated identity-bound pair in this receipt.** The
  digest scheme was exercised on real `.bench/ix69` generated artifacts
  (baseline vs sliced), and the unit fixtures cover all four type cases. A
  human `_CORRECT` artifact for the IX.65 six measured tasks is not present in
  `.bench`; producing one requires re-running those proofs, which this task did
  not do.
* **`TEKMERION_SLICE_DEBUG`** changes diagnostics rather than the proof, but it
  changes what is printed on the reasoning path, so it is declared and bound
  rather than omitted.
* **No tag.** `freeze-rust-closure-v6` is only the `tag` field in the manifest.
  Sealing is a separate decision and was not taken.
