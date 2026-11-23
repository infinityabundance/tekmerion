#!/usr/bin/env python3
"""witness.py -- the typed witness record for a straight-line ARM mod-p routine.

A *witness* is the small, structured fact set that a closer needs in order to
finish a symbolic-simulation proof.  It is deliberately not a script: every
field is either a HOL term string or a small list / dict of HOL term strings,
so it can be serialised, diffed and replayed without parsing tactic text.

The v3 schema is the six fields required by the programme brief:

  accumulate    sorted 1-based machine-code step numbers that carry an
                accumulator relation (derived by tools/derive_steps.py).
  congruence    the congruence principle that turns a digit sum into the
                postcondition equality, plus the witness value it needs.
  flag_meaning  which CPU flag decides the modular correction, and what that
                flag means in terms of the inputs.
  case_split    the single case distinction the congruence needs.
  bounds        the numeric side conditions the congruence needs.
  invariant     the postcondition equality, spelled as a HOL term.

Extra fields (``vars``, ``width``, ``final_state``, ...) are *derived
scaffolding* the closer needs; they are not claims about the proof.

This module has no file I/O and imports nothing from the repository, so it can
be unit-tested on its own.  The CLI serialises a record that a receipt can
quote verbatim.
"""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import asdict, dataclass, field
from typing import Any, Dict, List, Optional


SCHEMA_VERSION = "v3"


# --------------------------------------------------------------------------
# Structured sub-records
# --------------------------------------------------------------------------


@dataclass(frozen=True)
class Congruence:
    """How a digit sum becomes the postcondition equality.

    ``principle``   the HOL lemma applied (``MATCH_MP_TAC``).
    ``kind``        a small tag naming the arithmetic shape.
    ``width``       HOL term for the bit width ``k`` (e.g. ``64 * 4``).
    ``witness``     HOL term for the congruence witness value ``r'``/``z``.
    ``modulus``     HOL term for the modulus.
    ``quotient``    HOL term for the integer quotient (``INT_REM_UNIQ`` only).
    ``orientation`` which side of the goal gets expanded, as a tag.
    """

    principle: str
    kind: str
    width: str
    witness: str
    modulus: str
    quotient: Optional[str] = None
    orientation: str = ""

    def __post_init__(self) -> None:
        if not self.principle:
            raise ValueError("congruence.principle is empty")
        if not self.width:
            raise ValueError("congruence.width is empty")
        if not self.witness:
            raise ValueError("congruence.witness is empty")
        if not self.modulus:
            raise ValueError("congruence.modulus is empty")


@dataclass(frozen=True)
class FlagMeaning:
    """Which flag selects the modular correction, and what it means.

    ``flag``     HOL state name (e.g. ``carry_s16``) or ``None`` for a
                 zero-test that the closer does not name explicitly.
    ``meaning``  HOL term for the Boolean the flag is equivalent to.
    ``rule``     the lemma family used to establish the equivalence.
    ``bound``    HOL term for the exponent passed to the flag lemma.
    ``source_steps``  machine-code steps the flag is derived from.
    """

    flag: Optional[str]
    meaning: str
    rule: str
    bound: Optional[str] = None
    source_steps: List[int] = field(default_factory=list)

    def __post_init__(self) -> None:
        if not self.meaning:
            raise ValueError("flag_meaning.meaning is empty")
        if not self.rule:
            raise ValueError("flag_meaning.rule is empty")


@dataclass(frozen=True)
class CaseSplit:
    """The single case distinction the congruence uses."""

    kind: str
    condition: Optional[str] = None
    true_branch: Optional[str] = None
    false_branch: Optional[str] = None
    lemmas: List[str] = field(default_factory=list)

    def __post_init__(self) -> None:
        if not self.kind:
            raise ValueError("case_split.kind is empty")


# --------------------------------------------------------------------------
# The witness
# --------------------------------------------------------------------------


@dataclass(frozen=True)
class Witness:
    op: str
    theorem_name: str
    exec_name: str
    accumulate: List[int]
    congruence: Congruence
    flag_meaning: FlagMeaning
    case_split: CaseSplit
    bounds: List[str]
    invariant: str
    # Derived scaffolding (not claims).
    vars: List[str] = field(default_factory=list)
    var_types: Dict[str, str] = field(default_factory=dict)
    inputs: List["InputRef"] = field(default_factory=list)
    guard: List[str] = field(default_factory=list)
    width_bits: str = ""
    width_mod: str = ""
    final_state: str = ""
    simulation_range: List[int] = field(default_factory=list)
    development_only: List[str] = field(default_factory=list)
    schema: str = SCHEMA_VERSION

    def __post_init__(self) -> None:
        if self.op not in ("add", "sub", "neg"):
            raise ValueError(f"unsupported op: {self.op!r}")
        if not self.theorem_name:
            raise ValueError("theorem_name is empty")
        if not self.exec_name:
            raise ValueError("exec_name is empty")
        if not self.accumulate:
            raise ValueError("accumulate is empty")
        if sorted(set(self.accumulate)) != sorted(self.accumulate):
            raise ValueError("accumulate must be sorted and duplicate-free")
        if any(n < 1 for n in self.accumulate):
            raise ValueError("accumulate step numbers are 1-based")
        if not self.invariant:
            raise ValueError("invariant is empty")
        if not self.bounds:
            raise ValueError("bounds is empty")

    # -- serialisation -----------------------------------------------------

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)

    def to_json(self, indent: int = 2) -> str:
        return json.dumps(self.to_dict(), indent=indent, sort_keys=True)

    def accum_str(self) -> str:
        return "[" + ";".join(str(n) for n in self.accumulate) + "]"

    def digest(self) -> str:
        import hashlib

        return hashlib.sha256(self.to_json(indent=0).encode()).hexdigest()


@dataclass(frozen=True)
class InputRef:
    """One ``bignum_from_memory (ptr,4) s = value`` precondition."""

    pointer: str
    value: str
    digits: int

    @property
    def digit_prefix(self) -> str:
        return f"{self.value}_"


def from_dict(d: Dict[str, Any]) -> Witness:
    """Rebuild a witness from :meth:`Witness.to_dict` output."""
    inputs = [InputRef(**item) for item in d.get("inputs", [])]
    return Witness(
        op=d["op"],
        theorem_name=d["theorem_name"],
        exec_name=d["exec_name"],
        accumulate=list(d["accumulate"]),
        congruence=Congruence(**d["congruence"]),
        flag_meaning=FlagMeaning(**d["flag_meaning"]),
        case_split=CaseSplit(**d["case_split"]),
        bounds=list(d["bounds"]),
        invariant=d["invariant"],
        vars=list(d.get("vars", [])),
        var_types=dict(d.get("var_types", {})),
        inputs=inputs,
        guard=list(d.get("guard", [])),
        width_bits=d.get("width_bits", ""),
        width_mod=d.get("width_mod", ""),
        final_state=d.get("final_state", ""),
        simulation_range=list(d.get("simulation_range", [])),
        development_only=list(d.get("development_only", [])),
        schema=d.get("schema", SCHEMA_VERSION),
    )


def main(argv: Optional[List[str]] = None) -> int:
    ap = argparse.ArgumentParser(description="witness record utilities")
    ap.add_argument("--selftest", action="store_true",
                    help="round-trip a synthetic witness and exit")
    args = ap.parse_args(argv)
    if not args.selftest:
        ap.print_help()
        return 0
    w = Witness(
        op="add",
        theorem_name="T",
        exec_name="E",
        accumulate=[3, 4],
        congruence=Congruence("P", "nat_mod", "64 * 4", "&0", "p", "q",
                              "sym_rhs_bignum"),
        flag_meaning=FlagMeaning("carry_s4", "x < y", "FLAG_FROM_CARRY_LT",
                                 "320", [3, 4]),
        case_split=CaseSplit("mod_add_cases", "x < y", "a", "b", ["M"]),
        bounds=["x < p"],
        invariant="z = (x + y) MOD p",
        inputs=[InputRef("x", "m", 4)],
    )
    assert from_dict(w.to_dict()) == w
    assert w.accum_str() == "[3;4]"
    print(json.dumps(w.to_dict(), indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
