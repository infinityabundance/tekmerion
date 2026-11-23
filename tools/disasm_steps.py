#!/usr/bin/env python3
"""disasm_steps.py -- turn an AArch64 ELF object into a machine-code step list.

The step list is the sole input to :func:`tools.derive_steps.derive_accumulate`.
Nothing here reads a proof file or a tactic script: the bytes come from the
object file that the proof's ``define_assert_from_elf`` binds, and the text
comes from ``objdump``.

Usage:
  python3 tools/disasm_steps.py OBJECT.o [--objdump PATH] [--json OUT.json]

Prints one JSON object:

  {"object": "...", "steps": [{"index": 1, "mnemonic": "ldp",
                               "operands": ["x4","x5","[x1]"],
                               "raw": "ldp x4, x5, [x1]"}, ...]}
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys

from derive_steps import Step

_DEFAULT_OBJDUMP_NAMES = (
    "aarch64-linux-gnu-objdump",
    "llvm-objdump",
    "objdump",
)

_LINE_RE = re.compile(r"^\s*([0-9a-fA-F]+):\s+(.*?)\s*$")
_INSN_RE = re.compile(r"^([a-z][a-z0-9._]*)(?:\s+(.*))?$")


def _find_objdump(explicit: str | None) -> list[str]:
    candidates = [explicit] if explicit else list(_DEFAULT_OBJDUMP_NAMES)
    for name in candidates:
        if not name:
            continue
        path = shutil.which(name)
        if path:
            # llvm-objdump needs the same flags as GNU objdump here.
            return [path]
    raise SystemExit(
        "disasm_steps: no AArch64 objdump found; pass --objdump PATH "
        f"(tried: {', '.join(c for c in candidates if c)})"
    )


def split_operands(text: str) -> list[str]:
    """Split an operand list on commas that are not inside ``[...]``."""
    out: list[str] = []
    depth = 0
    current: list[str] = []
    for ch in text:
        if ch == "[":
            depth += 1
        elif ch == "]":
            depth = max(0, depth - 1)
        if ch == "," and depth == 0:
            out.append("".join(current).strip())
            current = []
        else:
            current.append(ch)
    tail = "".join(current).strip()
    if tail:
        out.append(tail)
    return [o.lower() for o in out]


def parse_disassembly(text: str) -> list[Step]:
    """Parse ``objdump -d`` output into :class:`Step` records, in program order."""
    steps: list[Step] = []
    for line in text.splitlines():
        m = _LINE_RE.match(line)
        if not m:
            continue
        body = m.group(2)
        # Drop objdump's trailing comment, including the "// #123" numeric hint.
        body = body.split("//", 1)[0].strip()
        if not body:
            continue
        im = _INSN_RE.match(body)
        if not im:
            continue
        mnemonic = im.group(1).lower()
        operand_text = (im.group(2) or "").strip()
        operands = tuple(split_operands(operand_text)) if operand_text else ()
        steps.append(Step(index=len(steps) + 1, mnemonic=mnemonic,
                          operands=operands, raw=body))
    return steps


def disassemble(path: str, objdump: str | None = None) -> list[Step]:
    if not os.path.isfile(path):
        raise SystemExit(f"disasm_steps: not a file: {path}")
    cmd = _find_objdump(objdump) + ["-d", "--no-show-raw-insn", path]
    proc = subprocess.run(cmd, capture_output=True, text=True)
    if proc.returncode != 0:
        raise SystemExit(
            f"disasm_steps: {' '.join(cmd)} failed rc={proc.returncode}\n"
            f"{proc.stderr.strip()}"
        )
    return parse_disassembly(proc.stdout)


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("object", help="AArch64 ELF object file")
    ap.add_argument("--objdump", default=None,
                    help="objdump executable (default: aarch64-linux-gnu-objdump)")
    ap.add_argument("--json", default=None, help="write the step list here")
    args = ap.parse_args(argv)

    steps = disassemble(args.object, args.objdump)
    payload = {
        "object": args.object,
        "steps": [
            {"index": s.index, "mnemonic": s.mnemonic,
             "operands": list(s.operands), "raw": s.raw}
            for s in steps
        ],
    }
    text = json.dumps(payload, indent=2, sort_keys=True) + "\n"
    if args.json:
        with open(args.json, "w", encoding="utf-8") as fh:
            fh.write(text)
    else:
        sys.stdout.write(text)
    return 0


if __name__ == "__main__":
    sys.exit(main())
