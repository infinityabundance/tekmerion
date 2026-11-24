# IX.60 — Matched inference counts under the axiom-stamp kernel

**Status:** receipt. **Date:** 2026-11-24. **Branch:** `staging`.
**Pins:** s2n-bignum `5e0fc782`; HOL Light `d30e823`; kernel
`.bench/hol-light-stamp2` (patched `fusion.ml` sha256
`9c15e5260955fc6f22ad2c845bb7c7287a9580f3669be6e62829b7a9a53a0bfd`).

**Headline.** The two proofs are **not of the same statement**. `compare` therefore
returns `Err(TheoremIdentityMismatch)` for all 27 rows, `MatchedComparison` is never
constructed, and **no inference ratio is published**. The deliverable is the pair of
counts with the difference described. Human proofs were measured with the same
primitive-inference counter as the generated proofs, on the same kernel build, at
`-j1 --force`, one task at a time: **27/27** completed.

Why the counts still matter: they are deterministic and machine-independent, but a
quotient is not a proof-cost comparison when the obligations differ. The obligation
comparison (Section 1) decides that question mechanically.

## 1. The obligation question (XIII.E3): different, and incomparable

For each of the 27 tasks the two conclusions were extracted from the built
`.correct` files (`val <NAME> : thm = |- ...`), the `val … = |- ` prefix was
stripped, whitespace was normalised, and the resulting strings were compared in
Rust (`dropin obligation`). Result: **27 different, 0 same, 0 missing**.

The first difference is the quantifier list:
- the 18 `add`/`sub` tasks differ at character 19: human
  `forall z x y m n pc returnaddress.` vs generated `forall z x y m n pc.`;
- the 9 `neg` tasks (6 ARM + 3 x86) differ at character 15: human
  `forall z x n pc returnaddress.` (x86: `... pc stackpointer returnaddress.`) vs
  generated `forall z x n pc.`.

Every task has the extra `returnaddress` (and, on x86, `stackpointer`)
quantifiers.

Beyond the quantifiers, every task shows the same structural difference:
- the human precondition adds the return-address assumption: ARM
  `read X30 s = returnaddress`; x86 `read RSP s = stackpointer` and
  `read (memory :> bytes64 stackpointer) s = returnaddress`;
- the human postcondition asserts `read PC s = returnaddress` (ARM) or
  `read RIP s = returnaddress` and `read RSP s = stackpointer + 8` (x86), where
  the generated postcondition asserts `read PC/RIP s = word (pc + N)` for a
  fixed `N`;
- the human frame is `MAYCHANGE_REGS_AND_FLAGS_PERMITTED_BY_ABI` (it permits every
  ABI-clobberable register), while the generated frame is an explicit narrower
  `MAYCHANGE [...]` list.

**Verdict.** `_SUBROUTINE_CORRECT` is **not the same statement under a different
name**. It is also **neither strictly stronger nor strictly weaker**; the two
contracts are **incomparable**:
- the human does not imply the generated statement: its frame is broader, so it
  does not establish the generated narrower frame, and it constrains `PC` to the
  caller-provided `returnaddress` rather than to `pc + N`;
- the generated does not imply the human statement: it never mentions
  `returnaddress`/`X30`/`RSP`, so it cannot establish the human return-address
  postcondition for an arbitrary caller-provided return address.

This is a stronger conclusion than "different": the human proof is a subroutine
contract, the generated proof is a whole-program contract, and neither subsumes the
other. The mechanical evidence is the table below; the per-task
`first_difference` and `description` are in `.bench/ix60-obligation.json` and are
reproduced in full in the tool output that follows.

### 1.1 Tool output (`dropin obligation`)

```
$ target/debug/dropin obligation \
    --human-tree .bench/human-baseline --generated-tree .bench/dropin \
    --json .bench/ix60-obligation.json
OBLIGATIONS total=27 same=0 different=27 missing=0
```



| arch | task | human theorem | generated theorem | same? | verdict |
|---|---|---|---|---|---|
| arm | curve25519/add_p25519 | BIGNUM_ADD_P25519_SUBROUTINE_CORRECT | BIGNUM_ADD_P25519_CORRECT | False | different: neither strictly stronger nor strictly weaker (incomparable) |
| arm | p256/add_p256 | BIGNUM_ADD_P256_SUBROUTINE_CORRECT | BIGNUM_ADD_P256_CORRECT | False | different: neither strictly stronger nor strictly weaker (incomparable) |
| arm | secp256k1/add_p256k1 | BIGNUM_ADD_P256K1_SUBROUTINE_CORRECT | BIGNUM_ADD_P256K1_CORRECT | False | different: neither strictly stronger nor strictly weaker (incomparable) |
| arm | p384/add_p384 | BIGNUM_ADD_P384_SUBROUTINE_CORRECT | BIGNUM_ADD_P384_CORRECT | False | different: neither strictly stronger nor strictly weaker (incomparable) |
| arm | p521/add_p521 | BIGNUM_ADD_P521_SUBROUTINE_CORRECT | BIGNUM_ADD_P521_CORRECT | False | different: neither strictly stronger nor strictly weaker (incomparable) |
| arm | sm2/add_sm2 | BIGNUM_ADD_SM2_SUBROUTINE_CORRECT | BIGNUM_ADD_SM2_CORRECT | False | different: neither strictly stronger nor strictly weaker (incomparable) |
| arm | curve25519/sub_p25519 | BIGNUM_SUB_P25519_SUBROUTINE_CORRECT | BIGNUM_SUB_P25519_CORRECT | False | different: neither strictly stronger nor strictly weaker (incomparable) |
| arm | p256/sub_p256 | BIGNUM_SUB_P256_SUBROUTINE_CORRECT | BIGNUM_SUB_P256_CORRECT | False | different: neither strictly stronger nor strictly weaker (incomparable) |
| arm | secp256k1/sub_p256k1 | BIGNUM_SUB_P256K1_SUBROUTINE_CORRECT | BIGNUM_SUB_P256K1_CORRECT | False | different: neither strictly stronger nor strictly weaker (incomparable) |
| arm | p384/sub_p384 | BIGNUM_SUB_P384_SUBROUTINE_CORRECT | BIGNUM_SUB_P384_CORRECT | False | different: neither strictly stronger nor strictly weaker (incomparable) |
| arm | p521/sub_p521 | BIGNUM_SUB_P521_SUBROUTINE_CORRECT | BIGNUM_SUB_P521_CORRECT | False | different: neither strictly stronger nor strictly weaker (incomparable) |
| arm | sm2/sub_sm2 | BIGNUM_SUB_SM2_SUBROUTINE_CORRECT | BIGNUM_SUB_SM2_CORRECT | False | different: neither strictly stronger nor strictly weaker (incomparable) |
| arm | curve25519/neg_p25519 | BIGNUM_NEG_P25519_SUBROUTINE_CORRECT | BIGNUM_NEG_P25519_CORRECT | False | different: neither strictly stronger nor strictly weaker (incomparable) |
| arm | p256/neg_p256 | BIGNUM_NEG_P256_SUBROUTINE_CORRECT | BIGNUM_NEG_P256_CORRECT | False | different: neither strictly stronger nor strictly weaker (incomparable) |
| arm | secp256k1/neg_p256k1 | BIGNUM_NEG_P256K1_SUBROUTINE_CORRECT | BIGNUM_NEG_P256K1_CORRECT | False | different: neither strictly stronger nor strictly weaker (incomparable) |
| arm | p384/neg_p384 | BIGNUM_NEG_P384_SUBROUTINE_CORRECT | BIGNUM_NEG_P384_CORRECT | False | different: neither strictly stronger nor strictly weaker (incomparable) |
| arm | p521/neg_p521 | BIGNUM_NEG_P521_SUBROUTINE_CORRECT | BIGNUM_NEG_P521_CORRECT | False | different: neither strictly stronger nor strictly weaker (incomparable) |
| arm | sm2/neg_sm2 | BIGNUM_NEG_SM2_SUBROUTINE_CORRECT | BIGNUM_NEG_SM2_CORRECT | False | different: neither strictly stronger nor strictly weaker (incomparable) |
| x86 | curve25519/add_p25519 | BIGNUM_ADD_P25519_SUBROUTINE_CORRECT | BIGNUM_ADD_P25519_CORRECT | False | different: neither strictly stronger nor strictly weaker (incomparable) |
| x86 | secp256k1/add_p256k1 | BIGNUM_ADD_P256K1_SUBROUTINE_CORRECT | BIGNUM_ADD_P256K1_CORRECT | False | different: neither strictly stronger nor strictly weaker (incomparable) |
| x86 | sm2/add_sm2 | BIGNUM_ADD_SM2_SUBROUTINE_CORRECT | BIGNUM_ADD_SM2_CORRECT | False | different: neither strictly stronger nor strictly weaker (incomparable) |
| x86 | curve25519/sub_p25519 | BIGNUM_SUB_P25519_SUBROUTINE_CORRECT | BIGNUM_SUB_P25519_CORRECT | False | different: neither strictly stronger nor strictly weaker (incomparable) |
| x86 | secp256k1/sub_p256k1 | BIGNUM_SUB_P256K1_SUBROUTINE_CORRECT | BIGNUM_SUB_P256K1_CORRECT | False | different: neither strictly stronger nor strictly weaker (incomparable) |
| x86 | sm2/sub_sm2 | BIGNUM_SUB_SM2_SUBROUTINE_CORRECT | BIGNUM_SUB_SM2_CORRECT | False | different: neither strictly stronger nor strictly weaker (incomparable) |
| x86 | curve25519/neg_p25519 | BIGNUM_NEG_P25519_SUBROUTINE_CORRECT | BIGNUM_NEG_P25519_CORRECT | False | different: neither strictly stronger nor strictly weaker (incomparable) |
| x86 | secp256k1/neg_p256k1 | BIGNUM_NEG_P256K1_SUBROUTINE_CORRECT | BIGNUM_NEG_P256K1_CORRECT | False | different: neither strictly stronger nor strictly weaker (incomparable) |
| x86 | sm2/neg_sm2 | BIGNUM_NEG_SM2_SUBROUTINE_CORRECT | BIGNUM_NEG_SM2_CORRECT | False | different: neither strictly stronger nor strictly weaker (incomparable) |

### Per-task evidence

#### arm/curve25519 add
- first difference at character 19:
  - human:     `forall z x y m n pc returnaddress. nonoverlapping (word pc,80) (z,8 * 4) ==> en`
  - generated: `forall z x y m n pc. nonoverlapping (word pc,80) (z,8 * 4) ==> ensures arm (\s.`
- description: the human conclusion quantifies a caller-provided `returnaddress` (and, on x86, `stackpointer`) that the generated conclusion does not mention; the human precondition assumes `read X30 s = returnaddress`; the human postcondition asserts `PC = returnaddress`; the generated postcondition asserts `PC = pc + N` for a fixed N; the human frame is `MAYCHANGE_REGS_AND_FLAGS_PERMITTED_BY_ABI` (broader: it permits every ABI-clobberable register), while the generated frame is an explicit `MAYCHANGE` list (narrower)
- verdict: different: neither strictly stronger nor strictly weaker (incomparable)

#### arm/p256 add
- first difference at character 19:
  - human:     `forall z x y m n pc returnaddress. nonoverlapping (word pc,92) (z,8 * 4) ==> en`
  - generated: `forall z x y m n pc. nonoverlapping (word pc,92) (z,8 * 4) ==> ensures arm (\s.`
- description: the human conclusion quantifies a caller-provided `returnaddress` (and, on x86, `stackpointer`) that the generated conclusion does not mention; the human precondition assumes `read X30 s = returnaddress`; the human postcondition asserts `PC = returnaddress`; the generated postcondition asserts `PC = pc + N` for a fixed N; the human frame is `MAYCHANGE_REGS_AND_FLAGS_PERMITTED_BY_ABI` (broader: it permits every ABI-clobberable register), while the generated frame is an explicit `MAYCHANGE` list (narrower)
- verdict: different: neither strictly stronger nor strictly weaker (incomparable)

#### arm/secp256k1 add
- first difference at character 19:
  - human:     `forall z x y m n pc returnaddress. nonoverlapping (word pc,96) (z,8 * 4) ==> en`
  - generated: `forall z x y m n pc. nonoverlapping (word pc,96) (z,8 * 4) ==> ensures arm (\s.`
- description: the human conclusion quantifies a caller-provided `returnaddress` (and, on x86, `stackpointer`) that the generated conclusion does not mention; the human precondition assumes `read X30 s = returnaddress`; the human postcondition asserts `PC = returnaddress`; the generated postcondition asserts `PC = pc + N` for a fixed N; the human frame is `MAYCHANGE_REGS_AND_FLAGS_PERMITTED_BY_ABI` (broader: it permits every ABI-clobberable register), while the generated frame is an explicit `MAYCHANGE` list (narrower)
- verdict: different: neither strictly stronger nor strictly weaker (incomparable)

#### arm/p384 add
- first difference at character 19:
  - human:     `forall z x y m n pc returnaddress. nonoverlapping (word pc,156) (z,8 * 6) ==> e`
  - generated: `forall z x y m n pc. nonoverlapping (word pc,156) (z,8 * 6) ==> ensures arm (\s`
- description: the human conclusion quantifies a caller-provided `returnaddress` (and, on x86, `stackpointer`) that the generated conclusion does not mention; the human precondition assumes `read X30 s = returnaddress`; the human postcondition asserts `PC = returnaddress`; the generated postcondition asserts `PC = pc + N` for a fixed N; the human frame is `MAYCHANGE_REGS_AND_FLAGS_PERMITTED_BY_ABI` (broader: it permits every ABI-clobberable register), while the generated frame is an explicit `MAYCHANGE` list (narrower)
- verdict: different: neither strictly stronger nor strictly weaker (incomparable)

#### arm/p521 add
- first difference at character 19:
  - human:     `forall z x y m n pc returnaddress. nonoverlapping (word pc,152) (z,8 * 9) ==> e`
  - generated: `forall z x y m n pc. nonoverlapping (word pc,152) (z,8 * 9) ==> ensures arm (\s`
- description: the human conclusion quantifies a caller-provided `returnaddress` (and, on x86, `stackpointer`) that the generated conclusion does not mention; the human precondition assumes `read X30 s = returnaddress`; the human postcondition asserts `PC = returnaddress`; the generated postcondition asserts `PC = pc + N` for a fixed N; the human frame is `MAYCHANGE_REGS_AND_FLAGS_PERMITTED_BY_ABI` (broader: it permits every ABI-clobberable register), while the generated frame is an explicit `MAYCHANGE` list (narrower)
- verdict: different: neither strictly stronger nor strictly weaker (incomparable)

#### arm/sm2 add
- first difference at character 19:
  - human:     `forall z x y m n pc returnaddress. nonoverlapping (word pc,92) (z,8 * 4) ==> en`
  - generated: `forall z x y m n pc. nonoverlapping (word pc,92) (z,8 * 4) ==> ensures arm (\s.`
- description: the human conclusion quantifies a caller-provided `returnaddress` (and, on x86, `stackpointer`) that the generated conclusion does not mention; the human precondition assumes `read X30 s = returnaddress`; the human postcondition asserts `PC = returnaddress`; the generated postcondition asserts `PC = pc + N` for a fixed N; the human frame is `MAYCHANGE_REGS_AND_FLAGS_PERMITTED_BY_ABI` (broader: it permits every ABI-clobberable register), while the generated frame is an explicit `MAYCHANGE` list (narrower)
- verdict: different: neither strictly stronger nor strictly weaker (incomparable)

#### arm/curve25519 sub
- first difference at character 19:
  - human:     `forall z x y m n pc returnaddress. nonoverlapping (word pc,72) (z,8 * 4) ==> en`
  - generated: `forall z x y m n pc. nonoverlapping (word pc,72) (z,8 * 4) ==> ensures arm (\s.`
- description: the human conclusion quantifies a caller-provided `returnaddress` (and, on x86, `stackpointer`) that the generated conclusion does not mention; the human precondition assumes `read X30 s = returnaddress`; the human postcondition asserts `PC = returnaddress`; the generated postcondition asserts `PC = pc + N` for a fixed N; the human frame is `MAYCHANGE_REGS_AND_FLAGS_PERMITTED_BY_ABI` (broader: it permits every ABI-clobberable register), while the generated frame is an explicit `MAYCHANGE` list (narrower)
- verdict: different: neither strictly stronger nor strictly weaker (incomparable)

#### arm/p256 sub
- first difference at character 19:
  - human:     `forall z x y m n pc returnaddress. nonoverlapping (word pc,72) (z,8 * 4) ==> en`
  - generated: `forall z x y m n pc. nonoverlapping (word pc,72) (z,8 * 4) ==> ensures arm (\s.`
- description: the human conclusion quantifies a caller-provided `returnaddress` (and, on x86, `stackpointer`) that the generated conclusion does not mention; the human precondition assumes `read X30 s = returnaddress`; the human postcondition asserts `PC = returnaddress`; the generated postcondition asserts `PC = pc + N` for a fixed N; the human frame is `MAYCHANGE_REGS_AND_FLAGS_PERMITTED_BY_ABI` (broader: it permits every ABI-clobberable register), while the generated frame is an explicit `MAYCHANGE` list (narrower)
- verdict: different: neither strictly stronger nor strictly weaker (incomparable)

#### arm/secp256k1 sub
- first difference at character 19:
  - human:     `forall z x y m n pc returnaddress. nonoverlapping (word pc,72) (z,8 * 4) ==> en`
  - generated: `forall z x y m n pc. nonoverlapping (word pc,72) (z,8 * 4) ==> ensures arm (\s.`
- description: the human conclusion quantifies a caller-provided `returnaddress` (and, on x86, `stackpointer`) that the generated conclusion does not mention; the human precondition assumes `read X30 s = returnaddress`; the human postcondition asserts `PC = returnaddress`; the generated postcondition asserts `PC = pc + N` for a fixed N; the human frame is `MAYCHANGE_REGS_AND_FLAGS_PERMITTED_BY_ABI` (broader: it permits every ABI-clobberable register), while the generated frame is an explicit `MAYCHANGE` list (narrower)
- verdict: different: neither strictly stronger nor strictly weaker (incomparable)

#### arm/p384 sub
- first difference at character 19:
  - human:     `forall z x y m n pc returnaddress. nonoverlapping (word pc,112) (z,8 * 6) ==> e`
  - generated: `forall z x y m n pc. nonoverlapping (word pc,112) (z,8 * 6) ==> ensures arm (\s`
- description: the human conclusion quantifies a caller-provided `returnaddress` (and, on x86, `stackpointer`) that the generated conclusion does not mention; the human precondition assumes `read X30 s = returnaddress`; the human postcondition asserts `PC = returnaddress`; the generated postcondition asserts `PC = pc + N` for a fixed N; the human frame is `MAYCHANGE_REGS_AND_FLAGS_PERMITTED_BY_ABI` (broader: it permits every ABI-clobberable register), while the generated frame is an explicit `MAYCHANGE` list (narrower)
- verdict: different: neither strictly stronger nor strictly weaker (incomparable)

#### arm/p521 sub
- first difference at character 19:
  - human:     `forall z x y m n pc returnaddress. nonoverlapping (word pc,140) (z,8 * 9) ==> e`
  - generated: `forall z x y m n pc. nonoverlapping (word pc,140) (z,8 * 9) ==> ensures arm (\s`
- description: the human conclusion quantifies a caller-provided `returnaddress` (and, on x86, `stackpointer`) that the generated conclusion does not mention; the human precondition assumes `read X30 s = returnaddress`; the human postcondition asserts `PC = returnaddress`; the generated postcondition asserts `PC = pc + N` for a fixed N; the human frame is `MAYCHANGE_REGS_AND_FLAGS_PERMITTED_BY_ABI` (broader: it permits every ABI-clobberable register), while the generated frame is an explicit `MAYCHANGE` list (narrower)
- verdict: different: neither strictly stronger nor strictly weaker (incomparable)

#### arm/sm2 sub
- first difference at character 19:
  - human:     `forall z x y m n pc returnaddress. nonoverlapping (word pc,72) (z,8 * 4) ==> en`
  - generated: `forall z x y m n pc. nonoverlapping (word pc,72) (z,8 * 4) ==> ensures arm (\s.`
- description: the human conclusion quantifies a caller-provided `returnaddress` (and, on x86, `stackpointer`) that the generated conclusion does not mention; the human precondition assumes `read X30 s = returnaddress`; the human postcondition asserts `PC = returnaddress`; the generated postcondition asserts `PC = pc + N` for a fixed N; the human frame is `MAYCHANGE_REGS_AND_FLAGS_PERMITTED_BY_ABI` (broader: it permits every ABI-clobberable register), while the generated frame is an explicit `MAYCHANGE` list (narrower)
- verdict: different: neither strictly stronger nor strictly weaker (incomparable)

#### arm/curve25519 neg
- first difference at character 15:
  - human:     `forall z x n pc returnaddress. nonoverlapping (word pc,80) (z,8 * 4) ==> en`
  - generated: `forall z x n pc. nonoverlapping (word pc,80) (z,8 * 4) ==> ensures arm (\s.`
- description: the human conclusion quantifies a caller-provided `returnaddress` (and, on x86, `stackpointer`) that the generated conclusion does not mention; the human precondition assumes `read X30 s = returnaddress`; the human postcondition asserts `PC = returnaddress`; the generated postcondition asserts `PC = pc + N` for a fixed N; the human frame is `MAYCHANGE_REGS_AND_FLAGS_PERMITTED_BY_ABI` (broader: it permits every ABI-clobberable register), while the generated frame is an explicit `MAYCHANGE` list (narrower)
- verdict: different: neither strictly stronger nor strictly weaker (incomparable)

#### arm/p256 neg
- first difference at character 15:
  - human:     `forall z x n pc returnaddress. nonoverlapping (word pc,64) (z,8 * 4) ==> en`
  - generated: `forall z x n pc. nonoverlapping (word pc,64) (z,8 * 4) ==> ensures arm (\s.`
- description: the human conclusion quantifies a caller-provided `returnaddress` (and, on x86, `stackpointer`) that the generated conclusion does not mention; the human precondition assumes `read X30 s = returnaddress`; the human postcondition asserts `PC = returnaddress`; the generated postcondition asserts `PC = pc + N` for a fixed N; the human frame is `MAYCHANGE_REGS_AND_FLAGS_PERMITTED_BY_ABI` (broader: it permits every ABI-clobberable register), while the generated frame is an explicit `MAYCHANGE` list (narrower)
- verdict: different: neither strictly stronger nor strictly weaker (incomparable)

#### arm/secp256k1 neg
- first difference at character 15:
  - human:     `forall z x n pc returnaddress. nonoverlapping (word pc,68) (z,8 * 4) ==> en`
  - generated: `forall z x n pc. nonoverlapping (word pc,68) (z,8 * 4) ==> ensures arm (\s.`
- description: the human conclusion quantifies a caller-provided `returnaddress` (and, on x86, `stackpointer`) that the generated conclusion does not mention; the human precondition assumes `read X30 s = returnaddress`; the human postcondition asserts `PC = returnaddress`; the generated postcondition asserts `PC = pc + N` for a fixed N; the human frame is `MAYCHANGE_REGS_AND_FLAGS_PERMITTED_BY_ABI` (broader: it permits every ABI-clobberable register), while the generated frame is an explicit `MAYCHANGE` list (narrower)
- verdict: different: neither strictly stronger nor strictly weaker (incomparable)

#### arm/p384 neg
- first difference at character 15:
  - human:     `forall z x n pc returnaddress. nonoverlapping (word pc,92) (z,8 * 6) ==> en`
  - generated: `forall z x n pc. nonoverlapping (word pc,92) (z,8 * 6) ==> ensures arm (\s.`
- description: the human conclusion quantifies a caller-provided `returnaddress` (and, on x86, `stackpointer`) that the generated conclusion does not mention; the human precondition assumes `read X30 s = returnaddress`; the human postcondition asserts `PC = returnaddress`; the generated postcondition asserts `PC = pc + N` for a fixed N; the human frame is `MAYCHANGE_REGS_AND_FLAGS_PERMITTED_BY_ABI` (broader: it permits every ABI-clobberable register), while the generated frame is an explicit `MAYCHANGE` list (narrower)
- verdict: different: neither strictly stronger nor strictly weaker (incomparable)

#### arm/p521 neg
- first difference at character 15:
  - human:     `forall z x n pc returnaddress. nonoverlapping (word pc,124) (z,8 * 9) ==> e`
  - generated: `forall z x n pc. nonoverlapping (word pc,124) (z,8 * 9) ==> ensures arm (\s`
- description: the human conclusion quantifies a caller-provided `returnaddress` (and, on x86, `stackpointer`) that the generated conclusion does not mention; the human precondition assumes `read X30 s = returnaddress`; the human postcondition asserts `PC = returnaddress`; the generated postcondition asserts `PC = pc + N` for a fixed N; the human frame is `MAYCHANGE_REGS_AND_FLAGS_PERMITTED_BY_ABI` (broader: it permits every ABI-clobberable register), while the generated frame is an explicit `MAYCHANGE` list (narrower)
- verdict: different: neither strictly stronger nor strictly weaker (incomparable)

#### arm/sm2 neg
- first difference at character 15:
  - human:     `forall z x n pc returnaddress. nonoverlapping (word pc,64) (z,8 * 4) ==> en`
  - generated: `forall z x n pc. nonoverlapping (word pc,64) (z,8 * 4) ==> ensures arm (\s.`
- description: the human conclusion quantifies a caller-provided `returnaddress` (and, on x86, `stackpointer`) that the generated conclusion does not mention; the human precondition assumes `read X30 s = returnaddress`; the human postcondition asserts `PC = returnaddress`; the generated postcondition asserts `PC = pc + N` for a fixed N; the human frame is `MAYCHANGE_REGS_AND_FLAGS_PERMITTED_BY_ABI` (broader: it permits every ABI-clobberable register), while the generated frame is an explicit `MAYCHANGE` list (narrower)
- verdict: different: neither strictly stronger nor strictly weaker (incomparable)

#### x86/curve25519 add
- first difference at character 19:
  - human:     `forall z x y m n pc stackpointer returnaddress. nonoverlapping (word pc,LENGTH `
  - generated: `forall z x y m n pc. nonoverlapping (word pc,90) (z,8 * 4) ==> ensures x86 (\s.`
- description: the human conclusion quantifies a caller-provided `returnaddress` (and, on x86, `stackpointer`) that the generated conclusion does not mention; the human precondition assumes the stack pointer and the saved return address in memory; the human postcondition asserts `PC = returnaddress`; the generated postcondition asserts `PC = pc + N` for a fixed N; the human postcondition asserts `RSP = stackpointer + 8`; the human frame is `MAYCHANGE_REGS_AND_FLAGS_PERMITTED_BY_ABI` (broader: it permits every ABI-clobberable register), while the generated frame is an explicit `MAYCHANGE` list (narrower)
- verdict: different: neither strictly stronger nor strictly weaker (incomparable)

#### x86/secp256k1 add
- first difference at character 19:
  - human:     `forall z x y m n pc stackpointer returnaddress. nonoverlapping (word pc,LENGTH `
  - generated: `forall z x y m n pc. nonoverlapping (word pc,101) (z,8 * 4) ==> ensures x86 (\s`
- description: the human conclusion quantifies a caller-provided `returnaddress` (and, on x86, `stackpointer`) that the generated conclusion does not mention; the human precondition assumes the stack pointer and the saved return address in memory; the human postcondition asserts `PC = returnaddress`; the generated postcondition asserts `PC = pc + N` for a fixed N; the human postcondition asserts `RSP = stackpointer + 8`; the human frame is `MAYCHANGE_REGS_AND_FLAGS_PERMITTED_BY_ABI` (broader: it permits every ABI-clobberable register), while the generated frame is an explicit `MAYCHANGE` list (narrower)
- verdict: different: neither strictly stronger nor strictly weaker (incomparable)

#### x86/sm2 add
- first difference at character 19:
  - human:     `forall z x y m n pc stackpointer returnaddress. nonoverlapping (word pc,LENGTH `
  - generated: `forall z x y m n pc. nonoverlapping (word pc,108) (z,8 * 4) ==> ensures x86 (\s`
- description: the human conclusion quantifies a caller-provided `returnaddress` (and, on x86, `stackpointer`) that the generated conclusion does not mention; the human precondition assumes the stack pointer and the saved return address in memory; the human postcondition asserts `PC = returnaddress`; the generated postcondition asserts `PC = pc + N` for a fixed N; the human postcondition asserts `RSP = stackpointer + 8`; the human frame is `MAYCHANGE_REGS_AND_FLAGS_PERMITTED_BY_ABI` (broader: it permits every ABI-clobberable register), while the generated frame is an explicit `MAYCHANGE` list (narrower)
- verdict: different: neither strictly stronger nor strictly weaker (incomparable)

#### x86/curve25519 sub
- first difference at character 19:
  - human:     `forall z x y m n pc stackpointer returnaddress. nonoverlapping (word pc,LENGTH `
  - generated: `forall z x y m n pc. nonoverlapping (word pc,72) (z,8 * 4) ==> ensures x86 (\s.`
- description: the human conclusion quantifies a caller-provided `returnaddress` (and, on x86, `stackpointer`) that the generated conclusion does not mention; the human precondition assumes the stack pointer and the saved return address in memory; the human postcondition asserts `PC = returnaddress`; the generated postcondition asserts `PC = pc + N` for a fixed N; the human postcondition asserts `RSP = stackpointer + 8`; the human frame is `MAYCHANGE_REGS_AND_FLAGS_PERMITTED_BY_ABI` (broader: it permits every ABI-clobberable register), while the generated frame is an explicit `MAYCHANGE` list (narrower)
- verdict: different: neither strictly stronger nor strictly weaker (incomparable)

#### x86/secp256k1 sub
- first difference at character 19:
  - human:     `forall z x y m n pc stackpointer returnaddress. nonoverlapping (word pc,LENGTH `
  - generated: `forall z x y m n pc. nonoverlapping (word pc,74) (z,8 * 4) ==> ensures x86 (\s.`
- description: the human conclusion quantifies a caller-provided `returnaddress` (and, on x86, `stackpointer`) that the generated conclusion does not mention; the human precondition assumes the stack pointer and the saved return address in memory; the human postcondition asserts `PC = returnaddress`; the generated postcondition asserts `PC = pc + N` for a fixed N; the human postcondition asserts `RSP = stackpointer + 8`; the human frame is `MAYCHANGE_REGS_AND_FLAGS_PERMITTED_BY_ABI` (broader: it permits every ABI-clobberable register), while the generated frame is an explicit `MAYCHANGE` list (narrower)
- verdict: different: neither strictly stronger nor strictly weaker (incomparable)

#### x86/sm2 sub
- first difference at character 19:
  - human:     `forall z x y m n pc stackpointer returnaddress. nonoverlapping (word pc,LENGTH `
  - generated: `forall z x y m n pc. nonoverlapping (word pc,82) (z,8 * 4) ==> ensures x86 (\s.`
- description: the human conclusion quantifies a caller-provided `returnaddress` (and, on x86, `stackpointer`) that the generated conclusion does not mention; the human precondition assumes the stack pointer and the saved return address in memory; the human postcondition asserts `PC = returnaddress`; the generated postcondition asserts `PC = pc + N` for a fixed N; the human postcondition asserts `RSP = stackpointer + 8`; the human frame is `MAYCHANGE_REGS_AND_FLAGS_PERMITTED_BY_ABI` (broader: it permits every ABI-clobberable register), while the generated frame is an explicit `MAYCHANGE` list (narrower)
- verdict: different: neither strictly stronger nor strictly weaker (incomparable)

#### x86/curve25519 neg
- first difference at character 15:
  - human:     `forall z x n pc stackpointer returnaddress. ALL (nonoverlapping (z,8 * 4)) `
  - generated: `forall z x n pc. nonoverlapping (word pc,82) (z,8 * 4) ==> ensures x86 (\s.`
- description: the human conclusion quantifies a caller-provided `returnaddress` (and, on x86, `stackpointer`) that the generated conclusion does not mention; the human precondition assumes the stack pointer and the saved return address in memory; the human postcondition asserts `PC = returnaddress`; the generated postcondition asserts `PC = pc + N` for a fixed N; the human postcondition asserts `RSP = stackpointer + 8`; the human frame is `MAYCHANGE_REGS_AND_FLAGS_PERMITTED_BY_ABI` (broader: it permits every ABI-clobberable register), while the generated frame is an explicit `MAYCHANGE` list (narrower)
- verdict: different: neither strictly stronger nor strictly weaker (incomparable)

#### x86/secp256k1 neg
- first difference at character 15:
  - human:     `forall z x n pc stackpointer returnaddress. ALL (nonoverlapping (z,8 * 4)) `
  - generated: `forall z x n pc. nonoverlapping (word pc,80) (z,8 * 4) ==> ensures x86 (\s.`
- description: the human conclusion quantifies a caller-provided `returnaddress` (and, on x86, `stackpointer`) that the generated conclusion does not mention; the human precondition assumes the stack pointer and the saved return address in memory; the human postcondition asserts `PC = returnaddress`; the generated postcondition asserts `PC = pc + N` for a fixed N; the human postcondition asserts `RSP = stackpointer + 8`; the human frame is `MAYCHANGE_REGS_AND_FLAGS_PERMITTED_BY_ABI` (broader: it permits every ABI-clobberable register), while the generated frame is an explicit `MAYCHANGE` list (narrower)
- verdict: different: neither strictly stronger nor strictly weaker (incomparable)

#### x86/sm2 neg
- first difference at character 15:
  - human:     `forall z x n pc stackpointer returnaddress. ALL (nonoverlapping (z,8 * 4)) `
  - generated: `forall z x n pc. nonoverlapping (word pc,90) (z,8 * 4) ==> ensures x86 (\s.`
- description: the human conclusion quantifies a caller-provided `returnaddress` (and, on x86, `stackpointer`) that the generated conclusion does not mention; the human precondition assumes the stack pointer and the saved return address in memory; the human postcondition asserts `PC = returnaddress`; the generated postcondition asserts `PC = pc + N` for a fixed N; the human postcondition asserts `RSP = stackpointer + 8`; the human frame is `MAYCHANGE_REGS_AND_FLAGS_PERMITTED_BY_ABI` (broader: it permits every ABI-clobberable register), while the generated frame is an explicit `MAYCHANGE` list (narrower)
- verdict: different: neither strictly stronger nor strictly weaker (incomparable)

OBLIGATIONS total=27 same=0 different=27 missing=0

## 2. Matched inference counts (XIV.B1, XIV.B2)

### What the human delta is

Each human proof file was instrumented in place (a guarded copy is restored after
`make`):

1. the literal first line is `let stamp_mark_<base> = inference_count ();;` (the
   underscore spelling `_stamp_mark_` in the brief is parsed by this camlp5 kernel
   as an unbound constructor, so the equivalent non-underscore spelling is used);
2. immediately after the shared `needs "<arch>/proofs/base.ml";;` line,
   `let stamp_body_<base> = inference_count ();;`;
3. at end of file, `STAMP_BEGIN`, the canonical `STAMP`, `STAMP_INF_BEFORE`,
   `STAMP_INF_AFTER`, `STAMP_INFERENCES`, `STAMP_INF_BODY_BEFORE`,
   `STAMP_INF_BODY`, the registry, the named-axiom lines, the footprint, and the
   `val <THEOREM> : thm = ...` line, in the same format the existing parser
   consumes.

The two marks measure different things:
- **`STAMP_INF_BEFORE` / `STAMP_INFERENCES`** is the **whole-file** delta from
  process start: the shared HOL Light bootstrap (`needs`/`base.ml` and the
  standard library), plus the decode, plus both the internal `BIGNUM_*_CORRECT`
  proof and the `BIGNUM_*_SUBROUTINE_CORRECT` wrapper. Its sum over the 27 human
  rows is **1,722,542,510** (from `human_inferences_whole_file` in the JSON;
  `jq '[.tasks[].human_inferences_whole_file] | add'`).
- **`STAMP_INF_BODY_BEFORE` / `STAMP_INF_BODY`** is the **proof-body** delta: it
  starts after the shared bootstrap. Its sum over the 27 human rows is
  **176,369,762**, and that is the `inferences` value in the table and JSON.

The generated side's `inference_delta` (IX.57b) is also a proof-body delta: its
mark is placed after the ELF decode and `ARM_MK_EXEC_RULE`/`X86_MK_EXEC_RULE` and
immediately before the main `BIGNUM_*_CORRECT` `prove`, and it runs to the end of
the file (including the inlined CSAT/liaison lemmas and any auxiliary `prove`s
that follow). The two sides therefore both exclude the shared bootstrap. The human
body delta additionally includes its decode, which is not separately measured
here, and it covers both the internal `_CORRECT` proof and the `_SUBROUTINE_CORRECT`
wrapper. **It is the primitive-inference work of the whole proof file, not "the
theorem's derivation size".**

### The table

`compare` refuses every row: the theorem identities differ
(`BIGNUM_*_SUBROUTINE_CORRECT` vs `BIGNUM_*_CORRECT`). The aggregates are therefore
over **n = 0 matched pairs**; the side totals below the table are explicitly
descriptive, not aggregates. The generated count is larger than the human count in
all 27 rows; the difference ranges from **2,384,033** (x86 `curve25519/neg`) to
**672,871,909** (arm `p521/add`). **No quotient is published as a proof-cost
ratio.**

```
$ target/debug/dropin matched-table \
    --human .bench/measure-logs/ix60-human/human.json \
    --generated research/receipts/ix57b-axiom-stamp.json \
    --human-kernel hol-light-stamp2 --generated-kernel hol-light-stamp2 \
    --json research/receipts/ix60-matched-inference.json
MATCHED 0/27 PAIRS; UNMATCHED 27; HUMAN ROWS 27; GENERATED ROWS 27
```


A row is published only as a pair. `compare` requires the exact theorem
identity, both judgments successful (`exit == 0`, `check_axioms`, theorem
proved), and identical measurement scopes. `human inf` is the proof-body
primitive-inference delta: the shared HOL Light bootstrap is excluded on
both sides.

| arch | task | human theorem | generated theorem | human inf | generated inf | human s | generated s | verdict |
|---|---|---|---|---:|---:|---:|---:|---|
| arm | curve25519/add_p25519 | BIGNUM_ADD_P25519_SUBROUTINE_CORRECT | BIGNUM_ADD_P25519_CORRECT | 3848039 | 170225275 | 25.000 | 46.000 | refused: theorem identity |
| arm | p256/add_p256 | BIGNUM_ADD_P256_SUBROUTINE_CORRECT | BIGNUM_ADD_P256_CORRECT | 3872266 | 170931670 | 25.000 | 47.000 | refused: theorem identity |
| arm | secp256k1/add_p256k1 | BIGNUM_ADD_P256K1_SUBROUTINE_CORRECT | BIGNUM_ADD_P256K1_CORRECT | 3672894 | 157240305 | 25.000 | 46.000 | refused: theorem identity |
| arm | p384/add_p384 | BIGNUM_ADD_P384_SUBROUTINE_CORRECT | BIGNUM_ADD_P384_CORRECT | 8059478 | 349139623 | 27.000 | 74.000 | refused: theorem identity |
| arm | p521/add_p521 | BIGNUM_ADD_P521_SUBROUTINE_CORRECT | BIGNUM_ADD_P521_CORRECT | 8835293 | 681707202 | 27.000 | 122.000 | refused: theorem identity |
| arm | sm2/add_sm2 | BIGNUM_ADD_SM2_SUBROUTINE_CORRECT | BIGNUM_ADD_SM2_CORRECT | 3856358 | 159985379 | 25.000 | 46.000 | refused: theorem identity |
| arm | curve25519/sub_p25519 | BIGNUM_SUB_P25519_SUBROUTINE_CORRECT | BIGNUM_SUB_P25519_CORRECT | 3261274 | 110957174 | 25.000 | 39.000 | refused: theorem identity |
| arm | p256/sub_p256 | BIGNUM_SUB_P256_SUBROUTINE_CORRECT | BIGNUM_SUB_P256_CORRECT | 3395549 | 103417925 | 24.000 | 38.000 | refused: theorem identity |
| arm | secp256k1/sub_p256k1 | BIGNUM_SUB_P256K1_SUBROUTINE_CORRECT | BIGNUM_SUB_P256K1_CORRECT | 3296935 | 110121226 | 25.000 | 39.000 | refused: theorem identity |
| arm | p384/sub_p384 | BIGNUM_SUB_P384_SUBROUTINE_CORRECT | BIGNUM_SUB_P384_CORRECT | 6475221 | 214410496 | 26.000 | 55.000 | refused: theorem identity |
| arm | p521/sub_p521 | BIGNUM_SUB_P521_SUBROUTINE_CORRECT | BIGNUM_SUB_P521_CORRECT | 8753292 | 221845891 | 26.000 | 59.000 | refused: theorem identity |
| arm | sm2/sub_sm2 | BIGNUM_SUB_SM2_SUBROUTINE_CORRECT | BIGNUM_SUB_SM2_CORRECT | 3391866 | 107504057 | 24.000 | 39.000 | refused: theorem identity |
| arm | curve25519/neg_p25519 | BIGNUM_NEG_P25519_SUBROUTINE_CORRECT | BIGNUM_NEG_P25519_CORRECT | 4077908 | 35457948 | 24.000 | 29.000 | refused: theorem identity |
| arm | p256/neg_p256 | BIGNUM_NEG_P256_SUBROUTINE_CORRECT | BIGNUM_NEG_P256_CORRECT | 2712659 | 45739252 | 24.000 | 31.000 | refused: theorem identity |
| arm | secp256k1/neg_p256k1 | BIGNUM_NEG_P256K1_SUBROUTINE_CORRECT | BIGNUM_NEG_P256K1_CORRECT | 2704476 | 45424790 | 24.000 | 30.000 | refused: theorem identity |
| arm | p384/neg_p384 | BIGNUM_NEG_P384_SUBROUTINE_CORRECT | BIGNUM_NEG_P384_CORRECT | 4872057 | 86799846 | 25.000 | 37.000 | refused: theorem identity |
| arm | p521/neg_p521 | BIGNUM_NEG_P521_SUBROUTINE_CORRECT | BIGNUM_NEG_P521_CORRECT | 5473461 | 123668935 | 26.000 | 44.000 | refused: theorem identity |
| arm | sm2/neg_sm2 | BIGNUM_NEG_SM2_SUBROUTINE_CORRECT | BIGNUM_NEG_SM2_CORRECT | 2716758 | 46665222 | 24.000 | 31.000 | refused: theorem identity |
| x86 | curve25519/add_p25519 | BIGNUM_ADD_P25519_SUBROUTINE_CORRECT | BIGNUM_ADD_P25519_CORRECT | 9417530 | 160584653 | 51.000 | 70.000 | refused: theorem identity |
| x86 | secp256k1/add_p256k1 | BIGNUM_ADD_P256K1_SUBROUTINE_CORRECT | BIGNUM_ADD_P256K1_CORRECT | 9687987 | 166694408 | 51.000 | 71.000 | refused: theorem identity |
| x86 | sm2/add_sm2 | BIGNUM_ADD_SM2_SUBROUTINE_CORRECT | BIGNUM_ADD_SM2_CORRECT | 11075199 | 127361354 | 51.000 | 66.000 | refused: theorem identity |
| x86 | curve25519/sub_p25519 | BIGNUM_SUB_P25519_SUBROUTINE_CORRECT | BIGNUM_SUB_P25519_CORRECT | 9235997 | 61298759 | 51.000 | 57.000 | refused: theorem identity |
| x86 | secp256k1/sub_p256k1 | BIGNUM_SUB_P256K1_SUBROUTINE_CORRECT | BIGNUM_SUB_P256K1_CORRECT | 9096392 | 111312547 | 51.000 | 64.000 | refused: theorem identity |
| x86 | sm2/sub_sm2 | BIGNUM_SUB_SM2_SUBROUTINE_CORRECT | BIGNUM_SUB_SM2_CORRECT | 9597139 | 67342905 | 51.000 | 57.000 | refused: theorem identity |
| x86 | curve25519/neg_p25519 | BIGNUM_NEG_P25519_SUBROUTINE_CORRECT | BIGNUM_NEG_P25519_CORRECT | 17589216 | 19973249 | 54.000 | 51.000 | refused: theorem identity |
| x86 | secp256k1/neg_p256k1 | BIGNUM_NEG_P256K1_SUBROUTINE_CORRECT | BIGNUM_NEG_P256K1_CORRECT | 8632786 | 23422025 | 50.000 | 52.000 | refused: theorem identity |
| x86 | sm2/neg_sm2 | BIGNUM_NEG_SM2_SUBROUTINE_CORRECT | BIGNUM_NEG_SM2_CORRECT | 8761732 | 18230539 | 51.000 | 51.000 | refused: theorem identity |

### Counts

| quantity | value |
|---|---:|
| human rows parsed | 27 |
| generated rows parsed | 27 |
| task rows (union of keys) | 27 |
| matched pairs (n) | 0 |
| unmatched rows | 27 |

### Matched totals (denominator: n matched pairs)

| metric | value |
|---|---:|
| matched pairs (n) | 0 |
| human inferences (n=0) | n/a (no matched pairs) |
| generated inferences (n=0) | n/a (no matched pairs) |
| aggregate inference ratio | n/a (n=0; not published) |
| per-task inference ratio median | n/a (n=0) |
| human seconds (n=0) | n/a (no matched pairs) |
| generated seconds (n=0) | n/a (no matched pairs) |
| aggregate seconds ratio | n/a (n=0; not published) |

### Unmatched rows (excluded from every aggregate)

| arch | task | human theorem | generated theorem | human inf | generated inf | reason |
|---|---|---|---|---:|---:|---|
| arm | curve25519/add_p25519 | BIGNUM_ADD_P25519_SUBROUTINE_CORRECT | BIGNUM_ADD_P25519_CORRECT | 3848039 | 170225275 | theorem identity differs: human "BIGNUM_ADD_P25519_SUBROUTINE_CORRECT", generated "BIGNUM_ADD_P25519_CORRECT" |
| arm | p256/add_p256 | BIGNUM_ADD_P256_SUBROUTINE_CORRECT | BIGNUM_ADD_P256_CORRECT | 3872266 | 170931670 | theorem identity differs: human "BIGNUM_ADD_P256_SUBROUTINE_CORRECT", generated "BIGNUM_ADD_P256_CORRECT" |
| arm | secp256k1/add_p256k1 | BIGNUM_ADD_P256K1_SUBROUTINE_CORRECT | BIGNUM_ADD_P256K1_CORRECT | 3672894 | 157240305 | theorem identity differs: human "BIGNUM_ADD_P256K1_SUBROUTINE_CORRECT", generated "BIGNUM_ADD_P256K1_CORRECT" |
| arm | p384/add_p384 | BIGNUM_ADD_P384_SUBROUTINE_CORRECT | BIGNUM_ADD_P384_CORRECT | 8059478 | 349139623 | theorem identity differs: human "BIGNUM_ADD_P384_SUBROUTINE_CORRECT", generated "BIGNUM_ADD_P384_CORRECT" |
| arm | p521/add_p521 | BIGNUM_ADD_P521_SUBROUTINE_CORRECT | BIGNUM_ADD_P521_CORRECT | 8835293 | 681707202 | theorem identity differs: human "BIGNUM_ADD_P521_SUBROUTINE_CORRECT", generated "BIGNUM_ADD_P521_CORRECT" |
| arm | sm2/add_sm2 | BIGNUM_ADD_SM2_SUBROUTINE_CORRECT | BIGNUM_ADD_SM2_CORRECT | 3856358 | 159985379 | theorem identity differs: human "BIGNUM_ADD_SM2_SUBROUTINE_CORRECT", generated "BIGNUM_ADD_SM2_CORRECT" |
| arm | curve25519/sub_p25519 | BIGNUM_SUB_P25519_SUBROUTINE_CORRECT | BIGNUM_SUB_P25519_CORRECT | 3261274 | 110957174 | theorem identity differs: human "BIGNUM_SUB_P25519_SUBROUTINE_CORRECT", generated "BIGNUM_SUB_P25519_CORRECT" |
| arm | p256/sub_p256 | BIGNUM_SUB_P256_SUBROUTINE_CORRECT | BIGNUM_SUB_P256_CORRECT | 3395549 | 103417925 | theorem identity differs: human "BIGNUM_SUB_P256_SUBROUTINE_CORRECT", generated "BIGNUM_SUB_P256_CORRECT" |
| arm | secp256k1/sub_p256k1 | BIGNUM_SUB_P256K1_SUBROUTINE_CORRECT | BIGNUM_SUB_P256K1_CORRECT | 3296935 | 110121226 | theorem identity differs: human "BIGNUM_SUB_P256K1_SUBROUTINE_CORRECT", generated "BIGNUM_SUB_P256K1_CORRECT" |
| arm | p384/sub_p384 | BIGNUM_SUB_P384_SUBROUTINE_CORRECT | BIGNUM_SUB_P384_CORRECT | 6475221 | 214410496 | theorem identity differs: human "BIGNUM_SUB_P384_SUBROUTINE_CORRECT", generated "BIGNUM_SUB_P384_CORRECT" |
| arm | p521/sub_p521 | BIGNUM_SUB_P521_SUBROUTINE_CORRECT | BIGNUM_SUB_P521_CORRECT | 8753292 | 221845891 | theorem identity differs: human "BIGNUM_SUB_P521_SUBROUTINE_CORRECT", generated "BIGNUM_SUB_P521_CORRECT" |
| arm | sm2/sub_sm2 | BIGNUM_SUB_SM2_SUBROUTINE_CORRECT | BIGNUM_SUB_SM2_CORRECT | 3391866 | 107504057 | theorem identity differs: human "BIGNUM_SUB_SM2_SUBROUTINE_CORRECT", generated "BIGNUM_SUB_SM2_CORRECT" |
| arm | curve25519/neg_p25519 | BIGNUM_NEG_P25519_SUBROUTINE_CORRECT | BIGNUM_NEG_P25519_CORRECT | 4077908 | 35457948 | theorem identity differs: human "BIGNUM_NEG_P25519_SUBROUTINE_CORRECT", generated "BIGNUM_NEG_P25519_CORRECT" |
| arm | p256/neg_p256 | BIGNUM_NEG_P256_SUBROUTINE_CORRECT | BIGNUM_NEG_P256_CORRECT | 2712659 | 45739252 | theorem identity differs: human "BIGNUM_NEG_P256_SUBROUTINE_CORRECT", generated "BIGNUM_NEG_P256_CORRECT" |
| arm | secp256k1/neg_p256k1 | BIGNUM_NEG_P256K1_SUBROUTINE_CORRECT | BIGNUM_NEG_P256K1_CORRECT | 2704476 | 45424790 | theorem identity differs: human "BIGNUM_NEG_P256K1_SUBROUTINE_CORRECT", generated "BIGNUM_NEG_P256K1_CORRECT" |
| arm | p384/neg_p384 | BIGNUM_NEG_P384_SUBROUTINE_CORRECT | BIGNUM_NEG_P384_CORRECT | 4872057 | 86799846 | theorem identity differs: human "BIGNUM_NEG_P384_SUBROUTINE_CORRECT", generated "BIGNUM_NEG_P384_CORRECT" |
| arm | p521/neg_p521 | BIGNUM_NEG_P521_SUBROUTINE_CORRECT | BIGNUM_NEG_P521_CORRECT | 5473461 | 123668935 | theorem identity differs: human "BIGNUM_NEG_P521_SUBROUTINE_CORRECT", generated "BIGNUM_NEG_P521_CORRECT" |
| arm | sm2/neg_sm2 | BIGNUM_NEG_SM2_SUBROUTINE_CORRECT | BIGNUM_NEG_SM2_CORRECT | 2716758 | 46665222 | theorem identity differs: human "BIGNUM_NEG_SM2_SUBROUTINE_CORRECT", generated "BIGNUM_NEG_SM2_CORRECT" |
| x86 | curve25519/add_p25519 | BIGNUM_ADD_P25519_SUBROUTINE_CORRECT | BIGNUM_ADD_P25519_CORRECT | 9417530 | 160584653 | theorem identity differs: human "BIGNUM_ADD_P25519_SUBROUTINE_CORRECT", generated "BIGNUM_ADD_P25519_CORRECT" |
| x86 | secp256k1/add_p256k1 | BIGNUM_ADD_P256K1_SUBROUTINE_CORRECT | BIGNUM_ADD_P256K1_CORRECT | 9687987 | 166694408 | theorem identity differs: human "BIGNUM_ADD_P256K1_SUBROUTINE_CORRECT", generated "BIGNUM_ADD_P256K1_CORRECT" |
| x86 | sm2/add_sm2 | BIGNUM_ADD_SM2_SUBROUTINE_CORRECT | BIGNUM_ADD_SM2_CORRECT | 11075199 | 127361354 | theorem identity differs: human "BIGNUM_ADD_SM2_SUBROUTINE_CORRECT", generated "BIGNUM_ADD_SM2_CORRECT" |
| x86 | curve25519/sub_p25519 | BIGNUM_SUB_P25519_SUBROUTINE_CORRECT | BIGNUM_SUB_P25519_CORRECT | 9235997 | 61298759 | theorem identity differs: human "BIGNUM_SUB_P25519_SUBROUTINE_CORRECT", generated "BIGNUM_SUB_P25519_CORRECT" |
| x86 | secp256k1/sub_p256k1 | BIGNUM_SUB_P256K1_SUBROUTINE_CORRECT | BIGNUM_SUB_P256K1_CORRECT | 9096392 | 111312547 | theorem identity differs: human "BIGNUM_SUB_P256K1_SUBROUTINE_CORRECT", generated "BIGNUM_SUB_P256K1_CORRECT" |
| x86 | sm2/sub_sm2 | BIGNUM_SUB_SM2_SUBROUTINE_CORRECT | BIGNUM_SUB_SM2_CORRECT | 9597139 | 67342905 | theorem identity differs: human "BIGNUM_SUB_SM2_SUBROUTINE_CORRECT", generated "BIGNUM_SUB_SM2_CORRECT" |
| x86 | curve25519/neg_p25519 | BIGNUM_NEG_P25519_SUBROUTINE_CORRECT | BIGNUM_NEG_P25519_CORRECT | 17589216 | 19973249 | theorem identity differs: human "BIGNUM_NEG_P25519_SUBROUTINE_CORRECT", generated "BIGNUM_NEG_P25519_CORRECT" |
| x86 | secp256k1/neg_p256k1 | BIGNUM_NEG_P256K1_SUBROUTINE_CORRECT | BIGNUM_NEG_P256K1_CORRECT | 8632786 | 23422025 | theorem identity differs: human "BIGNUM_NEG_P256K1_SUBROUTINE_CORRECT", generated "BIGNUM_NEG_P256K1_CORRECT" |
| x86 | sm2/neg_sm2 | BIGNUM_NEG_SM2_SUBROUTINE_CORRECT | BIGNUM_NEG_SM2_CORRECT | 8761732 | 18230539 | theorem identity differs: human "BIGNUM_NEG_SM2_SUBROUTINE_CORRECT", generated "BIGNUM_NEG_SM2_CORRECT" |

### Descriptive side totals (all parsed rows; NOT an aggregate over matched pairs)

| side | rows | rows with inferences | total inferences |
|---|---:|---:|---:|
| human | 27 | 27 | 176369762 |
| generated | 27 | 27 | 3697462655 |

MATCHED 0/27 PAIRS; UNMATCHED 27; HUMAN ROWS 27; GENERATED ROWS 27

## 3. The five costs that must never be merged (XIV.B4)

| cost | measured today? | what the number is |
|---|---|---|
| search time | **not measured** | `seconds` is one `Running time:` value over `loadt` + `check_axioms`; there is no search/replay split. |
| successful derivation size | **not measured separately** | one primitive-inference counter; it increments on every primitive rule application, including work later discarded. |
| discarded inference work | **not measured separately** | included in the same counter. |
| solver-free replay time | **not measured** | no replay-only mode; `check_axioms ()` checks the axiom stamp, not a replayed derivation. |
| total construction + verification cost | **partially measured** | whole-file wall clock (`Running time`) and primitive-inference work (counts). |

`XIV.B3` remains open: the counter **mixes** successful derivation with discarded
search, so the inference counts are **work**, not proof-tree size. Nothing in this
receipt splits them.

## 4. Wall-clock table (partial, 25 rows)

This section uses `research/receipts/wip/ix58-checktime.partial.json` (human, 25
rows: 18 ARM + 7 x86; x86 `secp256k1/neg` and `sm2/neg` are absent) and
`research/receipts/ix56-upstream-coverage.json` (generated, 27 rows). It is
**partial** and is **not superseded** as the 25-row wall-clock table. (The ix60
human run also recorded `Running time` for all 27; those values are the `human s`
column of Section 2. They are a separate, stamped run, not a replacement for this
partial table.) With the fixed machinery every row is refused as a theorem-identity
mismatch, so the table is per-task provenance, not a matched aggregate.

Provenance:
- human: `generated_at 1795523792`, tree `.bench/human-baseline`, s2n pin
  `5e0fc782`, HOL pin `d30e823`, `cores: 1`, `force: true`, `stamp: false`,
  `timeout_seconds: 900`, 25 tasks;
- generated: `generated_at 1795513397`, s2n pin `5e0fc782`, HOL pin `d30e823`,
  `cores: 1`, `force: true`, `stamp: false`, `timeout_seconds: 900`, 27 tasks;
- `seconds` is the last `Running time: <x> sec, Start unixtime: ...` line of the
  task's `.correct` file, printed by upstream's `build-proof.sh` around
  `loadt <proof>` plus `check_axioms ()`; it excludes the native compile but
  includes HOL Light loading/base.

```
$ target/debug/dropin checktime-table \
    --human research/receipts/wip/ix58-checktime.partial.json \
    --generated research/receipts/ix56-upstream-coverage.json
MATCHED 0/27 PAIRS; UNMATCHED 27; HUMAN ROWS 25; GENERATED ROWS 27
```


A row contributes to an aggregate only when `compare` returns a
`MatchedComparison`: exact theorem identity, both judgments successful,
and identical measurement scopes. `human s` and `generated s` are the
`Running time:` value parsed from each task's `.correct` file.

| arch | task | human theorem | generated theorem | human s | generated s | verdict |
|---|---|---|---|---:|---:|---|
| arm | curve25519/add_p25519 | BIGNUM_ADD_P25519_SUBROUTINE_CORRECT | BIGNUM_ADD_P25519_CORRECT | 24.000 | 47.000 | refused: theorem identity |
| arm | p256/add_p256 | BIGNUM_ADD_P256_SUBROUTINE_CORRECT | BIGNUM_ADD_P256_CORRECT | 25.000 | 47.000 | refused: theorem identity |
| arm | secp256k1/add_p256k1 | BIGNUM_ADD_P256K1_SUBROUTINE_CORRECT | BIGNUM_ADD_P256K1_CORRECT | 24.000 | 47.000 | refused: theorem identity |
| arm | p384/add_p384 | BIGNUM_ADD_P384_SUBROUTINE_CORRECT | BIGNUM_ADD_P384_CORRECT | 26.000 | 75.000 | refused: theorem identity |
| arm | p521/add_p521 | BIGNUM_ADD_P521_SUBROUTINE_CORRECT | BIGNUM_ADD_P521_CORRECT | 26.000 | 122.000 | refused: theorem identity |
| arm | sm2/add_sm2 | BIGNUM_ADD_SM2_SUBROUTINE_CORRECT | BIGNUM_ADD_SM2_CORRECT | 24.000 | 46.000 | refused: theorem identity |
| arm | curve25519/sub_p25519 | BIGNUM_SUB_P25519_SUBROUTINE_CORRECT | BIGNUM_SUB_P25519_CORRECT | 24.000 | 40.000 | refused: theorem identity |
| arm | p256/sub_p256 | BIGNUM_SUB_P256_SUBROUTINE_CORRECT | BIGNUM_SUB_P256_CORRECT | 24.000 | 39.000 | refused: theorem identity |
| arm | secp256k1/sub_p256k1 | BIGNUM_SUB_P256K1_SUBROUTINE_CORRECT | BIGNUM_SUB_P256K1_CORRECT | 24.000 | 40.000 | refused: theorem identity |
| arm | p384/sub_p384 | BIGNUM_SUB_P384_SUBROUTINE_CORRECT | BIGNUM_SUB_P384_CORRECT | 26.000 | 56.000 | refused: theorem identity |
| arm | p521/sub_p521 | BIGNUM_SUB_P521_SUBROUTINE_CORRECT | BIGNUM_SUB_P521_CORRECT | 26.000 | 60.000 | refused: theorem identity |
| arm | sm2/sub_sm2 | BIGNUM_SUB_SM2_SUBROUTINE_CORRECT | BIGNUM_SUB_SM2_CORRECT | 24.000 | 40.000 | refused: theorem identity |
| arm | curve25519/neg_p25519 | BIGNUM_NEG_P25519_SUBROUTINE_CORRECT | BIGNUM_NEG_P25519_CORRECT | 25.000 | 30.000 | refused: theorem identity |
| arm | p256/neg_p256 | BIGNUM_NEG_P256_SUBROUTINE_CORRECT | BIGNUM_NEG_P256_CORRECT | 24.000 | 30.000 | refused: theorem identity |
| arm | secp256k1/neg_p256k1 | BIGNUM_NEG_P256K1_SUBROUTINE_CORRECT | BIGNUM_NEG_P256K1_CORRECT | 25.000 | 30.000 | refused: theorem identity |
| arm | p384/neg_p384 | BIGNUM_NEG_P384_SUBROUTINE_CORRECT | BIGNUM_NEG_P384_CORRECT | 25.000 | 37.000 | refused: theorem identity |
| arm | p521/neg_p521 | BIGNUM_NEG_P521_SUBROUTINE_CORRECT | BIGNUM_NEG_P521_CORRECT | 26.000 | 44.000 | refused: theorem identity |
| arm | sm2/neg_sm2 | BIGNUM_NEG_SM2_SUBROUTINE_CORRECT | BIGNUM_NEG_SM2_CORRECT | 24.000 | 31.000 | refused: theorem identity |
| x86 | curve25519/add_p25519 | BIGNUM_ADD_P25519_SUBROUTINE_CORRECT | BIGNUM_ADD_P25519_CORRECT | 51.000 | 70.000 | refused: theorem identity |
| x86 | secp256k1/add_p256k1 | BIGNUM_ADD_P256K1_SUBROUTINE_CORRECT | BIGNUM_ADD_P256K1_CORRECT | 50.000 | 71.000 | refused: theorem identity |
| x86 | sm2/add_sm2 | BIGNUM_ADD_SM2_SUBROUTINE_CORRECT | BIGNUM_ADD_SM2_CORRECT | 51.000 | 66.000 | refused: theorem identity |
| x86 | curve25519/sub_p25519 | BIGNUM_SUB_P25519_SUBROUTINE_CORRECT | BIGNUM_SUB_P25519_CORRECT | 51.000 | 57.000 | refused: theorem identity |
| x86 | secp256k1/sub_p256k1 | BIGNUM_SUB_P256K1_SUBROUTINE_CORRECT | BIGNUM_SUB_P256K1_CORRECT | 50.000 | 64.000 | refused: theorem identity |
| x86 | sm2/sub_sm2 | BIGNUM_SUB_SM2_SUBROUTINE_CORRECT | BIGNUM_SUB_SM2_CORRECT | 51.000 | 57.000 | refused: theorem identity |
| x86 | curve25519/neg_p25519 | BIGNUM_NEG_P25519_SUBROUTINE_CORRECT | BIGNUM_NEG_P25519_CORRECT | 53.000 | 51.000 | refused: theorem identity |
| x86 | secp256k1/neg_p256k1 | n/a | BIGNUM_NEG_P256K1_CORRECT | n/a | 51.000 | no human row |
| x86 | sm2/neg_sm2 | n/a | BIGNUM_NEG_SM2_CORRECT | n/a | 50.000 | no human row |

### Measurement counts (XIV.A3/A4)

| quantity | value |
|---|---:|
| human rows parsed | 25 |
| generated rows parsed | 27 |
| task rows (union of keys) | 27 |
| rows with both sides present | 25 |
| matched pairs (n) | 0 |
| unmatched rows | 27 |

### Matched aggregates (denominator: n matched pairs)

| metric | value |
|---|---:|
| matched pairs (n) | 0 |
| human total s (n=0) | n/a (no matched pairs) |
| generated total s (n=0) | n/a (no matched pairs) |
| aggregate seconds ratio | n/a (n=0; not published) |
| per-task seconds ratio median | n/a (n=0) |
| human total inferences (n=0) | n/a (no matched pairs) |
| generated total inferences (n=0) | n/a (no matched pairs) |
| aggregate inference ratio | n/a (n=0; not published) |
| per-task inference ratio median | n/a (n=0) |

### Unmatched rows (excluded from every aggregate)

| arch | task | human theorem | generated theorem | human s | generated s | reason |
|---|---|---|---|---:|---:|---|
| arm | curve25519/add_p25519 | BIGNUM_ADD_P25519_SUBROUTINE_CORRECT | BIGNUM_ADD_P25519_CORRECT | 24.000 | 47.000 | theorem identity differs: human "BIGNUM_ADD_P25519_SUBROUTINE_CORRECT", generated "BIGNUM_ADD_P25519_CORRECT" |
| arm | p256/add_p256 | BIGNUM_ADD_P256_SUBROUTINE_CORRECT | BIGNUM_ADD_P256_CORRECT | 25.000 | 47.000 | theorem identity differs: human "BIGNUM_ADD_P256_SUBROUTINE_CORRECT", generated "BIGNUM_ADD_P256_CORRECT" |
| arm | secp256k1/add_p256k1 | BIGNUM_ADD_P256K1_SUBROUTINE_CORRECT | BIGNUM_ADD_P256K1_CORRECT | 24.000 | 47.000 | theorem identity differs: human "BIGNUM_ADD_P256K1_SUBROUTINE_CORRECT", generated "BIGNUM_ADD_P256K1_CORRECT" |
| arm | p384/add_p384 | BIGNUM_ADD_P384_SUBROUTINE_CORRECT | BIGNUM_ADD_P384_CORRECT | 26.000 | 75.000 | theorem identity differs: human "BIGNUM_ADD_P384_SUBROUTINE_CORRECT", generated "BIGNUM_ADD_P384_CORRECT" |
| arm | p521/add_p521 | BIGNUM_ADD_P521_SUBROUTINE_CORRECT | BIGNUM_ADD_P521_CORRECT | 26.000 | 122.000 | theorem identity differs: human "BIGNUM_ADD_P521_SUBROUTINE_CORRECT", generated "BIGNUM_ADD_P521_CORRECT" |
| arm | sm2/add_sm2 | BIGNUM_ADD_SM2_SUBROUTINE_CORRECT | BIGNUM_ADD_SM2_CORRECT | 24.000 | 46.000 | theorem identity differs: human "BIGNUM_ADD_SM2_SUBROUTINE_CORRECT", generated "BIGNUM_ADD_SM2_CORRECT" |
| arm | curve25519/sub_p25519 | BIGNUM_SUB_P25519_SUBROUTINE_CORRECT | BIGNUM_SUB_P25519_CORRECT | 24.000 | 40.000 | theorem identity differs: human "BIGNUM_SUB_P25519_SUBROUTINE_CORRECT", generated "BIGNUM_SUB_P25519_CORRECT" |
| arm | p256/sub_p256 | BIGNUM_SUB_P256_SUBROUTINE_CORRECT | BIGNUM_SUB_P256_CORRECT | 24.000 | 39.000 | theorem identity differs: human "BIGNUM_SUB_P256_SUBROUTINE_CORRECT", generated "BIGNUM_SUB_P256_CORRECT" |
| arm | secp256k1/sub_p256k1 | BIGNUM_SUB_P256K1_SUBROUTINE_CORRECT | BIGNUM_SUB_P256K1_CORRECT | 24.000 | 40.000 | theorem identity differs: human "BIGNUM_SUB_P256K1_SUBROUTINE_CORRECT", generated "BIGNUM_SUB_P256K1_CORRECT" |
| arm | p384/sub_p384 | BIGNUM_SUB_P384_SUBROUTINE_CORRECT | BIGNUM_SUB_P384_CORRECT | 26.000 | 56.000 | theorem identity differs: human "BIGNUM_SUB_P384_SUBROUTINE_CORRECT", generated "BIGNUM_SUB_P384_CORRECT" |
| arm | p521/sub_p521 | BIGNUM_SUB_P521_SUBROUTINE_CORRECT | BIGNUM_SUB_P521_CORRECT | 26.000 | 60.000 | theorem identity differs: human "BIGNUM_SUB_P521_SUBROUTINE_CORRECT", generated "BIGNUM_SUB_P521_CORRECT" |
| arm | sm2/sub_sm2 | BIGNUM_SUB_SM2_SUBROUTINE_CORRECT | BIGNUM_SUB_SM2_CORRECT | 24.000 | 40.000 | theorem identity differs: human "BIGNUM_SUB_SM2_SUBROUTINE_CORRECT", generated "BIGNUM_SUB_SM2_CORRECT" |
| arm | curve25519/neg_p25519 | BIGNUM_NEG_P25519_SUBROUTINE_CORRECT | BIGNUM_NEG_P25519_CORRECT | 25.000 | 30.000 | theorem identity differs: human "BIGNUM_NEG_P25519_SUBROUTINE_CORRECT", generated "BIGNUM_NEG_P25519_CORRECT" |
| arm | p256/neg_p256 | BIGNUM_NEG_P256_SUBROUTINE_CORRECT | BIGNUM_NEG_P256_CORRECT | 24.000 | 30.000 | theorem identity differs: human "BIGNUM_NEG_P256_SUBROUTINE_CORRECT", generated "BIGNUM_NEG_P256_CORRECT" |
| arm | secp256k1/neg_p256k1 | BIGNUM_NEG_P256K1_SUBROUTINE_CORRECT | BIGNUM_NEG_P256K1_CORRECT | 25.000 | 30.000 | theorem identity differs: human "BIGNUM_NEG_P256K1_SUBROUTINE_CORRECT", generated "BIGNUM_NEG_P256K1_CORRECT" |
| arm | p384/neg_p384 | BIGNUM_NEG_P384_SUBROUTINE_CORRECT | BIGNUM_NEG_P384_CORRECT | 25.000 | 37.000 | theorem identity differs: human "BIGNUM_NEG_P384_SUBROUTINE_CORRECT", generated "BIGNUM_NEG_P384_CORRECT" |
| arm | p521/neg_p521 | BIGNUM_NEG_P521_SUBROUTINE_CORRECT | BIGNUM_NEG_P521_CORRECT | 26.000 | 44.000 | theorem identity differs: human "BIGNUM_NEG_P521_SUBROUTINE_CORRECT", generated "BIGNUM_NEG_P521_CORRECT" |
| arm | sm2/neg_sm2 | BIGNUM_NEG_SM2_SUBROUTINE_CORRECT | BIGNUM_NEG_SM2_CORRECT | 24.000 | 31.000 | theorem identity differs: human "BIGNUM_NEG_SM2_SUBROUTINE_CORRECT", generated "BIGNUM_NEG_SM2_CORRECT" |
| x86 | curve25519/add_p25519 | BIGNUM_ADD_P25519_SUBROUTINE_CORRECT | BIGNUM_ADD_P25519_CORRECT | 51.000 | 70.000 | theorem identity differs: human "BIGNUM_ADD_P25519_SUBROUTINE_CORRECT", generated "BIGNUM_ADD_P25519_CORRECT" |
| x86 | secp256k1/add_p256k1 | BIGNUM_ADD_P256K1_SUBROUTINE_CORRECT | BIGNUM_ADD_P256K1_CORRECT | 50.000 | 71.000 | theorem identity differs: human "BIGNUM_ADD_P256K1_SUBROUTINE_CORRECT", generated "BIGNUM_ADD_P256K1_CORRECT" |
| x86 | sm2/add_sm2 | BIGNUM_ADD_SM2_SUBROUTINE_CORRECT | BIGNUM_ADD_SM2_CORRECT | 51.000 | 66.000 | theorem identity differs: human "BIGNUM_ADD_SM2_SUBROUTINE_CORRECT", generated "BIGNUM_ADD_SM2_CORRECT" |
| x86 | curve25519/sub_p25519 | BIGNUM_SUB_P25519_SUBROUTINE_CORRECT | BIGNUM_SUB_P25519_CORRECT | 51.000 | 57.000 | theorem identity differs: human "BIGNUM_SUB_P25519_SUBROUTINE_CORRECT", generated "BIGNUM_SUB_P25519_CORRECT" |
| x86 | secp256k1/sub_p256k1 | BIGNUM_SUB_P256K1_SUBROUTINE_CORRECT | BIGNUM_SUB_P256K1_CORRECT | 50.000 | 64.000 | theorem identity differs: human "BIGNUM_SUB_P256K1_SUBROUTINE_CORRECT", generated "BIGNUM_SUB_P256K1_CORRECT" |
| x86 | sm2/sub_sm2 | BIGNUM_SUB_SM2_SUBROUTINE_CORRECT | BIGNUM_SUB_SM2_CORRECT | 51.000 | 57.000 | theorem identity differs: human "BIGNUM_SUB_SM2_SUBROUTINE_CORRECT", generated "BIGNUM_SUB_SM2_CORRECT" |
| x86 | curve25519/neg_p25519 | BIGNUM_NEG_P25519_SUBROUTINE_CORRECT | BIGNUM_NEG_P25519_CORRECT | 53.000 | 51.000 | theorem identity differs: human "BIGNUM_NEG_P25519_SUBROUTINE_CORRECT", generated "BIGNUM_NEG_P25519_CORRECT" |
| x86 | secp256k1/neg_p256k1 | n/a | BIGNUM_NEG_P256K1_CORRECT | n/a | 51.000 | no human measurement |
| x86 | sm2/neg_sm2 | n/a | BIGNUM_NEG_SM2_CORRECT | n/a | 50.000 | no human measurement |

### Descriptive side totals (all parsed rows; NOT an aggregate over matched pairs)

| side | rows | rows with the value | total |
|---|---:|---:|---:|
| human seconds | 25 | 25 | 803.000 s |
| generated seconds | 27 | 27 | 1398.000 s |
| human inferences | 25 | 0 | 0 |
| generated inferences | 27 | 0 | 0 |

### XII.F9.2 budget: 120 s (70% = 84 s)

No matched generated task to test (n=0).

### Running time provenance

`seconds` is the last `Running time: <x> sec, Start unixtime: ...` line of the
task's `.correct` file. Upstream's `build-proof.sh` prints it around
`loadt <proof>` plus `check_axioms ()`; it excludes HOL Light load and the
native compile.

| arch | task | human `.correct` | generated make log |
|---|---|---|---|
| arm | curve25519/add_p25519 | `/mnt/1tb_kingston/Tekmerion/.bench/human-baseline/arm/curve25519/bignum_add_p25519.correct` | `/mnt/1tb_kingston/Tekmerion/.bench/dropin-gen/logs-arm/curve25519_add_p25519.log` |
| arm | p256/add_p256 | `/mnt/1tb_kingston/Tekmerion/.bench/human-baseline/arm/p256/bignum_add_p256.correct` | `/mnt/1tb_kingston/Tekmerion/.bench/dropin-gen/logs-arm/p256_add_p256.log` |
| arm | secp256k1/add_p256k1 | `/mnt/1tb_kingston/Tekmerion/.bench/human-baseline/arm/secp256k1/bignum_add_p256k1.correct` | `/mnt/1tb_kingston/Tekmerion/.bench/dropin-gen/logs-arm/secp256k1_add_p256k1.log` |
| arm | p384/add_p384 | `/mnt/1tb_kingston/Tekmerion/.bench/human-baseline/arm/p384/bignum_add_p384.correct` | `/mnt/1tb_kingston/Tekmerion/.bench/dropin-gen/logs-arm/p384_add_p384.log` |
| arm | p521/add_p521 | `/mnt/1tb_kingston/Tekmerion/.bench/human-baseline/arm/p521/bignum_add_p521.correct` | `/mnt/1tb_kingston/Tekmerion/.bench/dropin-gen/logs-arm/p521_add_p521.log` |
| arm | sm2/add_sm2 | `/mnt/1tb_kingston/Tekmerion/.bench/human-baseline/arm/sm2/bignum_add_sm2.correct` | `/mnt/1tb_kingston/Tekmerion/.bench/dropin-gen/logs-arm/sm2_add_sm2.log` |
| arm | curve25519/sub_p25519 | `/mnt/1tb_kingston/Tekmerion/.bench/human-baseline/arm/curve25519/bignum_sub_p25519.correct` | `/mnt/1tb_kingston/Tekmerion/.bench/dropin-gen/logs-arm/curve25519_sub_p25519.log` |
| arm | p256/sub_p256 | `/mnt/1tb_kingston/Tekmerion/.bench/human-baseline/arm/p256/bignum_sub_p256.correct` | `/mnt/1tb_kingston/Tekmerion/.bench/dropin-gen/logs-arm/p256_sub_p256.log` |
| arm | secp256k1/sub_p256k1 | `/mnt/1tb_kingston/Tekmerion/.bench/human-baseline/arm/secp256k1/bignum_sub_p256k1.correct` | `/mnt/1tb_kingston/Tekmerion/.bench/dropin-gen/logs-arm/secp256k1_sub_p256k1.log` |
| arm | p384/sub_p384 | `/mnt/1tb_kingston/Tekmerion/.bench/human-baseline/arm/p384/bignum_sub_p384.correct` | `/mnt/1tb_kingston/Tekmerion/.bench/dropin-gen/logs-arm/p384_sub_p384.log` |
| arm | p521/sub_p521 | `/mnt/1tb_kingston/Tekmerion/.bench/human-baseline/arm/p521/bignum_sub_p521.correct` | `/mnt/1tb_kingston/Tekmerion/.bench/dropin-gen/logs-arm/p521_sub_p521.log` |
| arm | sm2/sub_sm2 | `/mnt/1tb_kingston/Tekmerion/.bench/human-baseline/arm/sm2/bignum_sub_sm2.correct` | `/mnt/1tb_kingston/Tekmerion/.bench/dropin-gen/logs-arm/sm2_sub_sm2.log` |
| arm | curve25519/neg_p25519 | `/mnt/1tb_kingston/Tekmerion/.bench/human-baseline/arm/curve25519/bignum_neg_p25519.correct` | `/mnt/1tb_kingston/Tekmerion/.bench/dropin-gen/logs-arm/curve25519_neg_p25519.log` |
| arm | p256/neg_p256 | `/mnt/1tb_kingston/Tekmerion/.bench/human-baseline/arm/p256/bignum_neg_p256.correct` | `/mnt/1tb_kingston/Tekmerion/.bench/dropin-gen/logs-arm/p256_neg_p256.log` |
| arm | secp256k1/neg_p256k1 | `/mnt/1tb_kingston/Tekmerion/.bench/human-baseline/arm/secp256k1/bignum_neg_p256k1.correct` | `/mnt/1tb_kingston/Tekmerion/.bench/dropin-gen/logs-arm/secp256k1_neg_p256k1.log` |
| arm | p384/neg_p384 | `/mnt/1tb_kingston/Tekmerion/.bench/human-baseline/arm/p384/bignum_neg_p384.correct` | `/mnt/1tb_kingston/Tekmerion/.bench/dropin-gen/logs-arm/p384_neg_p384.log` |
| arm | p521/neg_p521 | `/mnt/1tb_kingston/Tekmerion/.bench/human-baseline/arm/p521/bignum_neg_p521.correct` | `/mnt/1tb_kingston/Tekmerion/.bench/dropin-gen/logs-arm/p521_neg_p521.log` |
| arm | sm2/neg_sm2 | `/mnt/1tb_kingston/Tekmerion/.bench/human-baseline/arm/sm2/bignum_neg_sm2.correct` | `/mnt/1tb_kingston/Tekmerion/.bench/dropin-gen/logs-arm/sm2_neg_sm2.log` |
| x86 | curve25519/add_p25519 | `/mnt/1tb_kingston/Tekmerion/.bench/human-baseline/x86/curve25519/bignum_add_p25519.correct` | `/mnt/1tb_kingston/Tekmerion/.bench/dropin-gen/logs-x86/curve25519_add_p25519.log` |
| x86 | secp256k1/add_p256k1 | `/mnt/1tb_kingston/Tekmerion/.bench/human-baseline/x86/secp256k1/bignum_add_p256k1.correct` | `/mnt/1tb_kingston/Tekmerion/.bench/dropin-gen/logs-x86/secp256k1_add_p256k1.log` |
| x86 | sm2/add_sm2 | `/mnt/1tb_kingston/Tekmerion/.bench/human-baseline/x86/sm2/bignum_add_sm2.correct` | `/mnt/1tb_kingston/Tekmerion/.bench/dropin-gen/logs-x86/sm2_add_sm2.log` |
| x86 | curve25519/sub_p25519 | `/mnt/1tb_kingston/Tekmerion/.bench/human-baseline/x86/curve25519/bignum_sub_p25519.correct` | `/mnt/1tb_kingston/Tekmerion/.bench/dropin-gen/logs-x86/curve25519_sub_p25519.log` |
| x86 | secp256k1/sub_p256k1 | `/mnt/1tb_kingston/Tekmerion/.bench/human-baseline/x86/secp256k1/bignum_sub_p256k1.correct` | `/mnt/1tb_kingston/Tekmerion/.bench/dropin-gen/logs-x86/secp256k1_sub_p256k1.log` |
| x86 | sm2/sub_sm2 | `/mnt/1tb_kingston/Tekmerion/.bench/human-baseline/x86/sm2/bignum_sub_sm2.correct` | `/mnt/1tb_kingston/Tekmerion/.bench/dropin-gen/logs-x86/sm2_sub_sm2.log` |
| x86 | curve25519/neg_p25519 | `/mnt/1tb_kingston/Tekmerion/.bench/human-baseline/x86/curve25519/bignum_neg_p25519.correct` | `/mnt/1tb_kingston/Tekmerion/.bench/dropin-gen/logs-x86/curve25519_neg_p25519.log` |
| x86 | secp256k1/neg_p256k1 | `/mnt/1tb_kingston/Tekmerion/.bench/human-baseline/x86/secp256k1/bignum_neg_p256k1.correct` | `/mnt/1tb_kingston/Tekmerion/.bench/dropin-gen/logs-x86/secp256k1_neg_p256k1.log` |
| x86 | sm2/neg_sm2 | `/mnt/1tb_kingston/Tekmerion/.bench/human-baseline/x86/sm2/bignum_neg_sm2.correct` | `/mnt/1tb_kingston/Tekmerion/.bench/dropin-gen/logs-x86/sm2_neg_sm2.log` |

MATCHED 0/27 PAIRS; UNMATCHED 27; HUMAN ROWS 25; GENERATED ROWS 27

## 5. What this does not establish

- **One host, one kernel build, one pin.** All numbers are from this host,
  `.bench/hol-light-stamp2` (patched `fusion.ml` sha256 `9c15e526…`), s2n-bignum
  `5e0fc782`, HOL Light `d30e823`, OCaml 5.4.0. Nothing here is a cross-host or
  cross-version stability claim.
- **The human proof is written for a different obligation.** Section 1 shows the
  statements are not the same; the human proof is a subroutine contract and ours
  is a whole-program contract. A ratio between them would not be a proof-cost
  comparison, which is why none is published.
- **`XIV.E3`.** The human `_SUBROUTINE_CORRECT` may include calling-convention and
  register-permission obligations that ours does not; the mechanical markers
  (return address, stack pointer, `MAYCHANGE_REGS_AND_FLAGS_PERMITTED_BY_ABI`)
  are evidence of that, not a complete semantic account of it.
- **Work is not derivation size.** `XIV.B3` is open; the counter mixes successful
  derivation with discarded search.
- **No replayable proof object.** The stamp is a dependency record, not a
  replayable derivation (IX.57b §7). `check_axioms` checks the stamp; it does not
  replay a proof object.
- **The generated counts are from IX.57b**, measured at the same pin with the same
  kernel build, but their mark placement (after decode) differs slightly from the
  human body mark (before decode). The human decode is included in its body delta
  and is not separately measured.

## 6. Reproduction

```
# code hygiene
cargo test --offline
cargo clippy --offline --all-targets -- -D warnings
cargo fmt --check

# obligation comparison (Part 2)
target/debug/dropin obligation \
  --human-tree .bench/human-baseline --generated-tree .bench/dropin \
  --json .bench/ix60-obligation.json

# human measurement (Part 3): 27 tasks, -j1, --force, instrumented, stamp kernel
env PATH=/mnt/1tb_kingston/Tekmerion/.bench/bin:/home/one/.local/bin:/usr/local/bin:/usr/bin:/bin \
  target/debug/dropin measure --tree .bench/human-baseline --arch all \
  --hol .bench/hol-light-stamp2 --force --instrument \
  --json .bench/measure-logs/ix60-human/human.json --label ix60-human

# matched inference table + JSON
target/debug/dropin matched-table \
  --human .bench/measure-logs/ix60-human/human.json \
  --generated research/receipts/ix57b-axiom-stamp.json \
  --human-kernel hol-light-stamp2 --generated-kernel hol-light-stamp2 \
  --json research/receipts/ix60-matched-inference.json

# wall-clock table (partial)
target/debug/dropin checktime-table \
  --human research/receipts/wip/ix58-checktime.partial.json \
  --generated research/receipts/ix56-upstream-coverage.json

git ls-files '*.py'
df -h /mnt/1tb_kingston | tail -1
```

The human run printed one line per task and finished `MEASURED 27/27`; the full
line-by-line output is `.bench/ix60-human-run.log`. The `.ml` files in
`.bench/human-baseline` were restored after each task (verified: zero files
contain `stamp_mark_`).

## 7. Artefacts and provenance

| artefact | role |
|---|---|
| `research/receipts/ix60-matched-inference.json` | tool-generated per-task counts, verdicts, totals |
| `.bench/measure-logs/ix60-human/human.json` | human measurement (working artefact; `.bench` is git-ignored) |
| `.bench/ix60-obligation.json` | obligation comparison (working artefact; the table and evidence are reproduced above) |
| `research/receipts/ix57b-axiom-stamp.json` | generated inference counts (input) |
| `research/receipts/ix56-upstream-coverage.json` | generated wall-clock times (input) |
| `research/receipts/wip/ix58-checktime.partial.json` | human wall-clock times, 25 rows (partial input; not a receipt) |

**Hygiene.** `git ls-files '*.py'` printed nothing; `cargo test --offline`,
`cargo clippy --offline --all-targets -- -D warnings` and `cargo fmt --check` all
pass; disk after the run is reported in the command output below.

```
$ git ls-files '*.py'
(no output)

$ df -h /mnt/1tb_kingston | tail -1
/dev/nvme1n1p1  1.3T  1.3T   11G 100% /mnt/1tb_kingston
```
