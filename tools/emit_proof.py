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
  python3 tools/emit_proof.py --op add --out /tmp/ans/add/answer.txt
  python3 tools/emit_proof.py --task arm.bignum_sub_p256.BIGNUM_SUB_P256_CORRECT \
                              --dir /tmp/ans --receipt /tmp/ans/sub.json
"""

from __future__ import annotations

import argparse
import json
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


OP_BY_MARKER = (
    ("bignum_add_p256.", "add"),
    ("bignum_sub_p256.", "sub"),
    ("bignum_neg_p256.", "neg"),
)


def op_from_task(task: str) -> str:
    for marker, op in OP_BY_MARKER:
        if marker in task:
            return op
    raise SystemExit(f"cannot determine op from task id {task!r}")


def assert_not_heldout(text: str) -> None:
    """Fail closed if a held-out curve name appears in *text*."""
    held_out = ("25519", "secp256k1", "p256k1", "sm2")
    low = text.lower()
    for marker in held_out:
        if marker in low:
            raise SystemExit(f"refusing held-out marker {marker!r} in {text!r}")


def guard_object(obj: Path) -> None:
    """Run the project guard CLI on the object before disassembling it."""
    import subprocess

    proc = subprocess.run(
        [sys.executable, str(HERE / "heldout_guard.py"), str(obj),
         "--root", str(S2N)],
        capture_output=True, text=True,
    )
    if proc.returncode != 0:
        raise SystemExit(f"heldout_guard refused {obj}: {proc.stderr.strip()}")


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
    obj = S2N / "arm" / "p256" / f"bignum_{op}_p256.o"
    if not obj.is_file():
        raise SystemExit(f"missing object file {obj}")
    guard_object(obj)
    steps = disasm_steps.disassemble(str(obj))
    return w2_witnesses.build_witness(
        op, steps, query, theorem_name=theorem, exec_name=exec_name)


def main(argv: Optional[List[str]] = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--op", choices=["add", "sub", "neg"])
    g.add_argument("--task", help="benchmark problem id")
    ap.add_argument("--out", help="write the answer here")
    ap.add_argument("--dir", help="write <dir>/<task>/answer.txt")
    ap.add_argument("--receipt", help="write the witness + metadata JSON here")
    args = ap.parse_args(argv)

    if args.task:
        op = op_from_task(args.task)
        task = args.task
    else:
        op = args.op
        task = {
            "add": "arm.bignum_add_p256.BIGNUM_ADD_P256_CORRECT",
            "sub": "arm.bignum_sub_p256.BIGNUM_SUB_P256_CORRECT",
            "neg": "arm.bignum_neg_p256.BIGNUM_NEG_P256_CORRECT",
        }[op]

    assert_not_heldout(task)
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
