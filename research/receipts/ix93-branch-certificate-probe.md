# IX.93 — branch-level certificate probe: a 108-inference refutation replaces a ~50,000-inference close

**Date:** 2026-11-27
**Branch:** `staging` at `3006b69`; probe harness under `.bench/xxvi-probe/` (git-ignored, scratch).
**Counter:** `hol-light-stamp2`. Each build under `tools/cap.sh 12G -- make -j1`, one at a time;
no cap raised, none fired (peak group RSS 1.77–1.81 GB). `free -g` before: 80 GB available.

## 1. What was probed

For `arm/neg_p25519`, at the pre-close point of two representative branches
(`research/receipts/xxvi-neg-preclose-goal.md` §4 and §6):

- **Branch 1** — goal shape A (`Σs = &0`, the `n = 0` output).
- **Branch 7** — goal shape B (`&0 = p − N`, the `n ≠ 0` output).

Four independent proof runs (`full`, `slim`, `min`, `cert`), each producing the accepted theorem
(`val BIGNUM_NEG_P25519_CORRECT`, `rc=0`). The `full` run reproduces the IX.83 baseline
**`STAMP_INFERENCES 5683733`** exactly, so the harness is validated.

## 2. Per-branch inference cost of the close

`inference_count()` delta around the close; the value is **in the final proof** for the route that
was accepted.

| branch | goal | antecedents full / slim / min | full close | slim close | `INT_ARITH` on min set | certificate |
|---:|---|---:|---:|---:|---:|---:|
| 1 | `Σs = &0` | 31 / 5 / **2** | 55,831 | 42,149 | 13,599 | **108** |
| 7 | `&0 = p − N` | 33 / 7 / **2** | 51,774 | 38,092 | 23,826 | **108** |

**Why 2 antecedents suffice: the branch is propositionally inconsistent.** Greedy deletion dropped
the aggregate, `bitval carry_s12 = 0`, all 26 direct bounds and both `N ≤ p` facts, leaving only the
`allzero` conjunction `P` and its AC-equivalent negation `¬Q` (branch 1: ants `#28`/`#29`; branch 7:
`#29`/`#31`). No linear combination is involved; the contradiction is logical.

The refutation is a real kernel proof:

```
pth : |- P        nth : |- ~Q          (P, Q AC-equivalent conjunctions)
pq  = EQ_MP (CONJ_ACI_RULE (mk_eq(P,Q))) pth     (* |- Q *)
fth = MP (NOT_ELIM nth) pq                        (* |- F *)
gth = CONTR w' fth                                (* |- w' *)
closed = EQ_MP (SYM w_eq) gth
```

Cost: **108 inferences per branch** (including the `CONJ_ACI` re-association, because the `/\`
nestings differ in association). That is **~500× cheaper** than the full close and **126–220×**
cheaper than `INT_ARITH` even on the two-element set.

## 3. Whole-proof effect (two branches only)

| route | `STAMP_INFERENCES` | Δ vs baseline |
|---|---:|---:|
| full (baseline) | 5,683,733 | — |
| slim (branches 1+7 only) | 5,656,369 | −27,364 |
| **certificate (branches 1+7 only)** | **5,576,042** | **−107,691** |
| `INT_ARITH` on min sets | 5,892,695 | +208,962 |

The last row is inflated by the **discarded greedy search** (103,386 + 176,058 attempts). Attempt
cost is not proof cost and is not claimed as a saving.

The per-branch deltas are exact; the certificate total sits **302 inferences (0.005 %)** below the
naive sum, an enclosing-proof residual from consuming a smaller oracle result.

## 4. What this implies and what it does not

- **Implication:** a sound, cheap close exists for any branch whose hypotheses contain a propositional
  contradiction. Branch 1 and branch 7 show it for `neg`. Whether the other eight branches have the
  same shape is **not measured**; if they do, the closing phase for `neg` could fall from ≈1.36M
  toward a few thousand inferences. That is the next experiment, not a claim.
- **It is not the witness compiler.** This closes *vacuous* branches cheaply; it does not prove the
  canonical-residue witness on the branches that carry real arithmetic. The witness route (`XXVI.C2`)
  is still required for those.
- **Sound?** Yes in the ordinary sense: every step is a derived HOL Light rule and the kernel accepts
  the theorem. It does exploit that the branch splitter leaves both `P` and `¬P` in a branch; if a
  branch were *not* contradictory the certificate would fail and the close would fall back.
- **Not established:** `equals_thm`, matched-session accounting, timing medians, and the count of
  vacuous branches.
