# IX.72 — Offline input fixtures for the emitter parity suite (XVI.A1–XVI.A3)

- **Date:** 2026-11-25
- **Branch:** `staging` (one commit)
- **Scope:** `src/tools/emit.rs` (corpus resolution), `src/tools/json.rs`
  (raw-span reader), `src/tools/corpus.rs` (new task list and fixture paths),
  `src/tools/mod.rs`, `src/bin/record_problems_fixture.rs` (new recorder),
  `tests/*`, `tests/fixtures/problems.json` (new),
  `tests/fixtures/bench/**` (new: 27 ELF objects, 3 Cadical sources), this
  receipt, and the `XVI.A*` lines of `TODO.md`. No `tools/*.ml`, no
  `tools/hol-light/axiom-stamp.diff`, no `src/native/`, no `verification/`, no
  other receipt, no other `research/receipts/*.json`, no `.bench` write, no tag.
  `.bench/` was read only (the recorder and the live-agreement test).
- **Environment:** rustc/cargo 1.98.0, rustfmt 1.9.0-stable, `objdump`
  (binutils) on `PATH`, `git` on `PATH`, OCaml 5.5.0. One host.

## Resource check before the runs (AGENTS.md §7a)

The brief fixes the cap at 20 G. The 3× rule therefore needs ≥ 60 G available.

```text
$ free -g | head -2
               total        used        free      shared  buff/cache   available
Mem:             125          28          89           1           9          97
$ df -h /mnt/1tb_kingston | tail -1
/dev/nvme1n1p1  1.3T  1.3T  7.8G 100% /mnt/1tb_kingston
```

97 G available is 4.85× the 20 G cap; 7.8 G free on the volume is above the
6 G floor. Every build and test run below went through `tools/cap.sh 20G --`.
**No HOL Light proof build was started.** Because `/tmp` is a 63 G tmpfs, the
bench-free copy excludes `target` as well as `.bench` (a copy of the 12 G target
tree would have consumed RAM, not disk).

After the runs (largest single new artefact is `target/release`):

```text
$ free -g | head -2
               total        used        free      shared  buff/cache   available
Mem:             125          28          91           2           8          96
$ df -h /mnt/1tb_kingston | tail -1
/dev/nvme1n1p1  1.3T  1.3T  7.0G 100% /mnt/1tb_kingston
```

## The defect, and the part of it the earlier fix did not see

CI is red on three tests: `x86_answers_are_byte_identical`,
`arm_answers_are_byte_identical` and `sealed_freeze_verification_end_to_end`.
The reference **answers** were committed in `tests/fixtures/answers/`
(`a1546d8`), so `have_reference()` no longer returned false in a bench-free
checkout. The tests then ran and reached the emitter, which reads
`<repo>/.bench/problems.json` — a 42 MB, git-ignored file. They error instead of
testing. Reproduced in a byte copy of the tree without `.bench`:

```text
$ (cd /tmp/ci-xvia && tools/cap.sh 20G -- cargo test --offline --test tools_emit \
      -- x86_answers_are_byte_identical arm_answers_are_byte_identical \
         sealed_freeze_verification_end_to_end)
running 3 tests
test arm_answers_are_byte_identical ... FAILED
test x86_answers_are_byte_identical ... FAILED
test sealed_freeze_verification_end_to_end ... FAILED

thread 'arm_answers_are_byte_identical' panicked at tests/tools_emit.rs:217:33:
arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT: /tmp/ci-xvia/.bench/problems.json: No such file or directory (os error 2)
thread 'x86_answers_are_byte_identical' panicked at tests/tools_emit.rs:189:33:
x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT: /tmp/ci-xvia/.bench/problems.json: No such file or directory (os error 2)
thread 'sealed_freeze_verification_end_to_end' panicked at tests/tools_emit.rs:339:60:
build: Other("/tmp/tek-emit-2142880-freeze-0/.bench/problems.json: No such file or directory (os error 2)")
```

The corpus is only the **first** missing input. Giving the copy a real corpus
moved the failure to the next one:

```text
thread 'x86_answers_are_byte_identical' panicked at tests/tools_emit.rs:189:33:
x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT: /tmp/ci-xvia/.bench/s2n-bignum/x86: No such file or directory (os error 2)
```

The emitter also disassembles an ELF object for the operation
(`emit::object_for` → `disasm::disassemble`) and inlines three Cadical sources
(`emit::ANSWER_SOURCES`). A fixture containing only `problems.json` would not
make the tests run; it would move the failure one step right. This receipt
therefore commits the **whole input set** for the 27 tasks, not just the corpus:

| input | live path | committed fixture | size |
|---|---|---|---|
| corpus excerpt (27 entries) | `.bench/problems.json` | `tests/fixtures/problems.json` | **40,104 B** |
| 27 ELF objects | `.bench/s2n-bignum/{arm,x86}/…` | `tests/fixtures/bench/s2n-bignum/…` | 47,782 B |
| 3 Cadical sources | `.bench/hol-light/Cadical/` | `tests/fixtures/bench/hol-light/Cadical/` | 20,089 B |

`tests/fixtures/bench/` holds 30 files totalling **51,985 B**. Together with the
corpus excerpt the tracked input fixtures are **92,089 B**.

## 1. The corpus override (XVI.A)

`src/tools/emit.rs` now resolves the corpus through one documented function:

```rust
pub const PROBLEMS_ENV: &str = "TEKMERION_PROBLEMS";

fn problems_path(repo: &Path) -> PathBuf {
    let live = bench(repo).join("problems.json");
    if live.is_file() {
        return live;
    }
    match std::env::var_os(PROBLEMS_ENV) {
        Some(value) if !value.is_empty() => PathBuf::from(value),
        _ => live,
    }
}
```

- With `TEKMERION_PROBLEMS` **unset**, resolution is exactly
  `<repo>/.bench/problems.json` — the historical default, unchanged.
- The variable is a **fallback, not a shadow**: a repo-local
  `.bench/problems.json` always wins. That is deliberate. It means (a) a
  committed fixture can never silently replace the live pin, and (b) the tests
  in `tools_emit.rs` that write their own synthetic `.bench/problems.json` are
  unaffected when another test in the same process sets the variable. The
  live-agreement test in §5 is what ties the fixture to the pin.
- The doc comment on `PROBLEMS_ENV` states this order and points at XVI.C1.

The three tests materialise `tests/fixtures/bench/` into a temporary `.bench`
and pass that root to the emitter; the corpus is **not** copied into it, so the
override is genuinely load-bearing in both a bench-free and a live tree.

### Freeze binding is pending

`TEKMERION_PROBLEMS` changes which corpus is read without changing any source
file or executable hash, so it belongs in the effective-configuration record
(`TODO.md` **XVI.C1**). **XVI.C has no list of variables yet** (XVI.C1/C2 are
unticked and there is no `TEKMERION_*` registry in the tree), so there is
nothing to append to. *Pending:* when XVI.C2 enumerates the variables the
binary reads, `TEKMERION_PROBLEMS` (read in `src/tools/emit.rs`) must be one of
them and XVI.C3 must refuse a recorded/live mismatch naming it.

## 2. The fixture and its digest

```text
$ sha256sum tests/fixtures/problems.json
2cded7f1bec3b0b341499742342585ae3ab692d9f682e90046580e41dc1b4909  tests/fixtures/problems.json
$ stat -c '%s bytes' tests/fixtures/problems.json
40104 bytes
```

- Exactly **27** top-level entries: 18 ARM + 9 x86, in the canonical order held
  in `src/tools/corpus.rs`.
- Every field of each live entry is kept (`json`, `category`, `query`,
  `legacy_id`, `inlined_locations`), because the entries are copied as **raw
  byte slices** from the live file; nothing is re-serialised. The emitter reads
  `query`, `json.theorem_name` and `inlined_locations`; the rest is kept so the
  entry is a faithful excerpt, not a lossy projection.
- Size: mean ~1,485 B per pretty-printed entry; compact JSON lengths range
  1,165–1,418 B. No entry is unexpectedly large — the bulk is the `query`
  string, which is the proof obligation itself. The whole fixture is ~40 KB,
  well under the 1 MB review target and 1/1000th of the live corpus.
- The object fixtures are the exact ELF bytes the emitter disassembles; the
  Cadical fixtures are the exact texts it inlines. They are binary/text inputs,
  not readable in review, so §5 compares every byte with the live pin.

## 3. Recorder, not hand-editing

`src/bin/record_problems_fixture.rs` extracts the 27 entries (raw spans) and
copies the 27 objects and 3 Cadical sources. It prints each written file's
size and SHA-256 and refuses to write if an entry or object is missing.

```text
$ tools/cap.sh 20G -- cargo run --release --offline --bin record_problems_fixture -- --repo /mnt/1tb_kingston/Tekmerion
    Finished `release` profile [optimized + debuginfo] target(s) in 4.53s
     Running `target/release/record_problems_fixture --repo /mnt/1tb_kingston/Tekmerion`
/mnt/1tb_kingston/Tekmerion/tests/fixtures/problems.json	40104	2cded7f1bec3b0b341499742342585ae3ab692d9f682e90046580e41dc1b4909
/mnt/1tb_kingston/Tekmerion/tests/fixtures/bench/s2n-bignum/arm/curve25519/bignum_add_p25519.o	1176	6d8f…
… (27 object lines, each size + sha256) …
/mnt/1tb_kingston/Tekmerion/tests/fixtures/bench/hol-light/Cadical/dimacs.ml	5223	3f891558cef8e1e7dc390232fee53bb434950b6bb7578b9c2e5c28c76cf11aad
/mnt/1tb_kingston/Tekmerion/tests/fixtures/bench/hol-light/Cadical/cnf.ml	7453	7813b91b504bfda9ad0d4e404fa1172f85de4480500a3972b0b726732e3eb581
/mnt/1tb_kingston/Tekmerion/tests/fixtures/bench/hol-light/Cadical/ldrup.ml	7413	3a34bfe69968dcdec7b6523ed1fb70b8d6806b1f24b9cc25b161b7f737086d1a
recorded 31 files, 92089 bytes (problems.json 40104 bytes, 27 object files, 3 Cadical sources)
```

Regenerating deliberately is exactly that command. `tests/fixtures/bench/.gitignore`
re-includes `*.o` under that directory only (the root `.gitignore` excludes `*.o`
because they are build artefacts everywhere else); this is the one-line reason
the 27 reviewed objects can be tracked without force-adding ignored files.

## 4. Bench-free run (XVI.A3)

The CI condition is reproduced by a copy without `.bench` **and** without
`target` (`/tmp` is tmpfs):

```text
$ rsync -a --exclude=.bench --exclude=target ./ /tmp/ci-xvia/
$ (cd /tmp/ci-xvia && tools/cap.sh 20G -- cargo test --offline)
…
     Running tests/tools_emit.rs (…)
running 14 tests
test arm_answers_are_byte_identical ... ok
test arm_dev_mode_refuses_a_spent_proof_file ... ok
test fixture_inputs_match_the_live_bench_when_present ... ok
test ix66_markers_are_additive_and_the_normal_answer_is_recovered ... ok
test ix69_slice_falls_back_to_the_full_set_and_the_prefix_names_the_task ... ok
test ix70_algebraic_close_is_opt_in_and_one_task_only ... ok
test phase_markers_are_removable_and_the_class_is_mechanical ... ok
test phase_mode_is_the_normal_emitter_plus_marks ... ok
test sealed_freeze_verification_end_to_end ... ok
test template_prefix_keeps_everything_when_linenum_exceeds_the_file ... ok
test template_prefix_refuses_a_task_without_locations ... ok
test template_prefix_takes_exactly_the_harness_line_count ... ok
test x86_answers_are_byte_identical ... ok
test x86_dev_mode_refuses_a_sealed_proof_file ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
…
   Doc-tests tekmerion
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
```

All **41** test binaries report `test result: ok`; there is no `FAILED` and no
`error[` line. The three named tests are in the 14 above: they executed, they
did not skip.

The three, isolated and uncaptured, in the same bench-free copy:

```text
$ (cd /tmp/ci-xvia && tools/cap.sh 20G -- cargo test --offline --test tools_emit \
      -- --nocapture --test-threads=1 x86_answers_are_byte_identical \
         arm_answers_are_byte_identical sealed_freeze_verification_end_to_end)
running 3 tests
test arm_answers_are_byte_identical ... ok
test sealed_freeze_verification_end_to_end ... warning: freeze manifest has no freeze_closure; the legacy scope (tools/) does not bind the Rust producer in src/tools, src/bin or target/release. See research/receipts/ix59-freeze-closure.md.
ok
test x86_answers_are_byte_identical ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 11 filtered out; finished in 0.18s
```

The real tree, same three plus the agreement test:

```text
$ tools/cap.sh 20G -- cargo test --offline --test tools_emit -- --nocapture --test-threads=1 \
      x86_answers_are_byte_identical arm_answers_are_byte_identical \
      sealed_freeze_verification_end_to_end fixture_inputs_match_the_live_bench_when_present
running 4 tests
test arm_answers_are_byte_identical ... ok
test fixture_inputs_match_the_live_bench_when_present ... ok
test sealed_freeze_verification_end_to_end ... ok
test x86_answers_are_byte_identical ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.38s
```

The whole real-tree suite (`tools/cap.sh 20G -- cargo test --offline`) is also
green: 41 `test result: ok`, including `tools_emit` 14/14.

## 5. Deliberate failures

**Missing fixture.** In the bench-free copy, moving the fixture away makes all
three tests fail, naming it (XVI.A2):

```text
$ mv /tmp/ci-xvia/tests/fixtures/problems.json /tmp/ci-xvia/tests/fixtures/problems.json.moved
$ (cd /tmp/ci-xvia && tools/cap.sh 20G -- cargo test --offline --test tools_emit \
      -- x86_answers_are_byte_identical arm_answers_are_byte_identical \
         sealed_freeze_verification_end_to_end)
test arm_answers_are_byte_identical ... FAILED
test sealed_freeze_verification_end_to_end ... FAILED
test x86_answers_are_byte_identical ... FAILED

thread 'arm_answers_are_byte_identical' panicked at tests/tools_emit.rs:85:5:
missing input fixture /tmp/ci-xvia/tests/fixtures/problems.json: regenerate it with `cargo run --release --offline --bin record_problems_fixture -- --repo /tmp/ci-xvia` (see research/receipts/ix72-offline-inputs.md)
… (same message for the other two) …
test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 11 filtered out; finished in 0.00s
$ mv /tmp/ci-xvia/tests/fixtures/problems.json.moved /tmp/ci-xvia/tests/fixtures/problems.json
```

**Live agreement bites.** In the real tree, one whitespace change to a fixture
entry makes the agreement test fail at the first differing byte; the fixture was
then restored and its digest rechecked:

```text
$ cp tests/fixtures/problems.json /tmp/problems.json.orig
$ sed -i '0,/"category":/s//"category" :/' tests/fixtures/problems.json
$ tools/cap.sh 20G -- cargo test --offline --test tools_emit fixture_inputs_match_the_live_bench_when_present
thread 'fixture_inputs_match_the_live_bench_when_present' panicked at tests/tools_emit.rs:198:5:
arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT: first difference at byte 494 (got 1433 bytes, want 1432 bytes)
  got:  "_colnum_end\": \"73\"\n    },\n    \"category\" : \"functional_correctness_arm\",\n    \"qu"
  want: "_colnum_end\": \"73\"\n    },\n    \"category\": \"functional_correctness_arm\",\n    \"que"
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 13 filtered out; finished in 0.20s
$ cp /tmp/problems.json.orig tests/fixtures/problems.json
$ sha256sum tests/fixtures/problems.json
2cded7f1bec3b0b341499742342585ae3ab692d9f682e90046580e41dc1b4909  tests/fixtures/problems.json
```

The live-agreement test also asserts the fixture's own shape (exactly 27
entries, key set equal to `corpus::tasks()`) **without** the bench, so a
malformed or shrunk fixture fails offline too. When `.bench` is present it
additionally compares all 27 corpus entries and every object and Cadical file
byte for byte.

## 6. Every skip in the offline suite, with its reason

The following prints a reason and returns. No test returns silently for a
missing input; the two returns that remain without a message are not skips
(`assert_identical`'s equality early-return and similar in-test guards).

| test | reason printed | genuinely machine-specific? |
|---|---|---|
| `tools_derive::real_object_derivation_matches_the_reference_lists` | `.bench/s2n-bignum/arm/p256` absent | yes — needs the pinned `arm/proofs/bignum_*_p256.ml` text for the human accumulate union |
| `tools_derive::x86_endbr64_is_trimmed_and_addresses_are_trimmed_relative` | `.bench/s2n-bignum/x86/p384` absent | yes — p384 object is outside the 27-task fixture set |
| `tools_derive::x86_entry_offset_skips_the_push_prologue` | `.bench/s2n-bignum/x86/p521` absent | yes — p521 object is outside the fixture set |
| `tools_derive::x86_real_accumulate_sets` | `.bench/s2n-bignum/x86/p384` absent | yes — p384/p521 objects are outside the fixture set |
| `tools_guard::sealed_allows_declared_nine_objects` | `.bench/s2n-bignum/x86` absent | not strictly — the 9 objects are in the fixture, but this test reads the live tree directly (follow-up, §7) |
| `tools_guard::spent_allows_listed_object_and_refuses_unlisted` | same | same |
| `tools_fcdist::pinned_checkout_matches_the_recorded_python_report` | `.bench/s2n-bignum` absent | yes — needs a checkout at the pinned commit |
| `tools_receipt::inline_answer_reproduces_all_four_recorded_digests` | `.bench sources are absent` | not strictly — the Cadical sources are in the fixture, but the test reads the live tree directly (follow-up, §7) |
| `tools_emit::fixture_inputs_match_the_live_bench_when_present` | live corpus / `s2n-bignum` / `Cadical` absent | yes — there is no pin to compare against in a bench-free checkout; the fixture's own shape is still asserted |
| `tools_receipt::curve_check_reproduces_the_recorded_report` | `tools/bridge.ml is not at the recorded digest` | **pre-existing drift, not `.bench`**: `BRIDGE_DIGEST` is `ca300bd2…`, the file is `0a135381…` (IX.69/IX.70 changed `bridge.ml`). It skips in the real tree too. Not fixed here — re-recording that digest is unrelated to XVI.A. |
| `tools_receipt::instrument_reproduces_the_recorded_output` | same stale digest | same |

`csat_trim_agreement` and `write_through` run (OCaml/ocamlc are present); their
skips were already printed and did not fire.

## 7. What is still not covered offline, honestly

- **The four `tools_derive` object tests** need object families (p256/p384/p521)
  and the human proof text that are not in the 27-task fixture set. They skip
  with a printed reason. Extending the fixture to those families would need the
  proof `.ml` files too, which are much larger and are not inputs to the emitter
  parity suite.
- **The two `tools_guard` object-allowance tests** could be pointed at
  `tests/fixtures/bench/s2n-bignum` in a follow-up; the 9 x86 objects are
  already committed. Left as printed skips to keep this change to the three
  named tests and the fixture, not to rewrite the guard suite.
- **`tools_receipt::inline_answer_reproduces_all_four_recorded_digests`** could
  likewise read the Cadical fixture; same follow-up.
- **`tools_fcdist::pinned_checkout_matches_the_recorded_python_report`** is
  inherently live: it compares against a checkout at a pinned commit.
- **`tools_receipt` digest tests** are blocked by an unrelated stale
  `BRIDGE_DIGEST`, not by missing bench inputs. They skip identically with the
  full bench present, so this change neither caused nor widened that gap.
- **Disassembler dependence.** The byte-identity tests now take their ELF bytes
  from tracked fixtures, but the emitter still shells out to `objdump`. A
  different binutils that renders a different disassembly could change the
  emitted answer on a different machine; the reference answers pin the pipeline
  for the toolchain used here (the same assumption the tests had against
  `.bench`). CI pins Ubuntu 22.04, as did the run above.
- **`TEKMERION_PROBLEMS` is not yet in a freeze-configuration list** because
  XVI.C has no such list yet; see §1. This is recorded as pending, not done.

## 8. Gates

```text
$ cargo clippy --offline --all-targets -- -D warnings
    Checking tekmerion v0.1.0 (/mnt/1tb_kingston/Tekmerion)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.08s
$ cargo fmt --check
(no output)
$ git ls-files '*.py'
(no output; 0 files)
$ git status --short
 M src/tools/emit.rs
 M src/tools/json.rs
 M src/tools/mod.rs
 M tests/tools_derive.rs
 M tests/tools_emit.rs
 M tests/tools_guard.rs
?? src/bin/record_problems_fixture.rs
?? src/tools/corpus.rs
?? tests/fixtures/bench/
?? tests/fixtures/problems.json
```

(`git status` above was taken before this receipt and `TODO.md` were written;
both are part of the same commit.)

## 9. Files

| file | change |
|---|---|
| `src/tools/emit.rs` | `PROBLEMS_ENV`, `problems_path`, `s2n_root`, public `object_for`; default unchanged when the variable is unset |
| `src/tools/json.rs` | `top_level_members`: raw-span top-level reader (byte-preserving); unit tests |
| `src/tools/corpus.rs` | the 27 task ids (single source of truth), Cadical list, fixture paths, bench materialiser |
| `src/bin/record_problems_fixture.rs` | the recorder |
| `tests/tools_emit.rs` | fixture bench + override; no silent skips; live-agreement test; strict reference-answer lookup |
| `tests/tools_derive.rs`, `tests/tools_guard.rs` | silent `.bench` returns now print a reason |
| `tests/fixtures/problems.json` | 27 entries, 40,104 B, sha256 `2cded7f1…b4909` |
| `tests/fixtures/bench/**` | 27 objects + 3 Cadical sources, 51,985 B |
| `TODO.md` | `XVI.A1`–`XVI.A3` ticked |
