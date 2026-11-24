# TNPS logical core — specification (Milestone B, IX.43)

- **Repository:** the `tekmerion` source tree (paths below are repository-relative).
- **Scope:** `src/native/{logic,bitblast,lrat,cert,check}.rs`
- **Status:** this document specifies the *abstract* system. It does **not** claim
  that the Rust implementation is verified. The gap between the two is named in
  §12 and measured in `research/receipts/ix43-tcb.md`.
- **Rule of reading:** every "is" below refers to the abstract relation. Where
  the executable deviates, §12 says so.

---

## 0. Conventions

- `ℕ = {0,1,2,…}`; `𝔹 = {0,1}`.
- `x`, `y`, `z` range over variable names; `n`, `m`, `k` over widths; `i`, `j`
  over step/clause identifiers.
- A *valuation* is a map `ρ : Name → ℕ`. The implementation's `eval` takes a
  partial map and returns `UnboundVariable` on a missing name; the mathematical
  relation used for validity is total.
- `⟦t⟧_ρ` is the value of term `t` under `ρ` (§4); `ρ ⊨ P` is defined in §4.
- `2^w` is exact integer exponentiation; word arithmetic is modulo `2^w`.
- ASCII space is written `SP`; line feed is `NL`.

Widths are always in `1..=64`. Zero-width and wider-than-64 words are not part of
the system; they are refused, not truncated.

---

## 1. Grammar

### 1.1 Terms

```
term    ::= "true"
          | "false"
          | "(" "not" SP term ")"
          | "(" "eq"    SP width SP term SP term ")"
          | "(" "add"   SP width SP term SP term ")"
          | "(" "sub"   SP width SP term SP term ")"
          | "(" "var"   SP width SP name ")"
          | "(" "const" SP width SP literal ")"

width   ::= "bv" decimal
name    ::= name-start name-char*
name-start ::= "A".."Z" | "a".."z" | "_"
name-char  ::= name-start | "0".."9"
decimal ::= "0" | nonzero digit*
literal ::= decimal
```

`nonzero` is `1`..`9`, `digit` is `0`..`9`.

### 1.2 Canonical form of terms

A term string `s` is *canonical* iff `s = render(t)` for exactly one `t`:

1. exactly one ASCII space between tokens, no leading or trailing space;
2. no leading zero in a width (`bv08` is not canonical) and no `+` sign;
3. `const` width `w` and value `v` satisfy `v < 2^w` (for `w = 64`, any `u64`);
4. `var` names satisfy `name` above;
5. the `width` annotation of `eq`, `add`, `sub` equals the width of both
   operands (a consequence of typing, §3).

The parser is a *recogniser of this canonical language*: it parses, then re-renders
and requires byte equality with the input. Thus whitespace variants, redundant
parentheses, mixed case, and leading zeros are all rejected rather than
normalised. (The `digest` of a term is computed over its structure, not its
rendering; see §1.6.)

### 1.3 CNF / DIMACS

```
dimacs  ::= "p cnf " decimal SP decimal NL clause*
clause  ::= "0" NL | lit (SP lit)* SP "0" NL
lit     ::= "-"? nonzero digit*
```

The empty clause is therefore the line `0`; there is no double space and no
leading space.

Canonical DIMACS additionally requires:

- exactly one ASCII space between literals and immediately before `0`;
- the declared variable count `n` is the number of Tseitin variables allocated by
  the blaster (`bb.next - 1`), not merely `max |lit|`;
- every `|lit| ≤ n`;
- exactly the declared number `m` of clause lines;
- no empty lines, no comment lines, no trailing bytes after the final `NL`;
- an empty clause is rendered `0`, never `" 0"` or `0 `.

### 1.4 LRAT

```
lrat        ::= record+
record      ::= addition | deletion | annotated-deletion
addition    ::= decimal SP clause-part SP hint-part "0" NL
clause-part ::= "0" | lit (SP lit)* SP "0"
hint-part   ::= "" | hint (SP hint)* SP
deletion    ::= "d" SP decimal (SP decimal)* SP "0" NL
annotated-deletion ::= decimal SP "d" SP decimal (SP decimal)* SP "0" NL
hint        ::= decimal | "-" decimal
lit         ::= "-"? nonzero digit*
```

For example, `12 -3 4 0 7 9 0` adds clause `{-3,4}` with hints `7,9`, and
`12 0 0` adds the empty clause with no hints. Every separator is one ASCII
space; a missing, doubled, or trailing separator is not canonical.

Canonical integers have no leading zeros, no `+`, and `-0` is not an integer.
Clause literals must be non-zero and within the DIMACS variable range; repeated
literals and tautological clauses are refused. Clause ids in additions must be
fresh and strictly greater than every id used by an addition so far (original
clauses are `1..=m`). `d` deletes existing, undeleted ids; deleting an absent id
is refused.
The annotated form is CaDiCaL's `<last-added-id> d <ids> 0` and is a deletion,
not an addition.

### 1.5 Certificates

```
certificate ::= magic NL
                "theorem " id NL
                step*
                blob*
                "end" NL

step ::= "step " id SP rule SP premise-field SP term NL
rule ::= "refl" | "sym" | "trans" | "congr" | "bitblast"
premise-field ::= "-"                       (refl)
                | id                        (sym, congr)
                | id SP id                  (trans)
                | digest SP digest          (bitblast)

blob ::= "blob " ("cnf"|"lrat") SP digest SP decimal SP hex NL
magic ::= "tekmerion-native-cert v1"
digest ::= 64 lowercase hex characters
hex ::= two lowercase hex characters per payload byte
```

Canonical form:

1. `theorem` names a step id that exists;
2. steps are rendered in strictly ascending id order; ids are unique;
3. blobs are sorted by `(kind, digest)` with `cnf < lrat`; digests are unique;
4. each blob's declared length equals its payload length, and its declared digest
   equals `digest_bytes(payload)`;
5. every referenced digest has a blob of the matching kind, and every blob is
   referenced by at least one step;
6. `end` is the final line; a trailing line, a second `end`, or a missing final
   `NL` is refused;
7. the conclusion of every step is a well-typed Boolean term (§3);
8. the whole string is exactly the canonical rendering, byte for byte.

Premise references may point forward; the graph must be acyclic (§8).

### 1.6 Digests

`digest_bytes(b)` is SHA-256 over a domain-separated, length-prefixed encoding
(`tekmerion/native/blob/v1`). A term digest is domain-separated by a constructor
tag; widths are 4-byte little-endian, constants 8-byte little-endian, names are
UTF-8 bytes, child digests are 32 raw bytes, and every field written by
`Canon::bytes` carries an 8-byte little-endian length prefix
(`tekmerion/native/term/v1`). Digests name content; they never establish truth.

### 1.7 Parser specification (canonical-language refinement)

This subsection is normative for `Term::parse`, `Certificate::parse`,
`Cnf::parse_dimacs` and the LRAT reader. It turns the grammars above into a
single precise obligation instead of a set of "did we reject the right things"
tests.

**Input domain.** The decoders consume a byte string. Only ASCII is legal:
any byte `>= 0x80` is refused before tokenising. The Rust entry points take
`&str`; that is a *precondition*, not a guarantee — a non-UTF-8 byte string can
never reach a decoder, and this is recorded as an assumption (A8 below), not as
parser logic. The abstract relation is stated over `Bytes = List u8` and
therefore does not depend on that precondition.

**Token decoders.** Each token class denotes a partial function:

```
dec_decimal : Bytes ⇀ Nat
  accepted iff  s = "0"  or  s = d₀ d₁ … dₖ with d₀ ∈ 1..9 and dᵢ ∈ 0..9
  value        =  Σ dᵢ · 10^(k−i)         (computed, never wrapped)

dec_hex : Bytes ⇀ Bytes
  accepted iff  s = 2n lowercase hex digits, n ≥ 0
  value        =  the n payload bytes

SP = 0x20,  NL = 0x0A;  CR, TAB and every other separator are refused.
```

**Numeric narrowing is a refusal, not a conversion.** A `decimal` used as a
width must satisfy `1 ≤ v ≤ 64`; as a clause/step id or a variable count it must
satisfy `v < 2^32`; as a DIMACS literal it must satisfy `1 ≤ v ≤ n_vars`; as an
`lrat` hint it must satisfy `v < 2^32`. A decimal whose mathematical value
exceeds the target bound is `Err`, never a truncation, wraparound or saturating
conversion. This is the clause that the earlier `IWORDIFY` failure mode lacked
in the HOL path (a Boolean became a numeral and a carry constraint was lost);
here it is a typing rule on the decoder.

**The parse relation.** For each artefact `A ∈ {Term, Cnf, Lrat, Certificate}`
there is a partial function

```
parse_A : Bytes ⇀ A
```

It is defined by the grammar of §1.1/§1.3/§1.4/§1.5 together with the token
decoders above. Because the grammar is LL(1) after the first token and every
production consumes a distinct prefix, `parse_A` is **deterministic**: at most
one `a` satisfies `parse_A s = Some a`.

**Rendering.** `render_A : A → Bytes` is the canonical printer of §1.2/§1.3/
§1.4/§1.5 (exactly one space between tokens, no trailing space, ascending step
ids, sorted blobs, `end\n` last).

**Refinement is a round-trip, not a lookalike.** The implementation does not
reimplement the grammar as a set of accept/reject checks; it parses, re-renders
and requires byte equality with the input. The specification therefore reduces
the whole parser to four properties:

```
(P1) soundness      parse_A s = Some a            ⇒  render_A a = s
(P2) completeness   (∀ a : A)                    ⇒  parse_A (render_A a) = Some a
(P3) injectivity    render_A a = render_A a'     ⇒  a = a'
(P4) totality       parse_A s = None  ⇔  ¬ ∃ a, render_A a = s
```

(P1) alone is what the `parse → render → compare` discipline enforces at
runtime: an accepted artefact is *its own* canonical rendering. (P1)+(P3)
make acceptance equivalent to membership in `image(render_A)`; (P2)+(P4) make
that image exactly the parsable set. Together they say:

> **Parser theorem.** `parse_A` accepts exactly the canonical language of the
> artefact and nothing else, and acceptance is byte-for-byte verifiable.

**The gap this leaves (G2, restated precisely).** (P1)–(P4) are properties of
the abstract relation. The open obligation is the *refinement*:

```
(G2)   ∀ s,  Term::parse(s)   = Ok a  ⇔  parse_Term s = Some a
            Term::parse(s)   = Err e ⇔  parse_Term s = None
       (and identically for the other three artefact types)
```

(P1) is cheap to discharge *by construction*: the executable already tests it
on every input because it re-renders and compares. (P2)/(P3) are grammar
lemmas. (P4) is the hard half — it is the statement that no non-canonical byte
string slips through, and it is what fuzzing can only sample, never prove.

**Fail-closed rejection table.** The following are `Err`, with no repair and no
defaulting. This list is the conformance surface for `tests/native.rs` and
`tests/native_semantics.rs`:

| class | examples |
| --- | --- |
| encoding | non-ASCII byte; `CR`/`TAB`; doubled or missing `SP`; trailing `SP`; missing final `NL`; second `end`; bytes after `end`; empty line; comment line |
| integers | leading zero (`bv08`, `012`); `+` sign; `-0`; value `> 2^32` in a count/id/hint; width outside `1..=64`; literal `>= 2^w` |
| literals | `0` inside a clause; `\|lit\| > n_vars`; repeated literal; tautological clause |
| ids | duplicate step id; non-ascending step id (certificate); clause id not strictly greater than every earlier addition |
| structure | wrong premise count for a rule; unknown rule; unknown blob kind; deleted id absent or already deleted; hint naming a non-live clause; records after the empty clause; root id absent; cyclic premises |
| hashes | digest not 64 lowercase hex; declared length ≠ payload length; odd-length hex payload; declared digest ≠ `digest_bytes(payload)` |
| coverage | a referenced digest without a blob; a blob no step references |

**A8 — UTF-8 precondition.** The decoders are only reachable with a `&str`. A
caller that hands a `&[u8]` must validate UTF-8 first; the checker's own API
does not expose such a path. Recorded here because (P4) is stated over `Bytes`
and the executable is stated over `&str`.

---

## 2. Sorts

```
Sort ::= Bool | BitVec(n),  n ∈ 1..=64
```

`Value ::= Bool(b) | Bits(v)`, with `0 ≤ v < 2^n` for `Bits` at sort `BitVec(n)`.

---

## 3. Term typing

Judgements have the form `⊢ t : Sort`. There is no variable environment: every
`var` carries its own width, so typing is local to the term. A term is well typed
iff the following rules derive it; `Term::validate` implements exactly these
rules.

```
──────────────           ──────────────
⊢ true : Bool            ⊢ false : Bool

⊢ t : Bool
──────────────
⊢ not t : Bool

⊢ a : BitVec(n)    ⊢ b : BitVec(n)    1 ≤ n ≤ 64
────────────────────────────────────────────────
⊢ eq bv n a b : Bool

⊢ a : BitVec(n)    ⊢ b : BitVec(n)    1 ≤ n ≤ 64
────────────────────────────────────────────────
⊢ add bv n a b : BitVec(n)

⊢ a : BitVec(n)    ⊢ b : BitVec(n)    1 ≤ n ≤ 64
────────────────────────────────────────────────
⊢ sub bv n a b : BitVec(n)

n ∈ 1..=64
────────────────────────
⊢ (var bv n x) : BitVec(n)

n ∈ 1..=64    v < 2^n
──────────────────────────────
⊢ (const bv n v) : BitVec(n)
```

There is no subtyping, no coercion, no implicit width promotion, and no
overloading. `not` applied to a word, `eq`/`add`/`sub` applied to Booleans, or
operands of different widths make the term ill typed and the certificate is
refused.

**Name discipline (deliberate).** Typing is local: the same name may occur at two
different widths in one term. The evaluator (§5) looks a name up once and masks
the value at each occurrence, while the bit-blaster keys variables by
`(name, width)`. Consequently two occurrences at different widths are *correlated*
by evaluation and *independent* in the CNF. Independence only enlarges the set of
assignments considered, so the blast remains sound (UNSAT over the larger set
implies validity over the correlated set); it can make the system incomplete for
such terms. All differential tests use width-consistent names. This is a
boundary of the blast lemma in §12 (G1), not a hidden assumption.

---

## 4. Term semantics

For a total valuation `ρ : Name → ℕ`, `⟦·⟧_ρ` is total and deterministic:

```
⟦true⟧_ρ            = Bool(1)
⟦false⟧_ρ           = Bool(0)
⟦not t⟧_ρ           = Bool(1 - b)              if ⟦t⟧_ρ = Bool(b)
⟦eq bv n a b⟧_ρ     = Bool(⟦a⟧_ρ = ⟦b⟧_ρ)      (word equality after masking)
⟦var bv n x⟧_ρ      = Bits(ρ(x) mod 2^n)
⟦const bv n v⟧_ρ    = Bits(v mod 2^n)
⟦add bv n a b⟧_ρ    = Bits((⟦a⟧_ρ + ⟦b⟧_ρ) mod 2^n)
⟦sub bv n a b⟧_ρ    = Bits((⟦a⟧_ρ - ⟦b⟧_ρ) mod 2^n)
```

For well-typed terms the cases are disjoint by sort. `eval` in the implementation
also returns an error for an unbound variable; the mathematical relation takes a
total `ρ` and is total. Determinism follows because the recursion is structural.

Validity:

```
ρ ⊨ P    iff  ⟦P⟧_ρ = Bool(1)
⊨ P      iff  ρ ⊨ P for every total ρ
```

Only Boolean terms are propositions.

---

## 5. CNF semantics

A CNF assignment is `α : {1,…,n} → 𝔹`.

```
α ⊨ (l₁ ∨ … ∨ l_k)  iff  some l_j is true under α
                     (l is true iff l > 0 and α(l) = 1, or l < 0 and α(-l) = 0)
α ⊨ F               iff  α satisfies every clause of F
F is UNSAT          iff  no α satisfies F
```

The empty clause is satisfied by no assignment.

---

## 6. Unit propagation, RUP, LRAT

### 6.1 Unit propagation

For a clause set `F` and a partial assignment `α`:

- a clause is *unit under α* if exactly one literal is unassigned and all others
  are false;
- a clause is *conflicting under α* if all its literals are false;
- `UP` repeatedly assigns the unique unassigned literal of a unit clause, for
  any choice order, until a fixpoint or a conflict. Unit propagation is
  confluent for conflict detection: if one order reaches conflict, every fair
  order does.

Write `F ⊢_UP ⊥` when propagation from the empty assignment reaches a conflict.

### 6.2 RUP

A clause `C` is a **reverse unit propagation** consequence of `F` iff

```
F ∧ ¬C  ⊢_UP  ⊥
```

where `¬C` is the conjunction of the negations of the literals of `C` (for
`C = ∅`, `¬C` is empty and `F ⊢_UP ⊥` is required).

*Lemma (RUP is sound).* If `C` is RUP w.r.t. `F` and `α ⊨ F`, then `α ⊨ C`.
*Proof.* Suppose `α ⊨ F` and `α ⊭ C`. Then `α` satisfies `F ∧ ¬C`, so `F ∧ ¬C` is
satisfiable. Unit propagation preserves satisfiability: assigning the only
unassigned literal of a unit clause is forced in every model of the current
formula. Hence propagation cannot reach conflict, contradicting RUP. ∎

### 6.3 LRAT step relation

The checker's state is `(F, D, maxid)` where `F` maps live clause ids to clauses,
`D` is the set of deleted ids, and `maxid` is the greatest id seen.

- **Addition** of id `i` with clause `C` and hints `h₁…hₖ`:
  `i > maxid`, `i ∉ dom F ∪ D`, all literals of `C` are in range and `C` is
  non-tautological. The hint sequence must be a *witness*: starting from `¬C`,
  each `h_j` names a live clause that is unit (assigning its unassigned literal,
  which must be fresh) and the final `h_k` names a conflicting clause. Negative
  hints denote RAT and are refused. Then `F' = F[i ↦ C]`, `maxid = i`.
- **Deletion** `d i₁…iₖ 0`: each `i_j` is live and not already deleted;
  `F' = F \ {i_j}`, `D' = D ∪ {i_j}`.
- **Annotated deletion** `<i> d i₁…iₖ 0`: legal only when `i` is the most
  recently added id; it deletes `i₁…iₖ` exactly as above.

A hint-annotated addition satisfying the witness condition implies `C` is RUP
w.r.t. `F`, by recording the propagation order. The converse is not required:
the checker accepts one witness, not every provable RUP clause. This is a
strictness, not a soundness gap.

An LRAT refutation is accepted iff its records are well formed, every addition is
accepted, every deletion is legal, and the **last** addition is the empty clause;
any record after the empty clause is refused.

*Lemma (LRAT soundness).* If an LRAT refutation of `F` is accepted, then `F` is
UNSAT.
*Proof.* Suppose, for contradiction, that `F` has a model `α`. The invariant is
**`α` satisfies every clause live at the current record**. It holds initially.
An addition of `C` is admitted only when `C` is RUP with respect to the live set
`F_live`; by the RUP lemma, `α ⊨ F_live` implies `α ⊨ C`, so the invariant is
restored after inserting `C`. A deletion removes clauses, so any assignment
satisfying the previous live set satisfies the smaller one — the invariant is
preserved. The final live set therefore still contains the empty clause, which
no assignment satisfies; this contradicts `α ⊨ ∅`. Hence `F` has no model. ∎

**Correction (2026-11-24).** An earlier draft said "deletions only remove
clauses, so an UNSAT set stays UNSAT". That is false: `{x, ¬x}` is UNSAT, and
deleting `x` leaves a satisfiable set. The theorem is still true, but the proof
must track a *model of the original formula* through every record (as above),
not preservation of UNSAT under deletion. This paragraph is the corrected
argument; it is also the statement formalised in
`verification/tnps/Tnps.lean` (G4).

---

## 7. Bit-blasting

`blast(P)` is defined only for a well-typed Boolean `P`, and produces the CNF
that asserts `¬P`.

**Literals.** `Lit ::= Var(i) | Const(b)`. Constants are folded: they never
become variables.

**Gates.** For variables `a,b,v`:

```
and(a,b) = v :  (¬a ∨ ¬b ∨ v), (a ∨ ¬v), (b ∨ ¬v)
xor(a,b) = v :  (¬a ∨ ¬b ∨ ¬v), (a ∨ b ∨ ¬v), (a ∨ ¬b ∨ v), (¬a ∨ b ∨ v)
or(a,b)      = ¬and(¬a, ¬b)
xnor(a,b)    = ¬xor(a,b)
majority(a,b,c) = or(and(a,b), and(c, xor(a,b)))
```

All gate constructors simplify when an operand is constant (`and(0,·)=0`,
`xor(1,·)=¬·`, idempotence `op(x,x)`, complement `op(x,¬x)`), so a clause never
contains a constant operand.

**Word encoding.** A word of width `n` is a vector of `n` literals, least
significant bit first.

```
bits(var bv n x) = fresh variables v₁…vₙ, memoised on (x, n)
bits(const bv n v) = [Const(bit_i(v)) for i in 0..n]

add: (s, c_out) = ripple(a, b, Const(0))
     s_i = xor(xor(a_i, b_i), c_i)
     c_{i+1} = majority(a_i, b_i, c_i)

sub: same ripple with b_i replaced by ¬b_i and c_0 = Const(1)
     (two's complement: a + ¬b + 1)

eq:  and over i of xnor(a_i, b_i)
```

The carry out of `add`/`sub` is discarded. Variable numbers are allocated in
canonical traversal order, so `blast(P)` is byte-for-byte deterministic. The
implementation caps CNF variables at `i32::MAX` and clauses at 4,000,000;
exceeding either trips an assertion (fail-closed abort, A2).

**Top level.** Let `ℓ = encode_bool(P)`; the CNF asserts `¬ℓ`:

```
ℓ = Const(1)  ->  the single empty clause      (¬ℓ is false)
ℓ = Const(0)  ->  no clause                    (¬ℓ is true)
ℓ = Var(v)    ->  the unit clause (¬v)         (assert ¬ℓ)
```

*Blast lemma (assumed, not proved — G1).* For every well-typed Boolean `P`,
`blast(P)` is UNSAT iff `⊨ P`. Soundness uses the direction
`blast(P) UNSAT ⇒ ⊨ P`; incompleteness would follow from the converse failing.
This lemma is the single semantic assumption of the `bitblast` rule.

---

## 8. Certificate DAG

A certificate is a finite DAG whose nodes are steps:

```
node = (id, rule, [premise ids], conclusion)
```

plus a finite set of blobs `(kind, digest, bytes)` and a distinguished root
`theorem`.

**Well-formedness.**

1. ids are unique `u32` values (zero is allowed); the root id exists;
2. `refl` has no premises, `sym`/`congr` one, `trans` two, `bitblast` none;
3. every premise id names a node; the induced directed graph is acyclic
   (Kahn's algorithm must consume every node);
4. every conclusion is a well-typed Boolean term;
5. `refl`'s conclusion is `eq bv n t t`;
6. `sym`/`trans`/`congr` conclusions are equalities (the semantic side conditions
   are checked in §9);
7. every `bitblast` digest names a blob of the matching kind, every referenced
   digest has a blob present with that kind, and every blob present is referenced
   by at least one step; blob digests and lengths are recomputed and match;
8. the canonical rendering conditions of §1.5 hold.

The dependency order used by the checker is any topological order; the
implementation uses ascending-id Kahn order, which is deterministic.

---

## 9. Acceptance

`Check(E, C, P) = Accept` means: from the assumption set `E`, certificate `C`
derives proposition `P`. The implementation has **no channel for `E`**; the only
accepting instances are `E = ∅`. Write `Check(C) = Accept` for
`Check(∅, C, P)` where `P` is the root conclusion, and `Thm(C)` for that
conclusion.

`Check(∅, C, P) = Accept` iff:

1. `C` is well formed (§8);
2. `Thm(C) = P` syntactically (the root's conclusion term);
3. for every node in topological order, its rule is justified:

```
refl      conclusion = eq bv n t t
sym       premise = eq bv n a b,  conclusion = eq bv n b a
trans     premise₁ = eq bv n a b, premise₂ = eq bv n b c,
          conclusion = eq bv n a c
congr     premise = eq bv n a b,
          conclusion = eq bv n L R, where L and R are both add (or both sub)
          at the same width and exactly one operand position is replaced:
            L = f(a, t) and R = f(b, t), or L = f(t, a) and R = f(t, b),
          with t syntactically identical in L and R and a = b the premise
bitblast  blob(cnf) parses as C, blob(lrat) parses as Π,
          blast(conclusion) = C, and Π is an accepted LRAT refutation of C
```

Clauses are compared structurally after canonical parsing, not by digest: a
different but equivalent CNF is refused (`CnfMismatch`).

The executable is fail-closed: every malformed field, unknown tag, dangling id,
cycle, non-canonical byte, out-of-range literal, bad hint, or non-RUP addition is
an error. There is no repair, coercion, or guessing.

---

## 10. Soundness

**Theorem 1 (abstract soundness).** For every assumption set `E`, if
`Check(E, C, P) = Accept` and the blast lemma (G1) holds, then `E ⊨ P`.

*Proof.* By induction over a topological order of the DAG. The invariant is:
the conclusion of every processed node is true in every model of `E`; for
equality conclusions, both sides have equal value in every model of `E`.

- `refl`: `⟦t⟧ = ⟦t⟧` in every model, so `eq bv n t t` holds.
- `sym`: if `a = b` holds in a model, so does `b = a`.
- `trans`: if `a = b` and `b = c` hold in a model, so does `a = c`; the checker
  requires the middle terms to be syntactically identical, which is stronger
  than needed.
- `congr`: `add` and `sub` are total functions of their operands, so `a = b`
  implies `f(…,a,…) = f(…,b,…)`; the checker requires exactly one replaced
  operand and syntactic identity of the other.
- `bitblast`: acceptance gives an accepted LRAT refutation of the canonical CNF
  `C = blast(P_n)`; by the LRAT soundness lemma `C` is UNSAT; by the blast lemma,
  `P_n` is valid, hence true in every model of `E`.

The root's conclusion is `P` by condition 2 of §9. With `E = ∅` this is the
implemented case: `Check(C) = Accept` implies `⊨ P`. ∎

**Remark 1 (why `E = ∅`).** The executable cannot introduce assumptions. If an
assumption channel were added, Theorem 1 would have to be relativised to
`E ⊨ P`, and every use of it would need a decision record (AGENTS §9). No such
mechanism exists.

**Remark 2 (identity is not truth).** Digest equality is used only to name bytes
and terms; it is never a premise of a rule. A SHA-256 collision could at worst
make two artefacts share a name; it cannot make a false proposition valid under
Theorem 1, because blob digests are recomputed from their bytes and the CNF is
recomputed from the conclusion.

---

## 11. Explicit assumptions

The following are assumed, not proved here.

- **A1 — SHA-256 collision resistance.** Used by `sha2` for content identity
  (blob digests, term digests, theorem digests). Required for cross-artefact
  binding, not for `Accept ⇒ valid`.
- **A2 — Rust semantics.** The source is compiled by a conforming Rust compiler
  and executes without undefined behaviour. The arithmetic avoids overflow
  (`wrapping_add/sub` plus masking; widths `1..=64`; shifts guarded). The
  `assert!` budget checks and the `unreachable!` in `Lit::require_var` are claimed
  unreachable for validated inputs; if reached they panic (a refusal, not an
  unsound acceptance). Allocation failure aborts. There is no explicit
  recursion-depth bound: a deeply nested term can exhaust the stack and abort.
  That is fail-closed but is a denial-of-service surface.
- **A3 — `sha2` correctness.** The crate computes SHA-256 as specified.
- **A4 — Parser faithfulness (G2).** `parse` accepts exactly the canonical
  language of §1 and `render` is its inverse.
- **A5 — Blast correctness (G1).** The blast lemma of §7.
- **A6 — Executable refinement (G3).** The compiled checker implements the
  abstract relations of §3–§9.
- **A7 — Paper proofs (now machine-checked for the abstract logic).** Lemmas 6.2–6.3 and
  Theorem 1 are formalised and checked in Lean 4 in `verification/tnps/Tnps.lean`. The
  remaining assumption is the blast lemma (A5/G1), which is a hypothesis of Theorem 1, and
  the connection to the executable (A6/G3).

---

## 12. What is **not** proved — named gaps

The checker must not be described as "verified". The following gaps are open.

- **G1 — Bit-blaster equivalence.** No proof that `blast(P)` has the truth table
  of `⟦P⟧` for every `P` and every width `1..=64`. Evidence collected:
  exhaustive evaluation of each tested proposition at widths 1–3, with generated
  refutations accepted and a false identity left SAT, plus hand-picked fixtures
  at widths 8 and 64. Width 4 and above has no exhaustive evidence. The
  width-indexed-name effect (§3) is also inside this gap. This is the assumption
  used by the `bitblast` rule.
- **G2 — Parser refinement.** No proof that the parser recognises exactly the
  canonical grammar and rejects every non-canonical string. Evidence: 500 random
  term round-trips, 6 certificate round-trips, a malformed-input corpus, and
  3000 fuzz iterations, all without panic. Testing is not refinement.
- **G3 — Executable-level refinement.** No proof that the compiled Rust binary
  computes the relations in §3–§9. Compiler correctness, the allocator, the
  `sha2` implementation, and the `Theorem` privacy boundary (enforced by Rust
  module visibility, not by logic) are all outside the kernel. Panics are
  fail-closed but are not modelled.
- **G4 — Machine-checked proof of the paper lemmas. DISCHARGED for the abstract logic
  (2026-11-24).** Lemmas 6.2, 6.3 and Theorem 1 are formalised and checked in Lean 4 core in
  `verification/tnps/Tnps.lean` (`rup_sound`, `Lrat.refutation_sound`, `Cert.theorem1`), with
  the blast lemma as a *hypothesis parameter*, not an axiom. Axioms reported: `propext`,
  `Classical.choice`, `Quot.sound`; no `sorry`. Limits of this discharge: `blast` is opaque
  (G1), the parser is not modelled (G2), the executable is not connected (G3), §1.5/§1.6/§8 are
  not modelled, and the Lean `HasType` omits the `1 ≤ n ≤ 64` side conditions. Receipt:
  `research/receipts/ix43-g4-lean.md`, narrative: `verification/tnps/README.md`.

There is no claim of:

- completeness (the checker may refuse valid propositions whose refutations it
  cannot produce, or whose hint discipline it cannot satisfy);
- proof search (a solver proposes bytes; the checker only decides);
- model-theoretic soundness of the *running process* (G3).

### 12.1 Metric for Milestone B

Milestone B is complete when **G1–G4 are discharged** (machine-checked or
replaced by a verified oracle) **and** the differential suite exercises
independently generated refutations at **every width `1..=64`** with an oracle
that does not decline.

Current measurement (see `research/receipts/ix43-tcb.md`):

| quantity | value |
| --- | --- |
| gaps discharged | **1 / 4** (G4 discharged for the abstract logic; G1, G2, G3 open) |
| widths with independent generation + acceptance agreement | **3 / 64** (`1,2,3`) |
| widths with signed-checker fixtures only | `8, 64` |
| oracle semantic budget | **20 free bits** (exhaustive evaluation above budget: declined) |
| differential acceptance cases | **10** |
| differential mutation cases | **19** (18 rejections, 1 benign acceptance) |
| parser round-trip cases | **500 terms + 6 certificates** |
| fuzz iterations | **2000 random inputs + 1000 certificate bit-flip mutations** |
| surviving divergences | **0** among the tested families |

The gap is *measurable*: it is the table above, not a disclaimer. A run that
claims Milestone B is complete must show `0/4` becoming `4/4` and `3/64` becoming
`64/64` with a non-declining oracle.

---

## 13. Test oracle

`src/native/reference.rs` is a second, deliberately naive implementation used
only by `tests/native_semantics.rs`. It is compiled but `#[doc(hidden)]`, and no
trusted module names it; the test
`reference_oracle_is_not_on_the_trusted_path` scans the trusted sources for any
code-level dependency. Its independence is:

- separate rule matching (not shared with `check.rs`);
- separate DIMACS/LRAT line parsers;
- RUP by unit propagation to a fixpoint over the whole live clause set, ignoring
  hint order (hint *references* are still required to be well formed);
- a separate evaluator plus exhaustive falsification search for propositions with
  at most 20 free bits.

It shares the `Term` data type and structural equality, and it calls `bitblast`
to compute the expected CNF. Those parts are therefore *not* independently
validated; a blast bug is caught only by channel 2 below, and only within the
20-bit budget.

It can disagree with the signed checker in the following ways; a disagreement is
data, not automatically an oracle error:

1. **Strictness.** The signed checker demands a valid hint witness; the oracle
   accepts any RUP clause. A proof with a corrupted but well-referenced hint can
   be refused by the signed checker and accepted by the oracle. This is a
   strictness difference, not a soundness difference.
2. **Blast bugs.** On tiny widths the oracle also evaluates the proposition under
   every assignment. If a bug in `bitblast` made a false proposition's CNF UNSAT,
   the signed checker could accept while the oracle refuses with a counterexample.
   This is the intended detection channel.
3. **Budget.** Above 20 free bits the oracle's semantic check is declined; it
   still checks canonical CNF equality and RUP. It is not used on the 8/64-bit
   fixtures.

The oracle is **not** part of the trusted base and must never be called by
`check`. A test scanning imports and call sites enforces that.
