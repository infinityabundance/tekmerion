# IX.81 — the interval engine, the tool selector, and a real elaboration gate

Date: 2026-11-25
Branch: `staging` (one commit; `main`, `freeze-rust-closure-v6` untouched)
Scope: `XIX.A1`–`A4`, `XIX.B1`–`B4`

This receipt records four things that share a property: each was a place where a
green test suite or a plausible-looking number was not evidence.

1. The discovery engine's interval arithmetic **added sums with a max/min join**
   and **substituted a missing endpoint instead of propagating unknown**.
2. The disassembler selector **only tried the first `PATH` match per name**, and
   its process-spawning tests were timing-sensitive.
3. No test ran the **real front end** over an emitted answer.
4. `curve_check`'s statement count fell 76 → 33 because its comment stripper
   read the multiplication quotation `` `(*):int->int->int` `` as a comment, then
   deleted the rest of the file. The control was measuring its scanner.

No theorem is affected by item 1: the proof step refused the false proposal
(IX.78 control). Item 4 changes a *control*, not a proof.

---

## 0. Environment

```
$ free -g | head -2
               total        used        free      shared  buff/cache   available
Mem:             125          33          87           0           3          91

$ df -h /mnt/1tb_kingston | tail -1
/dev/nvme1n1p1  1.3T  902G  360G  72% /mnt/1tb_kingston
```

Every heavy command below ran through `tools/cap.sh`; one at a time.

---

## 1. A — sums add bounds; unknown propagates

### 1.1 The defect

`bridge_form_bounds` accumulated the bounds of a linear form with
`bridge_bound_join_lo` / `bridge_bound_join_hi`, i.e. **max lower / min upper**,
starting from the constant. That is the right operation for two constraints on
the *same* quantity, and the wrong operation for a *sum*. With `0 ≤ x ≤ 3` and
`0 ≤ y ≤ 4` it produced an upper bound of 0, not 7. It also treated `None` (“no
bound for this atom”) as the identity of the join while summing, silently
pretending an unbounded summand was zero.

### 1.2 The fix

`tools/bridge.ml`, block `BRIDGE_INTERVAL_STANDALONE`:

* `bridge_bound_add_lo` / `bridge_bound_add_hi` add the endpoints and propagate
  strictness (`x > a`, `y ≥ b` ⇒ `x+y > a+b`). **A `None` endpoint on either side
  makes that endpoint `None`**: unknown propagates.
* `bridge_bound_contrib k bound` scales one atom: increasing for `k > 0`,
  endpoints swapped for `k < 0`, and exactly `0` for `k = 0` whatever the atom.
* `bridge_bounds_of_terms lookup c terms` folds contributions by **addition**,
  starting from the exact constant `c`.
* `bridge_form_bounds` is now a one-line wrapper around it.

The block is standalone (only HOL's `Num` interface is used), so it is extracted
verbatim by the exhaustive test.

### 1.3 Audit: add or join, and why

Every use of the two primitives in `tools/bridge.ml`:

| site | operation | why |
|---|---|---|
| `bridge_bounds_of_terms` (used by `bridge_form_bounds`) | **add** | a linear form is a sum of scaled atoms; bounds of a sum add |
| `bridge_direct_bounds` | **join** (max lower, min upper) | two hypotheses about the *same* atom are two constraints on one quantity; `None` means “this hypothesis gives no bound”, i.e. the ±∞ identity of max/min |
| `bridge_hi` | **join** (min upper) | two independent derivations of an upper bound for the *same* form; the tighter one wins; `None` is +∞ |
| `bridge_rel_hi` | no primitive of its own | uses `bridge_form_bounds` on the residual, so it inherits the add rule; a `None` upper bound yields `None` (no relational bound) |
| `bridge_lo_of_neg` | no primitive of its own | negates the upper bound of the negated form |

`bridge_lookup_bound` is unchanged (`List.assoc`, i.e. structural equality on
the HOL term). `bridge_direct_bounds` still starts from `(None,None)` per atom;
that is a join over constraints, not a sum.

### 1.4 Exhaustive small-domain tests

`tests/interval_engine_selftest.sh` extracts the block and compiles it with
`tests/interval_engine_shim.ml` (exact rationals, not `int`: strict endpoints
keep their meaning) and `tests/interval_engine_selftest_driver.ml`.

Coverage:

* endpoint family: `None` plus `{-2,-1,0,1,2} × {non-strict, strict}` for each
  of the two endpoints; non-empty intervals only (4,356 interval pairs);
* sample grid: multiples of `1/4` on `[-3,3]`, exact rational arithmetic,
  360,000 concrete `x+y` values;
* for each pair, every sampled sum must lie in the computed interval
  (**soundness**), and the computed endpoint must be within two grid steps of
  the sampled extreme (**no gross looseness**; two steps because a strict
  endpoint removes the boundary point on each side of the grid);
* coefficients `{-2,-1,0,1,2}` × all intervals, every sampled `k*x` in the
  computed scaled interval (330 cases);
* the join rule is checked directly against max/min, including `None` as the
  identity (121 bound pairs each way);
* the **negative control** runs the retired join-based accumulation through the
  same containment search and requires it to fail.

Verbatim (`tools/cap.sh 6G -- bash tests/interval_engine_selftest.sh`):

```
case: 0 <= x <= 3, 0 <= y <= 4
  shipped: lo=0 hi=7
OK sum lower bound is 0
OK sum upper bound is 7 (not the old join's 0)
OK upper bound <= 7
OK a missing lower endpoint makes the sum's lower endpoint unknown
OK one unbounded summand is enough for the lower endpoint
OK a missing upper endpoint makes the sum's upper endpoint unknown
OK the known upper endpoints of both summands are still added
exhaustive add: bound_pairs=4356 points=360000 failures=0
  counterexamples: none
OK every sampled sum lies in the computed interval
OK computed endpoints are tight to within two grid steps
exhaustive scale: cases=330 failures=0
  counterexamples: none
OK every sampled scaled value lies in the computed interval
join rule: lower_failures=0 upper_failures=0
OK join of lower bounds is max
OK join of upper bounds is min
negative control (legacy join rule): failures=185026
  first counterexample: legacy-join: -3 + -3 = -6 not in [0, 0]
OK the exhaustive test rejects the retired join-based rule

OK interval_engine_selftest
OK: interval_engine_selftest
cap.sh: group=3075623 rc=0 sampled_group_peak_rss_kb=16228 max_process_vmhwm_kb=12748
```

The negative control's first witness involves unbounded intervals because the
old rule also treated `None` as zero; the unit section shows the reported
`0 ≤ x ≤ 3`, `0 ≤ y ≤ 4` case giving `hi=7` and the retired rule rejected
somewhere in the same 4,356-pair family. In other words, a wrong rule surfaces
as a counterexample, not as a failed proof.

### 1.5 What is **not** proved

This is **tested interval arithmetic, not verified interval arithmetic**. The
exhaustive test is a finite counterexample search against an exact-rational
model, not a theorem, and it does not cover the reals or the full HOL `Num`
domain. A soundness theorem (monotonicity of the rational operations and the
add/scale rules) belongs with the native `LIA` module (`XVII.B3`). Until then,
the oracle *proposes* and the proof step *disposes*; that separation is what
kept the IX.78 false carry from becoming a theorem.

No proof run was performed in this task, so no proof-cost or admission number is
reported. `tools/bridge.ml` changed, so the answer fixtures and goldens were
re-recorded deliberately (§5).

---

## 2. B1/B2 — the selector is pure policy plus process-spawning integration

### 2.1 What changed

* `src/tools/engine.rs` (new): `select(&[CandidateName], probe)` is a pure
  function over a supplied candidate list. `CandidateName` carries one tool name
  and **every** executable path found for it; an empty list means “not found”.
  The policy probes every path of every name in order, records each rejection
  verbatim, and returns the first acceptance. It does no I/O, spawns nothing and
  reads no clock. Unit tests cover: second path of a name is tried; later names
  are tried; no candidate; missing names; note order; and a policy test with
  paths that do not exist (purity).
* `src/tools/disasm.rs`: `which_in` (first match only) became `which_all_in`
  (every match, in `PATH` order, deduplicated). `select_disassembler` builds the
  candidate list and calls `engine::select` with the real probe
  (`probe_candidate`). The failure text and install hint are unchanged. The
  module is declared with `#[path = "engine.rs"] pub mod engine;` so the policy
  file is owned by `disasm` without a second `mod.rs` entry.
* `tests/tools_disasm.rs`: the process tests retry **once** on `Text file busy`
  and print that they did; a new integration test places an unusable shim for a
  name in the first directory and a usable shim of the same name in the second,
  and requires the second to be selected.

### 2.2 Verbatim

```
$ tools/cap.sh 20G -- cargo test --offline --test tools_disasm an_unusable
     Running tests/tools_disasm.rs (target/debug/deps/tools_disasm-021e1b4d39c2480c)

running 2 tests
test an_unusable_explicit_tool_is_refused ... ok
test an_unusable_shim_earlier_on_path_is_skipped ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.03s

$ tools/cap.sh 20G -- cargo test --offline --test tools_disasm every_matching
     Running tests/tools_disasm.rs (target/debug/deps/tools_disasm-021e1b4d39c2480c)

running 1 test
test every_matching_path_for_a_name_is_probed ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.03s
```

The policy tests are `src/tools/engine.rs::tests` (6 tests, run by
`cargo test --offline`). They cannot race a freshly written executable because
they never create one.

---

## 3. B3 — the elaboration gate

### 3.1 The command and its status

`tests/answer_elaboration.rs` runs, on
`tests/fixtures/answers/arm/bignum_add_p25519.answer.txt`:

```
$ tools/cap.sh 20G -- cargo test --offline --test answer_elaboration -- --nocapture
running 3 tests
ocamlc: ocamlc (version 5.5.0)
answer: /mnt/1tb_kingston/Tekmerion/tests/fixtures/answers/arm/bignum_add_p25519.answer.txt (3761 lines, 319 quotations, 2067 upper-case identifiers lowered)
ocamlc -stop-after parsing -i: exit exit status: 0
full elaboration (bench template + answer): exit exit status: 0
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.79s
```

The gate prints the first five lines of the front end's output if it produces
any; for a parse-only run the output is empty, which is why the exit status is
the evidence. A missing `ocamlc` **panics with an instruction to install
`ocaml-nox`**; there is no skip path. `--nocapture` is used in CI so the version,
the quotation count and the exit status are in the log.

### 3.2 What it actually checks, and why the answer is normalised

HOL Light source is not stock OCaml:

* it uses backtick-delimited quotations, which are opaque to OCaml (HOL's own
  preprocessor `pa_j.cmo` parses their contents);
* it writes upper-case value names (`let rec GCONJUNCTS th acc = ...`), which
  camlp5r accepts and the OCaml grammar does not.

The gate therefore applies exactly two **lexical** normalisations before calling
`ocamlc`: quotations become string literals, and upper-case identifiers are
lower-cased except constructors (including `Tyapp`/`Tyvar`), dotted module paths
(`List.map`, `Sys.Break`) and names whose lower-case form is a keyword (`THEN`
stays upper case; `then` is syntax). It does not repair structure. A malformed
phrase still fails, and an odd backtick — the XVI.D polymorphic-variant
regression, which broke every emitted answer — is refused as an unterminated
quotation (unit test `a_stray_backtick_is_refused`).

It was checked in both directions manually: deleting the closing `)` of the
answer makes the same command exit 2 with `Syntax error: ) expected`; the
untouched answer exits 0.

**What it cannot check offline** is the *content* of a well-formed quotation.
That needs `pa_j.cmo`, hence HOL Light, hence `.bench`. The committed answer
alone also cannot be fully `-i`-typechecked, because it names helpers defined by
the bench template prefix; verbatim, from a deliberate attempt with the bench's
HOL switch:

```
File "/tmp/ans_hol2.ml", line 781, characters 15-28:
781 | let CSAT_TAC = BITBLAST_THEN (CONV_TAC o K CSAT_PROVE)
                     ^^^^^^^^^^^^^
Error: Unbound value BITBLAST_THEN
```

For that reason the test also runs the bench's own full check
(`ocamlc -pp "$(hol.sh -pp)" -I .bench/hol-light -i`) on the newest generated
combined task under `.bench/eval-*`, when this host has the bench. That arm
elaborates a real combined template plus answer and exited 0 above. It is not
the CI gate; the CI gate is the always-available parse arm. This is stated as a
limit rather than hidden: the class it catches offline is *syntax outside
quotations* (malformed phrases, stray delimiters, a stray backtick) and it would
have caught XVI.D's stray backtick, but not a semantically invalid well-formed
HOL term inside a quotation.

### 3.3 CI wiring

`.github/workflows/ci.yml` gains two explicit steps after `cargo test`:

```yaml
      - name: Answer elaboration gate (real OCaml front end)
        run: cargo test --offline --test answer_elaboration -- --nocapture

      - name: Interval engine exhaustive tests (offline)
        run: bash tests/interval_engine_selftest.sh
```

`ocaml-nox` is already installed by the workflow (it provides `ocamlc` and the
`ocaml` toplevel the interval test needs); there is no new dependency.

---

## 4. B4 — why the statement count fell, and why it is now 81

### 4.1 Diagnosis

`curve_check` strips `(* ... *)` blocks before looking for `prove` followed by a
pair of backticks. IX.78 (`e0dab21`) added this line to `tools/bridge.ml`:

```
and bridge_int_mul_tm = `(*):int->int->int`
```

The stripper did not know about HOL quotations, so the `(*` in the integer
multiplication term opened a comment that never closed: **everything after that
line was deleted from the scan**. The scanner was not “pairing backticks
differently” (the explanation recorded in IX.78); it lost the file.

Evidence, same file, same machine:

```
$ /tmp/oldcc tools/bridge.ml        # scanner from HEAD (git show HEAD:src/tools/curve_check.rs)
statements=33 hits=0

$ target/debug/curve_check tools/bridge.ml   # fixed scanner
statements=81 hits=0
```

The count 81 is the honest one: 76 at IX.77 plus the five code `prove`
occurrences IX.78 added in the oracle helpers
(`bridge_flat_conv`'s `INT_RING` conversion, `bridge_prove_from`,
`bridge_try_value` and two more `INT_ARITH` closes). `grep` counts 33 `prove`
words outside comments before the quotation line; the remaining 48 are in code
after it.

### 4.2 Fix

`strip_comments` now treats, in this order: nested `(* ... *)`, backtick-delimited
quotations, and double-quoted OCaml strings as opaque. A `(*` inside any of the
latter is data, not a comment opener. Regression tests:

* `a_quotation_operator_is_not_a_comment_opener` — the exact
  `` `(*):int->int->int` `` line followed by a `prove` (1 statement);
* `a_comment_opener_inside_a_string_is_not_a_comment`;
* `an_unclosed_quotation_consumes_the_rest_like_the_lexer`.

`tests/fixtures/curve_check_bridge.txt` was regenerated from the fixed binary
(`statements=81 hits=0`), and `tests/tools_receipt.rs` pins 81 with the boundary
history (76 → 33 was the defect, not a measurement).

The control's *hits* count is still 0; that property is unchanged. What changed
is that the denominator now counts what it claims to count.

---

## 5. Fixtures and goldens that moved (deliberately)

`tools/bridge.ml` changed, so every inlined answer changed. Re-recorded with:

```
$ TEKMERION_RECORD_ANSWERS=1 tools/cap.sh 20G -- cargo test --offline --test tools_emit
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.50s
```

| pin | was | now | why |
|---|---|---|---|
| `BRIDGE_DIGEST` (`tests/tools_receipt.rs`) | `c3a1e199…a8385c` | `c4b52cc7b4d6b5fa5901f0a6a00c6590914500bcbc063671f80dea5d74767066` | four sum-bound helpers added |
| `curve_check` statements | 33 (defect) | **81** | scanner fixed; IX.78's five proves visible |
| `curve_check` report fixture | 33 statements | 81 statements, hits 0 | regenerated |
| `instrument` phrases | 217 | 221 | four new top-level `let`s |
| instrumented output sha256 | `a70f5d58…d38fa6` | `61cded3eaacf6523c25671d041abc228f2195c694112a0543527e07006790cbf` | same |
| inline-answer parts | 261 | 265 | same |
| inline `add-ref` | `c26c0080…25f1f7` | `e2ca43d5c1cad5c8e4a977f074cf2b2b130c9ad2ef88bfad8d948a5bb1e29884` | inlined library text |
| inline `sub` | `f600e09a…d01546` | `45daa0a6008d7ea735b0ed8883975d34b80c5c01673fe9c9d30c9adbfcbe3ca4` | same |
| inline `neg` | `821f71d1…add5ac` | `0c9706a22500099f49a6196b9ba53e5c75a67e4b7c9985248b11de809fe7bbad` | same |
| inline `add-p384` | `91083c81…12a3c640` | `b5a742877ce79898fd35a2b46a9d3c19cc41578f88afb7c6649a69d53069aab1` | same |
| 27 files under `tests/fixtures/answers/` | — | +61 / −15 lines each (net +46; 1,647 insertions and 405 deletions over 27 files) | four `let` phrases inline into every answer; the old two-line bound comment is replaced |

The emitted tactic text is unchanged; the added lines are the four bound helpers.
`tests/fixtures/emit/bridge_min.ml` was not touched (its anchors are unaffected).

### 5.1 Freeze manifest: v7, untagged

The v6 seal and the `freeze-rust-closure-v6` tag (`cb054bd`) stand. A manifest
regenerated after a source change is a v7 delta:

```
$ tools/cap.sh 4G -- cargo run --offline --quiet --bin freeze -- manifest \
    --out research/receipts/freeze-closure-v7.json --tag freeze-rust-closure-v7
wrote research/receipts/freeze-closure-v7.json: 73 source files, source_digest 65e5eeead92da59e8623276397bea47f2c9901db930a73c14ea2ae9fdbadf78c, 0 runtime digests, repo_commit 2114402976fa30f9c6b67d21abf98c8a37ce683b

$ tools/cap.sh 4G -- cargo run --offline --quiet --bin freeze -- verify \
    --manifest research/receipts/freeze-closure-v7.json
OK: source closure verified: 73 files, source_digest 65e5eeead92da59e8623276397bea47f2c9901db930a73c14ea2ae9fdbadf78c
OK: effective configuration verified: 6 TEKMERION_* variables
```

73 files = v6's 72 plus `src/tools/engine.rs`. `repo_commit` is the HEAD at
generation time (`2114402`), which is the *parent* of the commit that carries
this manifest; no tag names v7, so the v6 seal is not re-pointed. `git tag
--list` shows only `freeze-rust-closure-v6`, unchanged.

---

## 6. Commands, verbatim

```
$ free -g | head -2
               total        used        free      shared  buff/cache   available
Mem:             125          33          87           0           3          91

$ df -h /mnt/1tb_kingston | tail -1
/dev/nvme1n1p1  1.3T  902G  360G  72% /mnt/1tb_kingston

$ tools/cap.sh 20G -- cargo test --offline
...
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

cap.sh: group=3075957 rc=0 sampled_group_peak_rss_kb=197924 max_process_vmhwm_kb=191408
   (44 suites `ok`, 0 `FAILED`)

$ cargo clippy --offline --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.02s

$ cargo fmt --check
$ git ls-files '*.py'
(no output)
```

The full exhaustive interval output and the two disassembler test runs are in
§1.4 and §2.2; the elaboration gate output is in §3.1.

---

## 7. What remains unproved or unverified

* **The interval arithmetic is tested, not verified.** No soundness theorem; it
  belongs with the native `LIA` module (`XVII.B3`). The exhaustive test is a
  finite rational counterexample search.
* **No proof was re-run.** Changing `bridge.ml` changes the oracle's proposals,
  not the SAT path or the proof step. The theorem status (no theorem admitted by
  the oracle route) is exactly as IX.78 recorded; this task did not re-measure
  proof cost, admission, or the live `arm.bignum_add_p25519` outcome.
* **Quotation contents are not parsed by the CI gate.** Well-formed but invalid
  HOL term syntax inside a quotation still needs HOL Light's `pa_j.cmo`. The
  local full `-i` arm covers it where the bench exists; CI runs only the parse
  arm.
* **The normalisation in the elaboration gate is lexical.** It is tested in both
  directions (a removed token fails; the committed answer passes), but it is not
  a HOL parser and a future answer using a constructor not in `KEEP_UPPER` could
  fail the gate for the wrong reason. The failure would be loud and fixable, not
  silent.
* **CI's OCaml version was not exercised locally.** The local runs used
  `ocamlc` 5.5.0. The workflow's pinned `ubuntu-22.04` runner installs
  `ocaml-nox` (4.13.1), whose parser and `-stop-after parsing -i` are the same
  constructs, but this receipt does not claim a run on that version.
* **`freeze-closure-v7.json` names the pre-commit HEAD** as `repo_commit`, as the
  v6 precedent did for its sealing commit. It is verified against the working
  tree, and no tag points at it.
