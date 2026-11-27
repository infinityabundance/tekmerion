# XXV.E — mutation operator 2 (silent acceptance)

**Date:** 2026-11-27
**Branch:** `staging`
**File:** `src/tools/mutate_guard.rs`
**Run:** `tools/cap.sh --timeout 3600 12G -- cargo run --offline --quiet --bin mutate_guard`
(reachability) and the same with `MUTATE_GUARD_OPERATOR=accept`.

## Why

The existing harness replaces `Err(Refused::new(..))` with `Ok(todo!())`. `todo!()` panics, so any
test that **reaches** the site kills the mutant. The 101/101 score therefore measures
**reachability**, not whether a test **asserts** the refusal. The failure a checker must not have is
**accepting what it should refuse**, which `todo!()` cannot model.

## What was added

A second operator, selected by `MUTATE_GUARD_OPERATOR=accept` (or `--operator accept`), which
replaces each refusal site with `Ok(Default::default())` — execution continues as if the input were
valid. The default remains `reachability`, so the existing CI step is unchanged. A valid accept
mutant survives unless a test asserts the refusal; the harness exits non-zero on any such survivor.

The selector is **not** named `TEKMERION_*`: `tools::config::tests::declared_binary_vars_match_the_source`
binds every `TEKMERION_*` literal under `src/` to the freeze-recorded configuration set, and a dev
harness switch does not belong there. `MUTATE_GUARD_OPERATOR` avoids changing the recorded schema.

## Results

### Reachability (unchanged)

```
src/tools/guard.rs refusal sites: 60 (0 invalid)     killed: 60  survived: 0
src/tools/freeze.rs refusal sites: 41 (0 invalid)    killed: 41  survived: 0
all targets: 101 refusal sites (0 invalid), killed 101, survived 0
mutation score: 101/101 = 1.000
```

### Assertion strength (`accept`)

```
operator: accept (mutant `Ok(Default::default())`)
src/tools/guard.rs refusal sites: 60 (27 invalid)    killed: 33  survived: 0
accept mutation score: 33/33 = 1.000
src/tools/freeze.rs refusal sites: 41 (13 invalid)   killed: 28  survived: 0
accept mutation score: 28/28 = 1.000
all targets: 101 refusal sites (40 invalid), killed 61, survived 0
accept mutation score: 61/61 = 1.000
```

| file | sites | invalid | valid | killed | survived |
|---|---:|---:|---:|---:|---:|
| `src/tools/guard.rs` | 60 | 27 | 33 | 33 | 0 |
| `src/tools/freeze.rs` | 41 | 13 | 28 | 28 | 0 |
| **total** | 101 | **40** | 61 | 61 | 0 |

**41/41 freeze sites in reachability includes the two IX.90 sites; in accept, 13 freeze sites are
invalid.** The invalid sites are those whose `Ok` payload does not implement `Default`
(`Result<Json>`, `Result<SealedManifest>`, `Result<GuardPolicy>`, `Result<Closure>`,
`Result<&BTreeMap<..>>`, `Result<&Json>`, `Result<std::process::Output>`,
`Result<EffectiveConfig>`). The 61 that compiled prove the span replacement is well formed, so the
40 invalid mutants are a payload limitation, not a parser fault.

## What this does and does not establish

- **Establishes:** at **61 of 101** refusal sites, silently accepting the invalid input makes the
  test suite fail. Those tests distinguish refusal from acceptance.
- **Does not establish:** anything about the **40** sites whose payload has no `Default`; they cannot
  be modelled by this operator. A variant-specific "plausible value" operator is the follow-up
  (`XXV.E3`).
- **Caveat:** "killed" means the target test failed once the refusal was accepted. It does not prove
  a dedicated `Err` assertion exists; a downstream failure on the default value also kills the
  mutant. There were **zero survivors**, so there is nothing to triage as a missing assertion today.
- The accept operator is **not yet wired into CI** (`XXV.E4`); doing so is a deliberate decision after
  a variant-specific operator raises the valid count.

## Validation

`cargo test --offline` exit 0; `cargo clippy --offline --all-targets -- -D warnings` exit 0, both
under `tools/cap.sh`. Only `src/tools/mutate_guard.rs` changed (+122/−23).
