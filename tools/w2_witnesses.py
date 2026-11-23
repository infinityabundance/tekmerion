#!/usr/bin/env python3
"""w2_witnesses.py -- build a v3 witness for add / sub / neg mod p (P-256 ARM).

Inputs to a template are exactly the three things the brief allows:

  * the machine-code step list (from the ELF object, via tools/disasm_steps),
  * the modulus (a HOL term string, read from the postcondition),
  * the specification terms (the postcondition equality and its guards, read
    from the benchmark query).

The *congruence* and the *flag meaning* are derived here from the postcondition
and from the derived steps; they are not read out of the human proof text.
The proof file is never opened by this module.

Everything that is still informed by the human close is named in
``DEVELOPMENT_ONLY`` (also copied into ``Witness.development_only``) and is
reported in the receipt:

  * the *order* in which the congruence lemma's side conditions are discharged,
  * the digit-expansion / accumulator-rewrite idiom used to finish them.

Those are tactic-ordering choices, not facts about the routine, so a witness
records them explicitly instead of hiding them.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path
from typing import Dict, List, Optional, Tuple

HERE = Path(__file__).resolve().parent
if str(HERE) not in sys.path:
    sys.path.insert(0, str(HERE))

import derive_steps  # noqa: E402
from witness import (  # noqa: E402
    CaseSplit,
    Congruence,
    FlagMeaning,
    InputRef,
    Witness,
)


# Fragments of the closer that are transcribed from the human proofs and are
# therefore labelled development-only.  Kept here so the label travels with
# the witness, not only with the code.
DEVELOPMENT_ONLY: Dict[str, str] = {
    "side_condition_order":
        "The order in which the congruence lemma's hypotheses are discharged "
        "follows the human proof (bounds first, case split last); HOL does not "
        "force this order.",
    "digit_expansion_idiom":
        "BIGNUM_LEXPAND_CONV / BIGNUM_EXPAND_CONV and DISCARD_STATE_TAC are the "
        "human proof's expansion idiom; the closer reproduces it.",
    "accumulator_close":
        "DECARRY_RULE / DESUM_RULE followed by BOUNDER_TAC or REAL_INTEGER_TAC "
        "is the human arithmetic close; the closer reproduces it.",
    "rewrite_lemma_sets":
        "The exact rewrite-lemma sets (REAL_BITVAL_NOT, WORD_AND_MASK, "
        "GSYM REAL_OF_NUM_CLAUSES, ...) are taken from the human proof.",
}


# --------------------------------------------------------------------------
# Specification parsing
# --------------------------------------------------------------------------


_BACKTICK = "`"


def _unquote(query: str) -> str:
    q = query.strip()
    if q.startswith(_BACKTICK):
        q = q[1:]
    if q.endswith(_BACKTICK):
        q = q[:-1]
    return q


def parse_spec(query: str) -> Dict[str, object]:
    """Extract the variables, input references, guard and conclusion.

    The result is a plain dict of strings/ints; it is *not* a witness.
    """
    q = _unquote(query)

    m = re.match(r"\s*!\s*([A-Za-z0-9_ ]+?)\s*\.", q, re.S)
    if not m:
        raise ValueError("query does not start with a universal quantifier")
    variables = m.group(1).split()

    # bignum_from_memory (ptr,N) s = <bare value>, where the value is the
    # whole right-hand side of that conjunct (terminated by /\ or )).
    refs: List[InputRef] = []
    for pm in re.finditer(
            r"bignum_from_memory\s*\(\s*(\w+)\s*,\s*(\d+)\s*\)\s*s\s*=\s*"
            r"([A-Za-z_]\w*)\s*(?=/\\|\))", q):
        ptr, ndigits, value = pm.group(1), int(pm.group(2)), pm.group(3)
        if value in variables:
            refs.append(InputRef(pointer=ptr, value=value, digits=ndigits))

    no = re.search(
        r"nonoverlapping\s*\(\s*word\s+pc\s*,\s*0x([0-9a-fA-F]+)\s*\)\s*"
        r"\(\s*(\w+)\s*,\s*(\d+)\s*\*\s*(\d+)\s*\)", q)
    if not no:
        raise ValueError("cannot find nonoverlapping shape in query")
    code_bytes = int(no.group(1), 16)
    result_ptr = no.group(2)
    # ``(z,8 * 4)`` is 8 bytes per limb times 4 limbs.
    limb_bits = int(no.group(3)) * 8
    digits = int(no.group(4))

    # Guard: the parenthesised antecedent of the postcondition equality.
    g = re.search(r"\(([^()]*?p_[A-Za-z0-9_]+[^()]*?)\s*==>\s*"
                  r"(?:&\(|\bbignum_from_memory)", q, re.S)
    if not g:
        raise ValueError("cannot find postcondition guard")
    guard = [part.strip() for part in re.split(r"/\\", g.group(1)) if part.strip()]

    # Conclusion forms.
    concl: Dict[str, object] = {}
    nat = re.search(
        r"bignum_from_memory\s*\(\s*" + re.escape(result_ptr) +
        r"\s*,\s*\d+\s*\)\s*s\s*=\s*\((.*?)\)\s*MOD\s*(\w+)", q, re.S)
    intrem = re.search(
        r"&\s*\(\s*bignum_from_memory\s*\(\s*" + re.escape(result_ptr) +
        r"\s*,\s*\d+\s*\)\s*s\s*\)\s*=\s*\(&\s*(\w+)\s*-\s*&\s*(\w+)\)\s*rem\s*&\s*(\w+)",
        q, re.S)
    if nat:
        inner, modulus = nat.group(1).strip(), nat.group(2)
        sub = re.fullmatch(r"(\w+)\s*-\s*(\w+)", inner)
        if sub:
            concl = {"kind": "nat_mod_self", "minuend": sub.group(1),
                     "subtrahend": sub.group(2), "modulus": modulus}
        else:
            plus = re.fullmatch(r"(\w+)\s*\+\s*(\w+)", inner)
            if plus:
                concl = {"kind": "nat_mod", "left": plus.group(1),
                         "right": plus.group(2), "sum": inner,
                         "modulus": modulus}
            else:
                raise ValueError(f"unrecognised MOD operand: {inner!r}")
    elif intrem:
        concl = {"kind": "int_rem", "minuend": intrem.group(1),
                 "subtrahend": intrem.group(2), "modulus": intrem.group(3)}
    else:
        raise ValueError("unrecognised postcondition equality shape")

    var_types: Dict[str, str] = {}
    pointers = {r.pointer for r in refs} | {result_ptr}
    for v in variables:
        var_types[v] = "int64" if v in pointers else "num"

    return {
        "variables": variables,
        "words": [r.pointer for r in refs],
        "inputs": refs,
        "result": result_ptr,
        "digits": digits,
        "limb_bits": limb_bits,
        "bits": digits * limb_bits,
        "code_bytes": code_bytes,
        "guard": guard,
        "conclusion": concl,
        "var_types": var_types,
    }


# --------------------------------------------------------------------------
# Derivation helpers
# --------------------------------------------------------------------------


def _split_guard(guard: List[str]) -> Tuple[List[str], List[str]]:
    """Best-effort numeric bound terms vs. other guard terms."""
    bounds = [g for g in guard if re.search(r"[<>]=?|(?:^|\W)<=?(?:\W|$)", g)]
    other = [g for g in guard if g not in bounds]
    return bounds, other


def _last_carry_step(steps, accumulate: List[int]) -> Optional[int]:
    for n in reversed(accumulate):
        sem = derive_steps.DEFAULT_SEMANTICS.get(steps[n - 1].mnemonic)
        if sem is not None and sem.writes_carry:
            return n
    return None


def _leading_sub_chain(steps, accumulate: List[int]) -> List[int]:
    """The accumulate steps in the first contiguous run of SUB forms."""
    run: List[int] = []
    started = False
    for n in accumulate:
        sem = derive_steps.DEFAULT_SEMANTICS.get(steps[n - 1].mnemonic)
        form = sem.form if sem is not None else None
        if form == "SUB":
            run.append(n)
            started = True
        elif started:
            break
    return run


def _zero_test_chain(steps) -> List[int]:
    """Indices of an ORR run followed by CMP then CSETM (the n = 0 probe)."""
    idx: List[int] = []
    i = 0
    n = len(steps)
    while i < n:
        if steps[i].mnemonic == "orr":
            j = i
            while j < n and steps[j].mnemonic == "orr":
                j += 1
            if (j + 1 < n and steps[j].mnemonic == "cmp"
                    and steps[j + 1].mnemonic in ("csetm", "cset")):
                return list(range(i + 1, j + 3))
            i = j
        i += 1
    return idx


# --------------------------------------------------------------------------
# Template entry point
# --------------------------------------------------------------------------


def build_witness(op: str, steps, query: str, *, theorem_name: str,
                  exec_name: str) -> Witness:
    """Produce the v3 witness for ``op`` from steps + query + modulus."""
    if op not in ("add", "sub", "neg"):
        raise ValueError(f"unsupported op {op!r}")

    spec = parse_spec(query)
    digits = int(spec["digits"])  # type: ignore[arg-type]
    bits = int(spec["bits"])  # type: ignore[arg-type]
    guard = list(spec["guard"])  # type: ignore[arg-type]
    concl = dict(spec["conclusion"])  # type: ignore[arg-type]
    inputs = list(spec["inputs"])  # type: ignore[arg-type]
    result = str(spec["result"])
    var_types = dict(spec["var_types"])  # type: ignore[arg-type]

    accumulate = derive_steps.derive_accumulate(steps)
    if not accumulate:
        raise ValueError("derive_steps found no accumulating steps")

    width_bits = str(bits)
    width_mod = f"64 * {digits}"
    modulus = str(concl["modulus"])
    value_vars = [inp.value for inp in inputs]
    exp_vars = "[" + "; ".join(f'"{v}"' for v in value_vars) + "]"
    final_state = f"s{len(steps) - 1}"
    sim_range = [1, len(steps) - 1]

    bounds, _other = _split_guard(guard)
    extra_bounds = [f"&{modulus} < &2 pow {width_bits}",
                    f"&0 < &{modulus}"]

    if concl["kind"] == "nat_mod":
        sum_expr = str(concl["sum"])
        # The flag decides which representative of (sum MOD modulus) is
        # written.  Its meaning is the sum-vs-modulus comparison.
        meaning = f"({sum_expr}) < {modulus}"
        last = _last_carry_step(steps, accumulate)
        if last is None:
            raise ValueError("no carry-writing accumulate step for nat_mod")
        flag = FlagMeaning(flag=f"carry_s{last}", meaning=meaning,
                           rule="FLAG_FROM_CARRY_LT",
                           bound=str(bits + 64), source_steps=accumulate)
        congruence = Congruence(
            principle="EQUAL_FROM_CONGRUENT_MOD_MOD",
            kind="nat_mod",
            width=width_mod,
            witness=(f"if {meaning} then &({sum_expr}) "
                     f"else &({sum_expr}) - &{modulus}:real"),
            modulus=modulus,
            orientation="sym_rhs_bignum",
        )
        case_split = CaseSplit(
            kind="mod_add_cases",
            condition=meaning,
            true_branch=f"&({sum_expr})",
            false_branch=f"&({sum_expr}) - &{modulus}",
            lemmas=["MOD_ADD_CASES", "GSYM NOT_LE", "COND_SWAP"],
        )
        bounds = bounds + extra_bounds

    elif concl["kind"] == "nat_mod_self":
        sub_expr = f"{concl['minuend']} - {concl['subtrahend']}"
        zero = f"{concl['subtrahend']} = 0"
        congruence = Congruence(
            principle="EQUAL_FROM_CONGRUENT_REAL",
            kind="nat_mod_self",
            width=width_bits,
            witness="&0:real",
            modulus=modulus,
            orientation="lhs_bignum",
        )
        idx = _zero_test_chain(steps)
        flag = FlagMeaning(flag=None, meaning=zero, rule="WORD_OR_EQ_0",
                           bound=None, source_steps=idx)
        case_split = CaseSplit(
            kind="mod_sub_self",
            condition=zero,
            true_branch="0",
            false_branch=sub_expr,
            lemmas=["COND_RAND"],
        )
        bounds = bounds + extra_bounds

    elif concl["kind"] == "int_rem":
        a, b = str(concl["minuend"]), str(concl["subtrahend"])
        chain = _leading_sub_chain(steps, accumulate)
        if not chain:
            raise ValueError("no leading SUB chain for int_rem")
        last = chain[-1]
        borrow = f"{a} < {b}"
        flag = FlagMeaning(flag=f"carry_s{last}", meaning=borrow,
                           rule="FLAG_FROM_CARRY_LT", bound=str(bits),
                           source_steps=chain)
        quotient = f"--(&(bitval({borrow}))):int"
        congruence = Congruence(
            principle="INT_REM_UNIQ",
            kind="int_rem",
            width=width_bits,
            witness=f"&(bignum_from_memory ({result},{digits}) s)",
            modulus=modulus,
            quotient=quotient,
            orientation="sym_int_rem",
        )
        case_split = CaseSplit(
            kind="int_rem_sign",
            condition=f"&{a}:real < &{b}",
            true_branch="--(&1)",
            false_branch="&0",
            lemmas=["INT_ARITH", "INT_REM_UNIQ"],
        )
        bounds = bounds + extra_bounds
    else:  # pragma: no cover - parse_spec rejects unknown kinds
        raise ValueError(concl["kind"])

    # The invariant is the postcondition equality itself.
    if concl["kind"] == "nat_mod":
        invariant = (f"bignum_from_memory ({result},{digits}) s = "
                     f"({concl['sum']}) MOD {modulus}")
    elif concl["kind"] == "nat_mod_self":
        invariant = (f"bignum_from_memory ({result},{digits}) s = "
                     f"({concl['minuend']} - {concl['subtrahend']}) MOD {modulus}")
    else:
        invariant = (f"&(bignum_from_memory ({result},{digits}) s) = "
                     f"(&{concl['minuend']} - &{concl['subtrahend']}) rem &{modulus}")

    return Witness(
        op=op,
        theorem_name=theorem_name,
        exec_name=exec_name,
        accumulate=accumulate,
        congruence=congruence,
        flag_meaning=flag,
        case_split=case_split,
        bounds=bounds,
        invariant=invariant,
        vars=list(spec["variables"]),  # type: ignore[arg-type]
        var_types=var_types,
        inputs=inputs,
        guard=guard,
        width_bits=width_bits,
        width_mod=width_mod,
        final_state=final_state,
        simulation_range=sim_range,
        development_only=sorted(DEVELOPMENT_ONLY),
    )


def main(argv: Optional[List[str]] = None) -> int:
    import argparse
    import json

    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("op", choices=["add", "sub", "neg"])
    ap.add_argument("object", help="AArch64 ELF object")
    ap.add_argument("query_file", help="file containing the benchmark query term")
    ap.add_argument("--name", default="THEOREM")
    ap.add_argument("--exec", dest="exec_name", default=None)
    args = ap.parse_args(argv)

    import disasm_steps

    steps = disasm_steps.disassemble(args.object)
    query = Path(args.query_file).read_text(encoding="utf-8")
    w = build_witness(
        args.op, steps, query,
        theorem_name=args.name,
        exec_name=args.exec_name or f"{args.name}_EXEC",
    )
    print(w.to_json())
    return 0


if __name__ == "__main__":
    sys.exit(main())
