# Receipt — G4 discharged: the abstract soundness lemmas are machine-checked

- **Date:** 2026-11-24
- **Branch:** `staging`
- **Artefact:** `verification/tnps/Tnps.lean` (949 lines), `verification/tnps/README.md`
- **Toolchain:** Lean 4.29.1, core only (no mathlib, no lake, no network)
- **Verdict claimed:** **G4 is discharged for the abstract logic only.** It is *not* a
  statement about the Rust checker.

## 1. What was proved

| spec | claim | Lean name |
|---|---|---|
| §3 | typing determines the sort | `HasType.unique` |
| §4 | well-typed evaluation agrees with the sort | `eval_bool`, `eval_bitvec`, `valid_eq_iff` |
| §6.2 | **RUP is sound** | `rup_sound` |
| §6.3 | **LRAT refutation soundness**, via model preservation of the original formula | `Lrat.accepted_models`, `Lrat.refutation_sound` |
| §9 | rule justifications imply validity | `Cert.checked_sound` |
| §10 | **Theorem 1**, with the blast lemma as a hypothesis | `Cert.theorem1` |

The earlier written argument claimed that deletion preserves UNSAT. That is false
(`{x, ¬x}` minus `x` is satisfiable). The formalisation proves the correct invariant: a
model of the **live** set survives every admitted RUP addition and every deletion, so a
final live empty clause contradicts the existence of a model of the initial formula.

## 2. Compile and axiom check (verbatim)

```
$ cd verification/tnps
$ /home/one/.elan/toolchains/leanprover--lean4---v4.29.1/bin/lean Tnps.lean
'hasConflict_sound' depends on axioms: [propext]
'propagate_no_conflict' depends on axioms: [propext]
'rup_sound' depends on axioms: [propext, Classical.choice, Quot.sound]
'Lrat.accepted_models' depends on axioms: [propext, Classical.choice, Quot.sound]
'Lrat.refutation_sound' depends on axioms: [propext, Classical.choice, Quot.sound]
'Cert.theorem1' depends on axioms: [propext, Classical.choice, Quot.sound]

exit status: 0
```

Independently re-run by the orchestrator; exit 0, identical output.

## 3. Proof hygiene (verbatim scan)

```
$ grep -nE '\bsorry\b|\badmit\b|^\s*axiom |\bunsafe\b|sorryAx' verification/tnps/Tnps.lean
20:  Lean 4.29.1 core only: no mathlib, no lake, no `sorry`, no `axiom`, no
21:  `unsafe`, no `native_decide`.  The only axioms reported by `#print axioms`
```

The only matches are comment text in the file header. `blast_sound` appears only as a
theorem **parameter** (`Cert.checked_sound`, `Cert.theorem1`), never as an `axiom`
declaration.

## 4. What this does **not** establish

- **G1 (blast lemma).** `Cert.blast` is an `opaque` function; its soundness is the
  hypothesis parameter `blast_sound`. Nothing is proved about the gate clauses, the
  ripple-carry encoder or blast determinism.
- **G2 (parser refinement).** Not addressed.
- **G3 (executable refinement).** Not addressed. **The Rust checker is not verified.**
- §1.5 canonical rendering, §1.6 digests/blobs, §8 id uniqueness/Kahn ordering and the
  §7 resource caps are not modelled.
- The formalisation's `HasType` omits the `1 ≤ n ≤ 64` side conditions on
  `eq`/`add`/`sub` (evaluation still masks at `2^n`). This is a deviation from §3 and is
  recorded in the README.

## 5. Metric movement

`research/receipts/ix43-tcb.md` recorded **gaps discharged 0/4**. This moves it to
**1/4**, with G4 specifically scoped to the abstract logic and G1 still an assumption of
the `bitblast` rule.
