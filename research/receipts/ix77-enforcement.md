# IX.77 — recording is not enforcing (XVII.A3–A5)

Date: 2026-11-25
Branch: `staging`
Code commit sealed by the regenerated v6 closure: `dec848071498b571edf96513ef0b92a405ec8c4c`

Three defects had the same shape: a fact was *recorded* in the manifest and
checked by one tool, while the path that actually admits a run did not check
it. Each is fixed at the enforcement point, each has a control that fires, and
each residual gap is listed below rather than left implicit.

No tag was created. `freeze-rust-closure-v6` remains the `tag` field of the
manifest only; the lead seals it as a separate decision.

## 0. Resources

```
$ free -g | head -2
               total        used        free      shared  buff/cache   available
Mem:             125          32          85           0           7          92
$ df -h /mnt/1tb_kingston | tail -1
/dev/nvme1n1p1  1.3T  901G  360G  72% /mnt/1tb_kingston
```

Every build/test/verify command below ran through `tools/cap.sh 20G`. Peak
group RSS observed in the full suite: 2,948,048 kB (2.9 GB). No proof build was
required.

## 1. XVII.A3 — the sealed path now enforces the configuration

### Fix

`guard::verify_freeze` loaded the closure, checked the tag, the source closure
and the runtime, but never called the configuration verifier. `freeze verify`
did, so a sealed run could proceed with an unbound `TEKMERION_*` environment.

`src/tools/guard.rs`, in the `freeze_closure` branch of `verify_freeze`, after
`verify_source` and before `verify_runtime`:

```rust
match &closure.configuration {
    Some(config) => crate::tools::config::verify_live(config)?,
    None => eprintln!(
        "{}",
        crate::tools::freeze::configuration_warning(&closure)
            .expect("a configuration-less closure has a warning")
    ),
}
```

The closure-carrying manifest is the one a sealed run must use; a manifest
without a closure still takes the legacy `tools/`-scope branch, which has no
configuration to compare (see §5).

### Firing control (the sealed emit, not `freeze verify`)

`tests/tools_freeze.rs::sealed_emit_refuses_a_live_variable_the_manifest_records_unset`
builds a throw-away repository with a real runtime block, commits the closure
with a configuration that records `TEKMERION_ALG_CLOSE` unset, tags it, writes a
sealed manifest, and invokes the built **emit** binary with the variable set:

```
$ TEKMERION_ALG_CLOSE=ix77-live-control target/debug/emit --sealed-eval \
      --sealed-manifest receipts/sealed.json --freeze-tag freeze-ix77-control \
      --task x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT --mode answer \
      --out /tmp/ix77-answer.txt
refused: configuration mismatch: TEKMERION_ALG_CLOSE recorded unset, live "ix77-live-control"
exit=1
```

The same test then runs emit with all six `TEKMERION_*` variables removed and
asserts that the refusal is *not* a configuration mismatch: the matching live
environment gets past the sealed policy.

`tests/tools_freeze.rs::guard_refuses_a_sealed_run_whose_recorded_configuration_is_not_live`
exercises `guard::verify_freeze` directly with a configuration recording a value
the process cannot have, and requires
`configuration mismatch: TEKMERION_ALG_CLOSE` and the recorded value in the
message.

The end-to-end script `tests/ix77_enforcement_controls.sh` reproduces the same
refusal against the built binaries and is quoted in §2 for A4.

## 2. XVII.A4 — the tag binds the complete committed manifest

### Fix

`check_tag` compared only `source_digest` and `repo_commit` of the manifest
committed at the tag. Any other field — the runtime block, the configuration
block, a file entry — could differ and pass.

`src/tools/freeze.rs` now also compares the **canonical bytes** of the committed
manifest (`render(&at_tag)`) with the canonical bytes of the manifest under
verification (`render(closure)`), after the two specific digest messages. Every
parseable field is bound, including `tag`, `created_at`, `source`,
`configuration` and `runtime`. On a difference it names the first differing
field from a structured comparison (`manifest_difference`, in canonical-render
order, recursing into source entries, configuration variables/flags and every
runtime sub-block); if the structures compare equal but the bytes do not, it
reports the first differing byte offset.

Unknown keys are already refused by the strict parser, so a committed manifest
cannot smuggle an extra field past the parser.

### Firing control (one non-digest field changed)

`tests/ix77_enforcement_controls.sh` (throw-away repository, real runtime
block):

```
$ git diff -- receipts/closure.json   # one field only
    "rustc": "ix77-changed-rustc-version",
Updated tag 'freeze-ix77-control' (was 0e5b0c2)
$ target/debug/freeze check-tag --manifest receipts/closure.json --repo <fixture>
REFUSED: manifest committed at tag 33b11e725987cd8d52f70f84fdf784c4aefe84d8 differs from receipts/closure.json at field runtime.rustc; the tag binds the complete manifest, not selected digests
exit=2
```

The script restores the original manifest into the working tree after re-tagging,
so the manifest under verification is the original and the tag carries the
change. The throw-away commit and tag ids differ on every run (timestamps); the
refused *field* and message do not.

`tests/tools_freeze.rs::check_tag_refuses_a_changed_non_digest_field` pins both a
`runtime.rustc` change (must name `runtime.rustc`) and a
`configuration.env.TEKMERION_ALG_CLOSE` change (must name that variable). The
existing `check_tag_refusals` cases for `source_digest` and `repo_commit` keep
their specific messages and still pass.

## 3. XVII.A5 — only recognised mathematical shapes retry

### Fix

`tools/bridge.ml` classified **every** `Failure` not caught by the sanity,
timeout or signal tests as `Mathematical` and retried the full hypothesis set. A
`Failure` can equally mean a broken rewrite, a missing lemma or an invariant
violation, so it must not be assumed mathematical.

The retry condition is now an explicit allow-list,
`bridge_math_failure_shape`:

| shape | match |
|---|---|
| `solver-sat` | message begins `CSAT: cadical did not report UNSAT, exit 10` |
| `solver-sat` | message contains `SATISFIABLE` |
| `slice-too-strong` | message begins `SLICE_TOO_STRONG` |

The classifier order is unchanged for the existing classes: `timeout`,
`Out_of_memory`, `Stack_overflow`, `Sys.Break`, `Invalid_argument` → unsupported,
the two CSAT sanity messages → internal, exit 124 → resource/timeout, exit ≥ 128
→ resource/signal. Only after those, a `Failure` retries if (and only if) its
message matches the allow-list; otherwise it is
`` `Internal ("unrecognised Failure: " ^ s) `` and `bridge_slice_retry`
re-raises it without running the fallback.

Supported helpers were added at top level (`bridge_failure_has_prefix`,
`bridge_failure_contains`, `bridge_math_failure_shape`); the classifier keeps its
local `exit_code` reader.

### Firing control (allow-listed retries, arbitrary does not)

```
$ bash tests/bridge_failure_selftest.sh
OK mathematical: fallback ran
OK mathematical: counter one
OK mathematical-slice: fallback ran
OK mathematical-slice: counter one
OK internal-generic: re-raised
OK internal-generic: no fallback
OK internal-generic: counter zero
OK classify allow-listed exit 10 is mathematical
OK classify SATISFIABLE is mathematical
OK classify slice-too-strong is mathematical
OK bridge_failure_taxonomy
OK: bridge_failure_selftest
```

The driver's `internal-generic` case raises `Failure "boom"`, asserts the
original exception is re-raised, asserts the fallback never runs and the counter
stays zero, and asserts the note is
`CSAT internal=unrecognised Failure: boom`. `tests/bridge_failure_taxonomy.rs`
now requires both `OK mathematical-slice: fallback ran` and
`OK internal-generic: no fallback`.

## 4. Deliberate re-record (bridge.ml moved for A5)

`tools/bridge.ml` changed only because of A5. Every tracked answer fixture and
every golden derived from it moved, and each was re-recorded deliberately:

```
$ TEKMERION_RECORD_ANSWERS=1 tools/cap.sh 20G -- cargo test --offline --test tools_emit
...
test result: ok. 14 passed; 0 failed
```

| value | before | after |
|---|---|---|
| `BRIDGE_DIGEST` (`tests/tools_receipt.rs`) | `deafeb4de6129686f7a521a0b4afee08bdd15b78aaf703f2bc0dbc39a5d1c072` | `ace0b8680be6e12f44d45dd9c00043effaf558dbc6c7d122ade66d51180ff63a` |
| instrument phrases | 157 | 160 |
| instrumented output sha256 | `cd52a52efb927b656079a786256d3aa4d5da96a24de7fd7cee5fd56a3255556f` | `7a11bed9e3edca32521763e3c0ff7d9cd5c2bb36423f14e66cfa1c3779173215` |
| inline-answer phrases | 201 | 204 |
| `add-ref` digest | `8090b1a9f9de4a1a42cfee4d54072c013b64d1a48e4ea02847ecb4d06fea1df7` | `4363d594f638546261404cc6d781dcf03546731b79029867a8b62074df8e9d4a` |
| `sub` digest | `d2b93b6dab4a1a39ce5b6b549d24ded367234af5b66d547799ab7d76b3bf8d4d` | `eecd75484aa705578407ef681ffd0c801ee9a8c45302ee7f13870396ec634f56` |
| `neg` digest | `93c457d6de3e754fe822d76b32e02d41edabfdedd99e84060bc716565d75476c` | `ee40990a290736addea5c20fa6041b449691fb763086e63d0a79e8c5e91d4e20` |
| `add-p384` digest | `3d9547195cb84b7701d313a3953fa195926c604bb7588c5577a3aecb1a66aed6` | `82a52246a149793a9bec7c55dd532ed6b48ad321234f4168a13a167808d679ad` |
| `curve_check` statements / hits | 76 / 0 | 76 / 0 (unchanged) |

Why each moved:

* The digest and phrase counts move because three top-level `let` phrases were
  added (`bridge_failure_has_prefix`, `bridge_failure_contains`,
  `bridge_math_failure_shape`), and the classifier body was rewritten.
* All four inline-answer digests move because the inlined library text is
  appended to every answer.
* The 27 files under `tests/fixtures/answers/` were re-recorded for the same
  reason (+54 lines each: the allow-list block; the tactic text is unchanged).
* `tests/fixtures/curve_check_bridge.txt` was regenerated with
  `target/debug/curve_check tools/bridge.ml`. The statement count and hit count
  are unchanged (the new code adds no `prove` call), but the report prints line
  numbers, so every statement after the insertion point shifted.

## 5. The v6 closure

Regenerated after the A3–A5 code commit, against that commit:

```
$ tools/cap.sh 20G -- cargo run --release --offline --bin freeze -- manifest \
      --out research/receipts/freeze-closure-v6.json \
      --tag freeze-rust-closure-v6 --hol .bench/hol-light-stamp2 --runtime
wrote research/receipts/freeze-closure-v6.json: 72 source files,
source_digest d2a802ed1be6a466d0e2edf6c6f89596ea5840126e37ce8b4ddc6329f60e7159,
10 runtime digests, repo_commit dec848071498b571edf96513ef0b92a405ec8c4c
```

* `source_digest`: `d2a802ed1be6a466d0e2edf6c6f89596ea5840126e37ce8b4ddc6329f60e7159`
  (was `66168775f1a7c6317782b78e392488640ed0d459ed0dfae05c58c1daf3b0cb88`;
  the source closure changed: `src/tools/freeze.rs`, `src/tools/guard.rs` and
  `tools/bridge.ml`).
* `repo_commit`: `dec848071498b571edf96513ef0b92a405ec8c4c` — the commit that
  contains the A3–A5 source changes.
* `tag`: `freeze-rust-closure-v6`, **named only — no tag was created**.
* 72 source files, 10 runtime digests.

Verification of the regenerated manifest:

```
$ tools/cap.sh 20G -- target/release/freeze verify \
      --manifest research/receipts/freeze-closure-v6.json --runtime
OK: source closure verified: 72 files, source_digest d2a802ed1be6a466d0e2edf6c6f89596ea5840126e37ce8b4ddc6329f60e7159
OK: effective configuration verified: 6 TEKMERION_* variables
OK: runtime closure verified against the recorded build
exit=0
```

Configuration control on the real v6 (the `freeze verify` half of A3; the sealed
half is §1):

```
$ TEKMERION_ALG_CLOSE=arm.bignum_add_p25519 tools/cap.sh 20G -- \
      target/release/freeze verify --manifest research/receipts/freeze-closure-v6.json
OK: source closure verified: 72 files, source_digest d2a802ed1be6a466d0e2edf6c6f89596ea5840126e37ce8b4ddc6329f60e7159
REFUSED: configuration mismatch: TEKMERION_ALG_CLOSE recorded unset, live "arm.bignum_add_p25519"
exit=2
```

### Note on the commit shape

A manifest cannot contain the hash of the commit that contains it (that would
require a SHA-1 fixed point). The v6 file therefore names
`dec848071498b571edf96513ef0b92a405ec8c4c` as the sealed *code* commit and is
committed in its child, which changes no source-closure file. When the lead
creates `freeze-rust-closure-v6` at that child (or any later source-neutral
commit), `check_tag` compares: the tag's source tree, the sealed code commit's
source tree (both equal the recorded 72-file closure) and the canonical bytes of
the committed manifest. The tag was **not** created here.

## 6. What is still recorded but not enforced

Exhaustive for the artefacts touched by this change.

**Freeze closure (`Closure`)**

1. `configuration.flags.mode`, `.stamp`, `.force` are recorded and now bound by
   the A4 tag comparison, but they are **not** compared with the live
   invocation. `config::verify_live` compares the six environment variables
   only. A sealed run started with a different `--mode`, `--stamp` or `--force`
   than the manifest records is not refused by the configuration check. (`.slice`
   is derived from `TEKMERION_SLICE_CONTROL`, which the env comparison covers.)
2. A closure with **no `configuration` block** (made by a build before XVI.C)
   is admitted by both `freeze verify` and `guard::verify_freeze` with a warning
   on stderr; exit status does not change. The warning is the only signal.
3. A sealed manifest with **no `freeze_closure`** takes the legacy `tools/`-only
   scope. `src/tools`, `src/bin` and `target/release` are not bound, by design,
   and the warning says so.
4. `created_at` is now byte-bound at the tag but has no independent semantics;
   nothing checks it against a time window or the tag's date.
5. `freeze verify` does not perform the tag binding at all (that is
   `freeze check-tag`), and without `--runtime` it does not check the runtime
   block. The sealed path (`guard::verify_freeze`) always does both for a
   closure-carrying manifest.

**Tag and sealed manifest**

6. The sealed manifest (`freeze_commit`, `sealed_ids`, `note`) is not
   content-addressed or committed as part of the closure. The guard checks its
   `freeze_commit` against the tag and uses its `sealed_ids`; nothing binds the
   sealed manifest file itself. Replacing it changes which ids are admitted.
7. `check_tag` resolves `tag^{commit}` and compares trees and bytes; it does not
   verify that the tag is annotated, who tagged it, or any signature on the tag.
8. `note` in the sealed manifest is parsed and unused.
9. The field `repo_commit` must have the recorded source tree, so the tag may
   only move among commits whose source closure is unchanged; this is checked,
   not assumed.

**Configuration capture (`src/tools/config.rs`)**

10. Variables outside the declared six are ignored by design (the list is
    asserted against the three sources by tests). A new variable that a future
    code path reads but that is not declared would be unbound until the test
    fails and the list is extended.
11. Non-UTF-8 values are decoded lossily: a non-UTF-8 value compares equal to
    its replacement-character rendering.
12. A literal value of `unset` is indistinguishable from an unset variable
    (documented in `config.rs`).
13. The environment is captured/compared in the process that runs the guard.
    The OCaml bridge inherits the emitter's environment when invoked by it; a
    bridge launched separately is outside the guard.

**Failure classification (`tools/bridge.ml`)**

14. A mathematical outcome whose message is not on the allow-list is now
    `internal` and does not retry. This is fail-closed by design: a solver
    wrapper reporting SAT in a different wording loses the automatic retry and
    must be added to the allow-list deliberately.
15. `Invalid_argument` is still classified `unsupported` even when it reflects
    an internal bug; `Out_of_memory`/`Stack_overflow`/`Sys.Break` and exit
    124/≥128 are unchanged, as required.
16. `bridge_fallback_count` records how many retries ran but is not asserted by
    any runtime consumer; only the selftest reads it.

## 7. Gates

```
$ free -g | head -2
               total        used        free      shared  buff/cache   available
Mem:             125          32          85           0           7          92
$ df -h /mnt/1tb_kingston | tail -1
/dev/nvme1n1p1  1.3T  901G  360G  72% /mnt/1tb_kingston
$ tools/cap.sh 20G -- cargo test --offline
    (all suites ok; peak group RSS 2,948,048 kB)
$ tools/cap.sh 20G -- cargo clippy --offline --all-targets -- -D warnings
    Finished `dev` profile
$ cargo fmt --check
    (no output)
$ git ls-files '*.py'
    (no output; 0 files)
```

`git status --short` before the manifest/receipt commit:

```
 M research/receipts/freeze-closure-v6.json
```

## 8. Files

| file | change |
|---|---|
| `src/tools/guard.rs` | `verify_freeze` calls `config::verify_live` on the sealed path |
| `src/tools/freeze.rs` | `check_tag` binds the complete canonical manifest; structured first-difference naming |
| `tools/bridge.ml` | mathematical-failure allow-list; unrecognised `Failure` → `internal` |
| `tests/bridge_failure_selftest_driver.ml` | allow-listed retries; arbitrary `Failure` does not retry |
| `tests/bridge_failure_taxonomy.rs` | requires the new selftest assertions |
| `tests/tools_freeze.rs` | A3 sealed-path and sealed-emit controls; A4 one-field controls |
| `tests/ix77_enforcement_controls.sh` | binary-level A3/A4 firing controls |
| `tests/tools_receipt.rs` | re-pinned `BRIDGE_DIGEST`, phrase counts, inline digests; boundaries recorded |
| `tests/fixtures/answers/**` | 27 answer fixtures re-recorded |
| `tests/fixtures/curve_check_bridge.txt` | regenerated report |
| `research/receipts/freeze-closure-v6.json` | regenerated, `source_digest d2a802ed…` |
| `TODO.md` | `XVII.A3`–`XVII.A5` ticked |
