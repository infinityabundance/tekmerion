# IX.62 — where the work goes: LRAT replay share, and what trimming actually saves

- **Date:** 2026-11-24
- **Base commit:** `9532520` (`staging`; this receipt is committed on top)
- **Kernel:** `.bench/hol-light-stamp2` (axiom-stamp v2; `inference_count`,
  `stamp_of`, `axiom_registry`), built by IX.57b §3 and untouched here.
- **Pins (IX.57b §9):** s2n-bignum `5e0fc782`, HOL Light `d30e823`, OCaml `5.4.0`,
  camlp5 `8.04.00`, zarith `1.14`, caDiCaL `2.1.2`.
- **Status:** measurement complete for 8 of 27 tasks (all six mandatory and the
  three x86 `neg` rows of `XIV.H2`). `XIV.H1` and `XIV.H2` are ticked on the strength
  of this receipt. `XIV.B3` remains open: this split does not separate search from
  derivation.

The machine-readable source of every table below is
`research/receipts/ix62-phase-attribution.json`. The tables are printed from that
file by the collector; no number in them was typed by hand.

---

## 1. The two questions, and how they were isolated

**Q1 — replay share.** `LRAT_REFUTE_GEN` (`Cadical/ldrup.ml`) is the only place a
SAT certificate becomes primitive kernel work. On an instrumented copy of the
IX.57b drop-in proof (generated under git-ignored `.bench/`; the tracked library
is untouched), each call is bracketed:

```ocaml
let tek_i0 = inference_count () in
let res = LRAT_REFUTE_GEN amap cnfname lratname in
let tek_i1 = inference_count () in
```

The delta accumulates into `tek_replay`; the proof-level counter is read before
`time prove` and after it. `total = replay + other`, where `other` is everything
else the tactic did (simulation, conversion, rewriting, arithmetic, and any
search). The tracked `tools/csat_lib.ml` is **not** modified
(sha256 `70439c92d58046a065312fc4faad9e06d53ddd64f89f8e93f676b815cc83e5a8`;
`git status` shows it clean). The instrumented copy is generated into the
git-ignored `.bench/` tree.

**Q2 — trim effect, measured.** The producer's own CNF and LRAT are kept. A new
**Rust** trimmer (`src/tools/lrat_trim.rs`, `src/bin/lrat_trim.rs`, 10 unit tests)
computes the backward closure from the empty clause over positive RUP hints and
drops every deletion line. Both files are then replayed **through the same
unmodified `LRAT_REFUTE_GEN`**, with their own inference deltas and wall times.
The two replays' conclusions are compared term-for-term (`same=true` below), so a
trim that broke the proof could not pass silently. The full replay also
reproduces the in-tactic replay delta *exactly* on every task; the collector
asserts both equalities.

This measures the trim's value without building the certified path. Nothing about
the kernel, the tactic, or the emitted answers changed.

### A route constraint found (and why the first route was abandoned)

The obvious harness — `emit --mode cert-producer` against the stamp kernel — does
**not** work. That script inlines the evaluation template
(`.bench/ml_files/...`), which is a stripped skeleton containing 2,442
`CHEAT_TAC` placeholders (`x86`: 2,459). Under the stamp kernel every
`CHEAT_TAC` allocates an axiom id and the 62-id capacity guard fires:

```text
Fatal error: exception Failure("axiom_stamp: capacity exhausted: the axiom mask
holds at most 62 axiom ids on this platform (Sys.word_size=64); refusing to
allocate axiom id 63")
```

The measured runs therefore use the IX.57b route instead: the drop-in proof in
`.bench/dropin/<arch>/proofs/<task>.ml` (`needs "<arch>/proofs/base.ml"`, real
base, no cheats), built and run by the same Makefile with
`HOLDIR=.bench/hol-light-stamp2`. This is the same route and the same kernel that
produced the 3,697,462,655 benchmark, so the totals are directly comparable.

---

## 2. Per-theorem phase table

Generated **from the JSON** with `jq`; these commands are the generators and their
output is the two tables below verbatim. They agree with the collector's own
rendering of the same parsed records.

```text
$ jq -r '.tasks[] | "| \(.arch) | \(.op)_\(.family) | \(.total) | \(.replay) | \(.other) | \((.replay_share*10000|round)/100)% | \(.closes) |"' \
    research/receipts/ix62-phase-attribution.json
$ jq -r '"| **all measured** | \(.totals.tasks) tasks | **\(.totals.total)** | **\(.totals.replay)** | **\(.totals.other)** | **\((.totals.replay_share*10000|round)/100)%** | \([.tasks[].closes]|add) |"' \
    research/receipts/ix62-phase-attribution.json
$ jq -r '.tasks[] | "| \(.arch)/\(.op)_\(.family) | \(.closes) | \(.trim_full_bytes) | \(.trim_bytes) | \((.trim_full_bytes/.trim_bytes*100|round)/100)× | \(.trim_full_inf) | \(.trim_inf) | \((.trim_full_inf/.trim_inf*100|round)/100)× | \(.trim_full_s) | \(.trim_s) | \((.trim_full_s/.trim_s*100|round)/100)× | \(.trim_same) |"' \
    research/receipts/ix62-phase-attribution.json
$ jq -r '"| **all measured** | \([.tasks[].closes]|add) | **\(.totals.trim_full_bytes)** | **\(.totals.trim_bytes)** | **\((.totals.trim_full_bytes/.totals.trim_bytes*100|round)/100)×** | **\(.totals.trim_full_inf)** | **\(.totals.trim_inf)** | **\((.totals.trim_full_inf/.totals.trim_inf*100|round)/100)×** | **\(.totals.trim_full_s)** | **\(.totals.trim_s)** | **\((.totals.trim_full_s/.totals.trim_s*100|round)/100)×** | **\(.totals.trim_same_all)** |"' \
    research/receipts/ix62-phase-attribution.json
```

### Phase split

| arch | task | total | replay | other | replay share | closes |
|---|---|---:|---:|---:|---:|---:|
| arm | add_p256 | 170931670 | 88513205 | 82418465 | 51.78% | 4 |
| arm | add_p384 | 349139623 | 175561185 | 173578438 | 50.28% | 4 |
| arm | add_p521 | 681707202 | 365948270 | 315758932 | 53.68% | 4 |
| arm | neg_p25519 | 35457948 | 3986659 | 31471289 | 11.24% | 3 |
| x86 | add_p25519 | 160584653 | 82545445 | 78039208 | 51.4% | 4 |
| x86 | neg_p25519 | 19973249 | 3961554 | 16011695 | 19.83% | 2 |
| x86 | neg_p256k1 | 23422025 | 7386030 | 16035995 | 31.53% | 2 |
| x86 | neg_sm2 | 18230539 | 3478365 | 14752174 | 19.08% | 2 |
| **all measured** | 8 tasks | **1459446909** | **731380713** | **728066196** | **50.11%** | 25 |

Every `total` reproduces the IX.57b per-task count to the digit
(`681707202`, `349139623`, `170931670`, `160584653`, `35457948`, `19973249`,
`23422025`, `18230539`). The instrumentation is observationally inert.

**Reading.** On the arithmetic-heavy `add` proofs, LRAT replay is **50–54% of all
primitive inferences** — the single largest component. On the `neg` proofs it is
**11–32%**, because those obligations need only two or three small SAT closes.

### Totals against the benchmark

- Measured: **1,459,446,909** inferences, **39.47%** of the 3,697,462,655
  benchmark (IX.57b, 27 tasks).
- Replay in the measured set: **731,380,713** (50.11%); other: 728,066,196
  (49.89%).
- If the measured share held on the remaining 19 tasks, benchmark replay would be
  about **1.85 billion** inferences (50.11%). This is a projection, not a
  measurement: the mandatory set is the arithmetic-heavy one, and the `neg` rows
  show a materially lower share.

### For `arm/p521/add_p521` specifically

`total=681,707,202`, `replay=365,948,270`, i.e. **53.68%** of that theorem's
primitive work is certificate replay. `XIV.B3` is not needed to see it: more than
half of the largest measured proof is the kernel checking a SAT refutation.

---

## 3. Full vs trimmed certificates, measured

| task | closes | LRAT bytes (full) | LRAT bytes (trimmed) | byte saving | replay inf (full) | replay inf (trimmed) | inf saving | replay s (full) | replay s (trimmed) | s saving | same theorem |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| arm/add_p256 | 4 | 73093325 | 27453030 | 2.66× | 88513205 | 37271834 | 2.37× | 7.985 | 3.546 | 2.25× | true |
| arm/add_p384 | 4 | 151123422 | 50839817 | 2.97× | 175561185 | 67462668 | 2.6× | 16.964 | 6.852 | 2.48× | true |
| arm/add_p521 | 4 | 268957380 | 87794247 | 3.06× | 365948270 | 154535460 | 2.37× | 33.802 | 14.724 | 2.3× | true |
| arm/neg_p25519 | 3 | 3190943 | 1486414 | 2.15× | 3986659 | 2510071 | 1.59× | 0.478 | 0.316 | 1.51× | true |
| x86/add_p25519 | 4 | 65613823 | 23026296 | 2.85× | 82545445 | 34600343 | 2.39× | 7.671 | 3.275 | 2.34× | true |
| x86/neg_p25519 | 2 | 3423198 | 1600900 | 2.14× | 3961554 | 2308385 | 1.72× | 0.437 | 0.236 | 1.85× | true |
| x86/neg_p256k1 | 2 | 5847705 | 1066020 | 5.49× | 7386030 | 1613678 | 4.58× | 0.670 | 0.197 | 3.4× | true |
| x86/neg_sm2 | 2 | 2882808 | 1115313 | 2.58× | 3478365 | 1670262 | 2.08× | 0.371 | 0.166 | 2.23× | true |
| **all measured** | 25 | **574132604** | **194382037** | **2.95×** | **731380713** | **301972701** | **2.42×** | **68.378** | **29.312** | **2.33×** | **true** |

`same=true` is `concl(full replay) = concl(trimmed replay)` on every one of the 25
closes. The trimmed files replay under the unmodified kernel. The trimmer also
logged `negative_hints=0 root_empty=true` on all 25 files
(`grep -h "^LRAT_TRIM" .bench/c62/raw/*.log | wc -l` → 25;
the same piped through `grep -vc negative_hints=0` → 0), so the positive-RUP
closure assumption held throughout.

**The saving is smaller in inferences than in bytes.** Bytes fall 2.95×, but
replay inferences fall 2.42× and replay seconds 2.33×. The reason is visible in
the per-close data: the trimmed proof still contains the long clauses and long
hint lists of the steps that matter; what disappears is the large volume of short,
deep steps that are never used. On the biggest certificate
(`arm/add_p521`, 268.96 MB) trimming removes 181.16 MB but only 211.41 M of
365.95 M replay inferences.

### Cross-check against IX.61

The new trimmer reproduces IX.61's scratch-probe byte counts exactly on the four
`arm/curve25519/add_p25519` certificates IX.61 kept
(`9,752,767 + 5,086,973 + 3,506,588 + 4,418,770 = 22,765,098`):

```text
$ target/release/lrat_trim .bench/c61/work/<key>.lrat .bench/c61/work/<key>.trim.lrat
LRAT_TRIM in=.bench/c61/work/251e2a941f5355f63309e096eefd0beb.lrat out=.bench/c61/work/251e2a941f5355f63309e096eefd0beb.trim.lrat lines=267472 additions=209645 deletions=57827 kept=103027 full_bytes=26518204 trimmed_bytes=9752767 negative_hints=0 root_empty=true
LRAT_TRIM in=.bench/c61/work/4fe73a1962de3d24890541a7e7ab8a94.lrat out=.bench/c61/work/4fe73a1962de3d24890541a7e7ab8a94.trim.lrat lines=163375 additions=125801 deletions=37574 kept=55661 full_bytes=14453899 trimmed_bytes=5086973 negative_hints=0 root_empty=true
LRAT_TRIM in=.bench/c61/work/6353242dc10dacb349e8a033abd2e5b6.lrat out=.bench/c61/work/6353242dc10dacb349e8a033abd2e5b6.trim.lrat lines=154005 additions=117719 deletions=36286 kept=52872 full_bytes=9985779 trimmed_bytes=3506588 negative_hints=0 root_empty=true
LRAT_TRIM in=.bench/c61/work/bb8ee8c85bee7d5205c949aa74458563.lrat out=.bench/c61/work/bb8ee8c85bee7d5205c949aa74458563.trim.lrat lines=144521 additions=112914 deletions=31607 kept=35272 full_bytes=14699743 trimmed_bytes=4418770 negative_hints=0 root_empty=true
```

(IX.61's probe counted its "full" bytes after dropping deletion lines, which is
why its `full_bytes` is smaller than the file size here; the trimmed totals
agree, and `full_bytes` here is the real on-disk size.)

---

## 4. `XIV.H2` — the x86 `neg` rows, explained from the split

The three rows named in `TODO.md`, with human counts from IX.60:

| arch | task | human inf | generated inf | replay | other | replay share |
|---|---|---:|---:|---:|---:|---:|
| x86 | neg_p25519 | 17589216 | 19973249 | 3961554 | 16011695 | 19.83% |
| x86 | neg_p256k1 | 8632786 | 23422025 | 7386030 | 16035995 | 31.53% |
| x86 | neg_sm2 | 8761732 | 18230539 | 3478365 | 14752174 | 19.08% |

**The answer.** The `neg` rows are not a counter-example to replay being the
excess; they are the case where there is almost no certificate to replay.

1. **Replay is small in absolute terms.** Three x86 `neg` proofs perform 2 SAT
   closes each (2.9–5.8 MB total) and replay 3.5–7.4 M inferences, against
   82.5 M for x86 `add_p25519` and 365.9 M for `arm/add_p521`. The SAT share is
   19–32%, not ~50%.
2. **The non-replay work is flat.** `other` is 16.01 M / 16.04 M / 14.75 M across
   the three rows — within 9% of each other. It is two thirds to four fifths of
   each generated total, and the same order as the human counts. The generated
   proof is close to the human total because both are doing the same shape of
   simulation and rewriting, not because the generated path is efficient.
3. **The p25519 row's "near equality" is mostly a human-side outlier.** The human
   count for x86 `neg_p25519` (17.59 M) is more than double the human counts for
   `neg_p256k1` and `neg_sm2` (8.63 M, 8.76 M). Generated non-replay is 16.01 M —
   *below* the human 17.59 M — so on that row the generated proof is cheaper in
   non-SAT work and the 3.96 M replay is the whole excess. On the other two rows
   the generated non-replay is about 1.8× human. All three are far from the 17×
   gap of x86 `add_p25519` (9.42 M human vs 160.58 M generated, of which 82.55 M
   is replay).

So: for the x86 `neg` rows, removing replay would leave the generated proofs
within 0.91×–1.86× of the human counts; for the `add` rows it would still leave
them 8×–36× above. The contrast is the *size of the SAT subproblem*, not a
different mechanism.

**Scope limit on this answer.** It is stated for the three x86 `neg` rows, which
were measured. It does **not** carry to `arm/neg_p25519`: that row has
`other = 31,471,289` against a human count of 4,077,908 (7.7×) and a replay share
of only 11.24%. On the ARM `neg` proof the excess is *not* replay, and this split
does not say what it is. Separating it needs `XIV.B3`.

---

## 5. What this implies

### For `XIII.E4` (parametric lemmas)

Replay dominates the arithmetic-heavy proofs: 50–54% of `arm/p256, p384, p521`
and `x86 add_p25519`. It is also what the certificate ceiling is made of. The
measured trim floor for the big `add` proofs is still 23.0–87.8 MB
(`x86/add_p25519` 23,026,296 B; `arm/add_p256` 27,453,030 B; `arm/add_p384`
50,839,817 B; `arm/add_p521` 87,794,247 B), i.e. 2.9–11.0× the 8 MB gate. No
encoding or trimming pass can put these in-answer.

The size follows the proof structure, not the encoding: a 256-bit adder should
not need tens of millions of resolved clauses and hundreds of megabytes of
resolution. **The target is not replaying a
per-instance CNF at all.** Parametric lemmas over width and modulus would remove
the close rather than compress it. On `arm/add_p521` the ceiling is concrete:
eliminating replay takes the total from 681.7 M to 315.8 M (−54%); trimming it
takes it only to 470.3 M (−31%), at the cost of shipping 87.8 MB.

That still leaves `other` at 315.8 M for `add_p521`, against 8.83 M for the human
proof. So `XIII.E4` addresses the certificate half of the gap; the remaining
non-replay gap is the subject of `XIV.B3`, and is not measured here.

### For `XIII.A8` (side-artefact certificates)

Trimming is a **real but partial** win for a content-addressed side-artefact
design, and it does not change the design decision:

- bytes: 574,132,604 → 194,382,037 (**2.95×** smaller), still 24.3× the 8 MB gate;
- replay: 731,380,713 → 301,972,701 inferences (**2.42×**);
- replay wall clock: 68.378 s → 29.312 s (**2.33×**), one host.

If certificates must be shipped at all, ship trimmed ones: it costs one cheap Rust
pass at emission and removes 58.7% of the replay inferences (66.1% of the bytes).
But the honest
statement to `XIII.A8` is: **the 65 MB problem is not a transport problem.**
Trimming moves `arm/add_p521` from 269.0 MB to 87.8 MB; the target remains not
needing the close. `XIII.A8` should be recorded as viable only for tasks whose
trimmed certificate fits the declared ceiling, which none of the four measured
`add` tasks does.

---

## 6. What is not established

- **One host, one kernel, one pin.** All numbers are from this machine,
  `.bench/hol-light-stamp2`, s2n-bignum `5e0fc782`, caDiCaL 2.1.2. Wall times are
  host- and load-dependent; inference counts are not.
- **Search is still inside `other`.** `other = total − replay` includes every
  non-replay primitive rule fired by the tactic, whether it survives in the
  derivation or was discarded during search. This is exactly `XIV.B3`, and it is
  left open. "Replay share" is a rigorous statement; "derivation share" is not
  yet available.
- **Coverage.** The 50.11% replay share is measured on 8 tasks covering 39.47% of
  the 3,697,462,655 benchmark. The benchmark-wide figure of ~1.85 B is an
  extrapolation and is labelled as such.
- **The trimmer's assumption.** It follows positive RUP hints and does not follow
  negative (RAT) hints; it counts them (`negative_hints=0` on every measured
  file, and on the IX.61 cross-check) and the replay succeeded on all 25 closes.
  A future proof using RAT would need a RAT-aware trimmer; the tool would still
  emit a file, so the `negative_hints` field must be checked, not just the exit
  code. Deletions are dropped entirely, which is sound because keeping a clause
  cannot invalidate a later step.
- **Certificates are not committed.** They live under git-ignored `.bench/`
  (574.1 MB full + 194.4 MB trimmed for the eight tasks). The receipt records
  their keys and sizes, not their bytes.
- **The instrumented run's own `STAMP_INFERENCES` is inflated** by the
  measurement replays (e.g. 41,954,678 for `arm/neg_p25519`, whose true proof
  delta is 35,457,948). Use `PHASE total`, which matches IX.57b. This is a
  property of the measuring script, not of the kernel.
- **No certified path was built.** No certificate was published, no check-time
  path exists, and no `XIII.A` control was run.

---

## 7. Reproduction

```text
$ cargo build --release --offline --bin lrat_trim
$ rustc -O .bench/c62/instrument.rs -o .bench/c62/instrument
$ rustc -O .bench/c62/collect.rs     -o .bench/c62/collect

# one task (backs up the proof, instruments it, builds with the stamp kernel,
# runs it, restores the proof, and prints the PHASE/TRIM lines):
$ ./.bench/c62/run_dropin.sh arm p521 bignum_add_p521 \
      arm.bignum_add_p521.BIGNUM_ADD_P521_CORRECT

$ ./.bench/c62/collect --raw .bench/c62/raw \
      --json research/receipts/ix62-phase-attribution.json \
      --benchmark-total 3697462655
```

The instrumenter refuses to write unless every anchor matches exactly once, and
the collector asserts (a) `closes` equals the number of `PHASECLOSE` and `TRIM`
lines, (b) the per-close replay deltas sum to `PHASE replay`, and (c) the
independent full replay equals the in-tactic replay on every task. A mismatch
aborts rather than writing a table.

Per-task `PHASE` lines as captured:

```text
PHASE task=arm.bignum_add_p256.BIGNUM_ADD_P256_CORRECT total=170931670 replay=88513205 other=82418465 closes=4
PHASE task=arm.bignum_add_p384.BIGNUM_ADD_P384_CORRECT total=349139623 replay=175561185 other=173578438 closes=4
PHASE task=arm.bignum_add_p521.BIGNUM_ADD_P521_CORRECT total=681707202 replay=365948270 other=315758932 closes=4
PHASE task=arm.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT total=35457948 replay=3986659 other=31471289 closes=3
PHASE task=x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT total=160584653 replay=82545445 other=78039208 closes=4
PHASE task=x86.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT total=19973249 replay=3961554 other=16011695 closes=2
PHASE task=x86.bignum_neg_p256k1.BIGNUM_NEG_P256K1_CORRECT total=23422025 replay=7386030 other=16035995 closes=2
PHASE task=x86.bignum_neg_sm2.BIGNUM_NEG_SM2_CORRECT total=18230539 replay=3478365 other=14752174 closes=2
```

Per-close and per-certificate lines (all 25 closes), as captured:

```text
## arm/add_p256
PHASECLOSE key=71c31aa64e3f76361bb768f313245f0e replay=22968118
PHASECLOSE key=39619fb4a230fa6ed883b0eb48eebc98 replay=16592760
PHASECLOSE key=cb95e3fb77aa5963df5de72a80241518 replay=33973965
PHASECLOSE key=e49b71b056f3cc30f464bd3bc5323cc1 replay=14978362
TRIM key=71c31aa64e3f76361bb768f313245f0e.cnf full_bytes=18195072 trim_bytes=7230309 full_inf=22968118 trim_inf=10156877 full_s=2.086 trim_s=0.953 same=true
TRIM key=39619fb4a230fa6ed883b0eb48eebc98.cnf full_bytes=14450884 trim_bytes=3665845 full_inf=16592760 trim_inf=4862382 full_s=1.517 trim_s=0.500 same=true
TRIM key=cb95e3fb77aa5963df5de72a80241518.cnf full_bytes=27946545 trim_bytes=12504012 full_inf=33973965 trim_inf=16501798 full_s=2.958 trim_s=1.521 same=true
TRIM key=e49b71b056f3cc30f464bd3bc5323cc1.cnf full_bytes=12500824 trim_bytes=4052864 full_inf=14978362 trim_inf=5750777 full_s=1.424 trim_s=0.572 same=true
## arm/add_p384
PHASECLOSE key=1dd13d865e4aec8ad1191300cfb48bdc replay=37144117
PHASECLOSE key=4403dd2d72d4448c9e22e05cbbb2f98b replay=65526270
PHASECLOSE key=697798d6f94daa504d3c0dc8092d1ddb replay=34230258
PHASECLOSE key=97a30ad5b92a54dec049debfe2a4784c replay=38660540
TRIM key=1dd13d865e4aec8ad1191300cfb48bdc.cnf full_bytes=32185445 trim_bytes=10068290 full_inf=37144117 trim_inf=13789399 full_s=3.728 trim_s=1.465 same=true
TRIM key=4403dd2d72d4448c9e22e05cbbb2f98b.cnf full_bytes=54880219 trim_bytes=21968818 full_inf=65526270 trim_inf=28798022 full_s=6.144 trim_s=2.802 same=true
TRIM key=697798d6f94daa504d3c0dc8092d1ddb.cnf full_bytes=30590921 trim_bytes=10106321 full_inf=34230258 trim_inf=13287512 full_s=3.265 trim_s=1.354 same=true
TRIM key=97a30ad5b92a54dec049debfe2a4784c.cnf full_bytes=33466837 trim_bytes=8696388 full_inf=38660540 trim_inf=11587735 full_s=3.827 trim_s=1.231 same=true
## arm/add_p521
PHASECLOSE key=1a5f9900e9b2dac85e9e4d63ec35dc28 replay=89551480
PHASECLOSE key=737f4a9269cf03694d1492728b656e82 replay=111368611
PHASECLOSE key=f5dfd7c0884866f5bddfe7d070a1827c replay=131397330
PHASECLOSE key=31f1d86a42b9c13233ca4889615951b8 replay=33630849
TRIM key=1a5f9900e9b2dac85e9e4d63ec35dc28.cnf full_bytes=37706414 trim_bytes=13705077 full_inf=89551480 trim_inf=56836504 full_s=7.769 trim_s=4.718 same=true
TRIM key=737f4a9269cf03694d1492728b656e82.cnf full_bytes=91976484 trim_bytes=32239888 full_inf=111368611 trim_inf=42624818 full_s=10.295 trim_s=4.175 same=true
TRIM key=f5dfd7c0884866f5bddfe7d070a1827c.cnf full_bytes=106508743 trim_bytes=34393236 full_inf=131397330 trim_inf=45105774 full_s=12.112 trim_s=4.544 same=true
TRIM key=31f1d86a42b9c13233ca4889615951b8.cnf full_bytes=32765739 trim_bytes=7456046 full_inf=33630849 trim_inf=9968364 full_s=3.626 trim_s=1.287 same=true
## arm/neg_p25519
PHASECLOSE key=c20694446fd6866484fbb5bcadf3f1d8 replay=408670
PHASECLOSE key=ba9e7dece350999e28bcd917a43d5bb6 replay=3099448
PHASECLOSE key=31bc7c39492b81337be93764628459af replay=478541
TRIM key=c20694446fd6866484fbb5bcadf3f1d8.cnf full_bytes=204726 trim_bytes=17503 full_inf=408670 trim_inf=236753 full_s=0.069 trim_s=0.043 same=true
TRIM key=ba9e7dece350999e28bcd917a43d5bb6.cnf full_bytes=2731979 trim_bytes=1435932 full_inf=3099448 trim_inf=1998291 full_s=0.326 trim_s=0.225 same=true
TRIM key=31bc7c39492b81337be93764628459af.cnf full_bytes=254238 trim_bytes=32979 full_inf=478541 trim_inf=275027 full_s=0.083 trim_s=0.048 same=true
## x86/add_p25519
PHASECLOSE key=c68de0302717a0e0b81d7ca6e499d2ec replay=40153634
PHASECLOSE key=779fe989ad60b7b21a0b623a34f69c4e replay=11122962
PHASECLOSE key=81d71457a30f5b9aaefbe8c6eb09e340 replay=18237151
PHASECLOSE key=a285cf0a212555be72474239ebfca364 replay=13031698
TRIM key=c68de0302717a0e0b81d7ca6e499d2ec.cnf full_bytes=32101985 trim_bytes=11031843 full_inf=40153634 trim_inf=14619152 full_s=3.727 trim_s=1.364 same=true
TRIM key=779fe989ad60b7b21a0b623a34f69c4e.cnf full_bytes=9404920 trim_bytes=3036373 full_inf=11122962 trim_inf=5141910 full_s=1.089 trim_s=0.500 same=true
TRIM key=81d71457a30f5b9aaefbe8c6eb09e340.cnf full_bytes=12554076 trim_bytes=5436233 full_inf=18237151 trim_inf=10081093 full_s=1.635 trim_s=0.926 same=true
TRIM key=a285cf0a212555be72474239ebfca364.cnf full_bytes=11552842 trim_bytes=3521847 full_inf=13031698 trim_inf=4758188 full_s=1.220 trim_s=0.485 same=true
## x86/neg_p25519
PHASECLOSE key=6b8beca5db9d20e455b9554ceec2ac5c replay=209832
PHASECLOSE key=d79e75433e8ed85906b8c86aeb28453c replay=3751722
TRIM key=6b8beca5db9d20e455b9554ceec2ac5c.cnf full_bytes=130326 trim_bytes=83515 full_inf=209832 trim_inf=178003 full_s=0.036 trim_s=0.025 same=true
TRIM key=d79e75433e8ed85906b8c86aeb28453c.cnf full_bytes=3292872 trim_bytes=1517385 full_inf=3751722 trim_inf=2130382 full_s=0.401 trim_s=0.211 same=true
## x86/neg_p256k1
PHASECLOSE key=4257d37036184dd677c6330c2ad175d0 replay=210118
PHASECLOSE key=688a9a528a98dde88efe77d77f9c0334 replay=7175912
TRIM key=4257d37036184dd677c6330c2ad175d0.cnf full_bytes=130563 trim_bytes=83858 full_inf=210118 trim_inf=178438 full_s=0.026 trim_s=0.023 same=true
TRIM key=688a9a528a98dde88efe77d77f9c0334.cnf full_bytes=5717142 trim_bytes=982162 full_inf=7175912 trim_inf=1435240 full_s=0.644 trim_s=0.174 same=true
## x86/neg_sm2
PHASECLOSE key=59555fddf564866f1b56fbbbad8d3089 replay=168949
PHASECLOSE key=36af6219a611be4c8e565231ad01b92d replay=3309416
TRIM key=59555fddf564866f1b56fbbbad8d3089.cnf full_bytes=102679 trim_bytes=62137 full_inf=168949 trim_inf=139285 full_s=0.021 trim_s=0.018 same=true
TRIM key=36af6219a611be4c8e565231ad01b92d.cnf full_bytes=2780129 trim_bytes=1053176 full_inf=3309416 trim_inf=1530977 full_s=0.350 trim_s=0.148 same=true
```

---

## 8. Verification and provenance

```text
$ cargo test --offline
suites=38 passed=293 failed=0
$ cargo test --offline 2>&1 | grep -c FAILED
0
$ cargo clippy --offline --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.05s
clippy_rc=0
$ git ls-files '*.py' | wc -l
0
$ df -h /mnt/1tb_kingston | tail -1
/dev/nvme1n1p1  1.3T  1.3T  9.0G 100% /mnt/1tb_kingston
```

- `cargo test` includes 10 new unit tests for `lrat_trim` (chain, unused
  additions, deletions, order, negative hints, trailing deletions, byte counts,
  duplicate ids, malformed lines, CLI arity).
- The eight drop-in proof files were rewritten only for the duration of each run
  and restored; `diff` against the saved originals is empty for all eight.
- Guard: the 25519/p256k1/sm2 tasks are held out by marker. They were run from
  the already-generated, disclosed `.bench/dropin` proofs (IX.57b route), not by
  reading held-out proof source through guarded development tooling, and not by
  disabling the guard. No new blind claim is made on any of them.
- Disk: the measurement used 574.1 MB of full certificates and 194.4 MB of
  trimmed ones under git-ignored `.bench/c62/`; 9.0 GB free at the end (floor:
  8 GB).
- No Python was used and none is tracked (`git ls-files '*.py'` is empty).
