# IX.57b — corrected per-theorem axiom stamps (type definitions included)

- **Date:** 2026-11-24, recorded run `13:00:42Z`–`13:47:19Z`
  (`generated_at` 1795528039 = 2026-11-24T13:47:19Z); wall clock **46 min 37 s**.
- **Branch:** `staging`
- **Trees:** `.bench/dropin` = upstream s2n-bignum at `5e0fc782`; corrected
  measurement build `.bench/hol-light-stamp2` = frozen HOL Light `d30e823` +
  `tools/hol-light/axiom-stamp.diff` **v2**. The frozen `.bench/hol-light` was
  **not modified** (`git -C .bench/hol-light status --short --untracked-files=no`
  is empty; HEAD `d30e823`).
- **Old instrument, kept for the negative control:** `.bench/hol-light-stamp`
  (patch v1, sha256 `f1b8a6db45bb053f76b4b51c5731519345bc8d778826bf0ef14df620fde0a6ac`).
- **Patch:** `sha256(tools/hol-light/axiom-stamp.diff)` =
  `a9808c99a0c8222fc256cad9d869b96722ba4a75912a23808076d5008da1285c` (v2, 359
  lines). Applying v2 to the frozen `fusion.ml` reproduces the stamp2 file
  byte-for-byte (`PATCH_REPRODUCES_STAMP2_FUSION`).
- **Result:** **27/27** tasks green under the strict `green()`, every stamp
  `{ETA_AX, SELECT_AX, INFINITY_AX}` (`outside_basic_axioms=0`), and the 27
  inference deltas are **identical** to IX.57 (byte-for-byte set equality).
- **Corrections to IX.57:** appended to `research/receipts/ix57-axiom-stamp.md`
  (append only). This receipt supersedes IX.57's mechanism claims where the two
  differ.

---

## 1. The defect, and what v2 changes

The v1 patch built `new_basic_type_definition` as

```ocaml
let new_basic_type_definition tyname (absname,repname) (Sequent(asl,c)) = ...
  Sequent([], ...), Sequent([], ...)
```

It discarded the witness theorem's mask. A type definition is a conservative
extension **given its existence witness**, but the justification of that witness
is a real dependency: a type whose witness is proved from an injected axiom is
not axiom-free. v1 reported both results as `[]`. §5 (N1) shows the old build
doing exactly that and the new build carrying the injected id in both results.

v2 makes four changes, all inside `fusion.ml`:

1. **Witness propagation.** `new_basic_type_definition tyname (absname,repname)
   (Sequent((asl,c),s))` returns both theorems with mask `s`.
2. **Capacity guard.** The mask is an OCaml `int`; `axiom_capacity () =
   Sys.word_size - 2` (62 on this 64-bit runtime). `register_axiom` fails with a
   named message before allocating id `capacity + 1`; it never wraps.
3. **Complete canonical descriptions.** Each registry entry is the axiom's
   conclusion serialised fully typed (`name:type` on every variable and
   constant, fully parenthesised), with no truncation, plus a 32-bit FNV-1a
   digest of that text. A stamp entry is `id:<n>#<digest>=|- <canonical text>`,
   so it resolves to exactly one registry entry and can be checked against it.
4. **Statement-level footprint.** `footprint_of : thm -> (string * string) list`
   lists the constants and type constructors occurring in the conclusion, each
   mapped to the `new_basic_definition` / `new_basic_type_definition` that
   introduced it when the kernel registry knows one. It is labelled everywhere
   as **statement-level footprint, not the transitive derivation closure**.

### New exports

```ocaml
val axiom_registry : unit -> (int * string * string) list  (* id, digest, canonical *)
val axiom_capacity : unit -> int
val stamp_of : thm -> int list
val stamp_string : thm -> string
val footprint_of : thm -> (string * string) list
val inference_count : unit -> int
```

`stamp_string` now returns the full entries; the old 80-character truncation and
the whitespace-collapsing pass are gone. The canonical form is the kernel's own
structural syntax (there is no user-facing printer this early in `fusion.ml`),
so it is deterministic and fully typed, but it is not the concrete surface
syntax and its spelling is not promised across HOL Light versions.

---

## 2. Rule-by-rule mask table

Every constructor of `Sequent` in `fusion.ml` is listed. "Parents" means
theorem arguments whose masks are consulted.

| rule | parents | result mask | why |
|---|---|---|---|
| `REFL` | none | `0` | primitive schema; invokes no `new_axiom` |
| `BETA` | none | `0` | primitive schema; no axiom |
| `ASSUME` | none | `0` | dependency is the hypothesis list, which is unchanged |
| `new_axiom` | none | `1 lsl (id-1)` | allocates its own bit and registry entry |
| `new_basic_definition` | none (term argument) | `0` | definitional equation; no axiom, no theorem parent |
| `new_basic_type_definition` | witness `s` | **`s`** (both results) | conservative **given** the witness; v1 dropped `s` |
| `TRANS` | 2 | `s1 lor s2` | union |
| `MK_COMB` | 2 | `s1 lor s2` | union |
| `ABS` | 1 | `s` | copy |
| `EQ_MP` | 2 | `s1 lor s2` | union |
| `DEDUCT_ANTISYM_RULE` | 2 | `s1 lor s2` | union |
| `INST_TYPE` | 1 | `s` | copy |
| `INST` | 1 | `s` | copy |
| `dest_thm`, `hyp`, `concl` | — | no theorem produced | destructors |
| `axioms ()` | — | stored masks unchanged | no new theorem |

The rules that contribute `0` do so **by definition**, not by omission:

- `REFL`, `BETA`: primitive schemata that add no assumption and no axiom.
- `ASSUME`: the theorem's conclusion is its own hypothesis; the stamp records
  *axioms*, not assumptions, and the hypothesis list is already the record.
- `new_basic_definition`: takes a **term**, so it has no theorem parent; the
  defining equation is a conservative extension and introduces no axiom. Its
  introduced constant is recorded for the footprint, not for the mask.
- `new_axiom`: contributes a bit rather than a zero, and the bit is exactly its
  own id.

`new_basic_type_definition` is the only rule whose *theorem* argument was being
ignored; that was the defect and it is closed.

---

## 3. The build

```
$ cd .bench/hol-light-stamp2 && make HOLLIGHT_USE_MODULE=1 hol_lib.cmxa hol.sh
        HOLLIGHT_DIR="`pwd`" ocaml inline_load.ml hol_lib.ml hol_lib_inlined.ml -omit-prelude
        OCAMLRUNPARAM=l=1000000000 ocamlopt.byte -verbose -c ...
+ camlp5r pa_lexer.cmo pa_extend.cmo q_MLast.cmo -I . pa_j.cmo 'hol_lib_inlined.ml' > /tmp/ocamlpp4d426e
        ocamlfind ocamlopt -package zarith -a -o hol_lib.cmxa bignum.cmx hol_loader.cmx hol_lib.cmx
...
exit=0
```

The build was run with the §7 environment (`.bench/bin` on `PATH`,
`CAML_LD_LIBRARY_PATH` into the frozen `_opam`). Artefacts:

```
$ ls -la hol_lib.cmxa hol.sh
-rw-r--r-- 1 one one 3353 2026-11-24 12:41 hol_lib.cmxa
-rwxr-xr-x 1 one one 1808 2026-11-24 12:41 hol.sh
```

The recipe is exactly `rsync -a --exclude=_opam .bench/hol-light/
.bench/hol-light-stamp2/`, `ln -s <frozen>/_opam .bench/hol-light-stamp2/_opam`,
`patch -p1 --forward < ../../tools/hol-light/axiom-stamp.diff`, then the `make`
line. Only `fusion.ml` differs from the frozen tree.

`grep -rn "new_basic_type_definition" -A12 tools/hol-light/axiom-stamp.diff`
(the decisive hunk):

```
330:-  let new_basic_type_definition tyname (absname,repname) (Sequent(asl,c)) =
331:+  let new_basic_type_definition tyname (absname,repname) (Sequent((asl,c),s)) =
332-+    bump_inferences ();
...
349-     let a = Var("a",aty) and r = Var("r",rty) in
350--    Sequent([],safe_mk_eq (Comb(abs,mk_comb(rep,a))) a),
351--    Sequent([],safe_mk_eq (Comb(P,r))
352--                          (safe_mk_eq (mk_comb(rep,mk_comb(abs,r))) r))
353-+    (* The existence witness is a real dependency: both results carry it. *)
354-+    Sequent(([],safe_mk_eq (Comb(abs,mk_comb(rep,a))) a),s),
355-+    Sequent(([],safe_mk_eq (Comb(P,r))
356-+                          (safe_mk_eq (mk_comb(rep,mk_comb(abs,r))) r)),s)
```

---

## 4. Mandatory controls

Each control is a scratch `.ml` file built with
`.bench/hol-light-stamp2/hol.sh inline-load / compile / link` (same path as
`build-proof.sh`) and run directly. The N1 source is identical for both
instruments, so the only variable is the kernel.

### N1 — the type-definition defect, old and new

Source: inject `QXA = new_axiom \`?n:num. n = n + 1\``, then
`new_basic_type_definition "qxtype" ("qx_abs","qx_rep") QXA`.

**Old instrument (`.bench/hol-light-stamp`, patch v1)** — verbatim:

```
N1_QXA_THM |- exists n. n = n + 1
N1_QXA_IDS [4]
N1_ABS_REP_THM |- qx_abs (qx_rep a) = a
N1_ABS_REP_IDS []
N1_ABS_REP_STAMP
N1_REP_ABS_THM |- (exists) r <=> qx_rep (qx_abs r) = r
N1_REP_ABS_IDS []
N1_REP_ABS_STAMP
N1_PLAIN_DEF_IDS []
N1_DONE
```

`N1_ABS_REP_IDS []` and `N1_REP_ABS_IDS []` are the defect: the injected
axiom's id 4 is lost.

**New instrument (`.bench/hol-light-stamp2`, patch v2)** — verbatim:

```
N1_QXA_THM |- exists n. n = n + 1
N1_QXA_IDS [4]
N1_ABS_REP_THM |- qx_abs (qx_rep a) = a
N1_ABS_REP_IDS [4]
N1_ABS_REP_STAMP id:4#fc9f18d9=|- (?:(fun (fun num bool) bool) (\n:num. ((=:(fun num (fun num bool)) n:num) ((+:(fun num (fun num num)) n:num) (NUMERAL:(fun num num) (BIT1:(fun num num) _0:num))))))
N1_REP_ABS_THM |- (exists) r <=> qx_rep (qx_abs r) = r
N1_REP_ABS_IDS [4]
N1_REP_ABS_STAMP id:4#fc9f18d9=|- (?:(fun (fun num bool) bool) (\n:num. ((=:(fun num (fun num bool)) n:num) ((+:(fun num (fun num num)) n:num) (NUMERAL:(fun num num) (BIT1:(fun num num) _0:num))))))
N1_PLAIN_DEF_IDS []
N1_DONE
```

Both results carry the witness mask `[4]`, and the stamp entry resolves to the
single registry entry for id 4 (digest `fc9f18d9`). `N1_PLAIN_DEF_IDS []` is the
control that the fix did not over-taint: a term-only definition still has no
theorem parent.

### N2 — capacity guard

Verbatim:

```
N2_WORD_SIZE 64
N2_CAPACITY 62
N2_REGISTRY_START 3
N2_REGISTRY_FULL 62
N2_LAST_IDS [62]
N2_GUARD_FAILED axiom_stamp: capacity exhausted: the axiom mask holds at most 62 axiom ids on this platform (Sys.word_size=64); refusing to allocate axiom id 63
N2_REGISTRY_AFTER 62
N2_AFTER_REFL |- x <=> x ids=[]
N2_AFTER_ARITH |- 2 + 2 = 4 ids=[1,2,3]
N2_AFTER_PROVE |- true /\ true
N2_DONE
```

The guard fires **before** the 63rd id is allocated (the registry stays at 62),
names the limit and the platform word size, and does not wrap. Afterwards
`REFL`, `ARITH_RULE` and `prove` still work and their masks are unchanged
(`REFL` empty, an arithmetic theorem still the three basic axioms). Note
`N2_LAST_IDS [62]`: id 62 is bit 61, which is the highest non-negative bit of an
OCaml `int` on this platform.

### C1 — zero taint

```
C1_REGISTRY_SIZE 3
C1_REFL_THM |- x <=> x
C1_REFL_IDS []
C1_REFL_STAMP []
C1_NAME ETA_AX ids=[1] text=id:1#8b87f0cf=|- (!:(fun (fun (fun 'A 'B) bool) bool) (\t:(fun 'A 'B). ((=:(fun (fun 'A 'B) (fun (fun 'A 'B) bool)) (\x:'A. (t:(fun 'A 'B) x:'A))) t:(fun 'A 'B))))
C1_NAME SELECT_AX ids=[2] text=id:2#397b0b82=|- (!:(fun (fun (fun 'A bool) bool) bool) (\P:(fun 'A bool). (!:(fun (fun 'A bool) bool) (\x:'A. ((==>:(fun bool (fun bool bool)) (P:(fun 'A bool) x:'A)) (P:(fun 'A bool) (@:(fun (fun 'A bool) 'A) P:(fun 'A bool))))))))
C1_NAME INFINITY_AX ids=[3] text=id:3#3af16d5f=|- (?:(fun (fun (fun ind ind) bool) bool) (\f:(fun ind ind). ((/\:(fun bool (fun bool bool)) (ONE_ONE:(fun (fun ind ind) bool) f:(fun ind ind))) (~:(fun bool bool) (ONTO:(fun (fun ind ind) bool) f:(fun ind ind))))))
C1_DONE
```

The types are now present on every constant and variable and nothing is
truncated. Digest and canonical text are the registry's, not a transcription.

### C2 — injected axiom propagates, `check_axioms` still bites

```
C2_REGISTRY_BEFORE 3
C2_QXA_THM |- (forall n. n = n + 1) <=> false
C2_QXA_IDS [4]
C2_QXA_STAMP id:4#2e6994be=|- ((=:(fun bool (fun bool bool)) (!:(fun (fun num bool) bool) (\n:num. ((=:(fun num (fun num bool)) n:num) ((+:(fun num (fun num num)) n:num) (NUMERAL:(fun num num) (BIT1:(fun num num) _0:num))))))) F:bool)
C2_DERIVED_THM forall n. n = n + 1
C2_DERIVED_IDS [4]
C2_DERIVED_STAMP id:4#2e6994be=|- ((=:(fun bool (fun bool bool)) (!:(fun (fun num bool) bool) (\n:num. ((=:(fun num (fun num bool)) n:num) ((+:(fun num (fun num num)) n:num) (NUMERAL:(fun num num) (BIT1:(fun num num) _0:num))))))) F:bool)
C2_REGISTRY_AFTER 4
C2_CHECK_AXIOMS FAILED check_axioms: [|- (forall n. n = n + 1) <=> false]
C2_DONE
```

### C3 — false statements remain unprovable

```
C3_ARITH_RAISED Failure("ARITH_RULE `1 = 2`: REAL_ARITH `~true`: linear_ineqs: no contradiction")
C3_REFL_RAISED Failure("REFL_TAC")
C3_EQ_MP_RAISED Failure("EQ_MP")
C3_TRANS_RAISED Failure("TRANS")
C3_SUM_RAISED Failure("ARITH_RULE `2 + 2 = 5`: REAL_ARITH `~true`: linear_ineqs: no contradiction")
C3_STILL_ALIVE
```

### C4 — equivalence with the frozen build, and with IX.57

Accepted set, frozen (IX.56) vs corrected stamp build:

```
$ wc -l /tmp/s56.txt /tmp/s57b.txt
  27 /tmp/s56.txt
  27 /tmp/s57b.txt
  54 total
$ diff /tmp/s56.txt /tmp/s57b.txt && echo SETS_IDENTICAL
SETS_IDENTICAL
```

Inference deltas, IX.57 (v1) vs IX.57b (v2):

```
$ wc -l /tmp/i57.txt /tmp/i57b.txt
  27 /tmp/i57.txt
  27 /tmp/i57b.txt
  54 total
$ diff /tmp/i57.txt /tmp/i57b.txt && echo INFERENCES_IDENTICAL
INFERENCES_IDENTICAL
```

Axiom content:

```
stamped=27 missing_stamp=0 outside_basic_axioms=0 total=27
```

so the corrected kernel accepts exactly the same 27 theorems, with the same
per-theorem inference counts and the same axiom sets as v1. **No number moved**
except wall time: `SUM57=1386 SUM57b=1391 MAXABS=2` (largest difference 2 s, at
`arm p521/add_p521`), which is run-to-run noise at the 1-second resolution of
`Running time:`.

---

## 5. Gradient re-check (IX.57 §10)

`research/receipts/ix57/probe_stamps.ml` (sha256
`302e77d127c18eb818f1acbf0c14a578b4aa52119f2b41c5d107833f4dffb793`), built and
run against `.bench/hol-light-stamp2` with the receipt's own recipe. Verbatim:

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

The ladder is identical to IX.57 §10, including `MESON [] \`!P. P ==> P\`` at
`[1,2]` and the ELF decode at `[]`. The corrected kernel did not change the
gradient; it changed the type-definition case, which this probe does not
exercise.

---

## 6. The empirical run

Two invocations are recorded honestly. The first, launched without `.bench/bin`
on `PATH`, failed all 27 at assembly:

```
/bin/sh: line 1: aarch64-linux-gnu-as: command not found
make: *** [Makefile:441: curve25519/bignum_add_p25519.o] Error 127
OK 0/27
```

That is an environment error (the host has no aarch64 binutils; `.bench/bin`
holds the shims), not a kernel result. Nothing from it is used below. The
recorded run re-ran every task with the correct environment:

```
cargo run --release --offline --bin dropin -- sweep --arch all --force --stamp \
  --hol .bench/hol-light-stamp2 --json research/receipts/ix57b-axiom-stamp.json
```

`--force` deletes each `.correct` and passes `make -B`; `-j1`; one task at a
time. All 27 printed `exit=0 theorem=True exception=False`, `OK 27/27`, and the
stamp validator accepted every record (no `stamp invalid:` line was printed).
The per-theorem table below is generated directly from the JSON by
`dropin stamp-table`; the footprint and registry blocks are its own output, not a
transcription.

## Per-theorem axiom stamp

| arch | task | axioms used | inferences | seconds |
|---|---|---|---:|---:|
| arm | curve25519/add_p25519 | {ETA_AX, SELECT_AX, INFINITY_AX} | 170225275 | 46.000 |
| arm | p256/add_p256 | {ETA_AX, SELECT_AX, INFINITY_AX} | 170931670 | 47.000 |
| arm | secp256k1/add_p256k1 | {ETA_AX, SELECT_AX, INFINITY_AX} | 157240305 | 46.000 |
| arm | p384/add_p384 | {ETA_AX, SELECT_AX, INFINITY_AX} | 349139623 | 74.000 |
| arm | p521/add_p521 | {ETA_AX, SELECT_AX, INFINITY_AX} | 681707202 | 122.000 |
| arm | sm2/add_sm2 | {ETA_AX, SELECT_AX, INFINITY_AX} | 159985379 | 46.000 |
| arm | curve25519/sub_p25519 | {ETA_AX, SELECT_AX, INFINITY_AX} | 110957174 | 39.000 |
| arm | p256/sub_p256 | {ETA_AX, SELECT_AX, INFINITY_AX} | 103417925 | 38.000 |
| arm | secp256k1/sub_p256k1 | {ETA_AX, SELECT_AX, INFINITY_AX} | 110121226 | 39.000 |
| arm | p384/sub_p384 | {ETA_AX, SELECT_AX, INFINITY_AX} | 214410496 | 55.000 |
| arm | p521/sub_p521 | {ETA_AX, SELECT_AX, INFINITY_AX} | 221845891 | 59.000 |
| arm | sm2/sub_sm2 | {ETA_AX, SELECT_AX, INFINITY_AX} | 107504057 | 39.000 |
| arm | curve25519/neg_p25519 | {ETA_AX, SELECT_AX, INFINITY_AX} | 35457948 | 29.000 |
| arm | p256/neg_p256 | {ETA_AX, SELECT_AX, INFINITY_AX} | 45739252 | 31.000 |
| arm | secp256k1/neg_p256k1 | {ETA_AX, SELECT_AX, INFINITY_AX} | 45424790 | 30.000 |
| arm | p384/neg_p384 | {ETA_AX, SELECT_AX, INFINITY_AX} | 86799846 | 37.000 |
| arm | p521/neg_p521 | {ETA_AX, SELECT_AX, INFINITY_AX} | 123668935 | 44.000 |
| arm | sm2/neg_sm2 | {ETA_AX, SELECT_AX, INFINITY_AX} | 46665222 | 31.000 |
| x86 | curve25519/add_p25519 | {ETA_AX, SELECT_AX, INFINITY_AX} | 160584653 | 70.000 |
| x86 | secp256k1/add_p256k1 | {ETA_AX, SELECT_AX, INFINITY_AX} | 166694408 | 71.000 |
| x86 | sm2/add_sm2 | {ETA_AX, SELECT_AX, INFINITY_AX} | 127361354 | 66.000 |
| x86 | curve25519/sub_p25519 | {ETA_AX, SELECT_AX, INFINITY_AX} | 61298759 | 57.000 |
| x86 | secp256k1/sub_p256k1 | {ETA_AX, SELECT_AX, INFINITY_AX} | 111312547 | 64.000 |
| x86 | sm2/sub_sm2 | {ETA_AX, SELECT_AX, INFINITY_AX} | 67342905 | 57.000 |
| x86 | curve25519/neg_p25519 | {ETA_AX, SELECT_AX, INFINITY_AX} | 19973249 | 51.000 |
| x86 | secp256k1/neg_p256k1 | {ETA_AX, SELECT_AX, INFINITY_AX} | 23422025 | 52.000 |
| x86 | sm2/neg_sm2 | {ETA_AX, SELECT_AX, INFINITY_AX} | 18230539 | 51.000 |

## Axiom registry (canonical typed conclusions)

| id | name | digest | conclusion |
|---|---|---|---|
| 1 | ETA_AX | `8b87f0cf` | `(!:(fun (fun (fun 'A 'B) bool) bool) (\t:(fun 'A 'B). ((=:(fun (fun 'A 'B) (fun (fun 'A 'B) bool)) (\x:'A. (t:(fun 'A 'B) x:'A))) t:(fun 'A 'B))))` |
| 2 | SELECT_AX | `397b0b82` | `(!:(fun (fun (fun 'A bool) bool) bool) (\P:(fun 'A bool). (!:(fun (fun 'A bool) bool) (\x:'A. ((==>:(fun bool (fun bool bool)) (P:(fun 'A bool) x:'A)) (P:(fun 'A bool) (@:(fun (fun 'A bool) 'A) P:(fun 'A bool))))))))` |
| 3 | INFINITY_AX | `3af16d5f` | `(?:(fun (fun (fun ind ind) bool) bool) (\f:(fun ind ind). ((/\:(fun bool (fun bool bool)) (ONE_ONE:(fun (fun ind ind) bool) f:(fun ind ind))) (~:(fun bool bool) (ONTO:(fun (fun ind ind) bool) f:(fun ind ind))))))` |

## Statement-level footprint (not the transitive derivation closure)

Constants and type constructors occurring in the 27 conclusions, each
mapped to the definition or type definition that introduced it when the
kernel registry knows it. This is **statement-level footprint, not the
transitive derivation closure**: it does not follow the proof DAG and it
says nothing about which definitions a derivation *used*.

| symbol | introducer |
|---|---|
| `const:!:fun(fun(num,bool),bool)` | new_basic_definition(!) |
| `const:!:fun(fun(word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))),bool),bool)` | new_basic_definition(!) |
| `const:*:fun(num,fun(num,num))` | new_basic_definition(*) |
| `const:+:fun(num,fun(num,num))` | new_basic_definition(+) |
| `const:,,:fun(fun(armstate,fun(armstate,bool)),fun(fun(armstate,fun(armstate,bool)),fun(armstate,fun(armstate,bool))))` | new_basic_definition(,,) |
| `const:,,:fun(fun(x86state,fun(x86state,bool)),fun(fun(x86state,fun(x86state,bool)),fun(x86state,fun(x86state,bool))))` | new_basic_definition(,,) |
| `const:,:fun(word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))),fun(num,prod(word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))),num)))` | new_basic_definition(,) |
| `const:-:fun(num,fun(num,num))` | new_basic_definition(-) |
| `const:/\:fun(bool,fun(bool,bool))` | new_basic_definition(/\) |
| `const::>:fun(component(armstate,fun(word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))),word(tybit0(tybit0(tybit0(1)))))),fun(component(fun(word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))),word(tybit0(tybit0(tybit0(1))))),num),component(armstate,num)))` | new_basic_definition(:>) |
| `const::>:fun(component(x86state,fun(word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))),word(tybit0(tybit0(tybit0(1)))))),fun(component(fun(word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))),word(tybit0(tybit0(tybit0(1))))),num),component(x86state,num)))` | new_basic_definition(:>) |
| `const:<:fun(num,fun(num,bool))` | new_basic_definition(<) |
| `const:<=:fun(num,fun(num,bool))` | new_basic_definition(<=) |
| `const:=:fun(int,fun(int,bool))` | primitive or unregistered |
| `const:=:fun(num,fun(num,bool))` | primitive or unregistered |
| `const:=:fun(word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))),fun(word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))),bool))` | primitive or unregistered |
| `const:==>:fun(bool,fun(bool,bool))` | new_basic_definition(==>) |
| `const:BIT0:fun(num,num)` | new_basic_definition(BIT0) |
| `const:BIT1:fun(num,num)` | new_basic_definition(BIT1) |
| `const:BUTLAST:fun(list(word(tybit0(tybit0(tybit0(1))))),list(word(tybit0(tybit0(tybit0(1))))))` | new_basic_definition(BUTLAST) |
| `const:CONS:fun(component(armstate,list(uarch_event)),fun(list(component(armstate,list(uarch_event))),list(component(armstate,list(uarch_event)))))` | new_basic_definition(CONS) |
| `const:CONS:fun(component(armstate,num),fun(list(component(armstate,num)),list(component(armstate,num))))` | new_basic_definition(CONS) |
| `const:CONS:fun(component(armstate,word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1)))))))),fun(list(component(armstate,word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))))),list(component(armstate,word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1)))))))))))` | new_basic_definition(CONS) |
| `const:CONS:fun(component(x86state,list(uarch_event)),fun(list(component(x86state,list(uarch_event))),list(component(x86state,list(uarch_event)))))` | new_basic_definition(CONS) |
| `const:CONS:fun(component(x86state,num),fun(list(component(x86state,num)),list(component(x86state,num))))` | new_basic_definition(CONS) |
| `const:CONS:fun(component(x86state,word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1)))))))),fun(list(component(x86state,word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))))),list(component(x86state,word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1)))))))))))` | new_basic_definition(CONS) |
| `const:CONS:fun(word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))),fun(list(word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1)))))))),list(word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))))))` | new_basic_definition(CONS) |
| `const:C_ARGUMENTS:fun(list(word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1)))))))),fun(armstate,bool))` | new_basic_definition(C_ARGUMENTS) |
| `const:C_ARGUMENTS:fun(list(word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1)))))))),fun(x86state,bool))` | new_basic_definition(C_ARGUMENTS) |
| `const:MAYCHANGE:fun(list(component(armstate,bool)),fun(armstate,fun(armstate,bool)))` | new_basic_definition(MAYCHANGE) |
| `const:MAYCHANGE:fun(list(component(armstate,list(uarch_event))),fun(armstate,fun(armstate,bool)))` | new_basic_definition(MAYCHANGE) |
| `const:MAYCHANGE:fun(list(component(armstate,num)),fun(armstate,fun(armstate,bool)))` | new_basic_definition(MAYCHANGE) |
| `const:MAYCHANGE:fun(list(component(armstate,word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))))),fun(armstate,fun(armstate,bool)))` | new_basic_definition(MAYCHANGE) |
| `const:MAYCHANGE:fun(list(component(x86state,bool)),fun(x86state,fun(x86state,bool)))` | new_basic_definition(MAYCHANGE) |
| `const:MAYCHANGE:fun(list(component(x86state,list(uarch_event))),fun(x86state,fun(x86state,bool)))` | new_basic_definition(MAYCHANGE) |
| `const:MAYCHANGE:fun(list(component(x86state,num)),fun(x86state,fun(x86state,bool)))` | new_basic_definition(MAYCHANGE) |
| `const:MAYCHANGE:fun(list(component(x86state,word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))))),fun(x86state,fun(x86state,bool)))` | new_basic_definition(MAYCHANGE) |
| `const:MOD:fun(num,fun(num,num))` | new_basic_definition(MOD) |
| `const:NIL:list(component(armstate,list(uarch_event)))` | new_basic_definition(NIL) |
| `const:NIL:list(component(armstate,num))` | new_basic_definition(NIL) |
| `const:NIL:list(component(armstate,word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1)))))))))` | new_basic_definition(NIL) |
| `const:NIL:list(component(x86state,list(uarch_event)))` | new_basic_definition(NIL) |
| `const:NIL:list(component(x86state,num))` | new_basic_definition(NIL) |
| `const:NIL:list(component(x86state,word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1)))))))))` | new_basic_definition(NIL) |
| `const:NIL:list(word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))))` | new_basic_definition(NIL) |
| `const:NUMERAL:fun(num,num)` | new_basic_definition(NUMERAL) |
| `const:PC:component(armstate,word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))))` | new_basic_definition(PC) |
| `const:R10:component(x86state,word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))))` | new_basic_definition(R10) |
| `const:R11:component(x86state,word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))))` | new_basic_definition(R11) |
| `const:R8:component(x86state,word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))))` | new_basic_definition(R8) |
| `const:R9:component(x86state,word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))))` | new_basic_definition(R9) |
| `const:RAX:component(x86state,word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))))` | new_basic_definition(RAX) |
| `const:RCX:component(x86state,word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))))` | new_basic_definition(RCX) |
| `const:RDX:component(x86state,word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))))` | new_basic_definition(RDX) |
| `const:RIP:component(x86state,word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))))` | new_basic_definition(RIP) |
| `const:RSI:component(x86state,word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))))` | new_basic_definition(RSI) |
| `const:SOME_FLAGS:list(component(armstate,bool))` | new_basic_definition(SOME_FLAGS) |
| `const:SOME_FLAGS:list(component(x86state,bool))` | new_basic_definition(SOME_FLAGS) |
| `const:X10:component(armstate,word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))))` | new_basic_definition(X10) |
| `const:X11:component(armstate,word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))))` | new_basic_definition(X11) |
| `const:X12:component(armstate,word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))))` | new_basic_definition(X12) |
| `const:X13:component(armstate,word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))))` | new_basic_definition(X13) |
| `const:X2:component(armstate,word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))))` | new_basic_definition(X2) |
| `const:X3:component(armstate,word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))))` | new_basic_definition(X3) |
| `const:X4:component(armstate,word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))))` | new_basic_definition(X4) |
| `const:X5:component(armstate,word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))))` | new_basic_definition(X5) |
| `const:X6:component(armstate,word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))))` | new_basic_definition(X6) |
| `const:X7:component(armstate,word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))))` | new_basic_definition(X7) |
| `const:X8:component(armstate,word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))))` | new_basic_definition(X8) |
| `const:X9:component(armstate,word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))))` | new_basic_definition(X9) |
| `const:_0:num` | new_basic_definition(_0) |
| `const:aligned_bytes_loaded:fun(armstate,fun(word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))),fun(list(word(tybit0(tybit0(tybit0(1))))),bool)))` | new_basic_definition(aligned_bytes_loaded) |
| `const:arm:fun(armstate,fun(armstate,bool))` | new_basic_definition(arm) |
| `const:bignum:fun(prod(word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))),num),component(fun(word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))),word(tybit0(tybit0(tybit0(1))))),num))` | new_basic_definition(bignum) |
| `const:bignum_add_p25519_mc:list(word(tybit0(tybit0(tybit0(1)))))` | new_basic_definition(bignum_add_p25519_mc) |
| `const:bignum_add_p25519_tmc:list(word(tybit0(tybit0(tybit0(1)))))` | new_basic_definition(bignum_add_p25519_tmc) |
| `const:bignum_add_p256_mc:list(word(tybit0(tybit0(tybit0(1)))))` | new_basic_definition(bignum_add_p256_mc) |
| `const:bignum_add_p256k1_mc:list(word(tybit0(tybit0(tybit0(1)))))` | new_basic_definition(bignum_add_p256k1_mc) |
| `const:bignum_add_p256k1_tmc:list(word(tybit0(tybit0(tybit0(1)))))` | new_basic_definition(bignum_add_p256k1_tmc) |
| `const:bignum_add_p384_mc:list(word(tybit0(tybit0(tybit0(1)))))` | new_basic_definition(bignum_add_p384_mc) |
| `const:bignum_add_p521_mc:list(word(tybit0(tybit0(tybit0(1)))))` | new_basic_definition(bignum_add_p521_mc) |
| `const:bignum_add_sm2_mc:list(word(tybit0(tybit0(tybit0(1)))))` | new_basic_definition(bignum_add_sm2_mc) |
| `const:bignum_add_sm2_tmc:list(word(tybit0(tybit0(tybit0(1)))))` | new_basic_definition(bignum_add_sm2_tmc) |
| `const:bignum_from_memory:fun(prod(word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))),num),fun(armstate,num))` | new_basic_definition(bignum_from_memory) |
| `const:bignum_from_memory:fun(prod(word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))),num),fun(x86state,num))` | new_basic_definition(bignum_from_memory) |
| `const:bignum_neg_p25519_mc:list(word(tybit0(tybit0(tybit0(1)))))` | new_basic_definition(bignum_neg_p25519_mc) |
| `const:bignum_neg_p25519_tmc:list(word(tybit0(tybit0(tybit0(1)))))` | new_basic_definition(bignum_neg_p25519_tmc) |
| `const:bignum_neg_p256_mc:list(word(tybit0(tybit0(tybit0(1)))))` | new_basic_definition(bignum_neg_p256_mc) |
| `const:bignum_neg_p256k1_mc:list(word(tybit0(tybit0(tybit0(1)))))` | new_basic_definition(bignum_neg_p256k1_mc) |
| `const:bignum_neg_p256k1_tmc:list(word(tybit0(tybit0(tybit0(1)))))` | new_basic_definition(bignum_neg_p256k1_tmc) |
| `const:bignum_neg_p384_mc:list(word(tybit0(tybit0(tybit0(1)))))` | new_basic_definition(bignum_neg_p384_mc) |
| `const:bignum_neg_p521_mc:list(word(tybit0(tybit0(tybit0(1)))))` | new_basic_definition(bignum_neg_p521_mc) |
| `const:bignum_neg_sm2_mc:list(word(tybit0(tybit0(tybit0(1)))))` | new_basic_definition(bignum_neg_sm2_mc) |
| `const:bignum_neg_sm2_tmc:list(word(tybit0(tybit0(tybit0(1)))))` | new_basic_definition(bignum_neg_sm2_tmc) |
| `const:bignum_sub_p25519_mc:list(word(tybit0(tybit0(tybit0(1)))))` | new_basic_definition(bignum_sub_p25519_mc) |
| `const:bignum_sub_p25519_tmc:list(word(tybit0(tybit0(tybit0(1)))))` | new_basic_definition(bignum_sub_p25519_tmc) |
| `const:bignum_sub_p256_mc:list(word(tybit0(tybit0(tybit0(1)))))` | new_basic_definition(bignum_sub_p256_mc) |
| `const:bignum_sub_p256k1_mc:list(word(tybit0(tybit0(tybit0(1)))))` | new_basic_definition(bignum_sub_p256k1_mc) |
| `const:bignum_sub_p256k1_tmc:list(word(tybit0(tybit0(tybit0(1)))))` | new_basic_definition(bignum_sub_p256k1_tmc) |
| `const:bignum_sub_p384_mc:list(word(tybit0(tybit0(tybit0(1)))))` | new_basic_definition(bignum_sub_p384_mc) |
| `const:bignum_sub_p521_mc:list(word(tybit0(tybit0(tybit0(1)))))` | new_basic_definition(bignum_sub_p521_mc) |
| `const:bignum_sub_sm2_mc:list(word(tybit0(tybit0(tybit0(1)))))` | new_basic_definition(bignum_sub_sm2_mc) |
| `const:bignum_sub_sm2_tmc:list(word(tybit0(tybit0(tybit0(1)))))` | new_basic_definition(bignum_sub_sm2_tmc) |
| `const:bytes_loaded:fun(x86state,fun(word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))),fun(list(word(tybit0(tybit0(tybit0(1))))),bool)))` | new_basic_definition(bytes_loaded) |
| `const:ensures:fun(fun(armstate,fun(armstate,bool)),fun(fun(armstate,bool),fun(fun(armstate,bool),fun(fun(armstate,fun(armstate,bool)),bool))))` | new_basic_definition(ensures) |
| `const:ensures:fun(fun(x86state,fun(x86state,bool)),fun(fun(x86state,bool),fun(fun(x86state,bool),fun(fun(x86state,fun(x86state,bool)),bool))))` | new_basic_definition(ensures) |
| `const:events:component(armstate,list(uarch_event))` | new_basic_definition(events) |
| `const:events:component(x86state,list(uarch_event))` | new_basic_definition(events) |
| `const:int_of_num:fun(num,int)` | new_basic_definition(int_of_num) |
| `const:int_sub:fun(int,fun(int,int))` | new_basic_definition(int_sub) |
| `const:memory:component(armstate,fun(word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))),word(tybit0(tybit0(tybit0(1))))))` | new_basic_definition(memory) |
| `const:memory:component(x86state,fun(word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))),word(tybit0(tybit0(tybit0(1))))))` | new_basic_definition(memory) |
| `const:nonoverlapping:fun(prod(word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))),num),fun(prod(word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))),num),bool))` | new_basic_definition(nonoverlapping) |
| `const:p_25519:num` | new_basic_definition(p_25519) |
| `const:p_256:num` | new_basic_definition(p_256) |
| `const:p_256k1:num` | new_basic_definition(p_256k1) |
| `const:p_384:num` | new_basic_definition(p_384) |
| `const:p_521:num` | new_basic_definition(p_521) |
| `const:p_sm2:num` | new_basic_definition(p_sm2) |
| `const:read:fun(component(armstate,word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1)))))))),fun(armstate,word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1)))))))))` | new_basic_definition(read) |
| `const:read:fun(component(x86state,word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1)))))))),fun(x86state,word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1)))))))))` | new_basic_definition(read) |
| `const:rem:fun(int,fun(int,int))` | new_basic_definition(rem) |
| `const:word:fun(num,word(tybit0(tybit0(tybit0(tybit0(tybit0(tybit0(1))))))))` | new_basic_definition(word) |
| `const:x86:fun(x86state,fun(x86state,bool))` | new_basic_definition(x86) |
| `tycon:1/0` | new_basic_type_definition(1) |
| `tycon:armstate/0` | new_basic_type_definition(armstate) |
| `tycon:bool/0` | primitive or unregistered |
| `tycon:component/2` | new_basic_type_definition(component) |
| `tycon:fun/2` | primitive or unregistered |
| `tycon:int/0` | new_basic_type_definition(int) |
| `tycon:list/1` | new_basic_type_definition(list) |
| `tycon:num/0` | new_basic_type_definition(num) |
| `tycon:prod/2` | new_basic_type_definition(prod) |
| `tycon:tybit0/1` | new_basic_type_definition(tybit0) |
| `tycon:uarch_event/0` | new_basic_type_definition(uarch_event) |
| `tycon:word/1` | new_basic_type_definition(word) |
| `tycon:x86state/0` | new_basic_type_definition(x86state) |

stamped=27 missing_stamp=0 outside_basic_axioms=0 total=27

---

## 7. What remains unproved

- **Transitive definitional closure is not implemented.** The footprint is
  statement-level only: the symbols in the *conclusion* and the definition that
  introduced each. It does not follow the proof DAG and cannot tell you which
  definitions a derivation *used*. Full dependency closure over the semantic
  environment needs a recorded proof/derivation DAG, which this patch does not
  produce. It is left open in `TODO.md` XIII.B2 and is not faked here.
- **No exported derivation.** The mask is a summary bit per axiom, not a proof
  object. It cannot be replayed by a third party; it is measured by the same
  kernel that made the theorem. Proof-object export and external re-checking
  remain open.
- **Capacity is a fixed 62 bits.** On a 64-bit OCaml runtime the mask is an
  `int`; the guard fails loudly past 62 ids. An unbounded set (bitset/list) was
  not implemented. 62 is far above the three basic axioms and the intended use,
  but it is a bound.
- **Canonical text is kernel-structural, not surface syntax.** It is fully typed
  and deterministic in a fixed build, but its spelling could change with the
  kernel's internal constructors; the digest is an identity tag over that text,
  not a stability promise across HOL Light versions.
- **The digest is 32-bit FNV-1a.** It is an identity/integrity tag for detecting
  mismatched entries, not a cryptographic hash and not collision-resistant
  against an adversary.
- **Ids are session-allocated.** They are assigned in `new_axiom` order. In a
  fresh session ids 1–3 are the basic axioms because those are the first three
  created; a different session order would assign different ids to the same
  conclusions. The receipt resolves names through the run's own `STAMP_NAME`
  lines, not by assuming an order.
- **Nothing new is claimed about theorem truth.** C3 shows the kernel still
  refuses false statements; the stamp is a dependency record, not a truth claim.

---

## 8. Hygiene

```
$ git ls-files '*.py'
(no output; py_count=0)

$ cargo test --offline
running 57 tests
test result: ok. 57 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
... (integration targets: 27, 7, 25, 21, 13, 14, 25, 5, 7, 26, 7, 10, 11 and 1 test, all ok)

$ cargo clippy --offline --all-targets -- -D warnings
    Checking tekmerion v0.1.0 (/mnt/1tb_kingston/Tekmerion)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.03s

$ df -h /mnt/1tb_kingston | tail -1
/dev/nvme1n1p1  1.3T  1.3T   13G 100% /mnt/1tb_kingston
```

13 GB free at the end of the run (floor: 8 GB).

`--force` → `make -B` makes upstream s2n-bignum run its own
`tools/collect-signatures.py` inside the git-ignored `.bench/dropin` copy. No
Tekmerion number in this receipt is produced, parsed or summarised by Python;
log parsing and both tables are Rust (`parse_stamp`, `stamp-table`).

Disk after the run: 13 GB free (line above), above the 8 GB floor.

---

## 9. Patch identities

| item | value |
|---|---|
| v1 patch sha256 | `f1b8a6db45bb053f76b4b51c5731519345bc8d778826bf0ef14df620fde0a6ac` |
| v2 patch sha256 | `a9808c99a0c8222fc256cad9d869b96722ba4a75912a23808076d5008da1285c` |
| frozen `fusion.ml` sha256 | `29544be92d9cc1e6b3b59ba5b210604b9a748366a68b3bcdc2619717d68fd98a` |
| patched `fusion.ml` sha256 (stamp2) | `9c15e5260955fc6f22ad2c845bb7c7287a9580f3669be6e62829b7a9a53a0bfd` |

Pins: s2n-bignum `5e0fc782`, HOL Light `d30e823`, OCaml `5.4.0`, camlp5
`8.04.00`, zarith `1.14`, cadical `2.1.2`. Environment identical to IX.57 §7.
