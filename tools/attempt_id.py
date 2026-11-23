#!/usr/bin/env python3
"""Compute the content-addressed ``attempt_id`` for one traced proof attempt.

The id is the SHA-256 of the canonical tuple, in this order:

    theorem_id, proof_text, injection, wrapper_sha256, module_sha256

Each component is encoded as an 8-byte big-endian length followed by its bytes,
so no redistribution of bytes between components can collide. ``proof_text`` is
the exact bytes of the proof file (not a digest), which is why the id changes
when the proof changes even if its name does not.

The id is the file stem of the attempt stream (``<attempt_id>.jsonl``) and the
manifest (``<attempt_id>.manifest.json``). It is printed as 64 lowercase hex
characters.

Usage:
    python3 tools/attempt_id.py --theorem-id ID --proof PATH \
        [--inject TEXT] --wrapper-sha SHA --module-sha SHA
    python3 tools/attempt_id.py --self-test
"""

from __future__ import annotations

import argparse
import hashlib
import sys
from pathlib import Path


def compute_attempt_id(
    theorem_id: str,
    proof_text: bytes,
    injection: str,
    wrapper_sha256: str,
    module_sha256: str,
) -> str:
    digest = hashlib.sha256()
    for component in (
        theorem_id.encode("utf-8"),
        proof_text,
        injection.encode("utf-8"),
        wrapper_sha256.encode("utf-8"),
        module_sha256.encode("utf-8"),
    ):
        digest.update(len(component).to_bytes(8, "big"))
        digest.update(component)
    return digest.hexdigest()


def self_test() -> int:
    ok = True
    base = compute_attempt_id("arm/proofs/word_max.ml", b"proof", "", "aa", "bb")
    again = compute_attempt_id("arm/proofs/word_max.ml", b"proof", "", "aa", "bb")
    if base == again and len(base) == 64 and all(c in "0123456789abcdef" for c in base):
        print("  ok: id is deterministic, 64 lowercase hex characters")
    else:
        print("  FAIL: id is not deterministic 64-character lowercase hex")
        ok = False

    mutations = {
        "theorem_id": compute_attempt_id("arm/proofs/word_min.ml", b"proof", "", "aa", "bb"),
        "proof_text": compute_attempt_id("arm/proofs/word_max.ml", b"prooF", "", "aa", "bb"),
        "injection": compute_attempt_id(
            "arm/proofs/word_max.ml", b"proof", 'TRY (FAIL_TAC "probe")', "aa", "bb"
        ),
        "wrapper": compute_attempt_id("arm/proofs/word_max.ml", b"proof", "", "ab", "bb"),
        "module": compute_attempt_id("arm/proofs/word_max.ml", b"proof", "", "aa", "bc"),
    }
    for name, value in mutations.items():
        if value != base:
            print(f"  ok: changing {name} changes the id")
        else:
            print(f"  FAIL: changing {name} did not change the id")
            ok = False

    # Length framing: moving a byte between adjacent components must not collide.
    left = compute_attempt_id("ab", b"c", "", "", "")
    right = compute_attempt_id("a", b"bc", "", "", "")
    if left != right:
        print("  ok: length framing prevents component-boundary collisions")
    else:
        print("  FAIL: component-boundary collision")
        ok = False

    print("SELF-TEST PASSED" if ok else "SELF-TEST FAILED")
    return 0 if ok else 1


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Content-addressed attempt id for a traced proof attempt."
    )
    parser.add_argument("--theorem-id", help="stable proof identifier")
    parser.add_argument("--proof", help="proof file; its exact bytes are digested")
    parser.add_argument("--inject", default="", help="injected tactic text, or empty")
    parser.add_argument("--wrapper-sha", help="sha256 of the kernel wrapper used")
    parser.add_argument("--module-sha", help="sha256 of exptrace_failure.ml used")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args(argv)

    if args.self_test:
        return self_test()

    missing = [
        option
        for option, value in (
            ("--theorem-id", args.theorem_id),
            ("--proof", args.proof),
            ("--wrapper-sha", args.wrapper_sha),
            ("--module-sha", args.module_sha),
        )
        if not value
    ]
    if missing:
        parser.error("missing required option(s): " + ", ".join(missing))

    try:
        proof_text = Path(args.proof).read_bytes()
    except OSError as exc:
        print(f"attempt_id: error: {exc}", file=sys.stderr)
        return 1

    print(
        compute_attempt_id(
            args.theorem_id,
            proof_text,
            args.inject,
            args.wrapper_sha,
            args.module_sha,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
