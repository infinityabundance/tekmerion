# TNPS abstract logical core — Lean 4 development (trusted-base gap G4)

A machine-checked formalisation of the mathematics in
`research/08-TNPS-SEMANTICS.md` §2–§10.  One file, Lean 4.29.1 **core only**
(no mathlib, no lake, no network).

```
verification/tnps/Tnps.lean     the development (949 lines)
verification/tnps/compile.log   verbatim compiler output, exit status, #print axioms
verification/tnps/README.md     this file
```

---

## 1. Scope: what G4 is, and what it is not

**Discharged: G4, for the abstract logic only.** The claims of the specification
that are *mathematical statements about the objects defined in the
specification itself* are proved:

* §6.2 Lemma (RUP is sound) — `rup_sound`;
* §6.3 Lemma (LRAT refutation soundness) — `Lrat.refutation_sound`, via the
  model-preservation invariant `Lrat.accepted_models`;
* §9 rule justifications and §10 Theorem 1 (abstract soundness) —
  `Cert.checked_sound`, `Cert.theorem1`.

**Not discharged, and not claimed:**

* **G1 (blast lemma, §7).** `Cert.blast` is an `opaque` uninterpreted function.
  Its soundness is the *hypothesis parameter* `blast_sound` of
  `Cert.checked_sound` / `Cert.theorem1`. It is **not** an `axiom` declaration.
  Nothing here proves anything about the gate clauses, the ripple-carry
  encoder, or `blast` determinism.
* **G2 (parser refinement, §1.7).** Not addressed at all.
* **G3 (executable refinement).** Not addressed at all. **Nothing in this file
  says that the Rust checker in `src/` is verified.** The development talks
  only about the Lean definitions in `Tnps.lean`.
* The §1.5 canonical rendering and §1.6 digest/blob conditions, §8 id
  uniqueness and Kahn ordering, and the §7 resource caps are not modelled.

`compile.log` reports axioms `propext`, `Classical.choice`, `Quot.sound` and
nothing else (see §5). There is no `sorry`, `admit`, `axiom`, `unsafe` or
`native_decide` in the development; the only textual matches for those words
are the two comment lines 20–21 of the file header.

---

## 2. Compile

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

No warnings. `wc -l Tnps.lean` → `949 Tnps.lean`.

---

## 3. Definition → specification map

| Lean name (line) | Specification |
|---|---|
| `Ty` (29) | §2 sorts `Bool`, `BitVec(n)` |
| `Term`, `Term.mkBin` (35, 48) | §1.1 term grammar; `mkBin` is `add`/`sub` selected by a `Bool` |
| `HasType` (54) | §3 typing rules |
| `HasType.unique` (100) | §2/§3 "no overloading" |
| `Value`, `eval`, `Valid` (69, 74, 97) | §2 values; §4 evaluation and validity |
| `eval_bool`, `eval_bitvec` (114, 129) | §4: well-typed terms evaluate at their own sort (and words stay in range) |
| `valid_eq_iff` (149) | §4/§9 bridge: `Valid (eq n a b)` ⇔ operands evaluate equally |
| `Lit`, `Clause`, `CNF`, `TotalAssign`, `PartialAssign` (177–181) | §5 assignment alphabet |
| `litVar`, `litPos`, `negLit`, `litTrue`, `litTrueP` (184–206) | §5/§6 literal meaning; `¬C` as `C.map negLit` |
| `assign`, `Agrees` (212, 216) | §6.1 partial-assignment extension and its link to a total model |
| `CSat`, `FSat`, `Sat`, `Unsat` (220–228) | §5 clause/CNF semantics; "empty clause is satisfied by no assignment" |
| `CSt`, `scan` (403, 411) | §6.1 deterministic clause classification (sat/conflict/unit/undetermined) |
| `hasConflict`, `findUnit` (501, 520) | §6.1 conflict detection and first unit |
| `Outcome`, `propagate`, `litCount` (543, 553, 548) | §6.1 deterministic unit propagation with explicit fuel |
| `unit_forces`, `propagate_no_conflict` (566, 581) | §6.2 proof of the propagation invariant |
| `Rup`, `rup_sound` (612, 617) | §6.2 RUP and Lemma 6.2 |
| `Lrat.Live`, `Live.cnf/insert/delete`, `run` (646–670) | §6.3 checker state and its deterministic transition |
| `Lrat.Step`, `Lrat.Accepted` (665, 679) | §6.3 LRAT records and the acceptance relation |
| `Lrat.Legal`, `Lrat.NonTaut`, `Lrat.Fresh` (656–662) | §6.3 range / non-tautology / freshness side conditions |
| `Lrat.accepted_models`, `Lrat.refutation_sound` (708, 728) | §6.3 Lemma (LRAT soundness), corrected proof |
| `Cert.blast` (747) | §7 `blast` (opaque; G1) |
| `Cert.Step`, `Step.premises`, `Step.conclusion` (752–769) | §8 DAG node, rule, premises, conclusion |
| `Cert.Justified`, `Cert.Checked` (785, 798) | §9 per-rule justification; §8/§9 whole-certificate acceptance |
| `Cert.checked_sound`, `Cert.theorem1` (828, 903) | §10 induction invariant; Theorem 1 |

---

## 4. Exact theorem statements

Part 2 — Lemma 6.2:

```lean
theorem rup_sound {F : CNF} {C : Clause} (α : TotalAssign)
    (hrup : Rup F C) (hnz : ∀ l, l ∈ C → l ≠ 0) (hF : FSat α F) : CSat α C
```

Part 3 — model preservation and Lemma 6.3:

```lean
theorem Lrat.accepted_models (α : TotalAssign) {ss : List Step} {L L' : Live}
    (h : Accepted ss L L') (hα : FSat α L.cnf) : FSat α L'.cnf

theorem Lrat.refutation_sound {F : CNF} {L₀ : Live} (hinit : L₀.cnf = F)
    {ss : List Step} {L : Live} (hacc : Accepted ss L₀ L)
    (hempty : ([] : Clause) ∈ L.cnf) : Unsat F
```

Part 4 — §10:

```lean
theorem Cert.checked_sound
    (blast_sound : ∀ P, HasType P .bool → Unsat (blast P) → Valid P) :
    ∀ {ss : List Step}, Checked ss →
      ∀ s ∈ ss, HasType s.conclusion .bool ∧ Valid s.conclusion

theorem Cert.theorem1
    (blast_sound : ∀ P, HasType P .bool → Unsat (blast P) → Valid P)
    {ss : List Step} {root : Step} (h : Checked (ss ++ [root])) :
    Valid root.conclusion
```

Supporting results used by the above (all proved, no axioms beyond the standard
three): `HasType.unique`, `eval_bool`, `eval_bitvec`, `valid_eq_iff`,
`scan_sat`, `scan_conflict`, `scan_unit`, `hasConflict_sound`,
`findUnit_sound`, `litTrue_neg_of_false`, `litTrue_true_imp`,
`litTrueP_agree_true`, `litTrueP_agree_false`, `litTrueP_assign_self`,
`Agrees.assign`, `unit_forces`, `propagate_no_conflict`, `Lrat.cnf_insert`,
`Lrat.cnf_delete_mem`, `Lrat.accepted_run`, `Cert.exists_of_premise`,
`Cert.premise_facts`.

Read the statements literally: `Unsat F` is `¬ Sat F`, `FSat α F` is
`∀ C ∈ F, CSat α C`, and `CSat α C` is `∃ l ∈ C, litTrue α l = true`.

---

## 5. `#print axioms` (verbatim)

```
'hasConflict_sound' depends on axioms: [propext]
'propagate_no_conflict' depends on axioms: [propext]
'rup_sound' depends on axioms: [propext, Classical.choice, Quot.sound]
'Lrat.accepted_models' depends on axioms: [propext, Classical.choice, Quot.sound]
'Lrat.refutation_sound' depends on axioms: [propext, Classical.choice, Quot.sound]
'Cert.theorem1' depends on axioms: [propext, Classical.choice, Quot.sound]
```

These are exactly Lean's standard axioms (`Classical.choice` enters through
`by_cases` on propositions, via `Classical.propDecidable`). No `sorryAx`, no
`blast_sound` axiom, no custom axiom.

---

## 6. Modelling choices

1. **Names.** `Name = Nat`. The specification leaves the name alphabet open;
   `Nat` keeps the development core-only and gives decidable equality.
2. **Literals.** `Lit = Int` with `0` reserved as "not a legal literal". The
   encodings make `0` permanently false (`litTrue α 0 = false`,
   `litTrueP σ 0 = some false`), and `Legal C` (and `rup_sound`'s `hnz`) exclude
   `0` exactly where soundness needs it. DIMACS forbids a `0` *inside* a clause,
   so no faithful input is excluded. Note `negLit 0 = 0`; the hypothesis
   `hnz : ∀ l ∈ C, l ≠ 0` is what makes `F ++ C.map (fun l => [negLit l])`
   a genuine `F ∧ ¬C`.
3. **Typing.** The draft's `HasType` carries the §3 side conditions `1 ≤ n ≤ 64`
   on `var` and `const` only; the `eq`/`add`/`sub` rules do not carry them. This
   is a *deviation from §3* (typing is slightly more permissive outside `1..64`)
   and does **not** weaken any soundness theorem: `eval` masks with `% 2^n`,
   which is total for every `n`, so `eval_bitvec`'s range proof still holds. It
   is recorded here rather than silently repaired.
4. **Negative values.** `sub` evaluates to `(x + 2^n - y) % 2^n`, the
   natural-number rendering of `(⟦a⟧ - ⟦b⟧) mod 2^n` for operands below `2^n`.
   The arithmetic identity with `Int` subtraction is not proved in this file and
   is not needed for any theorem here.
5. **Deterministic propagation order.** Clause order and literal order are the
   list order of `F`; each round runs `hasConflict`, then `findUnit`, then
   `assign`. Fuel is `litCount (F ++ ¬C) + 1`. A fuel shortfall can only return a
   fixpoint (refusal), never a false conflict, so the *soundness* statement is
   unaffected. Completeness of the fuel bound, and confluence of propagation
   under other orders, are **not** proved.
6. **LRAT state.** `Live := List (Nat × Clause)`; `Live.cnf` projects to the
   clause set; `run : List Step → Live → Live` is the deterministic machine and
   `Lrat.Accepted : List Step → Live → Live → Prop` is the relational
   specification. `Lrat.accepted_run` proves `Accepted ss L L' → run ss L = L'`.
   The initial live set is a *parameter* `L₀` and the only hypothesis linking it
   to the formula is `L₀.cnf = F`; the caller supplies the initial ids. No
   id-uniqueness check is modelled: `Fresh` and the deletion liveness hypothesis
   only make `Accepted` stricter, and ids are never read when proving soundness.
7. **RUP is taken as the criterion** for an addition. §6.3 establishes that an
   accepted hint sequence *implies* RUP; the converse is not required for
   soundness, so hint sequences are not formalised. This matches the
   specification's own remark that acceptance is one witness, not every RUP
   clause.
8. **Index-based premises (Part 4).** A certificate is a `List Step` in which a
   step's premises are *indices of earlier steps*. Acceptance (`Cert.Checked`) is
   built by `snoc`, so a step is only ever checked against the prefix that
   precedes it: every premise is an earlier node and the graph is acyclic **by
   construction**, without modelling ids, `dom`, or Kahn's algorithm. Dangling
   references are impossible (a premise index must be `< ss.length`, and
   `List.get?` resolution is total), and `Cert.exists_of_premise` recovers the
   referenced step. `Cert.Step`'s `id` fields are inert labels in this model.
9. **`congr`.** One rule with `slot : Bool`: `slot = true` replaces the first
   operand of `add`/`sub`, `slot = false` the second. The untouched operand is a
   single shared term `t`, so "syntactically identical on both sides" holds by
   construction; "exactly one replaced operand" holds because only one slot can
   be selected. `op : Bool` selects `add` vs `sub` at the shared width.
10. **`bitblast`.** Justified when there **exists an accepted LRAT refutation of
    `blast (conclusion)`** whose final live set contains `[]`
    (`Cert.Justified`, `.bitblast` case). The rule is therefore discharged by
    `Lrat.refutation_sound` + the hypothesis `blast_sound`; it is not assumed.
    `blast` itself is `opaque`, so no equation about it is available anywhere.
11. **No assumption channel.** As in the executable (§9, Remark 1), the only
    accepting instance is `E = ∅`; the root's conclusion is what is proved
    valid. Theorem 1 is therefore not relativised to an assumption set.

---

## 7. What is proved, and what is not

### Proved

* §6.2 **Lemma (RUP soundness)** — `rup_sound`, with the `l ≠ 0` side condition,
  by the propagation invariant `propagate_no_conflict`.
* **Propagation invariant** — if `α ⊨ F` and `α` agrees with `σ`, `propagate`
  cannot return `.confl`, for any fuel.
* §6.3 **Lemma (LRAT soundness)** — `accepted_models` states the invariant
  *"a model of the live set survives every accepted record"*, and
  `refutation_sound` derives `Unsat F` from an accepted run that ends with the
  empty clause live. The proof tracks a model of the **original** formula. It
  never claims that deletion preserves unsatisfiability — which is false for
  `{x, ¬x}` — matching the §6.3 correction note of 2026-11-24.
* §9 **rule justifications and §10 Theorem 1** — `checked_sound` proves, by
  induction over the snoc-built certificate, that every step's conclusion is a
  well-typed Boolean term and is valid; `theorem1` extracts the root's validity.
* **Non-vacuity.** Two `example`s are checked at compile time (they are not named
  theorems): a closed LRAT refutation of `[[1], [-1]]` using the empty clause, and
  a `refl` certificate for `x = x` at width 4. If the definitions were wrong in a
  way that made the hypotheses unsatisfiable, these would not typecheck.

### Not proved (open, and deliberately out of scope)

* **G1** — `blast_sound` is a hypothesis. `blast` is opaque; nothing is proved
  about the encoder, gate clauses, canonical variable allocation, or the
  `i32::MAX` / 4,000,000 caps of §7.
* **G2** — parser specification and canonical-language refinement (§1.7). Open.
* **G3** — executable refinement: no statement here relates `Tnps.lean` to the
  Rust checker in `src/`. **The Rust checker is not verified by this work.**
* **Completeness** — that the fuel bound suffices, that every RUP clause is
  accepted, or that hint sequences and `Rup` coincide.
* **§1.5/§1.6** canonical rendering, SHA-256 digests, blob lengths, and
  structural CNF comparison (`CnfMismatch`).
* **§8** id uniqueness (u32), root-id existence, ascending-id Kahn order, blob
  reference coverage.
* **§3 width side conditions** on `eq`/`add`/`sub` (see modelling choice 3).

---

## 8. Provenance

* Compiler: `/home/one/.elan/toolchains/leanprover--lean4---v4.29.1/bin/lean`,
  Lean 4.29.1, core only.
* `wc -l verification/tnps/Tnps.lean` → `949`.
* Compile exit status `0`; warnings: none.
* Hygiene grep: `grep -nE '\bsorry\b|\badmit\b|^\s*axiom |\bunsafe\b' verification/tnps/*.lean`
  returns exactly two hits, both comment text in the file header:

  ```
  20:  Lean 4.29.1 core only: no mathlib, no lake, no `sorry`, no `axiom`, no
  21:  `unsafe`, no `native_decide`.  The only axioms reported by `#print axioms`
  ```

  A broader grep (`sorry|admit|axiom|unsafe|native_decide`) additionally hits
  line 746 (`below; it is never declared as an axiom. -/`, also a comment) and
  the six `#print axioms` commands at lines 944–949. Nothing in the file is an
  axiom declaration, and there is no `sorry`, `admit`, `unsafe` or
  `native_decide` anywhere.
* Comparison with `research/08-TNPS-SEMANTICS.md` is by section, as in §3 above.
  Where the formalisation deviates (typing side conditions, `sub` rendering,
  opacity of `blast`, index-based premises), the deviation is stated in §6 and
  §7 rather than hidden.
