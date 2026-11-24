#!/usr/bin/env python3
"""Python refusal-site mutation harness for the held-out guard.

``tools/mutate.py`` mutates the *Rust* refusal sites.  The guard that decides
which tasks may be opened is Python, and it is on the acceptance path: a guard
refusal that no test notices is a vacuous check.  This harness disables each
``raise HeldOutRefused(...)`` in turn (replacing the statement with ``pass``)
and records whether the mode tests notice.

The mutation is AST-based, so multi-line calls and ``... from exc`` are handled
without regex fragility.  The file is restored after every mutant and once more
in ``finally``; the tool also refuses to start if a stale ``.orig`` backup from
an interrupted run exists, and tells you how to recover it.

Usage:
    python3 tools/mutate_guard.py [--tests PATH ...] [--keep]
"""

from __future__ import annotations

import argparse
import ast
import hashlib
import os
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TARGET = ROOT / "tools" / "heldout_guard.py"
DEFAULT_TESTS = [
    "tools/test_heldout_modes.py",
    "tools/test_guard_refusals.py",
    "tools/test_challenge.py",
]

# Equivalent mutants: removing the site still refuses, because a downstream
# check raises the same refusal. Keyed by (line, normalised source) so that any
# edit which shifts them forces a deliberate re-review rather than silently
# inheriting the exemption. See research/receipts/ix-o9-guard-mutation.md §4.
EQUIVALENT = {
    (271, 'raise HeldOutRefused("empty path")'),
    (520, 'raise HeldOutRefused( f"working tree has changes to tracked files under tools/: {first}")'),
    (607, 'raise HeldOutRefused("empty path")'),
    (632, 'raise HeldOutRefused("empty path")'),
    (647, 'raise HeldOutRefused("empty path")'),
    (640, 'raise HeldOutRefused( f"held-out marker {marker!r} in proof path {path}")'),
}


def find_sites(src: str) -> list[tuple[int, int, int, int, str]]:
    tree = ast.parse(src)
    sites = []
    for node in ast.walk(tree):
        if not isinstance(node, ast.Raise):
            continue
        exc = node.exc
        if not (isinstance(exc, ast.Call) and isinstance(exc.func, ast.Name)
                and exc.func.id == "HeldOutRefused"):
            continue
        sites.append((node.lineno, node.col_offset,
                      node.end_lineno, node.end_col_offset, "raise"))
    return sorted(sites)


def offsets(src: str) -> list[int]:
    out = [0]
    for line in src.splitlines(keepends=True):
        out.append(out[-1] + len(line))
    return out


def mutate(src: str, site) -> str:
    line0, col0, line1, col1, _ = site
    off = offsets(src)
    start = off[line0 - 1] + col0
    end = off[line1 - 1] + col1
    return src[:start] + "pass  # MUTATED" + src[end:]


def run_tests(tests: list[str]) -> tuple[int, str]:
    env = dict(os.environ)
    proc = subprocess.run(
        [sys.executable, "-m", "pytest", "-x", "-q", *tests],
        cwd=ROOT, capture_output=True, text=True, env=env, timeout=300,
    )
    return proc.returncode, proc.stdout + proc.stderr


def error_snippet(src: str, site) -> str:
    line0, _, line1, _, _ = site
    lines = src.splitlines()
    return " ".join(lines[line0 - 1:line1])[:120]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--tests", nargs="*", default=DEFAULT_TESTS)
    ap.add_argument("--keep", action="store_true")
    args = ap.parse_args()

    original = TARGET.read_bytes()
    digest = hashlib.sha256(original).hexdigest()
    src = original.decode("utf-8")
    backup = TARGET.with_suffix(".py.orig")

    if backup.exists():
        print(f"REFUSING: stale backup {backup} exists from an interrupted run.\n"
              f"  Recover with: mv {backup} {TARGET}", file=sys.stderr)
        return 2

    sites = find_sites(src)
    if not sites:
        print("no HeldOutRefused sites found", file=sys.stderr)
        return 2

    tmp = Path(tempfile.mkdtemp(prefix="tekmerion-guard-mut-"))
    backup.write_bytes(original)
    results = []
    try:
        for n, site in enumerate(sites, 1):
            mutant = mutate(src, site)
            assert mutant != src
            TARGET.write_text(mutant, encoding="utf-8")
            code, out = run_tests(args.tests)
            kind = "KILLED" if code != 0 else "SURVIVED"
            if "SyntaxError" in out or "IndentationError" in out:
                kind = "INVALID_MUTANT"
            results.append((site, kind, out))
            print(f"{kind:14s} line {site[0]:4d}  {error_snippet(src, site)}")
            TARGET.write_bytes(original)
    finally:
        TARGET.write_bytes(original)
        if hashlib.sha256(TARGET.read_bytes()).hexdigest() != digest:
            print("FATAL: restore failed; original is in %s" % backup,
                  file=sys.stderr)
            return 3
        backup.unlink(missing_ok=True)
        if not args.keep:
            for p in tmp.iterdir():
                p.unlink()
            tmp.rmdir()

    killed = sum(1 for _, k, _ in results if k == "KILLED")
    survived = [r for r in results if r[1] == "SURVIVED"]
    print(f"\nguard refusal sites: {len(sites)}")
    print(f"killed: {killed}  survived: {len(survived)}")
    print(f"mutation score: {killed}/{len(sites)} = {killed / len(sites):.3f}")

    unexplained = []
    for site, _, out in survived:
        key = (site[0], " ".join(error_snippet(src, site).split()))
        if key in EQUIVALENT:
            print(f"EQUIVALENT line {site[0]}: {error_snippet(src, site)}")
        else:
            unexplained.append((site, out))

    if unexplained:
        print("\nUNEXPLAINED SURVIVORS (add a test, or record an equivalence):")
        for site, out in unexplained:
            print(f"  line {site[0]}: {error_snippet(src, site)}")
            tail = [l for l in out.splitlines() if l.strip()][-1:]
            if tail:
                print(f"    last test output: {tail[0][:120]}")
        return 1
    print("all survivors are recorded equivalent mutants")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
