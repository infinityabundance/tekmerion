# IX.53 freeze reconciliation — annotated tag vs dereferenced commit

Status: resolved 2026-11-24. No re-tag was performed; none was needed.

## Claim under test

The IX.53 freeze was reported as inconsistent: `freeze_commit` in
`research/receipts/ix53-sealed-manifest.json` was `22fc118…`, but
`git rev-parse --short freeze-x86-sealed-v2` returned `120360b`.

## Measurement

```
$ git rev-parse freeze-x86-sealed-v2^{commit}
22fc118bb275cc12fe54123b035dc2406edb1511

$ git rev-parse freeze-x86-sealed-v2
120360bfbbaf32d8d733961483c7c3ea3843b1cf

$ git cat-file -t freeze-x86-sealed-v2
tag
```

`freeze-x86-sealed-v2` is an **annotated** tag. `git rev-parse <tag>` returns
the tag *object* id (`120360b`); `^{commit}` dereferences to the commit it
points at (`22fc118`). Only the dereferenced commit is a freeze identity.

## What the guard actually checks

`tools/heldout_guard.py::verify_freeze` (lines 395–400) resolves the tag with
`git rev-parse --verify "<TAG>^{commit}"` and compares it to
`manifest.freeze_commit`. It does not use the bare tag object id. It also
refuses if tracked files under `tools/` are dirty (lines 402–407).

Independent check:

```
$ python3 -c "import sys; sys.path.insert(0,'tools'); import heldout_guard as g; \
  m=g.load_sealed_manifest('research/receipts/ix53-sealed-manifest.json'); \
  g.verify_freeze(m,'freeze-x86-sealed-v2'); print('PASS')"
manifest ok: 22fc118bb275 freeze-x86-sealed-v2 9
verify_freeze: PASS
policy ok; sealed ids: 9
```

## Convention

- The tag points at the **code** freeze commit (`22fc118`).
- The manifest is committed afterwards (`a2a4b47`); the manifest commit is
  evidence, not part of the freeze identity.
- Therefore `git rev-parse <tag>` (# tag object) and `manifest.freeze_commit`
  (# code commit) are *different by construction* for annotated tags, and this
  is not a disagreement.

## Sealed objects present

```
.bench/s2n-bignum/x86/curve25519/bignum_add_p25519.o
.bench/s2n-bignum/x86/curve25519/bignum_sub_p25519.o   (same dir)
.bench/s2n-bignum/x86/curve25519/bignum_neg_p25519.o
.bench/s2n-bignum/x86/secp256k1/bignum_{add,sub,neg}_p256k1.o
.bench/s2n-bignum/x86/sm2/bignum_{add,sub,neg}_sm2.o
```

All nine resolved (checked before dispatch).

## Repository hygiene (same review)

```
$ git ls-files | wc -l
211
$ git ls-files -z | xargs -0 du -kc | tail -1
2584    total
```

No `cadical` binary, no symlinks, no multi-MB logs. The 52k-line debug/binary
blob described in the earlier review is not present in the tree.
