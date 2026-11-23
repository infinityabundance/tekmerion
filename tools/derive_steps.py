#!/usr/bin/env python3
"""derive_steps.py -- derive the accumulating step set from carry dataflow.

This module is deliberately blind to any proof script.  ``derive_accumulate``
accepts only

  * ``exec_steps``: the parsed machine-code step list (one :class:`Step` per
    instruction, in program order), and
  * ``semantics``: a mnemonic -> :class:`InstructionSemantics` table.

It has no file I/O, no network, no subprocess, and does not import anything
from this repository.  It cannot see a tactic line, a state-name list, or a
hand-written accumulate list, and it never receives the name of a curve or a
theorem.  The only inputs are the machine instructions and what those
instructions mean.

The derivation
--------------
An instruction participates in a carry chain when it computes ``d = a +/- b``
(optionally plus/minus an incoming carry flag) into a general-purpose
register.  Such a chain can only be *used* if

  * every carry-in it consumes was produced by an earlier flag write, and
  * every carry-out it produces is consumed by a later flag read,

so those are the liveness conditions checked below.  Two further exclusions
matter for reproducing the reference accumulator context:

  * a destination of ``xzr``/``sp`` writes no value (this removes ``CMP`` and
    ``CMN``-style flag-only probes), and
  * an instruction whose *sources* are all zero registers or immediates is a
    carry *materialisation* (``ADC Xd, XZR, XZR``), not a digit sum, and has no
    accumulator relation of its own.

The result is a sorted list of 1-based step numbers, matching the numbering the
ARM simulation uses (instruction 1 is the first word in the ELF ``.text``).
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Mapping, Optional, Sequence, Tuple


# --------------------------------------------------------------------------
# Machine-code step list and instruction semantics (the whole interface)
# --------------------------------------------------------------------------


@dataclass(frozen=True)
class Step:
    """One machine-code instruction, as parsed from an ELF disassembly.

    ``index`` is 1-based and follows program order.  ``operands`` are
    lower-cased and stripped but otherwise as the disassembler emitted them
    (memory operands keep their brackets), so ``x4``, ``xzr``, ``#0x1`` and
    ``[x1, #16]`` are all distinguishable.
    """

    index: int
    mnemonic: str
    operands: Tuple[str, ...] = ()
    raw: str = ""


@dataclass(frozen=True)
class InstructionSemantics:
    """What one mnemonic does to registers and the NZCV carry bit.

    ``form``          ``"ADD"``, ``"SUB"`` or ``None`` (not an add/sub form).
    ``dest``          operand index written, or ``None`` (flag-only compare).
    ``sources``       operand indices that are read as values.
    ``reads_carry``   consumes the incoming C flag (ADC/SBC/NGC family).
    ``writes_carry``  produces a C flag (the ``S`` forms, CMP, CMN, ...).
    ``reads_flags``   consumes NZCV for a conditional operation or branch.
    """

    form: Optional[str]
    dest: Optional[int] = None
    sources: Tuple[int, ...] = ()
    reads_carry: bool = False
    writes_carry: bool = False
    reads_flags: bool = False


_NOACC = InstructionSemantics(form=None)

# Only the ARM64 add/sub-with-carry family is modelled; every other mnemonic
# is a non-candidate for accumulation.  The table is data, not policy: the
# policy (liveness, zero-register exclusions) lives in derive_accumulate.
DEFAULT_SEMANTICS: Mapping[str, InstructionSemantics] = {
    # d = a + b            (no carry in, writes C)
    "adds": InstructionSemantics("ADD", dest=0, sources=(1, 2), writes_carry=True),
    # d = a + b + C        (carry in; +S writes C out)
    "adcs": InstructionSemantics("ADD", dest=0, sources=(1, 2),
                                 reads_carry=True, writes_carry=True),
    "adc": InstructionSemantics("ADD", dest=0, sources=(1, 2),
                                reads_carry=True, writes_carry=False),
    # d = a - b            (no carry in, writes C)
    "subs": InstructionSemantics("SUB", dest=0, sources=(1, 2), writes_carry=True),
    # d = a - b - C        (carry in; +S writes C out)
    "sbcs": InstructionSemantics("SUB", dest=0, sources=(1, 2),
                                 reads_carry=True, writes_carry=True),
    "sbc": InstructionSemantics("SUB", dest=0, sources=(1, 2),
                                reads_carry=True, writes_carry=False),
    # d = 0 - b - C        (the "negate with carry" forms)
    "ngcs": InstructionSemantics("SUB", dest=0, sources=(1,),
                                 reads_carry=True, writes_carry=True),
    "ngc": InstructionSemantics("SUB", dest=0, sources=(1,),
                                reads_carry=True, writes_carry=False),
    # flag-only probes: value written to XZR, so they never contribute a digit
    "cmp": InstructionSemantics("SUB", dest=None, sources=(0, 1),
                                writes_carry=True),
    "cmn": InstructionSemantics("ADD", dest=None, sources=(0, 1),
                                writes_carry=True),
    "ccmp": InstructionSemantics("SUB", dest=None, sources=(0, 1),
                                 reads_carry=True, reads_flags=True,
                                 writes_carry=True),
    "ccmn": InstructionSemantics("ADD", dest=None, sources=(0, 1),
                                 reads_carry=True, reads_flags=True,
                                 writes_carry=True),
    # Non-arithmetic flag writers: they matter only as chain boundaries.
    "ands": InstructionSemantics(form=None, writes_carry=True),
    "orrs": InstructionSemantics(form=None, writes_carry=True),
    "eors": InstructionSemantics(form=None, writes_carry=True),
    "bics": InstructionSemantics(form=None, writes_carry=True),
    "negs": InstructionSemantics(form=None, writes_carry=True),
    "tst": InstructionSemantics(form=None, writes_carry=True),
}


# Mnemonics that consume flags without being add/sub forms.  Kept separate so
# that the semantics table above stays a faithful description of the arithmetic
# instructions.  Conditional branches carry their condition in the mnemonic.
_FLAG_READERS = frozenset({
    "csel", "csinc", "csinv", "csneg", "cset", "csetm", "cinc", "cinv",
    "cneg",
})
_COND_SUFFIXES = (
    "eq", "ne", "cs", "hs", "cc", "lo", "mi", "pl", "vs", "vc",
    "hi", "ls", "ge", "lt", "gt", "le", "al",
)


def _is_conditional_branch(mnemonic: str) -> bool:
    if not mnemonic.startswith("b."):
        return False
    return mnemonic[2:] in _COND_SUFFIXES


def _is_flag_writer(sem: InstructionSemantics) -> bool:
    return sem.writes_carry


def _is_flag_reader(mnemonic: str, sem: InstructionSemantics) -> bool:
    return (sem.reads_flags or sem.reads_carry
            or mnemonic in _FLAG_READERS or _is_conditional_branch(mnemonic))


def _is_general_register(operand: str) -> bool:
    """True for x0..x30 / w0..w30; false for xzr, wzr, sp, immediates, memory."""
    if not operand:
        return False
    if operand.startswith("["):
        return False
    if operand in ("xzr", "wzr", "sp", "wsp"):
        return False
    if not (operand[0] in ("x", "w")):
        return False
    digits = operand[1:]
    return digits.isdigit() and 0 <= int(digits) <= 30


def _source_operands(step: Step, sem: InstructionSemantics) -> Tuple[str, ...]:
    out = []
    for k in sem.sources:
        if k < len(step.operands):
            out.append(step.operands[k])
    return tuple(out)


# --------------------------------------------------------------------------
# Derivation
# --------------------------------------------------------------------------


def _carry_in_is_live(sem: Sequence[InstructionSemantics], i: int) -> bool:
    """Is the incoming flag of step i produced by an earlier flag write?

    Walking backwards, a flag writer makes the carry-in live; a flag reader
    that does not write flags cannot produce one, so the chain is ungrounded.
    """
    for j in range(i - 1, -1, -1):
        if _is_flag_writer(sem[j]):
            return True
        if sem[j].reads_flags or sem[j].reads_carry:
            return False
    return False


def _carry_out_is_live(steps: Sequence[Step],
                       sem: Sequence[InstructionSemantics], i: int) -> bool:
    """Is the flag produced by step i consumed later before being overwritten?

    A reader is checked before a writer: the ``S`` forms both consume the
    incoming flag as carry-in and produce a new one, so an ``ADCS`` directly
    after an ``ADDS`` does consume the ``ADDS`` carry-out.
    """
    for j in range(i + 1, len(steps)):
        if _is_flag_reader(steps[j].mnemonic, sem[j]):
            return True
        if _is_flag_writer(sem[j]):
            return False
    return False


def derive_accumulate(
    exec_steps: Sequence[Step],
    semantics: Mapping[str, InstructionSemantics] = DEFAULT_SEMANTICS,
) -> list[int]:
    """Return the sorted 1-based accumulating step numbers for *exec_steps*.

    The decision procedure is described in the module docstring.  It is total:
    every step list yields a list (possibly empty), and no exception is raised
    for unknown mnemonics -- an unknown mnemonic is simply not an add/sub form.
    """
    steps = list(exec_steps)
    sem = [semantics.get(s.mnemonic, _NOACC) for s in steps]

    derived: list[int] = []
    for i, step in enumerate(steps):
        s = sem[i]
        if s.form not in ("ADD", "SUB"):
            continue

        # 1. The instruction must write a real general-purpose register.
        if s.dest is None or s.dest >= len(step.operands):
            continue
        if not _is_general_register(step.operands[s.dest]):
            continue

        # 2. It must have at least one real register source: `ADC Xd, XZR, XZR`
        #    only materialises the incoming flag and is not a digit sum.
        if not any(_is_general_register(o)
                   for o in _source_operands(step, s)):
            continue

        # 3. Carry/flag dataflow must justify taking an accumulator relation.
        if s.reads_carry and not _carry_in_is_live(sem, i):
            continue
        if s.writes_carry and not _carry_out_is_live(steps, sem, i):
            continue

        derived.append(step.index)

    return sorted(derived)


__all__ = [
    "Step",
    "InstructionSemantics",
    "DEFAULT_SEMANTICS",
    "derive_accumulate",
]
