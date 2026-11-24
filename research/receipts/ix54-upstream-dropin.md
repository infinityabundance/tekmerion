# IX.54 — upstream drop-in build, all nine sealed x86 tasks

- **Date:** 2026-11-24
- **Branch:** `staging`
- **Tasks:** the nine sealed x86 `add`/`sub`/`neg` tasks on p25519, p256k1, sm2
- **Verdict:** all nine generated proofs **build and run inside upstream s2n-bignum, and
  `check_axioms ()` passes** (only `INFINITY_AX`, `SELECT_AX`, `ETA_AX`).

This is a development re-run of the same nine tasks under a stronger acceptance test. It is
**not** a new held-out split and not a coverage claim beyond these nine.

## 1. Why this matters

The IX.53/IX.54 bench receipts carry the caveat that `OK` is under the bench's inherited
inlined context with 2,462 pre-loaded axioms. The upstream build runs

```
loadt "<proof>.ml";;
check_axioms ();;
```

and `check_axioms` (`.bench/hol-light/hol_lib.ml:109`) fails if `axioms()` contains anything
other than the three basic axioms. An exit-0 run is therefore **axiom-closed** for that task.

Measured correction to the old description: the 2,462 count is a property of the bench's
inlined template, not of upstream `base.ml`. Upstream `check_axioms` enforces three.

## 2. Method

- `.bench/s2n-bignum` (129 MB) copied to `.bench/dropin/`; the original was not modified.
- For each task, `.bench/dropin-gen/build_all.py` writes
  `.bench/dropin/x86/proofs/bignum_<op>_<family>.ml` from:
  * the upstream file shape (`needs "x86/proofs/base.ml"`, `define_assert_from_elf` over the
    task's own `.o` `.text` bytes, `define_trimmed`, `X86_MK_CORE_EXEC_RULE`, the modulus
    definition, a named `_CORRECT` theorem), with the shape learned from the **non-sealed**
    `x86/proofs/bignum_add_p384.ml`;
  * the **sealed-eval emitter's answer verbatim** (`--mode answer`, which reads no proof file).
- The nine sealed human proofs were **replaced without being opened**; none was read,
  grepped or `cat`-ed.
- Moduli are the published curve parameters: `2^255-19`, `2^256-2^32-977`,
  `2^256-2^224-2^96+2^64-1`. A wrong modulus is rejected (see §4).
- Commands, per task, from `.bench/dropin/x86`:
  ```
  make -j1 HOLDIR=/mnt/1tb_kingston/Tekmerion/.bench/hol-light \
       <dir>/bignum_<op>_<family>.correct
  ```
  (the `.correct` target depends on `.native`, which runs `build-proof.sh` →
  `loadt` + `check_axioms`). `cadical` 2.1.2 was on `PATH`.

## 3. Result

| op | p25519 | p256k1 | sm2 |
|---|---|---|---|
| add | **OK** (70 s) | **OK** (71 s) | **OK** (66 s) |
| sub | **OK** (59 s) | **OK** (65 s) | **OK** (57 s) |
| neg | **OK** (51 s) | **OK** (51 s) | **OK** (50 s) |

Times are the `.correct` `Running time` values. Each `.correct` contains exactly one
`val BIGNUM_<OP>_<FAM>_CORRECT : thm = …` line, with the expected pre/postcondition
(e.g. `bignum_from_memory (z,4) s = (m + n) MOD p_25519`), and no `Fatal error`,
`Exception` or `check_axioms` failure.

`git status -- tools` is empty. No tracked file other than this receipt and the preserved
scripts below was written.

## 4. Firing control (the build must actually check the proof)

For `add_p25519`, changing the modulus by 1 (`2^255 − 19` → `2^255 − 20`) and re-running
the same target gives:

```
Fatal error: exception Failure("CSAT: cadical did not report UNSAT, exit 10")
make: *** [Makefile:501: curve25519/bignum_add_p25519.correct] Error 2
```

Exit 2. Restoring the original re-runs to exit 0. The build does not accept the file blindly.

## 5. What this establishes, and what it does not

**Establishes:** for all nine sealed x86 tasks, a generated Tekmerion proof is upstream-grade
in the upstream tree at its pinned commit: it loads, proves the stated theorem, and adds no
axiom beyond the three basic ones.

**Does not establish:**

- a whole-tree upstream build (only these nine targets were built; every other proof in the
  copy is the upstream human proof);
- anything about ARM, P-384/P-521 x86, or routines outside these nine;
- constant-time or memory-safety properties (functional correctness only);
- that the generated `_CORRECT` theorem is *usable* by later proofs in upstream (no
  downstream consumer was built);
- a new held-out result. The nine are the same IX.53/IX.54 tasks; this run changes the
  **acceptance test**, not the split.

## 6. Preserved artefacts

Tracked under `research/receipts/ix54/`: `build_all.py` (the generator/builder) and
`dropin-add_p25519.log` (one build log). The full tree, all nine `.ml`/`.correct`/logs are
under git-ignored `.bench/`. To reproduce, copy `.bench/s2n-bignum` to `.bench/dropin`, then
run `build_all.py`.

---

## Addendum (Slice D): retired in favour of `dropin sweep`

Ported to Rust in Slice D on `staging` (base commit `b28016c`).
`research/receipts/ix54/build_all.py` is **retired, not ported as a byte-identical copy**.
Its two building blocks already existed in Rust (`dropin assemble`, `dropin build`); the
driver loop is now
`cargo run --release --bin dropin -- sweep --arch x86 [--dir D] [--gen G] [--hol H]`,
which assembles then builds the same nine tasks and prints the same per-task lines and
`OK n/9` summary. The tracked file and the copy that produced the recorded run
(`.bench/dropin-gen/build_all.py`) were byte-identical, sha256
`f93e3ab0b1d530472802db814bc97acc470c487712bf765ac3b596ac8b0deca9`.
The nine-task build is **unexercised** in the Rust path in this slice; see
`research/receipts/ix54-dropin-retire.md`. Historical text above is untouched.

---

## Addendum (IX.56): the full 27-task upstream pass

`research/receipts/ix56-upstream-coverage.md` extends the nine x86 tasks above to the whole
set of generated proofs in **one forced pass** (`make -B`, one core, one tree at `5e0fc782`,
HOL Light `d30e823`): **27/27** — 18 ARM + 9 x86 — built with the `_CORRECT` theorem present
and `check_axioms ()` passing. The new count is 27, not 9. The nine x86 numbers in §3 were
re-measured in that run and are superseded by the fresh values there (e.g. `add_p25519`
70 s). Two caveats carried by the new receipt: `arm/p521/add_p521` takes 122 s against a
120 s budget, and the ARM answers are spent development data, not a sealed result.
Nothing in the sections above is rewritten.

---

## Addendum (IX.57): the per-theorem axiom stamp now exists

`check_axioms ()` above is a **session** predicate: it can only say "no axiom other
than the three basic ones exists in this session". `research/receipts/ix57-axiom-stamp.md`
now records, for each of the same 27 theorems (18 ARM + 9 x86), the **set of
`new_axiom` results its own derivation touches**: all 27 use exactly
`{ETA_AX, SELECT_AX, INFINITY_AX}`, measured by an axiom-taint bitset carried in a
copy of the kernel (`tools/hol-light/axiom-stamp.diff`), not inferred from
`axioms()`. That run is 27/27 with `check_axioms` passing and the same task set as
IX.56; it also records a primitive-rule invocation count per theorem
(18,230,539–681,707,202) and per-theorem check time.

It does **not** make the theorems true relative to those axioms, does not cover
definitional dependencies, and does not export a derivation: proof-object export,
HOLTrace-style external re-checking and Dedukti export remain open. The three
sections above are untouched.
