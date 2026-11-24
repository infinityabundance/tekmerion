# IX.54 — retiring `build_all.py` in favour of `dropin sweep`

- **Date:** 2026-11-24
- **Branch:** `staging` (Slice D)
- **Decision:** `research/receipts/ix54/build_all.py` is **retired**. The Rust drop-in
  tool gains a `sweep` subcommand that drives the same nine tasks through the existing
  `assemble` and `build` code paths. The Python file is deleted.

## 1. What `build_all.py` did

For each `op` in `add`, `sub`, `neg` and each family in `p25519`, `p256k1`, `sm2` (nine
tasks, in that order):

1. **Assemble.** Read the task's `query` from `.bench/problems.json` and the sealed
   emitter's answer from `.bench/dropin-gen/answers/bignum_<op>_<fam>.answer.txt`; run
   `objcopy -O binary --only-section=.text` on
   `.bench/dropin/x86/<dir>/bignum_<op>_<fam>.o`; emit the upstream proof-file shape
   (`needs`, `define_assert_from_elf`, `define_trimmed`, `X86_MK_CORE_EXEC_RULE`, the
   modulus definition, `let <THEOREM> = time prove (...)`) to
   `.bench/dropin/x86/proofs/bignum_<op>_<fam>.ml`. Print `wrote <name>`.
2. **Build.** Run, in `.bench/dropin/x86`,
   `make -j1 HOLDIR=<hol-light> <dir>/bignum_<op>_<fam>.correct`, tee-ing to
   `.bench/dropin-gen/logs/<dir>_<op>_<fam>.log`. Report
   `  <dir>/<op>_<fam>: exit=<e> theorem=<bool> exception=<bool>`.
3. **Summarise.** Print a `SUMMARY` block, one line per task, then `OK <n>/9`; exit 0 iff
   all nine pass.

`theorem` is true when `<dir>/bignum_<op>_<fam>.correct` exists and contains
`val BIGNUM_<OP>_<FAM>_CORRECT : thm`; `exception` is true when the log contains
`ANSWER_EXCEPTION` or `Fatal error`.

## 2. What supersedes it

| capability | Rust command |
|---|---|
| assemble one task | `cargo run --release --bin dropin -- assemble --task <id> --answer <file>` |
| build one task | `cargo run --release --bin dropin -- build --task <id>` |
| all nine, assemble + build + summary | `cargo run --release --bin dropin -- sweep --arch x86 [--dir <dropin>] [--gen <gen>] [--hol <hol-light>] [--dry-run]` |

`sweep` calls the same `assemble_core` and `make_status` helpers as `assemble` and
`build`; no logic is duplicated. It prints the per-task lines and `OK <n>/9` in the same
shape as `build_all.py`, and exits 0 iff all nine report `exit=0` and `theorem=True`.

`--dry-run` assembles (without writing the proof file) and prints the exact `make` command
that would run, then `DRY-RUN 9/9 assembled; make not invoked`. It exists so the driver can
be exercised without a nine-task HOL Light build.

## 3. Byte-identity evidence for the retired file

The tracked copy and the copy that produced the recorded IX.54 run are identical:

```
$ sha256sum research/receipts/ix54/build_all.py .bench/dropin-gen/build_all.py
f93e3ab0b1d530472802db814bc97acc470c487712bf765ac3b596ac8b0deca9  research/receipts/ix54/build_all.py
f93e3ab0b1d530472802db814bc97acc470c487712bf765ac3b596ac8b0deca9  .bench/dropin-gen/build_all.py
```

So the recorded IX.54 upstream run (`research/receipts/ix54-upstream-dropin.md`, nine
`OK`) was produced by exactly the algorithm now driven by `dropin sweep`.

## 4. What is therefore reproducible in Rust, and what is not

Reproducible: the nine-task **driver** (task order, answer/object/query lookup, upstream
proof-file shape, `make -j1 HOLDIR=…` invocation, per-task `exit/theorem/exception`
reporting, `OK n/9` exit status), using the already-ported `assemble`/`build` logic.

Not claimed:

- **The nine-task build was not run in this slice.** It is ~10 minutes of HOL Light per
  task and is explicitly out of scope. `sweep` is verified by `--help`, argument parsing,
  unit tests, and `--dry-run` over all nine tasks (which exercises `objcopy`, the
  `problems.json` lookup, the answer reads and the proof-file body construction for every
  task). The `make` step is unexercised here.
- `dropin sweep` is **not byte-identical to `build_all.py`'s generated `.ml`**. It reuses
  the existing Rust `assemble` body, whose header wording (`generated Tekmerion x86 proof`
  vs `generated Tekmerion proof`) and machine-code row width (four 8-bit words per line vs
  eight) differ from the Python scratch tool. The task instruction was to reuse the
  existing code paths rather than re-implement the Python formatting; the `.correct`
  target and the acceptance rule are unchanged.

## 5. Verification performed here

```
$ ./target/debug/dropin sweep --arch x86 --dry-run
wrote bignum_add_p25519.ml (dry run)
  curve25519/add_p25519: would run make -j1 HOLDIR=/mnt/1tb_kingston/Tekmerion/.bench/hol-light curve25519/bignum_add_p25519.correct
... (nine tasks) ...
DRY-RUN 9/9 assembled; make not invoked
$ ./target/debug/dropin sweep --arch x86 --dry-run ; echo $?
0
```

`cargo test --offline` covers argument parsing (`tests/tools_receipt.rs` and the unit tests
in `src/tools/dropin.rs`). The existing `assemble`/`build` CLI behaviour is unchanged.
