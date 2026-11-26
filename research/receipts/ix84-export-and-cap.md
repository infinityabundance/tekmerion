# IX.84 — one exported derivation, end to end, and the authorised x86 cap decision

**Status: PROVISIONAL.** This is the kernel track. The object is *well-formed* and replays,
but the checker is **not proved sound**. The soundness obligations remain open (`XVII.B5`,
`G1–G3`). It becomes non-provisional only when the assurance track witnesses it at
identity-bound grade (`XVIII.D`).

- **Date:** 2026-11-26
- **Branch:** `staging` (base `c276f62`; `main` and tag `freeze-rust-closure-v6` at `cb054bd`
  untouched)
- **Code:** `src/tools/proof_object.rs`, `src/bin/proof_object.rs`, `src/tools/mod.rs`,
  `tools/hol-light/proof-object.diff`
- **Data:** `research/receipts/ix84-h1.json`, `research/receipts/ix84-export.json`

Everything below is measured on this host at this commit. No number is reported from a run
that did not complete; the failures are reported as failures.

---

## 1. Environment and resources

```
$ free -g | head -2
               total        used        free      shared  buff/cache   available
Mem:             125          15          48           1          61         110
$ df -h /mnt/1tb_kingston | tail -1
/dev/nvme1n1p1  1.3T  902G  360G  72% /mnt/1tb_kingston
```

The H1 aggregate run had 110 GB available against a 24 GB cap (well over the 3× rule). Every
proof and cargo run went through `tools/cap.sh`; `-j1`, one heavy process at a time.

---

## 2. Part H1 — the authorised 24 GB decision

### 2.1 What was authorised, and what was actually run

`XIX.H1` authorises **one** re-run of `x86/add_p25519` under `tools/cap.sh 24G`. Two 24 GB
invocations exist and both are recorded; the distinction matters.

1. **The first invocation did not test H1's mechanism.** `x86.bignum_add_p25519` is
   deliberately *not* in `ALGEBRAIC_CLOSE_TASKS` (`src/tools/emit.rs`), so
   `TEKMERION_ALG_CLOSE` was ignored and the emitted answer selected the default SAT entry. It
   closed at **120,066,521** inferences with a peak of **1,947,228 KB** and no `ORACLE`
   output. It never approached the cap. It is disclosed here for completeness and is **not**
   counted as the authorised test.
2. **The authorised test** fed the same aggregate answer the 12 GB attempt used
   (`.bench/ix83/xadd1/x86_add_p25519.answer`, which selects `BRIDGE_ADD_REF_ALG_TAC`), so the
   mechanism under test — not the route selection — is the one that fired at 12 GB.

### 2.2 The 24 GB aggregate run

```
$ TEKMERION_ALG_CLOSE=x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT \
    tools/cap.sh --timeout 3600 24G -- \
    bash .bench/ix84/run_task_24g.sh x86 curve25519 add p25519 \
         .bench/ix84/h1agg .bench/ix83/xadd1/x86_add_p25519.answer
RUN x86_add_p25519 rc=2 phase_marks=16
Fatal error: allocation failure during minor GC
make: *** [Makefile:501: curve25519/bignum_add_p25519.correct] Error 134
cap.sh: group=393038 rc=2 sampled_group_peak_rss_kb=24900604 max_process_vmhwm_kb=24887916
```

- **Cap:** 24 G = 25,165,824 KB `RLIMIT_AS`.
- **Peak:** 24,900,604 KB sampled group RSS / 24,887,916 KB `VmHWM` = **23.75 GiB**, 98.95% of
  the cap.
- **Outcome: capped.** The theorem did **not** close. `rc=134` from make, `rc=2` from the
  wrapper.
- **One branch closed** (`ORACLE close ok` = 1), no branch reported `ORACLE close failed`.
- **Aggregate found:** `ORACLE aggregate-carriers=2 [&(bitval carry_s16),&(bitval carry_s8)]`.

### 2.3 Phase marks (16 reached, in order)

```
00 rewrite 83158978            07 finalisation 84505745
01 rewrite 83158978            08 rewrite 84738831
02 rewrite 83159110            09 rewrite 84739025
03 finalisation 83159111       10 finalisation 84739431
init finalisation 83159273     11 bridge 84739487
04 rewrite 83159959            bridge:algebra 84790929
05 rewrite 83160320            bridge:algebra-arith 84919759
06 play 83160785               bridge:algebra-arith 85135368
```

The last marker before the allocation failure is
`ORACLE aggregate-carriers=2 [&(bitval carry_s16),&(bitval carry_s8)]`, immediately after the
second `bridge:algebra-arith`. **Memory grows in the second (correction-plus-mask) branch's
`INT_ARITH` call**, not in the aggregate construction.

### 2.4 Diagnosis and decision

> The x86 add's correction is a full-width `word_sub (word_add M N) p` guarded by `bit 63`;
> the second (correction-plus-mask) branch's `INT_ARITH` builds a linear system whose internal
> representation grows past any practical address-space cap, so the mechanism does not scale
> on x86 as written.

Doubling the cap from 12 G to 24 G roughly doubled the peak (12.31 GB → 24.89 GB) and still
did not close. **Do not raise it further. Do not weaken the close.** `x86.bignum_add_p25519`
remains outside `ALGEBRAIC_CLOSE_TASKS`. The published ARM totals (5,233,638 / 9,029,510 /
5,683,733) are unaffected.

---

## 3. Part E1 — one derivation, exported and replayed

### 3.1 The theorem choice, stated before running

The smallest of our 27 is `x86/bignum_neg_sm2` at ≈18M primitive inferences. Finalisation now
streams (G1.6), so the *export* is bounded by the frontier. The *replay*, however, still keeps
one `Thm` per reachable node and is capped by `--budget` (default 2,000,000). An 18M-node
replay would be refused as `BudgetExceeded`, not replayed. Shipping an object we cannot replay
inside the bound would violate the honesty rule.

**Therefore the first export is a genuinely small library theorem: `ADD_SYM`**
(`!m n. m + n = n + m`), re-derived under the recording kernel with the library's own proof
(`INDUCT_TAC THEN ASM_REWRITE_TAC[ADD_CLAUSES]`). This is the brief's explicit fallback. The
bignum export is deferred to the frontier-bounded replay, which is the next E1 step.

### 3.2 Recording

```
$ TEKMERION_PROOF_RAW=.bench/ix84/addsym.raw \
    .bench/dropin/x86/sm2/po_add_sym.native
proof_nodes=314
exported ADD_SYM
```

Raw stream: **314 nodes, 78,521 bytes**. `STAMP_INF_BEGIN 14,976,842`,
`STAMP_INF_END 14,977,147` → **305 primitive inferences**. 314 recorded nodes include 13 IMPORT
leaves; the difference between 314 and 305 is emitted structural nodes vs `bump_inferences`
calls.

### 3.3 Export and replay (published numbers)

| quantity | value |
|---|---:|
| theorem | `ADD_SYM` |
| nodes attempted (raw) | **314** |
| nodes reachable (replayed) | **227** |
| primitive derivation nodes (reachable − imports) | 214 |
| imported library theorems | 13 |
| object bytes | **7,039** |
| peak payload buffer during finalisation | 2,013 bytes |
| finalise wall time | 24 ms |
| finalise peak RSS | 3,716 KB |
| replay wall time | 24 ms |
| replay peak RSS | 3,804 KB |
| cap | 20 G, did not fire |

```
$ proof_object finalize --raw .bench/ix84/addsym.raw \
    --object .bench/ix84/e1/addsym.hlpo --store .bench/ix84/e1/addsym.hlst \
    --expected .bench/ix84/e1/addsym.expected --json .bench/ix84/e1/addsym.final.json
FINALIZED {"theorem":"ADD_SYM","statement_digest":"00879bba…","node_count":227,
  "raw_nodes":314,"bytes":7039,"root_id":226,"imports":13,
  "inference_total":14977147,"inference_begin":14976842,"inference_delta":305,
  "peak_buffered_payload_bytes":2013,"peak_rss_kb":3716,
  "axioms":["8b87f0cf","397b0b82","3af16d5f"]}

$ proof_object validate --object .bench/ix84/e1/addsym.hlpo \
    --theory .bench/ix84/e1/addsym.theory --store .bench/ix84/e1/addsym.hlst \
    --expected .bench/ix84/e1/addsym.expected
VALIDATED {"theorem":"ADD_SYM","statement_id":"362ae773…","node_count":227,
  "reachable":227,"imports":13,"peak_rss_kb":3804, ...}
```

The replayed conclusion is alpha-equal to the recorded statement, and the replayed
`statement_id` equals the expected one. Digests:

```
statement_digest  00879bbaf18ffe72623ab7cbac68e0b428454a510384255c9f397da267934341
statement_id      362ae773944141211ad4c03617ac00f82f93c3c176d0b287219de8124a0c015e
theory_digest     09e046f90bf202c06dc156b50eb3d5e1e1ae69aee6d358a79d3da6fab82dde73
store_digest      e3d3ef06517131546634707268667be5e10cb1df393c4c819777f4c0b8fd1de8
derivation_digest e509ccece229e4242b5dc1f04e042a9f6e6c03f08209a9022308a6f7f121ec0c
```

### 3.4 Foundational dependencies — claimed vs permitted

| set | certificate claims | validated theory permits | difference |
|---|---|---:|---|
| Axiom nodes | 0 | 0 | none |
| Definition nodes | 0 | 0 | none |
| Type-definition nodes | 0 | 0 | none |
| Imports (sequents) | 13 | 13 (the pinned store) | none |

The footer also records the three ambient axioms of the recording session
(`8b87f0cf`, `397b0b82`, `3af16d5f`). These are **provenance** for the library the proof ran
in; they are not re-assumed as certificate `Axiom` nodes. The refusal-on-difference mechanism
is exercised by `g1_2_axiom_requires_theory_authorisation` and
`control_unauthorised_axiom_is_refused`.

**Trust boundary, stated plainly.** The store is written by the exporter from the kernel's
`IMPORT` leaves and pinned by `store_digest` inside the theory digest. The checker never
trusts a digest supplied by the object: an import must be *present in the store*, and the
stored sequent (not the payload) becomes the theorem. Sourcing the store from an independently
validated library export is future work.

---

## 4. The six defects, prevented with a test each

| defect | prevention | test |
|---|---|---|
| **G1.1** import rebuilt from the object | `Rule::Import` resolves the sequent through a caller-supplied `CheckedTheoremStore`; payload must match; the stored theorem is used | `g1_1_import_requires_external_store` |
| **G1.2** axioms checked against the object's own set | `Axiom` requires the statement digest to be in `ValidatedTheory::axioms`; the declared set has no authority | `g1_2_axiom_requires_theory_authorisation` |
| **G1.3** definitions can rebind | a definition is refused if the constant exists in the theory or an earlier accepted definition | `g1_3_definition_rebinding_is_refused` |
| **G1.4** type definitions unregistered | `(type, abs, rep)` must be in the theory's extension registry, names must be fresh, both halves must be present | `g1_4_type_definition_requires_registry` |
| **G1.5** statement identity binds only the term | `statement_id = H(theory_digest ‖ hyps ‖ concl)`, and the theory digest binds the store digest | `g1_5_statement_identity_binds_the_theory` |
| **G1.6** finalisation accumulates nodes and output | payloads are read one record at a time; the node table is written through a digesting writer; nothing is accumulated | `g1_6_finalize_buffers_one_payload_not_the_object` |

A seventh, material defect was found by the real replay: the recovered scaffold's `rev_assocd`
matched the **first** component of each pair, while HOL Light matches the **second** and
returns the first. Type and term instantiation directions were therefore reversed, and the
ADD_SYM replay failed at node 45 (`EQ_MP`). Fixed; guarded by
`substitution_pair_order_matches_hol_light`; the ADD_SYM replay now reaches the root.

---

## 5. The four refusals, verbatim

From `cargo test --offline --lib 'proof_object::tests::control_' -- --nocapture`:

```
TRUNCATED REFUSED: object: bad magic
SPLICED REFUSED: refused: node 2 (TRANS) does not follow from its parents: middle terms differ
WRONG CONCLUSION REFUSED: refused: replayed statement identity 01efd6d0f08065a453e715d86da87fbf8d480a928e01184eb69ea163d1f8855e differs from expected 104a8e411f02c70462efb852951bf4bc45db0823f5df3a4ba5e129ead304f569
UNJUSTIFIED IMPORT REFUSED: refused: IMPORT sequent 20d053e45f1574d4e616031532dedb190cba4e45445cad68ba46096ed8429029 is not justified by the checked-theorem store
```

Plus the G1.2 end-to-end control:

```
UNAUTHORISED AXIOM REFUSED: refused: axiom a6f5e0281b602f265f700a371ffc3c703b692d1d4b1a6313b8c89ab31c5dbd55 is not authorised by the validated theory
```

Tests: `control_truncated_object_is_refused`, `control_spliced_object_is_refused`,
`control_wrong_conclusion_is_refused`, `control_unjustified_import_is_refused`,
`control_unauthorised_axiom_is_refused`. Two more controls:
`control_valid_object_replays`, `control_truncated_raw_stream_is_refused`.

The CLI classes these as `validate: REFUSED (truncated|spliced|conclusion|import|environment)`.

---

## 6. What remains unproved

- **The checker is not proved sound.** The G1 fixes make it *well-formed*; `XVII.B5` and
  `G1–G3` are open.
- **Replay is not frontier-bounded.** Finalisation is; replay keeps one theorem per reachable
  node and refuses above `--budget`. That is why the first export is ADD_SYM and not the
  18M-node bignum proof.
- **The store's provenance is the exporter**, pinned by digest; an independently validated
  library export is future work.
- **One theorem, one theory manifest, one host.** No number is claimed for the other 53.
- **The recording patch is the recovered WIP diff.** The export switch
  (`.bench/hol-light-objects`) was built from it; the patch was not re-applied to a fresh tree
  in this run.

---

## 7. Commands (verbatim)

```
$ free -g | head -2
$ df -h /mnt/1tb_kingston | tail -1
$ TEKMERION_ALG_CLOSE=x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT \
    tools/cap.sh --timeout 3600 24G -- bash .bench/ix84/run_task_24g.sh \
    x86 curve25519 add p25519 .bench/ix84/h1agg \
    .bench/ix83/xadd1/x86_add_p25519.answer
$ .bench/ix84/export_addsym.sh                # finalize -> theory -> validate
$ tools/cap.sh 20G -- cargo test --offline
$ cargo clippy --offline --all-targets -- -D warnings
$ cargo fmt --check
$ git ls-files '*.py'                         # empty
```
