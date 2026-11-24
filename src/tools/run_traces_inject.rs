//! Insert one tactic expression at the start of the first proof in a HOL file.
//!
//! Rust port of the earlier Python helper, with the same CLI and the same
//! deliberately narrow transform. **Not on the verdict path.**
//!
//! The insertion point is the first non-empty line after the first proof goal
//! terminator (a line whose right-stripped text ends with a backtick followed
//! by a comma). The inserted line is `<indent><TACTIC> THEN`, so the caller
//! supplies a complete tactic expression without a trailing THEN.
//!
//! If the file does not look like a `prove (...)` script, the helper exits
//! non-zero rather than guessing.

use std::fmt;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InjectError(pub String);

impl fmt::Display for InjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Insert `<tactic> THEN` at the first tactic position of the first proof.
pub fn inject(text: &str, tactic: &str) -> Result<String, InjectError> {
    if tactic.contains('\n') || tactic.contains('\r') {
        return Err(InjectError(
            "the injected tactic must be a single line".to_string(),
        ));
    }
    if tactic.trim().is_empty() {
        return Err(InjectError("the injected tactic is empty".to_string()));
    }

    let mut lines: Vec<String> = text.split('\n').map(|s| s.to_string()).collect();

    let prove_at = lines
        .iter()
        .position(|line| line.contains("prove"))
        .ok_or_else(|| InjectError("no `prove` found".to_string()))?;

    let goal_end = lines[prove_at..]
        .iter()
        .position(|line| line.trim_end().ends_with("`,"))
        .map(|offset| prove_at + offset)
        .ok_or_else(|| {
            InjectError(
                "no proof goal terminator (a line ending with a backtick and comma) \
                 found after the first `prove`"
                    .to_string(),
            )
        })?;

    let mut insert_at = goal_end + 1;
    while insert_at < lines.len() && lines[insert_at].trim().is_empty() {
        insert_at += 1;
    }
    if insert_at >= lines.len() {
        return Err(InjectError(
            "the first proof has no tactic after its goal".to_string(),
        ));
    }

    let target = &lines[insert_at];
    let indent = &target[..target.len() - target.trim_start().len()];
    lines.insert(insert_at, format!("{indent}{tactic} THEN"));
    Ok(lines.join("\n"))
}

/// `--self-test`: the insertion point, the indentation, and the refusals.
pub fn self_test() -> i32 {
    let mut ok = true;
    let proof = "let T = prove\n  (`x + 0 = x`,\n   REWRITE_TAC[] THEN\n   ARITH_TAC);;\n";
    match inject(proof, "TRY (FAIL_TAC \"probe\")") {
        Ok(out) => {
            if out.contains("   TRY (FAIL_TAC \"probe\") THEN\n   REWRITE_TAC[]") {
                println!("  ok: the tactic is inserted at the first proof step");
            } else {
                println!("  FAIL: the tactic was not inserted correctly: {out:?}");
                ok = false;
            }
        }
        Err(exc) => {
            println!("  FAIL: injection failed: {exc}");
            ok = false;
        }
    }

    for (label, text, tactic) in [
        ("a multi-line tactic is refused", proof, "A THEN\nB"),
        ("an empty tactic is refused", proof, "  "),
        ("input without `prove` is refused", "let x = 1\n", "TAC"),
    ] {
        match inject(text, tactic) {
            Err(exc) => println!("  ok: {label}: {exc}"),
            Ok(_) => {
                println!("  FAIL: {label}");
                ok = false;
            }
        }
    }

    match inject("let T = prove\n  (`x = x`,\n", "TAC") {
        Err(exc) => println!("  ok: a proof with no tactic after its goal is refused: {exc}"),
        Ok(_) => {
            println!("  FAIL: a proof with no tactic after its goal was accepted");
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
Usage: run_traces_inject INPUT OUTPUT \"TRY (FAIL_TAC \\\"probe\\\")\"
       run_traces_inject --self-test

Insert one tactic expression at the start of the first proof in a HOL file.";

pub fn run(args: &[String]) -> i32 {
    if args.len() == 1 && args[0] == "--self-test" {
        return self_test();
    }
    if args.len() == 1 && (args[0] == "-h" || args[0] == "--help") {
        println!("{USAGE}");
        return 0;
    }
    if args.len() != 3 {
        eprintln!("{USAGE}");
        return 2;
    }
    let source = Path::new(&args[0]);
    let destination = Path::new(&args[1]);
    let tactic = &args[2];

    let text = match fs::read_to_string(source) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("run_traces_inject: error: {e}");
            return 1;
        }
    };
    let injected = match inject(&text, tactic) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("run_traces_inject: error: {e}");
            return 1;
        }
    };
    if let Err(e) = fs::write(destination, injected) {
        eprintln!("run_traces_inject: error: {e}");
        return 1;
    }
    println!("injected `{tactic} THEN` into {}", destination.display());
    0
}
