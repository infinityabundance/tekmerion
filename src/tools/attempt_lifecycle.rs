//! Replace attempt-lifecycle sentinels in a TacticTrace-generated file.
//!
//! Rust port of the earlier Python helper, with the same CLI and the same
//! fail-closed behaviour. **Not on the verdict path.**
//!
//! `tools/run_traces.sh` puts three ordinary OCaml comments in the proof
//! template:
//!
//! ```text
//! (* TEKMERION_ATTEMPT_START *)
//! (* TEKMERION_ATTEMPT_TIMEOUT *)
//! (* TEKMERION_ATTEMPT_END *)
//! ```
//!
//! The template is type-checked by `TacticTrace/get-ast.sh` *before* the
//! failure module is spliced in, so the template cannot name `exptrace_*`
//! directly. The comments survive `tracer modify`; this tool replaces each
//! sentinel line with the real call, after the module has been placed in the
//! compilation unit. It fails closed if a sentinel is missing or duplicated.

use std::fmt;
use std::fs;
use std::path::Path;

pub const START: &str = "(* TEKMERION_ATTEMPT_START *)";
pub const TIMEOUT: &str = "(* TEKMERION_ATTEMPT_TIMEOUT *)";
pub const END: &str = "(* TEKMERION_ATTEMPT_END *)";

const START_BLOCK: &[&str] = &[
    "exptrace_attempt_start (Sys.getenv \"TEKMERION_ATTEMPT_ID\") \
     (Sys.getenv \"TEKMERION_THEOREM_ID\") \
     (Sys.getenv \"TEKMERION_PROOF_SHA\") \
     (Sys.getenv \"TEKMERION_INJECTION\") \
     (Sys.getenv \"TEKMERION_WRAPPER_SHA\") \
     (Sys.getenv \"TEKMERION_MODULE_SHA\");;",
    "exptrace_attempt_expect \"failed\";;",
];

const TIMEOUT_BLOCK: &[&str] = &[
    "let rt_timeout = (try int_of_string \
     (Sys.getenv \"TEKMERION_TIMEOUT_S\") with Not_found -> 0);;",
    "if rt_timeout > 0 then \
     Sys.set_signal Sys.sigalrm \
     (Sys.Signal_handle (fun _ -> exptrace_attempt_end \"timeout\"; \
     exit 124));;",
    "if rt_timeout > 0 then ignore (Unix.alarm rt_timeout);;",
];

const END_BLOCK: &[&str] = &[
    "exptrace_attempt_end \"proved\";;",
    "if rt_timeout > 0 then ignore (Unix.alarm 0);;",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleError(pub String);

impl fmt::Display for LifecycleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

fn block_for(sentinel: &str) -> Option<&'static [&'static str]> {
    match sentinel {
        START => Some(START_BLOCK),
        TIMEOUT => Some(TIMEOUT_BLOCK),
        END => Some(END_BLOCK),
        _ => None,
    }
}

/// Replace each sentinel line with its block, or refuse if any count is not 1.
pub fn expand(text: &str) -> Result<String, LifecycleError> {
    let mut counts = [0usize; 3];
    let mut out: Vec<String> = Vec::new();
    for line in text.split('\n') {
        let stripped = line.trim();
        match block_for(stripped) {
            Some(block) => {
                let index = match stripped {
                    START => 0,
                    TIMEOUT => 1,
                    _ => 2,
                };
                counts[index] += 1;
                out.extend(block.iter().map(|s| s.to_string()));
            }
            None => out.push(line.to_string()),
        }
    }
    let sentinels = [START, TIMEOUT, END];
    let missing: Vec<&str> = sentinels
        .iter()
        .enumerate()
        .filter(|(i, _)| counts[*i] != 1)
        .map(|(_, s)| *s)
        .collect();
    if !missing.is_empty() {
        return Err(LifecycleError(format!(
            "expected exactly one of each sentinel; missing or duplicated: {}",
            missing.join(", ")
        )));
    }
    Ok(out.join("\n"))
}

/// Ported `--self-test`.
pub fn self_test() -> i32 {
    let mut ok = true;
    let template = format!(
        "let rt_start = Unix.time();;\n{START}\n{TIMEOUT}\ncheck_axioms ();;\n{END}\n\
         Printf.printf \"ok\\n\";;\n"
    );
    let result = expand(&template).unwrap_or_default();
    if result.contains("exptrace_attempt_start")
        && result.matches("exptrace_attempt_end").count() == 2
    {
        println!("  ok: sentinels expand to the lifecycle calls");
    } else {
        println!("  FAIL: expansion is wrong");
        ok = false;
    }
    match expand("let x = 1\n") {
        Err(exc) => println!("  ok: missing sentinels are refused: {exc}"),
        Ok(_) => {
            println!("  FAIL: missing sentinels were accepted");
            ok = false;
        }
    }
    match expand(&format!("{START}\n{START}\n{TIMEOUT}\n{END}\n")) {
        Err(exc) => println!("  ok: duplicated sentinels are refused: {exc}"),
        Ok(_) => {
            println!("  FAIL: duplicated sentinels were accepted");
            ok = false;
        }
    }
    println!(
        "{}",
        if ok {
            "SELF-TEST PASSED"
        } else {
            "SELF-TEST FAILED"
        }
    );
    if ok {
        0
    } else {
        1
    }
}

const USAGE: &str = "\
Usage: attempt_lifecycle INPUT OUTPUT
       attempt_lifecycle --self-test

Replace attempt-lifecycle sentinels in a TacticTrace-generated file.";

pub fn run(args: &[String]) -> i32 {
    if args.len() == 1 && args[0] == "--self-test" {
        return self_test();
    }
    if args.len() == 1 && (args[0] == "-h" || args[0] == "--help") {
        println!("{USAGE}");
        return 0;
    }
    if args.len() != 2 {
        eprintln!("{USAGE}");
        return 2;
    }
    let source = Path::new(&args[0]);
    let destination = Path::new(&args[1]);
    let text = match fs::read_to_string(source) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("attempt_lifecycle: error: {e}");
            return 1;
        }
    };
    let expanded = match expand(&text) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("attempt_lifecycle: error: {e}");
            return 1;
        }
    };
    if let Err(e) = fs::write(destination, expanded) {
        eprintln!("attempt_lifecycle: error: {e}");
        return 1;
    }
    0
}
