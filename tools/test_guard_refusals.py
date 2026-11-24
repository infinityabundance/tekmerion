#!/usr/bin/env python3
"""Refusal coverage for the held-out guard (Ordeal O9.3 firing controls).

``tools/mutate_guard.py`` measured the guard at 22/71 killed.  This file adds
one test per reachable refusal site so the score can be re-measured honestly.
Each test calls the smallest public or private entry point that reaches the
site and asserts ``HeldOutRefused``.

Sites that cannot be reached without lying about the filesystem (for example a
race between ``stat`` and ``isfile``) are not simulated; they are named in the
receipt if they remain survivors.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path

import pytest

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import heldout_guard as g  # noqa: E402

SEALED_ID = "x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT"
SPENT_ID = "arm.bignum_add_p384.BIGNUM_ADD_P384_CORRECT"
COMMIT = "a" * 40


def _sealed(**kw):
    payload = {"freeze_commit": COMMIT, "sealed_ids": [SEALED_ID], "note": ""}
    payload.update(kw)
    return payload


def _write(tmp_path: Path, name: str, payload) -> Path:
    p = tmp_path / name
    p.write_text(payload if isinstance(payload, str) else json.dumps(payload),
                 encoding="utf-8")
    return p


def _sealed_manifest(tmp_path: Path):
    return g.load_sealed_manifest(_write(tmp_path, "s.json", _sealed()))


def _policy(mode, manifest=None):
    return g.GuardPolicy(mode, manifest=manifest)


# --- _check_regular / check_path -------------------------------------------


def test_check_regular_empty_path():
    with pytest.raises(g.HeldOutRefused, match="empty path"):
        g._check_regular("")


def test_check_regular_cannot_stat(tmp_path):
    with pytest.raises(g.HeldOutRefused, match="cannot stat"):
        g._check_regular(str(tmp_path / "nope"))


def test_check_regular_not_a_file(tmp_path):
    with pytest.raises(g.HeldOutRefused, match="not a regular file"):
        g._check_regular(str(tmp_path))


def test_check_regular_outside_root(tmp_path):
    inner = tmp_path / "inner"
    inner.mkdir()
    outside = tmp_path / "outside.txt"
    outside.write_text("x")
    with pytest.raises(g.HeldOutRefused, match="outside allowed root"):
        g._check_regular(str(outside), root=str(inner))


def test_check_regular_empty_file(tmp_path):
    empty = tmp_path / "empty.bin"
    empty.write_bytes(b"")
    with pytest.raises(g.HeldOutRefused, match="empty file"):
        g._check_regular(str(empty))


def test_check_path_empty_held_out():
    with pytest.raises(g.HeldOutRefused, match="empty held-out list"):
        g.check_path("anything", held_out=())


def test_check_path_empty_path():
    with pytest.raises(g.HeldOutRefused, match="empty path"):
        g.check_path("")


def test_check_path_marker(tmp_path):
    p = tmp_path / "p25519.txt"
    p.write_text("x")
    with pytest.raises(g.HeldOutRefused, match="held-out marker"):
        g.check_path(str(p))


# --- sealed manifest -------------------------------------------------------


def test_sealed_no_path():
    with pytest.raises(g.HeldOutRefused, match="no sealed manifest"):
        g.load_sealed_manifest("")


def test_sealed_symlink(tmp_path):
    real = _write(tmp_path, "real.json", _sealed())
    link = tmp_path / "link.json"
    os.symlink(real, link)
    with pytest.raises(g.HeldOutRefused, match="may not be a symlink"):
        g.load_sealed_manifest(str(link))


def test_sealed_cannot_stat(tmp_path):
    with pytest.raises(g.HeldOutRefused, match="cannot stat sealed manifest"):
        g.load_sealed_manifest(str(tmp_path / "nope.json"))


def test_sealed_not_a_file(tmp_path):
    with pytest.raises(g.HeldOutRefused, match="not a regular file"):
        g.load_sealed_manifest(str(tmp_path))


def test_sealed_empty(tmp_path):
    p = tmp_path / "empty.json"
    p.write_bytes(b"")
    with pytest.raises(g.HeldOutRefused, match="sealed manifest is empty"):
        g.load_sealed_manifest(str(p))


def test_sealed_not_object(tmp_path):
    p = _write(tmp_path, "arr.json", "[]")
    with pytest.raises(g.HeldOutRefused, match="must be a JSON object"):
        g.load_sealed_manifest(str(p))


def test_sealed_ids_missing(tmp_path):
    p = _write(tmp_path, "m.json", {"freeze_commit": COMMIT})
    with pytest.raises(g.HeldOutRefused, match="sealed_ids must be a non-empty"):
        g.load_sealed_manifest(str(p))


def test_sealed_id_not_string(tmp_path):
    p = _write(tmp_path, "m.json", _sealed(sealed_ids=[123]))
    with pytest.raises(g.HeldOutRefused, match="sealed id is not a string"):
        g.load_sealed_manifest(str(p))


def test_sealed_bad_tag(tmp_path):
    p = _write(tmp_path, "m.json", _sealed(freeze_tag="bad tag"))
    with pytest.raises(g.HeldOutRefused, match="freeze_tag is not a valid"):
        g.load_sealed_manifest(str(p))


def test_sealed_note_not_string(tmp_path):
    p = _write(tmp_path, "m.json", _sealed(note=5))
    with pytest.raises(g.HeldOutRefused, match="note must be a string"):
        g.load_sealed_manifest(str(p))


# --- spent manifest --------------------------------------------------------


def test_spent_no_path():
    with pytest.raises(g.HeldOutRefused, match="no spent manifest"):
        g.load_spent_manifest("")


def test_spent_symlink(tmp_path):
    real = _write(tmp_path, "real.json", {"decision": "research/receipts/x.md",
                                          "spent": {SPENT_ID: "r"}})
    link = tmp_path / "link.json"
    os.symlink(real, link)
    with pytest.raises(g.HeldOutRefused, match="may not be a symlink"):
        g.load_spent_manifest(str(link))


def test_spent_cannot_stat(tmp_path):
    with pytest.raises(g.HeldOutRefused, match="cannot stat spent manifest"):
        g.load_spent_manifest(str(tmp_path / "nope.json"))


def test_spent_not_a_file(tmp_path):
    with pytest.raises(g.HeldOutRefused, match="not a regular file"):
        g.load_spent_manifest(str(tmp_path))


def test_spent_empty(tmp_path):
    p = tmp_path / "empty.json"
    p.write_bytes(b"")
    with pytest.raises(g.HeldOutRefused, match="spent manifest is empty"):
        g.load_spent_manifest(str(p))


def test_spent_not_object(tmp_path):
    p = _write(tmp_path, "arr.json", "[]")
    with pytest.raises(g.HeldOutRefused, match="must be a JSON object"):
        g.load_spent_manifest(str(p))


def test_spent_unparseable(tmp_path):
    p = _write(tmp_path, "m.json", "{not json")
    with pytest.raises(g.HeldOutRefused, match="cannot parse spent manifest"):
        g.load_spent_manifest(str(p))


def test_spent_empty_decision(tmp_path):
    p = _write(tmp_path, "m.json",
               {"decision": "   ", "spent": {SPENT_ID: "r"}})
    with pytest.raises(g.HeldOutRefused, match="non-empty decision"):
        g.load_spent_manifest(str(p))


def test_spent_note_not_string(tmp_path):
    p = _write(tmp_path, "m.json",
               {"decision": "research/receipts/x.md",
                "spent": {SPENT_ID: "r"}, "note": 5})
    with pytest.raises(g.HeldOutRefused, match="note must be a string"):
        g.load_spent_manifest(str(p))


# --- verify_freeze ---------------------------------------------------------


def test_freeze_bad_tag(tmp_path):
    m = _sealed_manifest(tmp_path)
    with pytest.raises(g.HeldOutRefused, match="not a valid tag name"):
        g.verify_freeze(m, "bad tag")


def test_git_cannot_run(tmp_path):
    m = _sealed_manifest(tmp_path)
    with pytest.raises(g.HeldOutRefused, match="cannot run git"):
        g.verify_freeze(m, "t", repo=tmp_path / "nope")


def test_git_command_fails(tmp_path):
    m = _sealed_manifest(tmp_path)
    with pytest.raises(g.HeldOutRefused, match="failed in"):
        g.verify_freeze(m, "t", repo=tmp_path)


# --- GuardPolicy construction ---------------------------------------------


def test_policy_unknown_mode():
    with pytest.raises(g.HeldOutRefused, match="unknown guard mode"):
        g.GuardPolicy("bogus")


def test_policy_empty_held_out():
    with pytest.raises(g.HeldOutRefused, match="empty held-out list"):
        g.GuardPolicy(g.MODE_DEVELOP, held_out=())


def test_policy_sealed_needs_manifest():
    with pytest.raises(g.HeldOutRefused, match="sealed-eval requires a sealed"):
        g.GuardPolicy(g.MODE_SEALED_EVAL)


def test_policy_spent_needs_manifest():
    with pytest.raises(g.HeldOutRefused, match="spent requires a spent"):
        g.GuardPolicy(g.MODE_SPENT)


# --- check_task / check_object / check_proof / check_path ------------------


def test_task_not_a_string():
    with pytest.raises(g.HeldOutRefused, match="not a non-empty string"):
        g.develop_policy().check_task(None)


def test_task_marker_in_develop():
    with pytest.raises(g.HeldOutRefused, match="held-out marker"):
        g.develop_policy().check_task(SEALED_ID)


def test_task_malformed_in_admitted_mode(tmp_path):
    pol = _policy(g.MODE_SEALED_EVAL, _sealed_manifest(tmp_path))
    with pytest.raises(g.HeldOutRefused, match="not in the declared task-id"):
        pol.check_task("25519")


def test_object_empty_path():
    with pytest.raises(g.HeldOutRefused, match="empty path"):
        g.develop_policy().check_object("")


def test_object_looks_like_proof():
    with pytest.raises(g.HeldOutRefused, match="not an object path"):
        g.develop_policy().check_object("arm/proofs/whatever.ml")


def test_object_marker_in_develop():
    with pytest.raises(g.HeldOutRefused, match="held-out marker"):
        g.develop_policy().check_object("arm/p25519/bignum_add_p25519.o")


def test_object_not_a_bignum_object(tmp_path):
    pol = _policy(g.MODE_SEALED_EVAL, _sealed_manifest(tmp_path))
    with pytest.raises(g.HeldOutRefused, match="not a declared bignum object"):
        pol.check_object("arm/p25519/notbignum_p25519.o")


def test_proof_empty_path():
    with pytest.raises(g.HeldOutRefused, match="empty path"):
        g.develop_policy().check_proof("")


def test_proof_marker_in_develop():
    with pytest.raises(g.HeldOutRefused, match="held-out marker"):
        g.develop_policy().check_proof("arm/proofs/bignum_add_p25519.ml")


def test_path_empty():
    with pytest.raises(g.HeldOutRefused, match="empty path"):
        g.develop_policy().check_path("")


# --- policy_from_args ------------------------------------------------------


def _ns(**kw):
    base = dict(develop=False, sealed_eval=False, sealed_manifest=None,
                freeze_tag=None, spent=False, spent_manifest=None,
                held_out=None)
    base.update(kw)
    return argparse.Namespace(**base)


def test_args_spent_without_manifest():
    with pytest.raises(g.HeldOutRefused, match="--spent requires --spent-manifest"):
        g.policy_from_args(_ns(spent=True))


def test_args_manifest_without_sealed():
    with pytest.raises(g.HeldOutRefused, match="require --sealed-eval"):
        g.policy_from_args(_ns(sealed_manifest="x.json"))


def test_args_sealed_with_spent_manifest():
    with pytest.raises(g.HeldOutRefused, match="not --sealed-eval"):
        g.policy_from_args(_ns(sealed_eval=True, sealed_manifest="x.json",
                               freeze_tag="t", spent_manifest="y.json"))


def test_args_sealed_without_manifest():
    with pytest.raises(g.HeldOutRefused, match="requires --sealed-manifest"):
        g.policy_from_args(_ns(sealed_eval=True, freeze_tag="t"))


def test_args_sealed_without_tag():
    with pytest.raises(g.HeldOutRefused, match="requires --freeze-tag"):
        g.policy_from_args(_ns(sealed_eval=True, sealed_manifest="x.json"))
