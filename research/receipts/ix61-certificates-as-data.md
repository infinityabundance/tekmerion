# IX.61 — certificates as data: measurement gate fired, in-answer design stopped

- **Date:** 2026-11-24
- **Base commit:** `6b2a47a` (`staging`; this receipt is committed on top)
- **Subject:** `arm/curve25519/add_p25519` — the simplest of the 27
- **Status:** **step 1 only.** The measurement exceeds the declared size gate
  (8 MB), so the certified-answer design was stopped before it was built. No
  certified answer was emitted, no control was run, no `TODO` box is ticked.

## 1. What step 1 asked, and how it was measured

`XIII.A`/`XIV.D` want the SAT search moved to construction and the judge to
replay a supplied certificate with no solver and no file I/O. Before building
that, the brief set a gate:

> If a proof performs more than ~8 closes or the total LRAT exceeds ~8 MB, stop
> and report.

The instrumented run uses the **ordinary tactic**, unmodified, with a hook on
`CSAT_REFUTE_GEN` that copies each `(CNF, LRAT)` pair out before the temporary
files are deleted, plus an assertion that the clause list derived from the CNF
term (`clauses_of_term` over the `dimacs_of_hol` numbering) is exactly the list
`read_dimacs` reads back (`clauses_equal`).

The hook is applied to a **generated copy** of `tools/csat_lib.ml` inside the
measurement script (`instrument_csat_lib` in `src/tools/emit.rs`). The tracked
library is untouched: adding the hook to it changes every emitted answer and
would have broken the Python byte-identity regression baseline in
`tests/tools_emit.rs` (which still passes, 8/8, including the two byte-identity
tests).

```text
$ target/release/emit --task arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT \
    --mode cert-producer --spent \
    --spent-manifest research/receipts/spent-r4-arm.json \
    --work .bench/c61/work --out .bench/c61/producer.ml
wrote .bench/c61/producer.ml mode=cert-producer task=arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT work=.bench/c61/work bytes=1453935
```

The producer is compiled and run by the existing pipeline (`.bench/c61/run.sh`,
the `run-answers.sh` recipe: `ocamlopt.byte` against the frozen
`hol-light/hol_lib.cmxa`, then the executable from `.bench/objfiles`).

## 2. Verbatim measurement

```text
COMPILED c61/producer.ml
CERTCLOSE n=1 key=4fe73a1962de3d24890541a7e7ab8a94 cnf_bytes=1022278 lrat_bytes=14453899 nvars=22018 nclauses=60831 clauses_equal=true
CERTCLOSE n=2 key=6353242dc10dacb349e8a033abd2e5b6 cnf_bytes=979842 lrat_bytes=9985779 nvars=21249 nclauses=58529 clauses_equal=true
CERTCLOSE n=3 key=251e2a941f5355f63309e096eefd0beb cnf_bytes=1022631 lrat_bytes=26518204 nvars=22023 nclauses=60845 clauses_equal=true
CERTCLOSE n=4 key=bb8ee8c85bee7d5205c949aa74458563 cnf_bytes=980384 lrat_bytes=14699743 nvars=21254 nclauses=58543 clauses_equal=true
PRODUCER task=arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT ok=true hyps=0 closes=4 seconds=24.058
```

```text
lrat_total=65657625 cnf_total=4005135 lines=729373
```

| close | key (MD5 of `string_of_term ntdcnf`) | CNF bytes | LRAT bytes | vars | clauses |
|---|---|---|---|---|---|
| 1 | `4fe73a1962de3d24890541a7e7ab8a94` | 1,022,278 | 14,453,899 | 22,018 | 60,831 |
| 2 | `6353242dc10dacb349e8a033abd2e5b6` | 979,842 | 9,985,779 | 21,249 | 58,529 |
| 3 | `251e2a941f5355f63309e096eefd0beb` | 1,022,631 | 26,518,204 | 22,023 | 60,845 |
| 4 | `bb8ee8c85bee7d5205c949aa74458563` | 980,384 | 14,699,743 | 21,254 | 58,543 |
| **total** | | **4,005,135** | **65,657,625** | | |

`clauses_equal=true` on all four is the required clause-structure equivalence:
`clauses_of_term` over the numbering `dimacs_of_hol` writes reproduces exactly
what `read_dimacs` reads from the file cadical was given. Closing pairs 1/3 and
2/4 have near-identical CNFs (differing by 14 clauses), consistent with the two
kept branch conditions of `BRIDGE_SPLIT_CONDITIONS_KEEP_TAC` producing four
branches (`2²`). I did **not** add instrumentation that names each close's
goal, so that inference is not evidence; the counts and sizes are.

Nothing about the theorem changed: `ok=true hyps=0`, and this is the same
tactic `--mode answer` emits for this task.

## 3. Verdict against the gate

- closes: **4** — inside the "~8" budget.
- total LRAT: **65,657,625 bytes** — **8.2×** the 8 MB budget.

The gate is exceeded on size. Per the brief, the design was not built.

## 4. Encoding probes (same four CNFs, same machine)

Since the only question is how much data a certificate must carry, three
encodings were measured on the very same CNFs.

```text
== binary LRAT (cadical default encoding) ==
251e2a941f5355f63309e096eefd0beb 11647890
4fe73a1962de3d24890541a7e7ab8a94 6387507
6353242dc10dacb349e8a033abd2e5b6 4487910
bb8ee8c85bee7d5205c949aa74458563 6533857
binlrat_bytes_total=29057164
```

```text
== backward-closure trimmed text, deletion lines dropped ==
work/251e2a941f5355f63309e096eefd0beb.lrat lines=209645 kept=103027 full_bytes=24369546 trimmed_bytes=9752767 ratio=2.50
work/4fe73a1962de3d24890541a7e7ab8a94.lrat lines=125801 kept=55661 full_bytes=12966351 trimmed_bytes=5086973 ratio=2.55
work/6353242dc10dacb349e8a033abd2e5b6.lrat lines=117719 kept=52872 full_bytes=8623724 trimmed_bytes=3506588 ratio=2.46
work/bb8ee8c85bee7d5205c949aa74458563.lrat lines=112914 kept=35272 full_bytes=13400012 trimmed_bytes=4418770 ratio=3.03
trimmed_bytes_total=22765098
```

The trimming probe (`.bench/c61/trim.rs`, scratch, untrusted) keeps exactly the
additions reachable backwards from the empty clause through LRAT hints and drops
every deletion line. It is the lemma-level core of what `lrat-trim` emits; the
trimmed proofs were **not** replayed, so replayability is an argument by
construction (every hinted id is retained, and LRAT hints are explicit), not a
measurement.

| encoding | bytes | × 8 MB |
|---|---|---|
| caDiCaL text LRAT as emitted (`--no-binary --lrat`) | 65,657,625 | 8.2× |
| same, deletion lines removed (untrimmed otherwise) | 59,359,633 | 7.4× |
| binary caDiCaL LRAT (default) | 29,057,164 | 3.6× |
| backward-closure trimmed text | 22,765,098 | 2.8× |

Deletions are only ~8% of the text (163,294 of 729,373 lines). The additions —
566,079 RUP steps, ~105 bytes each — are the data.

An answer carrying the text would be ~66 MB plus OCaml escaping (each of the
729,373 newlines becomes two characters), i.e. roughly 66.4 MB in a single
string literal, against 103,709 bytes for the current answer.

## 5. What was built, and what was deliberately removed

Kept (verified by the runs above):

- `src/tools/emit.rs`: `template_prefix` (the harness's `lines[:linenum]`
  reader) and `cert_producer_source` (the measurement script), reachable as
  `--mode cert-producer --work DIR`. Marked measurement-only in the source.
- `tests/tools_emit.rs`: three offline tests for `template_prefix` (exact line
  count, line count past EOF, missing `inlined_locations`).
- The measurement run itself: `.bench/c61/` (git-ignored), including the
  producer source, the four CNF/LRAT pairs, and `measure.log`.

Removed before committing, because a half-built certified path is not an
artefact this repository should carry:

- a draft `tools/csat_cert.ml` (an in-memory port of `LRAT_PROVE_GEN` plus a
  `CSAT_PROVE_CERT` driver). It was never compiled or replayed, so it was
  deleted rather than shipped untested.
- draft `cert-answer`/`cert-check` emitter modes and a protocol-v2 prelude
  builder. They depend on the unverified replay and on decisions this
  measurement reopens.

Not touched: `.bench/hol-light/**`, `.bench/s2n-bignum/**`, `.bench/*.py`,
`src/native/`, `verification/`, `tools/hol-light/axiom-stamp.diff`,
`tools/bridge.ml`, `src/tools/script_gate.rs`, `src/tools/sealed_court.rs`,
other receipts, tags. No protocol change is declared, because none was made.

## 6. Recommended design (which this measurement selects)

The 8 MB gate is not reachable by compression: binary is 29.1 MB and the
measured trimmed floor is 22.8 MB. Shrinking the *close* is a different proof
structure, not an encoding. So:

1. **Ship the certificate as a content-addressed side artefact, not in the
   answer** (`XIII.A7`). The answer carries the digest (already the key) and
   the tactic; the judge reads one file, verifies its digest, and the kernel
   replays every step. This keeps the answer at the current ~100 KB scale and
   keeps the kernel as the sole judge. It changes the milestone's K2 from "no
   file I/O at check time" to **"no writes, no solver, exactly one
   digest-verified read"**, and that change must be declared as such.
2. **Measure a real `lrat-trim` pass at emission before building anything.**
   22.8 MB is the floor this probe can see; a proper trimmer (not installed
   here) can also drop hinted-but-irrelevant literals. If the trimmed
   certificate lands under ~1–2 MB, the in-answer no-I/O design becomes viable
   after all and should be preferred.
3. **If no-read is non-negotiable at any size**, the goal must be split into
   smaller SAT instances (per-limb or per-accumulator lemmas) so that the sum
   of certificate bytes falls below the budget. That changes the emitter's
   proof structure and is a much larger piece of work than the plumbing
   `XIII.A1–A3` describes.

No certificate was published, so `XIII.A7`'s store does not yet exist; (1) is
the next decision, not a completed item.

## 7. What is not done

- **All six controls (K1–K6)** — not run. They test the certified path, which
  does not exist. Nothing here should be read as evidence about solver-free
  checking, tamper rejection, or differential replay equality.
- **`XIII.A1–A3`, `A5`** — unticked. `A4` (27/27), `A6` (Candle) remain open as
  before. `A7` is now the recommended route.
- **The other 26 theorems** — not measured. Nothing in this receipt supports a
  claim about any task other than `arm/curve25519/add_p25519`.
- **Candle** — not attempted.
- **Protocol v2 / sealed court** — not changed. The judge currently has no way
  to supply a support library that a compiled script can see (a runtime `loadt`
  gives the type-checker no names), so any future no-inline design needs the
  builder to inject the library text into the template prefix and raise
  `inlined_locations` accordingly. That is a mechanism, not a plan: it was not
  implemented.
- **Freeze** — a new freeze is still required before any sealed use of the
  `src/tools/emit.rs` change (`XIV.I1`). None was created.

## 8. Verification and provenance

```text
$ cargo test --offline 2>&1 | grep -E "test result:" \
    | awk '{ok+=$4; fail+=$6} END{printf "suites=%d passed=%d failed=%d\n", NR, ok, fail}'
suites=37 passed=283 failed=0

$ cargo test --offline 2>&1 | grep -c FAILED
0

$ cargo clippy --offline --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.47s
clippy_rc=0

$ cargo fmt --check
fmt_rc=0

$ git ls-files '*.py'
[py count: 0]

$ df -h /mnt/1tb_kingston | tail -1
/dev/nvme1n1p1  1.3T  1.3T   11G 100% /mnt/1tb_kingston
```

- Guard: the task is held-out by marker (`25519`) but is disclosed development
  data, so it was run under the documented `--spent` allow-list
  (`research/receipts/spent-r4-arm.json`, IX.53), not by disabling the guard.
- Cadical: `/home/one/.elan/toolchains/leanprover--lean4---v4.29.1/bin/cadical`
  (2.1.2), exactly as the existing pipeline uses it. Only the producer (trusted
  to be untrusted) invokes it; no check-time path exists yet.
- Disk: the run used ~70 MB under git-ignored `.bench/c61/`; 11 GB free after.

## Appendix — the trimming probe (scratch, untrusted)

`.bench/c61/trim.rs`, compiled with `rustc -O`, run as
`./trim .bench/c61/work/*.lrat`. It is measurement code, not part of the
pipeline, and it is reproduced here so the numbers above are reproducible
without the git-ignored tree.

```rust
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;

fn main() {
    for path in env::args().skip(1) {
        let text = fs::read_to_string(&path).expect("read");
        let mut order: Vec<usize> = Vec::new();
        let mut lines: Vec<&str> = Vec::new();
        let mut hints: Vec<Vec<i64>> = Vec::new();
        for line in text.lines() {
            if line.is_empty() { continue; }
            let toks: Vec<&str> = line.split(' ').collect();
            if toks.len() >= 2 && toks[1] == "d" { continue; } // deletions
            let id: i64 = toks[0].parse().expect("id");
            let pos = toks.iter().position(|t| *t == "0").expect("lit 0");
            let mut h = Vec::new();
            for t in &toks[pos + 1..] {
                let v: i64 = t.parse().expect("hint");
                if v != 0 { h.push(v); }
            }
            order.push(id as usize);
            lines.push(line);
            hints.push(h);
        }
        let index: HashMap<usize, usize> =
            order.iter().enumerate().map(|(i, id)| (*id, i)).collect();
        let last = order.len() - 1;
        let mut keep = HashSet::new();
        let mut stack = vec![last];
        while let Some(i) = stack.pop() {
            if !keep.insert(i) { continue; }
            for h in &hints[i] {
                if let Some(j) = index.get(&(*h as usize)) {
                    if !keep.contains(j) { stack.push(*j); }
                }
            }
        }
        let total: usize = lines.iter().map(|l| l.len() + 1).sum();
        let trimmed: usize = keep.iter().map(|i| lines[*i].len() + 1).sum();
        println!("{} lines={} kept={} full_bytes={} trimmed_bytes={} ratio={:.2}",
            path, order.len(), keep.len(), total, trimmed,
            total as f64 / trimmed as f64);
    }
}
```
