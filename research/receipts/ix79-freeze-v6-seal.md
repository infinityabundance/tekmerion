# IX.79 — Freeze v6, sealed

**Date:** 2026-11-25
**Decision:** seal the producer as-is. Later integrity work is a `v7` delta, recorded against
`v6`; it is never a precondition for citing a `v6`-scoped result.

## What is sealed

| item | value |
|---|---|
| tag | `freeze-rust-closure-v6` (annotated, **unsigned**) |
| tag commit | `cb054bde47088d14619b27f242803b2ebf6535b7` |
| `repo_commit` | `7d8e5fa8223b4db09db29be35a3db28c8d488868` |
| manifest | `research/receipts/freeze-closure-v6.json` |
| `source_digest` | `e798c9b09709efef153fb036bbe2762663a1ee05397948d202feb37358bf6429` (72 files) |
| runtime digests | 10 |
| configuration | 6 `TEKMERION_*` variables, values literal (`unset` when unset) |

Verified, not asserted:

```
$ freeze verify   --manifest research/receipts/freeze-closure-v6.json
OK: source closure verified: 72 files, source_digest e798c9b0…
OK: effective configuration verified: 6 TEKMERION_* variables

$ freeze check-tag --manifest research/receipts/freeze-closure-v6.json
OK: tag freeze-rust-closure-v6 resolves to cb054bd…; source_digest e798c9b0… matches the manifest committed at the tag
```

## Why unsigned

This host has no GPG secret keys. Creating an unattributed key to make a signature appear would be
worse than an honest unsigned tag: content drift is caught against the recorded digests, while
history rewriting is not, and the receipt says so.

## What was stale, and why the seal was regenerated rather than forced

The first attempt to seal at `5c24303` was refused by `check-tag`:

```
REFUSED: sealed code commit 5c24303: source file src/tools/emit.rs differs:
         recorded 3084c452…, actual b02430f4…
```

The manifest recorded IX.78's `emit.rs` digest while naming `repo_commit 5c24303`, whose tree still
had the pre-IX.78 file. Forcing the tag would have sealed a false statement. The manifest was
regenerated so `repo_commit` names the tree it describes; `source_digest` did not change, which is
itself evidence that only the commit pointer was wrong.

## What v6 does and does not bind

Binds: every file under `src/**`, `Cargo.toml`, `Cargo.lock`, `tools/**/*.ml`, `tools/*.sh`; the
compiled executable, `rustc`, the HOL Light pin and `hol_lib.cmxa`, `cadical`, the OCaml bridge, the
benchmark input digest; and the effective `TEKMERION_*` configuration.

Does **not** bind (all queued as `XVII.A`/`XVIII.A` `v7` deltas):
`semantics_sha256` is `base.ml` alone rather than its transitive closure; executable identity is
computed after the build; the sealed task manifest is not closure-bound; `mode`/`stamp`/`force` are
recorded but not enforced; the disassembler probe matches a mnemonic, not a full decode; digest
fields are strings, not a validated `Digest` type.
