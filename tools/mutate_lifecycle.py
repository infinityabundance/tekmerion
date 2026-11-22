#!/usr/bin/env python3
"""Lifecycle mutation harness.

`tools/mutate.py` proves that every *wire-format* refusal is load-bearing. It establishes
nothing about the lifecycle semantics. This harness does the same job for the revocation
lifecycle: each code returned by ``tekmerion::fault`` is a guarded site whose correct
behaviour is needed by at least one test.

The feature is off by default. This harness enables ``fault-injection`` and sets
``TEKMERION_FAULT`` to one code per run. A run whose suite stays green means the control
is not load-bearing: ``SURVIVED``, and the script exits non-zero.

Usage:
    python3 tools/mutate_lifecycle.py
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FILES = ["src/lib.rs", "src/wire.rs"]
CODE_RE = re.compile(r'fault\(\s*"(L\d\d)"\s*\)')

DESCRIPTIONS = {
    "L01": "status ignores pins",
    "L02": "classify drops the pins comparison",
    "L03": "status stops at the direct edge instead of the closure",
    "L04": "status ignores attestation edges",
    "L05": "update forgets the superseded node",
}


def discover_codes() -> list[str]:
    codes: set[str] = set()
    for rel in FILES:
        text = (ROOT / rel).read_text(encoding="utf-8")
        codes.update(CODE_RE.findall(text))
    return sorted(codes)


def run_suite(fault: str | None) -> tuple[int, str]:
    env = dict(os.environ)
    env.pop("TEKMERION_FAULT", None)
    if fault is not None:
        env["TEKMERION_FAULT"] = fault
    proc = subprocess.run(
        [
            "cargo",
            "test",
            "--offline",
            "--quiet",
            "--features",
            "fault-injection",
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
        env=env,
        timeout=900,
    )
    return proc.returncode, proc.stdout + proc.stderr


def classify(code: int, output: str) -> str:
    if code == 0:
        return "SURVIVED"
    if "error[" in output or "could not compile" in output:
        return "INVALID_MUTANT"
    return "KILLED"


def parser_score() -> str:
    path = ROOT / "mutation-report.json"
    if not path.exists():
        return "unknown"
    report = json.loads(path.read_text(encoding="utf-8"))
    score = report.get("mutation_score")
    if score is None:
        return "unknown"
    return f"{report['killed']}/{report['valid_mutants']} = {score:.3f}"


def main() -> int:
    codes = discover_codes()
    if not codes:
        print("no lifecycle fault codes found", file=sys.stderr)
        return 2

    # A mutant is meaningful only if the unmutated suite is green with the feature on.
    base_code, base_output = run_suite(None)
    if base_code != 0:
        print("baseline suite failed with fault-injection enabled", file=sys.stderr)
        print(base_output[-4000:], file=sys.stderr)
        return 2

    results: list[tuple[str, str]] = []
    for code in codes:
        status, output = run_suite(code)
        outcome = classify(status, output)
        results.append((code, outcome))
        print(f"{code} ({DESCRIPTIONS.get(code, 'lifecycle guard')}): {outcome}", flush=True)

    valid = [r for r in results if r[1] != "INVALID_MUTANT"]
    killed = [r for r in valid if r[1] == "KILLED"]
    survived = [r for r in valid if r[1] == "SURVIVED"]

    print()
    print(f"lifecycle sites: {len(results)}")
    print(f"valid mutants: {len(valid)}  killed: {len(killed)}  survived: {len(survived)}")
    if valid:
        print(f"lifecycle mutation score: {len(killed)}/{len(valid)} = {len(killed)/len(valid):.3f}")
    print(f"parser mutation score: {parser_score()}")

    report = {
        "sites": dict(results),
        "valid_mutants": len(valid),
        "killed": len(killed),
        "survived": len(survived),
        "invalid_mutants": len(results) - len(valid),
        "lifecycle_mutation_score": (len(killed) / len(valid)) if valid else None,
        "parser_mutation_score": parser_score(),
    }
    (ROOT / "lifecycle-report.json").write_text(
        json.dumps(report, indent=2) + "\n", encoding="utf-8"
    )

    return 0 if not survived else 1


if __name__ == "__main__":
    raise SystemExit(main())
