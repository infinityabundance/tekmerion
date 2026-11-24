# IX.52 — two explicit held-out guard modes

Base commit: `7812160c7b416cd81c24fa578c10ea931aab591c` (branch `staging`,
working tree modified; nothing committed or pushed for IX.52).
Demonstration output: `research/receipts/ix52-demo.log` (verbatim CLI output,
generated 2026-11-24T00:36:15Z on the final file hashes).

## Mode semantics

| | `--develop` (default) | `--sealed-eval` |
|---|---|---|
| held-out task id | refused | allowed iff listed in the manifest |
| held-out ELF object | refused | allowed iff its derived task id is listed, the path is `<arm\|x86>/<dir>/bignum_<op>_<family>.o`, and `--root` is given |
| held-out proof `.ml` | refused | **always refused**, including `--mode dev` reads |
| non-held-out paths | unchanged | unchanged |
| preconditions | none | manifest + tag; `git rev-parse TAG` = manifest `freeze_commit`; no tracked changes under `tools/` |

The manifest is the only authority. There is no wildcard or blanket override;
`emit_proof.py --allow-heldout` was removed.

Manifest (JSON; unknown keys are refused):

```json
{
  "freeze_commit": "<40 lowercase hex>",
  "freeze_tag": "<tag, optional but checked when present>",
  "sealed_ids": ["x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT", "..."],
  "note": "optional"
}
```

Each `sealed_ids` entry must be a complete
`<arm|x86>.bignum_<add|sub|neg>_<family>.<BIGNUM_<OP>_<FAMILY>_CORRECT>` id
with the theorem name consistent with the operation and family. Prefixes,
wildcards, duplicates, non-strings, bad JSON and a bad commit hash are refused.

## Exact flags for a sealed run

```
python3 tools/emit_ref_proof_x86.py \
    --sealed-eval \
    --sealed-manifest research/receipts/<the IX.53 manifest>.json \
    --freeze-tag <the IX.53 tag> \
    --task x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT \
    --mode answer --out <answer.txt>
```

`emit_proof.py` and `emit_ref_proof.py` take the same three mode flags.
The guard itself takes them plus `--root <s2n-bignum>` for object allowance.

## Observed results

- `python3 -m pytest tools/` — `48 passed, 1 skipped, 1 warning` (the skip is
  the pre-existing `test_derive_steps.py` HOL gate, which needs
  `TEKMERION_RUN_HOL=1`; the warning is the pre-existing `TestFailure`
  collection warning).
- `python3 -m pytest tools/test_heldout_modes.py` — `17 passed`.
- `cargo test --offline` — `79 passed, 0 failed` (26 + 7 + 25 + 21 across
  the four populated binaries; two binaries report 0 tests).

The six required behaviours are in `tools/test_heldout_modes.py`: develop
refusal (task + object), sealed allowance of the nine ids and objects, refusal
of an unlisted held-out id/object, refusal of a listed task's proof `.ml`
(also through `emit_ref_proof{,_x86}.modulus_definition`), freeze/tag/commit/
dirty-tree refusal, and duplicate/malformed manifest refusal.

No sealed answer was generated. `--sealed-eval` on this tree refuses because
`tools/` is modified, which is the intended pre-freeze state.

## Still to do (IX.53)

- Write the IX.53 manifest, commit it, tag it, and re-run with a clean `tools/`
  tree. Until then no sealed number exists and none may be quoted.
- The demonstration used a throw-away clean git repository (`/tmp`) with the
  guard copied byte-for-byte; the real tree cannot pass the freeze check while
  the IX.52 edits are uncommitted.
- `emit_ref_proof.py` (ARM reference route) was also given the modes because it
  shares the proof-read path, although the brief named only the other two
  emitters.
