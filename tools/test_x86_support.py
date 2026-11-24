#!/usr/bin/env python3
"""Tests for the x86 step derivation, flag mapping and reference emitter.

The fast tests need no HOL Light and no benchmark tree.  The real-object tests
skip when `.bench/s2n-bignum/x86` is absent.
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

HERE = Path(__file__).resolve().parent
REPO = HERE.parent
BENCH = REPO / ".bench"
S2N = BENCH / "s2n-bignum"

sys.path.insert(0, str(HERE))

import derive_steps  # noqa: E402
import disasm_steps  # noqa: E402
import emit_ref_proof_x86  # noqa: E402


def steps(rows):
    return [derive_steps.Step(index=i + 1, mnemonic=m, operands=tuple(o.split()))
            for i, (m, o) in enumerate(rows)]


def x86(rows):
    return derive_steps.derive_accumulate(
        steps(rows), derive_steps.X86_SEMANTICS)


# --------------------------------------------------------------------------
# Flag mapping and materialisation
# --------------------------------------------------------------------------


def test_x86_sub_flag_is_borrow_and_arm_is_no_borrow():
    """The mapping is explicit: x86 SUB/SBB CF is a borrow.

    On AArch64 the C flag after SUBS/SBCS is the *inverse*; the two tables must
    not be interchangeable, and the documented meanings must say so.
    """
    assert "borrow" in derive_steps.X86_CF_MEANING["sub"]
    assert "borrow" in derive_steps.X86_CF_MEANING["sbb"]
    assert "borrow" in derive_steps.X86_CF_MEANING["neg"]
    assert "borrow" in derive_steps.X86_CF_MEANING["cmp"]
    assert "carry" in derive_steps.X86_CF_MEANING["adc"]
    # x86 value forms read and write the flag; the ARM S-suffix forms do not
    # read it except through ADC/SBC.
    assert derive_steps.X86_SEMANTICS["sbb"].reads_carry
    assert derive_steps.X86_SEMANTICS["sub"].reads_carry is False
    assert derive_steps.X86_SEMANTICS["sub"].writes_carry


def test_x86_self_carry_materialisation_is_excluded():
    """``ADC r,r`` / ``SBB r,r`` copy the flag; they sum no two digits."""
    assert x86([
        ("add", "rax rbx"),     # 1: carry-out consumed by 2
        ("adc", "rdx rdx"),     # 2: materialisation -> excluded
        ("sbb", "rcx rdx"),     # 3: reads the carry, writes a digit
        ("mov", "[rdi] rcx"),   # 4
    ]) == [1, 3]


def test_x86_stc_grounds_the_first_carry_in():
    """``STC`` is a flag writer, so the following ADC chain has a carry-in.

    ``STC`` is not itself an add/sub form and takes no accumulator relation;
    it only grounds the carry.
    """
    assert x86([
        ("stc", ""),            # 1: defines CF = 1, no value
        ("mov", "rax [rsi]"),   # 2
        ("adc", "rax [rdx]"),   # 3: carry-in from 1
        ("mov", "[rdi] rax"),   # 4
    ]) == [3]


def test_x86_neg_is_a_value_link_but_is_dropped_by_the_emitter():
    """NEG is a true SUB form; ACCUMULATE has no word_neg pattern for it.

    ``derive_accumulate`` keeps it (it is a digit-sum form), and the emitter
    drops it from the *requested* list because requesting it would be a no-op.
    """
    rows = [
        ("neg", "rax"),         # 1: CF = (rax != 0), read by 2
        ("sbb", "rcx rdx"),     # 2
        ("mov", "[rdi] rcx"),   # 3
    ]
    derived = x86(rows)
    assert 1 in derived and 2 in derived
    assert emit_ref_proof_x86.accumulate_set(steps(rows)) == [2]


def test_x86_logical_form_is_a_chain_boundary_not_a_digit():
    """AND/OR/XOR clear CF and are never accumulator candidates themselves.

    A value-writing ADD/SBB is still a digit sum even when its carry-out is
    dead (``always_link``), so it stays; the logical form only grounds or kills
    the flag for the *liveness* search.
    """
    assert x86([
        ("add", "rax rbx"),     # 1: value link (always_link)
        ("and", "rdx rsi"),     # 2: clears CF; not an add/sub form
        ("adc", "rcx rdx"),     # 3: carry-in grounded by 2
        ("mov", "[rdi] rcx"),   # 4
    ]) == [1, 3]


def test_arm_table_is_unchanged_by_the_x86_work():
    """The ARM default table keeps its own flag meaning and no always_link."""
    assert derive_steps.DEFAULT_SEMANTICS["sbcs"].reads_carry
    assert derive_steps.DEFAULT_SEMANTICS["adds"].always_link is False
    assert derive_steps.X86_SEMANTICS["adc"].always_link is True


# --------------------------------------------------------------------------
# Real x86 objects: trimming, entry and derivation
# --------------------------------------------------------------------------


def _obj(family, op):
    return S2N / "x86" / family / f"bignum_{op}_{family}.o"


@pytest.mark.skipif(not (S2N / "x86" / "p384").is_dir(),
                    reason="x86 s2n-bignum objects not present")
def test_x86_endbr64_is_trimmed_and_addresses_are_trimmed_relative():
    steps_x86 = disasm_steps.disassemble(str(_obj("p384", "add")))
    # define_trimmed drops the four-byte ENDBR64, so the first word is MOV.
    assert steps_x86[0].mnemonic == "mov"
    assert steps_x86[0].address == 0
    assert len(steps_x86) == 42


@pytest.mark.skipif(not (S2N / "x86" / "p521").is_dir(),
                    reason="x86 s2n-bignum objects not present")
def test_x86_entry_offset_skips_the_push_prologue():
    full = disasm_steps.disassemble(str(_obj("p521", "add")))
    assert [s.mnemonic for s in full[:3]] == ["push", "push", "stc"]
    entered = disasm_steps.enter_at(full, 3)
    assert entered[0].mnemonic == "stc"
    assert entered[0].index == 1
    assert entered[0].address == 3


@pytest.mark.skipif(not (S2N / "x86" / "p384").is_dir(),
                    reason="x86 s2n-bignum objects not present")
def test_x86_real_accumulate_sets():
    expected = {
        ("p384", "sub"): [2, 4, 6, 8, 10, 12, 17, 18, 20, 25, 27, 29, 31],
        ("p384", "neg"): [19, 20, 21, 22, 23, 24],
        ("p521", "neg"): [],
    }
    for (family, op), want in expected.items():
        obj = _obj(family, op)
        if not obj.exists():
            pytest.skip(f"missing {obj}")
        steps_x86 = disasm_steps.disassemble(str(obj))
        got = emit_ref_proof_x86.accumulate_set(steps_x86)
        assert got == want, (family, op, got, want)


# --------------------------------------------------------------------------
# Held-out refusal
# --------------------------------------------------------------------------


@pytest.mark.parametrize("task", [
    "x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT",
    "x86.bignum_sub_p256k1.BIGNUM_SUB_P256K1_CORRECT",
    "x86.bignum_neg_sm2.BIGNUM_NEG_SM2_CORRECT",
])
def test_x86_emitter_refuses_heldout(task):
    op = task.split("_", 1)[1].split("_", 1)[0]
    with pytest.raises(SystemExit):
        emit_ref_proof_x86.build(op, task)
