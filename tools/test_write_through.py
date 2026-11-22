#!/usr/bin/env python3
"""Offline proof that exptrace_failure.ml is write-through and standalone.

HOL Light is not built here. The real module source is compiled against tiny
stubs for the four HOL Light names it uses (`goal`, `pp_print_goal`, `term`,
`string_of_term`). The harness then:

  1. records one failing tactic application and one failing conversion and
     calls *nothing* else;
  2. asserts the tactic journal (`<tac>.failures.jsonl`) and the conversion
     journal (`<conv>.conv.failures.jsonl`) already exist and contain exactly
     one valid JSON object each, with the expected fields and scopes;
  3. exercises the optional consolidation dump with its explicit directory,
     its pretty array output, and its refusal to overwrite existing output;
  4. checks the two directory fallbacks: TEKMERION_FAILURE_DIR and
     ./tekmerion-failures.

Usage:
    python3 tools/test_write_through.py
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MODULE = ROOT / "trace" / "exptrace_failure.ml"

EXPECTED_TACTIC_FIELDS = {
    "scope",
    "tactic",
    "goal_before",
    "exception_text",
    "subgoal_index",
    "elapsed_cpu_s",
}

EXPECTED_CONV_FIELDS = {
    "scope",
    "conv",
    "input",
    "exception_text",
    "elapsed_cpu_s",
}

STUBS = r"""
(* Minimal stubs: just enough to compile exptrace_failure.ml outside HOL Light. *)
type goal = string
type term = string

let pp_print_goal fmt (g : goal) = Format.pp_print_string fmt g
let string_of_term (t : term) = t

let unset_jrh_lexer = ();;
let set_jrh_lexer = ();;
"""

DRIVER = r"""
let () =
  if Array.length Sys.argv < 2 then begin
    prerr_endline "usage: write_through_test OUTDIR [no-dump|dump|dump-twice|env|default]";
    exit 2
  end;
  let dir = Sys.argv.(1) in
  let mode =
    if Array.length Sys.argv >= 3 then Sys.argv.(2) else "no-dump"
  in
  (match mode with
   | "env" | "default" -> ()
   | _ -> exptrace_failures_set_dir dir);
  let goal : goal = "|- x = x" in
  (try
     ignore (exptrace_guard "REWRITE_TAC" goal (fun () -> failwith "boom"))
   with Failure _ -> ());
  (try
     ignore (exptrace_guard_conv "BETA_CONV" "x" (fun () -> failwith "conv-boom"))
   with Failure _ -> ());
  match mode with
  | "no-dump" ->
    (* Deliberately call nothing further. This is the simulated process exit:
       the records must already be durable. *)
    ()
  | "env" | "default" ->
    ()
  | "dump" ->
    exptrace_failures_dump dir;
    print_endline "DUMP-OK"
  | "dump-twice" ->
    exptrace_failures_dump dir;
    (try
       exptrace_failures_dump dir;
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


def run(command: list[str], cwd: Path | None = None,
        env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    proc = subprocess.run(command, capture_output=True, text=True, cwd=cwd,
                          env=env)
    if proc.returncode != 0:
        raise TestFailure(
            f"command failed ({proc.returncode}): {' '.join(command)}\n"
            f"stdout:\n{proc.stdout}\nstderr:\n{proc.stderr}"
        )
    return proc


def check_tactic_record(record: object, where: str) -> dict:
    check(isinstance(record, dict), f"{where}: record is not a JSON object")
    check(
        set(record) == EXPECTED_TACTIC_FIELDS,
        f"{where}: fields are {sorted(record)}, expected "
        f"{sorted(EXPECTED_TACTIC_FIELDS)}",
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


def check_conv_record(record: object, where: str) -> dict:
    check(isinstance(record, dict), f"{where}: record is not a JSON object")
    check(
        set(record) == EXPECTED_CONV_FIELDS,
        f"{where}: fields are {sorted(record)}, expected "
        f"{sorted(EXPECTED_CONV_FIELDS)}",
    )
    check(record["scope"] == "conv", f"{where}: scope is {record['scope']!r}")
    check(record["conv"] == "BETA_CONV", f"{where}: wrong conv")
    check(record["input"] == "`x`", f"{where}: wrong input {record['input']!r}")
    check(
        "Failure" in record["exception_text"]
        and "conv-boom" in record["exception_text"],
        f"{where}: wrong exception_text {record['exception_text']!r}",
    )
    check(
        isinstance(record["elapsed_cpu_s"], (int, float))
        and not isinstance(record["elapsed_cpu_s"], bool)
        and record["elapsed_cpu_s"] >= 0,
        f"{where}: elapsed_cpu_s is not a nonnegative number",
    )
    return record


def check_journals(outdir: Path, where: str) -> tuple[dict, dict]:
    tactic_journal = outdir / "REWRITE_TAC.failures.jsonl"
    conv_journal = outdir / "BETA_CONV.conv.failures.jsonl"
    check(tactic_journal.is_file(), f"{where}: tactic journal missing: {tactic_journal}")
    check(conv_journal.is_file(), f"{where}: conversion journal missing: {conv_journal}")

    for path in (tactic_journal, conv_journal):
        raw = path.read_text(encoding="utf-8")
        lines = raw.splitlines()
        check(
            len(lines) == 1,
            f"{where}: {path.name} should contain exactly one line, found {len(lines)}",
        )
        check(raw.endswith("\n"), f"{where}: {path.name} is not newline-terminated")

    tactic = check_tactic_record(
        json.loads(tactic_journal.read_text(encoding="utf-8")), f"{where} tactic"
    )
    conv = check_conv_record(
        json.loads(conv_journal.read_text(encoding="utf-8")), f"{where} conversion"
    )
    return tactic, conv


def main() -> int:
    if not MODULE.is_file():
        print(f"FAIL: missing {MODULE}", file=sys.stderr)
        return 1

    source = STUBS + "\n" + MODULE.read_text(encoding="utf-8") + "\n" + DRIVER
    ok = True

    base_env = os.environ.copy()
    base_env.pop("TEKMERION_FAILURE_DIR", None)

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
            run([str(executable), str(outdir), "no-dump"], env=base_env)
            tactic, conv = check_journals(outdir, "no-dump")
            check(
                not (outdir / "REWRITE_TAC.failures.json").exists(),
                "consolidated tactic .json exists even though dump was never called",
            )
            check(
                not (outdir / "BETA_CONV.conv.failures.json").exists(),
                "consolidated conv .json exists even though dump was never called",
            )
            print(
                "  ok: write-through journals contain one valid JSON line each "
                "before any dump"
            )

            # --- optional consolidation -----------------------------------
            dumpdir = tmp_path / "dump"
            proc = run([str(executable), str(dumpdir), "dump"], env=base_env)
            check("DUMP-OK" in proc.stdout, "dump mode did not complete")
            tactic_json = dumpdir / "REWRITE_TAC.failures.json"
            conv_json = dumpdir / "BETA_CONV.conv.failures.json"
            check(tactic_json.is_file(), "consolidated tactic .json was not written")
            check(conv_json.is_file(), "consolidated conv .json was not written")
            check(
                (dumpdir / "REWRITE_TAC.failures.jsonl").is_file(),
                "consolidation removed the raw tactic journal",
            )
            check(
                (dumpdir / "BETA_CONV.conv.failures.jsonl").is_file(),
                "consolidation removed the raw conversion journal",
            )
            tactic_data = json.loads(tactic_json.read_text(encoding="utf-8"))
            conv_data = json.loads(conv_json.read_text(encoding="utf-8"))
            check(isinstance(tactic_data, list), "tactic consolidation is not an array")
            check(isinstance(conv_data, list), "conv consolidation is not an array")
            check(len(tactic_data) == 1, f"tactic array has {len(tactic_data)} records")
            check(len(conv_data) == 1, f"conv array has {len(conv_data)} records")
            check_tactic_record(tactic_data[0], "consolidated tactic record 1")
            check_conv_record(conv_data[0], "consolidated conv record 1")
            pretty = tactic_json.read_text(encoding="utf-8")
            check(
                '  {\n    "scope": "tactic",\n' in pretty,
                "consolidated output is not in the success-dump field style",
            )
            check(
                tactic_data[0]["tactic"] == tactic["tactic"]
                and tactic_data[0]["goal_before"] == tactic["goal_before"]
                and tactic_data[0]["exception_text"] == tactic["exception_text"],
                "consolidated tactic record does not match the journal record",
            )
            check(
                conv_data[0]["conv"] == conv["conv"]
                and conv_data[0]["input"] == conv["input"]
                and conv_data[0]["exception_text"] == conv["exception_text"],
                "consolidated conv record does not match the journal record",
            )
            print("  ok: optional consolidation produces pretty JSON arrays")

            # --- fail closed on a second consolidation --------------------
            overwrite_dir = tmp_path / "dump-twice"
            proc = run([str(executable), str(overwrite_dir), "dump-twice"], env=base_env)
            check(
                "SECOND-DUMP-REFUSED" in proc.stdout,
                "second consolidation did not refuse to overwrite",
            )
            print("  ok: consolidation refuses to overwrite existing output")

            # --- TEKMERION_FAILURE_DIR fallback ---------------------------
            envdir = tmp_path / "from-env"
            env = base_env.copy()
            env["TEKMERION_FAILURE_DIR"] = str(envdir)
            run([str(executable), str(tmp_path / "unused"), "env"], env=env)
            check_journals(envdir, "env fallback")
            print("  ok: TEKMERION_FAILURE_DIR selects the journal directory")

            # --- ./tekmerion-failures default ----------------------------
            defaultcwd = tmp_path / "default-cwd"
            defaultcwd.mkdir()
            run([str(executable), str(tmp_path / "unused"), "default"],
                cwd=defaultcwd, env=base_env)
            check_journals(defaultcwd / "tekmerion-failures", "default fallback")
            print("  ok: unset directory falls back to ./tekmerion-failures")

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
