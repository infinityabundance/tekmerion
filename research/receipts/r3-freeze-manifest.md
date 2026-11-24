# R3 freeze manifest — ARM `add`/`sub`/`neg` coverage run (V8.19)

Frozen at **2026-11-23T13:57:05Z** UTC on branch `staging`.
Base commit: `672cc0b45270f262de9bf76773e2fa8d0c0f9304`
("bridge: add/sub/neg P-256 and P-384 add all OK; first coverage increase over
the template generator").

Working tree was clean at freeze (`git status --short` empty). Nothing below is
committed or pushed. The authoritative pin is the sha256 table, not the commit id.

## 1. Declared task set (fixed before any answer was generated)

Source: `.bench/problems.json`, every task with
`category == "functional_correctness_arm"` whose id matches
`arm.bignum_(add|sub|neg)_<family>.` — i.e. operation ∈ {add, sub, neg},
across all curves.

**N = 18** (6 curves × 3 operations).

```
arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT
arm.bignum_add_p256.BIGNUM_ADD_P256_CORRECT
arm.bignum_add_p256k1.BIGNUM_ADD_P256K1_CORRECT
arm.bignum_add_p384.BIGNUM_ADD_P384_CORRECT
arm.bignum_add_p521.BIGNUM_ADD_P521_CORRECT
arm.bignum_add_sm2.BIGNUM_ADD_SM2_CORRECT
arm.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT
arm.bignum_neg_p256.BIGNUM_NEG_P256_CORRECT
arm.bignum_neg_p256k1.BIGNUM_NEG_P256K1_CORRECT
arm.bignum_neg_p384.BIGNUM_NEG_P384_CORRECT
arm.bignum_neg_p521.BIGNUM_NEG_P521_CORRECT
arm.bignum_neg_sm2.BIGNUM_NEG_SM2_CORRECT
arm.bignum_sub_p25519.BIGNUM_SUB_P25519_CORRECT
arm.bignum_sub_p256.BIGNUM_SUB_P256_CORRECT
arm.bignum_sub_p256k1.BIGNUM_SUB_P256K1_CORRECT
arm.bignum_sub_p384.BIGNUM_SUB_P384_CORRECT
arm.bignum_sub_p521.BIGNUM_SUB_P521_CORRECT
arm.bignum_sub_sm2.BIGNUM_SUB_SM2_CORRECT
```

Taskset digest (sha256 of the sorted ids, one per line, trailing newline):

```
54de4367fd72ddce0618ea9c10499e9be91901fc7943f4088b0b1bee3ccb2cab
```

No task was added or removed after this point. All 18 carry the per-problem
timeout `120 s` in `.bench/timeout-map.json` (the harness adds 30 s to the alarm).

## 2. Declared dispatch (fixed before any answer was generated)

The pipeline has two answer generators. Dispatch is by operation and family, not
by expected outcome:

| route | generator | tasks |
|---|---|---|
| C-SAT bridge | `make_inlined_answer.py <out> add-ref` | `add_p256`, `add_p384` |
| C-SAT bridge | `make_inlined_answer.py <out> sub` | `sub_p256` |
| C-SAT bridge | `make_inlined_answer.py <out> neg` | `neg_p256` |
| witness | `tools/emit_proof.py --task … --dir … --receipt …` | the other 14 |

`--allow-heldout` is passed to `emit_proof.py` for the six `p25519` / `p256k1` /
`sm2` add/sub/neg tasks, because that is the frozen gate's designed exception
for a declared run. It is a task-id gate only; the generator reads the ELF
object and the query, never a proof file.

A task whose generator exits non-zero, or whose answer is rejected by the
harness's `ocamlc -i` syntactic gate, is recorded as **no answer emitted**. It is
not judged and not counted as `FAIL`.

## 3. Frozen pipeline files

| file | bytes | sha256 |
|---|---:|---|
| `tools/bridge.ml` | 41034 | `3a32e6b0e5e7f36fd8d49e9a79cac930668beca031416c6c9b992b227115d52f` |
| `tools/csat_bridge.ml` | 8807 | `8c1dc799f508d3e4f2df5e0b549ec2cd9f36bbda6e85f723464596ee31e2e726` |
| `tools/csat_lib.ml` | 5556 | `70439c92d58046a065312fc4faad9e06d53ddd64f89f8e93f676b815cc83e5a8` |
| `tools/emit_proof.py` | 6504 | `92c4f8964bc1477ecb4cec54e863e4205a092a679769223bf0d3f0b27eabfd65` |
| `tools/derive_steps.py` | 10759 | `894ef23f25bc64769b5471b242d9ae6e5d8b9b723e988ed2e24566335baf9ee8` |
| `tools/disasm_steps.py` | 4455 | `fe8c9d2502e2d033868e850178153f5f4892158ff4b7d99d4be8651dcf78ed50` |
| `research/receipts/bridge/make_inlined_answer.py` | 8129 | `9093b086998748cbc76cade2b71cb4ae66a424b3364b4ca18398ec177b65e6ae` |
| `tools/w2_witnesses.py` | 14886 | `4b1e9c605fe1d79a16fc283d2398f8c7db32f37059e578987bd755a659413d03` |
| `tools/w4_close.py` | 11910 | `71eeaa353452345cf1a56f973bf66d6edea58e64bfa71b64c1c9eba2d54d9cc6` |
| `tools/witness.py` | 8767 | `156ac91e615f133237a56a080d6f4c7e281fefd15bac6000e9a1bf8b2b5f6f8b` |

Combined digest (sha256 of the sorted `"<sha256>  <path>"` lines above, one per
line, trailing newline):

```
a6cc2a4dac8088b01b4503695ec74d94f556bf4ebeed02a16e009a6d70e2ba9f
```

`make_inlined_answer.py` inlines three HOL Light sources into every bridge
answer. They are inputs to the emitted bytes, so they are pinned too:

| file | sha256 |
|---|---|
| `.bench/hol-light/Cadical/dimacs.ml` | `3f891558cef8e1e7dc390232fee53bb434950b6bb7578b9c2e5c28c76cf11aad` |
| `.bench/hol-light/Cadical/cnf.ml` | `7813b91b504bfda9ad0d4e404fa1172f85de4480500a3972b0b726732e3eb581` |
| `.bench/hol-light/Cadical/ldrup.ml` | `3a34bfe69968dcdec7b6523ed1fb70b8d6806b1f24b9cc25b161b7f737086d1a` |

The judge harness files (`.bench/combine-answer-and-setup.py`,
`.bench/run-answers.sh`, `.bench/collect-verdicts.py`, `.bench/synchk.sh`) are
part of the official judge and are used unmodified.

## 4. Environment

- `cadical`: `/home/one/.elan/toolchains/leanprover--lean4---v4.29.1/bin/cadical`
  (ELF x86-64, 1109192 bytes). It is **not** on the default `PATH`; the run
  exports that directory on `PATH`. If it were absent, `CSAT_REFUTE_GEN` fails
  with exit 127, which is an environment failure, not a proof failure (V8.21).
- One core for `combine-answer-and-setup.py` (Python 3.14.10 crashes with >1).
- No network. No edits to `.bench/s2n-bignum`, `objfiles/`, `toplevel-thms/`.

## 5. What this freeze does and does not assert

- It fixes the bytes and the task set. It does **not** assert that either
  generator closes every task.
- It is **not** a blind held-out claim. All 18 tasks have prior visible
  outcomes in this programme: the nine R2 tasks (`p25519`, `p256k1`, `sm2` ×
  add/sub/neg), the four bridge-developed tasks (`p256` add/sub/neg, `p384`
  add), and the five R2 *training* tasks (`p384` sub/neg, `p521` add/sub/neg).
  This run is a **coverage** measurement over the current pipeline.
- Benchmark `OK` is under the `CHEAT_TAC` context of the harness. It is not an
  axiom-free development.
- The denominator is 18 ARM `add`/`sub`/`neg` tasks. It is not the 898-task or
  5/898 functional-correctness denominator, and no rate is compared across
  those denominators.

After the run the ten hashes above are recomputed and must match.

---

## Addendum (Slice D): ported to Rust

Ported to Rust in Slice D on `staging` (base commit `b28016c`).
`research/receipts/bridge/make_inlined_answer.py` is deleted. Reproduction is now
`cargo run --release --bin inline_answer -- <out-answer.txt> <mode>`; all four modes are
byte-identical to the Python original:

| mode | sha256 |
|---|---|
| `add-ref` | `39fd3d64c65063410c02f0a879d505632b4d82042ac3a64f26c137426e23264f` |
| `sub` | `b36a7fd0274396d72bdbe87f6cd8810888ad308bfe415e952f9bfed015c9e176` |
| `neg` | `b5a674d51aeb621f11611200a7e47451b4d6ada2e434728bf60e6eb49143cb6e` |
| `add-p384` | `83d90067847d411d3eee7fb7b6ca6b532d790ab262341e6ba5d248d54db15044` |

See `research/receipts/slice-d-rust-receipts.md`. Historical text above is untouched.
