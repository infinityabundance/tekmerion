#!/usr/bin/env python3
"""Tests for the W1 accumulate-step derivation.

The fast tests need no HOL Light.  The end-to-end gate (which runs HOL Light)
is exercised by tools/w1_gate.py and is re-run here only when
TEKMERION_RUN_HOL=1.
"""

from __future__ import annotations

import ast
import os
import subprocess
import sys
from pathlib import Path

import pytest

HERE = Path(__file__).resolve().parent
REPO = HERE.parent
BENCH = REPO / ".bench"
S2N = BENCH / "s2n-bignum"

sys.path.insert(0, str(HERE))

from derive_steps import (  # noqa: E402
    DEFAULT_SEMANTICS,
    InstructionSemantics,
    Step,
    derive_accumulate,
)
import disasm_steps  # noqa: E402
import heldout_guard  # noqa: E402

OPS = ("add", "sub", "neg")


def steps(rows):
    return [Step(index=i + 1, mnemonic=m, operands=tuple(o.split()))
            for i, (m, o) in enumerate(rows)]


# --------------------------------------------------------------------------
# Unit tests on hand-built machine-code fixtures
# --------------------------------------------------------------------------


def test_hand_built_carry_chain():
    """Known answer: a three-deep carry chain plus a flag consumer."""
    exec_steps = steps([
        ("ldp", "x4 x5 [x1]"),
        ("ldp", "x6 x7 [x2]"),
        ("adds", "x4 x4 x6"),    # 3: carry-out consumed by 4
        ("adcs", "x5 x5 x7"),    # 4: carry-in from 3, carry-out to 5
        ("sbcs", "x8 x8 x9"),    # 5: carry-in from 4, carry-out to 6
        ("csetm", "x10 cc"),     # 6: consumes the carry flag
        ("stp", "x4 x5 [x0]"),
        ("ret", ""),
    ])
    assert derive_accumulate(exec_steps) == [3, 4, 5]


def test_carry_materialisation_is_excluded():
    """`ADC Xd, XZR, XZR` only materialises the flag; it is not a digit sum."""
    exec_steps = steps([
        ("adds", "x4 x4 x6"),    # carry-out consumed by 2
        ("adc", "x3 xzr xzr"),   # zero sources -> excluded
        ("csel", "x4 x4 x6 cc"),  # also consumes the flag
    ])
    assert derive_accumulate(exec_steps) == [1]


def test_flag_writer_without_consumer_is_excluded():
    exec_steps = steps([
        ("adds", "x4 x4 x6"),    # flags are never read
        ("ldp", "x7 x8 [x1]"),
        ("stp", "x4 x4 [x0]"),
    ])
    assert derive_accumulate(exec_steps) == []


def test_compare_writing_xzr_is_excluded():
    exec_steps = steps([
        ("cmp", "x2 #0x0"),
        ("csetm", "x2 ne"),
        ("subs", "x4 x2 x4"),    # carry-in from 1, carry-out to 4
        ("csel", "x4 x4 x4 cc"),
    ])
    assert derive_accumulate(exec_steps) == [3]


# --------------------------------------------------------------------------
# Negative test: no flag consumers at all
# --------------------------------------------------------------------------


def test_no_flag_consumers_yields_empty():
    exec_steps = steps([
        ("adds", "x4 x4 x6"),    # produces flags, nobody reads them
        ("ldp", "x7 x8 [x1]"),
        ("stp", "x4 x5 [x0]"),
        ("ret", ""),
    ])
    assert derive_accumulate(exec_steps) == []


def test_ungrounded_carry_in_is_excluded():
    """ADC/SBC with no preceding flag writer has no carry dataflow to use."""
    exec_steps = steps([
        ("adc", "x4 x4 x6"),
        ("sbc", "x5 x5 x7"),
        ("csel", "x4 x4 x6 cc"),
    ])
    assert derive_accumulate(exec_steps) == []


def test_no_accumulate_instructions_yields_empty():
    exec_steps = steps([
        ("ldp", "x4 x5 [x1]"),
        ("orr", "x3 x4 x5"),
        ("and", "x3 x3 #0xffffffff"),
        ("mov", "x9 #0x1"),
        ("stp", "x4 x5 [x0]"),
        ("ret", ""),
    ])
    assert derive_accumulate(exec_steps) == []


def test_unknown_mnemonic_is_not_a_candidate():
    exec_steps = steps([("frobnicate", "x4 x5 x6")])
    assert derive_accumulate(exec_steps) == []


def test_custom_semantics_is_honoured():
    sem = dict(DEFAULT_SEMANTICS)
    sem["vadds"] = InstructionSemantics("ADD", dest=0, sources=(1, 2),
                                        writes_carry=True)
    exec_steps = steps([("vadds", "x4 x4 x6"), ("csel", "x4 x4 x6 cc")])
    assert derive_accumulate(exec_steps, sem) == [1]


# --------------------------------------------------------------------------
# No-peek enforcement
# --------------------------------------------------------------------------


def test_derive_module_has_no_io_and_no_proof_vocabulary():
    src = (HERE / "derive_steps.py").read_text(encoding="utf-8")
    tree = ast.parse(src)

    imported = set()
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            imported.update(a.name.split(".")[0] for a in node.names)
        elif isinstance(node, ast.ImportFrom) and node.module:
            imported.add(node.module.split(".")[0])
    assert imported <= {"__future__", "dataclasses", "typing"}, imported

    forbidden = [
        "ARM_ACCSTEPS_TAC", "p256", "p384", "p521", "p_256", "25519",
        "secp256k1", "sm2", "human", "read_text", "sys.argv",
    ]
    for token in forbidden:
        assert token not in src, f"derive_steps.py mentions {token!r}"

    # No file or process access anywhere in the module, including the code
    # objects reachable from derive_accumulate.
    for node in ast.walk(tree):
        if isinstance(node, ast.Call):
            f = node.func
            name = (f.id if isinstance(f, ast.Name)
                    else f.attr if isinstance(f, ast.Attribute) else "")
            assert name not in {"open", "system", "popen", "run", "os"}, \
                f"derive_steps.py calls {name!r}"


def test_derive_signature_takes_only_steps_and_semantics():
    import inspect
    params = list(inspect.signature(derive_accumulate).parameters)
    assert params == ["exec_steps", "semantics"]
    default = inspect.signature(derive_accumulate).parameters["semantics"].default
    assert default is DEFAULT_SEMANTICS


# --------------------------------------------------------------------------
# Real machine code: list-level comparison with the human tactic calls
# --------------------------------------------------------------------------

def _human_lists(op):
    """Extract the human accumulate calls, for list-level comparison only."""
    import importlib
    gate = importlib.import_module("w1_gate")
    proof = S2N / "arm" / "proofs" / f"bignum_{op}_p256.ml"
    heldout_guard.check_path(str(proof), root=S2N)
    calls = gate.extract_human_accsteps(proof)
    union = []
    for c in calls:
        for n in c["anums"]:
            if n not in union:
                union.append(n)
    return calls, sorted(union)


@pytest.mark.skipif(not (S2N / "arm" / "p256").is_dir(),
                    reason="s2n-bignum bench not present")
@pytest.mark.parametrize("op", OPS)
def test_real_object_derivation(op):
    obj = S2N / "arm" / "p256" / f"bignum_{op}_p256.o"
    if not obj.exists():
        pytest.skip(f"missing object {obj}")
    steps_list = disasm_steps.disassemble(str(obj))
    derived = derive_accumulate(steps_list)
    calls, human = _human_lists(op)

    # Derived steps must all exist and be genuinely accumulating instructions.
    assert set(derived) <= set(human), (derived, human)
    assert derived, f"empty derivation for {op}"

    if op in ("add", "neg"):
        # The human list is already minimal for these two routines.
        assert derived == human
    else:
        # The human sub proof uses blanket ranges (1--8) and (10--17); the
        # derived set is the carry-chain subset.  Keep that expectation
        # explicit so a regression is visible.
        assert derived == [3, 4, 7, 8, 10, 12, 13, 15]
        assert set(derived) < set(human)


# --------------------------------------------------------------------------
# End-to-end gate (expensive; opt in)
# --------------------------------------------------------------------------


@pytest.mark.skipif(os.environ.get("TEKMERION_RUN_HOL") != "1",
                    reason="set TEKMERION_RUN_HOL=1 to run the HOL gate")
def test_real_gate(tmp_path):
    """Compare against a real HOL capture (reuses an existing capture.out)."""
    capture = BENCH / "verify" / "w1" / "capture.out"
    if not capture.exists():
        pytest.skip("no capture.out; run tools/w1_gate.py first")
    json_out = tmp_path / "gate.json"
    proc = subprocess.run(
        [sys.executable, str(HERE / "w1_gate.py"), "--skip-hol",
         "--receipt", str(tmp_path / "gate.md"),
         "--json", str(json_out)],
        capture_output=True, text=True, timeout=600,
    )
    import json
    receipt = json.loads(json_out.read_text())
    assert receipt["operations"]["add"]["match"] is True
    assert receipt["operations"]["neg"]["match"] is True
    # The sub human proof accumulates on blanket ranges; the literal goal
    # differs.  Record the exact status rather than hiding it.
    sub = receipt["operations"]["sub"]
    assert sub["conclusion_equal"] is True
    assert sub["closed_ok"] is True
    assert sub["match"] is False
    assert proc.returncode == 1
