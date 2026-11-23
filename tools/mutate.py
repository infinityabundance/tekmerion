#!/usr/bin/env python3
"""Refusal-site mutation harness.

Invariant 5 of the programme: every instrument proves its sensitivity. A refusal check
that no test notices is a vacuous check. This harness disables each refusal site in turn
(replacing ``refuse!`` with the no-op ``skip_refusal!``) and records whether the test
suite notices.

Standard mutation-testing practice: a mutant that fails to compile is not a valid mutant
and is *excluded* from the score, rather than silently counted as killed.

Usage:
    python3 tools/mutate.py [--keep]
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SITE_RE = re.compile(r"refuse!\(\s*(R\d\d)")
FILES = ["src/lib.rs", "src/wire.rs"]


def discover_sites() -> list[str]:
    sites: set[str] = set()
    for rel in FILES:
        text = (ROOT / rel).read_text(encoding="utf-8")
        sites.update(SITE_RE.findall(text))
    return sorted(sites)


def run_tests(crate: Path, target: Path) -> tuple[int, str]:
    env = dict(os.environ)
    env["CARGO_TARGET_DIR"] = str(target)
    proc = subprocess.run(
        ["cargo", "test", "--offline", "--quiet"],
        cwd=crate,
        capture_output=True,
        text=True,
        env=env,
        timeout=600,
    )
    return proc.returncode, proc.stdout + proc.stderr


def classify(code: int, output: str) -> str:
    if code == 0:
        return "SURVIVED"
    if "error[" in output or "could not compile" in output:
        return "INVALID_MUTANT"
    return "KILLED"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--keep", action="store_true", help="keep the scratch directory")
    args = ap.parse_args()

    sites = discover_sites()
    if not sites:
        print("no refusal sites found", file=sys.stderr)
        return 2

    scratch = Path(tempfile.mkdtemp(prefix="tekmerion-mutate-"))
    target = scratch / "target"

    results: list[tuple[str, str]] = []
    try:
        for site in sites:
            work = scratch / site
            shutil.copytree(
                ROOT,
                work,
                # Only the crate and the tooling it tests are needed. `.bench` holds
                # benchmark artefacts and `research` holds receipts, logs and a
                # machine-specific solver symlink; neither is needed and both have
                # broken the runner before. `symlinks=True` keeps any stray link a
                # link instead of failing on a dangling target.
                ignore=shutil.ignore_patterns(
                    "target", ".git", ".bench", "research", "*.md", "*.log"
                ),
                symlinks=True,
            )
            for rel in FILES:
                path = work / rel
                text = path.read_text(encoding="utf-8")
                mutated = re.sub(
                    r"refuse!\(\s*" + site + r"\s*,",
                    f"skip_refusal!({site},",
                    text,
                )
                if mutated != text:
                    path.write_text(mutated, encoding="utf-8")

            check = "".join((work / rel).read_text(encoding="utf-8") for rel in FILES)
            if re.search(r"refuse!\(\s*" + site + r"\s*,", check):
                raise SystemExit(f"site {site} was not fully disabled")

            code, output = run_tests(work, target)
            outcome = classify(code, output)
            results.append((site, outcome))
            print(f"{site}: {outcome}", flush=True)
    finally:
        if not args.keep:
            shutil.rmtree(scratch, ignore_errors=True)
        else:
            print(f"scratch kept at {scratch}")

    valid = [r for r in results if r[1] != "INVALID_MUTANT"]
    killed = [r for r in valid if r[1] == "KILLED"]
    survived = [r for r in valid if r[1] == "SURVIVED"]

    print()
    print(f"refusal sites: {len(results)}")
    print(f"valid mutants: {len(valid)}  killed: {len(killed)}  survived: {len(survived)}")
    if valid:
        print(f"mutation score: {len(killed)}/{len(valid)} = {len(killed)/len(valid):.3f}")

    report = {
        "sites": dict(results),
        "valid_mutants": len(valid),
        "killed": len(killed),
        "survived": len(survived),
        "invalid_mutants": len(results) - len(valid),
        "mutation_score": (len(killed) / len(valid)) if valid else None,
    }
    (ROOT / "mutation-report.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")

    return 0 if not survived else 1


if __name__ == "__main__":
    raise SystemExit(main())
