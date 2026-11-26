# IX.87 — checking, not producing: three exported objects, a fixed replay, and an honest independent-checker baseline

**Status: PROVISIONAL.** The three objects below are well-formed and replay. The checker is
**not proved sound**: `XVII.B5` and `G1–G3` remain open. Every timing in this receipt is a
timing of a checker whose soundness is not established; it is evidence about a procedure, not
about a verified one.

- **Date:** 2026-11-26
- **Branch:** `staging` (base `64f06e1fb09ecaf34b8fa7f97554df03c66f0705`; `main` and the tags
  `freeze-rust-closure-v6`, `v0.2.0` are untouched)
- **Code:** `src/tools/proof_object.rs`, `src/bin/proof_object.rs`,
  `tools/export_proof_objects.sh`, `tools/compare_checkers.sh`
- **Data:** `research/receipts/ix87-objects.json`, `research/receipts/ix87-fixes.json`,
  `research/receipts/ix87-objects.csv`, `research/receipts/ix87-objects.md`

No number here is reported from a run that did not complete. The independent check did not
happen; that is reported as blocked, with the harness that would run it.

---

## 1. Environment

```
$ free -g | head -2
               total        used        free      shared  buff/cache   available
Mem:             125          18           9           1          97         107
$ df -h /mnt/1tb_kingston | tail -1
/dev/nvme1n1p1  1.3T 1011G  251G  81% /mnt/1tb_kingston
```

The proof builds ran under `tools/cap.sh 12G`, the finalisation and replay under
`tools/cap.sh 20G`, one heavy process at a time. The host kept at least 251 GB free.
The recording kernel is the untracked `.bench/hol-light-objects` build of
`tools/hol-light/proof-object.diff`; only the patch is in the repository.

---

## 2. Integrity before numbers (`XXI.B1`, `XXI.B4`) — and a third defect that blocked the replay

The frontier-bounded replay from `wip/xix-i-frontier-bounded` (`9beb6ee`) was recovered. Two
named defects were fixed first, and a third was found the moment the real object was replayed.

### 2.1 `XXI.B1` — time-of-check/time-of-use

`scan_object(path)` hashed the body, then `replay_frontier()` reopened the path with
`NodeReader::open(path, offset)`. A file replaced between the two opens would be replayed while
the reported derivation digest described different bytes.

**Fix.** `scan_object` reads the file **once** into an immutable `Vec<u8>`, verifies the digest
over that buffer, and parses the header, tables and node records from it. `ObjectScan` owns the
buffer; both replay modes read nodes through `node_reader`/`next_node` from `scan.bytes`, and
payloads are borrowed slices. The path is never reopened.

**Test.** `replay_is_bound_to_the_checked_bytes`: scan object A, copy a different valid object
B over A's path, then replay from the scan. It validates A with A's derivation digest, while
`validate_with` on the now-B path is refused.

```
TOCTOU BOUND: replay used the scanned bytes, not the replaced path
```

**Cost, stated.** The fix means the whole object file is resident. Peak RSS below includes it.
That is the price of binding replay to the checked bytes; it is not a frontier-bounded total
memory claim.

### 2.2 `XXI.B4` — budget before parse

The node budget was enforced inside the replay loop, after the scan had already sized per-node
arrays from the object's declared count.

**Fix.** `scan_object(path, budget)` now receives the budget and refuses with `BudgetExceeded`
when the declared node count exceeds it — **before** the consumer array is allocated. Every
other declared count (hypotheses, axioms, imports, registry, types, type arguments, terms) is
bounded by the bytes actually remaining in the verified body before `Vec::with_capacity`, and
every payload length is a bounds-checked slice (`Cursor::take`).

**Test.** `budget_is_enforced_before_parse`: a 1000-node chain is refused by the scan at budget
10, and scans at budget 1000.

```
BUDGET BEFORE PARSE: refused: replay reached node 1000, over its budget of 10
```

### 2.3 The defect the real object exposed (`XXI.B7`)

The recovered replay refused the real `arm/add_p25519` object at node 363:

```
validate: REFUSED (spliced) refused: node 363 (ABS) does not follow from its parents: ABS: variable free in hypotheses
```

Both replay modes, and the pre-IX.85 checker, refused identically — so this was not the frontier
logic. Tracing the chain showed a hypothesis that HOL Light had removed surviving in replay.
The cause is `alpha_eq`'s `ordav`: it walked the binder environment **outermost-first**, while
HOL Light's `ordav` walks its list head-first, i.e. **innermost-first**. With a shadowed binder
(`\q. ... \q. ...`) it reported alpha-inequality for an alpha-equal pair, so `term_remove` kept a
hypothesis carrying a free variable; the later `ABS r` then failed the kernel's side condition.

**Fix.** `ordav` iterates the binder environment in reverse (innermost binding first).
**Test.** `alpha_eq_follows_innermost_shadowing`.

```
test tools::proof_object::tests::alpha_eq_follows_innermost_shadowing ... ok
```

After the fix the same object validates (section 3). The consequence is worth stating plainly:
the WIP's replay had *no* usable number, because it did not accept the object it was meant to
check. The fix is a prerequisite for every number in this receipt.

### 2.4 What did not change

The refusal controls still fire, verbatim:

```
TRUNCATED REFUSED: object: bad magic
SPLICED REFUSED: refused: node 2 (TRANS) does not follow from its parents: middle terms differ
WRONG CONCLUSION REFUSED: refused: replayed statement identity 01efd6d0... differs from expected 104a8e41...
UNJUSTIFIED IMPORT REFUSED: refused: IMPORT sequent 20d053e4... is not justified by the checked-theorem store
UNAUTHORISED AXIOM REFUSED: refused: axiom a6f5e028... is not authorised by the validated theory
FRONTIER PEAK 2 of 20000 nodes (retained: 20000)
```

---

## 3. `XXI.C1` — the three solver-free objects

Each was recorded through the aggregate route (`TEKMERION_ALG_CLOSE`; the answers are the
`.bench/ix83/c1/*_alg.answer` artefacts), then finalised by the committed exporter:

```
$ tools/export_proof_objects.sh arm add p25519 .bench/ix83/c1/add_alg.answer .bench/ix87/objects
$ tools/export_proof_objects.sh arm sub p25519 .bench/ix83/c1/sub_alg.answer .bench/ix87/objects
$ tools/export_proof_objects.sh arm neg p25519 .bench/ix83/c1/neg_alg.answer .bench/ix87/objects
```

| object | object bytes | recorded nodes | reachable nodes | imports | reported inference delta | published total (matches) |
|---|---:|---:|---:|---:|---:|---:|
| `arm/add_p25519` | 64,895,202 | 4,335,020 | 3,205,212 | 612 | 5,745,767 | 5,745,767 |
| `arm/neg_p25519` | 62,436,134 | 5,119,107 | 3,075,686 | 545 | 5,683,733 | 5,683,733 |
| `arm/sub_p25519` | 90,698,353 | 6,258,132 | 4,215,004 | 587 | 9,029,510 | 9,029,510 |

The published totals are those of `TODO` `XXI.A1` / `ix86` (`add` 5,745,767, `sub` 9,029,510,
`neg` 5,683,733). The exported derivation delta equals each one exactly, so the object is a
faithful recording of the proof the project already publishes.

Recorded-vs-reachable: the object keeps only the nodes reachable from the root; `raw_nodes` in
the object header is the recorded count, `node_count` is the compacted reachable count. The
raw streams themselves are 12.5–40 GB and are not shipped; the object is what is measured.

---

## 4. `XXI.C2` — the Rust checker, twice per object

Command per object (the comparator does exactly this, sequentially, capped):

```
tools/cap.sh --timeout 1800 20G -- ./target/release/proof_object validate \
  --object .bench/ix87/objects/<tag>.hlpo \
  --theory .bench/ix87/objects/<tag>.theory \
  --store  .bench/ix87/objects/<tag>.hlst \
  --expected .bench/ix87/objects/<tag>.expected \
  --replay frontier --budget 8000000 --json <out>.json
```

| object | checker s (run 1 / 2) | process wall s (1 / 2) | peak RSS KB (1 / 2) | peak live nodes | replayed nodes/s (1 / 2) | reported inferences/s (1 / 2) |
|---|---:|---:|---:|---:|---:|---:|
| `arm/add_p25519` | 2.753 / 2.863 | 2.766 / 2.875 | 108,704 / 108,540 | 4,400 | 1,164,262 / 1,119,529 | 2,087,093 / 2,006,904 |
| `arm/neg_p25519` | 4.445 / 4.452 | 4.457 / 4.464 | 102,776 / 102,780 | 2,496 | 691,943 / 690,855 | 1,278,680 / 1,276,670 |
| `arm/sub_p25519` | 4.364 / 4.326 | 4.376 / 4.339 | 139,304 / 139,092 | 3,946 | 965,858 / 974,342 | 2,069,090 / 2,087,265 |

**Determinism: yes.** `statement_id`, `derivation_digest` and the reachable count are identical
in both runs for all three objects. Two runs are reported so determinism is evidence, not an
assumption.

**What each number divides by.**

- `replayed nodes/s = reachable_nodes / checker_seconds` — the rule applications the checker
  actually performed.
- `reported inferences/s = (inference_total − inference_begin) / checker_seconds` — the recorded
  proof session's primitive-inference counter delta. It counts inferences the *proof search*
  went through, not the nodes the *checker* replayed.

These are different numbers and must not be conflated. The checker's own work is the reachable
node count; the reported total is provenance from the recording run.

**What peak RSS includes.** The whole object file (a consequence of the `XXI.B1` read-once
fix), the type and term tables, the frontier and the rule engine. **`peak live nodes` is
theorem retention only.** Frontier-bounded theorem retention is not frontier-bounded total
memory; `XXI.B2` and `XXI.B3` remain open.

---

## 5. `XXI.C3` — the independent checker: BLOCKED, precisely

**No independent checker was available offline, so no independent number is claimed.** What was
looked for and what failed:

- `holtrace`, `holtrace_restore`, `holtrace-verify`, `candle`, `candle-checker` on `PATH`: not
  found.
- The project tree, `/home/one`, `/usr`, `/usr/local`, `/opt`: no HOLTrace or Candle source or
  binary.
- The opam switches `hol-light`, `hol-light-objects`, `hol-light-stamp2`, `dsfb`,
  `creusot-zic`: neither package installed.
- `holtrace.cr.cyp.to` (the spelling in the brief) does not resolve. The real host,
  `holtrace.cr.yp.to`, **does** resolve and serves HTTP 200 from this machine. Nothing was
  downloaded or built in this run.

**Why the objects cannot simply be fed to it.** HOLTrace verifies its own `.50` trace format
inside a HOL Light session (`holtrace_restore`; the C parts of the package are pruning tools,
not the checking kernel), and it checks axioms, constant definitions and type definitions against
that session's registries. The Tekmerion object format is different. Running *the same objects*
therefore needs either a `.hlpo` → `.50` converter or a re-recording with HOLTrace's tracer, which
produces different artefacts. Neither exists here.

**The harness is ready.** `tools/compare_checkers.sh` takes a directory of objects, runs the
Rust checker twice per object, and prints the side-by-side table. It runs an independent checker
only when one is actually supplied, and marks the column blocked otherwise:

```
$ tools/compare_checkers.sh .bench/ix87/objects 8000000
```

A person with HOLTrace supplies a runner:

```
$ INDEP_RUNNER=/path/to/runner TRACE_DIR=/path/to/traces \
    tools/compare_checkers.sh .bench/ix87/objects 8000000
```

where `runner <trace.50> <out>` writes exactly `ok <seconds> <peak_rss_kb>` or `fail <reason>`.
The exact commands to obtain and run HOLTrace are printed in the BLOCKED section (download
`https://holtrace.cr.yp.to/holtrace-latest-version.txt`, install per `doc/install.md`, then
`holtrace_restore !theorems "<tag>.50";;` in a HOL Light session; time the session under
`tools/cap.sh`). Candle has no local installation, and its kernel replay has its own proof
representation, so it needs a converter too.

**No competitor was simulated and no number was invented.**

---

## 6. `XXI.C4` — what can and cannot be claimed

**Can be claimed (provisional).** Three solver-free derivations have been exported as objects and
replayed by the Rust checker on this host; the objects are deterministic across runs; the replay
is frontier-bounded in **theorem retention** (4,400 / 2,496 / 3,946 live theorems against
3.2M / 3.1M / 4.2M reachable nodes); and the exported derivation deltas equal the published
inference totals.

**Cannot be claimed.** That the Rust checker is faster or smaller than any other checker. No
independent checker accepted these objects in this run, so there is no checker-versus-checker
comparison. These numbers are a baseline, not a win. The checker is not proved sound.

---

## 7. Commands (verbatim)

```
$ free -g | head -2
               total        used        free      shared  buff/cache   available
Mem:             125          18           9           1          97         107
$ df -h /mnt/1tb_kingston | tail -1
/dev/nvme1n1p1  1.3T 1011G  251G  81% /mnt/1tb_kingston

$ tools/cap.sh 20G -- cargo test --offline
# all test binaries ok; lib: 20 passed (proof_object tests), 0 failed
$ cargo clippy --offline --all-targets -- -D warnings
# Finished (no warnings)
$ cargo fmt --check
# (no output)
$ git ls-files '*.py'
# (empty)
$ git status --short
# src/bin/proof_object.rs, src/tools/proof_object.rs modified;
# tools/compare_checkers.sh and tools/export_proof_objects.sh added
# (the receipt files are added by the same commit)

$ tools/compare_checkers.sh .bench/ix87/objects 8000000
# writes .bench/ix87/objects/compare/rust.csv and rust.md, printed in §3–4
```

No tag was moved and `main` was not touched. `research/receipts/freeze-closure-v7.json` was
**not** regenerated: it still pins `81f84db`, while this change alters `src/tools/proof_object.rs`
and `src/bin/proof_object.rs`; a release must regenerate v7 at the release commit, and no tag
may be moved here.
