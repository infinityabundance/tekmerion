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

---

## Correction of record (2026-11-24, IX.60): the theorem is not upstream's theorem

This receipt says the generated proof was installed in a copy of upstream s2n-bignum and passed
the native build and `check_axioms()`. That remains literally true. What it must not be read as
is "the theorem upstream states is now proved by Tekmerion".

Measured in `research/receipts/ix60-matched-inference.md` §1, per task, by extracting both
conclusions and comparing them:

- upstream's human file proves `BIGNUM_<OP>_<FAM>_SUBROUTINE_CORRECT`;
- our drop-in file proves `BIGNUM_<OP>_<FAM>_CORRECT`, the obligation from the benchmark query;
- all **27 of 27** pairs differ. The human contract adds `read X30 s = returnaddress` (ARM) or
  the `RSP`/saved-return-address precondition (x86), asserts `PC/RIP = returnaddress` in the
  postcondition instead of `PC/RIP = word (pc + N)`, and permits
  `MAYCHANGE_REGS_AND_FLAGS_PERMITTED_BY_ABI` instead of a narrower explicit frame. Verified
  directly: `grep -c returnaddress` is 6 on the human `.correct` and 0 on ours, while ours
  contains `word (pc + 76)`.

Consequences that must travel with any citation of this receipt:

1. The two contracts are **incomparable**: the human one has a stronger postcondition, a
   weaker precondition and a broader frame. Neither implies the other.
2. Because the replacement file defines `_CORRECT` and no longer defines
   `_SUBROUTINE_CORRECT`, a **whole-tree** upstream `make` would fail on any file that depends
   on the subroutine theorem. The 27/27 result is a per-file build result, not whole-library
   compatibility (TODO `XIII.G3`).
3. "Upstream coverage" in the sense that matters to upstream — the same obligations upstream
   states — is therefore **0 of 27 today**, not 27 of 27. Proving the `_SUBROUTINE_CORRECT`
   obligation itself is TODO `XIV.G`.
4. No check-time and no inference-count *ratio* may be formed between the two, which is why
   IX.60 reports `n = 0 matched pairs`.

---

## Addendum (IX.64, 2026-11-24): the corrected position — the drop-in now proves upstream's name

`research/receipts/ix64-subroutine-obligation.md` settles consequence 2 and 3 above.

**Premise (G1a).** Our built `_CORRECT` conclusion is the *same statement* as upstream's
`_CORRECT` quotation for **27 of 27** tasks. `dropin premise` canonicalises hex numerals and
whitespace and reports `PREMISE total=27 same=27 different=0 missing=0`. The only
notational differences were `!` vs `forall` and `0x50`/`0x4c` vs `80`/`76`.

**Derivation.** Because the premise holds, the upstream one-line derivation can be applied
to our theorem. `assemble` now emits, **verbatim from the pinned
`<arch>/proofs/bignum_<op>_<fam>.ml`**, the `_SUBROUTINE_CORRECT` statement and its upstream
tactic (`ARM_ADD_RETURN_NOSTACK_TAC` on ARM; `X86_PROMOTE_RETURN_NOSTACK_TAC` plus
`ADD_IBT_RULE` on x86), after our unchanged `_CORRECT` proof.

**Result.** All **27 of 27** drop-in files now prove
`BIGNUM_<OP>_<FAM>_SUBROUTINE_CORRECT`, and `dropin obligation --generated-theorem
subroutine` reports `OBLIGATIONS total=27 same=27` against the human-built conclusions in
`.bench/human-baseline`. `check_axioms` passes throughout. The derived obligation costs
0.33–0.36 s CPU per ARM task and 0.39–0.42 s per x86 task on top of the already-measured
proof.

**What this repairs.** Consequence 3 is superseded: the obligations upstream states are now
proved for **27 of 27**, not 0 of 27. Consequence 2 is partly repaired: upstream's
`collect-specs.sh` finds all 27 names (18 ARM + 9 x86), and the directory default whole-tree
`make -j1` is green in both `arm/` and `x86/`. It is **not** fully repaired: we do not emit
the `_SUBROUTINE_SAFE` or `_WINDOWS` variants, and a full `make proofs` over the whole tree
was not run (452 ARM + 1758 x86 obligations). Consequence 1 is untouched: the contracts are
still different statements — we now prove the human one *in addition*.

This upgrades "per-file acceptance" to "upstream-compatible" for the correctness
obligation, and only for it. A new freeze (`v6`) is required before any sealed run citing
these files; none was created here.
