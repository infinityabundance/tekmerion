# F1 — the emitted proof script is arbitrary OCaml: gate implemented, 0/26 pass

- **Date:** 2026-11-24
- **Artefacts:** `src/tools/script_gate.rs`, `tests/tools.rs`
- **Subject:** the 26 emitted answers under `.bench/dropin-gen/answers{,-arm}/`

## 1. Why this exists

HOL Light's soundness rests on OCaml's type system: only the kernel can build a `thm`. A
generated proof script is arbitrary OCaml, so an untrusted producer can shadow `prove`,
`axioms` or `check_axioms`, use `Obj.magic`/`Marshal`, or mutate global state. The bench's
`axioms()` before/after comparison catches none of that.

## 2. What was built

`src/tools/script_gate.rs` parses a candidate with the **real OCaml front end**, through HOL
Light's own quotation preprocessor:

```
ocamlc -pp "<.bench/hol-light/hol.sh -pp>" -stop-after parsing -dparsetree -impl <tmp>
```

and walks the parsetree for:

- `Pexp_ident` in `Obj*`, `Marshal*`, `Unix*`, `Sys.command`, `Sys.*` (strict), `ref`, `:=`,
  file-I/O primitives (strict);
- `Ppat_var` naming a kernel/judge primitive (`prove`, `axioms`, `check_axioms`,
  `new_axiom`, `mk_thm`, `UNDISCH`, `TAUT`, `GEN`, `INST`, `EQ_MP`, `new_definition`);
- any `Pstr_primitive` (`external`).

A token fallback runs when no HOL switch is present (CI); every report labels which mode ran
and that the fallback is weaker.

Exit codes: 0 allowed, 1 refused by policy, 2 parse error, 3 tool unavailable.

## 3. Firing controls

`python3 -m pytest tests/tools.rs -q` → **12 passed**, including the authoritative
front-end control. Each control plants a defect that must be refused:

| control | result |
|---|---|
| clean tactic expression | allowed |
| `Obj.magic` | refused (front end: `forbidden identifier Obj.magic`) |
| `Marshal.to_string` | refused |
| `Sys.command "true"` | refused (front end: `Sys.command`) |
| `external` | refused |
| shadowed `check_axioms` | refused (`rebinding of kernel name`) |
| rebound `prove` | refused |
| kernel name as a parameter | refused |
| `ref`/`:=` under strict | refused |
| `Sys.getenv` under strict | refused |
| `ref`/`:=` under denylist | allowed (deliberate: the denylist is not the target grammar) |

## 4. Measurement: 0 of 26 pass

Every emitted answer was run through both profiles with the front end.

```
SUMMARY denylist: allowed 0 of 26
SUMMARY strict:   allowed 0 of 26
```

Reasons, from `bignum_add_p25519` (the others are the same class):

```
forbidden identifier Sys.command
forbidden identifier Unix.gettimeofday
strict-profile identifier :=
strict-profile identifier close_in / close_out / input_line
```

The counts over the corpus are `Unix` 52, `Sys.command` 78, `ref` 260, `:=` 182, and **zero**
`Obj`, `Marshal`, `external` or kernel rebindings.

## 5. What this means (the important part)

The gate did not find a forgery. It found something structural: **the emitted proof runs an
external process and does file I/O at check time.** The CSAT close shells out to `cadical`
and reads files; the inlined support library carries `read_dimacs`/`write_dimacs` and
`Unix.gettimeofday`. That is a search procedure inside the artifact, not a proof term.

So F1 is not satisfiable by tightening the emitter's syntax. It requires an architectural
change:

1. Move solver invocation to **construction** (untrusted) and make the accepted artifact a
   closed proof term that **checks a supplied certificate** — the LRAT bytes — with no
   subprocess and no file I/O.
2. Emit only the proof term; load the support library from the judge, not inline it.
3. Then the strict gate should accept all 27, and Wiedijk-style printing/parsing can be
   checked term-for-term.

Until that exists, the honest statement is: **Tekmerion's generated proofs are not yet
closed-grammar artifacts, and the current acceptance path cannot distinguish a forged
script from a genuine one at the OCaml level.** The kernel still checks the theorem; what is
unprotected is the *script language around it*.

## 6. Scope

- The gate is a policy checker, not a proof. It cannot establish honesty of an allowed
  script; it can only refuse known-dangerous syntax.
- Reading `Unix`/`Sys` as forbidden is the strict target profile; the denylist profile is the
  immediate control and is deliberately narrower.
- No verdict, receipt or benchmark number is changed by this work.
