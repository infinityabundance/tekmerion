#!/usr/bin/env python3
"""Replace attempt-lifecycle sentinels in a TacticTrace-generated file.

`tools/run_traces.sh` puts three ordinary OCaml comments in the proof template:

    (* TEKMERION_ATTEMPT_START *)
    (* TEKMERION_ATTEMPT_TIMEOUT *)
    (* TEKMERION_ATTEMPT_END *)

The template is type-checked by `TacticTrace/get-ast.sh` *before* the failure
module is spliced in, so the template cannot name `exptrace_*` directly. The
comments survive `tracer modify`; this tool replaces each sentinel line with
the real call, after the module has been placed in the compilation unit.

It fails closed if a sentinel is missing or duplicated.

Usage:
    python3 tools/attempt_lifecycle.py INPUT OUTPUT
    python3 tools/attempt_lifecycle.py --self-test
"""

from __future__ import annotations

import sys
from pathlib import Path

START = "(* TEKMERION_ATTEMPT_START *)"
TIMEOUT = "(* TEKMERION_ATTEMPT_TIMEOUT *)"
END = "(* TEKMERION_ATTEMPT_END *)"

BLOCKS = {
    START: [
        'exptrace_attempt_start (Sys.getenv "TEKMERION_ATTEMPT_ID") '
        '(Sys.getenv "TEKMERION_THEOREM_ID") '
        '(Sys.getenv "TEKMERION_PROOF_SHA") '
        '(Sys.getenv "TEKMERION_INJECTION") '
        '(Sys.getenv "TEKMERION_WRAPPER_SHA") '
        '(Sys.getenv "TEKMERION_MODULE_SHA");;',
        'exptrace_attempt_expect "failed";;',
    ],
    TIMEOUT: [
        "let rt_timeout = (try int_of_string "
        '(Sys.getenv "TEKMERION_TIMEOUT_S") with Not_found -> 0);;',
        "if rt_timeout > 0 then "
        "Sys.set_signal Sys.sigalrm "
        '(Sys.Signal_handle (fun _ -> exptrace_attempt_end "timeout"; '
        "exit 124));;",
        "if rt_timeout > 0 then ignore (Unix.alarm rt_timeout);;",
    ],
    END: [
        'exptrace_attempt_end "proved";;',
        "if rt_timeout > 0 then ignore (Unix.alarm 0);;",
    ],
}


class LifecycleError(Exception):
    pass


def expand(text: str) -> str:
    lines = text.split("\n")
    counts = {sentinel: 0 for sentinel in BLOCKS}
    out: list[str] = []
    for line in lines:
        stripped = line.strip()
        if stripped in BLOCKS:
            counts[stripped] += 1
            out.extend(BLOCKS[stripped])
        else:
            out.append(line)
    missing = [sentinel for sentinel, n in counts.items() if n != 1]
    if missing:
        raise LifecycleError(
            "expected exactly one of each sentinel; missing or duplicated: "
            + ", ".join(missing)
        )
    return "\n".join(out)


def self_test() -> int:
    ok = True
    template = (
        "let rt_start = Unix.time();;\n"
        f"{START}\n"
        f"{TIMEOUT}\n"
        'check_axioms ();;\n'
        f"{END}\n"
        'Printf.printf "ok\\n";;\n'
    )
    result = expand(template)
    if "exptrace_attempt_start" in result and result.count("exptrace_attempt_end") == 2:
        print("  ok: sentinels expand to the lifecycle calls")
    else:
        print("  FAIL: expansion is wrong")
        ok = False
    try:
        expand("let x = 1\n")
    except LifecycleError as exc:
        print(f"  ok: missing sentinels are refused: {exc}")
    else:
        print("  FAIL: missing sentinels were accepted")
        ok = False
    try:
        expand(f"{START}\n{START}\n{TIMEOUT}\n{END}\n")
    except LifecycleError as exc:
        print(f"  ok: duplicated sentinels are refused: {exc}")
    else:
        print("  FAIL: duplicated sentinels were accepted")
        ok = False
    print("SELF-TEST PASSED" if ok else "SELF-TEST FAILED")
    return 0 if ok else 1


def main(argv: list[str]) -> int:
    if len(argv) == 2 and argv[1] == "--self-test":
        return self_test()
    if len(argv) != 3:
        print(__doc__.strip(), file=sys.stderr)
        return 2
    source, destination = Path(argv[1]), Path(argv[2])
    try:
        destination.write_text(expand(source.read_text(encoding="utf-8")),
                               encoding="utf-8")
    except (OSError, LifecycleError) as exc:
        print(f"attempt_lifecycle: error: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
