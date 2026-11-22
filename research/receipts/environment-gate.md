# Receipt — benchmark environment gate (2026-11-22)

Environment: `/mnt/1tb_kingston/Tekmerion/.bench` (git-ignored copy of
`github.com/kings-crown/s2n-bignum-bench`). Pinned by `setup.sh`: HOL Light
`d30e8239887b2348a063f82232a864de1cc2df15`, TacticTrace
`7ea3808cc28258113fd41ef9d860dbfe7691c190`, s2n-bignum
`5e0fc782ae6d587c2ae40faf2c2c4d0b637e6240`.

## F1 — ARM build: pass

- Command: `setup.sh --reuse --skip-hol arm 8`
- Object build: 348 `.o` files.
- `make build_proofs -j8`: ~6,330 s.
- Total wall clock: **6,331 s (1 h 45 m 31 s), exit 0**.
- Peak RAM: 105 / 125 GB. No OOM, no exit 137.
- Artefacts: `toplevel-thms/arm/` present; 664 JSON files measured directly
  (`ls .bench/toplevel-thms/arm/*.json | wc -l`). A subagent reported "332 theorem JSONs";
  the two counts disagree and the reason is unresolved — quote 664 files, not 332 theorems,
  until it is.

### Correction to the earlier diagnosis

The first build failed with `Error 127`, and I attributed it to a missing
`aarch64-linux-gnu-as`. That was **wrong**. The true cause was that the invoked `PATH`
omitted `/home/one/.local/bin`, where `opam` lives, so `opam env` was a no-op and
`camlp5r` was not found. The `llvm-mc` shim at `.bench/bin/aarch64-linux-gnu-as` was
therefore unnecessary, though it was validated independently against real GNU as 2.44
(identical `.text` md5, 11,292 bytes) before that was known.

## F2 — corpus: partial, and the count dispute is resolved

`collect-problems.py toplevel-thms/ problems.json ml_files` at this pin, with ARM + common
only (x86 still building) → **1,915** problems:

| category | count |
|---|---|
| generic | 648 |
| functional_correctness_arm | 463 |
| program_state | 450 |
| bit_vector | 354 |
| functional_correctness_x86 | 0 (not yet collected) |

468 obligations were dropped with recorded reasons (352 "top-level subroutine that just uses
`_CORRECT`", 106 "not a proof", 10 "goal evaluated at runtime").

**Count dispute.** `timeout-map.json` contains exactly **2,622** entries: `arm` 1,106 +
`common` 809 + `x86` 707. 1,106 + 809 = 1,915, the observed ARM+common total. Therefore:

- 2,622 is the full corpus **at this pin** (the x86 half is the missing 707).
- **2,301 and 2,284 are not reproduced by this commit.** Do not quote either without naming
  the commit that produces it.

## F3 — adjudicator: pass, with one expectation corrected

No language model or weights were used.

| Check | Result | Evidence |
|---|---|---|
| Real human proofs (4 routines) | **OK ×4** | `eval-20261122_170720/*.judge.txt`; profile 0.013–0.084 s |
| `CHEAT_TAC` | **CHEATING** | `eval-20261122_170911/arm.word_clz.WORD_CLZ_CORRECT.judge.txt`; mechanism `mk_thm` → `new_axiom` at `hol_lib_inlined.ml:5051` |
| `ALL_TAC` (genuine failure) | **FAIL** | `eval-20261122_171002/arm.word_clz.WORD_CLZ_CORRECT.judge.txt` |
| `THIS_IS_NOT_A_TACTIC` | compile error, **no judge file** | `log-eval-20261122_170952.txt` |
| Empty answer | compile error, **no judge file** | `log-eval-20261122_170957.txt` |

**Correction:** this benchmark revision has **no `SYNTAX_ERROR` verdict**. `GUIDE.md` and
`collect-verdicts.py` list `OK`, `FAIL`, `CHEATING`, `ERROR`, `TIMEOUT` only. The preregistration
and any harness report must use `ERROR`/pre-check failure for malformed answers, not
`SYNTAX_ERROR`.

## Tooling defect found

`combine-answer-and-setup.py <dir> <cores>` crashes on Python 3.14 with `KeyError` in the
`forkserver` path when `cores > 1`. Workaround used: `cores = 1`. Report this upstream or pin a
Python version; do not silently run single-threaded without saying so.

## F4 — model arms: blocked

No model API credentials exist in this environment. The published 6.35% (Pass@1) and 10.26%
(Pass@5 with feedback) arms **cannot be reproduced here**. No substitution was attempted.

## State at the end of this receipt

- x86 object build complete (342 `.o`/`.obj`); x86 proofs still running (63/352 modules at
  17:42Z), projected completion ~19:45–20:15Z.
- Disk: 15 GB free on the project volume. Watch it; stop below 3 GB.
- Full corpus of 2,622 requires re-running `collect-problems.py` after x86 finishes.
