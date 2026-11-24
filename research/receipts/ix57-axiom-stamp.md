# IX.57 — per-theorem axiom stamps for the 27 generated proofs

- **Date:** 2026-11-24, run `10:56:32Z`–`11:43:09Z` (`generated_at` 1795520589 =
  2026-11-24T11:43:09Z); wall clock **46 min 37 s** against a 100-minute budget.
- **Branch:** `staging`
- **Trees:** `.bench/dropin` = upstream s2n-bignum at `5e0fc782`;
  measurement build `.bench/hol-light-stamp` = frozen HOL Light `d30e823` + the
  axiom-stamp patch (`tools/hol-light/axiom-stamp.diff`). The frozen
  `.bench/hol-light` was **not modified** (§6).
- **Cores:** 1 (`make -j1`, one task at a time).
- **Result:** **27/27** written proofs built, carried their `val
  BIGNUM_<OP>_<FAM>_CORRECT : thm` line, passed upstream's `check_axioms ()`, and
  printed a per-theorem stamp. Every stamp is exactly
  `{ETA_AX, SELECT_AX, INFINITY_AX}`; **no theorem touches an axiom outside the
  three basic ones** (`outside_basic_axioms=0`).
- **Machine-readable record:** `research/receipts/ix57-axiom-stamp.json`
  (`"ok": 27, "total": 27`, `"stamp": true`).

This measures something the previous receipts could not: `check_axioms ()` is a
*session* predicate, so IX.54/IX.56 could only say "no axiom other than the three
exists in this session". This receipt says, per theorem, **which** of the three
its derivation actually touches.

---

## 1. The mechanism, and why it is exact rather than inferred

`check_axioms` (`.bench/hol-light/hol_lib.ml:109`) filters `axioms()` — every
axiom in the session — against `[INFINITY_AX; SELECT_AX; ETA_AX]`. It cannot see
which of those a particular proof used. Instrumenting the kernel closes that gap
at the only place where the information is still exact: the primitive rules.

The patch changes one file, `fusion.ml`, and nothing else:

1. `type thm = Sequent of (term list * term) * int`. The `int` is a bitmask of
   the axioms the derivation touches.
2. A registry. `new_axiom` is the **only** rule that creates a bit: it allocates
   the next id, records `(id, conclusion-as-string)`, and returns a theorem whose
   mask is exactly `1 lsl (id-1)`. The three basic axioms are therefore
   registered by construction, in the order they are created by
   `class.ml`/`nums.ml`. The kernel contains **no name check** for
   `ETA_AX`/`SELECT_AX`/`INFINITY_AX`.
3. Every other primitive rule unions the masks of its theorem arguments: `TRANS`,
   `MK_COMB`, `ABS`, `EQ_MP`, `DEDUCT_ANTISYM_RULE`, `INST`, `INST_TYPE`.
   `REFL`, `BETA`, `ASSUME`, `new_basic_definition` and
   `new_basic_type_definition` return mask `0`, because those rules introduce no
   axiom: a definition is a conservative extension, and `ASSUME` carries its
   dependency in the hypothesis list, which is unchanged.
4. `dest_thm`, `hyp`, `concl` and `axioms()` behave exactly as before. `axioms()`
   still returns the axiom theorems, so `check_axioms` still bites (§3, C2).
5. New exports in `Hol_kernel` and in the implementation:
   - `axiom_registry : unit -> (int * string) list` — `(id, conclusion)`, in id
     order;
   - `stamp_of : thm -> int list` — sorted ids;
   - `stamp_string : thm -> string` — `id:1=|- <conclusion>|id:2=…`, each
     conclusion whitespace-collapsed and truncated to 80 characters;
   - `inference_count : unit -> int` — incremented once per invocation of a
     primitive rule.

**Why this is exact, not an estimate.** The mask is computed by the same kernel
transitions that produce the theorem. There is no separate analysis of the term,
no search over the session's axiom list, and no assumption about which library
lemmas were used. If a theorem is produced at all, it was produced by a sequence
of these rules, and the mask is the union along that sequence. Two facts make it
a *lower* bound that is also an upper bound: `new_axiom` is the only source of
bits (so a derivation that never calls it cannot acquire one), and every rule
that consumes theorems copies their masks (so no bit can be dropped). Taint can
only be dropped by `REFL`/`BETA`/`ASSUME`/definition rules, and those rules take
no theorem argument — the definition rules' argument is discarded because the
result is definitional.

**What the mask is not.** (a) It says nothing about definitions: `new_definition`
and `new_basic_type_definition` contribute `0`, so the stamp covers axioms only,
not the definitional dependency graph. (b) It is not a proof object: the mask is a
summary bit per axiom, not a derivation. (c) `inference_count` is *work*, not
proof-tree size: it counts every primitive-rule invocation made during `prove`,
including invocations inside tactic search that fail and are retried, and
including invocations that raise. The reported number is
`inference_count()` after the `prove` call minus the value captured immediately
before it (`STAMP_INF_BEFORE`/`STAMP_INF_AFTER` bracket the call).

`Sequent` occurs 26 times in `fusion.ml` and nowhere else in the tree (the other
matches for `Sequent` are `Sequential` in prose), so the representation change is
contained in the kernel.

### A note on how the controls were used during construction

The first version of the instrumented `TRANS` compared the right-hand side of
the second equation where it should have compared its left-hand side. The stamp
build failed immediately at library initialisation —
`Failure("EQT_ELIM")` raised inside `CONJ` in `bool.ml` — i.e. HOL Light's own
derived rules caught the defective kernel rule before any proof ran. The defect
was fixed and the controls below were re-run against the final build. No
measurement in this receipt was taken with the defective build. (The extra
`EQT_ELIM`-vs-`Failure` diagnosis needed a temporary patch to the *copy's*
`bool.ml`; that file was then restored byte-identically from the frozen tree —
`cmp` clean.)

---

## 2. The build

Only the stamp copy is built. Its `_opam` is a symlink to the frozen switch, so
the existing OCaml 5.4.0 toolchain is reused and nothing is fetched.

```
$ cd .bench/hol-light-stamp && make HOLLIGHT_USE_MODULE=1 hol_lib.cmxa hol.sh
        HOLLIGHT_DIR="`pwd`" ocaml inline_load.ml hol_lib.ml hol_lib_inlined.ml -omit-prelude
        OCAMLRUNPARAM=l=1000000000 ocamlopt.byte -verbose -c \
              -pp "camlp5r pa_lexer.cmo pa_extend.cmo q_MLast.cmo -I . pa_j.cmo" \
              hol_lib_inlined.ml hol_loader.cmx bignum.cmx -o hol_lib.cmx
+ camlp5r pa_lexer.cmo pa_extend.cmo q_MLast.cmo -I . pa_j.cmo 'hol_lib_inlined.ml' > /tmp/ocamlpp6755d9
        ocamlfind ocamlopt -package zarith -a -o hol_lib.cmxa bignum.cmx hol_loader.cmx hol_lib.cmx
+ camlp5r pa_lexer.cmo pa_extend.cmo q_MLast.cmo -I . pa_j.cmo 'hol_lib_inlined.ml' > /tmp/ocamlpp77a076
    ... (hol.sh recipe echo, last lines) ...
		chmod +x hol.sh ; \
else \
	echo 'FAILURE: hol.sh assumes Linux' ; \
fi
exit=0
```

(The grep above is verbatim; `make` also echoes the `hol.sh` recipe, which is why
the literal tail is the recipe text. Exit status 0.)

```
$ ls -la hol_lib.cmxa hol.sh
-rw-r--r-- 1 one one 3353 2026-11-24 11:44 hol_lib.cmxa
-rwxr-xr-x 1 one one 1806 2026-11-24 11:44 hol.sh
```

One environment note worth recording: the first attempt failed with

```
Fatal error: cannot load shared library dllpcre2_stubs
Reason: dllpcre2_stubs.so: cannot open shared object file: No such file or directory
```

because `camlp5r` needs the switch's `stublibs`. The Makefile does not call
`opam env`; the build must be run with the switch's `bin` on `PATH` **and**
`CAML_LD_LIBRARY_PATH` pointing into `_opam/lib` (see §7). Versions in the switch:
OCaml `5.4.0`, camlp5 `8.04.00`, zarith `1.14`, `camlp-streams` present.

---

## 3. The four mandatory controls

Each control is a small `.ml` file inlined into the stamp library with
`.bench/hol-light-stamp/hol.sh inline-load`, then compiled and linked against the
stamp `hol_lib.cmxa` (the same path `build-proof.sh` uses). The files are
scratch (`.bench/` is git-ignored); their text is in §8.

### C1 — zero-taint: `REFL` has an empty stamp

Input: `let refl_bool = REFL `x:bool`;;`, then print its ids and `stamp_string`.
Output, verbatim:

```
C1_REGISTRY_SIZE 3
C1_REGISTRY id=1 (! (\t. ((= (\x. (t x))) t)))
C1_REGISTRY id=2 (! (\P. (! (\x. ((==> (P x)) (P (@ P)))))))
C1_REGISTRY id=3 (? (\f. ((/\ (ONE_ONE f)) (~ (ONTO f)))))
C1_REFL_THM |- x <=> x
C1_REFL_IDS []
C1_REFL_STAMP []
C1_NAME ETA_AX ids=[1] text=id:1=|- (! (\t. ((= (\x. (t x))) t)))
C1_NAME SELECT_AX ids=[2] text=id:2=|- (! (\P. (! (\x. ((==> (P x)) (P (@ P)))))))
C1_NAME INFINITY_AX ids=[3] text=id:3=|- (? (\f. ((/\ (ONE_ONE f)) (~ (ONTO f)))))
C1_DONE
```

The stamp is empty: the instrument does not over-taint. The same run shows the
registry holds exactly the three basic axioms (session size 3) and that the
name→id map is taken from the kernel values themselves (`stamp_of ETA_AX = [1]`,
`stamp_of SELECT_AX = [2]`, `stamp_of INFINITY_AX = [3]`). Ids are allocated in
creation order (`ETA_AX` in `class.ml`, `SELECT_AX` in `class.ml`,
`INFINITY_AX` in `nums.ml`); the receipt does **not** rely on that order, it reads
it off the run.

### C2 — injected axiom: taint propagates, and `check_axioms` still bites

Input: `let QXA = new_axiom `(!n:num. n = n + 1) = F`;;` then
`let BAD = EQ_MP QXA (ASSUME `!n:num. n = n + 1`);;` (an `EQ_MP` derivation from
the new axiom). Output, verbatim:

```
C2_REGISTRY_BEFORE 3
C2_QXA_THM |- (forall n. n = n + 1) <=> false
C2_QXA_IDS [4]
C2_QXA_STAMP id:4=|- ((= (! (\n. ((= n) ((+ n) (NUMERAL (BIT1 _0))))))) F)
C2_DERIVED_THM forall n. n = n + 1
C2_DERIVED_IDS [4]
C2_DERIVED_STAMP id:4=|- ((= (! (\n. ((= n) ((+ n) (NUMERAL (BIT1 _0))))))) F)
C2_REGISTRY_AFTER 4
C2_CHECK_AXIOMS FAILED check_axioms: [|- (forall n. n = n + 1) <=> false]
```

The new axiom gets id 4; the mask of the theorem derived from it by `EQ_MP` is
exactly `[4]`; the registry grows from 3 to 4; and `check_axioms ()` now fails,
naming the injected axiom. Taint propagates through the rules and the check was
not relaxed.

### C3 — non-permissiveness: false statements remain unprovable

Output, verbatim:

```
C3_ARITH_RAISED Failure("ARITH_RULE `1 = 2`: REAL_ARITH `~true`: linear_ineqs: no contradiction")
C3_REFL_RAISED Failure("REFL_TAC")
C3_EQ_MP_RAISED Failure("EQ_MP")
C3_TRANS_RAISED Failure("TRANS")
C3_SUM_RAISED Failure("ARITH_RULE `2 + 2 = 5`: REAL_ARITH `~true`: linear_ineqs: no contradiction")
C3_STILL_ALIVE
```

`prove(`1 = 2`, ARITH_TAC)`, `prove(`1 = 2`, REFL_TAC)` and
`prove(`2 + 2 = 5`, ARITH_TAC)` all raise; a deliberately ill-formed `EQ_MP`
(`EQ_MP (REFL `T`) (ASSUME `F`)`) and a mismatched `TRANS`
(`TRANS (REFL `T`) (REFL `F`)`) also raise. The process is still alive after all
five failures, so these are refusals, not crashes.

### C4 — equivalence with the frozen build

The 27-task run must accept exactly the proofs the frozen build accepts. Two
checks, both from the JSON files:

Sets (arch, dir, op, family, exit) extracted from
`research/receipts/ix56-upstream-coverage.json` (frozen build, IX.56) and
`research/receipts/ix57-axiom-stamp.json` (stamp build, this receipt):

```
$ wc -l /tmp/s56.txt /tmp/s57.txt
  27 /tmp/s56.txt
  27 /tmp/s57.txt
  54 total
$ diff /tmp/s56.txt /tmp/s57.txt && echo "SETS_IDENTICAL"
SETS_IDENTICAL
```

where each file was produced by

```
sed -n 's/.*"arch": "\([a-z0-9]*\)", "dir": "\([a-z0-9]*\)", "op": "\([a-z]*\)", "family": "\([a-z0-9]*\)".*"exit": \([0-9]*\).*/\1 \2 \3 \4 exit=\5/p' <json> | sort
```

Accepted theorems and axiom sets:

```
stamped=27 missing_stamp=0 outside_basic_axioms=0 total=27
```

(`dropin stamp-table --json research/receipts/ix57-axiom-stamp.json`; full table
in §4.) So the stamp build accepts the same 27 theorems as the frozen build, and
each of them has a complete stamp that lies inside the three basic axioms. Set
equality, not just count equality.

Wall-time cross-check (same tasks, same answers; only the HOL build differs):

```
SUM56=1398 SUM57=1386 MAXABS=2
```

i.e. 1398 s frozen vs 1386 s stamped over the 27 check runs, largest per-task
difference 2 s. The taint and the counter are inside run-to-run noise at the 1-second
resolution of `Running time:`.

---

## 4. The empirical run

Exactly one command, from the repository root:

```
cargo run --release --offline --bin dropin -- sweep --arch all --force --stamp \
  --hol .bench/hol-light-stamp --json research/receipts/ix57-axiom-stamp.json
```

`--force` deletes each target `.correct` and passes `make -B`, so every task is
recompiled against the stamp build and re-run; nothing is read from a cached
result. `--stamp` assembles the proof with the stamp template (§8) and requires a
`STAMP` line for the task to count as green. One core, one task at a time.

Per-theorem table, generated from the JSON by
`cargo run --release --offline --bin dropin -- stamp-table --json research/receipts/ix57-axiom-stamp.json`
(pasted verbatim; no number here is typed by hand):

<!-- BEGIN GENERATED TABLE -->
## Per-theorem axiom stamp

| arch | task | axioms used | inferences | seconds |
|---|---|---|---:|---:|
| arm | curve25519/add_p25519 | {ETA_AX, SELECT_AX, INFINITY_AX} | 170225275 | 46.000 |
| arm | p256/add_p256 | {ETA_AX, SELECT_AX, INFINITY_AX} | 170931670 | 47.000 |
| arm | secp256k1/add_p256k1 | {ETA_AX, SELECT_AX, INFINITY_AX} | 157240305 | 46.000 |
| arm | p384/add_p384 | {ETA_AX, SELECT_AX, INFINITY_AX} | 349139623 | 73.000 |
| arm | p521/add_p521 | {ETA_AX, SELECT_AX, INFINITY_AX} | 681707202 | 120.000 |
| arm | sm2/add_sm2 | {ETA_AX, SELECT_AX, INFINITY_AX} | 159985379 | 46.000 |
| arm | curve25519/sub_p25519 | {ETA_AX, SELECT_AX, INFINITY_AX} | 110957174 | 40.000 |
| arm | p256/sub_p256 | {ETA_AX, SELECT_AX, INFINITY_AX} | 103417925 | 39.000 |
| arm | secp256k1/sub_p256k1 | {ETA_AX, SELECT_AX, INFINITY_AX} | 110121226 | 40.000 |
| arm | p384/sub_p384 | {ETA_AX, SELECT_AX, INFINITY_AX} | 214410496 | 55.000 |
| arm | p521/sub_p521 | {ETA_AX, SELECT_AX, INFINITY_AX} | 221845891 | 59.000 |
| arm | sm2/sub_sm2 | {ETA_AX, SELECT_AX, INFINITY_AX} | 107504057 | 39.000 |
| arm | curve25519/neg_p25519 | {ETA_AX, SELECT_AX, INFINITY_AX} | 35457948 | 29.000 |
| arm | p256/neg_p256 | {ETA_AX, SELECT_AX, INFINITY_AX} | 45739252 | 30.000 |
| arm | secp256k1/neg_p256k1 | {ETA_AX, SELECT_AX, INFINITY_AX} | 45424790 | 31.000 |
| arm | p384/neg_p384 | {ETA_AX, SELECT_AX, INFINITY_AX} | 86799846 | 37.000 |
| arm | p521/neg_p521 | {ETA_AX, SELECT_AX, INFINITY_AX} | 123668935 | 44.000 |
| arm | sm2/neg_sm2 | {ETA_AX, SELECT_AX, INFINITY_AX} | 46665222 | 30.000 |
| x86 | curve25519/add_p25519 | {ETA_AX, SELECT_AX, INFINITY_AX} | 160584653 | 69.000 |
| x86 | secp256k1/add_p256k1 | {ETA_AX, SELECT_AX, INFINITY_AX} | 166694408 | 71.000 |
| x86 | sm2/add_sm2 | {ETA_AX, SELECT_AX, INFINITY_AX} | 127361354 | 65.000 |
| x86 | curve25519/sub_p25519 | {ETA_AX, SELECT_AX, INFINITY_AX} | 61298759 | 57.000 |
| x86 | secp256k1/sub_p256k1 | {ETA_AX, SELECT_AX, INFINITY_AX} | 111312547 | 64.000 |
| x86 | sm2/sub_sm2 | {ETA_AX, SELECT_AX, INFINITY_AX} | 67342905 | 58.000 |
| x86 | curve25519/neg_p25519 | {ETA_AX, SELECT_AX, INFINITY_AX} | 19973249 | 50.000 |
| x86 | secp256k1/neg_p256k1 | {ETA_AX, SELECT_AX, INFINITY_AX} | 23422025 | 51.000 |
| x86 | sm2/neg_sm2 | {ETA_AX, SELECT_AX, INFINITY_AX} | 18230539 | 50.000 |

## Axiom registry (raw conclusions)

| id | name | conclusion |
|---|---|---|
| 1 | ETA_AX | `(! (\t. ((= (\x. (t x))) t)))` |
| 2 | SELECT_AX | `(! (\P. (! (\x. ((==> (P x)) (P (@ P)))))))` |
| 3 | INFINITY_AX | `(? (\f. ((/\ (ONE_ONE f)) (~ (ONTO f)))))` |

stamped=27 missing_stamp=0 outside_basic_axioms=0 total=27
<!-- END GENERATED TABLE -->

The "axioms used" column is `id:*` entries from the theorem's own `STAMP` line,
mapped to names through the run's own `STAMP_NAME` lines; the registry table is
the raw `STAMP_REGISTRY` text the proofs printed, i.e. the evidence, not a
transcription. Nothing outside the registry appears.

- Slowest task: **arm `p521/add_p521`, 120 s** (frozen: 122 s).
- Largest derivation work: **arm `p521/add_p521`, 681,707,202 primitive-rule
  invocations**.
- Smallest: **x86 `sm2/neg_sm2`, 18,230,539** invocations, 50 s.

Two independent complete runs of the same command produced **identical
inference deltas for all 27 tasks** (the per-task numbers above); the wall times
differed by at most 2 s. The count is deterministic for a fixed proof and build.

---

## 5. What this establishes, and what it does not

**Establishes.** For each of the 27 generated theorems, the exact set of
`new_axiom` results its derivation touches, measured inside the kernel: all 27
use `{ETA_AX, SELECT_AX, INFINITY_AX}` and nothing else. Combined with
`check_axioms ()` passing for all 27 (C4), the previously session-level claim is
now a per-theorem claim with a receipt. The measurement also gives a
primitive-inference count per theorem, which is a new size baseline for the
"generated vs human proof" comparison (XII.F2.5) — with the caveat that it counts
search work, not proof-object nodes.

**Does not establish:**

- that the theorems are *true* relative to those axioms. That is the kernel's
  job, and C3 shows the kernel still refuses false statements. The stamp is a
  dependency record, not a truth claim.
- the **definitional** dependency graph. `new_definition` and
  `new_basic_type_definition` contribute mask `0`, so the stamp says which
  *axioms* were used, not which definitions.
- an **exported derivation object** to check outside this process. There is no
  proof-object export here: proof-object export, HOLTrace-style external
  re-checking and Dedukti/hol2dk export all **remain open**. The mask cannot be
  replayed by a third party; it is a measurement made by the same kernel that
  made the theorem.
- anything about ARM/x86 tasks outside these 27, nor a new held-out split. These
  are the IX.53–IX.56 answers; only the acceptance test and the measurement are
  new.

---

## 6. The instrument is a measurement build, not the trusted base

- The frozen `.bench/hol-light` is **untouched**: `git -C .bench/hol-light status
  --short --untracked-files=no` is empty (0 modified tracked files), and HEAD is
  still `d30e823`. The only untracked entries there are build artefacts dated
  2026-11-22, before this work.
- The instrument lives in `.bench/hol-light-stamp/`, a git-ignored scratch copy
  (`rsync -a --exclude=_opam .bench/hol-light/ .bench/hol-light-stamp/`, plus
  `ln -s …/_opam .bench/hol-light-stamp/_opam`). It may be deleted; the diff is
  enough to rebuild it.
- The patch is tracked as `tools/hol-light/axiom-stamp.diff`, generated with

  ```
  diff -u --label a/fusion.ml --label b/fusion.ml \
      .bench/hol-light/fusion.ml .bench/hol-light-stamp/fusion.ml \
      > tools/hol-light/axiom-stamp.diff
  ```

  - `sha256(tools/hol-light/axiom-stamp.diff)` =
    `f1b8a6db45bb053f76b4b51c5731519345bc8d778826bf0ef14df620fde0a6ac`
    (273 lines).
  - `sha256(.bench/hol-light/fusion.ml)` (frozen, before) =
    `29544be92d9cc1e6b3b59ba5b210604b9a748366a68b3bcdc2619717d68fd98a`
  - `sha256(.bench/hol-light-stamp/fusion.ml)` (patched) =
    `430fde277fc6d226ac2970d58f242ad9f70bd2e84da568b2b937f25da365be45`
  - Applying the diff to a pristine copy reproduces the patched file exactly:

    ```
    $ cp .bench/hol-light/fusion.ml /tmp/axcheck/fusion.ml
    $ cd /tmp/axcheck && patch -p1 --forward < …/tools/hol-light/axiom-stamp.diff
    patching file fusion.ml
    $ diff -q /tmp/axcheck/fusion.ml …/.bench/hol-light-stamp/fusion.ml && echo PATCH_REPRODUCES_STAMP_FUSION
    PATCH_REPRODUCES_STAMP_FUSION
    ```

`hol_light_pin` in the JSON reads `d30e823` because the copy carries the frozen
`.git`; that is the **base** commit of the sources the patch was applied to, not
a claim that the stamp tree equals that commit.

---

## 7. Exact commands, pins, environment, reproduction

Run on 2026-11-24, `cores=1`. Pins:

| item | value |
|---|---|
| Tekmerion branch | `staging` (base commit `00d413c2a5e1195a3ff0cd49f95d43e2685fc576`) |
| s2n-bignum | `5e0fc782` |
| HOL Light (frozen and base of the patch) | `d30e823` |
| OCaml | `5.4.0` (switch at `.bench/hol-light/_opam`) |
| camlp5 / zarith | `8.04.00` / `1.14` |
| cadical | `2.1.2` at `/home/one/.elan/toolchains/leanprover--lean4---v4.29.1/bin/cadical` |

Environment actually used (literal):

```
PATH=/mnt/1tb_kingston/Tekmerion/.bench/bin:/home/one/.elan/toolchains/leanprover--lean4---v4.29.1/bin:/mnt/1tb_kingston/Tekmerion/.bench/hol-light/_opam/bin:/home/one/.local/bin:/usr/local/bin:/usr/bin:/bin
CAML_LD_LIBRARY_PATH=/mnt/1tb_kingston/Tekmerion/.bench/hol-light/_opam/lib/stublibs:/mnt/1tb_kingston/Tekmerion/.bench/hol-light/_opam/lib/ocaml/stublibs:/mnt/1tb_kingston/Tekmerion/.bench/hol-light/_opam/lib/ocaml
```

`.bench/bin` holds the `aarch64-linux-gnu-as`/`objdump` shims (the host has no
aarch64 binutils); `cadical` is required by s2n-bignum's `CSAT` rule;
`CAML_LD_LIBRARY_PATH` is required by `camlp5r` (see §2). `build-proof.sh` itself
runs `opam env --switch <HOLDIR>/`, which resolves through the `_opam` symlink.

Reproduction, from the repository root:

```
rsync -a --exclude=_opam .bench/hol-light/ .bench/hol-light-stamp/
ln -s "$(realpath .bench/hol-light/_opam)" .bench/hol-light-stamp/_opam
cd .bench/hol-light-stamp && patch -p1 --forward < ../../tools/hol-light/axiom-stamp.diff
cd ../.. && PATH=… CAML_LD_LIBRARY_PATH=… make -C .bench/hol-light-stamp HOLLIGHT_USE_MODULE=1 hol_lib.cmxa hol.sh
cargo run --release --offline --bin dropin -- sweep --arch all --force --stamp \
  --hol .bench/hol-light-stamp --json research/receipts/ix57-axiom-stamp.json
cargo run --release --offline --bin dropin -- stamp-table --json research/receipts/ix57-axiom-stamp.json
```

Relative `--hol`/`--json` paths are anchored at the repository root by
`parse_sweep_args` (a relative `HOLDIR` cannot work, because `make` runs with
`current_dir(<dropin>/<arch>)`).

### Commands run verbatim, with output

```
$ cargo test --offline
test result: ok. 41 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
… (8 further targets, all "0 passed; 0 failed")

$ cargo clippy --offline --all-targets -- -D warnings
    Checking tekmerion v0.1.0 (/mnt/1tb_kingston/Tekmerion)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.42s

$ git ls-files '*.py'
(no output; py_count=0)

$ df -h /mnt/1tb_kingston | tail -1
/dev/nvme1n1p1  1.3T  1.3T   14G  99% /mnt/1tb_kingston
```

14 GB free at the end of the run (limit: ≥ 8 GB).

---

## 8. The stamp template and the control sources

`dropin assemble --stamp` (and `sweep --stamp`) append the following to the
generated proof `.ml`, around the `time prove` call:

```ocaml
let stamp_before_BIGNUM_<OP>_<FAM>_CORRECT = inference_count ();;
let BIGNUM_<OP>_<FAM>_CORRECT = time prove
 (<query>,
<answer>);;

Printf.printf "STAMP_BEGIN %s\n%!" "BIGNUM_<OP>_<FAM>_CORRECT";;
Printf.printf "STAMP %s\n%!" (stamp_string BIGNUM_<OP>_<FAM>_CORRECT);;
Printf.printf "STAMP_INF_BEFORE %d\n%!" stamp_before_BIGNUM_<OP>_<FAM>_CORRECT;;
Printf.printf "STAMP_INF_AFTER %d\n%!" (inference_count ());;
Printf.printf "STAMP_INFERENCES %d\n%!" (inference_count () - stamp_before_BIGNUM_<OP>_<FAM>_CORRECT);;
List.iter
  (fun (id,txt) -> Printf.printf "STAMP_REGISTRY %d %s\n%!" id txt)
  (axiom_registry ());;
List.iter
  (fun (nm,th) -> Printf.printf "STAMP_NAME %s %s\n%!" nm
     (String.concat "," (List.map string_of_int (stamp_of th))))
  ["ETA_AX",ETA_AX; "SELECT_AX",SELECT_AX; "INFINITY_AX",INFINITY_AX];;
```

`STAMP_INFERENCES` is the delta, `STAMP_INF_BEFORE`/`STAMP_INF_AFTER` are the raw
counter values, and `STAMP_NAME` is how the receipt maps ids to names without
hard-coding them. `sweep` parses these lines in Rust (`parse_stamp` in
`src/tools/dropin.rs`) — no Python is used to parse logs.

For one task (`arm/curve25519/neg_p25519`), the `.correct` file contains, verbatim:

```
STAMP_BEGIN BIGNUM_NEG_P25519_CORRECT
STAMP id:1=|- (! (\t. ((= (\x. (t x))) t)))|id:2=|- (! (\P. (! (\x. ((==> (P x)) (P (@ P)))))))|id:3=|- (? (\f. ((/\ (ONE_ONE f)) (~ (ONTO f)))))
STAMP_INF_BEFORE 67649313
STAMP_INF_AFTER 103107261
STAMP_INFERENCES 35457948
STAMP_REGISTRY 1 (! (\t. ((= (\x. (t x))) t)))
STAMP_REGISTRY 2 (! (\P. (! (\x. ((==> (P x)) (P (@ P)))))))
STAMP_REGISTRY 3 (? (\f. ((/\ (ONE_ONE f)) (~ (ONTO f)))))
STAMP_NAME ETA_AX 1
STAMP_NAME SELECT_AX 2
STAMP_NAME INFINITY_AX 3
val BIGNUM_NEG_P25519_CORRECT : thm = |- forall z x n pc.
```

The three control sources (scratch, git-ignored, reproducible from this text):

```ocaml
(* ctrl1_refl.ml *)
Printf.printf "C1_REGISTRY_SIZE %d\n%!" (List.length (axiom_registry ()));;
List.iter (fun (id,txt) -> Printf.printf "C1_REGISTRY id=%d %s\n%!" id txt)
  (axiom_registry ());;
let refl_bool = REFL `x:bool`;;
Printf.printf "C1_REFL_THM %s\n%!" (string_of_thm refl_bool);;
Printf.printf "C1_REFL_IDS [%s]\n%!" (String.concat "," (List.map string_of_int (stamp_of refl_bool)));;
Printf.printf "C1_REFL_STAMP [%s]\n%!" (stamp_string refl_bool);;
Printf.printf "C1_NAME ETA_AX ids=[%s] text=%s\n%!"
  (String.concat "," (List.map string_of_int (stamp_of ETA_AX))) (stamp_string ETA_AX);;
Printf.printf "C1_NAME SELECT_AX ids=[%s] text=%s\n%!"
  (String.concat "," (List.map string_of_int (stamp_of SELECT_AX))) (stamp_string SELECT_AX);;
Printf.printf "C1_NAME INFINITY_AX ids=[%s] text=%s\n%!"
  (String.concat "," (List.map string_of_int (stamp_of INFINITY_AX))) (stamp_string INFINITY_AX);;
Printf.printf "C1_DONE\n%!";;

(* ctrl2_inject.ml *)
Printf.printf "C2_REGISTRY_BEFORE %d\n%!" (List.length (axiom_registry ()));;
let QXA = new_axiom `(!n:num. n = n + 1) = F`;;
Printf.printf "C2_QXA_THM %s\n%!" (string_of_thm QXA);;
Printf.printf "C2_QXA_IDS [%s]\n%!" (String.concat "," (List.map string_of_int (stamp_of QXA)));;
Printf.printf "C2_QXA_STAMP %s\n%!" (stamp_string QXA);;
let BAD = EQ_MP QXA (ASSUME `!n:num. n = n + 1`);;
Printf.printf "C2_DERIVED_THM %s\n%!" (string_of_thm BAD);;
Printf.printf "C2_DERIVED_IDS [%s]\n%!" (String.concat "," (List.map string_of_int (stamp_of BAD)));;
Printf.printf "C2_DERIVED_STAMP %s\n%!" (stamp_string BAD);;
Printf.printf "C2_REGISTRY_AFTER %d\n%!" (List.length (axiom_registry ()));;
(try (check_axioms (); Printf.printf "C2_CHECK_AXIOMS PASSED\n%!")
 with Failure m -> Printf.printf "C2_CHECK_AXIOMS FAILED %s\n%!" m);;

(* ctrl3_nonpermissive.ml *)
(try (ignore (prove (`1 = 2`, ARITH_TAC)); Printf.printf "C3_ARITH_UNEXPECTEDLY_PROVED\n%!")
 with e -> Printf.printf "C3_ARITH_RAISED %s\n%!" (Printexc.to_string e));;
(try (ignore (prove (`1 = 2`, REFL_TAC)); Printf.printf "C3_REFL_UNEXPECTEDLY_PROVED\n%!")
 with e -> Printf.printf "C3_REFL_RAISED %s\n%!" (Printexc.to_string e));;
(try (ignore (EQ_MP (REFL `T:bool`) (ASSUME `F:bool`)); Printf.printf "C3_EQ_MP_UNEXPECTEDLY_OK\n%!")
 with e -> Printf.printf "C3_EQ_MP_RAISED %s\n%!" (Printexc.to_string e));;
(try (ignore (TRANS (REFL `T:bool`) (REFL `F:bool`)); Printf.printf "C3_TRANS_UNEXPECTEDLY_OK\n%!")
 with e -> Printf.printf "C3_TRANS_RAISED %s\n%!" (Printexc.to_string e));;
(try (ignore (prove (`2 + 2 = 5`, ARITH_TAC)); Printf.printf "C3_SUM_UNEXPECTEDLY_PROVED\n%!")
 with e -> Printf.printf "C3_SUM_RAISED %s\n%!" (Printexc.to_string e));;
Printf.printf "C3_STILL_ALIVE\n%!";;
```

built with

```
$ .bench/hol-light-stamp/hol.sh inline-load <control>.ml <control>_inlined.ml
$ .bench/hol-light-stamp/hol.sh compile <control>_inlined.ml
$ .bench/hol-light-stamp/hol.sh link <control>_inlined.cmx -o <control>.native
$ ./<control>.native
```

---

## 9. Non-Tekmerion tooling touched by the run

With `--force` → `make -B`, upstream s2n-bignum's own Makefile regenerates
`proofs/subroutine_signatures.ml` by running `python3 tools/collect-signatures.py`
from *its* tree. That script is upstream's, in the git-ignored `.bench/dropin`
copy, and it is not used to produce, parse or summarise any Tekmerion number in
this receipt (log parsing and the table are Rust: `parse_stamp` / `stamp-table`).
`git ls-files '*.py'` is empty.

---

## 10. Diagnostic: where the taint enters (appended 2026-11-24 by the orchestrator)

§4 reports that **all 27** theorems carry **all three** basic axioms. On its own that
number is easy to misread as "this proof needed choice". It is not: the stamp measures
the theorem's *derivation*, and the derivation of every machine-code theorem includes
the lemmas that build its statement. This section localises the taint so the number is
interpreted correctly.

**Probe source** (tracked): `research/receipts/ix57/probe_stamps.ml`, sha256
`302e77d127c18eb818f1acbf0c14a578b4aa52119f2b41c5d107833f4dffb793`. It is not a proof
and carries no claim; it loads `x86/proofs/base.ml` and prints `stamp_of` for a ladder of
theorems from the base context and from one real routine's decode.

**Command** (from `.bench/dropin/x86`, with the IX.57 environment of §7):

```
../tools/build-proof.sh "proofs/probe_stamps.ml" \
  "/mnt/1tb_kingston/Tekmerion/.bench/hol-light-stamp/hol.sh" \
  "curve25519/probe_stamps.native"
../tools/run-proof.sh "curve25519/probe_stamps.native" "curve25519/probe_stamps.diag"
grep -E "^PROBE" curve25519/probe_stamps.diag
```

**Output** (verbatim; `run-proof.sh` reported no `error:`/`exception:` line, and the
program reached `PROBE_DONE`):

```
PROBE BIGNUM_FROM_MEMORY_BYTES     ids=[1,2,3]
PROBE C_ARGUMENTS                  ids=[1,2,3]
PROBE NONOVERLAPPING_CLAUSES       ids=[1,2,3]
PROBE SOME_FLAGS                   ids=[]
PROBE define_assert_from_elf       ids=[]
PROBE define_trimmed               ids=[1,2,3]
PROBE X86_MK_CORE_EXEC_RULE        ids=[1,2,3]
PROBE new_definition p_25519       ids=[]
PROBE REFL_int64                   ids=[]
PROBE ARITH_RULE 2+2=4             ids=[1,2,3]
PROBE MESON P==>P                  ids=[1,2]
PROBE_DONE
```

Registry (`§4`): `1 = ETA_AX`, `2 = SELECT_AX`, `3 = INFINITY_AX`.

A second probe in the core context (before any s2n-bignum material is loaded) gives the
same shape: `F_DEF`, `AND_DEF`, `REFL`, `CONJ (REFL,REFL)`, `TAUT \`T\`` are `[]`;
`EXISTS_THM` and `NOT_FORALL_THM` are `[1,2]`.

### What the ladder shows

1. **The instrument discriminates; it is not blanket over-tainting.**
   `new_definition`, `SOME_FLAGS`, `REFL \`x:int64\`` and the ELF decode are all
   axiom-free, while `ARITH_RULE \`2 + 2 = 4\`` already carries `{ETA, SELECT,
   INFINITY}` and `MESON [] \`!P. P ==> P\`` carries `{ETA, SELECT}` without infinity.
   The mask tracks the actual derivation.
2. **The taint is inherited from the substrate, not earned by the proof search.**
   `define_trimmed`, applied three lines after the ELF decode, already carries all
   three, and so does every library lemma the statement is phrased with. In this
   context *any* arithmetic statement carries all three — including `2 + 2 = 4`. The
   uniform `{ETA_AX, SELECT_AX, INFINITY_AX}` in §4 therefore says something true but
   weaker than it looks: the proofs live on top of HOL Light's arithmetic and choice
   substrate, and they add no axiom of their own.
3. **The conjecture that a fixed-width add might avoid choice is neither confirmed nor
   refuted here.** Settling it requires re-deriving the statement in a choice-free
   context (a definitional-footprint analysis), which the mask cannot answer because
   definitional rules contribute `0`. Recorded as open in `TODO.md` XII.F2.6.
4. **One positive datum for O17.** `define_assert_from_elf` — the step that turns the
   shipped bytes into a formal semantics for the routine — is **axiom-free**. The part
   of the trusted base that matters most for machine-code assurance (bytes to
   semantics) is a definitional extension, on this evidence, and not an axiomatic
   assumption. The axioms are inherited from the arithmetic library above it.
