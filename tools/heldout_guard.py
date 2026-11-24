#!/usr/bin/env python3
"""heldout_guard.py -- fail-closed modes for held-out evaluation targets.

Three controls are deliberately kept separate:

``develop`` (the default, and the only behaviour the older entry points have)
    Development tooling must not read evaluation targets.  Any path or task id
    whose curve or family is held out is refused, and the guard fails closed: a
    missing file, a directory, an unreadable file, a path outside the allowed
    root, or any unexpected error is a refusal, not an allow.

``sealed-eval``
    The sealed run must be *able* to generate answers for the declared held-out
    task ids and their ELF objects, and still must never read their proof text.
    This mode therefore requires an explicit manifest (``--sealed-manifest``)
    and a freeze tag (``--freeze-tag``).  It allows exactly the task ids listed
    in the manifest and the objects those ids name.  An unlisted held-out id or
    object is refused, every proof ``.ml`` of a sealed task is refused (also in
    ``--mode dev`` proof reads), and the mode refuses to run at all unless

      * ``git rev-parse <TAG>^{commit}`` equals the manifest's freeze commit,
      * the manifest's optional ``freeze_tag`` (when present) equals ``<TAG>``,
      * the working tree has no changes to tracked files under ``tools/``.

``spent``
    Development data that has already been disclosed is *spent*: answering it
    again is no longer a blind evaluation, but the guard still must not be
    disabled wholesale.  ``--spent --spent-manifest PATH`` allows exactly the
    task ids recorded in that manifest, together with the objects those ids
    name, and it still refuses to open any proof ``.ml`` for them.  The
    manifest must state, per id, why the id is spent, and carry a ``decision``
    that cites a receipt under ``research/receipts/``.  Unlike ``sealed-eval``
    no freeze tag is involved: a spent id is not a secrecy boundary, it is a
    scope boundary.

Held-out markers are matched case-insensitively against the requested path and
its fully resolved path (so a symlink with an innocent name does not slip
through).  Defaults cover the programme's held-out curves:

  25519       curve25519, edwards25519, p25519, n25519, m25519, x25519
  secp256k1   the secp256k1 curves files
  sm2         the SM2 curve

Markers can be added (never removed) with ``--held-out``.

Sealed manifest format (JSON; duplicate keys are refused, and the
``sealed_ids`` list must itself be duplicate-free)::

  {
    "freeze_commit": "<40 lowercase hex sha1>",
    "freeze_tag": "freeze-x86-sealed-v1",
    "sealed_ids": [
      "x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT"
    ],
    "note": "optional free text"
  }

Every entry of ``sealed_ids`` must be a declared sealed-set id of the form
``<arm|x86>.bignum_<add|sub|neg>_<family>.<BIGNUM_<OP>_<FAMILY>_CORRECT>``
with the theorem name consistent with the operation and family.  Wildcards,
partial ids and duplicates are refused.  Unknown top-level keys are refused so
that a manifest cannot quietly carry a second meaning.

Spent manifest format (JSON; duplicate keys are refused)::

  {
    "decision": "receipt path and rationale",
    "spent": {
      "arm.bignum_add_p384.BIGNUM_ADD_P384_CORRECT":
        "R4 development task; baseline verdict FAIL (r4-verdicts.csv)"
    },
    "note": "optional free text"
  }

``decision`` must be non-empty and cite a path under ``research/receipts/``;
``spent`` must map at least one declared id to a non-empty reason.  The reason
is required for every id, so a spent allowance cannot be granted silently.

Usage:
  python3 tools/heldout_guard.py PATH [PATH ...]                    # develop
  python3 tools/heldout_guard.py PATH --root .bench/s2n-bignum
  python3 tools/heldout_guard.py PATH --held-out mycurve
  python3 tools/heldout_guard.py PATH --sealed-eval \
      --sealed-manifest research/receipts/sealed.json \
      --freeze-tag freeze-x86-sealed-v1 --root .bench/s2n-bignum
  python3 tools/heldout_guard.py PATH --spent \
      --spent-manifest research/receipts/spent-r4-arm.json \
      --root .bench/s2n-bignum

Exit status: 0 allowed (and the file was opened read-only for hashing),
             2 refused (fail closed).

  $ python3 tools/heldout_guard.py .bench/s2n-bignum/arm/proofs/p256_montjadd.ml
  allowed: .../arm/proofs/p256_montjadd.ml sha256=... lines=...

  $ python3 tools/heldout_guard.py .bench/s2n-bignum/arm/proofs/secp256k1_jadd.ml
  REFUSED: held-out marker 'secp256k1' in .../arm/proofs/secp256k1_jadd.ml
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

DEFAULT_HELD_OUT = ("25519", "secp256k1", "p256k1", "sm2")

EXIT_ALLOWED = 0
EXIT_REFUSED = 2

MODE_DEVELOP = "develop"
MODE_SEALED_EVAL = "sealed-eval"
MODE_SPENT = "spent"

REPO_ROOT = Path(__file__).resolve().parent.parent

# A manifest entry must be a complete declared-sealed-set id.  The theorem name
# must agree with the operation and family; that rules out a wildcard, a bare
# task prefix, and an id whose suffix names a different operation.
_SEALED_ID_RE = re.compile(
    r"^(?P<arch>arm|x86)\.bignum_(?P<op>add|sub|neg)_(?P<family>[a-z0-9]{1,32})\."
    r"(?P<theorem>BIGNUM_(?P<top>ADD|SUB|NEG)_(?P<tfamily>[A-Z0-9]{1,32})_CORRECT)$"
)

_OBJECT_RE = re.compile(r"^bignum_(?P<op>add|sub|neg)_(?P<family>[a-z0-9]+)\.o$")
_PROOF_RE = re.compile(r"^bignum_(?P<op>add|sub|neg)_(?P<family>[a-z0-9]+)\.ml$")
_HEX40_RE = re.compile(r"^[0-9a-f]{40}$")

_MANIFEST_KEYS = frozenset({"freeze_commit", "freeze_tag", "sealed_ids", "note"})
_SPENT_MANIFEST_KEYS = frozenset({"decision", "spent", "note"})
_DECISION_CITATION = "research/receipts/"
_MANIFEST_NOUN = {
    MODE_SEALED_EVAL: "sealed",
    MODE_SPENT: "spent",
}


class HeldOutRefused(Exception):
    """Raised instead of opening a held-out or otherwise unsafe path."""


# --------------------------------------------------------------------------
# Path and task-id primitives
# --------------------------------------------------------------------------


def _haystacks(path):
    """Every string a marker may appear in: the literal and resolved path."""
    yield path
    yield os.path.realpath(path)
    yield os.path.abspath(path)


def _marker_in(text, held_out=DEFAULT_HELD_OUT):
    """Return the first held-out marker in *text*, or None."""
    if not isinstance(text, str):
        return None
    low = text.lower()
    for marker in held_out:
        if marker.lower() in low:
            return marker
    return None


def canonical_task_id(arch: str, op: str, family: str) -> str:
    return (f"{arch}.bignum_{op}_{family}."
            f"BIGNUM_{op.upper()}_{family.upper()}_CORRECT")


def parse_task_id(task: str):
    """Match a declared sealed-set id, or return None.

    A match implies the theorem name is consistent with the operation and
    family; this is the structural half of the manifest validation.
    """
    if not isinstance(task, str):
        return None
    m = _SEALED_ID_RE.fullmatch(task)
    if not m:
        return None
    if m.group("top") != m.group("op").upper():
        return None
    if m.group("tfamily") != m.group("family").upper():
        return None
    return m.groupdict()


def task_id_for_object(path: str):
    """The sealed id an ELF object path would answer, or None.

    Only ``<arch>/<dir>/bignum_<op>_<family>.o`` under an ``arm``/``x86`` tree
    is recognised.  A path under a ``proofs`` tree is never an object.
    """
    real = Path(os.path.realpath(path))
    if "proofs" in real.parts:
        return None
    m = _OBJECT_RE.fullmatch(real.name)
    if not m:
        return None
    # Only <arch>/<dir>/<object>.o under an arm or x86 tree is recognised; a
    # bare <arch>/<object>.o is not an object the emitter would ever select.
    if real.parent.parent.name not in ("arm", "x86"):
        return None
    arch = real.parent.parent.name
    return canonical_task_id(arch, m.group("op"), m.group("family"))


def task_id_for_proof(path: str):
    """The sealed id a proof-file path would belong to, or None."""
    real = Path(os.path.realpath(path))
    m = _PROOF_RE.fullmatch(real.name)
    if not m:
        return None
    if real.parent.name == "proofs" and real.parent.parent.name in ("arm", "x86"):
        arch = real.parent.parent.name
    elif real.parent.parent.name in ("arm", "x86"):
        arch = real.parent.parent.name
    else:
        return None
    return canonical_task_id(arch, m.group("op"), m.group("family"))


def _looks_like_proof(path: str) -> bool:
    real = Path(os.path.realpath(path))
    return real.suffix == ".ml" or "proofs" in real.parts


def _check_regular(path, root=None):
    """Stat/root/non-empty checks shared by develop and sealed allowances.

    Marker checking is the caller's job; this helper only refuses paths that
    are not readable regular non-empty files inside *root*.
    """
    if not path:
        raise HeldOutRefused("empty path")
    try:
        st = os.stat(path)
    except OSError as exc:
        raise HeldOutRefused(f"cannot stat {path}: {exc}") from exc

    if not os.path.isfile(path):
        raise HeldOutRefused(f"not a regular file: {path}")

    if root is not None:
        real = os.path.realpath(path)
        real_root = os.path.realpath(root)
        if os.path.commonpath([real, real_root]) != real_root:
            raise HeldOutRefused(f"outside allowed root {root!r}: {real}")

    if st.st_size == 0:
        raise HeldOutRefused(f"empty file: {path}")

    return os.path.realpath(path)


def check_path(path, held_out=DEFAULT_HELD_OUT, root=None):
    """Validate a path without opening it.  Raise HeldOutRefused on refusal.

    This is the unreduced develop-mode behaviour: every marker is refused.
    """
    if not held_out:
        raise HeldOutRefused("empty held-out list; refusing by default")

    if not path:
        raise HeldOutRefused("empty path")

    for hay in _haystacks(path):
        marker = _marker_in(hay, held_out)
        if marker is not None:
            raise HeldOutRefused(f"held-out marker {marker!r} in {hay}")

    return _check_regular(path, root=root)


def open_guarded(path, held_out=DEFAULT_HELD_OUT, root=None):
    """Validate, then open.  The check always runs before the open."""
    check_path(path, held_out=held_out, root=root)
    return open(path, "rb")


def describe(path, held_out=DEFAULT_HELD_OUT, root=None, policy=None):
    """Validate, open read-only, and return a digest line."""
    if policy is None:
        check_path(path, held_out=held_out, root=root)
    else:
        policy.check_path(path, root=root)
    with open(path, "rb") as fh:
        digest = hashlib.sha256()
        lines = 0
        last = b""
        while True:
            chunk = fh.read(1 << 16)
            if not chunk:
                break
            digest.update(chunk)
            lines += chunk.count(b"\n")
            last = chunk
        if last and not last.endswith(b"\n"):
            lines += 1
    return f"allowed: {os.path.realpath(path)} sha256={digest.hexdigest()} lines={lines}"


# --------------------------------------------------------------------------
# Sealed manifest and freeze verification
# --------------------------------------------------------------------------


@dataclass(frozen=True)
class SealedManifest:
    path: str
    freeze_commit: str
    sealed_ids: frozenset
    freeze_tag: object = None  # str | None
    note: str = ""


def _reject_duplicate_keys(pairs):
    """json object_pairs_hook: a duplicate top-level key is a refusal."""
    seen = {}
    for key, value in pairs:
        if key in seen:
            raise HeldOutRefused(
                f"duplicate key in sealed manifest: {key!r}")
        seen[key] = value
    return seen


def load_sealed_manifest(path) -> SealedManifest:
    """Load and strictly validate a sealed manifest.  Fail closed on anything."""
    if not path:
        raise HeldOutRefused("no sealed manifest given")

    if os.path.islink(path):
        raise HeldOutRefused(f"sealed manifest may not be a symlink: {path}")
    try:
        st = os.stat(path)
    except OSError as exc:
        raise HeldOutRefused(f"cannot stat sealed manifest {path}: {exc}") from exc
    if not os.path.isfile(path):
        raise HeldOutRefused(f"sealed manifest is not a regular file: {path}")
    if st.st_size == 0:
        raise HeldOutRefused(f"sealed manifest is empty: {path}")

    try:
        with open(path, "r", encoding="utf-8") as fh:
            payload = json.load(fh, object_pairs_hook=_reject_duplicate_keys)
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise HeldOutRefused(f"cannot parse sealed manifest {path}: {exc}") from exc

    if not isinstance(payload, dict):
        raise HeldOutRefused("sealed manifest must be a JSON object")

    unknown = set(payload) - _MANIFEST_KEYS
    if unknown:
        raise HeldOutRefused(
            f"sealed manifest has unknown keys: {sorted(unknown)}")

    commit = payload.get("freeze_commit")
    if not isinstance(commit, str) or not _HEX40_RE.fullmatch(commit):
        raise HeldOutRefused(
            "freeze_commit must be a 40-character lowercase hex sha1")

    raw_ids = payload.get("sealed_ids")
    if not isinstance(raw_ids, list) or not raw_ids:
        raise HeldOutRefused("sealed_ids must be a non-empty JSON list")

    seen = set()
    for item in raw_ids:
        if not isinstance(item, str):
            raise HeldOutRefused(f"sealed id is not a string: {item!r}")
        if parse_task_id(item) is None:
            raise HeldOutRefused(f"sealed id is not in the declared format: {item!r}")
        if item in seen:
            raise HeldOutRefused(f"duplicate sealed id: {item!r}")
        seen.add(item)

    tag = payload.get("freeze_tag")
    if tag is not None:
        if (not isinstance(tag, str) or not tag
                or any(ch.isspace() for ch in tag)):
            raise HeldOutRefused(f"freeze_tag is not a valid tag name: {tag!r}")

    note = payload.get("note", "")
    if not isinstance(note, str):
        raise HeldOutRefused("note must be a string")

    return SealedManifest(
        path=os.path.realpath(path),
        freeze_commit=commit,
        sealed_ids=frozenset(seen),
        freeze_tag=tag,
        note=note,
    )


@dataclass(frozen=True)
class SpentManifest:
    """An allow-list of disclosed development ids, each with its reason."""

    path: str
    spent: dict  # canonical task id -> non-empty reason
    decision: str
    note: str = ""


def load_spent_manifest(path) -> SpentManifest:
    """Load and strictly validate a spent manifest.  Fail closed on anything.

    A spent allowance is a scope decision, not a secrecy one, so no freeze tag
    is involved; the evidence that the scope decision was made is the
    ``decision`` string, which must cite a receipt, and a per-id reason.
    """
    if not path:
        raise HeldOutRefused("no spent manifest given")

    if os.path.islink(path):
        raise HeldOutRefused(f"spent manifest may not be a symlink: {path}")
    try:
        st = os.stat(path)
    except OSError as exc:
        raise HeldOutRefused(f"cannot stat spent manifest {path}: {exc}") from exc
    if not os.path.isfile(path):
        raise HeldOutRefused(f"spent manifest is not a regular file: {path}")
    if st.st_size == 0:
        raise HeldOutRefused(f"spent manifest is empty: {path}")

    try:
        with open(path, "r", encoding="utf-8") as fh:
            payload = json.load(fh, object_pairs_hook=_reject_duplicate_keys)
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise HeldOutRefused(f"cannot parse spent manifest {path}: {exc}") from exc

    if not isinstance(payload, dict):
        raise HeldOutRefused("spent manifest must be a JSON object")

    unknown = set(payload) - _SPENT_MANIFEST_KEYS
    if unknown:
        raise HeldOutRefused(
            f"spent manifest has unknown keys: {sorted(unknown)}")

    decision = payload.get("decision")
    if not isinstance(decision, str) or not decision.strip():
        raise HeldOutRefused("spent manifest needs a non-empty decision")
    if _DECISION_CITATION not in decision:
        raise HeldOutRefused(
            "spent manifest decision must cite a path under "
            f"{_DECISION_CITATION}")

    raw = payload.get("spent")
    if not isinstance(raw, dict) or not raw:
        raise HeldOutRefused("spent must be a non-empty JSON object")

    spent = {}
    for task, reason in raw.items():
        if parse_task_id(task) is None:
            raise HeldOutRefused(
                f"spent id is not in the declared format: {task!r}")
        if not isinstance(reason, str) or not reason.strip():
            raise HeldOutRefused(f"spent id {task!r} has no reason")
        spent[task] = reason

    note = payload.get("note", "")
    if not isinstance(note, str):
        raise HeldOutRefused("note must be a string")

    return SpentManifest(
        path=os.path.realpath(path),
        spent=spent,
        decision=decision,
        note=note,
    )


def _git(repo, *args):
    """Run git in *repo* with a bounded timeout.  Refuse on any error."""
    cmd = ["git", "--no-optional-locks", *args]
    try:
        proc = subprocess.run(
            cmd, cwd=str(repo), stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            text=True, timeout=30, check=False)
    except (OSError, subprocess.SubprocessError) as exc:
        raise HeldOutRefused(f"cannot run git in {repo}: {exc}") from exc
    if proc.returncode != 0:
        detail = (proc.stderr or proc.stdout or "").strip().splitlines()
        raise HeldOutRefused(
            f"git {' '.join(args)} failed in {repo}: "
            f"{detail[0] if detail else 'exit ' + str(proc.returncode)}")
    return proc.stdout


def verify_freeze(manifest: SealedManifest, freeze_tag: str,
                  repo=REPO_ROOT) -> None:
    """Refuse unless the tag, the manifest commit and the tools tree agree."""
    if (not isinstance(freeze_tag, str) or not freeze_tag
            or any(ch.isspace() for ch in freeze_tag)):
        raise HeldOutRefused(f"--freeze-tag is not a valid tag name: {freeze_tag!r}")

    if manifest.freeze_tag is not None and manifest.freeze_tag != freeze_tag:
        raise HeldOutRefused(
            f"manifest freeze_tag {manifest.freeze_tag!r} does not match "
            f"--freeze-tag {freeze_tag!r}")

    got = _git(repo, "rev-parse", "--verify",
               f"{freeze_tag}^{{commit}}").strip().lower()
    if got != manifest.freeze_commit:
        raise HeldOutRefused(
            f"tag {freeze_tag!r} resolves to {got!r}, but the manifest records "
            f"{manifest.freeze_commit!r}")

    dirty = _git(repo, "status", "--porcelain", "--untracked-files=no",
                 "--", "tools").strip()
    if dirty:
        first = dirty.splitlines()[0]
        raise HeldOutRefused(
            f"working tree has changes to tracked files under tools/: {first}")

    # Cleanliness is not enough: the tree must *be* the frozen code.  Without
    # this, a later commit could change the emitters and still be accepted
    # under an older freeze tag.
    changed = _git(repo, "diff", "--name-only", manifest.freeze_commit,
                   "--", "tools").strip()
    if changed:
        first = changed.splitlines()[0]
        raise HeldOutRefused(
            f"tools/ differs from the freezer commit {manifest.freeze_commit[:12]}: "
            f"{first}")


# --------------------------------------------------------------------------
# Policy object shared by the guard command and the emitters
# --------------------------------------------------------------------------


class GuardPolicy:
    """Mode + manifest + markers.  Immutable for the life of one process."""

    def __init__(self, mode, manifest=None, repo=REPO_ROOT,
                 held_out=DEFAULT_HELD_OUT):
        if mode not in (MODE_DEVELOP, MODE_SEALED_EVAL, MODE_SPENT):
            raise HeldOutRefused(f"unknown guard mode: {mode!r}")
        if not held_out:
            raise HeldOutRefused("empty held-out list; refusing by default")
        if mode == MODE_SEALED_EVAL and not isinstance(manifest, SealedManifest):
            raise HeldOutRefused("sealed-eval requires a sealed manifest")
        if mode == MODE_SPENT and not isinstance(manifest, SpentManifest):
            raise HeldOutRefused("spent requires a spent manifest")
        self.mode = mode
        self.manifest = manifest
        self.repo = Path(repo)
        self.held_out = tuple(held_out)

    @property
    def sealed(self) -> bool:
        return self.mode == MODE_SEALED_EVAL

    @property
    def spent(self) -> bool:
        return self.mode == MODE_SPENT

    @property
    def admitted(self) -> bool:
        """Does the mode admit any held-out marker at all?"""
        return self.mode in (MODE_SEALED_EVAL, MODE_SPENT)

    def _manifest_noun(self) -> str:
        return _MANIFEST_NOUN.get(self.mode, "sealed")

    def _marker(self, text):
        return _marker_in(text, self.held_out)

    def _listed(self, task) -> bool:
        if not self.admitted or self.manifest is None or task is None:
            return False
        if self.sealed:
            return task in self.manifest.sealed_ids
        return task in self.manifest.spent

    def check_task(self, task):
        """Allow a task id only if it is not held out or is manifest-listed."""
        if not isinstance(task, str) or not task:
            raise HeldOutRefused(f"task id is not a non-empty string: {task!r}")
        marker = self._marker(task)
        if marker is None:
            return task
        if not self.admitted:
            raise HeldOutRefused(
                f"held-out marker {marker!r} in task {task!r}")
        if parse_task_id(task) is None:
            raise HeldOutRefused(
                f"held-out task id is not in the declared task-id format: "
                f"{task!r}")
        if not self._listed(task):
            raise HeldOutRefused(
                f"held-out task id {task!r} is not in the "
                f"{self._manifest_noun()} manifest")
        return task

    def check_object(self, path, root=None):
        """Allow a held-out ELF object only when its task id is manifest-listed."""
        if not path:
            raise HeldOutRefused("empty path")
        if _looks_like_proof(path):
            raise HeldOutRefused(f"not an object path: {path}")
        marker = self._marker(path)
        if marker is None:
            return check_path(path, held_out=self.held_out, root=root)
        if not self.admitted:
            raise HeldOutRefused(f"held-out marker {marker!r} in {path}")
        task = task_id_for_object(path)
        if task is None:
            raise HeldOutRefused(
                f"held-out path is not a declared bignum object: {path}")
        if not self._listed(task):
            raise HeldOutRefused(
                f"object {path} belongs to {task!r}, which is not in the "
                f"{self._manifest_noun()} manifest")
        if root is None:
            raise HeldOutRefused(
                f"{self._manifest_noun()} object allowance for {path} requires "
                f"an allowed root")
        return _check_regular(path, root=root)

    def check_proof(self, path, root=None):
        """A listed task's proof text is never readable, in any mode."""
        if not path:
            raise HeldOutRefused("empty path")
        task = task_id_for_proof(path)
        if self._listed(task):
            raise HeldOutRefused(
                f"{self._manifest_noun()} proof file is never readable: "
                f"{path} ({task!r})")
        marker = self._marker(path)
        if marker is not None:
            raise HeldOutRefused(
                f"held-out marker {marker!r} in proof path {path}")
        return check_path(path, held_out=self.held_out, root=root)

    def check_path(self, path, root=None):
        """Generic path entry point: proofs are never objects."""
        if not path:
            raise HeldOutRefused("empty path")
        if self._marker(path) is not None:
            if _looks_like_proof(path):
                return self.check_proof(path, root=root)
            return self.check_object(path, root=root)
        if _looks_like_proof(path):
            return self.check_proof(path, root=root)
        return check_path(path, held_out=self.held_out, root=root)


def develop_policy(held_out=DEFAULT_HELD_OUT) -> GuardPolicy:
    return GuardPolicy(MODE_DEVELOP, held_out=held_out)


def sealed_policy(manifest_path, freeze_tag, repo=REPO_ROOT,
                  held_out=DEFAULT_HELD_OUT) -> GuardPolicy:
    """Load, verify and return a sealed policy; refuses on any mismatch."""
    manifest = load_sealed_manifest(manifest_path)
    verify_freeze(manifest, freeze_tag, repo=repo)
    return GuardPolicy(MODE_SEALED_EVAL, manifest=manifest, repo=repo,
                       held_out=held_out)


def spent_policy(manifest_path, repo=REPO_ROOT,
                 held_out=DEFAULT_HELD_OUT) -> GuardPolicy:
    """Load and return a spent policy.  No freeze tag is involved."""
    manifest = load_spent_manifest(manifest_path)
    return GuardPolicy(MODE_SPENT, manifest=manifest, repo=repo,
                       held_out=held_out)


# --------------------------------------------------------------------------
# CLI plumbing shared by the emitters
# --------------------------------------------------------------------------


def add_mode_arguments(ap: argparse.ArgumentParser) -> None:
    ap.add_argument("--develop", action="store_true",
                    help="development mode (default): refuse every held-out "
                         "task, object and proof file")
    ap.add_argument("--sealed-eval", action="store_true",
                    help="sealed evaluation: allow only manifest-listed task "
                         "ids and objects; never read their proof text")
    ap.add_argument("--sealed-manifest", default=None, metavar="PATH",
                    help="sealed manifest (required with --sealed-eval)")
    ap.add_argument("--freeze-tag", default=None, metavar="TAG",
                    help="git tag that must resolve to the manifest's freeze "
                         "commit (required with --sealed-eval)")
    ap.add_argument("--spent", action="store_true",
                    help="spent development data: allow only the ids listed in "
                         "--spent-manifest and their objects; never read "
                         "their proof text")
    ap.add_argument("--spent-manifest", default=None, metavar="PATH",
                    help="spent manifest (required with --spent)")


def policy_from_args(args, repo=REPO_ROOT) -> GuardPolicy:
    """Build the policy named by parsed CLI arguments.  Fail closed."""
    sealed = bool(getattr(args, "sealed_eval", False))
    spent = bool(getattr(args, "spent", False))
    develop = bool(getattr(args, "develop", False))
    sealed_manifest = getattr(args, "sealed_manifest", None)
    freeze_tag = getattr(args, "freeze_tag", None)
    spent_manifest = getattr(args, "spent_manifest", None)
    extra = tuple(getattr(args, "held_out", None) or ())
    held_out = DEFAULT_HELD_OUT + extra

    chosen = [name for name, flag in
              ((MODE_DEVELOP, develop), (MODE_SEALED_EVAL, sealed),
               (MODE_SPENT, spent)) if flag]
    if len(chosen) > 1:
        raise HeldOutRefused(
            "guard modes are mutually exclusive: " + ", ".join(chosen))

    if spent:
        if spent_manifest is None:
            raise HeldOutRefused("--spent requires --spent-manifest PATH")
        if sealed_manifest is not None:
            raise HeldOutRefused(
                "--sealed-manifest requires --sealed-eval, not --spent")
        if freeze_tag is not None:
            raise HeldOutRefused("--freeze-tag requires --sealed-eval")
        return spent_policy(spent_manifest, repo=repo, held_out=held_out)

    if not sealed:
        if sealed_manifest is not None or freeze_tag is not None:
            raise HeldOutRefused(
                "--sealed-manifest and --freeze-tag require --sealed-eval")
        if spent_manifest is not None:
            raise HeldOutRefused("--spent-manifest requires --spent")
        return GuardPolicy(MODE_DEVELOP, repo=repo, held_out=held_out)
    if spent_manifest is not None:
        raise HeldOutRefused("--spent-manifest requires --spent, not --sealed-eval")
    if sealed_manifest is None:
        raise HeldOutRefused("--sealed-eval requires --sealed-manifest PATH")
    if freeze_tag is None:
        raise HeldOutRefused("--sealed-eval requires --freeze-tag TAG")
    return sealed_policy(sealed_manifest, freeze_tag, repo=repo,
                         held_out=held_out)


def main(argv=None):
    ap = argparse.ArgumentParser(
        description="Fail-closed guard against reading held-out s2n-bignum "
                    "proofs (develop), gate their declared objects "
                    "(sealed-eval), or re-answer disclosed development ids "
                    "(spent).",
    )
    ap.add_argument("paths", nargs="*", help="files that development tooling wants to read")
    ap.add_argument("--root", default=None,
                    help="only allow paths inside this directory")
    ap.add_argument("--held-out", action="append", default=[],
                    help="add a held-out marker (defaults are always kept)")
    ap.add_argument("--list", action="store_true",
                    help="print the effective held-out markers and exit")
    add_mode_arguments(ap)
    args = ap.parse_args(argv)

    markers = DEFAULT_HELD_OUT + tuple(args.held_out)

    if args.list:
        for marker in markers:
            print(marker)
        return EXIT_ALLOWED

    try:
        policy = policy_from_args(args)
    except HeldOutRefused as exc:
        print(f"REFUSED: {exc}", file=sys.stderr)
        return EXIT_REFUSED

    if not args.paths:
        print("REFUSED: no path given (fail closed)", file=sys.stderr)
        return EXIT_REFUSED

    status = EXIT_ALLOWED
    for path in args.paths:
        try:
            print(describe(path, root=args.root, policy=policy))
        except HeldOutRefused as exc:
            status = EXIT_REFUSED
            print(f"REFUSED: {exc}", file=sys.stderr)
        except Exception as exc:  # noqa: BLE001 - fail closed on anything unexpected
            status = EXIT_REFUSED
            print(f"REFUSED (unexpected {type(exc).__name__}): {exc}", file=sys.stderr)
    return status


if __name__ == "__main__":
    sys.exit(main())
