# ix54 — sealed x86 evaluation inside a proof-free container (O10.2)

Date: 2026-11-24.  Branch: `staging` (unchanged, nothing committed).
This is a measurement record. No answer, emitter, guard, harness or input was tuned.

## 0. Freeze

```
$ python3 -c "import sys; sys.path.insert(0,'tools'); import heldout_guard as g; \
  m=g.load_sealed_manifest('research/receipts/ix54-sealed-manifest.json'); \
  g.verify_freeze(m,'freeze-x86-sealed-v3'); print('FREEZE PASS', m.freeze_commit[:12], len(m.sealed_ids))"
FREEZE PASS 2705f5b07adb 9
```

Manifest `research/receipts/ix54-sealed-manifest.json`, tag `freeze-x86-sealed-v3`,
`freeze_commit = 2705f5b07adb678075f6b31ae57f1515a3f00ae4`.  The same verification
re-ran **inside** the container as part of every emitter invocation (the guard is
fail-closed), so the nine answers below were emitted under `--sealed-eval` with a
live git tag check.

**Re-verification note (added after the run).** The guard was strengthened on
2026-11-24 to refuse unless `git diff <freeze_commit> -- tools` is empty, so the tree
must *be* the frozen code and not merely clean. `tools/` has since moved, so
`verify_freeze(v3)` now refuses at the current HEAD. That is intended: to re-verify this
receipt, check out `2705f5b` first. The recorded run itself is unaffected.

## 1. Step 1 — what the harness actually reads

Read-only inspection of `.bench/combine-answer-and-setup.py`,
`.bench/run-answers.sh`, `.bench/collect-verdicts.py`,
`tools/emit_ref_proof_x86.py`, `tools/heldout_guard.py`.

### 1.1 Does generation or judging read a proof file?

**Judging reads `ml_files/x86/*.ml`, not `s2n-bignum/*/proofs/*.ml`.**  For all nine
tasks `problems.json` gives `inlined_locations` of the form
`["ml_files/x86/bignum_add_p25519.ml", 38482]`.  Call sites in
`combine-answer-and-setup.py`:

| line | code | effect |
|---|---|---|
| 147–148 | `template_file_info = problems[prob_id]["inlined_locations"][0]; file_path, linenum = ...` | resolves the template path |
| 152–160 | `open(file_path, ...)`; `for line in lines[:linenum]: target.write(line)` | reads **only** lines `0 .. linenum-1` for the syntactic check |
| 184–196 | `template_candidates = problems[problem_name]["inlined_locations"]` | groups problems per template |
| 265–267 | `open(template_path, ...)`; `template_lines = list(template_file.readlines())` | reads the template |
| 214–224 | `for line in template_lines[prevline:linenum]: out_file.write(line)` | copies **only** the prefix up to the insertion line |

The files under `ml_files/x86/` are the benchmark templates.  They contain the
human proof *after* the insertion line (e.g. `bignum_add_p25519.ml` has 38616
lines; insertion at 38482), but the harness never consumes a byte at or after
`linenum`.  Nothing in the judge path opens `s2n-bignum/*/proofs/*.ml`.

Verdicts: `collect-verdicts.py` reads only `problems.json` (L53–55) and
`*.judge.txt` (L17–38).  `run-answers.sh` compiles the generated `.ml`, `cd objfiles`
(L35), and runs the executable.

### 1.2 Does the emitter in `--mode answer` read a proof file?

**No, confirmed in code.**  `main()` (L311–339) always calls `build(op, task, policy)`
(L329).  `build()` (L217–248) reads only:

- `BENCH/problems.json` (L123, via `load_problems()`);
- `S2N.glob(f"x86/*/bignum_{op}_{family}.o")` and the guard's `check_object` (L132–137);
- `disasm_steps.disassemble(str(obj), arch="x86")` (L228), which shells out to
  `objdump` (`disasm_steps.py` L217) and opens the ELF (L69);
- `w2_witnesses.parse_spec(query)` (L230), pure regex over the query string.

The two proof-reading functions — `modulus_definition` (L255–264) and
`machine_code_phrase` (L267–279), each `proof.read_text(...)` — are called **only**
from `dev_script()` (L282–308), which is reached only when `--mode dev` selects
`dev_script(info, policy)` at L330–331.  In `--mode answer` the expression is
`emit_ref_proof.inlined_answer(info["tactic"])`, which touches no file.
`emit_ref_proof.py` itself reads only its own bundled `tools/bridge.ml` /
`csat_lib.ml` sources when building a *dev* script (L244–246), not in answer mode.

### 1.3 Does judging need `s2n-bignum/` beyond the nine objects?

**No.**  Each generated eval file declares objects by *relative* path, e.g.
`define_assert_from_elf "bignum_add_p25519_mc" "x86/curve25519/bignum_add_p25519.o"`
and `"x86/generic/bignum_madd.o"`; `run-answers.sh` runs the executable with
`cwd=objfiles` (L35), so they resolve under `.bench/objfiles/`.  `s2n-bignum/` is
needed only by the **emitter** (glob + disassembly).

### 1.4 `template-cache.json`

Read if present (`load_template_cache`, L70–79, path L23) and written only when
recomputed (`save_template_cache`, L82–88, called L329).  It is a pure cache: absent,
`build_templates` reconstructs the identical mapping from
`problems.json` + the answered list (L173–204).  It was therefore **omitted** from
the court; `combine` recomputed it and wrote a fresh copy into the writable work
copy.  Result identical (all nine syntactic checks passed, templates matched).

## 2. Step 2 — the pruned tree

Built by `.bench/build-sealed-court-v3.py` into `.bench/sealed-court-v3/repo`
(64 MB, `du -sh`).  Contents:

- `.bench/{problems.json, timeouts.json, timeout-map.json, combine-answer-and-setup.py,
  run-answers.sh, collect-verdicts.py, synchk.sh}` (no `template-cache.json`);
- `.bench/ml_files/x86/<nine>.ml` — each truncated to exactly `linenum` lines;
- `.bench/objfiles/x86/{curve25519,secp256k1,sm2}/<nine>.o` + `objfiles/x86/generic/bignum_madd.o`;
- `.bench/s2n-bignum/x86/{curve25519,secp256k1,sm2}/<nine>.o` + `generic/bignum_madd.o`;
- `.bench/hol-light` → symlink to the bind-mounted frozen HOL Light;
- `tools/` (frozen copy), `.git/` (8 MB, for the guard's tag/cleanliness check),
  `research/receipts/ix54-sealed-manifest.json`.

**`ml_files/` is required**, contrary to the suggested contents list — it is the only
thing `combine` reads for templates.  What makes it safe is that the copy stops at
the insertion line.  The nine copies are byte-exact prefixes of the originals,
verified without reading past the prefix:

```
bignum_add_p25519 PREFIX_IDENTICAL bytes=1470971
bignum_sub_p25519 PREFIX_IDENTICAL bytes=1470667
bignum_neg_p25519 PREFIX_IDENTICAL bytes=1470780
bignum_add_p256k1 PREFIX_IDENTICAL bytes=1471043
bignum_sub_p256k1 PREFIX_IDENTICAL bytes=1470633
bignum_neg_p256k1 PREFIX_IDENTICAL bytes=1470701
bignum_add_sm2   PREFIX_IDENTICAL bytes=1471135
bignum_sub_sm2   PREFIX_IDENTICAL bytes=1470742
bignum_neg_sm2   PREFIX_IDENTICAL bytes=1470798
```
(`cmp -s -n "$(wc -c < truncated)" original truncated`).  Because the harness reads
only `lines[:linenum]`, the generated eval `.ml` is byte-identical to one produced
from the full template.

No `s2n-bignum/*/proofs/` directory and no `proofs/*.ml` of any benchmark task exists
in the tree.

## 3. Step 3 — the container

Image: local `ubuntu:24.04`,
`sha256:2b147973fd863c93ced60f325461fe0e262b91317b008e6a2d961f13e81af0e4`
(no pull; `--network none` makes a pull impossible).

`--network none`, `--read-only` root, `--user 1000:1000`, input mounts `:ro`,
writable `tmpfs /work`.  The exact launch line (`.bench/run-sealed-court-v3.sh`):

```
docker run --rm --network none --read-only \
  --tmpfs /tmp:rw,size=256m \
  --tmpfs /work:rw,exec,size=2g \
  --user 1000:1000 \
  --env HOME=/work/home \
  --env MODE=full \
  --volume /usr:/usr:ro \
  --volume /lib64:/lib64:ro \
  --volume /etc:/etc:ro \
  --volume /mnt/1tb_kingston/Tekmerion/.bench/sealed-court-v3:/court:ro \
  --volume /mnt/1tb_kingston/Tekmerion/.bench/hol-light:/mnt/1tb_kingston/Tekmerion/.bench/hol-light:ro \
  --volume /home/one/.elan/toolchains/leanprover--lean4---v4.29.1:/home/one/.elan/toolchains/leanprover--lean4---v4.29.1:ro \
  --entrypoint /usr/bin/bash \
  ubuntu:24.04 /court/run-inside.sh
```

### 3.1 Why host userland is bind-mounted (disclosure)

The OCaml switch (`hol-light/_opam`, compiler 5.4.0) was built against host glibc
**2.44**; every local image is at most glibc **2.41**:

```
$ docker run ... ubuntu:24.04 /switch/bin/ocamlopt -version
/switch/bin/ocamlopt: /lib/x86_64-linux-gnu/libm.so.6:
    version `GLIBC_2.44' not found (required by /switch/bin/ocamlopt)
```

The container therefore borrows `/usr`, `/lib64` and `/etc` (all `:ro`) from the
host.  **This is a data-isolation container, not a userspace-isolation container**:
the mount namespace contains the pruned court but no repository path.  It is the
limitation that makes the literal global `find` below non-empty; see §4.2.

`opam` itself cannot initialise on a read-only root
(`[ERROR] Opam has not been initialised`), so `/work/bin/opam` is a two-line shim
that replays the exact `opam env --switch <hol-light> --set-switch` output captured
on the host.  It sets only `OPAM_SWITCH_PREFIX`, `CAML_LD_LIBRARY_PATH`,
`OCAML_TOPLEVEL_PATH`, `OCAMLTOP_INCLUDE_PATH` and `PATH`; it is environment
plumbing, not a change to any frozen script.

### 3.2 Run history (disclosed in full)

Three container launches, all with the line above apart from the cadical mount:

1. **preflight** (emit only, `MODE=preflight`) — passed; nine answers emitted;
   proved the guard's git check works inside the container.
2. **run 1** — emit and judge ran, but every task returned `FAIL` because the solver
   binary could not start: `cadical: error while loading shared libraries:
   libunwind.so.1`, `exit 127`.  Cause: `cadical`'s `RUNPATH` is `$ORIGIN/../lib`
   and only its `bin/` directory had been mounted.  This is an instrument
   misconfiguration, not a verdict; the log is preserved at
   `.bench/sealed-court-v3-out/container.log` and is not reported as a result.
3. **run 2** — the only change was mounting the elan toolchain at its native path
   so `$ORIGIN/../lib` resolves.  No change to `tools/`, emitters, guard, harness
   inputs, answers, or the bench scripts.  This is the reported run; full log at
   `.bench/sealed-court-v3-out/container-run2.log`.  The nine emitted answer
   files have byte sizes identical to preflight and run 1.

## 4. Step 4 — isolation evidence (inside run 2's container)

### 4.1 The two commands

```
$ find / -path '*proofs*' -name '*.ml' 2>/dev/null
find_proofs_ml_lines=12
/usr/lib/ocaml/rocq-runtime/plugins/funind/functional_principles_proofs.ml
/usr/lib/ocaml/rocq-runtime/proofs/clenv.ml
/usr/lib/ocaml/rocq-runtime/proofs/miscprint.ml
/usr/lib/ocaml/rocq-runtime/proofs/logic.ml
/usr/lib/ocaml/rocq-runtime/proofs/goal_select.ml
/usr/lib/ocaml/rocq-runtime/proofs/tacmach.ml
/usr/lib/ocaml/rocq-runtime/proofs/proof_bullet.ml
/usr/lib/ocaml/rocq-runtime/proofs/refine.ml
/usr/lib/ocaml/rocq-runtime/proofs/proof.ml
/usr/lib/ocaml/rocq-runtime/proofs/subproof.ml
/usr/lib/ocaml/rocq-runtime/vernac/g_proofs.ml
/mnt/1tb_kingston/Tekmerion/.bench/hol-light/ProofTrace/proofs.ml

$ find / -path '*s2n-bignum*' -name '*.ml' 2>/dev/null | wc -l
0

$ test -e /work/repo/.bench/s2n-bignum/x86/proofs/bignum_add_p25519.ml \
    && echo TEST_E_PRESENT || echo TEST_E_ABSENT
TEST_E_ABSENT
```

`find_proofs_dirs_lines=1`: the only directory called `proofs` is
`/usr/lib/ocaml/rocq-runtime/proofs`.

### 4.2 Honest reading of the literal check

The literal global `find` is **not empty**.  All 12 matches are host-userland
library files: eleven Rocq runtime sources shipped under `/usr/lib/ocaml`, and HOL
Light's own `ProofTrace/proofs.ml`.  None is a benchmark proof, and none is under
any `s2n-bignum` path.  This non-emptiness is a direct consequence of §3.1 (host
userland is unavoidable with glibc 2.44 vs 2.41).  The benchmark-scoped predicate is
zero (`s2n_bignum_ml_lines=0`), which is the property the run depends on.

To make the literal global command return nothing, either:

- provide a base image with glibc ≥ 2.44 (or an OCaml toolchain compiled inside the
  image), so `/usr` need not be borrowed; or
- mount host libc/loader/libzstd as individual files and wrap `as`/`ld`/`gcc`
  against the host loader, leaving the image's own `/usr` in place; plus mask
  `hol-light/ProofTrace/proofs.ml`.

Neither was done here: masking unrelated system files would have been cosmetic, and
the result is reported as it stands.

### 4.3 O10.2 firing control (inside the container)

```
$ cat /work/repo/.bench/s2n-bignum/x86/proofs/bignum_add_p25519.ml
cat: /work/repo/.bench/s2n-bignum/x86/proofs/bignum_add_p25519.ml: No such file or directory
cat_exit=1

$ cat /mnt/1tb_kingston/Tekmerion/.bench/s2n-bignum/x86/proofs/bignum_add_p25519.ml
cat: /mnt/1tb_kingston/Tekmerion/.bench/s2n-bignum/x86/proofs/bignum_add_p25519.ml: No such file or directory
cat_hostpath_exit=1
```

The second path is the host's own proof path; it is not mounted, so it does not
resolve even by absolute path.

### 4.4 Mount view (driver trimmed option lists with `sed`)

```
/dev/nvme1n1p1 on /court type ext4 (ro
/dev/nvme1n1p3 on /etc   type ext4 (ro
tmpfs          on /tmp   type tmpfs (rw,nosuid,nodev,noexec
/dev/nvme1n1p3 on /usr   type ext4 (ro
tmpfs          on /work  type tmpfs (rw,nosuid,nodev
```

`/work` was mounted `exec` (Docker's `--tmpfs` defaults to `noexec`; the first
preflight showed `/work/bin/opam: Permission denied`, fixed before run 1).

## 5. Emitter output (verbatim, run 2)

```
wrote /work/answers/x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT/answer.txt mode=answer task=x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT entry=0 last=25 neg=False accumulate=[2, 4, 6, 8, 13, 14, 15, 16] bytes=103709
wrote /work/answers/x86.bignum_sub_p25519.BIGNUM_SUB_P25519_CORRECT/answer.txt mode=answer task=x86.bignum_sub_p25519.BIGNUM_SUB_P25519_CORRECT entry=0 last=20 neg=False accumulate=[2, 4, 6, 8, 12, 14, 16, 18] bytes=103614
wrote /work/answers/x86.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT/answer.txt mode=answer task=x86.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT entry=0 last=23 neg=True accumulate=[13, 16, 19, 21] bytes=103579
wrote /work/answers/x86.bignum_add_p256k1.BIGNUM_ADD_P256K1_CORRECT/answer.txt mode=answer task=x86.bignum_add_p256k1.BIGNUM_ADD_P256K1_CORRECT entry=0 last=26 neg=False accumulate=[2, 4, 6, 8, 14, 15, 16, 19, 21, 23, 25] bytes=103718
wrote /work/answers/x86.bignum_sub_p256k1.BIGNUM_SUB_P256K1_CORRECT/answer.txt mode=answer task=x86.bignum_sub_p256k1.BIGNUM_SUB_P256K1_CORRECT entry=0 last=19 neg=False accumulate=[3, 5, 7, 9, 12, 14, 16, 18] bytes=103614
wrote /work/answers/x86.bignum_neg_p256k1.BIGNUM_NEG_P256K1_CORRECT/answer.txt mode=answer task=x86.bignum_neg_p256k1.BIGNUM_NEG_P256K1_CORRECT entry=0 last=22 neg=True accumulate=[13, 16, 19, 21] bytes=103579
wrote /work/answers/x86.bignum_add_sm2.BIGNUM_ADD_SM2_CORRECT/answer.txt mode=answer task=x86.bignum_add_sm2.BIGNUM_ADD_SM2_CORRECT entry=0 last=27 neg=False accumulate=[3, 5, 7, 9, 11, 13, 14, 16, 17, 20, 22, 24, 26] bytes=103719
wrote /work/answers/x86.bignum_sub_sm2.BIGNUM_SUB_SM2_CORRECT/answer.txt mode=answer task=x86.bignum_sub_sm2.BIGNUM_SUB_SM2_CORRECT entry=0 last=21 neg=False accumulate=[2, 4, 6, 8, 14, 16, 18, 20] bytes=103609
wrote /work/answers/x86.bignum_neg_sm2.BIGNUM_NEG_SM2_CORRECT/answer.txt mode=answer task=x86.bignum_neg_sm2.BIGNUM_NEG_SM2_CORRECT entry=0 last=23 neg=True accumulate=[17, 18, 19] bytes=103571
```

`combine-answer-and-setup.py` (one core): nine `syntactic check PASSED`, eval dir
`eval-20261124_043844`.  `run-answers.sh` exit 0; `collect-verdicts.py` exit 0.

## 6. Verdicts

`research/receipts/ix54-sealed-verdicts.csv` (verbatim):

```
Problem,Verdict
x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT,OK
x86.bignum_add_p256k1.BIGNUM_ADD_P256K1_CORRECT,OK
x86.bignum_add_sm2.BIGNUM_ADD_SM2_CORRECT,OK
x86.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT,OK
x86.bignum_neg_p256k1.BIGNUM_NEG_P256K1_CORRECT,OK
x86.bignum_neg_sm2.BIGNUM_NEG_SM2_CORRECT,OK
x86.bignum_sub_p25519.BIGNUM_SUB_P25519_CORRECT,OK
x86.bignum_sub_p256k1.BIGNUM_SUB_P256K1_CORRECT,OK
x86.bignum_sub_sm2.BIGNUM_SUB_SM2_CORRECT,OK
--,
SUMMARY,
OK,9
```

**OK/N = 9/9.**

### Per-task PROFILE seconds (from `*.run.outlog`)

| task | seconds | verdict |
|---|---:|---|
| x86.bignum_add_p25519 | 22.650 | OK |
| x86.bignum_add_p256k1 | 23.864 | OK |
| x86.bignum_add_sm2 | 18.777 | OK |
| x86.bignum_neg_p25519 | 3.394 | OK |
| x86.bignum_neg_p256k1 | 3.748 | OK |
| x86.bignum_neg_sm2 | 3.203 | OK |
| x86.bignum_sub_p25519 | 9.276 | OK |
| x86.bignum_sub_p256k1 | 16.268 | OK |
| x86.bignum_sub_sm2 | 9.940 | OK |

`*.run.errlog` files were all empty in run 2.

### Axiom caveat

`OK` means `prove(query, tactic)` returned a theorem and `axioms_before =
axioms_after` inside `bench_run` (`.bench/combine-answer-and-setup.py` L46–58).  The
tactic is judged **in the inherited inlined context**: the 38482-line template
prefix, which includes the HOL Light library, s2n-bignum base material, the modulus
definition and the machine-code assertion.  This is **not** an axiom-free
reconstruction of that context, and it does not attest that the context itself is
axiom-free.  It attests only that the emitted tactic adds no axiom.

## 7. Where emit and judge ran

**Both ran inside the container.**  Emit: nine `emit_ref_proof_x86.py --sealed-eval`
invocations with a live `git rev-parse`/`git status` freeze check in the container.
Judge: `combine-answer-and-setup.py`, `run-answers.sh` and `collect-verdicts.py` all
in the same container.  There is **no host-side fallback** and no partial isolation
in this sense.  The residual limitation is the opposite direction — the container
borrows host userland (§3.1), so it is the *data* (proof files) that is isolated,
not the binaries.

## 8. Files and tree state

Tracked writes: this file and `research/receipts/ix54-sealed-verdicts.csv` only.
Everything else lives under git-ignored `.bench/`:
`sealed-court-v3/` (court + `run-inside.sh`), `sealed-court-v3-out/` (logs),
`build-sealed-court-v3.py`, `run-sealed-court-v3.sh`.

```
$ git --no-optional-locks status --porcelain -- tools
(empty)
```

## 9. Preserved artefacts and what the orchestrator verified

The run scripts and the host-side log are now tracked in `research/receipts/ix54/`
(see its `README.md`), because the judge directory itself lived in the container's
tmpfs.

Orchestrator verification, independent of the subagent's summary:

* `research/receipts/ix54/container-run.log` contains exactly 9
  `syntactic check PASSED`, 9 `PROFILE … OK` lines, `SUMMARY` / `OK,9`, and
  `docker_exit=0`; it contains no `FAIL`, `TIMEOUT`, `CHEATING`, `ERROR` or
  `Traceback`.
* `research/receipts/ix54-sealed-verdicts.csv` is `OK,9`.
* `git status -- tools` is empty.

Not independently re-run by the orchestrator: the judge binary itself. The
`*.judge.txt` files were in tmpfs and were not copied out, so
`collect-verdicts.py` cannot be re-executed against them post hoc. A future run
must bind-mount the eval directory; this is recorded as limitation 1 in
`ix54/README.md`.

Disk: 22 GB free before and after (floor 3 GB).  No branch change, no commit.

---

## Addendum (Slice D): ported to Rust

Ported to Rust in Slice D on `staging` (base commit `b28016c`).
`research/receipts/ix54/build-sealed-court-v3.py` is deleted. Reproduction is now
`cargo run --release --bin sealed_court` (defaults: repo = this repository,
court = `<repo>/.bench/sealed-court-v3`; `--repo` and `--court` override both). The rebuilt
tree is byte-identical to the Python one under `diff -r --no-dereference`, and the build
stdout (per-template `prefix_lines` lines plus `du -sh`) is identical; the golden tree
manifest digest is `107f0d9821bdda30df75dfb24626d262fb4866927800ff0f84ccc1884404b626`.
The container has **not** been re-run since the port. See
`research/receipts/slice-d-rust-receipts.md`. Historical text above is untouched.
