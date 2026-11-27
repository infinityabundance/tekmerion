# IX.92 — XXIII.B3: the close without direct bounds (slim close)

**Date:** 2026-11-27
**Branch:** `wip/xxiii-b3-slimclose`, rebased on `staging` at `b41474f`.
**Tree measured:** `tools/bridge.ml` sha256 `a37b4f1c8b65b3486d58d446ca8fbd41b37bbb5e8f6e811ef3a8a46a562ad9ff`.
**Host:** same host as IX.89–IX.91; 86 GB available before the proof sweep; 248 GB disk.
**Counter:** `hol-light-stamp2`. Every proof built with `tools/cap.sh 12G -- make -j1`, one at a
time, no cap raised, none fired (peak per-process `VmHWM` ≤ 1.71 GB).

## 1. What changed

XXVI.C5 captured the negation pre-close obligation and found that **26 of the 31–34 antecedents are
one-sided direct bounds** (`bridge_is_direct_bound`: `&0 <= &(val w)`, `&(val w) < &2^64`,
`&0 <= &(bitval c)`, `&(bitval c) <= &1`). The branch-defining facts are relational and are kept.

`BRIDGE_ORACLE_TAC` now tries the close **first without the direct bounds**, and falls back to the
exact original full-set close on failure. Both paths are kernel proofs; the fallback is the
unmodified original line, so a wrong guess costs one failed attempt and cannot change the result.

## 2. The measurement

Strict self-contained accounting (`XXIII.0` rule 3). `plain == phase` with `diff 0` on every total.

| task | IX.91 | IX.92 | Δ | ratio to human | close region IX.91 → IX.92 |
|---|---:|---:|---:|---:|---:|
| add | 5,268,391 | **5,139,300** | **−129,091 (−2.45 %)** | **2.78×** | 1,154,483 → **1,025,392 (−11.2 %)** |
| sub | 8,217,594 | **7,985,276** | **−232,318 (−2.83 %)** | **6.22×** | 2,811,008 → **2,578,690 (−8.3 %)** |
| neg | 5,683,733 | **5,469,909** | **−213,824 (−3.76 %)** | **7.58×** | 1,578,106 → **1,364,282 (−13.5 %)** |

Human baselines: 1,848,494 / 1,283,174 / 721,473.

**`neg` moves for the first time.** IX.91 left it unchanged because the aggregate fast path is a
no-op with one aggregate; the slim close is orthogonal to aggregates, so it bites on `neg` (10 of 10
branches).

Slim outcomes and set sizes (from the `.correct` files):

| task | slim ok / failed | slim antecedents | full antecedents |
|---|---:|---|---|
| add | 11 / 1 | 7–8 | 53–54 |
| sub | 21 / 3 | 8–9 | 57–59 |
| neg | 10 / 0 | 5–6 | 31–34 |

The `neg` counts reproduce `xxvi-neg-preclose-goal.md` exactly: `slim = full − 26`, i.e. precisely the
26 direct bounds. On `add`/`sub` the drop is larger (54 → 8), so part of the saving is also the
smaller `MP` fold.

**Independent confirmation (re-read from the run files, not the subagent's prose):**

```
add: STAMP_INFERENCES 5139300   slim ok=11 failed=1
sub: STAMP_INFERENCES 7985276   slim ok=21 failed=3
neg: STAMP_INFERENCES 5469909   slim ok=10 failed=0
```

## 3. Instrumentation caveat (must be fixed before the next D comparison)

The phase **D bucket is no longer the close**. `src/tools/emit.rs` anchors `bridge:pre-close` and
`bridge:post-close` to the full-set close line and to `ORACLE close ok`, and both now live inside the
`| None ->` fallback. So D bills only the fallback branches; the slim close's work appears under
`G:INT_RING:flat-end` (phase E). The diff closes to the inference:

| task | ΔD | Δ(G:INT_RING:flat-end) | Δ(bridge:post-close) | ΔE | Δtotal |
|---|---:|---:|---:|---:|---:|
| add | −1,042,923 | +939,698 | −25,866 | +913,832 | −129,091 |
| sub | −2,439,639 | +2,232,001 | −24,680 | +2,207,321 | −232,318 |
| neg | −1,563,214 | +1,364,282 | −14,892 | +1,349,390 | −213,824 |

`Δtotal = Δ(close region)` exactly on every task: the whole gain, and only the gain, is the close.
Fix: move the two marker anchors to bracket the *whole* close (before the slim attempt). Not done
here; recorded so no future D comparison on this branch is read as factual.

## 4. Pre-registered criteria (`XXVI.C1`, neg witness — not the witness route)

The slim close is **not** the witness compiler. Against `XXVI.C1`:

| criterion | target | result |
|---|---|---|
| neg closing phase < human 44,425 | < 44,425 | **1,364,282 — not met** (still 30.7×) |
| fact + close together lower than 3,074,910 | strictly lower | **2,875,978 — met** |
| matched-session total < 721,473 | < 721,473 | not measured; still ≈3.97M derived |
| `equals_thm` | mandatory | **not re-run** |
| SAT closes | 0 | inherited (algebraic route, no SAT call) |

So the close got 13.5 % cheaper on `neg`, but it is still three orders of magnitude above the human
closing phase. The witness route is still required.

## 5. Fixtures

`tools/bridge.ml` moved, so all 27 answer fixtures, `curve_check_bridge.txt`, `BRIDGE_DIGEST`
(`a37b4f1c…`), the instrument sha and the four inline-answer digests were re-recorded. `curve_check`
now reports **`statements=92 hits=0`** (the slim attempt adds one `prove` call; the fixture diff is
line numbers plus that one entry — every pre-existing statement text is unchanged). The assertion was
re-pinned from 91 to 92 with a boundary comment.

## 6. What is not established

- **No `equals_thm` re-check.** Still inherited from IX.89; mandatory before any victory claim.
- **No matched-session number.** These are strict self-contained totals.
- **No timing claim.** Wall times 27–32 s are observations, not medians of five.
- **Phase D attribution on this branch is artefactual** (§3).
- **`neg` remains 7.58× human.** This is a 3.76 % improvement, not a competitive result.

## Correction of record (2026-11-27, appended)

- §4 said the neg close is "three orders of magnitude above the human closing phase". The ratio is
  **1,364,282 / 44,425 ≈ 30.7×**, i.e. **≈1.5 orders of magnitude**, and it compares the generated
  close region with the human final-arithmetic phase; read it at equivalent phase boundaries. The
  original sentence is kept above; this line corrects it.
- IX.93's probe ran against the **full-close** baseline. Its 107,691 saving must not be subtracted
  from this receipt's 5,469,909 to form a cumulative figure; only a combined experiment may add them.
