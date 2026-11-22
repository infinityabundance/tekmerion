# Receipt — s2n-bignum program equivalence (measured locally)

- **Artifact:** `awslabs/s2n-bignum`
- **URL:** <https://github.com/awslabs/s2n-bignum>
- **Doc:** <https://github.com/awslabs/s2n-bignum/blob/main/doc/program_equivalence.md>
- **Commit measured:** `1d097f630f0ca91cc2271491b9dd57dc9c9023a5`, 2026-11-21 20:29:26 +0000
  ("CI: Compare proof runtimes (#507)")
- **Measured:** 2026-11-22
- **Type:** primary artifact + first-party documentation.

## Provenance note (important)

I reused an existing shallow clone at `/tmp/s2n` that was already present on the machine. I did
**not** re-clone it in this session. I verified that its `HEAD` equals the pin already recorded
in `research/sources.md` (`1d097f6...`) and that `git status --porcelain` is empty (0 lines), so
the counts below correspond to the commit and not to a dirty working tree.

## Measurements (run by me, at the commit above)

```
git -C /tmp/s2n rev-parse HEAD
1d097f630f0ca91cc2271491b9dd57dc9c9023a5

git -C /tmp/s2n --no-optional-locks status --porcelain | wc -l
0

wc -l /tmp/s2n/arm/proofs/equiv.ml
2392 /tmp/s2n/arm/proofs/equiv.ml

grep -vcE '^\s*$|^\s*\(\*' /tmp/s2n/arm/proofs/equiv.ml
1880

ls -l /tmp/s2n/doc/program_equivalence.md
-rw-r--r-- 1 one one 42950 ... /tmp/s2n/doc/program_equivalence.md

grep -rln 'arm/proofs/equiv.ml' /tmp/s2n/arm/proofs | wc -l
11
```

The 11 ARM proof files that load `arm/proofs/equiv.ml`:

```
bignum_sqr_p521.ml
bignum_sqr_8_16.ml
bignum_mul_p521.ml
bignum_mul_8_16.ml
bignum_montsqr_p521.ml
bignum_montsqr_p384.ml
bignum_montsqr_p256.ml
bignum_montmul_p521.ml
bignum_montmul_p384.ml
bignum_montmul_p256.ml
bignum_emontredc_8n_cdiff.ml
```

No `x86/proofs/*.ml` file loads it (`grep -rln` over `x86/proofs` returned nothing).

## Exact quotes from `doc/program_equivalence.md`

> This file explains how program equivalence proofs are written for correctness of NIST curves
> in s2n-bignum as well as the optimized Montgomery reduction algorithm. The theoretical
> foundation for these proofs is the relational Hoare logic `L2` described in Mazzucato et al.,
> *"Relational Hoare Logic for Realistically Modelled Machine Code"*
> ([arXiv 2505.14348](https://arxiv.org/abs/2505.14348), CAV'25)

> There are several proofs of linear optimizations for the field operations of the
> P-256/384/521 elliptic curves:
>
> - [`arm/proofs/bignum_montmul_p256.ml`](../arm/proofs/bignum_montmul_p256.ml),
>   [`bignum_montmul_p384.ml`](../arm/proofs/bignum_montmul_p384.ml),
>   [`bignum_montmul_p521.ml`](../arm/proofs/bignum_montmul_p521.ml),
> - corresponding `bignum_montsqr_*` files,
> - [`bignum_mul_p521.ml`](../arm/proofs/bignum_mul_p521.ml) and
>   [`bignum_sqr_p521.ml`](../arm/proofs/bignum_sqr_p521.ml).

> Each of these composes (1) NEON vectorization followed by (2) SLOTHY instruction reordering.
> Each step proves a separate `ensures2` equivalence theorem; the two are then composed into a
> single scalar↔optimized equivalence; and the composed equivalence is combined with the
> pre-existing `ensures`-style functional correctness of the scalar reference to yield
> functional correctness of the optimized program.

> | `bignum_montmul_p256_unopt_mc` | original, scalar reference | `arm/p256/unopt/bignum_montmul_p256_base.o` |
> | `bignum_montmul_p256_interm1_mc` | NEON-vectorized intermediate | inline opcode list in the `.ml` proof script |
> | `bignum_montmul_p256_mc` | final, NEON + SLOTHY-scheduled | `arm/p256/bignum_montmul_p256.o` |

> **P384/P521.** The structure is identical. Same three-versions layout
> (`…_unopt_core_mc`, `…_interm1_core_mc`, `…_core_mc`), same `eqin`/`eqout`

## What it establishes

1. The equivalence decomposition exists and is documented.
2. It is used for the P-256, P-384 and P-521 Montgomery **multiply and square** routines, and
   for `bignum_mul_p521` / `bignum_sqr_p521`.
3. `arm/proofs/equiv.ml` is **2,392 total lines** (1,880 non-blank, non-comment) and is loaded
   by **exactly 11** files under `arm/proofs`.
4. The decomposition is: prove `unopt` functional correctness (`ensures`), prove
   `unopt ↔ interm1` and `interm1 ↔ final` equivalences (`ensures2`), compose by transitivity,
   then derive `ensures` correctness of the final optimised core.

## What it does not establish

- That the *agent* writes the twin. The scalar reference is pre-existing (`arm/*/unopt/*.o`).
- Anything about x86 (`x86/` does not use `equiv.ml` at this commit).
- That the composition is automatic or cheap; the doc describes hand-written action lists and
  `EQUIV_TRANS_TAC` invocations.
- That 2,392 is a *code* line count. It is `wc -l`; the code count is 1,880.
