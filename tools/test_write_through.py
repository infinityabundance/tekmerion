#!/usr/bin/env python3
"""Offline proof for exptrace_failure.ml: write-through, retention, attempt stream.

HOL Light is not built here. The real module source is compiled against tiny
stubs for the four HOL Light names it uses (`goal`, `pp_print_goal`, `term`,
`string_of_term`). The harness then:

  1. records one failing tactic application and one failing conversion and
     calls *nothing* else; asserts both journals already exist with one valid
     JSON object each, the new digest/size fields, and the expected scopes;
  2. exercises the optional consolidation dump, its pretty array output, and
     its refusal to overwrite;
  3. checks the two failure-directory fallbacks;
  4. runs one bounded attempt: 100 failures for one tactic (first 32 + last 32
     retained, 36 counted and dropped), a success, a conversion success and
     failure, and an over-cap term; asserts the stream is strictly ordered by
     `seq`, ends with `attempt_end`, and that the manifest states totals,
     retained, dropped, caps and digests, and never claims completeness;
  5. checks SHA-256 against hashlib and the JSON encoder against json.loads,
     including control characters.

Usage:
    python3 tools/test_write_through.py
"""

from __future__ import annotations

import hashlib
import json
import os
import re
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
    "goal_sha256",
    "goal_bytes",
    "exception_text",
    "exception_text_bytes",
    "exception_truncated",
    "subgoal_index",
    "elapsed_cpu_s",
}

EXPECTED_CONV_FIELDS = {
    "scope",
    "conv",
    "input",
    "input_sha256",
    "input_bytes",
    "exception_text",
    "exception_text_bytes",
    "exception_truncated",
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
    prerr_endline "usage: write_through_test OUTDIR [no-dump|dump|dump-twice|env|default|attempt|sha|json|conv-lazy]";
    exit 2
  end;
  let dir = Sys.argv.(1) in
  let mode =
    if Array.length Sys.argv >= 3 then Sys.argv.(2) else "no-dump"
  in
  (match mode with
   | "env" | "default" | "sha" | "json" | "conv-lazy" -> ()
   | "attempt" ->
     (try Sys.mkdir dir 0o777 with Sys_error _ -> ());
     exptrace_failures_set_dir dir;
     exptrace_attempt_set_dir (dir ^ "/attempts")
   | _ -> exptrace_failures_set_dir dir);
  match mode with
  | "sha" ->
    let samples = [""; "abc"; "a\x01b\"c\\d"; "the quick brown fox"] in
    List.iter
      (fun s -> Printf.printf "%s %s\n" (exptrace_sha256_hex s)
          (String.escaped s))
      samples
  | "json" ->
    let samples = ["plain"; "a\x01b\"c\\d"; "\n\t\0007"; "sl/ash"] in
    List.iter
      (fun s -> Printf.printf "{\"s\":\"%s\"}\n" (exptrace_json_escape s))
      samples
  | "conv-lazy" ->
    (* No active attempt: the input thunk must not be forced on success. *)
    let forced = ref false in
    let value =
      exptrace_guard_conv "BETA_CONV"
        (fun () -> forced := true; "x")
        (fun () -> "ok")
    in
    Printf.printf "CONV_LAZY forced=%b value=%s\n" !forced value
  | "attempt" ->
    exptrace_attempt_start "ATTEMPTID123" "thm.word_max" "proofsha" ""
      "wrappersha" "modulesha";
    let goal : goal = "|- x = x" in
    (* In-order success. *)
    ignore (exptrace_guard "REFL_TAC" goal (fun () -> ()));
    (* A name containing a raw control character, quote and backslash. *)
    ignore (exptrace_guard "CTRL\x01\"\\TAC" goal (fun () -> ()));
    (* 100 failures with a control character in the message. *)
    for _ = 1 to 100 do
      (try
         ignore
           (exptrace_guard "RETENTION_TAC" goal
              (fun () -> failwith ("boom\x01\"\\tail")))
       with Failure _ -> ())
    done;
    (* One over-cap term: streamed, but its full text must be dropped. *)
    let big : goal = String.make 20000 'x' in
    (try ignore (exptrace_guard "BIG_TAC" big (fun () -> failwith "big"))
     with Failure _ -> ());
    (* Conversion failure and success; the success forces the thunk only
       because an attempt is active. *)
    let forced = ref false in
    (try
       ignore
         (exptrace_guard_conv "BETA_CONV" (fun () -> "x")
            (fun () -> failwith "conv-boom"))
     with Failure _ -> ());
    ignore
      (exptrace_guard_conv "ETA_CONV"
         (fun () -> forced := true; "y")
         (fun () -> "ok"));
    exptrace_attempt_end "proved";
    Printf.printf "ATTEMPT-OK forced=%b\n" !forced
  | "no-dump" ->
    let goal : goal = "|- x = x" in
    (try
       ignore (exptrace_guard "REWRITE_TAC" goal (fun () -> failwith "boom"))
     with Failure _ -> ());
    (try
       ignore
         (exptrace_guard_conv "BETA_CONV" (fun () -> "x")
            (fun () -> failwith "conv-boom"))
     with Failure _ -> ())
  | "dump" | "dump-twice" ->
    let goal : goal = "|- x = x" in
    (try
       ignore (exptrace_guard "REWRITE_TAC" goal (fun () -> failwith "boom"))
     with Failure _ -> ());
    (try
       ignore
         (exptrace_guard_conv "BETA_CONV" (fun () -> "x")
            (fun () -> failwith "conv-boom"))
     with Failure _ -> ());
    if mode = "dump" then begin
      exptrace_failures_dump dir;
      print_endline "DUMP-OK"
    end else begin
      exptrace_failures_dump dir;
      (try
         exptrace_failures_dump dir;
         print_endline "SECOND-DUMP-ACCEPTED";
         exit 3
       with Invalid_argument _ ->
         print_endline "SECOND-DUMP-REFUSED")
    end
  | "env" | "default" ->
    let goal : goal = "|- x = x" in
    (try
       ignore (exptrace_guard "REWRITE_TAC" goal (fun () -> failwith "boom"))
     with Failure _ -> ());
    (try
       ignore
         (exptrace_guard_conv "BETA_CONV" (fun () -> "x")
            (fun () -> failwith "conv-boom"))
     with Failure _ -> ())
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
        record["goal_sha256"] == hashlib.sha256(b"|- x = x").hexdigest(),
        f"{where}: goal_sha256 does not match the serialized goal",
    )
    check(record["goal_bytes"] == 8, f"{where}: wrong goal_bytes")
    check(
        "Failure" in record["exception_text"]
        and "boom" in record["exception_text"],
        f"{where}: wrong exception_text {record['exception_text']!r}",
    )
    check(record["subgoal_index"] is None, f"{where}: subgoal_index is not null")
    check(record["exception_truncated"] is False, f"{where}: truncated flag")
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
    check(record["input"] == "x", f"{where}: wrong input {record['input']!r}")
    check(
        record["input_sha256"] == hashlib.sha256(b"x").hexdigest(),
        f"{where}: input_sha256 does not match the input",
    )
    check(record["input_bytes"] == 1, f"{where}: wrong input_bytes")
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


def check_sha(driver: Path, env: dict[str, str]) -> None:
    samples = ["", "abc", "a\x01b\"c\\d", "the quick brown fox"]
    proc = run([str(driver), "unused", "sha"], env=env)
    lines = proc.stdout.splitlines()
    check(len(lines) == len(samples), "sha mode printed the wrong line count")
    for sample, line in zip(samples, lines):
        digest, _escaped = line.split(" ", 1)
        check(
            digest == hashlib.sha256(sample.encode("utf-8")).hexdigest(),
            f"sha256 mismatch for {sample!r}: {digest}",
        )
    print("  ok: SHA-256 matches hashlib on four samples")


def check_json(driver: Path, env: dict[str, str]) -> None:
    samples = ["plain", "a\x01b\"c\\d", "\n\t\0007", "sl/ash"]
    proc = run([str(driver), "unused", "json"], env=env)
    lines = proc.stdout.splitlines()
    check(len(lines) == len(samples), "json mode printed the wrong line count")
    for sample, line in zip(samples, lines):
        check("\\u0001" in line or "\x01" not in sample,
              f"control character not escaped as \\u0001: {line!r}")
        check("\\001" not in line, f"decimal escape emitted: {line!r}")
        parsed = json.loads(line)
        check(parsed["s"] == sample,
              f"JSON round-trip changed {sample!r}: {parsed['s']!r}")
    print("  ok: JSON encoder escapes control characters and round-trips")


def check_conv_lazy(driver: Path, env: dict[str, str]) -> None:
    proc = run([str(driver), "unused", "conv-lazy"], env=env)
    check("CONV_LAZY forced=false value=ok" in proc.stdout,
          f"conversion input thunk was forced on success: {proc.stdout!r}")
    print("  ok: exptrace_guard_conv does not force the input thunk on success")


def check_attempt(driver: Path, tmp_path: Path, env: dict[str, str]) -> None:
    outdir = tmp_path / "attempt"
    proc = run([str(driver), str(outdir), "attempt"], env=env)
    check("ATTEMPT-OK forced=true" in proc.stdout,
          f"attempt run did not finish as expected: {proc.stdout!r}")

    attempt_dir = outdir / "attempts"
    stream_path = attempt_dir / "ATTEMPTID123.jsonl"
    manifest_path = attempt_dir / "ATTEMPTID123.manifest.json"
    check(stream_path.is_file(), "attempt stream was not written")
    check(manifest_path.is_file(), "attempt manifest was not written")

    raw_lines = stream_path.read_text(encoding="utf-8").splitlines()
    records = [json.loads(line) for line in raw_lines]
    check(len(records) >= 3, "attempt stream is too short")
    check(records[0]["kind"] == "attempt_start", "stream does not start with attempt_start")
    check(records[-1]["kind"] == "attempt_end", "stream does not end with attempt_end")
    seqs = [r["seq"] for r in records]
    check(seqs == sorted(set(seqs)) and len(seqs) == len(set(seqs)),
          f"seq is not strictly increasing: {seqs}")

    kinds = [r["kind"] for r in records]
    for required in ("tactic_ok", "tactic_raised", "conv_ok", "conv_raised"):
        check(required in kinds, f"stream has no {required} event")

    for record in records:
        for field in ("seq", "kind", "name", "scope", "goal_sha256",
                      "goal_bytes", "elapsed_cpu_s"):
            check(field in record, f"event is missing {field}: {record}")
        if record["kind"] in ("tactic_ok", "tactic_raised", "conv_ok", "conv_raised"):
            check(re.fullmatch(r"[0-9a-f]{64}", record["goal_sha256"]) is not None,
                  f"bad digest: {record['goal_sha256']!r}")

    end = records[-1]
    check(end["outcome"] == "proved", f"wrong outcome: {end['outcome']!r}")
    check(end["events"] == 105, f"wrong total event count: {end['events']}")
    check(end["tactic_events"] == 103, "wrong tactic event count")
    check(end["conv_events"] == 2, "wrong conv event count")
    check(end["tactic_ok"] == 2, "wrong tactic_ok count")
    check(end["tactic_raised"] == 101, "wrong tactic_raised count")
    check(end["conv_ok"] == 1, "wrong conv_ok count")
    check(end["conv_raised"] == 1, "wrong conv_raised count")
    check(end["streamed_events"] == 69, f"wrong streamed count: {end['streamed_events']}")
    check(end["dropped_events"] == 36, f"wrong dropped count: {end['dropped_events']}")
    check(end["stream_complete"] is False, "end claims the stream is complete")

    ctrl = [r for r in records if "\x01" in r.get("name", "")]
    check(len(ctrl) == 1, "control-character name did not survive the stream")
    check(ctrl[0]["name"] == "CTRL\x01\"\\TAC",
          f"control-character name round-tripped wrongly: {ctrl[0]['name']!r}")

    retained = [r for r in records if r.get("name") == "RETENTION_TAC"]
    check(len(retained) == 64, f"RETENTION_TAC retained {len(retained)}, expected 64")
    hidden = [r for r in retained if r["kind"] == "tactic_raised"]
    check(len(hidden) == 64, "retained RETENTION_TAC events are not failures")
    check(any("\\u0001" in line for line in raw_lines),
          "no event carried the escaped control character")

    big = [r for r in records if r.get("name") == "BIG_TAC"]
    check(len(big) == 1, "BIG_TAC was not streamed")
    check(big[0]["term_retained"] is False, "over-cap term was retained in the stream")
    check(big[0]["goal_bytes"] == 20000, "over-cap term byte size is wrong")

    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    check(manifest["attempt_id"] == "ATTEMPTID123", "manifest has the wrong attempt id")
    check(manifest["outcome"] == "proved", "manifest has the wrong outcome")
    check(manifest["retention"]["first"] == 32, "manifest first cap is wrong")
    check(manifest["retention"]["last"] == 32, "manifest last cap is wrong")
    check(manifest["retention"]["stream_complete"] is False,
          "manifest claims the stream is complete")
    check(manifest["channels"]["tactic"]["total_events"] == 103,
          "manifest tactic total is wrong")
    check(manifest["channels"]["tactic"]["streamed"] == 67,
          "manifest tactic streamed is wrong")
    check(manifest["channels"]["tactic"]["dropped"] == 36,
          "manifest tactic dropped is wrong")
    check(manifest["channels"]["conv"]["total_events"] == 2,
          "manifest conv total is wrong")
    check(manifest["channels"]["conv"]["streamed"] == 2,
          "manifest conv streamed is wrong")
    check(manifest["channels"]["tactic"]["cap"]["max_term_bytes"] == 16384,
          "manifest max_term_bytes is wrong")
    check(len(manifest["channels"]["tactic"]["digest"]) == 64,
          "manifest tactic digest is not a sha256")
    wrappers = {w["name"]: w for w in manifest["wrappers"]}
    check(wrappers["RETENTION_TAC"]["total"] == 100, "wrapper total is wrong")
    check(wrappers["RETENTION_TAC"]["dropped"] == 36, "wrapper dropped is wrong")
    check(wrappers["BIG_TAC"]["retained_terms"] == 0,
          "over-cap term counted as retained")
    check(wrappers["BIG_TAC"]["dropped_terms"] == 1,
          "over-cap term not counted as a dropped term")
    check(manifest["declared_budget_bytes"] == 128 * 1024 * 1024,
          "declared budget is wrong")

    journal = outdir / "RETENTION_TAC.failures.jsonl"
    check(journal.is_file(), "retention tactic journal was not written")
    journal_lines = journal.read_text(encoding="utf-8").splitlines()
    check(len(journal_lines) == 64,
          f"retention journal has {len(journal_lines)} lines, expected 64")
    for line in journal_lines:
        json.loads(line)

    big_journal = outdir / "BIG_TAC.failures.jsonl"
    big_records = [json.loads(line) for line in
                   big_journal.read_text(encoding="utf-8").splitlines()]
    check(big_records[0]["goal_before"] is None,
          "over-cap journal record kept the full goal_before")
    check(big_records[0]["goal_bytes"] == 20000,
          "over-cap journal record has the wrong byte size")

    print(
        "  ok: attempt stream is ordered and bounded; manifest states totals, "
        "retained=68, dropped=36 and stream_complete=false"
    )


def main() -> int:
    if not MODULE.is_file():
        print(f"FAIL: missing {MODULE}", file=sys.stderr)
        return 1

    source = STUBS + "\n" + MODULE.read_text(encoding="utf-8") + "\n" + DRIVER
    ok = True

    base_env = os.environ.copy()
    base_env.pop("TEKMERION_FAILURE_DIR", None)
    base_env.pop("TEKMERION_ATTEMPT_DIR", None)

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

            check_sha(executable, base_env)
            check_json(executable, base_env)
            check_conv_lazy(executable, base_env)

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

            # --- bounded attempt stream ----------------------------------
            check_attempt(executable, tmp_path, base_env)

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
