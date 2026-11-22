#!/usr/bin/env python3
"""Patch a TacticTrace-generated ``kernel_wrapper.ml`` for failure capture.

The stock generator emits, for every tactic wrapper:

    let (trace_ga_inst,trace_goals_after,trace_justf) = <application> (trace_asl,trace_g) in
    exptrace_add_tac "<tacname>" {

This tool rewrites only the application line:

    let (trace_ga_inst,trace_goals_after,trace_justf) =
      exptrace_guard "<tacname>" trace_goal_before (fun () -> <application> (trace_asl,trace_g)) in

The subsequent ``exptrace_add_tac`` emission is left byte-for-byte alone, so
success records remain bit-identical. Conversions are a separate generator
branch and are counted but not patched in this version.

Usage:
    python3 tools/patch_wrapper.py IN OUT
    python3 tools/patch_wrapper.py --self-test
"""

from __future__ import annotations

import argparse
import re
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FIXTURES = ROOT / "trace" / "fixtures"

# <application> is emitted on a single line by tracer.ml.
APPLICATION_RE = re.compile(
    r"^    let \(trace_ga_inst,trace_goals_after,trace_justf\) = (.*) in$"
)
PATCHED_APPLICATION_RE = re.compile(
    r"^    let \(trace_ga_inst,trace_goals_after,trace_justf\) =$"
)
GUARD_RE = re.compile(
    r'^      exptrace_guard "([^"]+)" trace_goal_before \(fun \(\) -> (.*)\) in$'
)
ADD_TAC_RE = re.compile(r'^    exptrace_add_tac "([^"]+)" \{$')
ADD_CONV_RE = re.compile(r'^  exptrace_add_conv "([^"]+)" \{$')


class PatchError(Exception):
    """A precise, user-facing failure."""


@dataclass
class Stats:
    patched: int = 0
    already: int = 0
    conversions: int = 0


def patch_text(text: str) -> tuple[str, Stats]:
    """Return patched text and counts, or raise PatchError."""
    lines = text.split("\n")
    out: list[str] = []
    stats = Stats()
    i = 0

    while i < len(lines):
        line = lines[i]

        if ADD_CONV_RE.match(line):
            stats.conversions += 1

        match = APPLICATION_RE.match(line)
        if match:
            if i + 1 >= len(lines):
                raise PatchError(
                    f"line {i + 1}: tactic application is not followed by an "
                    "exptrace_add_tac line"
                )
            add = ADD_TAC_RE.match(lines[i + 1])
            if not add:
                raise PatchError(
                    f"line {i + 1}: cannot find the tactic name on line "
                    f"{i + 2} (expected `    exptrace_add_tac \"<tacname>\" {{`)"
                )
            name = add.group(1)
            application = match.group(1)
            out.append(
                "    let (trace_ga_inst,trace_goals_after,trace_justf) ="
            )
            out.append(
                f'      exptrace_guard "{name}" trace_goal_before '
                f"(fun () -> {application}) in"
            )
            stats.patched += 1
            # Leave lines[i + 1] (the exptrace_add_tac line) for the next
            # iteration, untouched.
            i += 1
            continue

        if PATCHED_APPLICATION_RE.match(line):
            if i + 1 >= len(lines) or not GUARD_RE.match(lines[i + 1]):
                raise PatchError(
                    f"line {i + 1}: already-patched application is not followed "
                    "by a well-formed exptrace_guard line"
                )
            stats.already += 1

        out.append(line)
        i += 1

    if stats.patched + stats.already == 0:
        raise PatchError(
            "no tactic wrapper matched; this does not look like a generated "
            "kernel_wrapper.ml, or it contains only conversions"
        )

    return "\n".join(out), stats


def patch_file(source: Path, destination: Path) -> Stats:
    text = source.read_text(encoding="utf-8")
    patched, stats = patch_text(text)
    destination.write_text(patched, encoding="utf-8")
    return stats


def _expect_error(label: str, text: str, needle: str) -> bool:
    try:
        patch_text(text)
    except PatchError as exc:
        if needle in str(exc):
            print(f"  ok: {label}: {exc}")
            return True
        print(f"  FAIL: {label}: wrong error: {exc}")
        return False
    print(f"  FAIL: {label}: expected PatchError")
    return False


def self_test() -> int:
    before_path = FIXTURES / "kernel_wrapper_before.ml"
    after_path = FIXTURES / "kernel_wrapper_after.ml"

    if not before_path.is_file() or not after_path.is_file():
        print(f"FAIL: missing fixtures under {FIXTURES}", file=sys.stderr)
        return 1

    expected = after_path.read_text(encoding="utf-8")
    ok = True

    got, stats = patch_text(before_path.read_text(encoding="utf-8"))
    if got == expected:
        print(
            "  ok: golden fixture matches kernel_wrapper_after.ml "
            f"(patched {stats.patched}, conversions {stats.conversions})"
        )
    else:
        print("  FAIL: golden fixture output differs from kernel_wrapper_after.ml")
        ok = False
        for index, (want, have) in enumerate(
            zip(expected.split("\n"), got.split("\n")), start=1
        ):
            if want != have:
                print(f"    first difference at line {index}:")
                print(f"      expected: {want!r}")
                print(f"      actual:   {have!r}")
                break

    # Idempotence: feeding the patched output back in must not double-wrap.
    got2, stats2 = patch_text(got)
    if got2 == expected and stats2.patched == 0 and stats2.already == stats.patched:
        print(
            f"  ok: idempotent (patched 0, already patched {stats2.already})"
        )
    else:
        print(
            "  FAIL: second pass changed the output or double-wrapped "
            f"(patched {stats2.patched}, already {stats2.already})"
        )
        ok = False

    ok &= _expect_error(
        "conversions-only input is refused",
        '  exptrace_add_conv "BETA_CONV" {\n',
        "no tactic wrapper matched",
    )
    ok &= _expect_error(
        "missing exptrace_add_tac name is refused",
        "    let (trace_ga_inst,trace_goals_after,trace_justf) = "
        "FOO (trace_asl,trace_g) in\n",
        "cannot find the tactic name",
    )

    # Exercise the file path end-to-end as well.
    with tempfile.TemporaryDirectory(prefix="patch-wrapper-selftest-") as tmp:
        destination = Path(tmp) / "kernel_wrapper_after.ml"
        file_stats = patch_file(before_path, destination)
        if (
            destination.read_text(encoding="utf-8") == expected
            and file_stats.patched == stats.patched
        ):
            print("  ok: patch_file round-trip")
        else:
            print("  FAIL: patch_file round-trip")
            ok = False

    print("SELF-TEST PASSED" if ok else "SELF-TEST FAILED")
    return 0 if ok else 1


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Wrap TacticTrace tactic applications with exptrace_guard."
    )
    parser.add_argument("input", nargs="?", help="generated kernel_wrapper.ml")
    parser.add_argument("output", nargs="?", help="patched output path")
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="compare the golden fixture with the expected patched file",
    )
    args = parser.parse_args(argv)

    if args.self_test:
        if args.input or args.output:
            parser.error("--self-test takes no positional arguments")
        return self_test()

    if not args.input or not args.output:
        parser.error("IN and OUT are required unless --self-test is used")

    source = Path(args.input)
    destination = Path(args.output)
    if not source.is_file():
        print(f"patch_wrapper: error: input not found: {source}", file=sys.stderr)
        return 1

    try:
        stats = patch_file(source, destination)
    except PatchError as exc:
        print(f"patch_wrapper: error: {exc}", file=sys.stderr)
        return 1
    except OSError as exc:
        print(f"patch_wrapper: error: {exc}", file=sys.stderr)
        return 1

    print(f"tactics patched: {stats.patched}")
    if stats.already:
        print(f"tactics already patched, not wrapped again: {stats.already}")
    print(f"conversions left untouched: {stats.conversions}")
    print(f"wrote {destination}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
