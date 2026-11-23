#!/usr/bin/env python3
"""heldout_guard.py -- refuse to open held-out proof files.

Development tooling must not read evaluation targets.  This guard rejects any
path whose curve or family is held out, and it fails closed: a missing file, a
directory, an unreadable file, a path outside the allowed root, or any
unexpected error is a refusal, not an allow.

Held-out markers are matched case-insensitively against the requested path and
its fully resolved path (so a symlink with an innocent name does not slip
through).  Defaults cover the programme's held-out curves:

  25519       curve25519, edwards25519, p25519, n25519, m25519, x25519
  secp256k1   the secp256k1 curves files
  sm2         the SM2 curve

Markers can be added (never removed) with --held-out.

Usage:
  python3 tools/heldout_guard.py PATH [PATH ...]
  python3 tools/heldout_guard.py PATH --root .bench/s2n-bignum
  python3 tools/heldout_guard.py PATH --held-out mycurve

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
import os
import sys

DEFAULT_HELD_OUT = ("25519", "secp256k1", "p256k1", "sm2")

EXIT_ALLOWED = 0
EXIT_REFUSED = 2


class HeldOutRefused(Exception):
    """Raised instead of opening a held-out or otherwise unsafe path."""


def _haystacks(path):
    """Every string a marker may appear in: the literal and resolved path."""
    yield path
    yield os.path.realpath(path)
    yield os.path.abspath(path)


def check_path(path, held_out=DEFAULT_HELD_OUT, root=None):
    """Validate a path without opening it.  Raise HeldOutRefused on refusal."""
    if not held_out:
        raise HeldOutRefused("empty held-out list; refusing by default")

    if not path:
        raise HeldOutRefused("empty path")

    for hay in _haystacks(path):
        low = hay.lower()
        for marker in held_out:
            if marker.lower() in low:
                raise HeldOutRefused(
                    f"held-out marker {marker!r} in {hay}"
                )

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


def open_guarded(path, held_out=DEFAULT_HELD_OUT, root=None):
    """Validate, then open.  The check always runs before the open."""
    check_path(path, held_out=held_out, root=root)
    return open(path, "rb")


def describe(path, held_out=DEFAULT_HELD_OUT, root=None):
    """Validate, open read-only, and return a digest line."""
    with open_guarded(path, held_out=held_out, root=root) as fh:
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


def main(argv=None):
    ap = argparse.ArgumentParser(
        description="Fail-closed guard against reading held-out s2n-bignum proofs.",
    )
    ap.add_argument("paths", nargs="*", help="files that development tooling wants to read")
    ap.add_argument("--root", default=None,
                    help="only allow paths inside this directory")
    ap.add_argument("--held-out", action="append", default=[],
                    help="add a held-out marker (defaults are always kept)")
    ap.add_argument("--list", action="store_true",
                    help="print the effective held-out markers and exit")
    args = ap.parse_args(argv)

    markers = DEFAULT_HELD_OUT + tuple(args.held_out)

    if args.list:
        for marker in markers:
            print(marker)
        return EXIT_ALLOWED

    if not args.paths:
        print("REFUSED: no path given (fail closed)", file=sys.stderr)
        return EXIT_REFUSED

    status = EXIT_ALLOWED
    for path in args.paths:
        try:
            print(describe(path, held_out=markers, root=args.root))
        except HeldOutRefused as exc:
            status = EXIT_REFUSED
            print(f"REFUSED: {exc}", file=sys.stderr)
        except Exception as exc:  # noqa: BLE001 - fail closed on anything unexpected
            status = EXIT_REFUSED
            print(f"REFUSED (unexpected {type(exc).__name__}): {exc}", file=sys.stderr)
    return status


if __name__ == "__main__":
    sys.exit(main())
