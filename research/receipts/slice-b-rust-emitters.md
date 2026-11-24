# Slice B — Rust proof-production chain (receipt)

Status: complete. The Python guard, step derivation, disassembly and the two
reference-answer emitters are replaced by Rust, their tests are ported to Rust,
and the Python files are deleted. No commit was made; the orchestrator owns git.

## Files

Added:

| Path | Contents |
|---|---|
| `src/tools/guard.rs` | `develop` / `sealed-eval` / `spent` admission control, strict manifest parsing, freeze verification |
| `src/tools/derive.rs` | `Step`, `InstructionSemantics`, AArch64 and x86-64 tables, `derive_accumulate` |
| `src/tools/disasm.rs` | ELF arch detection, `objdump` parsing, `ENDBR64` trimming, entry offsets |
| `src/tools/emit.rs` | ARM and x86 answer/dev emitters (shared spec parsing and answer assembly) |
| `src/bin/guard.rs`, `src/bin/emit.rs`, `src/bin/emit_x86.rs` | CLIs, same flag names as the Python tools |
| `tests/tools_guard.rs` | 26 tests, ported from `test_heldout_modes.py` and `test_guard_refusals.py` |
| `tests/tools_derive.rs` | 25 tests, ported from `test_derive_steps.py` and `test_x86_support.py` |
| `tests/tools_emit.rs` | 5 tests: 27-answer byte identity, dev-mode refusals, freeze end to end |

Rewritten: `src/tools/mutate_guard.rs` now mutates `src/tools/guard.rs` (it
previously scanned `tools/heldout_guard.py`, which this slice deletes).

Deleted:

```
tools/heldout_guard.py
tools/derive_steps.py
tools/disasm_steps.py
tools/emit_ref_proof.py
tools/emit_ref_proof_x86.py
tools/test_derive_steps.py
tools/test_x86_support.py
tools/test_heldout_modes.py
tools/test_guard_refusals.py
```

## Byte identity: 27/27

The reference answers were produced by the Python emitters at the pinned
inputs. Reproducing them requires the freeze check to pass, and the working
tree has necessarily moved since `freeze-x86-sealed-v4` (earlier slices in this
programme deleted tracked files under `tools/`). Running the sealed emitter
in the working tree therefore refuses, correctly:

```
$ ./target/debug/emit_x86 --sealed-eval \
    --sealed-manifest research/receipts/ix55-sealed-manifest.json \
    --freeze-tag freeze-x86-sealed-v4 \
    --task x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT \
    --mode answer --out /tmp/in-tree.txt
refused: working tree has changes to tracked files under tools/: D tools/derive_steps.py
in-tree-rc=1
```

The acceptance run therefore used a pristine worktree at the freeze commit,
with the benchmark tree symlinked in, so `git diff <commit> -- tools` is empty
and the guard accepts the freeze exactly as it did for the Python run:

```
$ git worktree add --detach /tmp/tek-freeze b8c2b6fb28f7ebba65327136d33b046eaa8873a2
$ ln -sfn /mnt/1tb_kingston/Tekmerion/.bench /tmp/tek-freeze/.bench
```

Every x86 task was emitted with `--sealed-eval --sealed-manifest
research/receipts/ix55-sealed-manifest.json --freeze-tag freeze-x86-sealed-v4`,
every ARM task with `--spent --spent-manifest
research/receipts/spent-r4-arm.json`, from `/tmp/tek-freeze`. The Python
baseline check confirms the same worktree reproduces the answers:

```
$ env -C /tmp/tek-freeze python3 tools/emit_ref_proof_x86.py --sealed-eval ... --task x86.bignum_add_p25519... --mode answer --out /tmp/x86-add-p25519.txt
wrote /tmp/x86-add-p25519.txt mode=answer task=x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT entry=0 last=25 neg=False accumulate=[2, 4, 6, 8, 13, 14, 15, 16] bytes=103709
$ cmp /tmp/x86-add-p25519.txt .bench/dropin-gen/answers/bignum_add_p25519.answer.txt && echo IDENTICAL
IDENTICAL
```

Rust results:

```
$ diff -r /tmp/rust-answers .bench/dropin-gen/answers
x86-diff-status=0
$ ls /tmp/rust-answers | wc -l
9
$ diff -r /tmp/rust-answers-arm .bench/dropin-gen/answers-arm
arm-diff-status=0
$ ls /tmp/rust-answers-arm | wc -l
18
```

`diff -r` reports no differences, so all 27 files are byte-identical (9 x86,
18 ARM). Each emit printed the reference accumulate list; e.g.

```
wrote .../bignum_add_p25519.answer.txt mode=answer task=x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT entry=0 last=25 neg=false accumulate=[2, 4, 6, 8, 13, 14, 15, 16] bytes=103709
wrote .../bignum_add_p256.answer.txt mode=answer task=arm.bignum_add_p256.BIGNUM_ADD_P256_CORRECT accumulate=[3, 4, 7, 8, 10, 12, 13, 15, 16] bytes=103708
```

The same 27 files are re-checked in `tests/tools_emit.rs`, which skips when
`.bench` is absent (CI).

### IX.63 retirement of the byte-identity baseline (2026-11-24)

**The 27/27 byte-identity claim above is retired as of the IX.63 commit.** It was
true for the state recorded in this receipt, and it is not deleted or disputed:
it is the boundary marker for the migration property.

Why it stops holding: `tools/csat_lib.ml` is inlined verbatim into every emitted
answer, and IX.63 added the backward-closure LRAT trim inside `CSAT_REFUTE_GEN`
(`research/receipts/ix63-trimmed-replay.md`). The answer text therefore changes
for all 27 tasks even though no tactic and no goal changed. `tests/tools_emit.rs`
still checks emitter-to-reference-answer byte equality, but the reference answers
in `.bench/dropin-gen/answers{,-arm}` were regenerated with the new library text;
the Python-era digests are the retired baseline, not a current assertion.

**New invariant (as of IX.63):** the theorems are unchanged — a sample
(`arm/curve25519/add_p25519`, `arm/p521/add_p521`, `x86/sm2/neg_sm2`) is
`equals_thm` to the untrimmed theorem, and all 27 are accepted with
`check_axioms` passing — and only the library text and the certificate handling
changed. The four fixed-tactic digests in `tests/tools_receipt.rs` were repinned
to the post-IX.63 text, with the old values retained in a comment as the boundary.

---

## `--mode dev`

The ARM dev harness was compared against the Python output and is
byte-identical; the spent proof-read path refuses for the same reason:

```
$ env -C /tmp/tek-freeze python3 tools/emit_ref_proof.py --develop --task arm.bignum_add_p256.BIGNUM_ADD_P256_CORRECT --mode dev --out /tmp/py-arm-dev.ml
$ env -C /tmp/tek-freeze ./target/debug/emit --develop --task arm.bignum_add_p256.BIGNUM_ADD_P256_CORRECT --mode dev --out /tmp/rust-arm-dev.ml
$ cmp /tmp/py-arm-dev.ml /tmp/rust-arm-dev.ml && echo DEV-IDENTICAL
DEV-IDENTICAL

$ ./target/debug/emit --spent --spent-manifest research/receipts/spent-r4-arm.json --task arm.bignum_add_p256.BIGNUM_ADD_P256_CORRECT --mode dev --out /tmp/rust-arm-dev-spent.ml
refused: spent proof file is never readable: .../arm/proofs/bignum_add_p256.ml ('arm.bignum_add_p256.BIGNUM_ADD_P256_CORRECT')
rust-spent-dev-rc=1
$ env -C /tmp/tek-freeze python3 tools/emit_ref_proof.py --spent --spent-manifest ... --task arm.bignum_add_p256.BIGNUM_ADD_P256_CORRECT --mode dev --out /tmp/py-arm-dev-spent.ml
refused: spent proof file is never readable: .../arm/proofs/bignum_add_p256.ml ('arm.bignum_add_p256.BIGNUM_ADD_P256_CORRECT')
py-spent-dev-rc=1
```

`tests/tools_emit.rs` asserts the same refusal for one ARM and one x86 task.

## Guard CLI

```
$ ./target/debug/guard --list
25519
secp256k1
p256k1
sm2
$ ./target/debug/guard .bench/s2n-bignum/arm/proofs/bignum_add_p256.ml
allowed: .../bignum_add_p256.ml sha256=8ee8cfd8e87b83da1eb253e563eeb6de717a0cd3d9313ca813d16b75eb059ea5 lines=122
develop-allow-rc=0
$ ./target/debug/guard .bench/s2n-bignum/arm/proofs/secp256k1_jadd.ml
REFUSED: held-out marker "secp256k1" in proof path .bench/s2n-bignum/arm/proofs/secp256k1_jadd.ml
develop-refuse-rc=2
```

## Guard tests

`tests/tools_guard.rs` covers: develop refuses held-out ids/objects/proofs;
`sealed-eval` admits exactly the nine declared ids and their objects; unlisted
held-out ids/objects are refused; every listed proof `.ml` is refused on both
the generic and the dev proof-read path; freeze mismatch, wrong commit and
moved `tools/` are refused; sealed and spent manifests reject symlinks,
empty/non-regular files, duplicate JSON keys, unknown keys, malformed ids,
bad commits, bad tags and missing reasons/decision; spent mode admits exactly
the 18 R4 ARM ids and refuses an x86 held-out id; and every refusal site from
`test_guard_refusals.py` is reached by the ported cases (grouped into eight
tests). 26 tests, all passing.

## Commands

```
$ cargo build --offline
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.11s

$ cargo clippy --offline --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.26s

$ cargo test --offline
  lib 3 passed; native 27; native_semantics 7; refusals 25; semantics 21;
  tools 13; tools_derive 25; tools_emit 5; tools_guard 26; 0 failed in total.

$ cargo run --offline --quiet --bin mutate_guard
guard refusal sites: 58 (0 invalid)
killed: 58  survived: 0
mutation score: 58/58 = 1.000
all survivors are recorded equivalent mutants

$ bash tools/selftest.sh
SELF-TEST PASSED
```

The guard mutation harness replaces each `Err(Refused::new(..))` with
`Ok(todo!())`. The mutant panics when reached, so a kill establishes that the
suite reaches the refusal and depends on it; it does not by itself pin the exact
message. (`?`-site refusals inside `ok_or_else`/`map_err` closures are not
mutated by the scanner.)

## CI

`python3 -m pytest tools/ -q` was removed together with the pytest install
step. After this slice the only remaining Python harness under `tools/` is
`test_write_through.py`, which is a standalone script (pytest collects zero
tests from it and exits 5); `tools/selftest.sh` already runs it directly. The
mutation-harness comment in CI now describes the Rust guard.

## Known follow-ups (outside this slice's write scope)

`tools/csat_close.py`, `tools/emit_proof.py`, `tools/w1_gate.py` and
`tools/w2_witnesses.py` still `import heldout_guard` / `derive_steps` /
`disasm_steps`, all now deleted. They are `.py` files outside the Slice B
delete list and were left untouched; they need their own port slice.
`research/receipts/ix-o9-guard-mutation.md` describes the old Python target and
is likewise stale; it is under `research/**` and was not edited.
