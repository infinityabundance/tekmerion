#!/usr/bin/env python3
"""emit_ref_proof_x86.py -- reference-route answer for an x86 add/sub/neg task.

This is the x86 sibling of :mod:`tools.emit_ref_proof`.  The mathematics is the
same value-level reference route:

  * digitize the inputs and step the machine code (the accumulate set comes from
    :func:`derive_steps.derive_accumulate` over :data:`derive_steps.X86_SEMANTICS`),
  * expand the written bignum into a digit sum,
  * close with ``BRIDGE_{ADD,SUB,NEG}_REF_TAC``.

What is x86-specific, and derived rather than assumed:

  * the step list is disassembled from the x86 ELF object in Intel syntax;
    ``define_trimmed`` drops the four-byte ``ENDBR64`` prefix, so the tool
    requires that prefix and reports trimmed-relative addresses;
  * the entry offset is read from the query's ``read RIP s = word (pc + k)``.
    The x86 ``CORRECT`` statements enter after a ``PUSH`` prologue, so the
    prologue is not simulated and does not take a step number;
  * the simulation ends at the last instruction that writes memory.  The
    postcondition's ``read RIP`` is checked to be the address of the *next*
    instruction, which fails closed if the disassembly and the query disagree;
  * ``cfg`` flows are *not* read.  The reference route never interprets the
    carry flag, which matters on x86 because its CF is a **borrow** for
    ``SUB``/``SBB`` while AArch64's C is a **no-borrow** (see
    ``derive_steps.X86_CF_MEANING``).

Transcribed fragments are named in ``DEVELOPMENT_ONLY`` and copied into the
receipt.  ``--mode dev`` reads the modulus definition and the
``define_assert_from_elf`` word list out of the (non-held-out) human proof to
build a scratch process; the judged answer does not.

Held-out families are refused: this tool cannot be pointed at the sealed
counterparts even by mistake.

Usage:
  python3 tools/emit_ref_proof_x86.py --task ID --mode answer --out answer.txt
  python3 tools/emit_ref_proof_x86.py --task ID --mode dev --out dev.ml
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parent
BENCH = REPO / ".bench"
S2N = BENCH / "s2n-bignum"

if str(HERE) not in sys.path:
    sys.path.insert(0, str(HERE))

import derive_steps  # noqa: E402
import disasm_steps  # noqa: E402
import emit_ref_proof  # noqa: E402
import heldout_guard  # noqa: E402
import w2_witnesses  # noqa: E402

TASK_RE = re.compile(r"^x86\.bignum_(add|sub|neg)_([a-z0-9]+)\.")
OP_RE = re.compile(r"bignum_(add|sub|neg)_")

# Included so the label travels with every emitted answer, not only the code.
DEVELOPMENT_ONLY = {
    "simulation.accsteps":
        "The single X86_ACCSTEPS_TAC block over the whole routine is the x86 "
        "human proofs' simulation idiom; the accumulate set itself is derived. "
        "x86 NEG is dropped from the requested set because ACCUMULATE_ARITH_TAC "
        "has no word_neg pattern (requesting it is a no-op).",
    "simulation.entry":
        "Skipping the PUSH prologue follows the query's `read RIP s = word "
        "(pc + k)`; that the harness means an executable entry (rather than a "
        "non-executed prefix) is taken from the x86 proof style.",
    "post.neg_flag_rewrite":
        "RULE_ASSUM_TAC(REWRITE_RULE[WORD_NEG_EQ_0; WORD_BITVAL_EQ_0]) is "
        "copied from the x86 human closes. It is emitted only when the routine "
        "contains NEG: NEG's CF is `~(x = 0)`, and without this rewrite the "
        "bit-valued form `bitval(~(word_neg(word(bitval c)) = word 0))` is left "
        "free by BITBLAST, making the residual SAT instance satisfiable (measured "
        "on four of the six declared tasks).",
    "post.zero_test_fold":
        "BRIDGE_NEG_REF_ZERO_TAC folds the machine's OR zero test and the "
        "reference function's joined-word zero test to the same limb-wise "
        "conjunction before the case split (BRIDGE_ZERO_TEST_TAC, built from "
        "WORD_OR_EQ_0 and WORD_JOIN_EQ_0, both proved in tools/bridge.ml). The "
        "fold is generic; what is transcribed is the decision to use it for "
        "routines containing NEG. Measured: BITBLAST alone does not prove "
        "`word_join a b = word 0 ==> word_or a b = word 0'.",
    "post.digit_expansion":
        "BIGNUM_EXPAND_CONV / BIGNUM_LEXPAND_CONV with the LAND_CONV/RAND_CONV "
        "traversal and DISCARD_STATE_TAC are the human expansion idiom.",
    "post.bitval_rewrites":
        "RULE_ASSUM_TAC(REWRITE_RULE[REAL_BITVAL_NOT; ADD_CLAUSES; "
        "VAL_WORD_BITVAL]) is copied from the human add closes.",
}


def _refuse_heldout(task: str) -> None:
    low = task.lower()
    for marker in heldout_guard.DEFAULT_HELD_OUT:
        if marker.lower() in low:
            raise SystemExit(
                f"refused: task {task!r} carries held-out marker {marker!r}"
            )


def load_problems():
    import json
    with open(BENCH / "problems.json", "r", encoding="utf-8") as fh:
        return json.load(fh)


def object_for(op: str, task: str) -> Path:
    m = TASK_RE.match(task)
    if not m or m.group(1) != op:
        raise SystemExit(f"not an x86 bignum {op} task id: {task!r}")
    family = m.group(2)
    matches = sorted(S2N.glob(f"x86/*/bignum_{op}_{family}.o"))
    if len(matches) != 1:
        raise SystemExit(f"expected one x86 object for {op}/{family}, found {matches}")
    heldout_guard.check_path(str(matches[0]), root=S2N)
    return matches[0]


_RIP = re.compile(
    r"read\s+RIP\s+s\s*=\s*word\s*\(?\s*pc\s*(?:\+\s*(0x[0-9a-fA-F]+))?\s*\)?")


def entry_and_end(query: str) -> tuple[int, int]:
    """The precondition entry and postcondition target, as trimmed offsets."""
    found = _RIP.findall(query)
    if len(found) != 2:
        raise SystemExit(
            f"expected exactly two `read RIP s = word (pc + k)` clauses, "
            f"found {len(found)}"
        )
    values = [int(v, 16) if v else 0 for v in found]
    return values[0], values[1]


def accumulate_set(steps) -> list[int]:
    """Derived x86 accumulate list, minus the mnemonic ACCUMULATE cannot unfold."""
    derived = derive_steps.derive_accumulate(steps, derive_steps.X86_SEMANTICS)
    return [n for n in derived if steps[n - 1].mnemonic != "neg"]


def last_store(steps, end: int) -> int:
    """Index of the last instruction that writes memory, with a consistency check.

    The postcondition's target address must be the address of the instruction
    right after that store.  If it is not, the disassembly and the query do not
    describe the same code and no step range can be trusted.
    """
    stores = [s.index for s in steps if s.operands and "[" in s.operands[0]]
    if not stores:
        raise SystemExit("no memory-writing instruction found in the routine")
    last = stores[-1]
    if last >= len(steps):
        raise SystemExit("last store is the final instruction; no trailing address")
    next_address = steps[last].address  # steps is 1-based: index last+1
    if next_address != end:
        raise SystemExit(
            f"disassembly/query mismatch: after the last store (step {last}) "
            f"the next instruction is at {next_address}, but the query's "
            f"postcondition RIP is {end}"
        )
    return last


def simulation(exec_name: str, accumulate, last: int) -> str:
    if accumulate:
        acc = "[" + ";".join(str(n) for n in accumulate) + "]"
        return f"X86_ACCSTEPS_TAC {exec_name} {acc} (1--{last})"
    return f"X86_STEPS_TAC {exec_name} (1--{last})"


# The NEG-derived CF is `~(x = 0)`.  Rewriting the two eq-0 facts before the
# bridge turns `bitval(~(word_neg(word(bitval c)) = word 0))` into `c`, which
# BITBLAST can encode; without it that bit is free and the instance is
# satisfiable.  Both lemmas are generic word identities from Library/words.ml.
_NEG_CF_REWRITE = (
    "RULE_ASSUM_TAC(REWRITE_RULE[WORD_NEG_EQ_0; WORD_BITVAL_EQ_0])"
)


def post_simulation(op: str, state: str, modulus: str,
                    uses_neg: bool) -> str:
    text = emit_ref_proof._POST[op].format(state=state, modulus=modulus)
    if not uses_neg:
        return text
    if op == "neg":
        # The reference function's guard is `x = word 0` on a joined word; the
        # machine tests zero with an OR of the limbs.  The ZERO variant folds
        # both to the same limb-wise conjunction before the case split.
        text = text.replace("BRIDGE_NEG_REF_TAC", "BRIDGE_NEG_REF_ZERO_TAC")
    lines = text.split("\n")
    # The bridge call is the final line; apply the carry rewrite just before it.
    lines.insert(len(lines) - 1, _NEG_CF_REWRITE + " THEN")
    return "\n".join(lines)


def build(op: str, task: str):
    _refuse_heldout(task)
    problems = load_problems()
    if task not in problems:
        raise SystemExit(f"unknown task id {task!r}")
    query = problems[task]["query"]
    theorem = problems[task]["json"]["theorem_name"]
    exec_name = theorem.removesuffix("_CORRECT") + "_EXEC"
    obj = object_for(op, task)
    entry, end = entry_and_end(query)
    full_steps = disasm_steps.disassemble(str(obj), arch="x86")
    steps = disasm_steps.enter_at(full_steps, entry)
    spec = w2_witnesses.parse_spec(query)
    accumulate = accumulate_set(steps)
    last = last_store(steps, end)
    uses_neg = any(s.mnemonic == "neg" for s in steps)
    modulus = str(spec["conclusion"]["modulus"])
    tactic = "\n".join([
        emit_ref_proof.preamble(spec) + " THEN",
        simulation(exec_name, accumulate, last) + " THEN",
        post_simulation(op, f"s{last}", modulus, uses_neg),
    ])
    return {
        "task": task, "op": op, "query": query, "spec": spec,
        "accumulate": accumulate, "exec_name": exec_name,
        "objrel": "/".join(obj.relative_to(S2N).parts),
        "family": TASK_RE.match(task).group(2),
        "modulus": modulus, "tactic": tactic, "last": last,
        "entry": entry, "end": end, "uses_neg": uses_neg,
        "development_only": sorted(DEVELOPMENT_ONLY),
    }


# --------------------------------------------------------------------------
# Development harness (scratch process only; not the judged artefact)
# --------------------------------------------------------------------------

def modulus_definition(op: str, family: str) -> str:
    proof = S2N / "x86" / "proofs" / f"bignum_{op}_{family}.ml"
    heldout_guard.check_path(str(proof), root=S2N)
    src = proof.read_text(encoding="utf-8")
    m = re.search(r"^(let (p_\w+) = new_definition\s*`[^`]*`;;)", src, re.M)
    if not m:
        raise SystemExit(f"no modulus definition found in {proof}")
    return m.group(1)


def machine_code_phrase(op: str, family: str, mcname: str, objrel: str) -> str:
    proof = S2N / "x86" / "proofs" / f"bignum_{op}_{family}.ml"
    heldout_guard.check_path(str(proof), root=S2N)
    src = proof.read_text(encoding="utf-8")
    m = re.search(r"define_assert_from_elf\s*\"[^\"]*\"\s*\"[^\"]*\"\s*\[(.*?)\];;",
                  src, re.S)
    if not m:
        raise SystemExit(f"no define_assert_from_elf block in {proof}")
    words = m.group(1)
    return (f'let {mcname} = define_assert_from_elf "{mcname}" '
            f'"{objrel}"[{words}];;')


def dev_script(info) -> str:
    mcname = info["exec_name"].lower().removesuffix("_exec") + "_mc"
    tmcname = mcname.removesuffix("_mc") + "_tmc"
    moddef = modulus_definition(info["op"], info["family"])
    return "\n".join([
        f"(* DEVELOPMENT_ONLY dev harness for {info['task']}.",
        "   Not a receipt: the judged artefact is the inlined answer.",
        "   Run from .bench/s2n-bignum (define_assert_from_elf resolves the",
        "   object relative to that directory). *)",
        '#load "unix.cma";;',
        'needs "x86/proofs/base.ml";;',
        'needs "Cadical/make.ml";;',
        f'loadt "{REPO}/tools/csat_lib.ml";;',
        f'loadt "{REPO}/tools/bridge.ml";;',
        moddef,
        machine_code_phrase(info["op"], info["family"], mcname, info["objrel"]),
        f'let {tmcname} = define_trimmed "{tmcname}" {mcname};;',
        f'let {info["exec_name"]} = X86_MK_CORE_EXEC_RULE {tmcname};;',
        f"let ref_goal = {info['query']};;",
        "let ref_t0 = Unix.gettimeofday ();;",
        f"let ref_th = time prove (ref_goal,\n{info['tactic']});;",
        "Printf.printf \"REF_DEV %s ok=%b seconds=%.3f axioms=%d\\n%!\"",
        f'  "{info["task"]}" (concl ref_th = ref_goal)',
        "  (Unix.gettimeofday () -. ref_t0) (List.length (axioms ()));;",
    ]) + "\n"


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--task", required=True)
    ap.add_argument("--mode", choices=["answer", "dev"], default="answer")
    ap.add_argument("--out", required=True)
    ap.add_argument("--accumulate-out", help="write the derived step list here")
    args = ap.parse_args(argv)

    m = OP_RE.search(args.task)
    if not m:
        raise SystemExit(f"not a bignum add/sub/neg task: {args.task!r}")
    op = m.group(1)
    info = build(op, args.task)
    text = (emit_ref_proof.inlined_answer(info["tactic"])
            if args.mode == "answer" else dev_script(info))
    Path(args.out).parent.mkdir(parents=True, exist_ok=True)
    Path(args.out).write_text(text, encoding="utf-8")
    if args.accumulate_out:
        Path(args.accumulate_out).write_text(
            " ".join(str(n) for n in info["accumulate"]) + "\n")
    print(f"wrote {args.out} mode={args.mode} task={args.task} "
          f"entry={info['entry']} last={info['last']} neg={info['uses_neg']} "
          f"accumulate={info['accumulate']} bytes={len(text)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
