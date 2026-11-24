# IX.53 — the `--spent` guard mode (trusted-boundary decision)

Base commit: `3fedd8c` (branch `staging`; working tree modified).
Nothing was committed or pushed. Date: 2026-11-24.
Demonstration output: `research/receipts/ix53-spent-demo.log` (verbatim CLI
output on the hashes below).

## Decision

The 18 `arm.bignum_{add,sub,neg}_<family>` tasks in `r4-verdicts.csv` are
**spent development data**. `research/receipts/r4-reference-route.md` §0 records
that every one of them had a prior visible outcome and that none of the R4
numbers is held out. `--develop` is nevertheless right to refuse them: the
markers (`25519`, `secp256k1`/`p256k1`, `sm2`) cannot distinguish a spent id
from a held-out one, and the guard must fail closed.

Disabling the guard to re-answer two of them would be a trusted-boundary
regression. Instead this change adds a third explicit mode:

```
--spent --spent-manifest PATH
```

It is an **allow-list, not a bypass**:

| | `--develop` (default) | `--sealed-eval` | `--spent` |
|---|---|---|---|
| held-out task id | refused | allowed iff manifest-listed | allowed iff manifest-listed |
| held-out ELF object | refused | allowed iff its derived task id is listed and `--root` is given | same |
| held-out proof `.ml` | refused | **always refused** | **always refused** |
| non-held-out paths | unchanged | unchanged | unchanged |
| preconditions | none | manifest + tag + commit + clean `tools/` | manifest only |

The modes are mutually exclusive, `--spent` requires its manifest, and the
default remains `--develop`. No wildcard, no blanket override, and no change to
what `--develop` or `--sealed-eval` allow.

## Manifest

`research/receipts/spent-r4-arm.json`:

* `decision` — non-empty and cites `research/receipts/r4-reference-route.md`
  (the loader refuses a decision that does not contain
  `research/receipts/`).
* `spent` — a JSON object mapping all 18 R4 ARM ids to a non-empty reason. Each
  reason names the baseline verdict recorded for that id in
  `r4-verdicts.csv`, so the manifest records *why* every id is spent, not just
  that it is.
* `note` — provenance (generated from `r4-verdicts.csv`).

Duplicate keys, unknown keys, malformed/partial ids, a missing reason, an empty
`spent` object, an uncited decision, a symlink, an empty file, and invalid JSON
are all refusals. An id that is held out but absent from the manifest is still
refused, and the real manifest is ARM-only, so `x86.bignum_add_p25519...` is
refused under it.

## Evidence

CLI, verbatim (`ix53-spent-demo.log`):

```
$ python3 tools/heldout_guard.py .bench/s2n-bignum/arm/secp256k1/bignum_add_p256k1.o --spent --spent-manifest research/receipts/spent-r4-arm.json --root .bench/s2n-bignum
allowed: /mnt/1tb_kingston/Tekmerion/.bench/s2n-bignum/arm/secp256k1/bignum_add_p256k1.o sha256=ceb6df0d7aeba00e000cb285c582be7c1e1ef970fa035ebca654bcd4f9f598ce lines=2
exit=0

$ python3 tools/heldout_guard.py .bench/s2n-bignum/arm/proofs/bignum_add_p256k1.ml --spent --spent-manifest research/receipts/spent-r4-arm.json --root .bench/s2n-bignum
REFUSED: spent proof file is never readable: .bench/s2n-bignum/arm/proofs/bignum_add_p256k1.ml ('arm.bignum_add_p256k1.BIGNUM_ADD_P256K1_CORRECT')
exit=2

$ python3 tools/heldout_guard.py .bench/s2n-bignum/arm/secp256k1/bignum_add_p256k1.o   # default --develop
REFUSED: held-out marker 'secp256k1' in .bench/s2n-bignum/arm/secp256k1/bignum_add_p256k1.o
exit=2

$ python3 tools/heldout_guard.py .bench/s2n-bignum/x86/curve25519/bignum_add_p25519.o --spent --spent-manifest research/receipts/spent-r4-arm.json --root .bench/s2n-bignum   # not listed (ARM-only manifest)
REFUSED: object .bench/s2n-bignum/x86/curve25519/bignum_add_p25519.o belongs to 'x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT', which is not in the spent manifest
exit=2

$ python3 tools/heldout_guard.py --spent .bench/s2n-bignum/arm/p384/bignum_add_p384.o   # manifest is mandatory
REFUSED: --spent requires --spent-manifest PATH
exit=2

$ python3 tools/emit_ref_proof.py --spent --spent-manifest research/receipts/spent-r4-arm.json --task arm.bignum_add_p256k1.BIGNUM_ADD_P256K1_CORRECT --mode dev --out /tmp/x.ml   # proof read is still refused
refused: spent proof file is never readable: /mnt/1tb_kingston/Tekmerion/.bench/s2n-bignum/arm/proofs/bignum_add_p256k1.ml ('arm.bignum_add_p256k1.BIGNUM_ADD_P256K1_CORRECT')
exit=1
```

Tests (no network, no HOL):

```
$ python3 -m pytest tools/test_heldout_modes.py -q
23 passed in 0.08s
$ python3 -m pytest tools/test_derive_steps.py tools/test_x86_support.py tools/test_heldout_modes.py -q
54 passed, 1 skipped in 0.13s
```

The new tests are `test_spent_allows_only_listed_ids`,
`test_spent_allows_listed_object_and_refuses_unlisted`,
`test_spent_refuses_listed_proof_files`,
`test_spent_manifest_requires_decision_and_reasons`,
`test_spent_mode_is_explicit_and_develop_is_the_default`, and
`test_real_spent_manifest_lists_the_eighteen_r4_arm_ids` (which also checks the
manifest's id set equals the R4 verdict CSV's id set). The pre-existing
`--develop`/`--sealed-eval` tests are unchanged and still pass.

## Files and hashes

```
4f0998d1ba3d4d2e2f1c602b3967f06c65f3287622b0970683618f5288c84476  tools/heldout_guard.py
c85d218836745b291af5b43d2e540b3c7db8e5a162435d18f9e3cb644b280d42  tools/test_heldout_modes.py
47e2680bd2ee79862ead28617a41db3f5fdff9c70b28ded812a85c6f6297dbe4  research/receipts/spent-r4-arm.json
```

`tools/emit_ref_proof.py` received a docstring-only mention of the new mode; its
behaviour is unchanged and it picks the mode up through
`heldout_guard.add_mode_arguments`.

## Why this does not widen the trusted base

* The guard code path for `--develop` is byte-for-byte the same policy: the only
  refactor is `self.sealed` -> `self.admitted` in the marker branches, and the
  IX.52 tests still pass.
* A spent allowance is granted per id, with a per-id reason and a cited decision;
  there is deliberately no `--allow-all` and no wildcard.
* Proof text for every listed id remains unreadable in every mode, including
  `--mode dev`; the emitter's proof-read path is asserted by the tests.
* The mode grants nothing for ids it does not list, and the real manifest is
  restricted to the 18 ARM R4 development ids.
