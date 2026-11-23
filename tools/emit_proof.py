#!/usr/bin/env python3
"""emit_proof.py -- assemble a complete THEN-form answer from a witness.

The emitted text is

    <derived preamble> THEN
    <ARM_ACCSTEPS_TAC ... (derived list) ...> THEN
    <derived close>

The preamble (variable introduction, digitisation) is a deterministic function
of the query's variables and memory references.  The accumulate list comes from
tools/derive_steps.py.  The close comes from tools/w4_close.py and is driven by
the witness.

Usage:
  python3 tools/emit_proof.py \
      --task arm.bignum_sub_p384.BIGNUM_SUB_P384_CORRECT \
      --dir /tmp/ans --receipt /tmp/receipts/sub.json

The task id is the only curve-bearing input; the operation, family and object
path are parsed from it, and every arithmetic term comes from the query.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Dict, List, Optional

HERE = Path(__file__).resolve().parent
REPO = HERE.parent
BENCH = REPO / ".bench"
S2N = BENCH / "s2n-bignum"

if str(HERE) not in sys.path:
    sys.path.insert(0, str(HERE))

import disasm_steps  # noqa: E402
import w2_witnesses  # noqa: E402
import w4_close  # noqa: E402
from witness import Witness  # noqa: E402


OP_RE = re.compile(r"bignum_(add|sub|neg)_")
FAMILY_RE = re.compile(r"^arm\.bignum_(?:add|sub|neg)_([a-z0-9]+)\.")

HELD_OUT_MARKERS = ("25519", "secp256k1", "p256k1", "sm2")


def op_from_task(task: str) -> str:
    m = OP_RE.search(task)
    if not m:
        raise SystemExit(f"cannot determine op from task id {task!r}")
    return m.group(1)


def is_heldout(text: str) -> bool:
    low = text.lower()
    return any(marker in low for marker in HELD_OUT_MARKERS)


def assert_not_heldout(text: str, allow_heldout: bool = False) -> None:
    """Fail closed if a held-out curve name appears in *text*."""
    if allow_heldout:
        return
    if is_heldout(text):
        raise SystemExit(f"refusing held-out marker in {text!r} "
                         "(pass --allow-heldout for the frozen run)")


def object_for(op: str, task: str) -> Path:
    """Resolve the ELF object from the task id, not from a curve constant."""
    m = FAMILY_RE.match(task)
    if not m:
        raise SystemExit(f"not an ARM bignum task id: {task!r}")
    family = m.group(1)
    matches = sorted(S2N.glob(f"arm/*/bignum_{op}_{family}.o"))
    if len(matches) != 1:
        raise SystemExit(
            f"expected one object for {op}/{family}, found {matches}")
    return matches[0]


def guard_object(obj: Path) -> None:
    """Refuse anything that is not a machine-code object.

    The pipeline reads only the ELF object and the benchmark query.  It never
    opens a proof file, so the held-out guard is not applied to the object
    path (the guard's markers deliberately match the object names too).
    """
    if obj.suffix != ".o":
        raise SystemExit(f"refusing non-object input {obj}")
    if "proofs" in obj.parts:
        raise SystemExit(f"refusing proof-tree path {obj}")


def load_problems() -> Dict[str, dict]:
    with open(BENCH / "problems.json", "r", encoding="utf-8") as fh:
        return json.load(fh)


def preamble(w: Witness) -> str:
    types = "; ".join(f"`{v}:{w.var_types[v]}`" for v in w.vars)
    bits = int(w.width_bits)
    digits = w.inputs[0].digits if w.inputs else 4
    bytes_per = bits // digits // 8
    shape = f"{bytes_per} * {digits}"
    lines = [
        f"MAP_EVERY X_GEN_TAC [{types}] THEN",
        "REWRITE_TAC[C_ARGUMENTS; C_RETURN; SOME_FLAGS; "
        "NONOVERLAPPING_CLAUSES] THEN",
        "DISCH_THEN(REPEAT_TCL CONJUNCTS_THEN ASSUME_TAC) THEN",
        'REWRITE_TAC[BIGNUM_FROM_MEMORY_BYTES] THEN ENSURES_INIT_TAC "s0" THEN',
    ]
    for inp in w.inputs:
        lines.append(
            f'BIGNUM_DIGITIZE_TAC "{inp.digit_prefix}" '
            f"`read (memory :> bytes ({inp.pointer},{shape})) s0` THEN")
    # Drop the trailing THEN; the assembler adds the join.
    lines[-1] = lines[-1][:-len(" THEN")]
    return "\n".join(lines)


def emit(w: Witness) -> str:
    return "\n".join([
        preamble(w) + " THEN",
        w4_close.simulation(w) + " THEN",
        w4_close.close(w),
    ]) + "\n"


def build(op: str, task: str) -> Witness:
    problems = load_problems()
    if task not in problems:
        raise SystemExit(f"unknown task id {task!r}")
    query = problems[task]["query"]
    theorem = problems[task]["json"]["theorem_name"]
    exec_name = theorem.removesuffix("_CORRECT") + "_EXEC"
    obj = object_for(op, task)
    if not obj.is_file():
        raise SystemExit(f"missing object file {obj}")
    guard_object(obj)
    steps = disasm_steps.disassemble(str(obj))
    return w2_witnesses.build_witness(
        op, steps, query, theorem_name=theorem, exec_name=exec_name)


def main(argv: Optional[List[str]] = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--task", required=True, help="benchmark problem id")
    ap.add_argument("--out", help="write the answer here")
    ap.add_argument("--dir", help="write <dir>/<task>/answer.txt")
    ap.add_argument("--receipt", help="write the witness + metadata JSON here")
    ap.add_argument("--allow-heldout", action="store_true",
                    help="permit a held-out task id (frozen evaluation only)")
    args = ap.parse_args(argv)

    op = op_from_task(args.task)
    task = args.task

    assert_not_heldout(task, allow_heldout=args.allow_heldout)
    w = build(op, task)
    text = emit(w)

    out_path: Optional[Path] = None
    if args.dir:
        out_path = Path(args.dir) / task / "answer.txt"
    elif args.out:
        out_path = Path(args.out)
    if out_path is not None:
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(text, encoding="utf-8")

    if args.receipt:
        payload = {
            "task": task,
            "op": op,
            "answer_sha256": __import__("hashlib").sha256(
                text.encode()).hexdigest(),
            "answer": text,
            "witness": w.to_dict(),
            "witness_digest": w.digest(),
            "development_only": w4_close.DEVELOPMENT_ONLY,
        }
        Path(args.receipt).parent.mkdir(parents=True, exist_ok=True)
        Path(args.receipt).write_text(
            json.dumps(payload, indent=2, sort_keys=True) + "\n",
            encoding="utf-8")

    sys.stdout.write(text)
    return 0


if __name__ == "__main__":
    sys.exit(main())
