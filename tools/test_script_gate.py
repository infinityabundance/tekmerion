#!/usr/bin/env python3
"""Firing controls for tools/script_gate.py (F1).

The token fallback runs everywhere; the front-end test is skipped when the HOL
Light switch is absent (CI without OCaml). The authoritative measurement is in
research/receipts/ix-f1-script-gate.md.
"""

import sys
from pathlib import Path

import pytest

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import script_gate  # noqa: E402

CLEAN = "(fun g -> (X_GEN_TAC `x:int` THEN REWRITE_TAC[]) g)\n"


def run(tmp_path, text, *args):
    p = tmp_path / "candidate.txt"
    p.write_text(text, encoding="utf-8")
    return script_gate.main([str(p), *args, "--tokens-only"])


def test_clean_expression_is_allowed(tmp_path):
    assert run(tmp_path, CLEAN, "--expr") == 0


def test_obj_magic_is_refused(tmp_path):
    assert run(tmp_path, "(fun g -> (let x = Obj.magic g in x))\n", "--expr") == 1


def test_marshal_is_refused(tmp_path):
    assert run(tmp_path, "(fun g -> ignore (Marshal.to_string g []); g)\n",
               "--expr") == 1


def test_sys_command_is_refused(tmp_path):
    assert run(tmp_path, '(fun g -> ignore (Sys.command "true"); g)\n',
               "--expr") == 1


def test_external_is_refused(tmp_path):
    assert run(tmp_path, 'external evil : int -> int = "evil"\nlet _ = evil 1\n') == 1


def test_shadowed_check_axioms_is_refused(tmp_path):
    assert run(tmp_path, "let check_axioms () = ()\nlet _ = check_axioms ()\n") == 1


def test_rebound_prove_is_refused(tmp_path):
    assert run(tmp_path, "let prove x y = ()\nlet _ = prove 1 2\n") == 1


def test_kernel_name_as_parameter_is_refused(tmp_path):
    assert run(tmp_path, "let f (axioms : int) = axioms\n") == 1


def test_strict_refuses_references(tmp_path):
    assert run(tmp_path, "(fun g -> (let r = ref 0 in (r := 1; g)))\n",
               "--expr", "--profile", "strict") == 1


def test_strict_refuses_sys_even_without_command(tmp_path):
    assert run(tmp_path, "(fun g -> ignore (Sys.getenv \"X\"); g)\n",
               "--expr", "--profile", "strict") == 1


def test_denylist_allows_refs(tmp_path):
    # Deliberate: the strict profile is the target grammar, not the denylist.
    assert run(tmp_path, "(fun g -> (let r = ref 0 in (r := 1; g)))\n",
               "--expr") == 0


@pytest.mark.skipif(not (HERE.parent / ".bench" / "hol-light" / "hol.sh").is_file(),
                    reason="HOL Light switch not present")
def test_front_end_refuses_obj_magic(tmp_path):
    p = tmp_path / "candidate.txt"
    p.write_text("(fun g -> (let x = Obj.magic g in x))\n", encoding="utf-8")
    rc = script_gate.main([str(p), "--expr", "--hol-light",
                           str(HERE.parent / ".bench" / "hol-light")])
    assert rc == 1
