#!/usr/bin/env python3
"""disasm_steps.py -- turn an AArch64 or x86-64 ELF object into a step list.

The step list is the sole input to :func:`tools.derive_steps.derive_accumulate`.
Nothing here reads a proof file or a tactic script: the bytes come from the
object file that the proof's ``define_assert_from_elf`` binds, and the text
comes from ``objdump``.

AArch64 and x86 differ in two ways that matter to the step numbering:

  * s2n-bignum's x86 proofs bind a *trimmed* machine code constant
    (``define_trimmed`` = ``TRIM_LIST(4,0)``), which drops the four-byte
    ``ENDBR64`` prefix.  The x86 path therefore requires that prefix, drops it
    and reports addresses relative to the trimmed constant.  A code object
    without it is refused rather than silently numbered against the wrong base.
  * an x86 ``ensures`` precondition may enter the code after a prologue, e.g.
    ``read RIP s = word (pc + 3)``.  :func:`enter_at` keeps only the
    instructions at or after that entry and renumbers them from 1, which is the
    numbering ``X86_STEPS_TAC``/``X86_ACCSTEPS_TAC`` uses.

The x86 disassembly is Intel syntax (``objdump -M intel``) so that operands
read destination-first, matching ``derive_steps``' operand indices.

Usage:
  python3 tools/disasm_steps.py OBJECT.o [--arch arm|x86] [--objdump PATH]
                                         [--json OUT.json]

Prints one JSON object:

  {"object": "...", "arch": "x86", "trimmed": 4,
   "steps": [{"index": 1, "address": 3, "mnemonic": "push",
              "operands": ["rbx"], "raw": "push   rbx"}, ...]}
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

_DEFAULT_OBJDUMP_NAMES = {
    "arm": ("aarch64-linux-gnu-objdump", "llvm-objdump", "objdump"),
    "x86": ("objdump", "llvm-objdump", "x86_64-linux-gnu-objdump"),
}

# ELF e_machine values we can name.
_EM_386 = 0x03
_EM_X86_64 = 0x3E
_EM_AARCH64 = 0xB7
_ARCH_BY_MACHINE = {_EM_X86_64: "x86", _EM_AARCH64: "arm", _EM_386: "x86"}

# ``define_trimmed`` in s2n-bignum's x86 proofs is ``TRIM_LIST(4,0)``.
X86_TRIM_BYTES = 4
_X86_ENDBR64 = b"\xf3\x0f\x1e\xfa"

_LINE_RE = re.compile(r"^\s*([0-9a-fA-F]+):\s+(.*?)\s*$")
_INSN_RE = re.compile(r"^([a-z][a-z0-9._]*)(?:\s+(.*))?$")


def elf_arch(path: str) -> str | None:
    """Return ``"x86"``/``"arm"`` from the ELF header, or ``None``."""
    try:
        with open(path, "rb") as fh:
            head = fh.read(20)
    except OSError:
        return None
    if len(head) < 20 or head[:4] != b"\x7fELF":
        return None
    if head[5] != 1:  # EI_DATA: only little-endian objects are used here
        return None
    machine = int.from_bytes(head[18:20], "little")
    return _ARCH_BY_MACHINE.get(machine)


def _find_objdump(explicit: str | None, arch: str) -> list[str]:
    if arch not in _DEFAULT_OBJDUMP_NAMES:
        raise SystemExit(f"disasm_steps: unknown architecture {arch!r}")
    candidates = [explicit] if explicit else list(_DEFAULT_OBJDUMP_NAMES[arch])
    for name in candidates:
        if not name:
            continue
        path = shutil.which(name)
        if path:
            # llvm-objdump needs the same flags as GNU objdump here.
            return [path]
    raise SystemExit(
        f"disasm_steps: no {arch} objdump found; pass --objdump PATH "
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
        address = int(m.group(1), 16)
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
                          operands=operands, raw=body, address=address))
    return steps


def _renumber(steps: list[Step]) -> list[Step]:
    return [Step(index=i + 1, mnemonic=s.mnemonic, operands=s.operands,
                 raw=s.raw, address=s.address)
            for i, s in enumerate(steps)]


def trim_x86_prefix(steps: list[Step], path: str = "") -> list[Step]:
    """Drop the four-byte ``ENDBR64`` that ``define_trimmed`` removes.

    The returned addresses are relative to the trimmed machine code constant
    (so the first real instruction is at address 0).  Fail closed when the
    object does not start with exactly that prefix: trimming a different
    number of bytes would shift every step number without any check.
    """
    if not steps:
        raise SystemExit(f"disasm_steps: no instructions in {path or 'object'}")
    first = steps[0]
    next_address = steps[1].address if len(steps) > 1 else None
    if (first.mnemonic != "endbr64" or first.address != 0
            or next_address != X86_TRIM_BYTES):
        raise SystemExit(
            "disasm_steps: x86 object must start with a 4-byte endbr64 "
            f"(found {first.mnemonic!r} at {first.address}, next at "
            f"{next_address}); {path or 'object'} is not a trimmed x86 "
            "s2n-bignum code constant"
        )
    shifted = [Step(index=s.index, mnemonic=s.mnemonic, operands=s.operands,
                    raw=s.raw, address=s.address - X86_TRIM_BYTES)
               for s in steps[1:]]
    return _renumber(shifted)


def enter_at(steps: list[Step], entry: int) -> list[Step]:
    """Keep instructions at or after *entry* (a trimmed-code byte offset).

    ``X86_STEPS_TAC`` starts numbering at the instruction the precondition's
    ``read RIP`` names, so a prologue before that entry is not simulated and
    must not occupy a step number.  The entry must land on an instruction
    boundary; otherwise the numbering cannot be produced and we refuse.
    """
    if entry < 0:
        raise SystemExit(f"disasm_steps: negative entry offset {entry}")
    kept = [s for s in steps if s.address >= entry]
    if not kept:
        raise SystemExit(f"disasm_steps: no instruction at or after entry {entry}")
    if kept[0].address != entry:
        raise SystemExit(
            f"disasm_steps: entry {entry} is not an instruction boundary "
            f"(first kept instruction is at {kept[0].address})"
        )
    return _renumber(kept)


def disassemble(path: str, objdump: str | None = None,
                arch: str | None = None) -> list[Step]:
    if not os.path.isfile(path):
        raise SystemExit(f"disasm_steps: not a file: {path}")
    detected = elf_arch(path)
    if arch is None:
        arch = detected
    if arch is None:
        raise SystemExit(
            f"disasm_steps: cannot detect ELF architecture of {path}; "
            "pass --arch arm|x86"
        )
    if detected is not None and detected != arch:
        raise SystemExit(
            f"disasm_steps: {path} is a {detected} object but --arch {arch} "
            "was requested"
        )
    cmd = _find_objdump(objdump, arch) + ["-d", "--no-show-raw-insn"]
    if arch == "x86":
        cmd += ["-M", "intel"]
    cmd.append(path)
    proc = subprocess.run(cmd, capture_output=True, text=True)
    if proc.returncode != 0:
        raise SystemExit(
            f"disasm_steps: {' '.join(cmd)} failed rc={proc.returncode}\n"
            f"{proc.stderr.strip()}"
        )
    steps = parse_disassembly(proc.stdout)
    if arch == "x86":
        steps = trim_x86_prefix(steps, path)
    return steps


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("object", help="ELF object file (AArch64 or x86-64)")
    ap.add_argument("--arch", choices=["arm", "x86"], default=None,
                    help="architecture (default: read from the ELF header)")
    ap.add_argument("--entry", type=lambda v: int(v, 0), default=0,
                    help="trimmed-code entry offset for x86 (default 0)")
    ap.add_argument("--objdump", default=None,
                    help="objdump executable (default: an arch-appropriate one)")
    ap.add_argument("--json", default=None, help="write the step list here")
    args = ap.parse_args(argv)

    steps = disassemble(args.object, args.objdump, args.arch)
    arch = args.arch or elf_arch(args.object)
    if arch == "x86" and args.entry:
        steps = enter_at(steps, args.entry)
    payload = {
        "object": args.object,
        "arch": arch,
        "steps": [
            {"index": s.index, "address": s.address, "mnemonic": s.mnemonic,
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
