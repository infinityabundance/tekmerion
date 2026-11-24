/-
  Tnps.lean — machine-checked soundness of the TNPS abstract logical core.

  This file formalises the mathematics on paper of
  `research/08-TNPS-SEMANTICS.md` §2–§10:

    Part 1  terms, local typing, evaluation, validity             (§2–§4)
    Part 2  CNF, unit propagation, RUP soundness (Lemma 6.2)      (§5–§6.2)
    Part 3  LRAT steps, refutation soundness (Lemma 6.3)          (§6.3)
    Part 4  certificate DAG, acceptance soundness (Theorem 1)     (§8–§10)

  Scope (see README.md for the full statement of what is and is not proved):

    * G4 (this file) is discharged for the ABSTRACT LOGIC only.
    * G1 (bit-blasting equivalence) is the *hypothesis* `blast_sound`; `blast`
      itself is an uninterpreted `opaque` function.
    * G2 (parser refinement) and G3 (executable refinement of the Rust checker)
      are not addressed at all.

  Lean 4.29.1 core only: no mathlib, no lake, no `sorry`, no `axiom`, no
  `unsafe`, no `native_decide`.  The only axioms reported by `#print axioms`
  are Lean's standard `propext`, `Classical.choice`, `Quot.sound` (see the end
  of the file).
-/

/-! ## Part 1 — terms, typing, semantics (§2–§4) -/

/-- §2. Sorts ("Sort" is a Lean keyword, hence `Ty`). -/
inductive Ty where
  | bool
  | bitvec (n : Nat)
  deriving DecidableEq, Repr

/-- §1.1/§3. Terms.  Variable names are `Nat`. -/
inductive Term where
  | tru
  | fls
  | not (t : Term)
  | eq (n : Nat) (a b : Term)
  | add (n : Nat) (a b : Term)
  | sub (n : Nat) (a b : Term)
  | var (n : Nat) (x : Nat)
  | const (n : Nat) (v : Nat)
  deriving DecidableEq, Repr

/-- `mkBin op n x y` is `add n x y` when `op = true` and `sub n x y` when
    `op = false`; it lets the `congr` rule of §9 be stated once. -/
def Term.mkBin (op : Bool) (n : Nat) (x y : Term) : Term :=
  match op with
  | true => .add n x y
  | false => .sub n x y

/-- §3. Local typing `⊢ t : Sort`. -/
inductive HasType : Term → Ty → Prop where
  | tru : HasType .tru .bool
  | fls : HasType .fls .bool
  | not {t : Term} : HasType t .bool → HasType (.not t) .bool
  | eq {n : Nat} {a b : Term} :
      HasType a (.bitvec n) → HasType b (.bitvec n) → HasType (.eq n a b) .bool
  | add {n : Nat} {a b : Term} :
      HasType a (.bitvec n) → HasType b (.bitvec n) → HasType (.add n a b) (.bitvec n)
  | sub {n : Nat} {a b : Term} :
      HasType a (.bitvec n) → HasType b (.bitvec n) → HasType (.sub n a b) (.bitvec n)
  | var {n : Nat} (x : Nat) (h1 : 1 ≤ n) (h2 : n ≤ 64) : HasType (.var n x) (.bitvec n)
  | const {n : Nat} (v : Nat) (h1 : 1 ≤ n) (h2 : n ≤ 64) (h3 : v < 2 ^ n) :
      HasType (.const n v) (.bitvec n)

/-- §2. Values. -/
inductive Value where
  | bool (b : Bool)
  | bits (v : Nat)

/-- §4. Evaluation under a total valuation `ρ : Name → Nat`. -/
def eval (ρ : Nat → Nat) : Term → Option Value
  | .tru => some (.bool true)
  | .fls => some (.bool false)
  | .not t =>
      match eval ρ t with
      | some (.bool b) => some (.bool (!b))
      | _ => none
  | .eq _ a b =>
      match eval ρ a, eval ρ b with
      | some (.bits x), some (.bits y) => some (.bool (decide (x = y)))
      | _, _ => none
  | .add n a b =>
      match eval ρ a, eval ρ b with
      | some (.bits x), some (.bits y) => some (.bits ((x + y) % 2 ^ n))
      | _, _ => none
  | .sub n a b =>
      match eval ρ a, eval ρ b with
      | some (.bits x), some (.bits y) => some (.bits ((x + 2 ^ n - y) % 2 ^ n))
      | _, _ => none
  | .var n x => some (.bits (ρ x % 2 ^ n))
  | .const n v => some (.bits (v % 2 ^ n))

/-- §4. Validity. -/
def Valid (P : Term) : Prop := ∀ ρ : Nat → Nat, eval ρ P = some (.bool true)

/-- §2/§3. `HasType` determines the sort. -/
theorem HasType.unique {t : Term} {s s' : Ty} (h : HasType t s) (h' : HasType t s') :
    s = s' := by
  induction h with
  | tru => cases h'; rfl
  | fls => cases h'; rfl
  | not _ ih => cases h'; rfl
  | eq _ _ => cases h'; rfl
  | add _ _ ih1 ih2 => cases h'; rfl
  | sub _ _ ih1 ih2 => cases h'; rfl
  | var _ _ _ => cases h'; rfl
  | const _ _ _ _ => cases h'; rfl

mutual
/-- §2/§4. Well-typed terms evaluate to a value of their own sort. -/
theorem eval_bool (ρ : Nat → Nat) {t : Term} (h : HasType t .bool) :
    ∃ b, eval ρ t = some (.bool b) := by
  cases h with
  | tru => exact ⟨true, rfl⟩
  | fls => exact ⟨false, rfl⟩
  | not hb =>
      obtain ⟨b, hb'⟩ := eval_bool ρ hb
      cases b with
      | false => exact ⟨true, by simp [eval, hb']⟩
      | true => exact ⟨false, by simp [eval, hb']⟩
  | eq ha hb =>
      obtain ⟨x, hx, _⟩ := eval_bitvec ρ ha
      obtain ⟨y, hy, _⟩ := eval_bitvec ρ hb
      exact ⟨decide (x = y), by simp [eval, hx, hy]⟩

theorem eval_bitvec (ρ : Nat → Nat) {t : Term} {n : Nat} (h : HasType t (.bitvec n)) :
    ∃ v, eval ρ t = some (.bits v) ∧ v < 2 ^ n := by
  cases h with
  | add ha hb =>
      obtain ⟨x, hx, _⟩ := eval_bitvec ρ ha
      obtain ⟨y, hy, _⟩ := eval_bitvec ρ hb
      exact ⟨(x + y) % 2 ^ n, by simp [eval, hx, hy],
        Nat.mod_lt _ (Nat.pow_pos (by decide))⟩
  | sub ha hb =>
      obtain ⟨x, hx, _⟩ := eval_bitvec ρ ha
      obtain ⟨y, hy, _⟩ := eval_bitvec ρ hb
      exact ⟨(x + 2 ^ n - y) % 2 ^ n, by simp [eval, hx, hy],
        Nat.mod_lt _ (Nat.pow_pos (by decide))⟩
  | var x _ _ =>
      exact ⟨ρ x % 2 ^ n, rfl, Nat.mod_lt _ (Nat.pow_pos (by decide))⟩
  | const v _ _ _ =>
      exact ⟨v % 2 ^ n, rfl, Nat.mod_lt _ (Nat.pow_pos (by decide))⟩
end

/-- §4. Equality of two word terms, in terms of evaluation of the operands. -/
theorem valid_eq_iff {n : Nat} {a b : Term}
    (ha : HasType a (.bitvec n)) (hb : HasType b (.bitvec n)) :
    Valid (.eq n a b) ↔ ∀ ρ : Nat → Nat, eval ρ a = eval ρ b := by
  constructor
  · intro hv ρ
    obtain ⟨x, hx, _⟩ := eval_bitvec ρ ha
    obtain ⟨y, hy, _⟩ := eval_bitvec ρ hb
    have h : eval ρ (.eq n a b) = some (.bool true) := hv ρ
    simp only [eval, hx, hy] at h
    have hxy : x = y := by
      injection h with h1
      injection h1 with h2
      exact of_decide_eq_true h2
    rw [hx, hy, hxy]
  · intro hv ρ
    obtain ⟨x, hx, _⟩ := eval_bitvec ρ ha
    obtain ⟨y, hy, _⟩ := eval_bitvec ρ hb
    have hxy : x = y := by
      have h := hv ρ
      rw [hx] at h
      rw [hy] at h
      injection h with h1
      exact Value.bits.inj h1
    simp only [eval, hx, hy, hxy]
    simp

/-! ## Part 2 — CNF, unit propagation, RUP (§5–§6.2) -/

abbrev Lit := Int
abbrev Clause := List Lit
abbrev CNF := List Clause
abbrev TotalAssign := Nat → Bool
abbrev PartialAssign := Nat → Option Bool

/-- Variable index of a literal, `|l|`. -/
def litVar : Lit → Nat
  | Int.ofNat n => n
  | Int.negSucc n => n + 1

/-- The truth value that makes a literal true (`l > 0`). -/
def litPos : Lit → Bool
  | Int.ofNat n => decide (n ≠ 0)
  | Int.negSucc _ => false

/-- Literal negation `-l` (§6.2's `¬C`). -/
def negLit : Lit → Lit
  | Int.ofNat 0 => Int.ofNat 0
  | Int.ofNat (n + 1) => Int.negSucc n
  | Int.negSucc n => Int.ofNat (n + 1)

/-- §5. Literal truth under a total assignment.  `0` is never true. -/
def litTrue (α : TotalAssign) : Lit → Bool
  | Int.ofNat 0 => false
  | Int.ofNat (n + 1) => α (n + 1)
  | Int.negSucc n => !(α (n + 1))

/-- Literal truth under a partial assignment; `none` means unassigned. -/
def litTrueP (σ : PartialAssign) : Lit → Option Bool
  | Int.ofNat 0 => some false
  | Int.ofNat (n + 1) => σ (n + 1)
  | Int.negSucc n => (σ (n + 1)).map (!·)

/-- Extending a partial assignment with `l := litPos l`. -/
def assign (σ : PartialAssign) (l : Lit) : PartialAssign :=
  fun x => if x = litVar l then some (litPos l) else σ x

/-- A partial assignment agrees with a total one on every assigned variable. -/
def Agrees (σ : PartialAssign) (α : TotalAssign) : Prop :=
  ∀ (x : Nat) (b : Bool), σ x = some b → α x = b

/-- §5. A clause is satisfied if some literal in it is true. -/
def CSat (α : TotalAssign) (C : Clause) : Prop :=
  ∃ l, l ∈ C ∧ litTrue α l = true

/-- §5. A CNF is satisfied if every clause is. -/
def FSat (α : TotalAssign) (F : CNF) : Prop := ∀ C, C ∈ F → CSat α C

/-- §5. Satisfiability / unsatisfiability of a CNF. -/
def Sat (F : CNF) : Prop := ∃ α, FSat α F
def Unsat (F : CNF) : Prop := ¬ Sat F

/-! ### Reduction lemmas for the literal encodings -/

theorem litVar_pos (n : Nat) : litVar (Int.ofNat (n + 1)) = n + 1 := rfl

theorem litVar_neg (n : Nat) : litVar (Int.negSucc n) = n + 1 := rfl

theorem litPos_pos (n : Nat) : litPos (Int.ofNat (n + 1)) = true := rfl

theorem litPos_neg (n : Nat) : litPos (Int.negSucc n) = false := rfl

theorem litPos_zero : litPos (Int.ofNat 0) = false := rfl

theorem negLit_pos (n : Nat) : negLit (Int.ofNat (n + 1)) = Int.negSucc n := rfl

theorem negLit_neg (n : Nat) : negLit (Int.negSucc n) = Int.ofNat (n + 1) := rfl

theorem litTrue_pos (α : TotalAssign) (n : Nat) :
    litTrue α (Int.ofNat (n + 1)) = α (n + 1) := rfl

theorem litTrue_neg (α : TotalAssign) (n : Nat) :
    litTrue α (Int.negSucc n) = !(α (n + 1)) := rfl

theorem litTrue_zero (α : TotalAssign) : litTrue α (Int.ofNat 0) = false := rfl

theorem litTrueP_pos (σ : PartialAssign) (n : Nat) :
    litTrueP σ (Int.ofNat (n + 1)) = σ (n + 1) := rfl

theorem litTrueP_neg (σ : PartialAssign) (n : Nat) :
    litTrueP σ (Int.negSucc n) = (σ (n + 1)).map (!·) := rfl

theorem litTrueP_zero (σ : PartialAssign) : litTrueP σ (Int.ofNat 0) = some false := rfl

/-! ### Basic literal facts -/

theorem litTrue_neg_of_false (α : TotalAssign) (l : Lit) (hnz : l ≠ 0)
    (h : litTrue α l = false) : litTrue α (negLit l) = true := by
  cases l with
  | ofNat n =>
      cases n with
      | zero => exact absurd rfl hnz
      | succ k =>
          rw [negLit_pos, litTrue_neg, litTrue_pos] at *
          rw [h]
          rfl
  | negSucc n =>
      rw [negLit_neg, litTrue_pos, litTrue_neg] at *
      have hf : α (n + 1) = true := by
        cases hc : α (n + 1) with
        | false => rw [hc] at h; exact absurd h (by simp)
        | true => rfl
      rw [hf]

/-- For any literal, truth of `l` forces the assignment at `|l|` to `litPos l`. -/
theorem litTrue_true_imp (α : TotalAssign) (l : Lit) (h : litTrue α l = true) :
    α (litVar l) = litPos l := by
  cases l with
  | ofNat n =>
      cases n with
      | zero => rw [litTrue_zero] at h; exact absurd h (by simp)
      | succ k =>
          rw [litTrue_pos] at h
          rw [litVar_pos, litPos_pos]
          exact h
  | negSucc n =>
      rw [litTrue_neg] at h
      have hf : α (n + 1) = false := by
        cases hc : α (n + 1) with
        | false => rfl
        | true => rw [hc] at h; exact absurd h (by simp)
      rw [litVar_neg, litPos_neg, hf]

theorem litTrueP_none_ne_zero (σ : PartialAssign) (l : Lit) (h : litTrueP σ l = none) :
    l ≠ 0 := by
  cases l with
  | ofNat n =>
      cases n with
      | zero => rw [litTrueP_zero] at h; exact absurd h (by simp)
      | succ k => exact fun hh => Nat.succ_ne_zero k (Int.ofNat.inj hh)
  | negSucc n => exact fun hh => nomatch hh

/-- Agreement lifts a *true* partial literal to a true total one. -/
theorem litTrueP_agree_true {σ : PartialAssign} {α : TotalAssign} (hag : Agrees σ α)
    (l : Lit) (h : litTrueP σ l = some true) : litTrue α l = true := by
  cases l with
  | ofNat n =>
      cases n with
      | zero => rw [litTrueP_zero] at h; exact absurd h (by simp)
      | succ k =>
          rw [litTrueP_pos] at h
          rw [litTrue_pos]
          exact hag (k + 1) true h
  | negSucc n =>
      rw [litTrueP_neg] at h
      rw [litTrue_neg]
      cases hs : σ (n + 1) with
      | none => rw [hs] at h; exact absurd h (by simp)
      | some b =>
          rw [hs] at h
          have hb : b = false := by
            cases b with
            | false => rfl
            | true => exact absurd h (by simp)
          have hf : α (n + 1) = false := by rw [hb] at hs; exact hag (n + 1) false hs
          rw [hf]
          rfl

/-- Agreement lifts a *false* partial literal to a false total one. -/
theorem litTrueP_agree_false {σ : PartialAssign} {α : TotalAssign} (hag : Agrees σ α)
    (l : Lit) (h : litTrueP σ l = some false) : litTrue α l = false := by
  cases l with
  | ofNat n =>
      cases n with
      | zero => rw [litTrue_zero]
      | succ k =>
          rw [litTrueP_pos] at h
          rw [litTrue_pos]
          exact hag (k + 1) false h
  | negSucc n =>
      rw [litTrueP_neg] at h
      rw [litTrue_neg]
      cases hs : σ (n + 1) with
      | none => rw [hs] at h; exact absurd h (by simp)
      | some b =>
          rw [hs] at h
          have hb : b = true := by
            cases b with
            | false => exact absurd h (by simp)
            | true => rfl
          have ht : α (n + 1) = true := by rw [hb] at hs; exact hag (n + 1) true hs
          rw [ht]
          rfl

theorem assign_self (σ : PartialAssign) (l : Lit) :
    assign σ l (litVar l) = some (litPos l) := by
  simp [assign]

theorem assign_other (σ : PartialAssign) (l : Lit) (x : Nat) (h : x ≠ litVar l) :
    assign σ l x = σ x := by
  simp only [assign]
  rw [if_neg h]

theorem litTrueP_assign_self (σ : PartialAssign) (l : Lit) (h : litTrueP σ l = none) :
    litTrueP (assign σ l) l = some true := by
  cases l with
  | ofNat n =>
      cases n with
      | zero => rw [litTrueP_zero] at h; exact absurd h (by simp)
      | succ k =>
          rw [litTrueP_pos]
          exact assign_self σ (Int.ofNat (k + 1))
  | negSucc n =>
      rw [litTrueP_neg]
      have ha : assign σ (Int.negSucc n) (n + 1) = some false :=
        assign_self σ (Int.negSucc n)
      rw [ha]
      rfl

/-- Assignment preserves agreement with a total assignment that already has the
    forced value at the assigned variable. -/
theorem Agrees.assign {σ : PartialAssign} {α : TotalAssign} {l : Lit}
    (hag : Agrees σ α) (hα : α (litVar l) = litPos l) : Agrees (assign σ l) α := by
  intro x b hx
  by_cases h : x = litVar l
  · rw [h] at hx
    rw [assign_self] at hx
    have hb : b = litPos l := (Option.some.inj hx).symm
    rw [h, hα, hb]
  · rw [assign_other σ l x h] at hx
    exact hag x b hx

/-! ### Clause scanning (§6.1) -/

/-- Status of one clause under a partial assignment. -/
inductive CSt where
  | sat
  | confl
  | undef
  | unit (l : Lit)
  deriving DecidableEq

/-- Deterministic single-pass classification of a clause. -/
def scan (σ : PartialAssign) : Clause → CSt
  | [] => .confl
  | l :: ls =>
      match litTrueP σ l with
      | some true => .sat
      | some false => scan σ ls
      | none =>
          match scan σ ls with
          | .confl => .unit l
          | .sat => .sat
          | _ => .undef

theorem scan_sat {σ : PartialAssign} {C : Clause} (h : scan σ C = .sat) :
    ∃ l, l ∈ C ∧ litTrueP σ l = some true := by
  induction C with
  | nil => simp [scan] at h
  | cons l ls ih =>
      simp only [scan] at h
      cases htp : litTrueP σ l with
      | none =>
          rw [htp] at h
          cases hs : scan σ ls with
          | confl => rw [hs] at h; exact absurd h (fun hh => nomatch hh)
          | sat => rw [hs] at h; obtain ⟨m, hm, hm2⟩ := ih hs; exact ⟨m, by simp [hm], hm2⟩
          | undef => rw [hs] at h; exact absurd h (fun hh => nomatch hh)
          | unit m => rw [hs] at h; exact absurd h (fun hh => nomatch hh)
      | some b =>
          cases b with
          | true => rw [htp] at h; exact ⟨l, by simp, htp⟩
          | false => rw [htp] at h; obtain ⟨m, hm, hm2⟩ := ih h; exact ⟨m, by simp [hm], hm2⟩

theorem scan_conflict {σ : PartialAssign} {C : Clause} (h : scan σ C = .confl) :
    ∀ l, l ∈ C → litTrueP σ l = some false := by
  induction C with
  | nil => intro l hl; simp at hl
  | cons l ls ih =>
      simp only [scan] at h
      cases htp : litTrueP σ l with
      | none =>
          rw [htp] at h
          cases hs : scan σ ls with
          | confl => rw [hs] at h; exact absurd h (fun hh => nomatch hh)
          | sat => rw [hs] at h; exact absurd h (fun hh => nomatch hh)
          | undef => rw [hs] at h; exact absurd h (fun hh => nomatch hh)
          | unit m => rw [hs] at h; exact absurd h (fun hh => nomatch hh)
      | some b =>
          cases b with
          | true => rw [htp] at h; exact absurd h (fun hh => nomatch hh)
          | false =>
              rw [htp] at h
              intro m hm
              rcases List.mem_cons.mp hm with rfl | hm'
              · exact htp
              · exact ih h m hm'

theorem scan_unit {σ : PartialAssign} {C : Clause} {l : Lit} (h : scan σ C = .unit l) :
    l ∈ C ∧ litTrueP σ l = none ∧ ∀ m, m ∈ C → m ≠ l → litTrueP σ m = some false := by
  induction C with
  | nil => simp [scan] at h
  | cons l' ls ih =>
      simp only [scan] at h
      cases htp : litTrueP σ l' with
      | some b =>
          cases b with
          | true => rw [htp] at h; exact absurd h (fun hh => nomatch hh)
          | false =>
              rw [htp] at h
              obtain ⟨hmem, hnone, hfalse⟩ := ih h
              refine ⟨List.mem_cons_of_mem _ hmem, hnone, ?_⟩
              intro m hm hne
              rcases List.mem_cons.mp hm with rfl | hm'
              · exact htp
              · exact hfalse m hm' hne
      | none =>
          rw [htp] at h
          cases hs : scan σ ls with
          | confl =>
              rw [hs] at h
              have hl : l' = l := CSt.unit.inj h
              subst hl
              refine ⟨List.mem_cons_self .., htp, ?_⟩
              intro m hm hne
              rcases List.mem_cons.mp hm with rfl | hm'
              · exact absurd rfl hne
              · exact scan_conflict hs m hm'
          | sat => rw [hs] at h; exact absurd h (fun hh => nomatch hh)
          | undef => rw [hs] at h; exact absurd h (fun hh => nomatch hh)
          | unit m => rw [hs] at h; exact absurd h (fun hh => nomatch hh)

/-- Does some clause of the CNF conflict under `σ`? -/
def hasConflict (σ : PartialAssign) : CNF → Bool
  | [] => false
  | C :: F => match scan σ C with
              | .confl => true
              | _ => hasConflict σ F

theorem hasConflict_sound {σ : PartialAssign} {F : CNF} (h : hasConflict σ F = true) :
    ∃ C, C ∈ F ∧ scan σ C = .confl := by
  induction F with
  | nil => simp [hasConflict] at h
  | cons C F ih =>
      simp only [hasConflict] at h
      cases hs : scan σ C with
      | confl => exact ⟨C, by simp, hs⟩
      | sat => rw [hs] at h; obtain ⟨D, hD, hDc⟩ := ih h; exact ⟨D, by simp [hD], hDc⟩
      | undef => rw [hs] at h; obtain ⟨D, hD, hDc⟩ := ih h; exact ⟨D, by simp [hD], hDc⟩
      | unit m => rw [hs] at h; obtain ⟨D, hD, hDc⟩ := ih h; exact ⟨D, by simp [hD], hDc⟩

/-- The first unit clause of the CNF, in list order. -/
def findUnit (σ : PartialAssign) : CNF → Option Lit
  | [] => none
  | C :: F => match scan σ C with
              | .unit l => some l
              | _ => findUnit σ F

theorem findUnit_sound {σ : PartialAssign} {F : CNF} {l : Lit}
    (h : findUnit σ F = some l) : ∃ C, C ∈ F ∧ scan σ C = .unit l := by
  induction F with
  | nil => simp [findUnit] at h
  | cons C F ih =>
      simp only [findUnit] at h
      cases hs : scan σ C with
      | unit m =>
          rw [hs] at h
          have hm : m = l := Option.some.inj h
          subst hm
          exact ⟨C, by simp, hs⟩
      | confl => rw [hs] at h; obtain ⟨D, hD, hDc⟩ := ih h; exact ⟨D, by simp [hD], hDc⟩
      | sat => rw [hs] at h; obtain ⟨D, hD, hDc⟩ := ih h; exact ⟨D, by simp [hD], hDc⟩
      | undef => rw [hs] at h; obtain ⟨D, hD, hDc⟩ := ih h; exact ⟨D, by simp [hD], hDc⟩

/-- §6.1. Outcome of deterministic unit propagation. -/
inductive Outcome where
  | confl
  | fix (σ : PartialAssign)

/-- Number of literal occurrences in a CNF (the fuel bound used to define `Rup`). -/
def litCount : CNF → Nat
  | [] => 0
  | C :: F => C.length + litCount F

/-- §6.1. Deterministic unit propagation with explicit fuel. -/
def propagate : Nat → CNF → PartialAssign → Outcome
  | 0, _, σ => .fix σ
  | n + 1, F, σ =>
      if hasConflict σ F = true then .confl
      else
        match findUnit σ F with
        | some l => propagate n F (assign σ l)
        | none => .fix σ

/-! ### Lemma 6.2, the unit-propagation invariant -/

/-- If a clause is unit under `σ` and satisfied by a total `α` that agrees with
    `σ`, then `α` already takes the forced value at the unit literal. -/
theorem unit_forces {σ : PartialAssign} {α : TotalAssign} {C : Clause} {l : Lit}
    (hs : scan σ C = .unit l) (hC : CSat α C) (hag : Agrees σ α) :
    α (litVar l) = litPos l := by
  obtain ⟨hmem, hnone, hfalse⟩ := scan_unit hs
  obtain ⟨m, hm, hmt⟩ := hC
  by_cases hml : m = l
  · rw [hml] at hmt
    exact litTrue_true_imp α l hmt
  · have hf : litTrue α m = false := litTrueP_agree_false hag m (hfalse m hm hml)
    rw [hf] at hmt
    exact absurd hmt (by simp)

/-- §6.1. **Propagation invariant.**  If `α` satisfies the clause set and agrees
    with the current partial assignment, propagation (at any fuel) cannot reach
    a conflict. -/
theorem propagate_no_conflict (F : CNF) (α : TotalAssign) :
    ∀ (fuel : Nat) (σ : PartialAssign), FSat α F → Agrees σ α →
      propagate fuel F σ ≠ .confl := by
  intro fuel
  induction fuel with
  | zero => intro σ hF hag h; exact absurd h (fun hh => nomatch hh)
  | succ fuel ih =>
      intro σ hF hag h
      simp only [propagate] at h
      by_cases hc : hasConflict σ F = true
      · rw [if_pos hc] at h
        exfalso
        obtain ⟨C, hC, hCs⟩ := hasConflict_sound hc
        obtain ⟨m, hm, hmt⟩ := hF C hC
        have hf : litTrue α m = false := litTrueP_agree_false hag m (scan_conflict hCs m hm)
        rw [hf] at hmt
        exact absurd hmt (by simp)
      · rw [if_neg hc] at h
        cases hf : findUnit σ F with
        | some l =>
            rw [hf] at h
            obtain ⟨C, hC, hCs⟩ := findUnit_sound hf
            have hα : α (litVar l) = litPos l := unit_forces hCs (hF C hC) hag
            exact ih (assign σ l) hF (Agrees.assign hag hα) h
        | none =>
            rw [hf] at h
            exact absurd h (fun hh => nomatch hh)

/-! ### RUP (§6.2) -/

/-- §6.2. `Rup F C` -- `C` is a reverse-unit-propagation consequence of `F`. -/
def Rup (F : CNF) (C : Clause) : Prop :=
  propagate (litCount F + C.length + 1) (F ++ C.map (fun l => [negLit l]))
    (fun _ => none) = .confl

/-- **Lemma 6.2 — RUP is sound.** -/
theorem rup_sound {F : CNF} {C : Clause} (α : TotalAssign)
    (hrup : Rup F C) (hnz : ∀ l, l ∈ C → l ≠ 0) (hF : FSat α F) : CSat α C := by
  by_cases hnC : CSat α C
  · exact hnC
  · exfalso
    let G : CNF := F ++ C.map (fun l => [negLit l])
    have hFG : FSat α G := by
      intro D hD
      have hD' : D ∈ F ++ C.map (fun l => [negLit l]) := hD
      rcases List.mem_append.mp hD' with hD | hD
      · exact hF D hD
      · obtain ⟨l, hl, rfl⟩ := List.mem_map.mp hD
        refine ⟨negLit l, by simp, ?_⟩
        have hlf : litTrue α l = false := by
          cases hlt : litTrue α l with
          | false => rfl
          | true => exact absurd ⟨l, hl, hlt⟩ hnC
        exact litTrue_neg_of_false α l (hnz l hl) hlf
    have hag : Agrees (fun _ => none) α := by
      intro x b hx
      exact absurd hx (by simp)
    exact (propagate_no_conflict G α (litCount F + C.length + 1) (fun _ => none) hFG hag)
      hrup

/-! ## Part 3 — LRAT steps, refutation soundness (§6.3) -/

namespace Lrat

/-- A live clause together with its LRAT id. -/
abbrev Live := List (Nat × Clause)

/-- The clause set currently live. -/
def Live.cnf (L : Live) : CNF := L.map (fun p => p.2)

def Live.insert (i : Nat) (C : Clause) (L : Live) : Live := (i, C) :: L

def Live.delete (i : Nat) (L : Live) : Live := L.filter (fun p => p.1 != i)

/-- Every literal of the clause is a legal (non-zero) literal. -/
def Legal (C : Clause) : Prop := ∀ l, l ∈ C → l ≠ 0

/-- The clause is not tautological (no literal and its negation). -/
def NonTaut (C : Clause) : Prop := ∀ l, l ∈ C → negLit l ∉ C

/-- The id is not already used by a live clause. -/
def Fresh (i : Nat) (L : Live) : Prop := ∀ C, (i, C) ∉ L

/-- §6.3. LRAT records. -/
inductive Step where
  | add (id : Nat) (C : Clause)
  | del (id : Nat)

/-- The deterministic LRAT state machine (the state is the live clause set). -/
def run : List Step → Live → Live
  | [], L => L
  | .add i C :: ss, L => run ss (Live.insert i C L)
  | .del i :: ss, L => run ss (Live.delete i L)

/-- §6.3. An *accepted* run: `Accepted ss L L'` means the records `ss` are all
    legal when started from the live set `L`, and end in `L'`.  An addition
    needs a non-zero, non-tautological, freshly numbered clause that is RUP with
    respect to the live set; a deletion needs a live id. -/
inductive Accepted : List Step → Live → Live → Prop where
  | nil (L : Live) : Accepted [] L L
  | add (ss : List Step) (L : Live) (L' : Live) (i : Nat) (C : Clause) :
      Rup L.cnf C → Legal C → NonTaut C → Fresh i L →
      Accepted ss (Live.insert i C L) L' → Accepted (.add i C :: ss) L L'
  | del (ss : List Step) (L : Live) (L' : Live) (i : Nat) :
      (∃ C, (i, C) ∈ L) →
      Accepted ss (Live.delete i L) L' → Accepted (.del i :: ss) L L'

theorem cnf_insert (i : Nat) (C : Clause) (L : Live) :
    (Live.insert i C L).cnf = C :: L.cnf := rfl

theorem cnf_delete_mem {i : Nat} {L : Live} {D : Clause}
    (h : D ∈ (Live.delete i L).cnf) : D ∈ L.cnf := by
  obtain ⟨p, hp, rfl⟩ := List.mem_map.mp h
  obtain ⟨hpL, _⟩ := List.mem_filter.mp hp
  exact List.mem_map.mpr ⟨p, hpL, rfl⟩

/-- `Accepted` computes the same state as `run`. -/
theorem accepted_run {ss : List Step} {L L' : Live} (h : Accepted ss L L') :
    run ss L = L' := by
  induction h with
  | nil L => rfl
  | add ss L L' i C hrup hleg hnt hfresh hrest ih => exact ih
  | del ss L L' i hlive hrest ih => exact ih

/-- **Lemma 6.3, invariant.**  A model of the live set survives every accepted
    record: an addition is satisfied because it is a RUP consequence of the live
    set, and a deletion only shrinks the set. -/
theorem accepted_models (α : TotalAssign) {ss : List Step} {L L' : Live}
    (h : Accepted ss L L') (hα : FSat α L.cnf) : FSat α L'.cnf := by
  induction h with
  | nil L => exact hα
  | add ss L L' i C hrup hleg hnt hfresh hrest ih =>
      apply ih
      rw [cnf_insert]
      intro D hD
      rcases List.mem_cons.mp hD with rfl | hD
      · exact rup_sound α hrup hleg hα
      · exact hα D hD
  | del ss L L' i hlive hrest ih =>
      apply ih
      intro D hD
      exact hα D (cnf_delete_mem hD)

/-- **Lemma 6.3 — LRAT refutation soundness.**  If an accepted run of `F`
    finishes with the empty clause live, then `F` is unsatisfiable.  The proof
    tracks a model of the *original* formula: it is never claimed that deletion
    preserves unsatisfiability. -/
theorem refutation_sound {F : CNF} {L₀ : Live} (hinit : L₀.cnf = F)
    {ss : List Step} {L : Live} (hacc : Accepted ss L₀ L)
    (hempty : ([] : Clause) ∈ L.cnf) : Unsat F := by
  intro hsat
  obtain ⟨α, hα⟩ := hsat
  have hα₀ : FSat α L₀.cnf := by rw [hinit]; exact hα
  have hempty' : CSat α [] := accepted_models α hacc hα₀ [] hempty
  obtain ⟨l, hl, _⟩ := hempty'
  simp at hl

end Lrat

/-! ## Part 4 — certificate DAG, Theorem 1 (§8–§10) -/

namespace Cert

/-- §7/G1. The bit-blaster is *uninterpreted*: this development says nothing
    about its implementation.  Its soundness is the hypothesis `blast_sound`
    below; it is never declared as an axiom. -/
opaque blast : Term → CNF

/-- §8. One certificate node.  Premises are *indices of earlier steps*; a step
    is only ever checked against the prefix that precedes it, so acyclicity is
    obtained by construction (no id map, no graph traversal). -/
inductive Step where
  | refl (id : Nat) (n : Nat) (t : Term)
  | sym (id p : Nat) (n : Nat) (a b : Term)
  | trans (id p q : Nat) (n : Nat) (a b c : Term)
  | congr (id p : Nat) (op : Bool) (n : Nat) (a b t : Term) (slot : Bool)
  | bitblast (id : Nat) (P : Term)

/-- §8. Premise indices of a step, in the order the rule expects them. -/
def Step.premises : Step → List Nat
  | .refl _ _ _ => []
  | .sym _ p _ _ _ => [p]
  | .trans _ p q _ _ _ _ => [p, q]
  | .congr _ p _ _ _ _ _ _ => [p]
  | .bitblast _ _ => []

/-- §8/§9. The conclusion drawn by a step.  `slot = true` replaces the first
    operand of the binary operator, `slot = false` the second. -/
def Step.conclusion : Step → Term
  | .refl _ n t => .eq n t t
  | .sym _ _ n a b => .eq n b a
  | .trans _ _ _ n a _ c => .eq n a c
  | .congr _ _ op n a b t slot =>
      match slot with
      | true => .eq n (Term.mkBin op n a t) (Term.mkBin op n b t)
      | false => .eq n (Term.mkBin op n t a) (Term.mkBin op n t b)
  | .bitblast _ P => P

/-- §9. Side conditions of a rule, checked against the earlier steps `earlier`.
    `refl` has no side condition because its conclusion is `eq n t t` by
    construction.  `congr` requires exactly one operand to be replaced and the
    other (`t`) to be syntactically identical on both sides.  `bitblast` holds
    when an accepted LRAT refutation of `blast (conclusion)` exists (so the
    rule is discharged by Lemma 6.3, not assumed). -/
def Justified (earlier : List Step) : Step → Prop
  | .refl _ _ _ => True
  | .sym _ p n a b => (earlier[p]?).map Step.conclusion = some (.eq n a b)
  | .trans _ p q n a b c =>
      (earlier[p]?).map Step.conclusion = some (.eq n a b) ∧
      (earlier[q]?).map Step.conclusion = some (.eq n b c)
  | .congr _ p _ n a b _ _ => (earlier[p]?).map Step.conclusion = some (.eq n a b)
  | .bitblast _ P =>
      ∃ (L₀ : Lrat.Live) (ss : List Lrat.Step) (L : Lrat.Live),
        Lrat.Live.cnf L₀ = blast P ∧ Lrat.Accepted ss L₀ L ∧ ([] : Clause) ∈ L.cnf

/-- §8/§9. A checked certificate: a list of steps, each justified from the
    prefix that precedes it, each with a well-typed Boolean conclusion. -/
inductive Checked : List Step → Prop where
  | nil : Checked []
  | snoc (ss : List Step) (s : Step) :
      Checked ss →
      (∀ i ∈ s.premises, i < ss.length) →
      HasType s.conclusion .bool →
      Justified ss s →
      Checked (ss ++ [s])

/-- A premise index always resolves to a step of the prefix. -/
theorem exists_of_premise {ss : List Step} {p : Nat} {T : Term}
    (h : (ss[p]?).map Step.conclusion = some T) : ∃ s ∈ ss, s.conclusion = T := by
  cases hg : ss[p]? with
  | none => rw [hg] at h; exact absurd h (by simp)
  | some s =>
      rw [hg] at h
      exact ⟨s, List.mem_of_getElem? hg, Option.some.inj h⟩

/-- Typing and validity transfer from a premise step to the conclusion it names. -/
theorem premise_facts {ss : List Step}
    (ih : ∀ s ∈ ss, HasType s.conclusion .bool ∧ Valid s.conclusion)
    {p : Nat} {T : Term} (h : (ss[p]?).map Step.conclusion = some T) :
    HasType T .bool ∧ Valid T := by
  obtain ⟨s, hmem, hc⟩ := exists_of_premise h
  obtain ⟨hty, hv⟩ := ih s hmem
  rw [hc] at hty hv
  exact ⟨hty, hv⟩

/-- §10. **Induction invariant.**  In a checked certificate every step's
    conclusion is a well-typed Boolean term and is valid. -/
theorem checked_sound (blast_sound : ∀ P, HasType P .bool → Unsat (blast P) → Valid P) :
    ∀ {ss : List Step}, Checked ss →
      ∀ s ∈ ss, HasType s.conclusion .bool ∧ Valid s.conclusion := by
  intro ss h
  induction h with
  | nil => intro s hs; simp at hs
  | snoc ss s hchk hpre hty hok ih =>
      intro x hx
      rcases List.mem_append.mp hx with hx | hx
      · exact ih x hx
      · have hxs : x = s := List.mem_singleton.mp hx
        rw [hxs]
        refine ⟨hty, ?_⟩
        cases s with
        | refl id n t =>
            cases hty with
            | eq ha hb =>
                show Valid (.eq n t t)
                exact (valid_eq_iff ha hb).mpr (fun _ => rfl)
        | sym id p n a b =>
            cases hty with
            | eq hb ha =>
                obtain ⟨hpty, hpv⟩ := premise_facts ih hok
                cases hpty with
                | eq ha' hb' =>
                    show Valid (.eq n b a)
                    rw [valid_eq_iff hb ha]
                    intro ρ
                    exact ((valid_eq_iff ha' hb').mp hpv ρ).symm
        | trans id p q n a b c =>
            cases hty with
            | eq ha hc =>
                obtain ⟨h1ty, h1v⟩ := premise_facts ih hok.1
                obtain ⟨h2ty, h2v⟩ := premise_facts ih hok.2
                cases h1ty with
                | eq ha' hb' =>
                    cases h2ty with
                    | eq hb'' hc' =>
                        show Valid (.eq n a c)
                        rw [valid_eq_iff ha hc]
                        intro ρ
                        exact ((valid_eq_iff ha' hb').mp h1v ρ).trans
                          ((valid_eq_iff hb'' hc').mp h2v ρ)
        | congr id p op n a b t slot =>
            obtain ⟨hpty, hpv⟩ := premise_facts ih hok
            cases hpty with
            | eq ha hb =>
                have hv : ∀ ρ : Nat → Nat, eval ρ a = eval ρ b :=
                  (valid_eq_iff ha hb).mp hpv
                cases slot with
                | true =>
                    cases hty with
                    | eq hL hR =>
                        show Valid (.eq n (Term.mkBin op n a t) (Term.mkBin op n b t))
                        rw [valid_eq_iff hL hR]
                        intro ρ
                        cases op with
                        | true => simp [Term.mkBin, eval, hv ρ]
                        | false => simp [Term.mkBin, eval, hv ρ]
                | false =>
                    cases hty with
                    | eq hL hR =>
                        show Valid (.eq n (Term.mkBin op n t a) (Term.mkBin op n t b))
                        rw [valid_eq_iff hL hR]
                        intro ρ
                        cases op with
                        | true => simp [Term.mkBin, eval, hv ρ]
                        | false => simp [Term.mkBin, eval, hv ρ]
        | bitblast id P =>
            obtain ⟨L₀, ssL, L, hinit, hacc, hempty⟩ := hok
            exact blast_sound P hty (Lrat.refutation_sound hinit hacc hempty)

/-- **Theorem 1 (abstract soundness).**  Given the blast lemma as a hypothesis,
    if the certificate `ss ++ [root]` is checked then the root's conclusion is
    valid. -/
theorem theorem1 (blast_sound : ∀ P, HasType P .bool → Unsat (blast P) → Valid P)
    {ss : List Step} {root : Step} (h : Checked (ss ++ [root])) :
    Valid root.conclusion :=
  ((checked_sound blast_sound h) root (List.mem_append_right _ (List.mem_singleton_self _))).2

end Cert

/-! ### Sanity checks (non-vacuity of the definitions above) -/

namespace Lrat

/-- A concrete refutation: `{x, ¬x}` is refuted by adding the empty clause.
    This exercises the whole Part 2 → Part 3 pipeline on a closed instance. -/
example : Unsat [[1], [-1]] := by
  refine refutation_sound (F := [[1], [-1]]) (L₀ := [(0, [1]), (1, [-1])])
    (ss := [.add 2 []]) (L := [(2, [])] ++ [(0, [1]), (1, [-1])]) rfl ?_ ?_
  · refine Accepted.add [] [(0, [1]), (1, [-1])] ([(2, [])] ++ [(0, [1]), (1, [-1])])
      2 [] ?_ ?_ ?_ ?_ (Accepted.nil _)
    · rfl
    · intro l hl; exact absurd hl (by simp)
    · intro l hl; exact absurd hl (by simp)
    · intro C hC
      simp at hC
  · exact List.mem_map.mpr ⟨(2, []), List.mem_cons_self .., rfl⟩

end Lrat

namespace Cert

/-- A concrete certificate: the `refl` rule proves `x = x` at width 4. -/
example (blast_sound : ∀ P, HasType P .bool → Unsat (blast P) → Valid P) :
    Valid (Term.eq 4 (.var 4 0) (.var 4 0)) :=
  theorem1 blast_sound (ss := [])
    (Checked.snoc [] (.refl 0 4 (.var 4 0)) Checked.nil
      (by intro i hi; simp [Step.premises] at hi)
      (HasType.eq (HasType.var 0 (by decide) (by decide))
                  (HasType.var 0 (by decide) (by decide)))
      trivial)

end Cert

#print axioms hasConflict_sound
#print axioms propagate_no_conflict
#print axioms rup_sound
#print axioms Lrat.accepted_models
#print axioms Lrat.refutation_sound
#print axioms Cert.theorem1
