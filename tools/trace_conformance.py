#!/usr/bin/env python3
"""Compare stock and failure-instrumented TacticTrace success records.

Each directory holds the per-tactic files TacticTrace writes, ``<tac>.json``.
Files whose records carry a ``conv`` key are conversion traces and are skipped;
conversion success records are a separate channel and are not compared here.
Records are aligned by (tactic, ordinal within file); JSON key order and
whitespace are normalised by parsing, but field values are compared exactly.

Fields asserted equal:
    tactic, definition_line_number, user_line_number, goal_before,
    goals_after, num_subgoals, arg_names, arg_types, arg_values, arg_exprs

Usage:
    python3 tools/trace_conformance.py STOCK_DIR PATCHED_DIR
    python3 tools/trace_conformance.py --self-test
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FIXTURES = ROOT / "trace" / "fixtures" / "conformance"

FIELDS = [
    "tactic",
    "definition_line_number",
    "user_line_number",
    "goal_before",
    "goals_after",
    "num_subgoals",
    "arg_names",
    "arg_types",
    "arg_values",
    "arg_exprs",
]

_MISSING = object()


class ConformanceError(Exception):
    """A precise, user-facing failure."""


def _load_file(path: Path) -> list[dict] | None:
    """Return tactic records, or None for a conversion trace.

    Conversion traces use a ``conv`` key and are a separate generator branch,
    so they are skipped rather than mixed into the tactic alignment. Conversion
    failure journals are a separate channel and are not compared here.
    """
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise ConformanceError(f"{path}: invalid JSON: {exc}") from exc
    if not isinstance(data, list):
        raise ConformanceError(f"{path}: expected a JSON array of records")
    if not data:
        return []
    if not any(isinstance(record, dict) and "tactic" in record for record in data):
        if all(isinstance(record, dict) and "conv" in record for record in data):
            return None
        raise ConformanceError(
            f"{path}: records have neither a 'tactic' nor a 'conv' field"
        )
    records: list[dict] = []
    for ordinal, record in enumerate(data):
        if not isinstance(record, dict):
            raise ConformanceError(f"{path}: record {ordinal} is not an object")
        if not isinstance(record.get("tactic"), str):
            raise ConformanceError(
                f"{path}: record {ordinal} has no string 'tactic' field"
            )
        records.append(record)
    return records


def collect(directory: Path) -> tuple[dict[tuple[str, int], tuple[str, dict]], int]:
    if not directory.is_dir():
        raise ConformanceError(f"not a directory: {directory}")
    aligned: dict[tuple[str, int], tuple[str, dict]] = {}
    skipped_conversions = 0
    for path in sorted(directory.glob("*.json")):
        records = _load_file(path)
        if records is None:
            skipped_conversions += 1
            continue
        for ordinal, record in enumerate(records):
            key = (record["tactic"], ordinal)
            if key in aligned:
                raise ConformanceError(
                    f"duplicate alignment key {key} in {path} and {aligned[key][0]}"
                )
            aligned[key] = (path.name, record)
    return aligned, skipped_conversions


def _short(value: object) -> str:
    if value is _MISSING:
        return "<missing>"
    text = repr(value)
    if len(text) > 160:
        return text[:157] + "..."
    return text


def deep_diff(
    stock: object, patched: object, path: str
) -> list[tuple[str, object, object]]:
    """Return (field path, stock value, patched value) differences."""
    diffs: list[tuple[str, object, object]] = []
    if isinstance(stock, dict) and isinstance(patched, dict):
        for key in sorted(set(stock) | set(patched)):
            sub = f"{path}.{key}" if path else key
            if key not in stock:
                diffs.append((sub, _MISSING, patched[key]))
            elif key not in patched:
                diffs.append((sub, stock[key], _MISSING))
            else:
                diffs.extend(deep_diff(stock[key], patched[key], sub))
    elif isinstance(stock, list) and isinstance(patched, list):
        if len(stock) != len(patched):
            diffs.append((f"{path}[length]", len(stock), len(patched)))
        for index, (left, right) in enumerate(zip(stock, patched)):
            diffs.extend(deep_diff(left, right, f"{path}[{index}]"))
    else:
        if type(stock) is not type(patched) or stock != patched:
            diffs.append((path, stock, patched))
    return diffs


def compare_dirs(
    stock_dir: Path, patched_dir: Path
) -> tuple[int, int, list[str]]:
    stock, stock_conversions = collect(stock_dir)
    patched, patched_conversions = collect(patched_dir)
    if not stock and not patched:
        raise ConformanceError(
            f"no <tac>.json records found in {stock_dir} or {patched_dir}"
        )

    diffs: list[str] = []
    checked = 0
    keys = sorted(set(stock) | set(patched), key=lambda k: (k[0], k[1]))

    for tactic, ordinal in keys:
        if (tactic, ordinal) not in patched:
            diffs.append(
                f"{tactic}[{ordinal}]: missing from patched "
                f"({stock[(tactic, ordinal)][0]})"
            )
            continue
        if (tactic, ordinal) not in stock:
            diffs.append(
                f"{tactic}[{ordinal}]: unexpected in patched "
                f"({patched[(tactic, ordinal)][0]})"
            )
            continue

        stock_name, stock_record = stock[(tactic, ordinal)]
        patched_name, patched_record = patched[(tactic, ordinal)]
        checked += 1

        for field in FIELDS:
            left = stock_record.get(field, _MISSING)
            right = patched_record.get(field, _MISSING)
            if left is _MISSING or right is _MISSING:
                diffs.append(
                    f"{tactic}[{ordinal}] field '{field}': "
                    f"stock={'missing' if left is _MISSING else 'present'}, "
                    f"patched={'missing' if right is _MISSING else 'present'}"
                )
                continue
            for field_path, left_value, right_value in deep_diff(
                left, right, field
            ):
                diffs.append(
                    f"{tactic}[{ordinal}] field '{field}' ({field_path}): "
                    f"stock={_short(left_value)} "
                    f"patched={_short(right_value)} "
                    f"[{stock_name} vs {patched_name}]"
                )

    return checked, stock_conversions + patched_conversions, diffs


def report(stock_dir: Path, patched_dir: Path) -> int:
    try:
        checked, skipped, diffs = compare_dirs(stock_dir, patched_dir)
    except ConformanceError as exc:
        print(f"trace_conformance: error: {exc}", file=sys.stderr)
        return 2

    for diff in diffs:
        print(f"DIFF {diff}")
    if skipped:
        print(
            f"skipped {skipped} conversion trace file(s); conversion success "
            "records are not compared"
        )
    print(
        f"checked {checked} aligned record pair(s); "
        f"{len(diffs)} difference(s)"
    )
    if diffs:
        print("CONFORMANCE FAILED")
        return 1
    print("CONFORMANCE PASSED")
    return 0


def self_test() -> int:
    stock = FIXTURES / "stock"
    patched = FIXTURES / "patched"
    diverged = FIXTURES / "diverged"

    if not (stock.is_dir() and patched.is_dir() and diverged.is_dir()):
        print(f"FAIL: missing fixture directories under {FIXTURES}", file=sys.stderr)
        return 1

    ok = True

    print("matching fixture pair (must pass):")
    code = report(stock, patched)
    if code == 0:
        print("  ok: matching pair passed")
    else:
        print(f"  FAIL: matching pair returned {code}")
        ok = False

    print("diverging fixture pair (must fail and name num_subgoals):")
    try:
        checked, _skipped, diffs = compare_dirs(stock, diverged)
    except ConformanceError as exc:
        print(f"  FAIL: unexpected load error: {exc}")
        return 1
    for diff in diffs:
        print(f"  DIFF {diff}")
    if not diffs:
        print("  FAIL: diverging pair produced no differences")
        ok = False
    elif not any("num_subgoals" in diff for diff in diffs):
        print("  FAIL: divergence report did not name 'num_subgoals'")
        ok = False
    else:
        print(
            f"  ok: detected {len(diffs)} difference(s) across "
            f"{checked} pair(s), including num_subgoals"
        )

    print("SELF-TEST PASSED" if ok else "SELF-TEST FAILED")
    return 0 if ok else 1


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Compare stock and patched TacticTrace success records."
    )
    parser.add_argument("stock", nargs="?", help="stock trace directory")
    parser.add_argument("patched", nargs="?", help="patched trace directory")
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="run the matching and deliberately diverging fixture pairs",
    )
    args = parser.parse_args(argv)

    if args.self_test:
        if args.stock or args.patched:
            parser.error("--self-test takes no positional arguments")
        return self_test()

    if not args.stock or not args.patched:
        parser.error("STOCK_DIR and PATCHED_DIR are required unless --self-test is used")

    return report(Path(args.stock), Path(args.patched))


if __name__ == "__main__":
    raise SystemExit(main())
