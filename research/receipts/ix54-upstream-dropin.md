# IX.54 — upstream drop-in build (one task): `check_axioms` passes

- **Date:** 2026-11-24
- **Branch:** `staging`
- **Task:** `x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT`
- **Verdict:** the generated proof **builds and runs inside upstream s2n-bignum, and
  `check_axioms ()` passes** (only `INFINITY_AX`, `SELECT_AX`, `ETA_AX`).

This is one task, not a coverage claim.

## 1. Why this matters

The IX.53/IX.54 sealed receipts carry the caveat that `OK` is under the bench's
inherited inlined context with 2,462 pre-loaded axioms. The upstream build runs

```
loadt "<proof>.ml";;
check_axioms ();;
```

and `check_axioms` (`.bench/hol-light/hol_lib.ml:109`) fails if `axioms()` contains
anything other than the three basic ones. An exit-0 run is therefore an
**axiom-closed** result for this task.

Measured correction to the old description: the 2,462 count is a property of the
bench's inlined template, not of upstream `base.ml`. Loading `base.ml` yields the
three basic axioms, and `check_axioms` enforces that.

## 2. What was built

- Copied `.bench/s2n-bignum` (129 MB) to `.bench/dropin/`; the original was not modified.
- Assembled `.bench/dropin/x86/proofs/bignum_add_p25519.ml` (2,289 lines) from:
  the upstream file shape (`needs "x86/proofs/base.ml"`, `define_assert_from_elf`
  over the task's own `.o` bytes, `define_trimmed`, `X86_MK_CORE_EXEC_RULE`,
  `p_25519`, a named `_CORRECT` theorem), and the **sealed-eval emitter's answer
  verbatim** (`--mode answer`, which reads no proof file).
- The sealed human proof `x86/proofs/bignum_add_p25519.ml` was **replaced without
  being opened**; it was never read, grepped or `cat`-ed.

Digests:

```
3a8cd19562e0d1427c6f3c927c37e56ad088511534ba7aa430fbf6118e64a87c  dropin/x86/proofs/bignum_add_p25519.ml
f110ec0fdf1bceca2919e07b8430d04d36a4aac64fbddebf5ab51658cb79dad8  dropin-gen/answer.txt (2,226 lines)
```

## 3. Commands and result

```
cd .bench/dropin/x86
env PATH=<cadical-bin>:<normal PATH> make -j1 \
    HOLDIR=/mnt/1tb_kingston/Tekmerion/.bench/hol-light \
    curve25519/bignum_add_p25519.native
# -> exit 0  (build-proof.sh: loadt the drop-in, check_axioms ())

env PATH=<cadical-bin>:<normal PATH> make -j1 \
    HOLDIR=/mnt/1tb_kingston/Tekmerion/.bench/hol-light \
    curve25519/bignum_add_p25519.correct
# -> exit 0
```

`curve25519/bignum_add_p25519.correct` ends with the theorem and timing:

```
val BIGNUM_ADD_P25519_CORRECT : thm = |- forall z x y m n pc.
       nonoverlapping (word pc,90) (z,8 * 4)
       ==> ensures x86
           (\s. bytes_loaded s (word pc) (BUTLAST bignum_add_p25519_tmc) /\
                read RIP s = word pc /\
                C_ARGUMENTS [z; x; y] s /\
                bignum_from_memory (x,4) s = m /\
                bignum_from_memory (y,4) s = n)
           (\s. read RIP s = word (pc + 89) /\
                (m < p_25519 /\ n < p_25519
                 ==> bignum_from_memory (z,4) s = (m + n) MOD p_25519))
           (MAYCHANGE [RIP; RSI; RDX; RAX; RCX; R8; R9; R10; R11] ,,
            MAYCHANGE SOME_FLAGS ,,
            MAYCHANGE [events] ,,
            MAYCHANGE [memory :> bignum (z,4)])
Running time: 70.000000 sec
```

`check_axioms ()` appears before the theorem in `build-proof.sh`'s template, so a
non-zero exit would have aborted the run. The run exited 0.

## 4. Firing control (the build must actually check the proof)

Changed the modulus in the drop-in from `2^255 − 19` to `2^255 − 20` (one digit) and
re-ran the same target:

```
Fatal error: exception Failure("CSAT: cadical did not report UNSAT, exit 10")
make: *** [Makefile:501: curve25519/bignum_add_p25519.correct] Error 2
```

Exit 2. Restored the original file and re-ran: exit 0, theorem printed again. The
build is not accepting the file blindly.

## 5. What this establishes, and what it does not

**Establishes:** for `bignum_add_p25519` (x86), a generated Tekmerion proof is
upstream-grade: it loads in the upstream tree, proves the stated theorem, and adds
no axiom beyond the three basic ones.

**Does not establish:**

- anything about the other eight sealed tasks (one task only);
- a coverage or success-rate claim;
- that the other eight are drop-in compatible without their own wrappers;
- constant-time or memory-safety properties (only the functional-correctness theorem);
- that the upstream *whole-tree* build passes (only this one target was built; the
  rest of `.bench/dropin` has unmodified human proofs).

## 6. Files

Tracked: this receipt. Everything else is under git-ignored `.bench/`
(`dropin/`, `dropin-gen/`, `dropin-*.log`). `git status -- tools` is empty.
