#!/usr/bin/env python3
"""IX.52 -- two explicit, fail-closed held-out guard modes.

The six behaviours under test:

1. ``--develop`` still refuses a held-out task id and its object.
2. ``--sealed-eval`` with the nine declared ids allows them (and their objects).
3. ``--sealed-eval`` still refuses an unlisted held-out id/object.
4. ``--sealed-eval`` still refuses a listed task's proof ``.ml`` (also on the
   ``--mode dev`` proof-read path).
5. A wrong tag, a wrong recorded commit, or a dirty ``tools/`` tree is refused.
6. Duplicate or malformed manifest entries are refused.

Freeze verification is exercised against a throw-away git repository so the
tests do not depend on the real repository's tag or working-tree state.
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest

HERE = Path(__file__).resolve().parent
REPO = HERE.parent
S2N = REPO / ".bench" / "s2n-bignum"

sys.path.insert(0, str(HERE))

import emit_ref_proof  # noqa: E402
import emit_ref_proof_x86  # noqa: E402
import heldout_guard  # noqa: E402

FREEZE_TAG = "freeze-test-v1"

# The declared sealed set from research/receipts/ix51-freeze-manifest.md.
NINE = [
    "x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT",
    "x86.bignum_sub_p25519.BIGNUM_SUB_P25519_CORRECT",
    "x86.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT",
    "x86.bignum_add_p256k1.BIGNUM_ADD_P256K1_CORRECT",
    "x86.bignum_sub_p256k1.BIGNUM_SUB_P256K1_CORRECT",
    "x86.bignum_neg_p256k1.BIGNUM_NEG_P256K1_CORRECT",
    "x86.bignum_add_sm2.BIGNUM_ADD_SM2_CORRECT",
    "x86.bignum_sub_sm2.BIGNUM_SUB_SM2_CORRECT",
    "x86.bignum_neg_sm2.BIGNUM_NEG_SM2_CORRECT",
]


# --------------------------------------------------------------------------
# Helpers
# --------------------------------------------------------------------------


def _task_parts(task: str):
    head = task.rsplit(".", 1)[0]
    _arch, rest = head.split(".", 1)
    _bignum, op, family = rest.split("_", 2)
    return op, family


def _object(op: str, family: str):
    hits = sorted(S2N.glob(f"x86/*/bignum_{op}_{family}.o"))
    return hits[0] if len(hits) == 1 else None


def _proof(op: str, family: str) -> Path:
    return S2N / "x86" / "proofs" / f"bignum_{op}_{family}.ml"


def _git(repo: Path, *args: str) -> str:
    proc = subprocess.run(["git", "--no-optional-locks", *args], cwd=repo,
                          capture_output=True, text=True, check=True)
    return proc.stdout.strip()


def _init_repo(tmp_path: Path):
    """A clean repository whose tracked tools/ file is unmodified."""
    repo = tmp_path / "repo"
    (repo / "tools").mkdir(parents=True)
    (repo / "tools" / "tracked.py").write_text("# tracked\n", encoding="utf-8")
    _git(repo, "init", "-q")
    _git(repo, "add", "tools/tracked.py")
    _git(repo, "-c", "user.name=infinityabundance",
         "-c", "user.email=255699974+infinityabundance@users.noreply.github.com",
         "-c", "commit.gpgsign=false", "commit", "-q", "-m", "freeze")
    _git(repo, "tag", FREEZE_TAG)
    return repo, _git(repo, "rev-parse", "HEAD")


def _write_manifest(tmp_path: Path, ids, commit: str, name: str,
                    tag=FREEZE_TAG) -> Path:
    payload = {"freeze_commit": commit, "sealed_ids": list(ids)}
    if tag is not None:
        payload["freeze_tag"] = tag
    path = tmp_path / name
    path.write_text(json.dumps(payload), encoding="utf-8")
    return path


def _sealed(tmp_path: Path, ids, repo=None, commit=None, tag=FREEZE_TAG):
    if repo is None:
        repo, commit = _init_repo(tmp_path)
    if commit is None:
        commit = _git(repo, "rev-parse", "HEAD")
    path = _write_manifest(tmp_path, ids, commit, "sealed.json", tag=tag)
    return heldout_guard.sealed_policy(str(path), FREEZE_TAG, repo=repo), repo


# --------------------------------------------------------------------------
# 1. Develop (default) is unchanged
# --------------------------------------------------------------------------


def test_develop_refuses_heldout_task_and_object():
    policy = heldout_guard.develop_policy()
    task = "x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT"

    with pytest.raises(heldout_guard.HeldOutRefused):
        policy.check_task(task)

    obj = _object("add", "p25519") or (
        S2N / "x86" / "curve25519" / "bignum_add_p25519.o")
    with pytest.raises(heldout_guard.HeldOutRefused):
        policy.check_object(str(obj), root=S2N)

    # The emitter's untouched default entry point still refuses it.
    with pytest.raises(SystemExit):
        emit_ref_proof_x86.build("add", task)


def test_develop_default_is_the_cli_default():
    args = type("Args", (), {"held_out": []})()
    policy = heldout_guard.policy_from_args(args)
    assert policy.mode == heldout_guard.MODE_DEVELOP
    with pytest.raises(heldout_guard.HeldOutRefused):
        policy.check_task("x86.bignum_neg_sm2.BIGNUM_NEG_SM2_CORRECT")


# --------------------------------------------------------------------------
# 2. Sealed evaluation allows exactly the manifest's nine
# --------------------------------------------------------------------------


def test_sealed_allows_declared_nine_ids(tmp_path):
    policy, _repo = _sealed(tmp_path, NINE)
    for task in NINE:
        assert policy.check_task(task) == task


def test_sealed_allows_declared_nine_objects(tmp_path):
    objects = []
    for task in NINE:
        op, family = _task_parts(task)
        obj = _object(op, family)
        if obj is None:
            pytest.skip("x86 s2n-bignum objects not present")
        objects.append(obj)

    policy, _repo = _sealed(tmp_path, NINE)
    for obj in objects:
        allowed = policy.check_object(str(obj), root=S2N)
        assert Path(allowed).name == obj.name


# --------------------------------------------------------------------------
# 3. Sealed evaluation still refuses unlisted held-out ids and objects
# --------------------------------------------------------------------------


def test_sealed_refuses_unlisted_heldout_id_and_object(tmp_path):
    # Omit the last of the nine from the manifest; it stays held out.
    eight = NINE[:-1]
    policy, _repo = _sealed(tmp_path, eight)

    with pytest.raises(heldout_guard.HeldOutRefused):
        policy.check_task(NINE[-1])

    # An id that is held out but was never in the declared set is refused too.
    with pytest.raises(heldout_guard.HeldOutRefused):
        policy.check_task("x86.bignum_cmul_p25519.BIGNUM_CMUL_P25519_CORRECT")

    op, family = _task_parts(NINE[-1])
    obj = _object(op, family)
    if obj is not None:
        with pytest.raises(heldout_guard.HeldOutRefused):
            policy.check_object(str(obj), root=S2N)

    cmul = _object("cmul", "p25519")
    if cmul is not None:
        with pytest.raises(heldout_guard.HeldOutRefused):
            policy.check_object(str(cmul), root=S2N)


# --------------------------------------------------------------------------
# 4. Sealed evaluation never opens a sealed task's proof .ml
# --------------------------------------------------------------------------


def test_sealed_refuses_sealed_proof_file(tmp_path):
    policy, _repo = _sealed(tmp_path, NINE)

    proof = _proof("add", "p25519")
    with pytest.raises(heldout_guard.HeldOutRefused):
        policy.check_proof(str(proof), root=S2N)

    # The guard's generic entry point must not treat a .ml as an object.
    with pytest.raises(heldout_guard.HeldOutRefused):
        policy.check_path(str(proof), root=S2N)

    # The --mode dev proof-read path is refused for a listed task as well.
    with pytest.raises(SystemExit):
        emit_ref_proof_x86.modulus_definition("add", "p25519", policy)
    with pytest.raises(SystemExit):
        emit_ref_proof_x86.machine_code_phrase(
            "add", "p25519", "p25519_mc", "x86/curve25519/bignum_add_p25519.o",
            policy)

    # The ARM reference emitter has the same proof-read path.
    with pytest.raises(SystemExit):
        emit_ref_proof.modulus_definition("add", "p25519", policy)


# --------------------------------------------------------------------------
# 5. Freeze verification
# --------------------------------------------------------------------------


def test_sealed_refuses_freeze_mismatch(tmp_path):
    repo, commit = _init_repo(tmp_path)
    manifest = _write_manifest(tmp_path, NINE, commit, "sealed.json")

    with pytest.raises(heldout_guard.HeldOutRefused):
        heldout_guard.sealed_policy(str(manifest), "no-such-tag", repo=repo)

    wrong_commit = _write_manifest(tmp_path, NINE, "0" * 40, "wrong-commit.json")
    with pytest.raises(heldout_guard.HeldOutRefused):
        heldout_guard.sealed_policy(str(wrong_commit), FREEZE_TAG, repo=repo)

    wrong_tag_field = _write_manifest(
        tmp_path, NINE, commit, "wrong-tag-field.json", tag="another-tag")
    with pytest.raises(heldout_guard.HeldOutRefused):
        heldout_guard.sealed_policy(str(wrong_tag_field), FREEZE_TAG, repo=repo)

    # A tracked file under tools/ is modified: the seal is not valid.
    (repo / "tools" / "tracked.py").write_text("# dirty\n", encoding="utf-8")
    with pytest.raises(heldout_guard.HeldOutRefused):
        heldout_guard.sealed_policy(str(manifest), FREEZE_TAG, repo=repo)


# --------------------------------------------------------------------------
# 6. Manifest validation
# --------------------------------------------------------------------------


@pytest.mark.parametrize("name,payload", [
    ("duplicate", {"sealed_ids": [NINE[0], NINE[0]]}),
    ("bare-prefix", {"sealed_ids": ["x86.bignum_add_p25519"]}),
    ("wildcard", {"sealed_ids": ["x86.bignum_add_p25519.*"]}),
    ("theorem-mismatch",
     {"sealed_ids": ["x86.bignum_add_p25519.BIGNUM_SUB_P25519_CORRECT"]}),
    ("not-a-list", {"sealed_ids": NINE[0]}),
    ("bad-commit", {"sealed_ids": [NINE[0]], "freeze_commit": "abc"}),
    ("unknown-key", {"sealed_ids": [NINE[0]], "allow_all": True}),
])
def test_manifest_rejects_duplicate_and_malformed(tmp_path, name, payload):
    body = {"freeze_commit": "0" * 40}
    body.update(payload)
    path = tmp_path / f"{name}.json"
    path.write_text(json.dumps(body), encoding="utf-8")
    with pytest.raises(heldout_guard.HeldOutRefused):
        heldout_guard.load_sealed_manifest(str(path))


def test_manifest_rejects_unparseable_json(tmp_path):
    path = tmp_path / "broken.json"
    path.write_text("{not json", encoding="utf-8")
    with pytest.raises(heldout_guard.HeldOutRefused):
        heldout_guard.load_sealed_manifest(str(path))


def test_manifest_rejects_duplicate_json_keys(tmp_path):
    path = tmp_path / "duplicate-key.json"
    path.write_text(
        '{"freeze_commit": "' + "0" * 40 + '", '
        '"sealed_ids": ["' + NINE[0] + '"], '
        '"sealed_ids": ["' + NINE[1] + '"]}',
        encoding="utf-8")
    with pytest.raises(heldout_guard.HeldOutRefused, match="duplicate key"):
        heldout_guard.load_sealed_manifest(str(path))


def test_manifest_duplicate_message_names_the_id(tmp_path):
    path = _write_manifest(tmp_path, [NINE[0], NINE[0]], "0" * 40, "dup.json")
    with pytest.raises(heldout_guard.HeldOutRefused, match="duplicate"):
        heldout_guard.load_sealed_manifest(str(path))
