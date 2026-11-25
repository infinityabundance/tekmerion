# ix76 — the disassembler is chosen by capability; executable identity is digests

Branch `staging`. Files changed:

- `.github/workflows/ci.yml` — provision `binutils-aarch64-linux-gnu`.
- `src/tools/disasm.rs` — capability probe, `DisassemblerIdentity`.
- `src/tools/dropin.rs` — `ExecutableIdentity`, `RecordedIdentity`, JSON and comparison changes.
- `tests/fixtures/aarch64_add.o`, `tests/fixtures/x86_add.o`, `tests/fixtures/README.md` (new).
- `tests/tools_disasm.rs` (new).
- `TODO.md` — `XVII.A1`, `XVII.A2` ticked.

No file under `tools/` changed, so no fixture or golden was re-recorded
(`research/receipts/ix72-offline-inputs.md`'s addendum does not apply). No proof
build was run; the only heavy command was `cargo test`, capped at 20 G.

## Environment

```
$ free -g | head -2
               total        used        free      shared  buff/cache   available
Mem:             125          32          62           0          30          93

$ df -h /mnt/1tb_kingston | tail -1
/dev/nvme1n1p1  1.3T  901G  361G  72% /mnt/1tb_kingston
```

## XVII.A1 — capability-based disassembler selection

### The defect

`find_objdump()` returned the first executable found on `PATH`. On the CI
runner the sequence is `aarch64-linux-gnu-objdump` (absent),
`llvm-objdump` (absent), `objdump` (present, x86-only). The last one was
returned and then failed inside objdump:

```
objdump: can't disassemble for architecture UNKNOWN!
```

The name of a program was treated as evidence that it could do the job.

### Fixtures

Two committed, minimal, relocatable ELF objects. `.text` is one `add` (x86
also carries the `endbr64` prefix that `define_trimmed` requires), so the probe
is fast and its meaning is obvious.

| fixture | sha256 | bytes | disassembly |
|---|---|---:|---|
| `tests/fixtures/aarch64_add.o` | `d33232c3a489030e452cd4853747353c17c4b19747ed3684ddd1f8b02f2e96c5` | 408 | `0: add x0, x0, x1` |
| `tests/fixtures/x86_add.o` | `b6d943aaef58f66fc856539e1850fce048e9675abead332cc6ec5dd719bf60fe` | 376 | `0: endbr64` / `4: add rax, rbx` |

`readelf -h aarch64_add.o` reports `Class: ELF64`, `Type: REL`,
`Machine: AArch64`. Regeneration (also in `tests/fixtures/README.md`):

```sh
cd tests/fixtures
printf 'add x0, x0, x1\n'             | llvm-mc -triple=aarch64 -filetype=obj -o aarch64_add.o
printf 'endbr64\naddq %%rbx, %%rax\n' | llvm-mc -triple=x86_64  -filetype=obj -o x86_add.o
```

### Probe design

`select_disassembler(explicit, arch, search_path)`:

1. Resolves the architecture's fixture with `env!("CARGO_MANIFEST_DIR")`, so a
   different working directory cannot change what is probed. A missing fixture
   is an error, not a pass.
2. Tries candidates in order — ARM: `aarch64-linux-gnu-objdump`,
   `llvm-objdump`, `objdump`; x86: `objdump`, `llvm-objdump`,
   `x86_64-linux-gnu-objdump`. An explicit `--objdump PATH` replaces the list
   but is **still probed**, so naming a tool does not bypass the check.
3. For each candidate that exists, runs exactly the production argument vector
   (`-d --no-show-raw-insn`, plus `-M intel` for x86) on the fixture and parses
   the output with `parse_disassembly`. The candidate is accepted only if the
   output decodes the mnemonic `add` **and** the process exited 0. A program
   that exits 0 printing prose, or an x86-only objdump printing its
   “can't disassemble” diagnostic, is skipped.
4. If no candidate passes, returns an error that names every candidate, what
   each produced, the install step, and the `--objdump` override.

`disassemble()` now calls the same selector, so a tool that passes the probe is
invoked the same way on the real object. `disassembler_for(arch)` is the public
entry point; `disassembler_for_with(arch, explicit, search_path)` exists so tests
can inject a search path without mutating the process environment.

### Actual probe (this host)

```
$ F=tests/fixtures/aarch64_add.o
$ for t in aarch64-linux-gnu-objdump llvm-objdump objdump; do ...; done
== aarch64-linux-gnu-objdump
not found on PATH
== llvm-objdump

tests/fixtures/aarch64_add.o:	file format elf64-littleaarch64

Disassembly of section .text:

0000000000000000 <.text>:
       0:      	add	x0, x0, x1
rc=0
== objdump

tests/fixtures/aarch64_add.o:     file format elf64-little

objdump: can't disassemble for architecture UNKNOWN!

rc=1
```

Chosen: `/usr/bin/llvm-objdump`. The x86 probe:

```
== objdump
tests/fixtures/x86_add.o:     file format elf64-x86-64
   0:	endbr64
   4:	add    rax,rbx
rc=0
== llvm-objdump
tests/fixtures/x86_add.o:	file format elf64-x86-64
       0:      	endbr64
       4:      	add	rax, rbx
rc=0
== x86_64-linux-gnu-objdump
not found on PATH
```

Chosen: `/usr/bin/objdump`. From the tests:

```
chosen x86 disassembler: /usr/bin/objdump
chosen arm disassembler: /usr/bin/llvm-objdump
```

### Where the tool identity is recorded

In the producer's **run-record JSON**, under `environment.disassembler.<arch>`
(written by `results_json` and `measure_json` in `src/tools/dropin.rs`). That is
the same record that carries XVII.A2's identity; `config.rs` records
`TEKMERION_*` variables and is not an environment-of-binaries record, and the
freeze runtime block is out of this task's writable set. The block is written on
every sweep and measure, including when no tool works (then it carries
`{"error": ...}` rather than disappearing):

```json
"environment": {
  "disassembler": {
    "arm": {
      "name": "llvm-objdump",
      "path": "/usr/bin/llvm-objdump",
      "version": "LLVM (http://llvm.org/):",
      "sha256": "11afb034446361b589601553052e85a1405c43721dd287598f82c571ef1d01ae",
      "probe": "/mnt/1tb_kingston/Tekmerion/tests/fixtures/aarch64_add.o",
      "probe_mnemonic": "add"
    },
    "x86": {
      "name": "objdump",
      "path": "/usr/bin/objdump",
      "version": "GNU objdump (GNU Binutils) 2.47",
      "sha256": "0305569dc1b3f1dbebdd0d3253082301fd15014148fa74ad2cd43a3c2da8cd2e",
      "probe": "/mnt/1tb_kingston/Tekmerion/tests/fixtures/x86_add.o",
      "probe_mnemonic": "add"
    }
  }
}
```

(`llvm-objdump --version`'s second line is `LLVM version 23.1.5`; the record
keeps the first non-empty line, as it does for GNU objdump. The digest is over
the executable file's bytes, computed with `sha256::hex`, not by shelling out.)

`json_writer_shape_on_two_results` asserts the block exists and that each arch
record is either an `error` or carries a 64-character digest and
`probe_mnemonic: "add"`. It does not require a particular tool name, so the
assertion is honest on a runner that provisions a different working tool.

### CI change

`.github/workflows/ci.yml`, after the OCaml setup and before `cargo fetch`:

```yaml
      # The ARM tests disassemble AArch64 objects, and the runner's GNU objdump
      # is x86-only: without this package they fail inside objdump with
      # "can't disassemble for architecture UNKNOWN!". This provisions the
      # tool; src/tools/disasm.rs still *probes* it on the committed fixture
      # `tests/fixtures/aarch64_add.o` before selecting it, so a package that
      # does not work is refused rather than trusted. No test is skipped.
      - name: Install AArch64 binutils (disassembler for the ARM tests)
        run: |
          sudo apt-get update
          sudo apt-get install -y --no-install-recommends binutils-aarch64-linux-gnu
          aarch64-linux-gnu-objdump --version | head -1
```

No `#[ignore]`, no skip, no `continue-on-error` was added. The workflow's header
comment now says there are two provisioning steps that use the network
(`ocaml-nox`, `binutils-aarch64-linux-gnu`) plus `cargo fetch`; every check still
runs `--offline`.

## XVII.A2 — executable identity is three digests

### The type

```rust
pub struct ExecutableIdentity {
    pub architecture: String,      // "arm" | "x86"
    pub object_sha256: String,     // the .o the proof is against
    pub code_sha256: String,       // the .text bytes the proof's mc list encodes
    pub semantics_sha256: String,  // the ISA model file the proof is elaborated in
}
```

- `object_sha256` is `sha256::hex(fs::read(<dropin>/<arch>/<dir>/bignum_<op>_<fam>.o))`.
- `code_sha256` is the digest of the bytes returned by the existing
  `copy_text()` path (`objcopy -O binary --only-section=.text`; `llvm-objcopy`
  for ARM) — the same bytes `mclist()` turns into the `mc` constant. One
  extraction path, so the digest cannot drift from what the proof asserts.
- `semantics_sha256` is the digest of `<dropin>/<arch>/proofs/base.ml`. That is
  the file the emitted proof loads with `needs "<arch>/proofs/base.ml"` and the
  one in which `ARM_MK_EXEC_RULE` / `X86_MK_CORE_EXEC_RULE` and the instruction
  semantics are defined. It is the model the proof is elaborated in, not merely
  a neighbouring file.

The producer computes this in `executable_identity()` and writes it to
`tasks[].identity` in the sweep/measure JSON. If any file is missing the
producer prints the reason and writes `"identity": null`; it never substitutes a
label. Reading is per-field: `RecordedIdentity` holds four `Option<String>`, so
a record from before this change has all four absent and is diagnostic.

### The comparison

- `IdentityBoundComparison` requires both sides to record all four fields, and
  requires them equal. `bind_identity` refuses otherwise.
- `DiagnosticComparison` may be built on weaker evidence but records exactly
  which digest is missing on which side (`IdentityEvidence::missing_object_digest`,
  `missing_code_digest`, `missing_semantics_digest`,
  `missing_identity_fields`), and `diagnose` refuses a **present** mismatched
  digest (`ComparisonError::ExecutableDigestMismatch`).
- An old record without the fields therefore degrades to diagnostic with a
  printed reason, never silently to bound:

```
statement digest missing on human, generated; executable identity missing on
human, generated; object digest missing on human, generated; code digest
missing on human, generated; semantics digest missing on human, generated;
executable architecture missing on human, generated
```

The label (`arch.dir.op`) is still carried and compared for display and for old
records, but it is no longer sufficient and no longer the identity.

### Worked example — one ARM and one x86 task

Task `bignum_add_p25519` in `.bench/dropin` (the producer tree the emitted proof
is built in; read-only for this task).

**ARM**

```
$ obj=.bench/dropin/arm/curve25519/bignum_add_p25519.o
$ llvm-objcopy -O binary --only-section=.text "$obj" /tmp/ix76-arm-text.bin
$ sha256sum "$obj" /tmp/ix76-arm-text.bin .bench/dropin/arm/proofs/base.ml
95465f86abb3d61ecb9d3589f64b851697859ec85e7359144a17325315973424  .bench/dropin/arm/curve25519/bignum_add_p25519.o
1e5d225413ccd829ab0e99d3337a35da3717b084f109fa1cad70db645550ec6f  /tmp/ix76-arm-text.bin
4e08e32c6aa3afe27c89a500a9ef5b0d4e357d5d44ad525c9a82abaebaa0a5d0  .bench/dropin/arm/proofs/base.ml
$ stat -c%s /tmp/ix76-arm-text.bin
80
```

**x86**

```
$ obj=.bench/dropin/x86/curve25519/bignum_add_p25519.o
$ objcopy -O binary --only-section=.text "$obj" /tmp/ix76-x86-text.bin
$ sha256sum "$obj" /tmp/ix76-x86-text.bin .bench/dropin/x86/proofs/base.ml
a23838856b2e761c1ac93189d50d2f5a06865d09ed78e1e1841dbe683ad753f7  .bench/dropin/x86/curve25519/bignum_add_p25519.o
08965e677e96a52ddd8818955a4b88b790977d1742a0c5bbd615fb633423a70d  /tmp/ix76-x86-text.bin
0883b22b2e92e4a72e03eb8203a2e4e78616e41e91df9b38d2938e324a1e9527  .bench/dropin/x86/proofs/base.ml
$ stat -c%s /tmp/ix76-x86-text.bin
94
```

The Rust producer computes the same values. From
`a_real_dropin_pair_produces_an_identity_bound_pair` (`--nocapture`):

```
identity arm: ExecutableIdentity { architecture: "arm", object_sha256: "95465f86abb3d61ecb9d3589f64b851697859ec85e7359144a17325315973424", code_sha256: "1e5d225413ccd829ab0e99d3337a35da3717b084f109fa1cad70db645550ec6f", semantics_sha256: "4e08e32c6aa3afe27c89a500a9ef5b0d4e357d5d44ad525c9a82abaebaa0a5d0" }
identity x86: ExecutableIdentity { architecture: "x86", object_sha256: "a23838856b2e761c1ac93189d50d2f5a06865d09ed78e1e1841dbe683ad753f7", code_sha256: "08965e677e96a52ddd8818955a4b88b790977d1742a0c5bbd615fb633423a70d", semantics_sha256: "0883b22b2e92e4a72e03eb8203a2e4e78616e41e91df9b38d2938e324a1e9527" }
test result: ok. 1 passed
```

To recompute any row: hash the `.o`, extract `.text` with the same `objcopy`
invocation the producer uses and hash that, then hash the architecture's
`proofs/base.ml`. All three are plain SHA-256 over file bytes; the Rust code
uses `src/tools/sha256.rs` and does not shell out for hashing.

### What the identity binds, and what it still does not

Binds:

- the exact object file's bytes (`object_sha256`);
- the exact code bytes the theorem's `mc` list encodes (`code_sha256`), through
  the same extraction path;
- the semantics file the proof is elaborated against (`semantics_sha256`);
- equality of all of the above on both sides of an identity-bound pair.

Does **not** bind:

- the assembler/linker/binutils version or flags that produced the `.o`. Two
  runs of different toolchains that emit identical bytes are the same identity
  — which is intended, since the proof is about the bytes — and two runs that
  emit different bytes are different identities even if semantically equal;
- sections other than `.text` as proof objects (they are inside `object_sha256`
  but the proof only claims about `.text`);
- files that `base.ml` itself `needs` (the digest covers `base.ml`, not its
  transitive closure);
- the proof script / answer text (that is `statement_digest`, a printed-form
  digest until XVII.A6) and the HOL Light kernel build / s2n-bignum pin (carried
  in `MeasurementScope`, compared for equality, not part of this identity);
- the disassembler used to derive step lists; that is recorded separately in
  `environment.disassembler`, with its own digest.

## Tests

New (`tests/tools_disasm.rs`):

```
test selection_picks_a_working_arm_tool_and_decodes_the_fixture ... ok
test selection_picks_a_working_x86_tool_and_decodes_the_fixture ... ok
test an_unusable_shim_earlier_on_path_is_skipped ... ok
test an_unusable_explicit_tool_is_refused ... ok
test with_no_usable_tool_the_error_names_every_candidate_and_the_install_step ... ok
test the_probe_fixtures_are_committed_objects ... ok
test result: ok. 6 passed; 0 failed
```

The shim test puts an executable `aarch64-linux-gnu-objdump` first on the search
path that exits 0 printing `this is not a disassembler`; selection skips it and
picks a tool that decodes the fixture. The no-tool test asserts the error names
all three candidates, says `not found on PATH`, names
`binutils-aarch64-linux-gnu`, and mentions `--objdump PATH`.

A2 (`src/tools/dropin.rs` unit tests):

```
test tools::dropin::tests::the_same_tuple_with_a_different_code_digest_is_refused ... ok
test tools::dropin::tests::a_missing_semantics_digest_forces_diagnostic ... ok
test tools::dropin::tests::a_real_dropin_pair_produces_an_identity_bound_pair ... ok
```

- same tuple, different `code_sha256`: `diagnose` and `bind_identity` both refuse
  with `ExecutableDigestMismatch { field: "code_sha256", ... }`; equalising the
  digest restores a bound pair.
- missing `semantics_sha256`: `diagnose` succeeds, evidence records
  `["generated"]`, and `bind_identity` refuses with
  `ExecutableDigestUnrecorded { side: "generated", field: "semantics_sha256" }`.
  A record with no identity at all reports every field missing on the generated
  side and refuses first on `architecture`.
- real pair: the producer digests for ARM and x86 tasks in `.bench/dropin`
  (above) bind; a different `code_sha256` on the same tuple refuses.

ARM parity with the real tool (`.bench/s2n-bignum` present, no skip printed):

```
test real_object_derivation_matches_the_reference_lists ... ok
test x86_real_accumulate_sets ... ok
```

## Commands and results

```
$ tools/cap.sh 20G -- cargo test --offline
running 120 tests  ... test result: ok. 120 passed; 0 failed
(all integration binaries: ok; doc-tests: ok. 3 passed)
cap.sh: rc=0

$ cargo clippy --offline --all-targets -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.39s   (rc 0)

$ cargo fmt --check
(no output, rc 0)

$ git ls-files '*.py'
(no output)

$ git status --short
 M .github/workflows/ci.yml
 M TODO.md
 M src/tools/disasm.rs
 M src/tools/dropin.rs
?? research/receipts/ix76-disassembler-and-identity.md
?? tests/fixtures/README.md
?? tests/tools_disasm.rs
```

The two `.o` fixtures do not appear because the repository's global `*.o`
ignore matches them; they are nevertheless tracked, added deliberately with
`git add -f tests/fixtures/aarch64_add.o tests/fixtures/x86_add.o`. No
`.gitignore` rule was changed. A tracked file is unaffected by the ignore rule
after it is added; `git ls-files tests/fixtures/*.o` lists both.

Not verified here: the CI run itself (this host is not the runner). The step is
copied from the documented Ubuntu 22.04 package name
`binutils-aarch64-linux-gnu`; the local equivalent capability is exercised by
the probe tests with the real `llvm-objdump`.
