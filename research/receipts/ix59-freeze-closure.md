# IX.59 — Freeze closure: bind the Rust producer, not just `tools/` (XIII.C)

Date: 2026-11-24. Branch `staging`. No Python was used or added.

## 1. The defect

`src/tools/guard.rs::verify_freeze` verified the freeze with two git commands
scoped to `tools/`:

```
git status --porcelain --untracked-files=no -- tools
git diff --name-only <manifest.freeze_commit> -- tools
```

Before the migration the emitters were Python under `tools/`. After it they are
Rust under `src/tools/` and `src/bin/`, compiled into `target/release/*`. Neither
command observes any of that, so the seal was satisfied by:

- a different, clean commit carrying a modified emitter
  (`git status` is clean, `git diff -- tools` is empty), and
- a swapped binary under the recorded name (`target/release/emit` is not in any
  git tree at all).

A freeze that does not bind the program that produced the proofs seals nothing.
This is the assurance gap XIII.C names; it is closed here for everything a
checkout or the recorded machine can observe.

## 2. What the closure records

New module `src/tools/freeze.rs`, new binary `src/bin/freeze.rs`. Canonical JSON,
hand-serialised (no new crate; the build stays `--offline` with only `sha2`).

**Source layer** — portable, recomputable from a checkout with no build:

- every regular file under `src/**` (which includes `src/bin/**` and
  `src/native/**`), recursively;
- `Cargo.toml`, `Cargo.lock`, and `rust-toolchain.toml` / `rust-toolchain` when
  present (neither toolchain file exists on this tree);
- every `tools/**/*.ml`, recursively, and every `tools/*.sh`, top level only;
- one SHA-256 per file, sorted by path (strictly, duplicates refused);
- `source_digest` = SHA-256 over the sorted concatenation
  `path\0sha256\n` of those entries.

This tree: **65 source files**, `source_digest` =
`8a0d2439c494647b2c0dd10cff16393b93b6bdc35cd34a55701d02443186b88d`.

**Runtime layer** — machine-specific, recorded at run time:

- the compiled executable (`target/release/emit`) and its SHA-256;
- `rustc --version`;
- `Cargo.lock` SHA-256;
- the frozen HOL Light pin (`git rev-parse --short HEAD` of the copy) and the
  `hol_lib.cmxa` SHA-256;
- the `cadical` path, SHA-256 and `--version`;
- the OCaml bridge: every top-level `tools/*.ml`, one SHA-256 each;
- the benchmark input: `.bench/problems.json` SHA-256 (and its path).

This run recorded 10 runtime digests: executable, HOL library, cadical,
benchmark input, and the six top-level `tools/*.ml` bridge files.

## 3. Why the two layers are kept separate

Only the source layer is portable. A binary digest is a statement about one
build on one machine: rebuild the same sources with a different `rustc`, a
different linker, or a different feature set and the digest changes while the
source has not. Putting the executable in the portable layer would either make
the closure unverifiable off this host or tempt someone to treat a build hash as
a source hash. So the runtime layer says exactly what it is — evidence about
the recorded environment, not a claim about every environment — and
`freeze verify` without `--runtime` checks only the portable half.

The runtime layer is nevertheless required on a sealed run: a closure-carrying
sealed manifest is enforced with source *and* runtime (`verify_freeze`), because
the run being sealed is the run that must have been produced by the recorded
binary.

## 4. `check-tag` and the commit that cannot name itself

A file cannot contain the hash of the commit that contains it: the blob hash
feeds the tree hash feeds the commit hash feeds the blob. Empirically, the
literal rule "`git rev-parse <tag>^{commit}` equals `repo_commit`, and the
manifest is committed at that tag" has no solution for any commit. The probe
here is structural, not a failed attempt: the manifest is committed at the tag,
so its `repo_commit` would have to be the tag's own hash.

The implemented relation is the strongest satisfiable one:

1. the tag resolves to a commit `T`;
2. `repo_commit` is `T` or an ancestor of `T`;
3. the source closure recomputed from `repo_commit`'s tree equals the record;
4. the source closure recomputed from `T`'s tree equals the record;
5. the manifest committed at `T` (same repo-relative path) parses and carries
   the same `source_digest` and the same `repo_commit`.

Steps 3–5 are what stop "a clean commit with modified tools": a tag moved to a
commit whose source differs is refused at step 4, whether or not the tree is
clean. Step 5 is the required refusal when the tag's committed manifest differs
in `source_digest`.

In the seal built here, `repo_commit` is the code commit
`d25c34cf2c04b525644b9503dadf1099363eb93e`; the tag points at its child, the
commit that commits this receipt and the manifest. That child's hash cannot be
written here for the reason above; it is the commit of this file.

## 5. The tag, and what is (not) signed

- Tag: **`freeze-rust-closure-v5`**, annotated, at the seal commit.
- **Unsigned.** There is no GPG key on this host and inventing an unattributed
  one is forbidden. `tag.gpgsign` is disabled explicitly. This means the tag
  protects against content drift relative to the recorded digests, not against
  a rewrite of the repository history, which would need a signature or an
  external witness.
- Manifest committed at the tag: `research/receipts/freeze-closure-v5.json`.
- `sha256` of that manifest, as committed:
  `cdc32f2223897c0a7d4947f3d844f7832a83df1aad3dadd7ffef79b32d9e3d1b`
  (9469 bytes).
- `research/receipts/ix59-freeze-closure.json` is a byte-identical copy of that
  manifest (`sha256sum` agrees), placed here so the receipt bundle carries the
  seal itself.

Recorded environment: `rustc 1.98.0 (88d9e12ae 2026-08-18)`, `cadical` 2.1.2 at
`/home/one/.elan/toolchains/leanprover--lean4---v4.29.1/bin/cadical`
(`sha256 48bbd538966156dedba887201e656549152a9eaf6ac5a72e00d47f26fcf3c846`),
HOL Light pin `d30e823` at `.bench/hol-light-stamp`
(`hol_lib.cmxa sha256 e7fea282f3818e13ec19491a069d53cb0de29b1748ab6de38dd7edd7f576ddce`),
`.bench/problems.json sha256 74f314445fa5f579d18b03e73c167ea0ca56e0581e26add8a11d8d077f7ea8d4`.

## 6. Generation and verification

```
$ cargo run --release --offline --bin freeze -- manifest --out research/receipts/freeze-closure-v5.json --runtime --tag freeze-rust-closure-v5
wrote research/receipts/freeze-closure-v5.json: 65 source files, source_digest 8a0d2439c494647b2c0dd10cff16393b93b6bdc35cd34a55701d02443186b88d, 10 runtime digests, repo_commit d25c34cf2c04b525644b9503dadf1099363eb93e

$ cargo run --release --offline --bin freeze -- verify --manifest research/receipts/freeze-closure-v5.json --runtime
OK: source closure verified: 65 files, source_digest 8a0d2439c494647b2c0dd10cff16393b93b6bdc35cd34a55701d02443186b88d
OK: runtime closure verified against the recorded build
```

The positive control below (§8, control 6) is the same `verify` run from a fresh
`git clone` that never saw the working tree: it reproduces the identical
`source_digest`, which is acceptance criterion 1.

## 7. Wiring into `verify_freeze`, without breaking freeze v4

- A sealed manifest may carry `freeze_closure` (a clean repo-relative path). If
  present, `verify_freeze` refuses unless the closure names the same tag, the
  tag binding of §4 holds, the working tree's source closure matches, and the
  runtime closure matches.
- If absent, the two legacy `tools/` checks run **unchanged**, and the run
  prints:

  ```
  warning: freeze manifest has no freeze_closure; the legacy scope (tools/) does not bind the Rust producer in src/tools, src/bin or target/release. See research/receipts/ix59-freeze-closure.md.
  ```

  reproduced verbatim with `cargo test --offline --test tools_guard -- --nocapture`
  (the legacy tests trigger it).

**Freeze v4 remains valid only for its stated scope, and that scope does not
bind the Rust producer.** IX.53–IX.57 sealed manifests are untouched, still
parse, and still verify under the `tools/` rule; they were not retroactively
upgraded, and their tags still resolve exactly as before. Concretely, a legacy
manifest verified against *today's* tree is refused by the legacy rule whenever
`tools/` has moved since its freeze commit — that is the old rule doing exactly
what it always did, not a regression. The unit tests exercise legacy acceptance
on a clean throwaway repository, and the warning above is emitted on that path.

## 8. Firing controls

Each control was run on a throwaway clone
(`/mnt/1tb_kingston/tekmerion-freeze-controls/ctl`, removed afterwards) of the
code commit, with the seal copied in, `target/release/emit` copied in, and
`.bench` symlinked for the runtime inputs. The real tree was never dirtied.

**Control 1 — one byte appended to `src/tools/emit.rs`:**

```
$ ./target/release/freeze --repo <ctl> verify --manifest <ctl>/research/receipts/freeze-closure-v5.json --runtime
REFUSED: sealed source closure: source file src/tools/emit.rs differs: recorded 0cf6939ea3f830da073a212a97037f4146470bcbe966ddadcd8cc7373c130c50, actual 7a362189bd62ea116e9288fe81ae980ce057f9632dfb6e73a5181019c4ecca63
exit 2
```

**Control 2 — one byte appended to `target/release/emit`:**

```
$ ./target/release/freeze --repo <ctl> verify --manifest <ctl>/research/receipts/freeze-closure-v5.json --runtime
OK: source closure verified: 65 files, source_digest 8a0d2439c494647b2c0dd10cff16393b93b6bdc35cd34a55701d02443186b88d
REFUSED: runtime executable digest mismatch for target/release/emit: recorded a9798594bb0ea5664ca220367f1ffc8fe638ccc6b1035bb27d48c969177feaf0, actual 63906c04abc8c82053f3e46d93eddba74f25f17a808f3b96850ba82fcc136f0e
exit 2
```

(The source layer is untouched, as intended: the binary is not source.)

**Control 3 — newline appended to `Cargo.lock`:**

```
$ ./target/release/freeze --repo <ctl> verify --manifest <ctl>/research/receipts/freeze-closure-v5.json --runtime
REFUSED: sealed source closure: source file Cargo.lock differs: recorded bb23bec90e5ead40633fe4db645113c536c3048e6d1e9214e73d798ba1226637, actual b5460f71fd3cc712d6282cbfd22759899b1b2919ffd89a19bdcb63f7888469b3
exit 2
```

**Control 4a — recorded `cadical` digest altered to zeros:**

```
$ ./target/release/freeze --repo <ctl> verify --manifest <ctl>/research/receipts/ix59-control4.json --runtime
OK: source closure verified: 65 files, source_digest 8a0d2439c494647b2c0dd10cff16393b93b6bdc35cd34a55701d02443186b88d
REFUSED: runtime cadical digest mismatch for /home/one/.elan/toolchains/leanprover--lean4---v4.29.1/bin/cadical: recorded 0000000000000000000000000000000000000000000000000000000000000000, actual 48bbd538966156dedba887201e656549152a9eaf6ac5a72e00d47f26fcf3c846
exit 2
```

**Control 4b — runtime check pointed at a different `cadical` (byte-changed copy):**

```
$ ./target/release/freeze --repo <ctl> verify --manifest <ctl>/research/receipts/ix59-control4b.json --runtime
OK: source closure verified: 65 files, source_digest 8a0d2439c494647b2c0dd10cff16393b93b6bdc35cd34a55701d02443186b88d
REFUSED: runtime cadical digest mismatch for /mnt/1tb_kingston/tekmerion-freeze-controls/ctl/cadical-altered: recorded 48bbd538966156dedba887201e656549152a9eaf6ac5a72e00d47f26fcf3c846, actual 9c8bc7735df5ce2595fddd77cee8e65f98cba329320b79282a9090b7aee1ba91
exit 2
```

**Control 5a — recorded HOL Light pin altered:**

```
$ ./target/release/freeze --repo <ctl> verify --manifest <ctl>/research/receipts/ix59-control5a.json --runtime
OK: source closure verified: 65 files, source_digest 8a0d2439c494647b2c0dd10cff16393b93b6bdc35cd34a55701d02443186b88d
REFUSED: runtime HOL Light pin mismatch for .bench/hol-light-stamp: recorded 0000000, actual d30e823
exit 2
```

**Control 5b — recorded `hol_lib.cmxa` digest altered:**

```
$ ./target/release/freeze --repo <ctl> verify --manifest <ctl>/research/receipts/ix59-control5b.json --runtime
OK: source closure verified: 65 files, source_digest 8a0d2439c494647b2c0dd10cff16393b93b6bdc35cd34a55701d02443186b88d
REFUSED: runtime HOL Light hol_lib.cmxa digest mismatch for /mnt/1tb_kingston/tekmerion-freeze-controls/ctl/.bench/hol-light-stamp/hol_lib.cmxa: recorded 0000000000000000000000000000000000000000000000000000000000000000, actual e7fea282f3818e13ec19491a069d53cb0de29b1748ab6de38dd7edd7f576ddce
exit 2
```

**Control 6 — positive: untouched fresh checkout:**

```
$ ./target/release/freeze --repo <ctl> verify --manifest <ctl>/research/receipts/freeze-closure-v5.json --runtime
OK: source closure verified: 65 files, source_digest 8a0d2439c494647b2c0dd10cff16393b93b6bdc35cd34a55701d02443186b88d
OK: runtime closure verified against the recorded build
exit 0
```

## 9. `check-tag` controls (same throwaway copy)

The closure manifest was committed in the throwaway clone and tagged; the tag
points one commit past `repo_commit`, exercising the ancestor path.

```
$ ./target/release/freeze --repo <ctl> check-tag --manifest <ctl>/research/receipts/freeze-closure-v5.json
OK: tag freeze-rust-closure-v5 resolves to f436dcb85e94c672f75757205c5aea4b17eaf3f8; source_digest 8a0d2439c494647b2c0dd10cff16393b93b6bdc35cd34a55701d02443186b88d matches the manifest committed at the tag
exit 0
```

Then the manifest committed at the tag was replaced by a self-consistent
manifest generated from a tree with one byte added to `src/tools/emit.rs`
(`source_digest 70188817780c8f43728bfaaa16bd38ec6009ba743610dbc9803b471d8489a249`),
the source was restored, the tag was moved to that commit, and the working-tree
manifest was restored to the sealed one before the check:

```
$ ./target/release/freeze --repo <ctl> check-tag --manifest <ctl>/research/receipts/freeze-closure-v5.json
REFUSED: manifest committed at tag 41e1575280acd692027689390510205fbad10c9d records source_digest 70188817780c8f43728bfaaa16bd38ec6009ba743610dbc9803b471d8489a249, but research/receipts/freeze-closure-v5.json records 8a0d2439c494647b2c0dd10cff16393b93b6bdc35cd34a55701d02443186b88d
exit 2
```

A tag moved to a *clean* commit with a modified emitter is refused earlier, by
the tag-tree comparison (step 4 of §4); the unit test
`check_tag_binds_the_sealed_tree` asserts that path names `src/tools/emit.rs`.

The real tag did not exist while this file was written (the file is committed at
the tag). The real `check-tag` run is therefore reported outside this receipt,
in the task report, and can be reproduced with the command in §10.

## 10. Mutation harness: before and after

Before XIII.C: `src/tools/mutate_guard.rs` mutated only `src/tools/guard.rs` and
reported **58/58 killed, 0 survivors**.

After: the harness takes a table of (file, test target) and mutates both
`src/tools/guard.rs` (run against `tools_guard`) and `src/tools/freeze.rs` (run
against `tools_freeze`). Two new refusal sites were added to `guard.rs`
(`freeze_closure` path validation; closure tag versus `--freeze-tag`), and 38
new sites in `freeze.rs`. Every new site has a test that asserts its specific
message, so replacing the site with `Ok(todo!())` fails the suite.

```
$ cargo run --release --offline --bin mutate_guard

src/tools/guard.rs refusal sites: 60 (0 invalid)
killed: 60  survived: 0
mutation score: 60/60 = 1.000

src/tools/freeze.rs refusal sites: 38 (0 invalid)
killed: 38  survived: 0
mutation score: 38/38 = 1.000

all targets: 98 refusal sites (0 invalid), killed 98, survived 0
mutation score: 98/98 = 1.000
all survivors are recorded equivalent mutants
```

So the pre-existing 58 remain killed (58 of the 60), and the 40 new sites
(2 in `guard.rs`, 38 in `freeze.rs`) are all killed.

## 11. Exact commands and checks

```
$ cargo test --offline
… test result: ok. 9 passed (tools_freeze); 28 passed (tools_guard);
  57 lib tests and all other targets ok; 0 failed.

$ cargo clippy --offline --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s)

$ cargo fmt --check
(no output)

$ bash tools/selftest.sh
SELF-TEST PASSED

$ git ls-files '*.py'
(no output — no Python is tracked)

$ df -h /mnt/1tb_kingston | tail -1
/dev/nvme1n1p1  1.3T  1.3T   11G 100% /mnt/1tb_kingston
```

Reproduction, from the repository root:

```
cargo build --release --offline --bin emit --bin freeze
cargo run --release --offline --bin freeze -- verify --manifest research/receipts/freeze-closure-v5.json
cargo run --release --offline --bin freeze -- verify --manifest research/receipts/freeze-closure-v5.json --runtime
cargo run --release --offline --bin freeze -- check-tag --manifest research/receipts/freeze-closure-v5.json
cargo run --release --offline --bin mutate_guard
```

The first `verify` needs nothing but a checkout. The second recomputes the
runtime layer and is only expected to pass on the machine that produced the
recorded build; on another machine it refuses, which is the point of keeping
that layer separate.

## 12. What this establishes, and what it does not

Establishes: the seal now covers every source file that produces the proofs
(all of `src/**`, the cargo manifests, the OCaml bridge, the shell entry
points), and every runtime dependency recorded for this host; a one-byte change
to any of them is refused, naming the first differing path and both digests; a
tag whose committed tree or manifest differs is refused; and every refusal site
is held by a test.

Does not establish:

- **Signature.** The tag is unsigned — there is no GPG key on this host. Content
  drift is detected against the recorded digests; history rewriting is not.
- **Portability of the runtime layer.** The executable, HOL and cadical digests
  are statements about this build on this host, deliberately not about any
  other. The source layer is the portable half.
- **The HOL Light patch text.** `tools/hol-light/axiom-stamp.diff` is not in the
  source closure (it is neither `.ml` nor a top-level `.sh`). The HOL build is
  bound at runtime by its pin plus `hol_lib.cmxa` digest, not by that diff. If
  the patch text must be sealed, it belongs in `SOURCE_FILES` — a separate
  decision, recorded separately.
- **`tests/`, `examples/`, `.github/`.** They are not part of the producer and
  are outside the closure by construction.
- **The `repo_commit` field naming the tag's own commit.** Proven impossible in
  §4; the tag commit is the commit of this file and is reported with the push.
