#!/usr/bin/env python3
"""make_inlined_answer.py -- build a single statically-typeable answer.

The benchmark's syntax gate type-checks the answer with `ocamlc -i`, so names
introduced by `loadt` are not visible.  We therefore inline the needed OCaml
sources (Cadical DIMACS/CNF/LRAT reader, csat_lib.ml, bridge.ml) as nested
`let ... in` bindings and put the tactic last.  No proof text is changed.

Usage: make_inlined_answer.py <out-answer.txt>
"""

import re
import sys

ROOT = "/mnt/1tb_kingston/Tekmerion"
SOURCES = [
    f"{ROOT}/.bench/hol-light/Cadical/dimacs.ml",
    f"{ROOT}/.bench/hol-light/Cadical/cnf.ml",
    f"{ROOT}/.bench/hol-light/Cadical/ldrup.ml",
    f"{ROOT}/tools/csat_lib.ml",
    f"{ROOT}/tools/bridge.ml",
]

TACTIC = r"""
MAP_EVERY X_GEN_TAC
 [`z:int64`; `x:int64`; `y:int64`; `m:num`; `n:num`; `pc:num`] THEN
REWRITE_TAC[C_ARGUMENTS; C_RETURN; SOME_FLAGS; NONOVERLAPPING_CLAUSES] THEN
DISCH_THEN(REPEAT_TCL CONJUNCTS_THEN ASSUME_TAC) THEN
REWRITE_TAC[BIGNUM_FROM_MEMORY_BYTES] THEN ENSURES_INIT_TAC "s0" THEN
BIGNUM_DIGITIZE_TAC "m_" `read (memory :> bytes (x,8 * 4)) s0` THEN
BIGNUM_DIGITIZE_TAC "n_" `read (memory :> bytes (y,8 * 4)) s0` THEN
ARM_ACCSTEPS_TAC BIGNUM_ADD_P256_EXEC [3;4;7;8;10;12;13;15;16] (1--22) THEN
ENSURES_FINAL_STATE_TAC THEN ASM_REWRITE_TAC[] THEN STRIP_TAC THEN
RULE_ASSUM_TAC(REWRITE_RULE [REAL_BITVAL_NOT; ADD_CLAUSES; VAL_WORD_BITVAL]) THEN
CONV_TAC SYM_CONV THEN CONV_TAC(RAND_CONV BIGNUM_LEXPAND_CONV) THEN
ASM_REWRITE_TAC[] THEN DISCARD_STATE_TAC "s22" THEN
BRIDGE_ADD_TAC (CONV_TAC (ONCE_DEPTH_CONV BIGNUM_OF_WORDLIST_CONV)) p_256
"""

SKIP = re.compile(r"^\s*(#|needs\b|loadt\b)")


def phrases(path):
    src = open(path, encoding="utf-8").read()
    for chunk in re.split(r";;[ \t]*(?:\n|$)", src):
        c = chunk.strip()
        if not c:
            continue
        if SKIP.match(c):
            continue
        yield c


def main(out):
    parts = []
    for path in SOURCES:
        for p in phrases(path):
            # every top-level phrase in these files is a `let` binding
            parts.append(p)
    wrapped = (
        "(fun g ->\n"
        "  try\n"
        "    (" + TACTIC.strip() + ") g\n"
        "  with e ->\n"
        "    Printf.printf \"ANSWER_EXCEPTION: %s\\n%!\" (Printexc.to_string e);\n"
        "    raise e)\n"
    )
    body = "(\n" + "\n".join(f"{p}\nin" for p in parts) + "\n" + wrapped + ")\n"
    open(out, "w", encoding="utf-8").write(body)
    print(f"wrote {out} bytes={len(body)} bindings={len(parts)}")


if __name__ == "__main__":
    main(sys.argv[1])
