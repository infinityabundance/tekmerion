//! `proof_object` — finalise and independently check HOL Light proof objects.
//!
//! **Not on the native verdict path.**
//!
//! Subcommands:
//!
//! * `finalize --raw S --object O [--json J]` — compact a recording-kernel raw
//!   stream into an object, computing reachability from the recorded root.
//! * `validate --object O [--json J]` — replay the object independently.
//!
//! Exit status: 0 on success, 1 when an object is *refused* (with the reason on
//! stderr), 2 on a usage error.

use std::path::PathBuf;
use std::process::ExitCode;

use tekmerion::tools::proof_object::{self, Error};

fn usage() -> ! {
    eprintln!(
        "usage:\n  proof_object finalize --raw S --object O [--json J]\n  proof_object validate --object O [--json J]"
    );
    std::process::exit(2)
}

fn arg_value(args: &[String], i: &mut usize, name: &str) -> String {
    if *i + 1 >= args.len() {
        eprintln!("missing value for {name}");
        usage();
    }
    *i += 1;
    args[*i].clone()
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn json_array(items: &[String]) -> String {
    let mut s = String::from("[");
    for (i, it) in items.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push('"');
        s.push_str(&json_escape(it));
        s.push('"');
    }
    s.push(']');
    s
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        usage();
    }
    match args[1].as_str() {
        "finalize" => {
            let mut raw = None;
            let mut object = None;
            let mut json = None;
            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--raw" => raw = Some(PathBuf::from(arg_value(&args, &mut i, "--raw"))),
                    "--object" => {
                        object = Some(PathBuf::from(arg_value(&args, &mut i, "--object")))
                    }
                    "--json" => json = Some(PathBuf::from(arg_value(&args, &mut i, "--json"))),
                    other => {
                        eprintln!("unknown argument {other}");
                        usage();
                    }
                }
                i += 1;
            }
            let (Some(raw), Some(object)) = (raw, object) else {
                usage();
            };
            match proof_object::finalize(&raw, &object) {
                Ok(s) => {
                    let out = format!(
                        "{{\"theorem\":\"{}\",\"statement_digest\":\"{}\",\"derivation_digest\":\"{}\",\"node_count\":{},\"bytes\":{},\"root_id\":{},\"imports\":{},\"inference_total\":{},\"inference_begin\":{},\"inference_delta\":{},\"axioms\":{}}}",
                        json_escape(&s.theorem),
                        s.statement_digest,
                        s.derivation_digest,
                        s.node_count,
                        s.bytes,
                        s.root_id,
                        s.imports,
                        s.inference_total,
                        s.inference_begin,
                        s.inference_delta(),
                        json_array(&s.axioms),
                    );
                    if let Some(p) = json {
                        if let Err(e) = std::fs::write(&p, format!("{out}\n")) {
                            eprintln!("finalize: cannot write {}: {e}", p.display());
                            return ExitCode::FAILURE;
                        }
                    }
                    println!("FINALIZED {out}");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("finalize: refusal: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        "scan" => {
            let mut raw = None;
            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--raw" => raw = Some(PathBuf::from(arg_value(&args, &mut i, "--raw"))),
                    other => {
                        eprintln!("unknown argument {other}");
                        usage();
                    }
                }
                i += 1;
            }
            let Some(raw) = raw else { usage() };
            match proof_object::scan_stream(&raw) {
                Ok(s) => {
                    println!(
                        "SCANNED {{\"nodes\":{},\"bytes\":{},\"payload_bytes\":{},\"max_payload\":{},\"finished\":{}}}",
                        s.nodes, s.bytes, s.payload_bytes, s.max_payload, s.finished
                    );
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("scan: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        "validate" => {
            let mut object = None;
            let mut json = None;
            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--object" => {
                        object = Some(PathBuf::from(arg_value(&args, &mut i, "--object")))
                    }
                    "--json" => json = Some(PathBuf::from(arg_value(&args, &mut i, "--json"))),
                    other => {
                        eprintln!("unknown argument {other}");
                        usage();
                    }
                }
                i += 1;
            }
            let Some(object) = object else { usage() };
            match proof_object::validate(&object) {
                Ok(v) => {
                    let out = format!(
                        "{{\"theorem\":\"{}\",\"statement_digest\":\"{}\",\"derivation_digest\":\"{}\",\"node_count\":{},\"raw_nodes\":{},\"imports\":{},\"inference_total\":{},\"inference_begin\":{},\"axioms\":{}}}",
                        json_escape(&v.theorem),
                        v.statement_digest,
                        v.derivation_digest,
                        v.node_count,
                        v.raw_nodes,
                        v.imports,
                        v.inference_total,
                        v.inference_begin,
                        json_array(&v.axioms),
                    );
                    if let Some(p) = json {
                        if let Err(e) = std::fs::write(&p, format!("{out}\n")) {
                            eprintln!("validate: cannot write {}: {e}", p.display());
                            return ExitCode::FAILURE;
                        }
                    }
                    println!("VALIDATED {out}");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    // Keep the refusal class visible and stable for the controls.
                    eprintln!("validate: REFUSED ({}) {e}", refusal_kind(&e));
                    ExitCode::FAILURE
                }
            }
        }
        other => {
            eprintln!("unknown subcommand {other}");
            usage();
        }
    }
}

/// A stable machine-readable class for each refusal. The three controls assert
/// on these strings.
pub fn refusal_kind(e: &Error) -> &'static str {
    match e {
        Error::DigestMismatch
        | Error::CountMismatch { .. }
        | Error::RawTruncated
        | Error::RawShortRecord => "truncated",
        Error::RuleFailure { .. } => "spliced",
        Error::ConclusionMismatch { .. } => "conclusion",
        Error::AssumptionSetMismatch { .. } => "spliced",
        _ => "malformed",
    }
}
