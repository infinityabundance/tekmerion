# IX.64 — The `_SUBROUTINE_CORRECT` obligation, derived

**Date:** 2026-11-24
**Pins:** s2n-bignum `5e0fc782ae6d587c2ae40faf2c2c4d0b637e6240` (`.bench/s2n-bignum`, read-only),
HOL Light `d30e8239887b2348a063f82232a864de1cc2df15` (`.bench/hol-light`).
**Kernel for the identity check:** the same unmodified HOL Light build; `check_axioms ()`
is run by upstream's own `tools/build-proof.sh` after `loadt`, so a green `make` means it
passed.

## 1. The question

IX.60 established that our generated files prove `BIGNUM_<OP>_<FAM>_CORRECT`, while
upstream's human files also prove `BIGNUM_<OP>_<FAM>_SUBROUTINE_CORRECT`, and that the two
contracts are **incomparable**. That left "27/27 upstream acceptance" as a per-file build
result, not "the theorems upstream states are now proved" (TODO `XIV.G4`, `XIII.G3`).

Upstream obtains `_SUBROUTINE_CORRECT` from `_CORRECT` with one library tactic. If our
`_CORRECT` is the *same statement* as upstream's `_CORRECT`, the same one-line derivation
gives the real upstream obligation at no extra proof cost.

## 2. G1a — the premise, verified mechanically

New subcommand `dropin premise` extracts

- upstream's `_CORRECT` quotation from `.bench/s2n-bignum/<arch>/proofs/<base>.ml`, and
- our **built** `_CORRECT` conclusion from the `val … : thm = |- …` line in
  `.bench/dropin/<arch>/<dir>/<base>.correct`,

canonicalises both (hex numerals → decimal; `!`/`?` → `forall`/`exists`; whitespace and
spacing around punctuation erased; tokens otherwise compared exactly), and compares.

```
$ target/debug/dropin premise \
    --upstream-tree .bench/s2n-bignum --generated-tree .bench/dropin \
    --json .bench/ix64-premise.json
PREMISE total=27 same=27 different=0 missing=0
```

**Verdict: the premise holds for all 27.** Our built `_CORRECT` conclusion is the same
statement as upstream's `_CORRECT` quotation, after notation canonicalisation. The
canonical differences that had to be erased are exactly two: HOL's printer spells the
quantifier `forall` where the source quotation writes `!`, and prints `80`/`76` where the
source writes `0x50`/`0x4c`. No structural difference remains.

This is the statement-level claim only. It does not say the proofs are the same; the
generated proof is Tekmerion's, upstream's is human.

## 3. What was changed

`src/tools/dropin.rs`, `assemble_core`:

- `upstream_subroutine_blocks` reads the pinned upstream `.ml` and extracts, **verbatim**,
  the `let BIGNUM_<OP>_<FAM>_SUBROUTINE_CORRECT = time prove … ;;` block. On x86 it also
  extracts the `_NOIBT_SUBROUTINE_CORRECT` block, because the IBT theorem is derived from
  it by `MATCH_ACCEPT_TAC(ADD_IBT_RULE …)`.
- The emitted file appends those blocks after our unchanged `_CORRECT` proof and prints the
  derived conclusion. The derivation is upstream's own line:
  - ARM: `ARM_ADD_RETURN_NOSTACK_TAC <EXEC> <CORRECT>` (all 18 tasks);
  - x86: `X86_PROMOTE_RETURN_NOSTACK_TAC <base>_tmc <CORRECT>` (NOIBT), then
    `MATCH_ACCEPT_TAC(ADD_IBT_RULE <NOIBT>)` (IBT) (all 9 tasks).
- The assembler refuses a block that does not carry one of those known upstream tactics.
- No new axiom, no new proof obligation: the derived theorem is a consequence of the
  `_CORRECT` theorem already proved. The trust story is unchanged.

The statement is not retyped and not derived from the benchmark: the bytes come from the
pinned human file, identified per task by `<arch>/proofs/bignum_<op>_<fam>.ml`.

## 4. One task end to end: `arm/curve25519/add_p25519`

Assembled, then built with upstream's own Makefile:

```
$ (cd .bench/dropin/arm)
$ env HOME=/home/one HOLDIR=/mnt/1tb_kingston/Tekmerion/.bench/hol-light \
    PATH=…/cadical:… timeout --kill-after=30 900 make -j1 -B \
    HOLDIR=/mnt/1tb_kingston/Tekmerion/.bench/hol-light \
    curve25519/bignum_add_p25519.correct > .bench/ix64-one-task.log 2>&1
exit=0
```

`.correct` now carries both names:

```
val BIGNUM_ADD_P25519_CORRECT : thm = |- forall z x y m n pc.
       …
       (MAYCHANGE [PC; X3; X4; X5; X6; X7; X8; X9; X10] ,,
        MAYCHANGE SOME_FLAGS ,, MAYCHANGE [events] ,,
        MAYCHANGE [memory :> bignum (z,4)])
…
val BIGNUM_ADD_P25519_SUBROUTINE_CORRECT : thm = |- forall z x y m n pc returnaddress.
       nonoverlapping (word pc,80) (z,8 * 4)
       ==> ensures arm (…read X30 s = returnaddress…)
           (\s. read PC s = returnaddress /\ …)
           (MAYCHANGE_REGS_AND_FLAGS_PERMITTED_BY_ABI ,,
            MAYCHANGE [memory :> bignum (z,4)])
Running time: 42.000000 sec, Start unixtime: 1795543160.000000, End unixtime: 1795543202.000000
```

Per-step CPU (HOL's own timing): `_CORRECT` **15.508162 s**, derived subroutine step
**0.340068 s**; whole-file `Running time` **42 s**. `check_axioms ()` passed (exit 0).

### G1b — identity with the human-built theorem

```
$ target/debug/dropin obligation \
    --human-tree .bench/human-baseline --generated-tree .bench/dropin \
    --generated-theorem subroutine
| arm | curve25519/add_p25519 | BIGNUM_ADD_P25519_SUBROUTINE_CORRECT | BIGNUM_ADD_P25519_SUBROUTINE_CORRECT | True | same statement |
```

The derived conclusion is the human-built conclusion, canonicalised: same statement.

### G1c — `check_axioms`

Passes. `tools/build-proof.sh` appends `check_axioms ();;` after `loadt`, so exit 0 is the
pass; the sweep reports `AXIOMS true` for the task.

### G1e — firing control (the step is not vacuous)

The same file with the upstream tactic replaced by `ALL_TAC`:

```
$ sed -i 's/^  ARM_ADD_RETURN_NOSTACK_TAC .*/  ALL_TAC);;/' …/bignum_add_p25519.ml
$ make -j1 -B … curve25519/bignum_add_p25519.correct
exit=2
Fatal error: exception Failure("TAC_PROOF: Unsolved goals")
make: *** [Makefile:490: curve25519/bignum_add_p25519.correct] Error 2
```

(`.bench/ix64-control.log` line 758.) Omitting the derivation leaves the goal unsolved, so
the one-line upstream tactic is doing the work. The good artefacts were restored from
`.bench/ix64-control/good.{ml,native,correct}` and re-assembled from the answer file.

## 5. All 27

```
$ target/debug/dropin sweep --arch all --force --timeout 900 --json .bench/ix64-sweep.json
…
OK 27/27
```

```
$ target/debug/dropin premise --upstream-tree .bench/s2n-bignum --generated-tree .bench/dropin
PREMISE total=27 same=27 different=0 missing=0

$ target/debug/dropin obligation --human-tree .bench/human-baseline \
    --generated-tree .bench/dropin --generated-theorem subroutine
OBLIGATIONS total=27 same=27 different=0 missing=0
```

Theorem names present (counted by `grep -l` over the 27 `.correct` files): 27 files carry
`val … _SUBROUTINE_CORRECT : thm`, 27 carry `val … _CORRECT : thm`, and the 9 x86 files
also carry `_NOIBT_SUBROUTINE_CORRECT`.

Per-task timings, from `.bench/ix64-times.tsv` (HOL `CPU time (user)` for each `time prove`,
and upstream's whole-file `Running time`):

| arch | task | `_CORRECT` CPU s | derived step CPU s | whole-file `Running time` s | exit / axioms |
|---|---|---|---|---|---|
| arm | curve25519/add_p25519 | 15.508162 | 0.340068 | 42 | 0 / true |
| arm | curve25519/sub_p25519 | 12.218988 | 0.334204 | 38 | 0 / true |
| arm | curve25519/neg_p25519 | 4.614316 | 0.335848 | 29 | 0 / true |
| arm | p256/add_p256 | 15.804233 | 0.343648 | 44 | 0 / true |
| arm | p256/sub_p256 | 11.610184 | 0.328836 | 38 | 0 / true |
| arm | p256/neg_p256 | 5.751342 | 0.333169 | 30 | 0 / true |
| arm | secp256k1/add_p256k1 | 15.212682 | 0.336326 | 42 | 0 / true |
| arm | secp256k1/sub_p256k1 | 12.100189 | 0.329921 | 38 | 0 / true |
| arm | secp256k1/neg_p256k1 | 5.836503 | 0.333742 | 31 | 0 / true |
| arm | p384/add_p384 | 34.005339 | 0.336221 | 65 | 0 / true |
| arm | p384/sub_p384 | 23.991722 | 0.352564 | 51 | 0 / true |
| arm | p384/neg_p384 | 11.774389 | 0.337722 | 37 | 0 / true |
| arm | p521/add_p521 | 66.931578 | 0.355615 | 104 | 0 / true |
| arm | p521/sub_p521 | 27.097830 | 0.356609 | 56 | 0 / true |
| arm | p521/neg_p521 | 18.316314 | 0.357577 | 44 | 0 / true |
| arm | sm2/add_sm2 | 15.049015 | 0.360905 | 42 | 0 / true |
| arm | sm2/sub_sm2 | 11.730956 | 0.336338 | 38 | 0 / true |
| arm | sm2/neg_sm2 | 5.765047 | 0.327708 | 30 | 0 / true |
| x86 | curve25519/add_p25519 | 15.139856 | 0.000358 (IBT) + 0.419106 (NOIBT) | 66 | 0 / true |
| x86 | curve25519/sub_p25519 | 6.861967 | 0.000343 + 0.390416 | 56 | 0 / true |
| x86 | curve25519/neg_p25519 | 2.499539 | 0.000234 + 0.420521 | 51 | 0 / true |
| x86 | secp256k1/add_p256k1 | 16.204397 | 0.000323 + 0.393418 | 68 | 0 / true |
| x86 | secp256k1/sub_p256k1 | 11.909242 | 0.000330 + 0.395514 | 61 | 0 / true |
| x86 | secp256k1/neg_p256k1 | 2.423298 | 0.000224 + 0.420520 | 52 | 0 / true |
| x86 | sm2/add_sm2 | 10.632550 | 0.000328 + 0.394230 | 63 | 0 / true |
| x86 | sm2/sub_sm2 | 6.961619 | 0.000327 + 0.396168 | 57 | 0 / true |
| x86 | sm2/neg_sm2 | 2.263112 | 0.000220 + 0.390881 | 51 | 0 / true |

Sums: ARM `_CORRECT` proofs 313.32 s CPU and 6.14 s of subroutine derivation; x86
`_CORRECT` 74.90 s CPU, 3.62 s NOIBT + 0.0027 s IBT. The derived obligation costs **0.3–0.4 s
per task** on top of the proof that was already there.

## 6. Whole-tree build (G1d)

The directory default target in both architectures is `libs2nbignum.a` (the first rule of
the generated Makefile; proofs are a separate `proofs` target). Run with `-j1`:

```
$ (cd .bench/dropin/arm) make -j1 HOLDIR=/mnt/1tb_kingston/Tekmerion/.bench/hol-light
exit=0
$ (cd .bench/dropin/x86) make -j1 HOLDIR=/mnt/1tb_kingston/Tekmerion/.bench/hol-light
exit=0
```

Both green (logs `.bench/ix64-make-arm.log`, `.bench/ix64-make-x86.log`). This is the
whole-tree assembly build; it does not evaluate proofs.

**What "dependants now find the name" means, concretely.** The real consumer of the names
is `tools/collect-specs.sh`, which greps the proof sources for
`let …_SUBROUTINE_CORRECT` and feeds the generated `build-proof.sh` template. Before this
change our 27 files contributed **0** names there. Now:

```
$ ./tools/collect-specs.sh arm | grep -c -E 'BIGNUM_(ADD|SUB|NEG)_(P25519|P256|P256K1|P384|P521|SM2)_SUBROUTINE_CORRECT'
18
$ ./tools/collect-specs.sh x86 | grep -c -E 'BIGNUM_(ADD|SUB|NEG)_(P25519|P256K1|SM2)_SUBROUTINE_CORRECT'
9
```

`tools/count-proofs.sh` — which diffs proven theorem names against
`proofs/specifications.txt` — now counts our names rather than reporting them missing:

```
$ ./tools/count-proofs.sh arm
Total # SUBROUTINE_CORRECT: 18/452
$ ./tools/count-proofs.sh x86
Total # SUBROUTINE_CORRECT: 18/1758
```

Both exit 1, honestly, because only the 27 drop-in `.correct` files exist: the remaining
ARM names include the seven `BIGNUM_*_SUBROUTINE_SAFE` theorems for our `p384`/`p521`/`neg`
tasks, and the x86 remaining names include the `_WINDOWS_SUBROUTINE_CORRECT` and `_SAFE`
variants for ours, plus every proof outside the drop-in set. **A full `make proofs` over the
whole tree was not run**: it is 452 ARM + 1758 x86 obligations and does not fit the
100-minute budget. The per-file `.correct` results for all 27 *are* the proof-level result.

## 7. What is now true, and what is not

Now true:

- For **27 of 27** tasks the drop-in file proves upstream's own theorem name
  `BIGNUM_<OP>_<FAM>_SUBROUTINE_CORRECT`, with the conclusion identical to the human-built
  one (canonicalised), and the derivation is upstream's own tactic line, copied verbatim
  from the pinned file.
- `dropin premise` shows our `_CORRECT` is the same statement as upstream's `_CORRECT` for
  **27 of 27**, so the derivation is applicable for the reason the premise predicted.
- The whole-tree assembly default target is green in `arm/` and `x86/`, and upstream's name
  collector finds all 27 names.

Not true / not attempted:

- We do **not** emit `_SUBROUTINE_SAFE`, `_NOIBT_WINDOWS_SUBROUTINE_CORRECT`,
  `_WINDOWS_SUBROUTINE_CORRECT`, or the safety obligations. `count-proofs.sh` still lists
  those for our tasks, so the whole-tree proof target would still be incomplete if only our
  27 files were present. The task scope was the correctness obligation and its derivation.
- The proof bodies are still Tekmerion's, not upstream's. "Upstream-compatible" here means
  same contract, same theorem name, same library derivation; it is not a claim of proof
  identity.
- A **freeze `v6`** is required before any sealed run that cites these files: the emitted
  proof shape and the assembler both changed. This receipt creates no freeze.

## 8. Reproduction

```
cd /mnt/1tb_kingston/Tekmerion
export HOME=/home/one
export HOLDIR=/mnt/1tb_kingston/Tekmerion/.bench/hol-light
export PATH="/mnt/1tb_kingston/Tekmerion/.bench/bin:/mnt/1tb_kingston/Tekmerion/.bench/hol-light/_opam/bin:/home/one/.elan/toolchains/leanprover--lean4---v4.29.1/bin:/home/one/.local/bin:/usr/local/bin:/usr/bin:/bin"

cargo build --offline --bin dropin
target/debug/dropin premise --upstream-tree .bench/s2n-bignum --generated-tree .bench/dropin \
    --json .bench/ix64-premise.json
target/debug/dropin sweep --arch all --force --timeout 900 --json .bench/ix64-sweep.json
target/debug/dropin obligation --human-tree .bench/human-baseline \
    --generated-tree .bench/dropin --generated-theorem subroutine \
    --json .bench/ix64-identity.json
(cd .bench/dropin/arm && make -j1 HOLDIR=/mnt/1tb_kingston/Tekmerion/.bench/hol-light)
(cd .bench/dropin/x86 && make -j1 HOLDIR=/mnt/1tb_kingston/Tekmerion/.bench/hol-light)
```

`cadical` is required by the answer files and lives at
`/home/one/.elan/toolchains/leanprover--lean4---v4.29.1/bin/cadical` (2.1.2). Without it the
proofs fail with `CSAT: cadical did not report UNSAT, exit 127`.

## 9. Artefacts

Under `.bench/` (git-ignored scratch; cited, not committed):

- `ix64-premise.json`, `ix64-premise.out` — G1a, 27/27 same.
- `ix64-sweep.json`, `ix64-sweep.out` — the 27-task run (`OK 27/27`).
- `ix64-identity.json`, `ix64-identity.out` — G1b, 27/27 same statement.
- `ix64-times.tsv`, `ix64-times.awk` — per-task CPU and whole-file times.
- `ix64-one-task.log` — the `arm/curve25519/add_p25519` build log.
- `ix64-control.log` — the G1e wrong-derivation control (exit 2, unsolved goals).
- `ix64-make-arm.log`, `ix64-make-x86.log` — whole-tree default-target builds.
- `ix64-count-arm.log`, `ix64-count-x86.log` — `count-proofs.sh` output.
- `.bench/dropin/**` — the assembled files and `.correct` logs.
- `.bench/ix64-control/good.{ml,native,correct}` — the restored good artefacts.

Committed: this receipt, the `src/tools/dropin.rs` change and its tests, addenda to IX.54
and IX.56, and the TODO corrections.
