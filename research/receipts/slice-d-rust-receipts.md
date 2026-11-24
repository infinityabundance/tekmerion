# Slice D — the last five Python files, ported to Rust

- **Date:** 2026-11-24
- **Branch:** `staging` (base commit `b28016c`)
- **Scope:** `research/receipts/**.py` — the only tracked Python left after Slices A–C.
- **Result:** four files ported byte-identically; one (`build_all.py`) retired in favour of
  `dropin sweep`. `git ls-files '*.py'` is empty. No `.bench` file and no benchmark script
  was touched.

## 1. What moved

| former Python | Rust module | Rust binary |
|---|---|---|
| `research/receipts/bridge/curve_check.py` | `src/tools/curve_check.rs` | `curve_check` |
| `research/receipts/bridge/instrument.py` | `src/tools/instrument.rs` | `instrument` |
| `research/receipts/bridge/make_inlined_answer.py` | `src/tools/inline_answer.rs` | `inline_answer` |
| `research/receipts/ix54/build-sealed-court-v3.py` | `src/tools/sealed_court.rs` | `sealed_court` |
| `research/receipts/ix54/build_all.py` | `src/tools/dropin.rs` (`sweep`) | `dropin` |

`src/tools/sha256.rs` is a small raw-digest helper over the already-vendored `sha2`
crate; no dependency was added.

## 2. Golden table

Goldens were captured **before** any deletion into `.bench/slice-d-golden/` (git-ignored).
Each Rust figure is the output of the Rust binary invoked with the *same argument strings*
as the Python original.

| artefact | Python sha256 | Rust sha256 | identical |
|---|---|---|---|
| `curve_check` stdout, `tools/bridge.ml` | `080cb2a11dcd3ef75b0d09227d506118620d8cfb6ea6427c5f800ce589a9f581` | `080cb2a11dcd3ef75b0d09227d506118620d8cfb6ea6427c5f800ce589a9f581` | yes |
| `curve_check` exit code (71 statements, 0 hits) | `0` | `0` | yes |
| `instrument` output file, argv `tools/bridge.ml <out>` | `f4bba97fba1ba3281acd60cb8aa09275ef7f62a205d3c32254c858a5999c5b23` | `f4bba97fba1ba3281acd60cb8aa09275ef7f62a205d3c32254c858a5999c5b23` | yes |
| `instrument` stdout (`wrote … sha256=ceb690a8… phrases=119`) | `38ab10a276a4424494192487f2047ef1bee5e34b05df0e22ab6fd5fb0781c9e3` | `38ab10a276a4424494192487f2047ef1bee5e34b05df0e22ab6fd5fb0781c9e3` | yes |
| `inline_answer` `add-ref` | `39fd3d64c65063410c02f0a879d505632b4d82042ac3a64f26c137426e23264f` | `39fd3d64c65063410c02f0a879d505632b4d82042ac3a64f26c137426e23264f` | yes |
| `inline_answer` `sub` | `b36a7fd0274396d72bdbe87f6cd8810888ad308bfe415e952f9bfed015c9e176` | `b36a7fd0274396d72bdbe87f6cd8810888ad308bfe415e952f9bfed015c9e176` | yes |
| `inline_answer` `neg` | `b5a674d51aeb621f11611200a7e47451b4d6ada2e434728bf60e6eb49143cb6e` | `b5a674d51aeb621f11611200a7e47451b4d6ada2e434728bf60e6eb49143cb6e` | yes |
| `inline_answer` `add-p384` | `83d90067847d411d3eee7fb7b6ca6b532d790ab262341e6ba5d248d54db15044` | `83d90067847d411d3eee7fb7b6ca6b532d790ab262341e6ba5d248d54db15044` | yes |
| `inline_answer` stdout, `add-ref` | `0c633448ba9da4a6eb2bbeb1752ce7205c9e844d45545a68a85f59b858645b8b` | `0c633448ba9da4a6eb2bbeb1752ce7205c9e844d45545a68a85f59b858645b8b` | yes |
| `inline_answer` stdout, `sub` | `177fbe6dec27ff2dbab809ed9534142e58bb4c9ea9d70d10dedd43a5447535c5` | `177fbe6dec27ff2dbab809ed9534142e58bb4c9ea9d70d10dedd43a5447535c5` | yes |
| `inline_answer` stdout, `neg` | `ed5146954a0df8d519faaa9e6d3d4a6ac99bdceabdd91f6f2c885395942a1c87` | `ed5146954a0df8d519faaa9e6d3d4a6ac99bdceabdd91f6f2c885395942a1c87` | yes |
| `inline_answer` stdout, `add-p384` | `08fbed417093c6b516ffa54e2b8e25ddda18a5e7582ab7284080ad566a5f605a` | `08fbed417093c6b516ffa54e2b8e25ddda18a5e7582ab7284080ad566a5f605a` | yes |
| `sealed_court` stdout (per-template lines + `du -sh`) | `a5697894990f32c5877fbab6e19b96f3295cb9b4fbb367b33ffa1637c7ecc585` | `a5697894990f32c5877fbab6e19b96f3295cb9b4fbb367b33ffa1637c7ecc585` | yes |
| `sealed_court` tree manifest (928 regular files, sorted) | `107f0d9821bdda30df75dfb24626d262fb4866927800ff0f84ccc1884404b626` | `107f0d9821bdda30df75dfb24626d262fb4866927800ff0f84ccc1884404b626` | yes |
| `sealed_court` `diff -r --no-dereference` | — | — | clean (empty, exit 0) |

The instrument phrase count is `119`; the inline answer has `155` bindings and the same
byte length in every mode except the tactic (103709 / 104185 / 103541 / 104081). The
`curve_check` report contains `statements=71 hits=0`.

A second instrument run from two different working directories with the identical
*relative* argv (`../../../tools/bridge.ml instrument.ml`) produced identical stdout and
identical files (sha256 `7da2b227c9f713f89c1c3c16d70dfffd341c8882618b73ba45de8a202950db0b`
for both), confirming that the only difference between runs is the source label embedded in
the header.

## 3. Commands run, verbatim

### 3.1 `cargo test --offline`

```
$ cargo test --offline
...
     Running unittests src/lib.rs
test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/native.rs
test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/native_semantics.rs
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/refusals.rs
test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/semantics.rs
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/tools.rs
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/tools_attempt.rs
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/tools_derive.rs
test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/tools_emit.rs
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.02s
     Running tests/tools_fcdist.rs
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.03s
     Running tests/tools_guard.rs
test result: ok. 26 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/tools_patch_wrapper.rs
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/tools_receipt.rs
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/tools_trace_conformance.rs
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/write_through.rs
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
   Doc-tests tekmerion
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

$ awk '/test result:/{p+=$4; f+=$6} END{print "passed="p" failed="f}' cargo-test.log
passed=224 failed=0
```

(33 harnesses: the library unit tests, 19 zero-test binary harnesses, 14 integration
suites, and doc-tests. The new `tests/tools_receipt.rs` contributes 10 passing tests, and
the four new `#[cfg(test)]` modules — `curve_check`, `instrument`, `inline_answer`,
`sealed_court`, `sha256` — run as part of the library harness.)

### 3.2 `cargo clippy --offline --all-targets -- -D warnings`

```
    Checking tekmerion v0.1.0 (/mnt/1tb_kingston/Tekmerion)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.02s
```

Exit 0, no warnings.

### 3.3 `bash tools/selftest.sh`

```
valid mutants: 5  killed: 5  survived: 0
lifecycle mutation score: 5/5 = 1.000
parser mutation score: 16/16 = 1.000
OK: cargo run --bin mutate_lifecycle passed

SELF-TEST PASSED
```

Exit 0.

### 3.4 `git ls-files '*.py'`

```
$ git ls-files '*.py'
```

Empty (exit 0). The five tracked files were deleted; `.bench/**/*.py` was never tracked and
remains (benchmark-owned, untouched).

### 3.5 References to the deleted scripts in Markdown/shell

```
$ grep -rn "python3 .*research/receipts" --include='*.md' --include='*.sh' . | grep -v '^./.bench'
./research/receipts/ix53-spent-mode.md:62:$ python3 tools/heldout_guard.py .bench/s2n-bignum/arm/secp256k1/bignum_add_p256k1.o --spent --spent-manifest research/receipts/spent-r4-arm.json --root .bench/s2n-bignum
./research/receipts/ix53-spent-mode.md:66:$ python3 tools/heldout_guard.py .bench/s2n-bignum/arm/proofs/bignum_add_p256k1.ml --spent --spent-manifest research/receipts/spent-r4-arm.json --root .bench/s2n-bignum
./research/receipts/ix53-spent-mode.md:74:$ python3 tools/heldout_guard.py .bench/s2n-bignum/x86/curve25519/bignum_add_p25519.o --spent --spent-manifest research/receipts/spent-r4-arm.json --root .bench/s2n-bignum   # not listed (ARM-only manifest)
./research/receipts/ix53-spent-mode.md:82:$ python3 tools/emit_ref_proof.py --spent --spent-manifest research/receipts/spent-r4-arm.json --task arm.bignum_add_p256k1.BIGNUM_ADD_P256K1_CORRECT --mode dev --out /tmp/x.ml   # proof read is still refused
./research/receipts/r4-reference-route.md:81:`python3 research/receipts/bridge/curve_check.py tools/bridge.ml`
./research/receipts/ix53-sealed-run.md:25:$ python3 -c "import sys; sys.path.insert(0,'tools'); import heldout_guard as g; m=g.load_sealed_manifest('research/receipts/ix53-sealed-manifest.json'); g.verify_freeze(m,'freeze-x86-sealed-v2'); print('FREEZE PASS', m.freeze_commit[:12], len(m.sealed_ids))"
./research/receipts/ix53-sealed-run.md:251:python3 -c "import sys; sys.path.insert(0,'tools'); import heldout_guard as g; m=g.load_sealed_manifest('research/receipts/ix53-sealed-manifest.json'); g.verify_freeze(m,'freeze-x86-sealed-v2'); print('FREEZE PASS', m.freeze_commit[:12], len(m.sealed_ids))"
./research/receipts/r5-xzr.md:331:$ python3 research/receipts/bridge/curve_check.py tools/bridge.ml
./research/receipts/r6-x86.md:280:python3 research/receipts/bridge/curve_check.py tools/bridge.ml
```

Every remaining hit is **historical prose**: a command line that was true when written.
`r4-reference-route.md`, `r5-xzr.md` and `r6-x86.md` each carry a Slice D addendum giving
the Rust replacement; `ix53-spent-mode.md` and `ix53-sealed-run.md` are outside this
slice's edit list and still describe the Slice A–C era.

### 3.6 `sha256sum` of the golden and Rust artefacts

```
$ sha256sum .bench/slice-d-golden/golden/* .bench/slice-d-golden/*.txt .bench/slice-d-golden/*.stdout
83d90067847d411d3eee7fb7b6ca6b532d790ab262341e6ba5d248d54db15044  .bench/slice-d-golden/golden/inline-add-p384.txt
39fd3d64c65063410c02f0a879d505632b4d82042ac3a64f26c137426e23264f  .bench/slice-d-golden/golden/inline-add-ref.txt
b5a674d51aeb621f11611200a7e47451b4d6ada2e434728bf60e6eb49143cb6e  .bench/slice-d-golden/golden/inline-neg.txt
b36a7fd0274396d72bdbe87f6cd8810888ad308bfe415e952f9bfed015c9e176  .bench/slice-d-golden/golden/inline-sub.txt
f4bba97fba1ba3281acd60cb8aa09275ef7f62a205d3c32254c858a5999c5b23  .bench/slice-d-golden/golden/instrument.ml
080cb2a11dcd3ef75b0d09227d506118620d8cfb6ea6427c5f800ce589a9f581  .bench/slice-d-golden/curve_check.rust.txt
080cb2a11dcd3ef75b0d09227d506118620d8cfb6ea6427c5f800ce589a9f581  .bench/slice-d-golden/curve_check.txt
83d90067847d411d3eee7fb7b6ca6b532d790ab262341e6ba5d248d54db15044  .bench/slice-d-golden/inline-add-p384.txt
39fd3d64c65063410c02f0a879d505632b4d82042ac3a64f26c137426e23264f  .bench/slice-d-golden/inline-add-ref.txt
b5a674d51aeb621f11611200a7e47451b4d6ada2e434728bf60e6eb49143cb6e  .bench/slice-d-golden/inline-neg.txt
b36a7fd0274396d72bdbe87f6cd8810888ad308bfe415e952f9bfed015c9e176  .bench/slice-d-golden/inline-sub.txt
08fbed417093c6b516ffa54e2b8e25ddda18a5e7582ab7284080ad566a5f605a  .bench/slice-d-golden/inline-add-p384.rust.stdout
08fbed417093c6b516ffa54e2b8e25ddda18a5e7582ab7284080ad566a5f605a  .bench/slice-d-golden/inline-add-p384.stdout
0c633448ba9da4a6eb2bbeb1752ce7205c9e844d45545a68a85f59b858645b8b  .bench/slice-d-golden/inline-add-ref.rust.stdout
0c633448ba9da4a6eb2bbeb1752ce7205c9e844d45545a68a85f59b858645b8b  .bench/slice-d-golden/inline-add-ref.stdout
ed5146954a0df8d519faaa9e6d3d4a6ac99bdceabdd91f6f2c885395942a1c87  .bench/slice-d-golden/inline-neg.rust.stdout
ed5146954a0df8d519faaa9e6d3d4a6ac99bdceabdd91f6f2c885395942a1c87  .bench/slice-d-golden/inline-neg.stdout
177fbe6dec27ff2dbab809ed9534142e58bb4c9ea9d70d10dedd43a5447535c5  .bench/slice-d-golden/inline-sub.rust.stdout
177fbe6dec27ff2dbab809ed9534142e58bb4c9ea9d70d10dedd43a5447535c5  .bench/slice-d-golden/inline-sub.stdout
fbb2488a1ac5e4a38961fa37e61099558d77c720a905120c821e4cb893385d15  .bench/slice-d-golden/instrument.py.stdout
38ab10a276a4424494192487f2047ef1bee5e34b05df0e22ab6fd5fb0781c9e3  .bench/slice-d-golden/instrument.rust2.stdout
fbb2488a1ac5e4a38961fa37e61099558d77c720a905120c821e4cb893385d15  .bench/slice-d-golden/instrument.rust.stdout
38ab10a276a4424494192487f2047ef1bee5e34b05df0e22ab6fd5fb0781c9e3  .bench/slice-d-golden/instrument.stdout
a5697894990f32c5877fbab6e19b96f3295cb9b4fbb367b33ffa1637c7ecc585  .bench/slice-d-golden/sealed-court.rust.stdout
a5697894990f32c5877fbab6e19b96f3295cb9b4fbb367b33ffa1637c7ecc585  .bench/slice-d-golden/sealed-court.stdout
```

The `*.rust.txt` / `*.rust.stdout` files are the Rust re-runs with the original argv; each
pairs line-for-line with its Python counterpart. `instrument.py.stdout` and
`instrument.rust.stdout` are the two-working-directory run described above.

## 4. What is NOT covered

1. **The nine-task `dropin sweep` build is unexercised.** No `make`/HOL Light run was
   performed. `sweep` is verified by `--help`, argument parsing, unit tests, and
   `--dry-run` over all nine tasks. See `research/receipts/ix54-dropin-retire.md`.
2. **The sealed container has not been re-run** since `run-inside.sh` was changed to use
   the prebuilt `emit_x86` binary via `EMIT_X86`. The container mounts only select paths;
   the next run must mount that binary read-only and ensure the emitter resolves its
   repository root to the court tree.
3. **`curve_check` word/number/whitespace classes are ASCII.** Python's `\b`, `\d` and
   `str.isspace` are Unicode-aware; the port uses ASCII equivalents. The corpus
   (`tools/bridge.ml`) is pure ASCII, so the recorded output is identical. A non-ASCII
   source could differ on exotic characters.
4. **`instrument` does not translate CRLF.** Python opened the source with universal
   newlines; the port hashes and transforms the raw bytes. All inputs here are LF, where
   the two coincide.
5. **`inline_answer` resolves sources under `CARGO_MANIFEST_DIR`** (the repository the
   binary was built in) instead of the Python's hard-coded `/mnt/1tb_kingston/Tekmerion`.
   The same five files are read; the output digests above confirm it.
6. **The Python `sealed_court` builder only works when its copy lives in `.bench/`**
   (its repository root is derived from `__file__`). The tracked copy
   (`d3690cd20fe81fa34c645b0a5bdf2974f825fad1050420f2350e6b320075f180`) run in place
   fails with `FileNotFoundError: .../research/receipts/.bench/problems.json`; the golden
   was therefore captured with the byte-identical `.bench/build-sealed-court-v3.py` copy.
   The Rust binary has no such restriction (`--repo` / `--court`).
