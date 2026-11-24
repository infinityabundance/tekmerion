#!/usr/bin/env python3
"""emit_ref_proof.py -- assemble a reference-route answer for an ARM add/sub/neg task.

The route simulated here is:

  * digitize the inputs and run the machine code (the accumulate set is derived
    from the instruction stream by tools/derive_steps.py; a routine with no
    carry chain at all is run with plain ARM_STEPS_TAC),
  * expand the written bignum into a digit sum,
  * close with BRIDGE_{ADD,SUB,NEG}_REF_TAC.

No flag *meaning* is derived and no curve arithmetic appears in the close: the
only residual obligation handed to certified SAT is a word equation
`val(<written words>) = val(ref_{op} (word p) x y)` with the range hypotheses
(`x < p`, `y < p`, or `n <= p` for neg) among the bit-level assumptions, plus
the machine's own accumulator equations.

The machine code, the query, the state names and the modulus name are read
from the benchmark inputs; nothing is copied from a human proof except the
simulation/expansion idiom, which is labelled DEVELOPMENT_ONLY.

Guard modes come from tools/heldout_guard.py.  ``--develop`` (default) refuses
held-out task ids, objects and proof reads; ``--sealed-eval`` with an explicit
manifest and freeze tag allows exactly the listed ids and objects and still
refuses their proof text, including in ``--mode dev``.

Usage:
  python3 tools/emit_ref_proof.py --task <id> --mode answer --out answer.txt
  python3 tools/emit_ref_proof.py --task <id> --mode dev --out dev.ml
  python3 tools/emit_ref_proof.py --sealed-eval \
      --sealed-manifest research/receipts/sealed.json \
      --freeze-tag freeze-x86-sealed-v1 --task <id> --mode answer --out answer.txt
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

import disasm_steps  # noqa: E402
import derive_steps  # noqa: E402
import heldout_guard  # noqa: E402
import w2_witnesses  # noqa: E402

OP_RE = re.compile(r"bignum_(add|sub|neg)_")
FAMILY_RE = re.compile(r"^arm\.bignum_(?:add|sub|neg)_([a-z0-9]+)\.")


def _guard(fn, *args, **kwargs):
    """Run one guard check, turning a refusal into a clean SystemExit."""
    try:
        return fn(*args, **kwargs)
    except heldout_guard.HeldOutRefused as exc:
        raise SystemExit(f"refused: {exc}") from exc


# Transcribed fragments, named so they cannot hide in the emitted answer.
DEVELOPMENT_ONLY = {
    "simulation.accsteps":
        "The single ARM_ACCSTEPS_TAC block over the whole routine is the human "
        "proofs' simulation idiom; the accumulate set itself is derived.",
    "post.digit_expansion":
        "BIGNUM_EXPAND_CONV / BIGNUM_LEXPAND_CONV with the LAND_CONV/RAND_CONV "
        "traversal and DISCARD_STATE_TAC are the human expansion idiom.",
    "post.bitval_rewrites":
        "RULE_ASSUM_TAC(REWRITE_RULE[REAL_BITVAL_NOT; ADD_CLAUSES; "
        "VAL_WORD_BITVAL]) is copied from the human add closes.",
}


def load_problems():
    import json
    with open(BENCH / "problems.json", "r", encoding="utf-8") as fh:
        return json.load(fh)


def object_for(op: str, task: str, policy=None) -> Path:
    m = FAMILY_RE.match(task)
    if not m:
        raise SystemExit(f"not an ARM bignum task id: {task!r}")
    family = m.group(1)
    matches = sorted(S2N.glob(f"arm/*/bignum_{op}_{family}.o"))
    if len(matches) != 1:
        raise SystemExit(f"expected one object for {op}/{family}, found {matches}")
    policy = policy or heldout_guard.develop_policy()
    _guard(policy.check_object, str(matches[0]), root=S2N)
    return matches[0]


def modulus_definition(op: str, family: str, policy=None) -> str:
    """The `let p_<family> = new_definition ...` phrase, for --mode dev only.

    The judged answer does not use this: the benchmark setup already defines the
    modulus.  It is needed only to run the same tactic from a scratch process.
    """
    proof = S2N / "arm" / "proofs" / f"bignum_{op}_{family}.ml"
    policy = policy or heldout_guard.develop_policy()
    _guard(policy.check_proof, str(proof), root=S2N)
    src = proof.read_text(encoding="utf-8")
    m = re.search(r"^(let (p_\w+) = new_definition\s*`[^`]*`;;)",
                  src, re.M)
    if not m:
        raise SystemExit(f"no modulus definition found in {proof}")
    return m.group(1)


def machine_code_phrase(op: str, family: str, mcname: str, objrel: str,
                        policy=None) -> str:
    """The `define_assert_from_elf ... [words]` phrase, for --mode dev only.

    The judged answer uses the setup's own machine-code definition.  A scratch
    process needs one, so the instruction words are read back out of the same
    `.o` file the pipeline uses (the proof file records the encoding of that
    object; the words are also what the ELF contains).
    """
    proof = S2N / "arm" / "proofs" / f"bignum_{op}_{family}.ml"
    policy = policy or heldout_guard.develop_policy()
    _guard(policy.check_proof, str(proof), root=S2N)
    src = proof.read_text(encoding="utf-8")
    m = re.search(r"define_assert_from_elf\s*\"[^\"]*\"\s*\"[^\"]*\"\s*\[(.*?)\];;",
                  src, re.S)
    if not m:
        raise SystemExit(f"no define_assert_from_elf block in {proof}")
    words = m.group(1)
    return (f'let {mcname} = define_assert_from_elf "{mcname}" '
            f'"{objrel}"[{words}];;')


def preamble(spec) -> str:
    types = "; ".join(f"`{v}:{spec['var_types'][v]}`" for v in spec["variables"])
    bits = int(spec["bits"])
    digits = int(spec["digits"])
    bytes_per = bits // digits // 8
    shape = f"{bytes_per} * {digits}"
    lines = [
        f"MAP_EVERY X_GEN_TAC [{types}]",
        "REWRITE_TAC[C_ARGUMENTS; C_RETURN; SOME_FLAGS; NONOVERLAPPING_CLAUSES]",
        "DISCH_THEN(REPEAT_TCL CONJUNCTS_THEN ASSUME_TAC)",
        'REWRITE_TAC[BIGNUM_FROM_MEMORY_BYTES] THEN ENSURES_INIT_TAC "s0"',
    ]
    for inp in spec["inputs"]:
        lines.append(
            f'BIGNUM_DIGITIZE_TAC "{inp.digit_prefix}" '
            f"`read (memory :> bytes ({inp.pointer},{shape})) s0`")
    return " THEN\n".join(lines)


def simulation(exec_name: str, accumulate, last: int) -> str:
    if accumulate:
        acc = "[" + ";".join(str(n) for n in accumulate) + "]"
        return f"ARM_ACCSTEPS_TAC {exec_name} {acc} (1--{last})"
    return f"ARM_STEPS_TAC {exec_name} (1--{last})"


_POST = {
    "add": (
        "ENSURES_FINAL_STATE_TAC THEN ASM_REWRITE_TAC[] THEN STRIP_TAC THEN\n"
        "RULE_ASSUM_TAC(REWRITE_RULE [REAL_BITVAL_NOT; ADD_CLAUSES; "
        "VAL_WORD_BITVAL]) THEN\n"
        "CONV_TAC SYM_CONV THEN CONV_TAC(RAND_CONV BIGNUM_LEXPAND_CONV) THEN\n"
        "ASM_REWRITE_TAC[] THEN DISCARD_STATE_TAC \"{state}\" THEN\n"
        "BRIDGE_ADD_REF_TAC (CONV_TAC (ONCE_DEPTH_CONV "
        "BIGNUM_OF_WORDLIST_CONV)) {modulus}"
    ),
    "sub": (
        "ENSURES_FINAL_STATE_TAC THEN ASM_REWRITE_TAC[] THEN STRIP_TAC THEN\n"
        "CONV_TAC(LAND_CONV(RAND_CONV BIGNUM_EXPAND_CONV)) THEN\n"
        "ASM_REWRITE_TAC[] THEN DISCARD_STATE_TAC \"{state}\" THEN\n"
        "BRIDGE_SUB_REF_TAC (CONV_TAC (ONCE_DEPTH_CONV "
        "BIGNUM_OF_WORDLIST_CONV)) {modulus}"
    ),
    "neg": (
        "ENSURES_FINAL_STATE_TAC THEN ASM_REWRITE_TAC[] THEN STRIP_TAC THEN\n"
        "CONV_TAC(LAND_CONV BIGNUM_EXPAND_CONV) THEN\n"
        "ASM_REWRITE_TAC[] THEN DISCARD_STATE_TAC \"{state}\" THEN\n"
        "BRIDGE_NEG_REF_TAC (CONV_TAC (ONCE_DEPTH_CONV "
        "BIGNUM_OF_WORDLIST_CONV)) {modulus}"
    ),
}


def build(op: str, task: str, policy=None):
    policy = policy or heldout_guard.develop_policy()
    _guard(policy.check_task, task)
    problems = load_problems()
    if task not in problems:
        raise SystemExit(f"unknown task id {task!r}")
    query = problems[task]["query"]
    theorem = problems[task]["json"]["theorem_name"]
    exec_name = theorem.removesuffix("_CORRECT") + "_EXEC"
    obj = object_for(op, task, policy)
    steps = disasm_steps.disassemble(str(obj))
    spec = w2_witnesses.parse_spec(query)
    accumulate = derive_steps.derive_accumulate(steps)
    last = len(steps) - 1
    modulus = str(spec["conclusion"]["modulus"])
    tactic = "\n".join([
        preamble(spec) + " THEN",
        simulation(exec_name, accumulate, last) + " THEN",
        _POST[op].format(state=f"s{last}", modulus=modulus),
    ])
    return {
        "task": task, "op": op, "query": query, "spec": spec,
        "accumulate": accumulate, "exec_name": exec_name,
        "objrel": "/".join(obj.relative_to(S2N).parts),
        "family": FAMILY_RE.match(task).group(1),
        "modulus": modulus, "tactic": tactic, "last": last,
    }


# --------------------------------------------------------------------------
# Answer assembly (the judged artefact)
# --------------------------------------------------------------------------

SKIP = re.compile(r"^\s*(#|needs\b|loadt\b)")


def phrases(path: Path):
    src = path.read_text(encoding="utf-8")
    for chunk in re.split(r";;[ \t]*(?:\n|$)", src):
        c = chunk.strip()
        if not c or SKIP.match(c):
            continue
        yield c


def inlined_answer(tactic: str) -> str:
    sources = [BENCH / "hol-light/Cadical/dimacs.ml",
               BENCH / "hol-light/Cadical/cnf.ml",
               BENCH / "hol-light/Cadical/ldrup.ml",
               HERE / "csat_lib.ml",
               HERE / "bridge.ml"]
    parts = [p for path in sources for p in phrases(path)]
    wrapped = (
        "(fun g ->\n"
        "  try\n"
        "    (" + tactic.strip() + ") g\n"
        "  with e ->\n"
        "    Printf.printf \"ANSWER_EXCEPTION: %s\\n%!\" (Printexc.to_string e);\n"
        "    raise e)\n"
    )
    return "(\n" + "\n".join(f"{p}\nin" for p in parts) + "\n" + wrapped + ")\n"


def dev_script(info, policy=None) -> str:
    policy = policy or heldout_guard.develop_policy()
    mcname = info["exec_name"].lower().removesuffix("_exec") + "_mc"
    moddef = modulus_definition(info["op"], info["family"], policy)
    return "\n".join([
        f"(* DEVELOPMENT_ONLY dev harness for {info['task']}.",
        "   Not a receipt: the judged artefact is the inlined answer. *)",
        '#load "unix.cma";;',
        'needs "arm/proofs/base.ml";;',
        'needs "Cadical/make.ml";;',
        f'loadt "{REPO}/tools/csat_lib.ml";;',
        f'loadt "{REPO}/tools/bridge.ml";;',
        moddef,
        machine_code_phrase(info["op"], info["family"], mcname,
                            info["objrel"], policy),
        f"let {info['exec_name']} = ARM_MK_EXEC_RULE {mcname};;",
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
    heldout_guard.add_mode_arguments(ap)
    args = ap.parse_args(argv)

    try:
        policy = heldout_guard.policy_from_args(args)
    except heldout_guard.HeldOutRefused as exc:
        raise SystemExit(f"refused: {exc}") from exc

    op = OP_RE.search(args.task).group(1)
    info = build(op, args.task, policy)
    text = inlined_answer(info["tactic"]) if args.mode == "answer" \
        else dev_script(info, policy)
    Path(args.out).parent.mkdir(parents=True, exist_ok=True)
    Path(args.out).write_text(text, encoding="utf-8")
    if args.accumulate_out:
        Path(args.accumulate_out).write_text(
            " ".join(str(n) for n in info["accumulate"]) + "\n")
    print(f"wrote {args.out} mode={args.mode} task={args.task} "
          f"accumulate={info['accumulate']} bytes={len(text)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
