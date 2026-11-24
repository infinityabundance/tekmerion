# IX.63 — trim inside the check: LRAT backward closure between solver and kernel

- **Date:** 2026-11-24
- **Branch:** `staging`
- **Change under test:** `tools/csat_lib.ml` gains a conservative backward-closure
  trim over positive RUP hints, applied in `CSAT_REFUTE_GEN` between the solver
  writing its LRAT and the kernel replaying it.
- **Result:** all **27/27** tasks accepted with `check_axioms` passing. Total
  primitive inferences **3,697,462,655 → 2,772,864,005** (delta **924,598,650**,
  25.0%); total `time prove` seconds **1391 → 1321** (delta **70 s**). Over the
  **93** closes, retained LRAT bytes fall **3.03×** and retained replay steps
  **2.29×**. `arm/p521/add_p521` is **470,294,392 inferences and 104 s**, under
  the 120 s budget, `equals_thm` to the untrimmed theorem.

---

## 1. Mechanism

The previous measurement (`ix62-phase-attribution.md`) showed replay is 50.11% of
primitive work on the measured set (53.68% for `arm/p521/add_p521`) and that a
backward-closure trim of the LRAT saves 2.95× bytes, 2.42× replay inferences and
2.33× replay seconds, with every replay still succeeding. This commit moves that
trim **inside the check**, so the saving is on the production path rather than in
a measurement harness.

### 1.1 The trimmer

`tools/csat_lib.ml` now defines, as ordinary `let` phrases (the emitted answer
inlines each phrase as a nested binding, so no new structure items):

| phrase | role |
|---|---|
| `csat_lrat_tokens` | whitespace splitter (space, tab, CR) |
| `csat_lrat_parse` | one line → `(id, hints option, raw)`; `None` is a refusal |
| `csat_lrat_read` | whole file → `(line array, file bytes)` or `Error reason` |
| `csat_trim_lrat` | the closure, the refusal guards, in-place rewrite, report |
| `csat_trim_enabled` | a reference, true in every emitted answer; false only in T4 |

Semantics are those of the reference implementation `src/tools/lrat_trim.rs`,
whose unit tests state the contract (chain, unused additions, deletions, order,
trailing deletion, negative hints, byte counts, duplicate ids, malformed lines,
CLI arity). The OCaml port keeps exactly the additions reachable backwards from
the last addition through positive hints, in file order, and drops **every**
deletion line. A kept clause that was deleted later cannot hurt: RUP uses the
hint list, and extra clauses in `cls` are never consulted.

**Conservative by construction.** The trim is refused, leaving the certificate
byte-for-byte untouched, when:

| reason | condition |
|---|---|
| `negative_hint` | **any** hint in the file is negative (RAT) |
| `no_empty_clause` | the last addition has one or more literals |
| `malformed` | a non-blank line is not an LRAT addition or deletion |
| `duplicate_id` | an addition id is redefined |
| `empty_lrat` / `no_additions` | nothing to follow |
| `io_error` / `write_error` | read or rewrite failed |

On refusal it prints exactly `CSAT_TRIM skipped=<reason>`. On success it prints

```text
CSAT_TRIM before=<bytes> after=<bytes> steps_before=<n> steps_after=<n>
```

where `before` is the on-disk LRAT size, `after` the rewritten size, and the
steps are addition lines (each replayed as one primitive inference).

### 1.2 The wiring

In `CSAT_REFUTE_GEN`, between the solver exit check and the replay:

```ocaml
let _ = if !csat_trim_enabled then csat_trim_lrat lratname in
let res = LRAT_REFUTE_GEN amap cnfname lratname in
```

There is no fallback. A wrong closure makes `LRAT_REFUTE_GEN` fail, which is the
desired behaviour; nothing catches that failure. The kernel still replays every
kept step; no trusted code was added and no certificate is accepted on the
solver's word.

### 1.3 Equivalence with the Rust reference

On the largest certificate in the IX.62 set (106,508,743 bytes,
`.bench/c62/work/f5dfd7c0884866f5bddfe7d070a1827c.lrat`), the OCaml trimmer gives

```text
CSAT_TRIM before=106508743 after=34393236 steps_before=779393 steps_after=362084
```

and `cmp` against `target/release/lrat_trim` output reports byte identity. In a
native build (`ocamlopt`) the trim takes **1.43 s** on that file; the interpreter
takes 7.99 s. The proof binaries are native, so the run pays the native cost.

---

## 2. The 27-task run, before and after

Command (one task at a time, `-j1`, forced rebuild, axiom-stamp v2 kernel):

```text
$ export PATH="$ROOT/.bench/bin:$HOME/.elan/toolchains/leanprover--lean4---v4.29.1/bin:$PATH"
$ ./target/release/dropin sweep --arch all --force --stamp \
    --hol /mnt/1tb_kingston/Tekmerion/.bench/hol-light-stamp2 \
    --json .bench/c63/sweep.json
...
OK 27/27
```

The baseline is `research/receipts/ix57b-axiom-stamp.json`: the same kernel pin,
the same 27 tasks, and the same `tools/csat_lib.ml` (unchanged since `558f640`,
verified with `git log -- tools/csat_lib.ml`). The "after" run uses the same
answers with only the inlined `csat_lib` phrases replaced by the new library
text — exactly what re-running the emitter produces, since the answer inlines
`phrases_from(csat_lib.ml)`. No tactic and no goal differs.

### 2.1 Per-theorem table

Generated from `ix63-trimmed-replay.json` by the command shown in §2.2. The two
reduction columns are `before/after` ratios; `inf ×` is over the whole theorem's
primitive inferences, `bytes ×` and `steps ×` are over its closes.

| arch | task | closes | inf before | inf after | inf delta | inf × | s before | s after | bytes × | steps × |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| arm | curve25519/add_p25519 | 4 | 170225275 | 119194788 | 51030487 | 1.43× | 46 | 42 | 2.88× | 2.29× |
| arm | p256/add_p256 | 4 | 170931670 | 119690299 | 51241371 | 1.43× | 47 | 43 | 2.66× | 2.07× |
| arm | secp256k1/add_p256k1 | 4 | 157240305 | 112805964 | 44434341 | 1.39× | 46 | 42 | 2.81× | 2.2× |
| arm | p384/add_p384 | 4 | 349139623 | 241041106 | 108098517 | 1.45× | 74 | 65 | 2.97× | 2.32× |
| arm | p521/add_p521 | 4 | 681707202 | 470294392 | 211412810 | 1.45× | 122 | 104 | 3.06× | 2.37× |
| arm | sm2/add_sm2 | 4 | 159985379 | 112407207 | 47578172 | 1.42× | 46 | 42 | 2.89× | 2.17× |
| arm | curve25519/sub_p25519 | 4 | 110957174 | 88587146 | 22370028 | 1.25× | 39 | 39 | 3.31× | 2.41× |
| arm | p256/sub_p256 | 4 | 103417925 | 85125865 | 18292060 | 1.21× | 38 | 37 | 3.01× | 2.24× |
| arm | secp256k1/sub_p256k1 | 4 | 110121226 | 88185214 | 21936012 | 1.25× | 39 | 37 | 3.4× | 2.46× |
| arm | p384/sub_p384 | 4 | 214410496 | 165327853 | 49082643 | 1.3× | 55 | 51 | 3.89× | 2.85× |
| arm | p521/sub_p521 | 2 | 221845891 | 177146926 | 44698965 | 1.25× | 59 | 56 | 2.87× | 2.1× |
| arm | sm2/sub_sm2 | 4 | 107504057 | 85497537 | 22006520 | 1.26× | 39 | 38 | 3.46× | 2.49× |
| arm | curve25519/neg_p25519 | 3 | 35457948 | 33981360 | 1476588 | 1.04× | 29 | 29 | 2.15× | 2.28× |
| arm | p256/neg_p256 | 4 | 45739252 | 42601162 | 3138090 | 1.07× | 31 | 31 | 3.29× | 2.63× |
| arm | secp256k1/neg_p256k1 | 4 | 45424790 | 42764145 | 2660645 | 1.06× | 30 | 30 | 2.98× | 2.48× |
| arm | p384/neg_p384 | 4 | 86799846 | 81409148 | 5390698 | 1.07× | 37 | 37 | 2.94× | 2.33× |
| arm | p521/neg_p521 | 4 | 123668935 | 119587114 | 4081821 | 1.03× | 44 | 44 | 7.14× | 6.15× |
| arm | sm2/neg_sm2 | 4 | 46665222 | 42391685 | 4273537 | 1.1× | 31 | 30 | 4.5× | 2.81× |
| x86 | curve25519/add_p25519 | 4 | 160584653 | 112639551 | 47945102 | 1.43× | 70 | 67 | 2.85× | 2.24× |
| x86 | secp256k1/add_p256k1 | 4 | 166694408 | 118172103 | 48522305 | 1.41× | 71 | 68 | 2.87× | 2.14× |
| x86 | sm2/add_sm2 | 2 | 127361354 | 77037787 | 50323567 | 1.65× | 66 | 63 | 3.05× | 2.02× |
| x86 | curve25519/sub_p25519 | 2 | 61298759 | 49312896 | 11985863 | 1.24× | 57 | 56 | 2.47× | 1.85× |
| x86 | secp256k1/sub_p256k1 | 4 | 111312547 | 85369650 | 25942897 | 1.3× | 64 | 62 | 3.95× | 2.63× |
| x86 | sm2/sub_sm2 | 2 | 67342905 | 49900918 | 17441987 | 1.35× | 57 | 57 | 3.06× | 2.02× |
| x86 | curve25519/neg_p25519 | 2 | 19973249 | 18320080 | 1653169 | 1.09× | 51 | 51 | 2.14× | 1.6× |
| x86 | secp256k1/neg_p256k1 | 2 | 23422025 | 17649673 | 5772352 | 1.33× | 52 | 50 | 5.49× | 2.52× |
| x86 | sm2/neg_sm2 | 2 | 18230539 | 16422436 | 1808103 | 1.11× | 51 | 50 | 2.58× | 1.77× |
| **all** | **27 tasks** | **93** | **3697462655** | **2772864005** | **924598650** | **1.33×** | **1391** | **1321** | **3.03×** | **2.29×** |

Every row has `exit=0`, `theorem=True`, `exception=False`, `check_axioms=true`.
No close was skipped: `grep -c '^CSAT_TRIM skipped'` over the 27 outputs is 0.
All 93 real certificates satisfied `negative_hints=0` and `root_empty=true`, as
in IX.62.

### 2.2 Generator

```text
$ jq -r '.tasks[] | "| \(.arch) | \(.dir)/\(.op)_\(.family) | \(.closes) | \(.before_inferences) | \(.after_inferences) | \(.inference_delta) | \((.before_inferences/.after_inferences*100|round)/100)× | \(.before_seconds|round) | \(.after_seconds|round) | \((.trim_bytes_before/.trim_bytes_after*100|round)/100)× | \((.steps_before/.steps_after*100|round)/100)× |"' \
    research/receipts/ix63-trimmed-replay.json
```

Totals from the same JSON:

```text
$ jq '.totals' research/receipts/ix63-trimmed-replay.json
{
  "tasks": 27,
  "accepted": 27,
  "closes": 93,
  "before_inferences": 3697462655,
  "after_inferences": 2772864005,
  "inference_delta": 924598650,
  "before_seconds": 1391,
  "after_seconds": 1321,
  "seconds_delta": 70,
  "trim_bytes_before": 1270870346,
  "trim_bytes_after": 419662688,
  "trim_steps_before": 12325924,
  "trim_steps_after": 5374983,
  "closes_trimmed": 93
}
```

### 2.3 Reading the totals

- **Inferences fall 25.0%** (924,598,650 of 3,697,462,655). The delta is exactly
  the replay delta: non-replay work is untouched.
- **Seconds fall 70 s**, not more, because the trimmer runs inside `time prove`.
  It reads the full LRAT, walks it, and writes the trimmed file: on `add_p521`
  that is about 3.6 s (1.43 s per 106 MB, scaled to its 269 MB of certificates)
  against 19.1 s of replay saved in IX.62. On the small `neg`/`sub` proofs the
  two nearly cancel (several rows are unchanged).
- **Bytes fall further (3.03×) than steps (2.29×)**, as in IX.62: trimming drops
  many short unused steps but must keep the long clauses that matter.

### 2.4 `arm/p521/add_p521` against both budgets

```text
inf 681,707,202 -> 470,294,392   (delta 211,412,810, 1.45x)
sec 122 -> 104                  (120 s budget: under by 16 s; 900 s sweep timeout: 104/900)
```

Per-close, from the same run:

| close | bytes before | bytes after | bytes × | steps before | steps after | steps × |
|---|---:|---:|---:|---:|---:|---:|
| 1a5f9900 | 37706414 | 13705077 | 2.75× | 389283 | 153940 | 2.53× |
| 737f4a92 | 91976484 | 32239888 | 2.85× | 708282 | 310514 | 2.28× |
| f5dfd7c0 | 106508743 | 34393236 | 3.10× | 779393 | 362084 | 2.15× |
| 31f1d86a | 32765739 | 7456046 | 4.39× | 381687 | 126966 | 3.01× |

**Verdict:** under the 120 s budget, statement unchanged. It is still 86.7% of
the budget, so it remains above the 70% `XII.F9.2` threshold; `XIII.E2` is met,
`XII.F9.2` is not.

---

## 3. Controls

All controls were run against `.bench/hol-light-stamp2` (axiom-stamp v2, the
kernel used for the 27-run). The T1–T3 session loads `Cadical/make.ml` and
`tools/csat_lib.ml` and uses a two-clause CNF (`p cnf 1 2`, clauses `[1]` and
`[-1]`). T2/T3 certificates are synthetic: they exercise the replayer and the
refusal paths without a solver.

### T1 — trim to nothing: the replayer must reject

```text
$ ./.bench/c63/build_controls.sh
T1_CASE /mnt/1tb_kingston/Tekmerion/.bench/c63/controls_empty.lrat
T1_EMPTY_REJECTED true
```

An empty LRAT handed to `LRAT_REFUTE_GEN` raises. There is no fallback in the
wiring that could turn this into an acceptance.

### T2 — negative hint: no trim, replay unchanged

The certificate is

```text
-1 -1 0 2 0
3 0 -1 1 0
```

The empty clause hints `-1`, so the guard refuses. (The negative number resolves
to an addition whose *id* is `-1`, which is why this un-trimmed file can replay
under this kernel; a genuine CaDiCaL RAT step could not, which is precisely why
the guard refuses.) Captured verbatim:

```text
T2_CASE /mnt/1tb_kingston/Tekmerion/.bench/c63/controls_negative.lrat
CSAT_TRIM skipped=negative_hint
T2_UNCHANGED true
T2_REPLAYS true
```

### T3 — no empty clause: no trim, replay fails as before

Certificate: one non-empty addition and nothing else.

```text
T3_CASE /mnt/1tb_kingston/Tekmerion/.bench/c63/controls_no_empty.lrat
CSAT_TRIM skipped=no_empty_clause
T3_UNCHANGED true
T3_REPLAY_FAILS true
```

The trimmer leaves the bytes alone (`T3_UNCHANGED true`) and the replay then
fails, exactly as it would have without the trimmer.

A positive control on the same CNF (the full T2 certificate) replays:

```text
CSAT_TRIM skipped=negative_hint
T0_POSITIVE_REPLAYS true
T0_TRIMMED_BYTES 23
CONTROLS_DONE
```

(`T0` deliberately reuses the negative-hint file, so it also reports the guard.)

### T4 — theorem equality for the three samples

Each sample was built twice in one session: once with the trimmed answer, once
with the original (untrimmed, no trimmer) answer, then compared with
`equals_thm`. Verbatim:

```text
EQUALS_THM BIGNUM_ADD_P25519_CORRECT true
T4_DONE BIGNUM_ADD_P25519_CORRECT
EQUALS_THM BIGNUM_ADD_P521_CORRECT true
T4_DONE BIGNUM_ADD_P521_CORRECT
EQUALS_THM BIGNUM_NEG_SM2_CORRECT true
T4_DONE BIGNUM_NEG_SM2_CORRECT
```

### T5 — the budget

`arm/p521/add_p521` is **104 s** (see §2.4), under the 120 s budget. It is not
under the 70% threshold (0.867).

### OCaml-side unit evidence

`tests/csat_trim_selftest.sh` extracts the delimited `CSAT_TRIM_STANDALONE`
block from `tools/csat_lib.ml` verbatim and runs 11 cases with `ocaml` (no HOL,
no solver, no Python): chain closure, unused additions, deletions (including of
kept clauses), original order, trailing deletion, `negative_hint`,
`no_empty_clause`, malformed lines, empty file, deletion-only file, duplicate
ids, and a hint to a CNF clause. Captured:

```text
$ tests/csat_trim_selftest.sh
...
PASS csat_trim_selftest cases=11
OK: csat_trim_selftest
```

---

## 4. What is not established

- **Replay share only.** This trims replay work. It does not separate successful
  derivation inferences from discarded search in the remaining count; `XIV.B3`
  stays open, and search is still mixed into the 2,772,864,005. `XIII.E5` is
  untouched.
- **One host, one kernel, one pin.** One machine, one `-j1` run, axiom-stamp v2
  at the pin recorded in the JSON, CaDiCaL 2.1.2. No cross-host or cross-pin
  claim.
- **`seconds` is `time prove`.** It excludes HOL Light load and native compile;
  it does not include the build. The whole 27-run wall time is not reported as a
  per-task number.
- **The trimmer cost is not separately instrumented.** It is included in the
  theorem seconds, which is the honest comparison for the check, but the split
  between parse/closure/write and replay is inferred from the IX.62 replay
  measurements, not measured in this run.
- **The guard is exercised on a synthetic negative-hint certificate, not a real
  RAT proof.** No real CaDiCaL certificate in the set contains a negative hint
  (`CSAT_TRIM skipped=0` over 93 closes). The synthetic file shows the refusal
  path and that the un-trimmed bytes replay; it does not claim this kernel can
  replay genuine RAT steps (it cannot, and that is why the guard is required).
- **No sealed run was attempted.** `tools/csat_lib.ml` and `src/tools` moved, so
  the freeze identity is stale. A new freeze (`v6`) is required before any
  sealed run. This receipt does not create it.
- **`XII.F9.2` remains open:** 104/120 = 0.867, above the 0.70 threshold.

---

## 5. Byte-identity baseline: retired as of this commit

`tools/csat_lib.ml` is inlined into every emitted answer, so adding the trimmer
changes the answer text for all 27 tasks. The migration property "27/27 answers
byte-identical to the Python-era originals" (recorded in
`research/receipts/slice-b-rust-emitters.md`, "Byte identity: 27/27") **no longer
holds after this commit**. That old claim is not deleted; a marked retirement
note is appended at that section.

**New invariant as of IX.63:** the theorems are unchanged — the three samples
are `equals_thm` to the untrimmed theorem, and all 27 are accepted with
`check_axioms` passing — and only the library text and the certificate handling
changed. The four fixed-tactic digests in `tests/tools_receipt.rs` were repinned
to the post-IX.63 text, with the old Python-era values retained there in a
comment as the boundary; `tests/tools_emit.rs` now pins emitter consistency with
the regenerated (post-IX.63) `.bench` reference answers.

---

## 6. Reproduction

```text
$ tests/csat_trim_selftest.sh                       # the shipped block, 11 cases
$ ./.bench/c63/build_controls.sh                    # T1-T3
$ ./.bench/c63/make_t4 <assembled.ml> <trim.answer> <orig.answer> <out.ml>
$ export PATH="$ROOT/.bench/bin:$HOME/.elan/toolchains/leanprover--lean4---v4.29.1/bin:$PATH"
$ ./target/release/dropin sweep --arch all --force --stamp \
    --hol /mnt/1tb_kingston/Tekmerion/.bench/hol-light-stamp2 \
    --json .bench/c63/sweep.json
$ ./.bench/c63/collect.sh                            # rebuild this JSON from sweep + ix57b + logs
$ jq -r '<the generator in §2.2>' research/receipts/ix63-trimmed-replay.json
```

The answer splice is scratch, not shipped: it replaces
`phrases_from(tools/csat_lib.ml)` inside the 27 reference answers, asserts the
old block occurs exactly once per file, and reports
`SPLICE answers=27 changed=27 old_block_bytes=5558 new_block_bytes=12321`. It
uses the crate's own `inline_answer::phrases_from`, so it is the same transform
the emitter applies.

---

## 7. Verification and provenance

```text
$ cargo test --offline
suites=38 passed=293 failed=0
$ cargo clippy --offline --all-targets -- -D warnings
clippy_rc=0
$ cargo fmt --check
fmt_rc=0
$ git ls-files '*.py' | wc -l
0
```

- The trimmer's output is byte-identical to `src/tools/lrat_trim.rs` on a
  106.5 MB certificate; the Rust tests are unchanged and still green.
- All 27 were recorded by the sweep as `exit=0 theorem=True exception=False`
  with `check_axioms=true`; the `.correct` files it produced carry the
  `val BIGNUM_<OP>_<FAM>_CORRECT : thm` line. The three T4 `.correct` files were
  then overwritten by the T4 harness and print `EQUALS_THM ... true` instead.
- The T4 builds run both answers in one session and compare with `equals_thm`;
  they are not text comparisons.
- Disk floor: 8 GB. `.bench/hol-light-stamp` (the superseded v1 instrument,
  409 MB) was deleted; scratch is git-ignored.
- No Python was used and none is tracked.
