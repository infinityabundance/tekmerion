//! Independent oracle for `trace/exptrace_failure.ml`.
//!
//! Port of the Python `tools/test_write_through.py` harness. HOL Light is not
//! built here: the real module source is compiled against tiny stubs for the
//! four HOL Light names it uses (`goal`, `pp_print_goal`, `term`,
//! `string_of_term`). The test then:
//!
//! 1. records one failing tactic application and one failing conversion and
//!    calls *nothing* else; asserts both journals already exist with one valid
//!    JSON object each, the new digest/size fields, and the expected scopes;
//! 2. exercises the optional consolidation dump, its pretty array output, and
//!    its refusal to overwrite;
//! 3. checks the two failure-directory fallbacks;
//! 4. runs one bounded attempt and asserts the stream is strictly ordered and
//!    bounded and that the manifest states totals, retained, dropped, caps and
//!    digests, and never claims completeness;
//! 5. checks the module's SHA-256 and JSON encoder against independent Rust
//!    implementations.
//!
//! The SHA-256 check is deliberately independent of the module under test: it
//! compares the module's `exptrace_sha256_hex` output with the crate's own
//! hasher on the same bytes. The JSON check parses the module's escape output
//! with the crate's strict JSON reader and round-trips the value.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use tekmerion::tools::challenge::sha256_hex;
use tekmerion::tools::json::{self, Json};

static SEQ: AtomicU32 = AtomicU32::new(0);

const STUBS: &str = r#"
(* Minimal stubs: just enough to compile exptrace_failure.ml outside HOL Light. *)
type goal = string
type term = string

let pp_print_goal fmt (g : goal) = Format.pp_print_string fmt g
let string_of_term (t : term) = t

let unset_jrh_lexer = ();;
let set_jrh_lexer = ();;
"#;

const DRIVER: &str = r#"
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
"#;

const TACTIC_FIELDS: &[&str] = &[
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
];

const CONV_FIELDS: &[&str] = &[
    "scope",
    "conv",
    "input",
    "input_sha256",
    "input_bytes",
    "exception_text",
    "exception_text_bytes",
    "exception_truncated",
    "elapsed_cpu_s",
];

struct Tmp(PathBuf);

impl Tmp {
    fn new(tag: &str) -> Tmp {
        let n = SEQ.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!(
            "tek-write-through-{}-{}-{}",
            std::process::id(),
            tag,
            n
        ));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        Tmp(p)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

// --- JSON helpers ----------------------------------------------------------

fn at<'a>(value: &'a Json, path: &[&str]) -> &'a Json {
    let mut current = value;
    for key in path {
        current = current
            .get(key)
            .unwrap_or_else(|| panic!("missing JSON key {key:?} in {path:?}"));
    }
    current
}

fn int_at(value: &Json, path: &[&str]) -> i64 {
    match at(value, path) {
        Json::Num(n) => *n as i64,
        other => panic!("{path:?} is not a number: {other:?}"),
    }
}

fn num_at(value: &Json, path: &[&str]) -> f64 {
    match at(value, path) {
        Json::Num(n) => *n,
        other => panic!("{path:?} is not a number: {other:?}"),
    }
}

fn str_at<'a>(value: &'a Json, path: &[&str]) -> &'a str {
    at(value, path)
        .as_str()
        .unwrap_or_else(|| panic!("{path:?} is not a string"))
}

fn bool_at(value: &Json, path: &[&str]) -> bool {
    match at(value, path) {
        Json::Bool(b) => *b,
        other => panic!("{path:?} is not a bool: {other:?}"),
    }
}

fn is_null(value: &Json, path: &[&str]) -> bool {
    matches!(at(value, path), Json::Null)
}

fn parse_json(bytes: &[u8], what: &str) -> Json {
    json::parse(bytes).unwrap_or_else(|e| panic!("{what}: invalid JSON: {e}"))
}

fn read_json(path: &Path) -> Json {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    parse_json(&bytes, &path.display().to_string())
}

fn sorted_keys(value: &Json) -> Vec<String> {
    value
        .as_obj()
        .unwrap_or_else(|| panic!("not an object: {value:?}"))
        .keys()
        .cloned()
        .collect()
}

fn wrapper_named<'a>(wrappers: &'a [Json], want: &str) -> &'a Json {
    wrappers
        .iter()
        .find(|w| w.get("name").and_then(|v| v.as_str()) == Some(want))
        .unwrap_or_else(|| panic!("no wrapper {want}"))
}

// --- process helpers -------------------------------------------------------

fn base_command(program: &Path) -> Command {
    let mut cmd = Command::new(program);
    cmd.env_remove("TEKMERION_FAILURE_DIR");
    cmd.env_remove("TEKMERION_ATTEMPT_DIR");
    cmd
}

fn run_ok(program: &Path, args: &[&str], cwd: Option<&Path>, failure_dir: Option<&Path>) -> String {
    let mut cmd = base_command(program);
    cmd.args(args);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    if let Some(dir) = failure_dir {
        cmd.env("TEKMERION_FAILURE_DIR", dir);
    }
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("could not run {}: {e}", program.display()));
    assert!(
        out.status.success(),
        "command failed ({}): {:?}\nstdout:\n{}\nstderr:\n{}",
        out.status,
        args,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

// --- record checks ---------------------------------------------------------

fn check_tactic_record(record: &Json, where_: &str) {
    let mut keys = sorted_keys(record);
    let mut expected: Vec<String> = TACTIC_FIELDS.iter().map(|s| s.to_string()).collect();
    expected.sort();
    keys.sort();
    assert_eq!(keys, expected, "{where_}: wrong field set");

    assert_eq!(str_at(record, &["scope"]), "tactic", "{where_}");
    assert_eq!(str_at(record, &["tactic"]), "REWRITE_TAC", "{where_}");
    assert_eq!(str_at(record, &["goal_before"]), "|- x = x", "{where_}");
    assert_eq!(
        str_at(record, &["goal_sha256"]),
        sha256_hex(b"|- x = x"),
        "{where_}: goal_sha256 does not match the serialized goal"
    );
    assert_eq!(int_at(record, &["goal_bytes"]), 8, "{where_}");
    let exception = str_at(record, &["exception_text"]);
    assert!(
        exception.contains("Failure") && exception.contains("boom"),
        "{where_}: wrong exception_text {exception:?}"
    );
    assert!(is_null(record, &["subgoal_index"]), "{where_}");
    assert!(!bool_at(record, &["exception_truncated"]), "{where_}");
    let elapsed = num_at(record, &["elapsed_cpu_s"]);
    assert!(
        elapsed >= 0.0,
        "{where_}: elapsed_cpu_s is not a nonnegative number"
    );
}

fn check_conv_record(record: &Json, where_: &str) {
    let mut keys = sorted_keys(record);
    let mut expected: Vec<String> = CONV_FIELDS.iter().map(|s| s.to_string()).collect();
    expected.sort();
    keys.sort();
    assert_eq!(keys, expected, "{where_}: wrong field set");

    assert_eq!(str_at(record, &["scope"]), "conv", "{where_}");
    assert_eq!(str_at(record, &["conv"]), "BETA_CONV", "{where_}");
    assert_eq!(str_at(record, &["input"]), "x", "{where_}");
    assert_eq!(
        str_at(record, &["input_sha256"]),
        sha256_hex(b"x"),
        "{where_}: input_sha256 does not match the input"
    );
    assert_eq!(int_at(record, &["input_bytes"]), 1, "{where_}");
    let exception = str_at(record, &["exception_text"]);
    assert!(
        exception.contains("Failure") && exception.contains("conv-boom"),
        "{where_}: wrong exception_text {exception:?}"
    );
    assert!(num_at(record, &["elapsed_cpu_s"]) >= 0.0, "{where_}");
}

fn check_journals(outdir: &Path, where_: &str) -> (Json, Json) {
    let tactic_journal = outdir.join("REWRITE_TAC.failures.jsonl");
    let conv_journal = outdir.join("BETA_CONV.conv.failures.jsonl");
    assert!(
        tactic_journal.is_file(),
        "{where_}: tactic journal missing: {}",
        tactic_journal.display()
    );
    assert!(
        conv_journal.is_file(),
        "{where_}: conversion journal missing: {}",
        conv_journal.display()
    );

    for path in [&tactic_journal, &conv_journal] {
        let raw = fs::read_to_string(path).unwrap();
        assert_eq!(
            raw.lines().count(),
            1,
            "{where_}: {} should contain exactly one line",
            path.display()
        );
        assert!(
            raw.ends_with('\n'),
            "{where_}: {} is not newline-terminated",
            path.display()
        );
    }

    let tactic = read_json(&tactic_journal);
    let conv = read_json(&conv_journal);
    check_tactic_record(&tactic, &format!("{where_}/tactic"));
    check_conv_record(&conv, &format!("{where_}/conv"));
    (tactic, conv)
}

// --- the harness -----------------------------------------------------------

#[test]
fn exptrace_failure_write_through_and_attempt_stream() {
    if Command::new("ocamlc")
        .arg("-version")
        .output()
        .map(|o| !o.status.success())
        .unwrap_or(true)
    {
        eprintln!("skipping: ocamlc is not available");
        return;
    }

    let module =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("trace/exptrace_failure.ml"))
            .expect("trace/exptrace_failure.ml");
    let source = format!("{STUBS}\n{module}\n{DRIVER}");

    let tmp = Tmp::new("harness");
    let source_path = tmp.path().join("write_through_test.ml");
    let executable = tmp.path().join("write_through_test");
    fs::write(&source_path, source).unwrap();

    let compile = Command::new("ocamlc")
        .args(["-w", "-a", "-o"])
        .arg(&executable)
        .arg(&source_path)
        .output()
        .expect("run ocamlc");
    assert!(
        compile.status.success(),
        "could not compile exptrace_failure.ml against stubs\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&compile.stdout),
        String::from_utf8_lossy(&compile.stderr)
    );

    // --- SHA-256 and JSON encoders, against independent implementations ----
    let samples = ["", "abc", "a\u{1}b\"c\\d", "the quick brown fox"];
    let stdout = run_ok(&executable, &["unused", "sha"], None, None);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines.len(),
        samples.len(),
        "sha mode printed the wrong line count"
    );
    for (sample, line) in samples.iter().zip(lines.iter()) {
        let digest = line.split_once(' ').map(|(d, _)| d).unwrap_or(line);
        assert_eq!(
            digest,
            sha256_hex(sample.as_bytes()),
            "sha256 mismatch for {sample:?}"
        );
    }

    let json_samples = ["plain", "a\u{1}b\"c\\d", "\n\t\u{0}7", "sl/ash"];
    let stdout = run_ok(&executable, &["unused", "json"], None, None);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines.len(),
        json_samples.len(),
        "json mode printed the wrong line count"
    );
    for (sample, line) in json_samples.iter().zip(lines.iter()) {
        if sample.contains('\u{1}') {
            assert!(
                line.contains("\\u0001"),
                "control character not escaped: {line:?}"
            );
        }
        assert!(!line.contains("\\001"), "decimal escape emitted: {line:?}");
        let parsed = parse_json(line.as_bytes(), "json mode line");
        assert_eq!(
            parsed.get("s").and_then(|v| v.as_str()),
            Some(*sample),
            "JSON round-trip changed {sample:?}"
        );
    }

    let stdout = run_ok(&executable, &["unused", "conv-lazy"], None, None);
    assert!(
        stdout.contains("CONV_LAZY forced=false value=ok"),
        "conversion input thunk was forced on success: {stdout:?}"
    );

    // --- write-through: record, then call nothing --------------------------
    let outdir = tmp.path().join("no-dump");
    run_ok(
        &executable,
        &[&outdir.display().to_string(), "no-dump"],
        None,
        None,
    );
    check_journals(&outdir, "no-dump");
    assert!(!outdir.join("REWRITE_TAC.failures.json").exists());
    assert!(!outdir.join("BETA_CONV.conv.failures.json").exists());

    // --- optional consolidation -------------------------------------------
    let dumpdir = tmp.path().join("dump");
    let stdout = run_ok(
        &executable,
        &[&dumpdir.display().to_string(), "dump"],
        None,
        None,
    );
    assert!(stdout.contains("DUMP-OK"), "dump mode did not complete");
    let tactic_json = dumpdir.join("REWRITE_TAC.failures.json");
    let conv_json = dumpdir.join("BETA_CONV.conv.failures.json");
    assert!(tactic_json.is_file());
    assert!(conv_json.is_file());
    assert!(dumpdir.join("REWRITE_TAC.failures.jsonl").is_file());
    assert!(dumpdir.join("BETA_CONV.conv.failures.jsonl").is_file());

    let tactic_data = read_json(&tactic_json);
    let conv_data = read_json(&conv_json);
    let tactic_array = tactic_data.as_array().expect("tactic array");
    let conv_array = conv_data.as_array().expect("conv array");
    assert_eq!(tactic_array.len(), 1);
    assert_eq!(conv_array.len(), 1);
    check_tactic_record(&tactic_array[0], "consolidated tactic");
    check_conv_record(&conv_array[0], "consolidated conv");

    let pretty = fs::read_to_string(&tactic_json).unwrap();
    assert!(
        pretty.contains("  {\n    \"scope\": \"tactic\",\n"),
        "consolidated output is not in the success-dump field style"
    );

    let (journal_tactic, journal_conv) = check_journals(&dumpdir, "dump");
    assert_eq!(
        str_at(&tactic_array[0], &["tactic"]),
        str_at(&journal_tactic, &["tactic"])
    );
    assert_eq!(
        str_at(&tactic_array[0], &["goal_before"]),
        str_at(&journal_tactic, &["goal_before"])
    );
    assert_eq!(
        str_at(&tactic_array[0], &["exception_text"]),
        str_at(&journal_tactic, &["exception_text"])
    );
    assert_eq!(
        str_at(&conv_array[0], &["conv"]),
        str_at(&journal_conv, &["conv"])
    );
    assert_eq!(
        str_at(&conv_array[0], &["input"]),
        str_at(&journal_conv, &["input"])
    );
    assert_eq!(
        str_at(&conv_array[0], &["exception_text"]),
        str_at(&journal_conv, &["exception_text"])
    );

    // --- fail closed on a second consolidation ----------------------------
    let overwrite_dir = tmp.path().join("dump-twice");
    let stdout = run_ok(
        &executable,
        &[&overwrite_dir.display().to_string(), "dump-twice"],
        None,
        None,
    );
    assert!(
        stdout.contains("SECOND-DUMP-REFUSED"),
        "second consolidation did not refuse to overwrite"
    );

    // --- TEKMERION_FAILURE_DIR fallback -----------------------------------
    let envdir = tmp.path().join("from-env");
    let unused = tmp.path().join("unused");
    run_ok(
        &executable,
        &[&unused.display().to_string(), "env"],
        None,
        Some(&envdir),
    );
    check_journals(&envdir, "env fallback");

    // --- ./tekmerion-failures default -------------------------------------
    let defaultcwd = tmp.path().join("default-cwd");
    fs::create_dir_all(&defaultcwd).unwrap();
    run_ok(
        &executable,
        &[&unused.display().to_string(), "default"],
        Some(&defaultcwd),
        None,
    );
    check_journals(&defaultcwd.join("tekmerion-failures"), "default fallback");

    // --- bounded attempt stream -------------------------------------------
    let attempt_root = tmp.path().join("attempt-run");
    let stdout = run_ok(
        &executable,
        &[&attempt_root.display().to_string(), "attempt"],
        None,
        None,
    );
    assert!(
        stdout.contains("ATTEMPT-OK forced=true"),
        "attempt run did not finish as expected: {stdout:?}"
    );

    let attempt_dir = attempt_root.join("attempts");
    let stream_path = attempt_dir.join("ATTEMPTID123.jsonl");
    let manifest_path = attempt_dir.join("ATTEMPTID123.manifest.json");
    assert!(stream_path.is_file(), "attempt stream was not written");
    assert!(manifest_path.is_file(), "attempt manifest was not written");

    let raw = fs::read_to_string(&stream_path).unwrap();
    let raw_lines: Vec<&str> = raw.lines().collect();
    let records: Vec<Json> = raw_lines
        .iter()
        .map(|line| parse_json(line.as_bytes(), "attempt stream line"))
        .collect();
    assert!(records.len() >= 3, "attempt stream is too short");
    assert_eq!(str_at(&records[0], &["kind"]), "attempt_start");
    assert_eq!(
        str_at(&records[records.len() - 1], &["kind"]),
        "attempt_end"
    );

    let mut seqs: Vec<i64> = records.iter().map(|r| int_at(r, &["seq"])).collect();
    let before = seqs.clone();
    seqs.sort_unstable();
    seqs.dedup();
    assert_eq!(before, seqs, "seq is not strictly increasing");

    let kinds: Vec<&str> = records.iter().map(|r| str_at(r, &["kind"])).collect();
    for required in ["tactic_ok", "tactic_raised", "conv_ok", "conv_raised"] {
        assert!(kinds.contains(&required), "stream has no {required} event");
    }

    for record in &records {
        for field in [
            "seq",
            "kind",
            "name",
            "scope",
            "goal_sha256",
            "goal_bytes",
            "elapsed_cpu_s",
        ] {
            assert!(record.get(field).is_some(), "event is missing {field}");
        }
        let kind = str_at(record, &["kind"]);
        if ["tactic_ok", "tactic_raised", "conv_ok", "conv_raised"].contains(&kind) {
            let digest = str_at(record, &["goal_sha256"]);
            assert_eq!(digest.len(), 64, "bad digest: {digest:?}");
            assert!(
                digest
                    .bytes()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
                "bad digest: {digest:?}"
            );
        }
    }

    let end = &records[records.len() - 1];
    assert_eq!(str_at(end, &["outcome"]), "proved");
    assert_eq!(int_at(end, &["events"]), 105);
    assert_eq!(int_at(end, &["tactic_events"]), 103);
    assert_eq!(int_at(end, &["conv_events"]), 2);
    assert_eq!(int_at(end, &["tactic_ok"]), 2);
    assert_eq!(int_at(end, &["tactic_raised"]), 101);
    assert_eq!(int_at(end, &["conv_ok"]), 1);
    assert_eq!(int_at(end, &["conv_raised"]), 1);
    assert_eq!(int_at(end, &["streamed_events"]), 69);
    assert_eq!(int_at(end, &["dropped_events"]), 36);
    assert!(!bool_at(end, &["stream_complete"]));

    let ctrl: Vec<&Json> = records
        .iter()
        .filter(|r| {
            r.get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.contains('\u{1}'))
                .unwrap_or(false)
        })
        .collect();
    assert_eq!(
        ctrl.len(),
        1,
        "control-character name did not survive the stream"
    );
    assert_eq!(
        str_at(ctrl[0], &["name"]),
        "CTRL\u{1}\"\\TAC",
        "control-character name round-tripped wrongly"
    );

    let retained: Vec<&Json> = records
        .iter()
        .filter(|r| r.get("name").and_then(|v| v.as_str()) == Some("RETENTION_TAC"))
        .collect();
    assert_eq!(
        retained.len(),
        64,
        "RETENTION_TAC retained the wrong number"
    );
    assert!(retained
        .iter()
        .all(|r| str_at(r, &["kind"]) == "tactic_raised"));
    assert!(
        raw_lines.iter().any(|line| line.contains("\\u0001")),
        "no event carried the escaped control character"
    );

    let big: Vec<&Json> = records
        .iter()
        .filter(|r| r.get("name").and_then(|v| v.as_str()) == Some("BIG_TAC"))
        .collect();
    assert_eq!(big.len(), 1, "BIG_TAC was not streamed");
    assert!(!bool_at(big[0], &["term_retained"]));
    assert_eq!(int_at(big[0], &["goal_bytes"]), 20000);

    let manifest = read_json(&manifest_path);
    assert_eq!(str_at(&manifest, &["attempt_id"]), "ATTEMPTID123");
    assert_eq!(str_at(&manifest, &["outcome"]), "proved");
    assert_eq!(int_at(&manifest, &["retention", "first"]), 32);
    assert_eq!(int_at(&manifest, &["retention", "last"]), 32);
    assert!(!bool_at(&manifest, &["retention", "stream_complete"]));
    assert_eq!(
        int_at(&manifest, &["channels", "tactic", "total_events"]),
        103
    );
    assert_eq!(int_at(&manifest, &["channels", "tactic", "streamed"]), 67);
    assert_eq!(int_at(&manifest, &["channels", "tactic", "dropped"]), 36);
    assert_eq!(int_at(&manifest, &["channels", "conv", "total_events"]), 2);
    assert_eq!(int_at(&manifest, &["channels", "conv", "streamed"]), 2);
    assert_eq!(
        int_at(&manifest, &["channels", "tactic", "cap", "max_term_bytes"]),
        16384
    );
    assert_eq!(
        str_at(&manifest, &["channels", "tactic", "digest"]).len(),
        64
    );

    let wrappers = at(&manifest, &["wrappers"])
        .as_array()
        .expect("wrappers array");
    assert_eq!(
        int_at(wrapper_named(wrappers, "RETENTION_TAC"), &["total"]),
        100
    );
    assert_eq!(
        int_at(wrapper_named(wrappers, "RETENTION_TAC"), &["dropped"]),
        36
    );
    assert_eq!(
        int_at(wrapper_named(wrappers, "BIG_TAC"), &["retained_terms"]),
        0
    );
    assert_eq!(
        int_at(wrapper_named(wrappers, "BIG_TAC"), &["dropped_terms"]),
        1
    );
    assert_eq!(
        int_at(&manifest, &["declared_budget_bytes"]),
        128 * 1024 * 1024
    );

    let journal = attempt_root.join("RETENTION_TAC.failures.jsonl");
    assert!(
        journal.is_file(),
        "retention tactic journal was not written"
    );
    let journal_text = fs::read_to_string(&journal).unwrap();
    let journal_lines: Vec<&str> = journal_text.lines().collect();
    assert_eq!(
        journal_lines.len(),
        64,
        "retention journal has the wrong line count"
    );
    for line in journal_lines {
        parse_json(line.as_bytes(), "retention journal line");
    }

    let big_journal = attempt_root.join("BIG_TAC.failures.jsonl");
    let big_text = fs::read_to_string(&big_journal).unwrap();
    let big_first = parse_json(
        big_text
            .lines()
            .next()
            .expect("BIG_TAC journal line")
            .as_bytes(),
        "big journal line",
    );
    assert!(is_null(&big_first, &["goal_before"]));
    assert_eq!(int_at(&big_first, &["goal_bytes"]), 20000);
}
