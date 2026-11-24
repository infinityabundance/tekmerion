# IX.54 — preserved artefacts for the structural-isolation run

These are copies of the exact scripts and host-side log from the run recorded in
`research/receipts/ix54-sealed-container.md`. They are tracked so the run can be
reviewed and reproduced without access to `.bench/`.

| file | what it is |
|---|---|
| `build-sealed-court-v3.py` | **Ported to Rust and deleted in Slice D.** The builder is now `src/bin/sealed_court.rs` (`cargo run --release --bin sealed_court`); it reproduces the Python tree byte for byte (`diff -r --no-dereference` clean). The row is kept for the historical record. |
| `run-sealed-court-v3.sh` | the host launcher: the exact `docker run` line (`--network none`, `--read-only`, tmpfs `/work`, read-only input mounts). |
| `run-inside.sh` | what runs inside the container: emit → combine → judge → collect, then the O10.2 control. The emitter is now the prebuilt Rust `emit_x86` binary selected by `EMIT_X86`; the next sealed run must mount it read-only into the container. |
| `container-run.log` | verbatim host-side output of the **valid** run: 9 × `syntactic check PASSED`, 9 × `PROFILE … OK`, `SUMMARY / OK,9`, `docker_exit=0`. |

## Limitations, stated plainly

1. **The judge directory (`eval-20261124_043844`) lived in the container's tmpfs**
   and was not preserved. `container-run.log` is the host-side record of the
   judge's own `PROFILE` and verdict lines; the individual `*.judge.txt` files
   were not copied out. A future run should bind-mount the eval directory.
2. **The container borrowed host userland** (`/usr`, `/lib64`, `/etc`,
   read-only) because the OCaml switch needs glibc 2.44 and the local images
   have ≤ 2.41. This does not expose the benchmark proofs (they are not
   mounted), but it means the container is not a self-contained userland.
3. **The benchmark templates are required.** The judge reads
   `ml_files/x86/*.ml` up to the insertion line only; the copies in the court
   are truncated at that line and verified byte-identical prefixes with
   `cmp -n`. The human proof body sits *after* the insertion line and is absent.
4. `OK` is under the inherited inlined context (`axioms_before = axioms_after`);
   it is not an axiom-free result.

## Reproduce

```
cargo run --release --bin sealed_court   # rebuilds .bench/sealed-court-v3
bash .bench/run-sealed-court-v3.sh full  # runs the container
```

The builder used to be `build-sealed-court-v3.py`; it is deleted (Slice D) and the Rust
binary now writes the same tree. `run-sealed-court-v3.sh` and `run-inside.sh` are still in
this directory; copy them to `.bench/` before use, since they write there.
