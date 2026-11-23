# R2 freeze manifest — witness-compilation pipeline (K1′)

Frozen at **2026-11-23T01:28:28Z** on branch `staging`.

Base commit: `8dc7a224bedb6d5ad467e9e5189ec7c76aac4656` ("w2/w4: first generated
benchmark OK x3 on P-256 (zero model calls, no new axioms)").

Per instruction, the frozen tree is **not committed and not pushed**. The base
commit above is the last commit; the two modified files below are the
uncommitted working-tree state at freeze. The authoritative pin is the sha256
table, not the commit id.

After this timestamp none of the seven files below may change. The training
validation and the single held-out evaluation must use exactly these bytes.

## Frozen pipeline files

| file | bytes | sha256 |
|---|---:|---|
| `tools/witness.py` | 8767 | `156ac91e615f133237a56a080d6f4c7e281fefd15bac6000e9a1bf8b2b5f6f8b` |
| `tools/w2_witnesses.py` | 14886 | `4b1e9c605fe1d79a16fc283d2398f8c7db32f37059e578987bd755a659413d03` |
| `tools/w4_close.py` | 11910 | `71eeaa353452345cf1a56f973bf66d6edea58e64bfa71b64c1c9eba2d54d9cc6` |
| `tools/emit_proof.py` | 6504 | `92c4f8964bc1477ecb4cec54e863e4205a092a679769223bf0d3f0b27eabfd65` |
| `tools/derive_steps.py` | 10472 | `d0034b31d77ba675d0b883cc186111810fa1da14c55b0a3cfd4625f004774aa6` |
| `tools/disasm_steps.py` | 4455 | `fe8c9d2502e2d033868e850178153f5f4892158ff4b7d99d4be8651dcf78ed50` |
| `tools/heldout_guard.py` | 5263 | `6d346402febbdfecd31294a5b1d8558ef015394da012bba28d2204ac5cd14f70` |

Combined digest (sha256 of the sorted `"<sha256>  <path>"` lines above, one per
line, trailing newline):

```
63b39c75ada0bd8ba2da86048152c0ce2d25d732e2aac3a08b1da252a8017d9c
```

Among these seven pipeline files, only `tools/w2_witnesses.py` and
`tools/emit_proof.py` differ from the base commit. `witness.py`,
`w4_close.py`, `derive_steps.py`, `disasm_steps.py` and `heldout_guard.py` are
byte-identical to `8dc7a22`. Receipts and `TODO.md` added after the freeze are
not part of the frozen digest.

## Witness schema

`tools/witness.py` defines `SCHEMA_VERSION = "v3"`. A witness is a JSON object
with these fields (top-level order as declared in the frozen file):

| field | kind | meaning |
|---|---|---|
| `op` | string, one of `add`/`sub`/`neg` | operation family |
| `theorem_name` | string | benchmark theorem name |
| `exec_name` | string | `*_EXEC` constant bound to the ELF object |
| `accumulate` | sorted 1-based ints | derived carry-chain step numbers |
| `congruence` | object | `principle`, `kind`, `width`, `witness`, `modulus`, optional `quotient`, `orientation` |
| `flag_meaning` | object | `flag`, `meaning`, `rule`, optional `bound`, `source_steps` |
| `case_split` | object | `kind`, optional `condition`, `true_branch`, `false_branch`, `lemmas` |
| `bounds` | list of HOL terms | side conditions |
| `invariant` | HOL term | postcondition equality |
| `vars`, `var_types`, `inputs`, `guard` | derived scaffolding | query variables, types, input refs, postcondition guard |
| `width_bits`, `width_mod`, `final_state`, `simulation_range` | derived scaffolding | width, state name, simulated step range |
| `development_only` | list of keys | closer fragments transcribed from the human proofs |
| `schema` | string | `"v3"` |

The schema is versioned in code: `witness.py` rejects an empty principle,
width, witness or modulus; `w2_witnesses.py` builds the record. The digest of a
witness is `sha256(json.dumps(record, indent=0, sort_keys=True))`.

## Scope and gates in the frozen bytes

- The modulus, bit width, input references, digit count, result pointer, guard
  and conclusion are parsed from the benchmark query by
  `w2_witnesses.parse_spec`; no curve name or P-256 constant appears in that
  path.
- `emit_proof.py --task ID` derives the operation, family and ELF object path
  from the task id and a glob under `.bench/s2n-bignum/arm/*/`. The `--op`
  shortcut that carried P-256 defaults was removed before freeze.
- `emit_proof.py` refuses a held-out task id unless `--allow-heldout` is passed;
  the single held-out evaluation passes it. The gate is in the frozen hash.
- `guard_object` refuses any input that is not a `.o` file and any path under a
  `proofs` tree. The pipeline reads the ELF object and `problems.json` only; it
  opens no proof file.
- The close skeleton remains `DEVELOPMENT_ONLY` and is transcribed from the
  P-256 human proofs, as recorded in `w4_close.DEVELOPMENT_ONLY` and in each
  witness's `development_only` list.

## What this freeze does not assert

Freezing fixes the bytes, not the scope of their competence. It does **not**
assert that the pipeline closes every training curve, and it does **not** assert
a like-for-like comparison with the published 5/898 FC figure. Those statements
belong to the evaluation receipt, with their denominators.
