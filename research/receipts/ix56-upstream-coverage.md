# IX.56 — one forced upstream pass over all 27 generated proofs

- **Date:** 2026-11-24 (run 08:56:23Z–09:43:17Z, `generated_at` 1795513397 = 2026-11-24T09:43:17Z)
- **Branch:** `staging`
- **Tree:** `.bench/dropin` = upstream s2n-bignum at `5e0fc782`; `.bench/hol-light` at `d30e823`
- **Cores:** 1 (`-j1`), one task at a time
- **Result:** **27/27** tasks exit 0, carry their `val BIGNUM_<OP>_<FAM>_CORRECT : thm` line,
  and pass upstream's `check_axioms ()`. Machine-readable record:
  `research/receipts/ix56-upstream-coverage.json` (`"ok": 27, "total": 27`).

This closes TODO **XII.F7.3** and **XII.F7.4**. It is an acceptance-test run over
already-generated answers, not a new held-out split.

## 1. What was run

Exactly one command, from the repository root:

```
cargo run --release --offline --bin dropin -- sweep --arch all --force \
    --json research/receipts/ix56-upstream-coverage.json
```

`--force` makes the driver delete the target `.correct` and pass `make -B`, so every task is
recompiled (HOL native binary) and re-run from scratch; nothing is reported from a cached
`.correct`. `--timeout` was left at its default 900 s per task; no task came close, so the
kill branch did not fire in this run (its exit-code classification, 124/137 from GNU
`timeout`, is unit-tested in `src/tools/dropin.rs`).

Environment (all values are literal):

| variable | value |
|---|---|
| `HOME` | `/home/one` |
| `HOLDIR` | `/mnt/1tb_kingston/Tekmerion/.bench/hol-light` |
| `PATH` | `/mnt/1tb_kingston/Tekmerion/.bench/bin:/mnt/1tb_kingston/Tekmerion/.bench/hol-light/_opam/bin:/home/one/.elan/toolchains/leanprover--lean4---v4.29.1/bin:/home/one/.local/bin:/usr/local/bin:/usr/bin:/bin` |

Notes on that `PATH`, because the run does not build without it:

- `.bench/bin` holds the aarch64 assembler/objdump shims; ARM targets need them (the host has
  no `binutils-aarch64-linux-gnu`).
- `_opam/bin` supplies the `ocamlopt`/`ocamlfind`/`camlp5r` that match `hol_lib.cmxa`;
  `/home/one/.local/bin` supplies `opam`, which upstream `tools/build-proof.sh` calls to set
  `CAML_LD_LIBRARY_PATH` (without it the preprocessor fails to load `dllpcre2_stubs`).
- The lean toolchain `bin` supplies `cadical` 2.1.2, which the generated proofs' self-contained
  `CSAT_PROVE` invokes. (Without it the ARM `neg` proofs fail with
  `CSAT: cadical did not report UNSAT, exit 127`.)

Per task, upstream runs its own `make -j1 HOLDIR=… <dir>/bignum_<op>_<fam>.correct`. That
target goes through `tools/build-proof.sh`, which generates a template ending in

```
loadt "<proof>.ml";;
check_axioms ();;
```

and then through `tools/run-proof.sh` under `set -e`. `check_axioms`
(`.bench/hol-light/hol_lib.ml:109`) filters `INFINITY_AX`, `SELECT_AX`, `ETA_AX` out of
`axioms ()` and `failwith`s if anything remains; `run-proof.sh` forwards a non-zero status and
also greps for `error:`/`exception:`. **Make exiting 0 is therefore equivalent to
`check_axioms` passing** for that target. The driver records `check_axioms` as the exit-0
consequence and does not claim more.

`make -B` also re-runs upstream's own `tools/collect-signatures.py` (its `subroutine_signatures.ml`
rule). That is upstream's build step; Tekmerion neither owns nor writes Python
(`git ls-files '*.py'` is empty).

Pins are read mechanically by the driver (`git -C <tree> rev-parse --short HEAD`) and are in
the JSON: s2n-bignum `5e0fc782`, HOL Light `d30e823`.

## 2. The table

Generated from `research/receipts/ix56-upstream-coverage.json` with

```
awk 'function val(l,k,  m){match(l,"\"" k "\": [^,}]*");m=substr(l,RSTART,RLENGTH);sub("\"" k "\": ","",m);gsub(/"/,"",m);return m} /"exit": /{printf "| %s | %s | %s | %s | %s | %s | %s |\n", val($0,"arch"), val($0,"family"), val($0,"op"), val($0,"exit"), val($0,"theorem"), val($0,"check_axioms"), val($0,"seconds")}' research/receipts/ix56-upstream-coverage.json
```

`seconds` is the `.correct` `Running time:` value verbatim (the wall time of the `time prove`
call), printed at the file's own six decimals.

| arch | family | operation | exit | theorem | check_axioms | seconds |
|---|---|---|---|---|---|---|
| arm | p25519 | add | 0 | BIGNUM_ADD_P25519_CORRECT | true | 47.000000 |
| arm | p256 | add | 0 | BIGNUM_ADD_P256_CORRECT | true | 47.000000 |
| arm | p256k1 | add | 0 | BIGNUM_ADD_P256K1_CORRECT | true | 47.000000 |
| arm | p384 | add | 0 | BIGNUM_ADD_P384_CORRECT | true | 75.000000 |
| arm | p521 | add | 0 | BIGNUM_ADD_P521_CORRECT | true | 122.000000 |
| arm | sm2 | add | 0 | BIGNUM_ADD_SM2_CORRECT | true | 46.000000 |
| arm | p25519 | sub | 0 | BIGNUM_SUB_P25519_CORRECT | true | 40.000000 |
| arm | p256 | sub | 0 | BIGNUM_SUB_P256_CORRECT | true | 39.000000 |
| arm | p256k1 | sub | 0 | BIGNUM_SUB_P256K1_CORRECT | true | 40.000000 |
| arm | p384 | sub | 0 | BIGNUM_SUB_P384_CORRECT | true | 56.000000 |
| arm | p521 | sub | 0 | BIGNUM_SUB_P521_CORRECT | true | 60.000000 |
| arm | sm2 | sub | 0 | BIGNUM_SUB_SM2_CORRECT | true | 40.000000 |
| arm | p25519 | neg | 0 | BIGNUM_NEG_P25519_CORRECT | true | 30.000000 |
| arm | p256 | neg | 0 | BIGNUM_NEG_P256_CORRECT | true | 30.000000 |
| arm | p256k1 | neg | 0 | BIGNUM_NEG_P256K1_CORRECT | true | 30.000000 |
| arm | p384 | neg | 0 | BIGNUM_NEG_P384_CORRECT | true | 37.000000 |
| arm | p521 | neg | 0 | BIGNUM_NEG_P521_CORRECT | true | 44.000000 |
| arm | sm2 | neg | 0 | BIGNUM_NEG_SM2_CORRECT | true | 31.000000 |
| x86 | p25519 | add | 0 | BIGNUM_ADD_P25519_CORRECT | true | 70.000000 |
| x86 | p256k1 | add | 0 | BIGNUM_ADD_P256K1_CORRECT | true | 71.000000 |
| x86 | sm2 | add | 0 | BIGNUM_ADD_SM2_CORRECT | true | 66.000000 |
| x86 | p25519 | sub | 0 | BIGNUM_SUB_P25519_CORRECT | true | 57.000000 |
| x86 | p256k1 | sub | 0 | BIGNUM_SUB_P256K1_CORRECT | true | 64.000000 |
| x86 | sm2 | sub | 0 | BIGNUM_SUB_SM2_CORRECT | true | 57.000000 |
| x86 | p25519 | neg | 0 | BIGNUM_NEG_P25519_CORRECT | true | 51.000000 |
| x86 | p256k1 | neg | 0 | BIGNUM_NEG_P256K1_CORRECT | true | 51.000000 |
| x86 | sm2 | neg | 0 | BIGNUM_NEG_SM2_CORRECT | true | 50.000000 |

**`OK 27/27`** (18 ARM + 9 x86). Every row: exit 0, theorem name present in the `.correct`
file, `check_axioms` = true, no timeout. The sweep itself printed `OK 27/27` and exited 0.

Each `<dir>/bignum_<op>_<fam>.correct` contains exactly one `Running time:` line and no
`error:`/`exception:` text. `neg_p521` and `neg_sm2` (the two ARM tasks that were missing
from the interrupted earlier run) are both green here, at 44 s and 31 s.

## 3. What this proves, and what it does not

**Proves.** All **27** proofs that Tekmerion's generators currently produce for these tasks —
18 ARM + 9 x86, `add`/`sub`/`neg` over p25519, p256, p256k1, p384, p521, sm2 (ARM) and
p25519, p256k1, sm2 (x86) — are accepted by upstream s2n-bignum's **own** build at pin
`5e0fc782`, in **one pass, one tree, one core**, with `check_axioms ()` passing. Each proof
loads, states its `_CORRECT` theorem, and the session contains no axiom outside the three
basic ones.

**Does not prove.**

- *Coverage of s2n-bignum.* The denominator in F7 is **675 straight-line FC theorems**
  (`tekmerion-mentorship-findings.md` §F7; TODO XII.F7.1). This run is **27/675 = 4.0%**.
  The remaining **~648 theorems were not attempted**. This is not an upstream-coverage number;
  it is a per-task acceptance result on the tasks the generators were pointed at.
- *Sealed provenance for the ARM half.* The ARM answers in `.bench/dropin-gen/answers-arm/`
  were produced by the **spent-manifest emitter** against `research/receipts/spent-r4-arm.json`
  as **development data** (the R2 held-out set is already spent). They are not a sealed-freeze
  result. The x86 answers in `.bench/dropin-gen/answers/` came from the **frozen v4 sealed
  emitter** (`research/receipts/ix55-sealed-manifest.json`, tag `freeze-x86-sealed-v4`).
- *Per-theorem axiom dependency.* `axioms ()` is a **session-level** query. The honest
  statement is "no axiom outside `INFINITY_AX`, `SELECT_AX`, `ETA_AX` is present in the
  session that checked this theorem", **not** "this theorem depends only on those three".
  The per-theorem export/stamp is XII.F2, which remains open.
- *A whole-tree build.* Only these 27 targets were built. Every other proof in
  `.bench/dropin` is upstream's own human proof, untouched.
- *Downstream usability.* No consumer of the generated `_CORRECT` theorems was built.
- *Anything but functional correctness.* No constant-time or memory-safety property is
  measured here.

## 4. Timing

- **Slowest task: `arm/p521/add_p521`, 122.000000 s.** Against the 120 s per-task budget this
  is **101.7% — over budget**. Against XII.F9.2, which makes anything above 70% of its timeout
  a defect to fix before a sealed run (threshold 84 s), it is a defect by a wide margin. The
  earlier development datapoint (96.9 s) is superseded: this is a fresh `make -B` rebuild.
- Next highest, all below the 84 s threshold: `arm/p384/add_p384` 75 s (62.5%),
  `x86/p256k1/add_p256k1` 71 s (59.2%), `x86/p25519/add_p25519` 70 s (58.3%),
  `x86/sm2/add_sm2` 66 s (55.0%). Only `add_p521` is above 70%.
- Sum of `Running time` across the 27 tasks: **1398 s** (ARM 861 s, x86 537 s) — from the JSON
  with `jq '[.tasks[].seconds] | add'`. The forced pass wall clock was **46 m 54 s**
  (08:56:23Z → 09:43:17Z); roughly half of that is the HOL native recompilation that `-B`
  forces per task, not proof checking.

An earlier attempt at this command (08:26:25Z–08:56:12Z) completed the 18 ARM tasks green and
then aborted before the first x86 `make`, because the new driver created the per-architecture
log directory only once. That is a driver defect, not a proof failure; it was fixed
(`fs::create_dir_all` per task) and the whole 27-task pass was re-run from scratch. The table
above is the second, single, successful pass. Total wall time for both attempts was
29 m 47 s + 46 m 54 s = 76 m 41 s, inside the 80-minute budget.

## 5. Reproduction

```
cd /mnt/1tb_kingston/Tekmerion
export HOME=/home/one
export HOLDIR=/mnt/1tb_kingston/Tekmerion/.bench/hol-light
export PATH="/mnt/1tb_kingston/Tekmerion/.bench/bin:/mnt/1tb_kingston/Tekmerion/.bench/hol-light/_opam/bin:/home/one/.elan/toolchains/leanprover--lean4---v4.29.1/bin:/home/one/.local/bin:/usr/local/bin:/usr/bin:/bin"
cargo run --release --offline --bin dropin -- sweep --arch all --force \
    --json research/receipts/ix56-upstream-coverage.json
```

Prerequisites: `.bench/dropin` (upstream s2n-bignum at `5e0fc782`, objects built),
`.bench/hol-light` (`d30e823`, with `_opam`), and the 27 answer files under
`.bench/dropin-gen/answers{,-arm}/`. To check the plan without invoking `make`, use
`--dry-run` instead of `--force`; it prints `DRY-RUN 27/27 assembled; make not invoked`.
A single architecture is `--arch arm` (18 tasks) or `--arch x86` (9 tasks).
