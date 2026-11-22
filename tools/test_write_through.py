#!/usr/bin/env python3
"""Offline proof that exptrace_failure.ml is write-through.

HOL Light is not built here. The real module source is compiled against tiny
stubs for the three HOL Light/TacticTrace names it uses (`goal`,
`pp_print_goal`, `ExportTrace`). The harness then:

  1. records one failing tactic application and calls *nothing* else;
  2. asserts `<dir>/REWRITE_TAC.failures.jsonl` already exists and contains
     exactly one valid JSON object with the expected fields and
     `"scope": "tactic"`;
  3. separately exercises the optional consolidation dump, its pretty array
     output, and its refusal to overwrite existing output.

Usage:
    python3 tools/test_write_through.py
"""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MODULE = ROOT / "trace" / "exptrace_failure.ml"

EXPECTED_FIELDS = {
    "scope",
    "tactic",
    "goal_before",
    "exception_text",
    "subgoal_index",
    "elapsed_cpu_s",
}

STUBS = r"""
(* Minimal stubs: just enough to compile exptrace_failure.ml outside HOL Light. *)
type goal = string

let pp_print_goal fmt (g : goal) = Format.pp_print_string fmt g

module ExportTrace = struct
  let current_dir : string option ref = ref None
  let set_dir dir = current_dir := Some dir
  let get_sampling_config () = !current_dir
  let effective_output_dir = function
    | Some dir -> dir
    | None -> invalid_arg "write_through_test: no output directory"
end

let unset_jrh_lexer = ();;
let set_jrh_lexer = ();;
"""

DRIVER = r"""
let () =
  if Array.length Sys.argv < 2 then begin
    prerr_endline "usage: write_through_test OUTDIR [no-dump|dump|dump-twice]";
    exit 2
  end;
  let dir = Sys.argv.(1) in
  ExportTrace.set_dir dir;
  let goal : goal = "|- x = x" in
  (try
     ignore (exptrace_guard "REWRITE_TAC" goal (fun () -> failwith "boom"))
   with Failure _ -> ());
  let mode =
    if Array.length Sys.argv >= 3 then Sys.argv.(2) else "no-dump"
  in
  match mode with
  | "no-dump" ->
    (* Deliberately call nothing further. This is the simulated process exit:
       the record must already be durable. *)
    ()
  | "dump" ->
    exptrace_failures_dump ();
    print_endline "DUMP-OK"
  | "dump-twice" ->
    exptrace_failures_dump ();
    (try
       exptrace_failures_dump ();
       print_endline "SECOND-DUMP-ACCEPTED";
       exit 3
     with Invalid_argument _ ->
       print_endline "SECOND-DUMP-REFUSED")
  | other ->
    prerr_endline ("unknown mode: " ^ other);
    exit 2
"""


class TestFailure(Exception):
    pass


def check(condition: bool, message: str) -> None:
    if not condition:
        raise TestFailure(message)


def run(command: list[str]) -> subprocess.CompletedProcess[str]:
    proc = subprocess.run(command, capture_output=True, text=True)
    if proc.returncode != 0:
        raise TestFailure(
            f"command failed ({proc.returncode}): {' '.join(command)}\n"
            f"stdout:\n{proc.stdout}\nstderr:\n{proc.stderr}"
        )
    return proc


def check_record(record: object, where: str) -> dict:
    check(isinstance(record, dict), f"{where}: record is not a JSON object")
    check(
        set(record) == EXPECTED_FIELDS,
        f"{where}: fields are {sorted(record)}, expected {sorted(EXPECTED_FIELDS)}",
    )
    check(record["scope"] == "tactic", f"{where}: scope is {record['scope']!r}")
    check(record["tactic"] == "REWRITE_TAC", f"{where}: wrong tactic")
    check(record["goal_before"] == "|- x = x", f"{where}: wrong goal_before")
    check(
        "Failure" in record["exception_text"]
        and "boom" in record["exception_text"],
        f"{where}: wrong exception_text {record['exception_text']!r}",
    )
    check(record["subgoal_index"] is None, f"{where}: subgoal_index is not null")
    check(
        isinstance(record["elapsed_cpu_s"], (int, float))
        and not isinstance(record["elapsed_cpu_s"], bool)
        and record["elapsed_cpu_s"] >= 0,
        f"{where}: elapsed_cpu_s is not a nonnegative number",
    )
    return record


def main() -> int:
    if not MODULE.is_file():
        print(f"FAIL: missing {MODULE}", file=sys.stderr)
        return 1

    source = STUBS + "\n" + MODULE.read_text(encoding="utf-8") + "\n" + DRIVER
    ok = True

    try:
        with tempfile.TemporaryDirectory(prefix="exptrace-write-through-") as tmp:
            tmp_path = Path(tmp)
            source_path = tmp_path / "write_through_test.ml"
            executable = tmp_path / "write_through_test"
            source_path.write_text(source, encoding="utf-8")

            compile_proc = subprocess.run(
                ["ocamlc", "-w", "-a", "-o", str(executable), str(source_path)],
                capture_output=True,
                text=True,
            )
            if compile_proc.returncode != 0:
                print(
                    "FAIL: could not compile exptrace_failure.ml against stubs\n"
                    f"stdout:\n{compile_proc.stdout}\n"
                    f"stderr:\n{compile_proc.stderr}"
                )
                return 1
            print("  ok: exptrace_failure.ml compiles against the stubs")

            # --- write-through: record, then call nothing -----------------
            outdir = tmp_path / "no-dump"
            run([str(executable), str(outdir), "no-dump"])
            journal = outdir / "REWRITE_TAC.failures.jsonl"
            check(journal.is_file(), f"journal was not written: {journal}")
            raw = journal.read_text(encoding="utf-8")
            lines = raw.splitlines()
            check(
                len(lines) == 1,
                f"journal should contain exactly one line, found {len(lines)}",
            )
            check(raw.endswith("\n"), "journal line is not newline-terminated")
            record = check_record(json.loads(lines[0]), "journal line 1")
            check(
                not (outdir / "REWRITE_TAC.failures.json").exists(),
                "consolidated .json exists even though the dump was never called",
            )
            print(
                "  ok: write-through journal contains one valid JSON line "
                "before any dump"
            )

            # --- optional consolidation -----------------------------------
            dumpdir = tmp_path / "dump"
            proc = run([str(executable), str(dumpdir), "dump"])
            check("DUMP-OK" in proc.stdout, "dump mode did not complete")
            consolidated = dumpdir / "REWRITE_TAC.failures.json"
            check(consolidated.is_file(), "consolidated .json was not written")
            check(
                (dumpdir / "REWRITE_TAC.failures.jsonl").is_file(),
                "consolidation removed the raw journal",
            )
            data = json.loads(consolidated.read_text(encoding="utf-8"))
            check(isinstance(data, list), "consolidated output is not an array")
            check(len(data) == 1, f"consolidated array has {len(data)} records")
            check_record(data[0], "consolidated record 1")
            pretty = consolidated.read_text(encoding="utf-8")
            check(
                '  {\n    "scope": "tactic",\n' in pretty,
                "consolidated output is not in the success-dump field style",
            )
            check(
                data[0]["tactic"] == record["tactic"]
                and data[0]["goal_before"] == record["goal_before"]
                and data[0]["exception_text"] == record["exception_text"],
                "consolidated record does not match the journal record",
            )
            print("  ok: optional consolidation produces a pretty JSON array")

            # --- fail closed on a second consolidation --------------------
            overwrite_dir = tmp_path / "dump-twice"
            proc = run([str(executable), str(overwrite_dir), "dump-twice"])
            check(
                "SECOND-DUMP-REFUSED" in proc.stdout,
                "second consolidation did not refuse to overwrite",
            )
            print("  ok: consolidation refuses to overwrite existing output")

    except TestFailure as exc:
        print(f"  FAIL: {exc}")
        ok = False
    except OSError as exc:
        print(f"  FAIL: {exc}")
        ok = False

    print("WRITE-THROUGH SELF-TEST PASSED" if ok else "WRITE-THROUGH SELF-TEST FAILED")
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
