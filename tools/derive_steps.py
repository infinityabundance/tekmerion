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
(optionally plus/minus an incoming carry flag).  Such a chain can only be
*used* if

  * every carry-in it consumes was produced by an earlier flag write, and
  * every carry-out it produces is consumed by a later flag read,

so those are the liveness conditions checked below.  Two further rules matter
for reproducing the reference accumulator context:

  * the instruction must read at least one real register: one whose *sources*
    are all zero registers or immediates is a carry *materialisation*
    (``ADC Xd, XZR, XZR``), not a digit sum, and has no accumulator relation of
    its own,
  * the instruction must either write a general-purpose register or be a
    *flag-only* add/sub form -- ``CMP``/``CMN`` (no destination) or ``ADCS``/
    ``SBCS`` to ``XZR``, and
  * a flag-only form has no value output, so it is a link in the chain only if
    its carry-out is read as a carry-*in* by a later add/sub that does write a
    register.  A ``CSEL``/``CSETM`` that turns the flag into a mask is a
    predicate, not a carry, and an accumulator relation for it would relate a
    compare to a word it never feeds.

The last rule matters because a flag-only form that *is* in the chain defines
the flag the consumer selects on: without a relation for it the flag survives
only as an arithmetic comparison over the previous flag, which the word-level
SAT encoding cannot see, and the branch it selects is left free.

``SP`` as a destination is neither a value write for accumulation (the stack
pointer is not part of the digit sum) nor a flag-only form, so it is excluded.

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
    address: int = 0


@dataclass(frozen=True)
class InstructionSemantics:
    """What one mnemonic does to registers and the carry flag.

    ``form``          ``"ADD"``, ``"SUB"`` or ``None`` (not an add/sub form).
    ``dest``          operand index written, or ``None`` (flag-only compare).
    ``sources``       operand indices that are read as values.
    ``reads_carry``   consumes the incoming carry flag.
    ``writes_carry``  produces a carry flag.
    ``reads_flags``   consumes flags for a conditional operation or branch.
    ``self_carry_materialisation``
                      the two-operand form ``d = d +/- d +/- carry`` copies the
                      carry into ``d`` instead of summing two digits (x86
                      ``ADC r,r`` / ``SBB r,r``).  It is excluded only when
                      every source operand is the destination operand; a
                      genuine ``ADC d, s`` with distinct operands accumulates.
    ``always_link``   a value-writing add/sub is a digit sum whether or not its
                      carry-out is later read.  The dead-carry case is the top
                      limb of a multi-limb add/sub: the relation is taken to
                      expose the digit, not to connect a flag.  AArch64 leaves
                      this false (the reference proofs always consume the top
                      carry), x86 sets it for the value forms.
    """

    form: Optional[str]
    dest: Optional[int] = None
    sources: Tuple[int, ...] = ()
    reads_carry: bool = False
    writes_carry: bool = False
    reads_flags: bool = False
    self_carry_materialisation: bool = False
    always_link: bool = False


_NOACC = InstructionSemantics(form=None)

# --------------------------------------------------------------------------
# AArch64 table
#
# Only the ARM64 add/sub-with-carry family is modelled here; every other
# mnemonic is a non-candidate for accumulation.  The table is data, not policy:
# the policy (liveness, zero-register exclusions) lives in derive_accumulate.
# --------------------------------------------------------------------------
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

# --------------------------------------------------------------------------
# x86-64 table
#
# Two differences from AArch64 are load-bearing:
#
#  1. The flag semantics are inverted for subtraction.  On AArch64 the C flag
#     after SUBS/SBCS is "no borrow" (set iff minuend >= subtrahend + carry).
#     On x86 the CF flag after SUB/SBB/NEG/CMP is "borrow" (set iff
#     minuend < subtrahend + carry).  The accumulator relation built by
#     ACCUMULATE_ARITH_TAC still has the same real shape on both, but the
#     *meaning* of its ``carry_sN`` bit is the opposite on each side.  A proof
#     that reads the flag as on AArch64 would be wrong on x86; the reference
#     route used here never interprets the flag, and this comment is what a
#     route that does interpret it must start from.  See X86_CF_MEANING.
#
#  2. Every x86 ADD/SUB/ADC/SBB writes the flags unconditionally (there is no
#     ``S`` suffix).  The two-operand form is ``d = d +/- src``, so operand 0
#     is both the destination and a source for the value forms.

X86_CF_MEANING: Mapping[str, str] = {
    "add": "carry (unsigned overflow): set iff val d + val s + c >= 2^64",
    "adc": "carry (unsigned overflow): set iff val d + val s + c >= 2^64",
    "sub": "borrow: set iff val d < val s",
    "sbb": "borrow: set iff val d < val s + c",
    "neg": "borrow: set iff val d != 0",
    "cmp": "borrow: set iff val d < val s",
}

X86_SEMANTICS: Mapping[str, InstructionSemantics] = {
    # d = d + s
    "add": InstructionSemantics("ADD", dest=0, sources=(0, 1),
                                writes_carry=True, always_link=True),
    # d = d + s + CF; ADC r,r copies CF into r
    "adc": InstructionSemantics("ADD", dest=0, sources=(0, 1),
                                reads_carry=True, writes_carry=True,
                                self_carry_materialisation=True,
                                always_link=True),
    # d = d - s
    "sub": InstructionSemantics("SUB", dest=0, sources=(0, 1),
                                writes_carry=True, always_link=True),
    # d = d - s - CF; SBB r,r copies -CF into r
    "sbb": InstructionSemantics("SUB", dest=0, sources=(0, 1),
                                reads_carry=True, writes_carry=True,
                                self_carry_materialisation=True,
                                always_link=True),
    # d = 0 - d  (CF is the borrow, i.e. d != 0)
    "neg": InstructionSemantics("SUB", dest=0, sources=(0,),
                                writes_carry=True, always_link=True),
    # flag-only forms: no value destination
    "cmp": InstructionSemantics("SUB", dest=None, sources=(0, 1),
                                writes_carry=True),
    # CF-only writers that are not value adds/subs
    "stc": InstructionSemantics(form=None, writes_carry=True),
    "clc": InstructionSemantics(form=None, writes_carry=True),
    "cmc": InstructionSemantics(form=None, writes_carry=True),
    # Logical forms clear CF (and OF), so they kill an earlier carry-out.
    "and": InstructionSemantics(form=None, writes_carry=True),
    "or": InstructionSemantics(form=None, writes_carry=True),
    "xor": InstructionSemantics(form=None, writes_carry=True),
    "test": InstructionSemantics(form=None, writes_carry=True),
    # ADCX reads and writes CF only; ADOX drives OF, not CF, and is absent.
    "adcx": InstructionSemantics("ADD", dest=0, sources=(0, 1),
                                 reads_carry=True, writes_carry=True,
                                 self_carry_materialisation=True,
                                 always_link=True),
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
    """True for a general-purpose register of either architecture.

    AArch64: ``x0..x30`` / ``w0..w30``; ``xzr``/``wzr``/``sp``/``wsp`` are not
    value registers.  x86-64: the 64/32/16/8-bit names of RAX..R15, with RSP
    excluded for the same reason as SP (the stack pointer is not a digit).
    Memory operands (they contain ``[``) and immediates are excluded.
    """
    if not operand:
        return False
    if "[" in operand:
        return False
    if operand in ("xzr", "wzr", "sp", "wsp"):
        return False
    # AArch64
    if operand[0] in ("x", "w"):
        digits = operand[1:]
        if digits.isdigit() and 0 <= int(digits) <= 30:
            return True
    # x86-64
    if operand not in {"rsp", "esp", "sp", "spl"} and operand in _X86_GPRS:
        return True
    return False


_X86_GPRS = frozenset({
    "rax", "rbx", "rcx", "rdx", "rsi", "rdi", "rbp", "rsp",
    "eax", "ebx", "ecx", "edx", "esi", "edi", "ebp", "esp",
    "ax", "bx", "cx", "dx", "si", "di", "bp", "sp",
    "al", "ah", "bl", "bh", "cl", "ch", "dl", "dh",
    "sil", "dil", "bpl", "spl",
    "r8", "r9", "r10", "r11", "r12", "r13", "r14", "r15",
    "r8d", "r9d", "r10d", "r11d", "r12d", "r13d", "r14d", "r15d",
    "r8w", "r9w", "r10w", "r11w", "r12w", "r13w", "r14w", "r15w",
    "r8b", "r9b", "r10b", "r11b", "r12b", "r13b", "r14b", "r15b",
})


_ZERO_REGISTERS = frozenset({"xzr", "wzr"})


def _dest_operand(step: Step, sem: InstructionSemantics):
    """The written operand, or ``None`` for a form with no destination."""
    if sem.dest is None or sem.dest >= len(step.operands):
        return None
    return step.operands[sem.dest]


def _writes_value(step: Step, sem: InstructionSemantics) -> bool:
    """Does the instruction leave a value in a general-purpose register?"""
    dest = _dest_operand(step, sem)
    return dest is not None and _is_general_register(dest)


def _is_flag_only(step: Step, sem: InstructionSemantics) -> bool:
    """A compare, or an add/sub-with-carry whose destination is a zero register.

    These write the flags and no value: ``CMP``/``CMN`` (no destination at all)
    and ``ADCS``/``SBCS``/``CCMP``/``CCMN`` to ``XZR``.  An ``SP`` destination is
    deliberately *not* flag-only here: it does write a value, just not one the
    digit sum uses.
    """
    dest = _dest_operand(step, sem)
    return dest is None or dest in _ZERO_REGISTERS


def _source_operands(step: Step, sem: InstructionSemantics) -> Tuple[str, ...]:
    out = []
    for k in sem.sources:
        if k < len(step.operands):
            out.append(step.operands[k])
    return tuple(out)


def _is_self_carry_materialisation(step: Step,
                                  sem: InstructionSemantics) -> bool:
    """``ADC r,r`` / ``SBB r,r``: copy the carry into r, sum no two digits."""
    if not sem.self_carry_materialisation:
        return False
    dest = _dest_operand(step, sem)
    if dest is None:
        return False
    sources = _source_operands(step, sem)
    return bool(sources) and all(o == dest for o in sources)


# --------------------------------------------------------------------------
# Derivation
# --------------------------------------------------------------------------


def _carry_in_is_live(sem: Sequence[InstructionSemantics], i: int) -> bool:
    """Is the incoming flag of step i produced by an earlier flag write?

    This is a backward reaching-definition search for the C flag.  A flag
    **writer** starts a new definition and therefore grounds the carry-in; a
    flag **reader** does not write the flag, so it is *transparent* to the
    definition and the search continues past it.  This matters for
    ``ADDS; ADC; ADCS``: the intervening ``ADC`` reads and preserves C, so the
    final ``ADCS`` legitimately depends on the ``ADDS`` definition.  Only
    running off the start of the program leaves the carry-in ungrounded.
    """
    for j in range(i - 1, -1, -1):
        if _is_flag_writer(sem[j]):
            return True
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


def _carry_out_feeds_chain(steps: Sequence[Step],
                           sem: Sequence[InstructionSemantics], i: int) -> bool:
    """Does step i's carry-out reach a later add/sub that writes a register?

    This is the liveness condition for a *flag-only* form.  Unlike
    :func:`_carry_out_is_live` it does not count a bare flag reader
    (``CSEL``/``CSETM``) as the consumer: such an instruction consumes the flag
    as a predicate, so relating the flag to a word buys nothing and may relate
    a compare to a word its chain never reaches.  The chain continues through
    flag-only add/sub forms (they consume the carry and redefine it) and is
    killed by any other flag writer.
    """
    for j in range(i + 1, len(steps)):
        s = sem[j]
        if s.form in ("ADD", "SUB") and s.reads_carry:
            if _writes_value(steps[j], s):
                return True
            if s.writes_carry:
                continue
            return False
        if _is_flag_writer(s):
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

        # 1. Either it writes a general-purpose register, or it is a flag-only
        #    form (whose own claim to a relation is checked in 3).  `ADC Xd,
        #    XZR, XZR` writes Xd but fails 2; `SUBS SP, ...` is excluded by both.
        writes_value = _writes_value(step, s)
        flag_only = _is_flag_only(step, s)
        if not (writes_value or flag_only):
            continue

        # 2. It must have at least one real register source: `ADC Xd, XZR, XZR`
        #    only materialises the incoming flag and is not a digit sum, and a
        #    flag-only form with zero sources constrains no word at all.  The
        #    x86 `ADC r,r` / `SBB r,r` forms are the same idea with d as its own
        #    source, so they are excluded by their semantics flag.
        if _is_self_carry_materialisation(step, s):
            continue
        if not any(_is_general_register(o)
                   for o in _source_operands(step, s)):
            continue

        # 3. Carry/flag dataflow must justify taking an accumulator relation.
        if s.reads_carry and not _carry_in_is_live(sem, i):
            continue
        if flag_only:
            if not _carry_out_feeds_chain(steps, sem, i):
                continue
        elif s.writes_carry and not s.always_link \
                and not _carry_out_is_live(steps, sem, i):
            continue

        derived.append(step.index)

    return sorted(derived)


__all__ = [
    "Step",
    "InstructionSemantics",
    "DEFAULT_SEMANTICS",
    "X86_SEMANTICS",
    "X86_CF_MEANING",
    "derive_accumulate",
]
