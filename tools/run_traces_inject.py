#!/usr/bin/env python3
"""Insert one tactic expression at the start of the first proof in a HOL file.

Usage:
    python3 tools/run_traces_inject.py INPUT OUTPUT "TRY (FAIL_TAC \\"probe\\")"

The insertion point is the first non-empty line after the first proof goal
terminator (a line whose right-stripped text ends with a backtick followed by
a comma). The inserted line is `<indent><TACTIC> THEN`, so the caller supplies
a complete tactic expression without a trailing THEN.

The transform is deliberately narrow. If the file does not look like a
`prove (...)` script, the helper exits non-zero rather than guessing.
"""

from __future__ import annotations

import sys
from pathlib import Path


class InjectError(Exception):
    pass


def inject(text: str, tactic: str) -> str:
    if "\n" in tactic or "\r" in tactic:
        raise InjectError("the injected tactic must be a single line")
    if not tactic.strip():
        raise InjectError("the injected tactic is empty")

    lines = text.split("\n")

    prove_at = None
    for index, line in enumerate(lines):
        if "prove" in line:
            prove_at = index
            break
    if prove_at is None:
        raise InjectError("no `prove` found")

    goal_end = None
    for index in range(prove_at, len(lines)):
        if lines[index].rstrip().endswith("`,"):
            goal_end = index
            break
    if goal_end is None:
        raise InjectError(
            "no proof goal terminator (a line ending with a backtick and comma) "
            "found after the first `prove`"
        )

    insert_at = goal_end + 1
    while insert_at < len(lines) and not lines[insert_at].strip():
        insert_at += 1
    if insert_at >= len(lines):
        raise InjectError("the first proof has no tactic after its goal")

    target = lines[insert_at]
    indent = target[: len(target) - len(target.lstrip())]
    lines.insert(insert_at, f"{indent}{tactic} THEN")
    return "\n".join(lines)


def main(argv: list[str]) -> int:
    if len(argv) != 4:
        print(__doc__.strip(), file=sys.stderr)
        return 2
    source = Path(argv[1])
    destination = Path(argv[2])
    tactic = argv[3]
    try:
        text = source.read_text(encoding="utf-8")
        destination.write_text(inject(text, tactic), encoding="utf-8")
    except (OSError, InjectError) as exc:
        print(f"run_traces_inject: error: {exc}", file=sys.stderr)
        return 1
    print(f"injected `{tactic} THEN` into {destination}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
