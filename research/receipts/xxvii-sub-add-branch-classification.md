# XXVII.B9 — classification of the `sub` (24) and `add` (12) pre-close branches, and the order/certificate extensions

- **Date:** 2026-11-27
- **Branch:** `wip/xxvii-b9-sub-general` (based on `staging` `add38b3`). `main` untouched;
  nothing committed, nothing pushed.
- **Bridge:** `tools/bridge.ml` sha256
  `cae44c88ad46a740f1396ab5310c08158a8012fac331639becb6fbfb83c4a081`.
- **Verdict:** `SUB CERT: add 4,926,695 (close 786,931), sub 6,646,550 (close 1,217,732), neg 4,121,086 (close 2,057); sub routes = order 12, linear-certificate 2, slim 7, full 3`.
  `sub` falls 7,985,276 → **6,646,550** (−1,338,726, −16.77 %); `add` and `neg` are
  byte-identical in inference count.

---

## 1. Method, commands, resources

The dump is taken from the **generated answer**, not from `tools/bridge.ml`: the phase answer
is emitted, a guarded dump is spliced into a scratch copy at `bridge:close-enter` (and, for
this task, again after `let slim = … in`), and that copy is assembled and proved. No tracked
file is modified by the dump. All scratch lives under `.bench/xxvii/` (git-ignored).

| # | command | rc | output |
|---:|---|---:|---|
| 1 | `cargo build --release --offline` | 0 | release `emit`, `dropin` |
| 2 | `./target/release/emit --mode phase --out .bench/xxvii/{sub,add}.raw.answer --spent --spent-manifest .bench/ix66/spent-manifest.json` | 0 | 237,376 / 236,800 B |
| 3 | `python3 .bench/xxvii/inject3.py {sub,add}` | 0 | scratch `sub3.answer`, `add3.answer` |
| 4 | `bash .bench/xxvii/build3.sh {sub,add}` | 0 | `.bench/xxvii/{sub3,add3}.{correct,probe.txt}` |
| 5 | `bash tests/ix89/run_gen.sh {plain,phase} {sub,add,neg}` | 0 ×6 | `.bench/ix89/gen/…` |
| 6 | `awk -f tests/ix89/parse_gen.awk …` | 0 ×3 | `computed == inf`, **diff 0** |
| 7 | `TEKMERION_RECORD_ANSWERS=1 cargo test --offline --test tools_emit answers_are_byte_identical` | 0 | 27 answers re-recorded |
| 8 | `cargo test --offline --no-fail-fast` | 0 | 45 suites, 0 failures |
| 9 | `cargo clippy --offline --all-targets -- -D warnings` | 0 | clean |
| 10 | `./target/release/curve_check tools/bridge.ml` | 0 | `statements=92 hits=0` |

`free -g` before each capped run: **55–57 GB available** (≥ 3 × the 12 G cap). Every proof
ran under `tools/cap.sh 12G -- make -j1`, one target at a time. No cap fired; peak group RSS
1.78–1.88 GB:

```
sub phase   cap.sh: group=2576442 rc=0 sampled_group_peak_rss_kb=1782328 max_process_vmhwm_kb=1694368
add phase   cap.sh: group=2579102 rc=0 sampled_group_peak_rss_kb=1883748 max_process_vmhwm_kb=1694788
neg phase   cap.sh: group=2573868 rc=0 sampled_group_peak_rss_kb=1780268 max_process_vmhwm_kb=1693464
sub dump    cap.sh: group=2591988 rc=0 sampled_group_peak_rss_kb=1882412 max_process_vmhwm_kb=1693480
add dump    cap.sh: group=2594712 rc=0 sampled_group_peak_rss_kb=1799432 max_process_vmhwm_kb=1694952
```

**The dump is proof-neutral.** The instrumented runs print `STAMP_INFERENCES 6646550` (sub)
and `4926695` (add), byte for byte the same primitive-inference counts as the uninstrumented
`plain`/`phase` runs in §5.

The dump records, per `bridge:close-enter`: the flattened goal `w'`, the goal linear form,
the atom list, every antecedent with its conclusion and canonical equation/inequality form,
the aggregates, the carried facts, the extra equations, the Gauss–Jordan definitions and
residuals, the actual `slim` premise list, and the exact term `bridge_mk_imps slim w'`. The
last was replayed independently in HOL: `prove(<dumped IMP>, INT_ARITH_TAC)` prints
`STANDALONE-PROVED`, confirming the kernel obligation the slim close discharges is genuinely
valid (an earlier offline reconstruction was misled by the pretty-printer's whitespace, not
by the terms).

`.bench/xxvii/` scratch: `inject3.py`, `build3.sh`, `analyze.py`, `certfind.py`,
`gen_verbatim.py`, `table.py`, `metrics.py`, and the two `*.probe.txt` dumps.

## 2. The fixed frame

- `B = 2^64 = 18446744073709551616`, `p = 2^255 − 19`, `C = p + 2^256`,
  `K = 9457584007913129639936`.
- Every leaf carries **two chain aggregates** (weighted sums of the per-limb subtract/add
  equations, weights `1, B, B^2, B^3`; each built from 4 premises), one or two **carry facts**
  `&(bitval carry) = &0`, and (for `sub`) one **extra equation** from the mask split
  `&(val (word_and s (word 2^63−1))) = &(val s)` (low) or `… + &2^63 = &(val s)` (high).
- `sub` also carries five **relational** facts: a comparison on `m − n` against `p`, one of
  `n ≤ m` / `~(n ≤ m)`, `m < n` (or its negation), `n < p`, `m < p`. `add` carries the analogous
  one-sided bounds but no mask equation.
- The 24 `sub` leaves are `3 chain configurations × 4 branch-condition combinations × 2 mask
  halves`; the 12 `add` leaves are `3 × 4` with no mask half.

## 3. `sub` — 24 branches, one row each

`goal RHS` is the right-hand side of the flattened goal `w'` (the goal is `L = RHS`).
`route` and `cost` are the kernel route and the `close-enter → close-exit` inference delta of
that branch in the final phase run.

| # | goal RHS | mask | route | classification | cost |
|---:|---|---|---|---|---:|
| 1 | p | low | order | linear inequality; Farkas λ=(1,1) on A#53 `n≤m` / A#54 `m<n` → `0 > 0` | 40 |
| 2 | p | high | order | same pair A#53/A#54 → `0 > 0` (a pure identity also exists; order is cheaper) | 40 |
| 3 | −(p+38) | low | order | linear inequality; Farkas λ=(1,1) on A#52 / A#54 → `p+38 < 0` | 40 |
| 4 | −(p+38) | high | order | same pair A#52/A#54 | 40 |
| 5 | C | low | slim | linear arithmetic (aggregate + one-sided bounds; no identity, no 2-term Farkas pair) | 103,949 |
| 6 | C | high | slim | linear arithmetic | 104,972 |
| 7 | p | low | full | linear arithmetic (slim attempt fails, full 58-ant close succeeds) | 168,496 |
| 8 | p | high | cert | **linear identity** `agg0 + agg1 + B^4·(#2 carry=0) + B^3·(#57 mask=s)` | 3,322 |
| 9 | p | low | order | linear inequality; Farkas λ=(1,1) on A#54 / A#55 → `0 > 0` | 40 |
| 10 | p | high | order | same pair A#54/A#55 | 40 |
| 11 | −(p+38) | low | order | linear inequality; Farkas λ=(1,1) on A#53 / A#55 → `p+38 < 0` | 40 |
| 12 | −(p+38) | high | order | same pair A#53/A#55 | 40 |
| 13 | C | low | slim | linear arithmetic | 101,532 |
| 14 | C | high | slim | linear arithmetic | 102,555 |
| 15 | p | low | slim | linear arithmetic | 100,884 |
| 16 | p | high | slim | linear arithmetic | 101,907 |
| 17 | 0 | low | full | linear arithmetic (slim fails, full close) | 175,772 |
| 18 | 0 | high | full | linear arithmetic (slim fails, full close) | 178,866 |
| 19 | B^4 | low | order | linear inequality; Farkas λ=(1,1) on the two **negated** conditions A#52 `~(n≤m)` / A#53 `~(m<n)` → `0 > 0` | 84 |
| 20 | B^4 | high | order | same negated pair A#52/A#53 | 84 |
| 21 | 0 | low | cert | **linear identity** `agg0 + agg1 + B^4·(#3 carry=0) + B^3·(#57 mask=s)` | 2,177 |
| 22 | 0 | high | slim | linear arithmetic | 72,644 |
| 23 | B^4 | low | order | linear inequality; Farkas λ=(1,1) on the negated pair A#53 / A#54 → `0 > 0` | 84 |
| 24 | B^4 | high | order | same negated pair A#53/A#54 | 84 |

Totals: **12 order-refutations, 2 linear-identity certificates, 7 slim, 3 full.** No `sub`
branch is propositionally inconsistent (literal/AC both miss on all 24) and none is a
single-atom range contradiction; the vacuity is order-theoretic or linear-arithmetic.

**The 12 order branches** are all a two-premise Farkas refutation with multipliers `(1,1)`:
the normalised rows are exact negatives and the constants cancel (or leave `−(p+38)`), with at
least one strict inequality, so the pair sums to `0 > 0` or `p+38 < 0`. Eight arrive as
ordinary `≤`/`<` conditions; four (19/20, 23/24) arrive **negated** and are converted by
`INT_NOT_LE`/`INT_NOT_LT` first.

**The 2 certificate branches** (8, 21) are exact integer combinations of the four equation
antecedents.  Branch 8 closes with
`#0·1 + #1·1 + #2·B^4 + #57·B^3`
(`#0`,`#1` the two aggregates, `#2` the `carry_s14 = 0` fact, `#57` the mask equation);
branch 21 is the same shape with the carry fact at `#3`.  The weights are the digit positions
`B^3 = 2^192`, `B^4 = 2^256`.  Branch 2 admits the same identity but is closed by the cheaper
order refutation first.

**The 10 arithmetic branches** (5–7, 13–18, 22) have no pure equation identity and no
two-term antisymmetry.  Their `slim` set is the 2 aggregates + carry fact + mask equation +
five relational facts (9–10 premises); 7 close on that slim set and 3 (7, 17, 18) need the
full 58-antecedent set because the slim attempt fails.  The kernel proves the exact
`bridge_mk_imps slim w'` obligation with `INT_ARITH` (independently replayed:
`STANDALONE-PROVED`).  This is a genuine multi-premise linear-arithmetic refutation; I did not
hand-build a Farkas certificate for it (see §6).

## 4. `add` — 12 branches

`add` has no mask split, so no `mask` column. It is **unchanged** by this experiment.

| # | goal RHS | route | classification | cost |
|---:|---|---|---|---:|
| 1 | p | full | linear arithmetic (full close) | 126,614 |
| 2 | −(p+38) | slim | linear arithmetic | 56,296 |
| 3 | C | slim | linear arithmetic | 98,156 |
| 4 | p | slim | linear arithmetic | 107,002 |
| 5 | 0 | cert | **linear identity** `#1·(−1) + #2·B^4` | 1,842 |
| 6 | B^4 | slim | linear arithmetic | 60,810 |
| 7 | p | cert | **linear identity** `#0·(−1) + #1·(−1) + #2·B^4` | 2,446 |
| 8 | −(p+38) | slim | linear arithmetic | 68,077 |
| 9 | C | slim | linear arithmetic | 109,733 |
| 10 | p | cert | **linear identity** `#0·(−1) + #1·(−1) + #2·B^4` | 2,631 |
| 11 | 0 | slim | linear arithmetic | 80,728 |
| 12 | B^4 | slim | linear arithmetic | 72,596 |

Totals: **3 linear identities, 8 slim, 1 full**, exactly the IX.95 route set. No `add` branch
has an order-antisymmetry pair, so the new path does not fire; the mask-substitution search is
not reached because the existing one/two-premise search already finds all three certificates.

## 5. Measurement (`plain == phase`, `diff 0`)

| task | baseline | now | Δ | close (enter→exit) | ratio to human |
|---|---:|---:|---:|---:|---:|
| add | 4,926,695 | **4,926,695** | 0 | 786,931 | 2.67× |
| sub | 7,985,276 | **6,646,550** | **−1,338,726 (−16.77 %)** | 2,554,010 → **1,217,732** | 5.18× |
| neg | 4,121,086 | **4,121,086** | 0 | 2,057 | 5.71× |

Human: 1,848,494 / 1,283,174 / 721,473.

Per task, from `parse_gen.awk` (`before`, `after`, `inf`, `computed`, `diff`):

```
sub   before 67670291  after 74316841  inf 6646550  computed 6646550  diff 0
add   before 67646907  after 72573602  inf 4926695  computed 4926695  diff 0
neg   before 67649313  after 71770399  inf 4121086  computed 4121086  diff 0
```

`plain` and `phase` `STAMP_INFERENCES` are identical: `6646550 / 4926695 / 4121086`.
`sub` routes: **order 12, linear-certificate 2, slim 7, full 3**; per-branch close deltas
`[40,40,40,40,103949,104972,168496,3322,40,40,40,40,101532,102555,100884,101907,175772,178866,84,84,2177,72644,84,84]`.
`add` routes unchanged (linear 3, slim 8, full 1); `neg` routes unchanged (prop 3, literal 3,
linear 1, AC 1, range 2).

## 6. What was added to `tools/bridge.ml` (each with fall-through)

1. **Order-antisymmetry refutation** (`order_closed`).  Every antecedent is normalised to a
   positive `a ≤ b` or `a < b`; a negated comparison is read as its complement
   (`~(a ≤ b) ≡ b < a` by `INT_NOT_LE`, `~(a < b) ≡ b ≤ a` by `INT_NOT_LT`) **lazily**, so a
   no-match spends no inference.  A `a ≤ b` / `b < a` pair is refuted by
   `INT_LET_ANTISYM`; an `a < b` / `b < a` pair by `INT_LT_ANTISYM`; the proof is
   `CONJ` → `NOT_ELIM` → `MP` → `CONTR w'` → `EQ_MP (SYM w_eq)`.  This is the two-premise
   Farkas combination with multipliers `(1,1)`.  It is tried **after** the literal and AC
   complement matchers (so `neg`'s 15-inference literal branches keep their route) and
   **before** the certificate/search/slim closes.  Cost: **40** inferences per ordinary pair,
   **84** per negated pair; 12 `sub` branches use it (4 not previously reachable by the
   direct matcher because one condition is negated).
2. **Mask-substitution identity search** (`search`, inside `linear_certificate`).  The
   existing greedy only continues while the residual's support *shrinks*; a mask equation
   `mask + k = val` moves support from `mask` to `val` without shrinking it, so
   `agg0 + agg1 + B^4·fact + B^3·mask` is invisible.  The new bounded depth-first search
   accepts any step that cancels the residual's leading atom, keeps each premise at most once,
   tries every unused premise (it backtracks, unlike the greedy), and is entered only after
   `first_one`/`first_two`/`greedy` all miss.  Every candidate is still verified by the
   existing `INT_POLY_CONV` identity before a proof is built, so nothing unsound can be
   produced.  Cost: **3,322** (branch 8) and **2,177** (branch 21) inferences, which include
   the two `INT_POLY_CONV` normalisations of the large aggregate terms.

The certificate indices and weights are printed by the existing phase line and reproduced in
§3; weights are the digit positions `B^3 = 2^192`, `B^4 = 2^256`.

`add` and `neg` do not regress: the new paths either do not fire (`add`'s three certificates
are found earlier; there are no order pairs) or are placed after the cheaper existing paths
(`neg`'s literal complements).

## 7. Honest notes

- **No new `prove`.**  Both additions are derived-rule only; the library `prove` count stays 92,
  the instrumented phrase count stays 249, the inlined-phrase count stays 293, and the
  `curve_check` report is `statements=92 hits=0` (line numbers moved).
- **Ten `sub` branches remain on `INT_ARITH`.**  They are real linear-arithmetic refutations:
  the aggregate equations plus the one-sided `n<p`, `m<p`, the `m−n` comparison, the `m<n`
  condition and the mask equation, with no pure identity and no two-term antisymmetry.  A
  general multi-premise Farkas fast path (nonnegative multipliers over the limb upper bounds
  and the aggregate) would close them but requires hand-built order-rule bound chains; per
  the brief it was not built, and is reported rather than guessed.  Their kernel obligation
  was replayed and confirmed valid (`STANDALONE-PROVED`).
- **Offline reconstruction caveat.**  Re-parsing `string_of_term` output into a separate HOL
  term is unreliable for atom identity because the pretty-printer varies whitespace; the
  implementation therefore matches with `aconv` on the actual terms, and the classification's
  route/cost columns come from the kernel run, not from the parser.
- **Not established:** `equals_thm`, `axioms()`, matched-session accounting, timing medians.
  The measurement is scoped to this bridge revision, these three tasks and the IX.89 harness.

## 8. Final record

- `git diff --stat`: 30 files — `tools/bridge.ml` (+120), 27 re-recorded
  `tests/fixtures/answers/**.answer.txt`, `tests/fixtures/curve_check_bridge.txt`, and
  `tests/tools_receipt.rs` (digest, instrument sha, four inline digests + boundaries).
- `BRIDGE_DIGEST = cae44c88…`; instrument sha256(source) `cae44c88…`, output
  `12f972bd…`; inline digests `95e9f9a3…` (add-ref), `e71fb192…` (sub), `5bdd2350…` (neg),
  `7b2b3b0b…` (add-p384).
- Verification: `cargo test --offline --no-fail-fast` **rc 0** (45 suites, 0 failures);
  `cargo clippy --offline --all-targets -- -D warnings` **rc 0**; six `run_gen.sh` runs
  **rc 0**; three `parse_gen.awk` **diff 0**.
