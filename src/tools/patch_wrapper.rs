//! Patch a TacticTrace-generated `kernel_wrapper.ml` for failure capture.
//!
//! Rust port of the earlier Python helper, with the same CLI and the same
//! fail-closed, idempotent transform. **Not on the verdict path.**
//!
//! The stock generator emits, for every tactic wrapper:
//!
//! ```text
//! let (trace_ga_inst,trace_goals_after,trace_justf) = <application> (trace_asl,trace_g) in
//! exptrace_add_tac "<tacname>" {
//! ```
//!
//! and for every conversion wrapper:
//!
//! ```text
//! let thm_output = <application> in
//! exptrace_add_conv "<convname>" {
//!   ...
//!   input = <term-expression>;
//! ```
//!
//! This tool rewrites only the application lines. The subsequent
//! `exptrace_add_tac` / `exptrace_add_conv` emissions are left byte-for-byte
//! alone, so success records remain bit-identical. The patch is idempotent and
//! fails closed if the shape is not recognised.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchError(pub String);

impl fmt::Display for PatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

fn err<T>(message: impl Into<String>) -> Result<T, PatchError> {
    Err(PatchError(message.into()))
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    pub patched: usize,
    pub already: usize,
    pub conv_patched: usize,
    pub conv_already: usize,
}

const TAC_APPLICATION_PREFIX: &str = "    let (trace_ga_inst,trace_goals_after,trace_justf) = ";
const TAC_PATCHED: &str = "    let (trace_ga_inst,trace_goals_after,trace_justf) =";
const TAC_ADD_PREFIX: &str = "    exptrace_add_tac \"";
const CONV_APPLICATION_PREFIX: &str = "  let thm_output = ";
const CONV_PATCHED: &str = "  let thm_output =";
const CONV_ADD_PREFIX: &str = "  exptrace_add_conv \"";
const CONV_INPUT_PREFIX: &str = "    input = ";

/// `^    let (trace_ga_inst,trace_goals_after,trace_justf) = (.*) in$`
fn tac_application(line: &str) -> Option<&str> {
    line.strip_prefix(TAC_APPLICATION_PREFIX)?
        .strip_suffix(" in")
}

/// `^      exptrace_guard "([^"]+)" trace_goal_before \(fun \(\) -> (.*)\) in$`
fn tac_guard(line: &str) -> Option<(&str, &str)> {
    let rest = line.strip_prefix("      exptrace_guard \"")?;
    let quote = rest.find('"')?;
    if quote == 0 {
        return None;
    }
    let (name, tail) = (&rest[..quote], &rest[quote + 1..]);
    let capture = tail
        .strip_prefix(" trace_goal_before (fun () -> ")?
        .strip_suffix(") in")?;
    Some((name, capture))
}

/// `^    exptrace_add_tac "([^"]+)" \{$`
fn tac_add(line: &str) -> Option<&str> {
    let rest = line.strip_prefix(TAC_ADD_PREFIX)?.strip_suffix("\" {")?;
    if rest.is_empty() || rest.contains('"') {
        return None;
    }
    Some(rest)
}

/// `^  let thm_output = (.*) in$`
fn conv_application(line: &str) -> Option<&str> {
    line.strip_prefix(CONV_APPLICATION_PREFIX)?
        .strip_suffix(" in")
}

/// `^    exptrace_guard_conv "([^"]+)" \(fun \(\) -> (.*)\) \(fun \(\) -> (.*)\) in$`
fn conv_guard(line: &str) -> Option<(&str, &str, &str)> {
    let rest = line.strip_prefix("    exptrace_guard_conv \"")?;
    let quote = rest.find('"')?;
    if quote == 0 {
        return None;
    }
    let (name, tail) = (&rest[..quote], &rest[quote + 1..]);
    let tail = tail.strip_prefix(" (fun () -> ")?.strip_suffix(") in")?;
    // The first `.*` is greedy: it ends at the last `) (fun () -> `.
    let at = tail.rfind(") (fun () -> ")?;
    Some((name, &tail[..at], &tail[at + ") (fun () -> ".len()..]))
}

/// `^  exptrace_add_conv "([^"]+)" \{$`
fn conv_add(line: &str) -> Option<&str> {
    let rest = line.strip_prefix(CONV_ADD_PREFIX)?.strip_suffix("\" {")?;
    if rest.is_empty() || rest.contains('"') {
        return None;
    }
    Some(rest)
}

/// `^    input = (.*);$`
fn conv_input(line: &str) -> Option<&str> {
    line.strip_prefix(CONV_INPUT_PREFIX)?.strip_suffix(";")
}

/// Return (index, expression) of the `input = ...;` line of a conv block.
///
/// The application line is followed by the `exptrace_add_conv` line and then
/// the record fields. Searching stops at the record's argument closure so a
/// later conversion cannot be mistaken for this one.
fn find_conv_input(lines: &[String], start: usize) -> Result<(usize, String), PatchError> {
    let end = lines.len().min(start + 20);
    for (index, line) in lines.iter().enumerate().take(end).skip(start) {
        if let Some(expr) = conv_input(line) {
            return Ok((index, expr.to_string()));
        }
        if line.starts_with("  } (fun () -> {") {
            break;
        }
    }
    err(format!(
        "line {}: cannot find the conversion input term \
         (expected `    input = <term>;`)",
        start + 1
    ))
}

/// Return patched text and counts, or a precise error.
pub fn patch_text(text: &str) -> Result<(String, Stats), PatchError> {
    let lines: Vec<String> = text.split('\n').map(|s| s.to_string()).collect();
    let mut out: Vec<String> = Vec::new();
    let mut stats = Stats::default();
    let mut i = 0;

    while i < lines.len() {
        let line = &lines[i];

        if let Some(application) = tac_application(line) {
            if i + 1 >= lines.len() {
                return err(format!(
                    "line {}: tactic application is not followed by an exptrace_add_tac line",
                    i + 1
                ));
            }
            let Some(name) = tac_add(&lines[i + 1]) else {
                return err(format!(
                    "line {}: cannot find the tactic name on line {} \
                     (expected `    exptrace_add_tac \"<tacname>\" {{`)",
                    i + 1,
                    i + 2
                ));
            };
            out.push(TAC_PATCHED.to_string());
            out.push(format!(
                "      exptrace_guard \"{name}\" trace_goal_before (fun () -> {application}) in"
            ));
            stats.patched += 1;
            // Leave lines[i + 1] (the exptrace_add_tac line) for the next
            // iteration, untouched.
            i += 1;
            continue;
        }

        if line == TAC_PATCHED {
            if i + 1 >= lines.len() || tac_guard(&lines[i + 1]).is_none() {
                return err(format!(
                    "line {}: already-patched application is not followed \
                     by a well-formed exptrace_guard line",
                    i + 1
                ));
            }
            stats.already += 1;
        }

        if let Some(application) = conv_application(line) {
            if i + 1 >= lines.len() {
                return err(format!(
                    "line {}: conversion application is not followed by \
                     an exptrace_add_conv line",
                    i + 1
                ));
            }
            let Some(name) = conv_add(&lines[i + 1]) else {
                return err(format!(
                    "line {}: cannot find the conversion name on line {} \
                     (expected `  exptrace_add_conv \"<convname>\" {{`)",
                    i + 1,
                    i + 2
                ));
            };
            let (_, input_expr) = find_conv_input(&lines, i + 2)?;
            out.push(CONV_PATCHED.to_string());
            out.push(format!(
                "    exptrace_guard_conv \"{name}\" (fun () -> {input_expr}) \
                 (fun () -> {application}) in"
            ));
            stats.conv_patched += 1;
            // Leave the exptrace_add_conv line and its record for the next
            // iterations, untouched.
            i += 1;
            continue;
        }

        if line == CONV_PATCHED {
            if i + 1 >= lines.len() || conv_guard(&lines[i + 1]).is_none() {
                return err(format!(
                    "line {}: already-patched conversion is not followed \
                     by a well-formed exptrace_guard_conv line",
                    i + 1
                ));
            }
            stats.conv_already += 1;
        }

        out.push(line.clone());
        i += 1;
    }

    if stats.patched + stats.already + stats.conv_patched + stats.conv_already == 0 {
        return err(
            "no tactic or conversion wrapper matched; this does not look like a \
             generated kernel_wrapper.ml",
        );
    }

    Ok((out.join("\n"), stats))
}

fn patch_file(source: &Path, destination: &Path) -> Result<Stats, PatchError> {
    let text = fs::read_to_string(source).map_err(|e| PatchError(e.to_string()))?;
    let (patched, stats) = patch_text(&text)?;
    fs::write(destination, patched).map_err(|e| PatchError(e.to_string()))?;
    Ok(stats)
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("trace/fixtures")
}

fn expect_error(label: &str, text: &str, needle: &str) -> bool {
    match patch_text(text) {
        Err(exc) => {
            if exc.0.contains(needle) {
                println!("  ok: {label}: {exc}");
                true
            } else {
                println!("  FAIL: {label}: wrong error: {exc}");
                false
            }
        }
        Ok(_) => {
            println!("  FAIL: {label}: expected PatchError");
            false
        }
    }
}

/// Ported `--self-test`: the golden fixture, idempotence, a conversion-only
/// file, four refusals and the file round-trip.
pub fn self_test() -> i32 {
    let dir = fixtures();
    let before_path = dir.join("kernel_wrapper_before.ml");
    let after_path = dir.join("kernel_wrapper_after.ml");

    if !before_path.is_file() || !after_path.is_file() {
        eprintln!("FAIL: missing fixtures under {}", dir.display());
        return 1;
    }

    let expected = fs::read_to_string(&after_path).unwrap_or_default();
    let mut ok = true;

    let (got, stats) = match fs::read_to_string(&before_path)
        .map_err(|e| PatchError(e.to_string()))
        .and_then(|text| patch_text(&text))
    {
        Ok(pair) => pair,
        Err(exc) => {
            println!("  FAIL: patching the golden fixture failed: {exc}");
            return 1;
        }
    };

    if got == expected {
        println!(
            "  ok: golden fixture matches kernel_wrapper_after.ml \
             (tactics patched {}, conversions patched {})",
            stats.patched, stats.conv_patched
        );
    } else {
        println!("  FAIL: golden fixture output differs from kernel_wrapper_after.ml");
        ok = false;
        for (index, (want, have)) in expected.split('\n').zip(got.split('\n')).enumerate() {
            if want != have {
                println!("    first difference at line {}:", index + 1);
                println!("      expected: {want:?}");
                println!("      actual:   {have:?}");
                break;
            }
        }
    }

    // Idempotence: feeding the patched output back in must not double-wrap.
    match patch_text(&got) {
        Ok((got2, stats2)) => {
            if got2 == expected
                && stats2.patched == 0
                && stats2.already == stats.patched
                && stats2.conv_patched == 0
                && stats2.conv_already == stats.conv_patched
            {
                println!(
                    "  ok: idempotent (tactics patched 0, already patched {}; \
                     conversions patched 0, already patched {})",
                    stats2.already, stats2.conv_already
                );
            } else {
                println!(
                    "  FAIL: second pass changed the output or double-wrapped \
                     (tactics patched {}, already {}; conversions patched {}, already {})",
                    stats2.patched, stats2.already, stats2.conv_patched, stats2.conv_already
                );
                ok = false;
            }
        }
        Err(exc) => {
            println!("  FAIL: second pass failed: {exc}");
            ok = false;
        }
    }

    // A conversion-only file is a valid patch target now.
    let conv_only = concat!(
        "  let thm_output = BETA_CONV tm in\n",
        "  exptrace_add_conv \"BETA_CONV\" {\n",
        "    input = tm;\n",
        "  } (fun () -> {});\n",
    );
    match patch_text(conv_only) {
        Ok((conv_patched, conv_stats)) => {
            if conv_stats.conv_patched == 1
                && conv_patched.contains(
                    "exptrace_guard_conv \"BETA_CONV\" (fun () -> tm) \
                     (fun () -> BETA_CONV tm) in",
                )
            {
                println!("  ok: conversion-only input is patched");
            } else {
                println!("  FAIL: conversion-only input was not patched correctly");
                ok = false;
            }
        }
        Err(exc) => {
            println!("  FAIL: conversion-only input was refused: {exc}");
            ok = false;
        }
    }

    ok &= expect_error(
        "missing exptrace_add_tac name is refused",
        concat!(
            "    let (trace_ga_inst,trace_goals_after,trace_justf) = ",
            "FOO (trace_asl,trace_g) in\n",
        ),
        "cannot find the tactic name",
    );
    ok &= expect_error(
        "missing exptrace_add_conv name is refused",
        "  let thm_output = BETA_CONV tm in\n",
        "cannot find the conversion name",
    );
    ok &= expect_error(
        "missing conversion input is refused",
        concat!(
            "  let thm_output = BETA_CONV tm in\n",
            "  exptrace_add_conv \"BETA_CONV\" {\n",
            "  } (fun () -> {});\n",
        ),
        "cannot find the conversion input term",
    );
    ok &= expect_error(
        "unrecognised input is refused",
        "let x = 1\n",
        "no tactic or conversion wrapper matched",
    );

    // Exercise the file path end-to-end as well.
    let destination = std::env::temp_dir().join(format!(
        "patch-wrapper-selftest-{}-{}.ml",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    match patch_file(&before_path, &destination) {
        Ok(file_stats) => {
            let round_trip = fs::read_to_string(&destination).unwrap_or_default();
            if round_trip == expected
                && file_stats.patched == stats.patched
                && file_stats.conv_patched == stats.conv_patched
            {
                println!("  ok: patch_file round-trip");
            } else {
                println!("  FAIL: patch_file round-trip");
                ok = false;
            }
        }
        Err(exc) => {
            println!("  FAIL: patch_file round-trip: {exc}");
            ok = false;
        }
    }
    let _ = fs::remove_file(&destination);

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
Usage: patch_wrapper IN OUT
       patch_wrapper --self-test

Wrap TacticTrace tactic and conversion applications with
exptrace_guard / exptrace_guard_conv.";

pub fn run(args: &[String]) -> i32 {
    let mut positional: Vec<&str> = Vec::new();
    let mut self_test_flag = false;
    for arg in args {
        match arg.as_str() {
            "--self-test" => self_test_flag = true,
            "-h" | "--help" => {
                println!("{USAGE}");
                return 0;
            }
            other if other.starts_with('-') && other.len() > 1 => {
                eprintln!("patch_wrapper: error: unrecognized argument: {other}");
                eprintln!("{USAGE}");
                return 2;
            }
            other => positional.push(other),
        }
    }

    if self_test_flag {
        if !positional.is_empty() {
            eprintln!("patch_wrapper: error: --self-test takes no positional arguments");
            return 2;
        }
        return self_test();
    }

    if positional.len() != 2 {
        eprintln!("patch_wrapper: error: IN and OUT are required unless --self-test is used");
        eprintln!("{USAGE}");
        return 2;
    }

    let source = Path::new(positional[0]);
    let destination = Path::new(positional[1]);
    if !source.is_file() {
        eprintln!(
            "patch_wrapper: error: input not found: {}",
            source.display()
        );
        return 1;
    }

    let stats = match patch_file(source, destination) {
        Ok(s) => s,
        Err(exc) => {
            eprintln!("patch_wrapper: error: {exc}");
            return 1;
        }
    };

    println!("tactics patched: {}", stats.patched);
    if stats.already > 0 {
        println!(
            "tactics already patched, not wrapped again: {}",
            stats.already
        );
    }
    println!("conversions patched: {}", stats.conv_patched);
    if stats.conv_already > 0 {
        println!(
            "conversions already patched, not wrapped again: {}",
            stats.conv_already
        );
    }
    println!("wrote {}", destination.display());
    0
}
