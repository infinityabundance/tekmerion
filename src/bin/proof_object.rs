//! `proof_object` — finalise and independently check HOL Light proof objects.
//!
//! **Not on the native verdict path.**
//!
//! Subcommands:
//!
//! * `finalize --raw S --object O --store T [--theory H] [--expected E] [--json J]`
//!   — compact a recording-kernel raw stream, write the checked-theorem store
//!   and the expected statement.
//! * `theory --store T --out H [--axiom HEX]… [--definition HEX]… [--constant NAME]…
//!   [--type NAME]… [--typedef TY ABS REP]…` — write a validated-theory manifest.
//! * `validate --object O --theory H --store T --expected E [--budget N] [--json J]`
//!   — replay the object against an external environment.
//! * `scan --raw S` — statistics over a partial raw stream.
//!
//! Exit status: 0 on success, 1 when an object is *refused* (with the reason on
//! stderr), 2 on a usage error.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::ExitCode;

use tekmerion::tools::proof_object::{self, CheckedTheoremStore, Error, Expected, ValidatedTheory};

fn usage() -> ! {
    eprintln!(
        "usage:\n  \
         proof_object finalize --raw S --object O [--store T] [--theory H] [--expected E] [--json J]\n  \
         proof_object theory --store T --out H [--axiom HEX]... [--definition HEX]... [--constant NAME]... [--type NAME]... [--typedef TY ABS REP]...\n  \
         proof_object validate --object O --theory H --store T --expected E [--budget N] [--json J]\n  \
         proof_object scan --raw S"
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

fn parse_hex32(s: &str) -> [u8; 32] {
    if s.len() != 64 {
        eprintln!("expected 64 hex characters: {s}");
        usage();
    }
    let mut out = [0u8; 32];
    for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
        let hi = (chunk[0] as char).to_digit(16).unwrap_or_else(|| usage()) as u8;
        let lo = (chunk[1] as char).to_digit(16).unwrap_or_else(|| usage()) as u8;
        out[i] = (hi << 4) | lo;
    }
    out
}

/// This process's peak resident set size (`VmHWM`), for the receipt.  The cap
/// wrapper is the tree-wide accounting; this is the exact per-process figure.
fn self_peak_rss_kb() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmHWM:"))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|v| v.parse().ok())
        })
        .unwrap_or(0)
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
            let mut store = None;
            let mut theory = None;
            let mut expected = None;
            let mut json = None;
            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--raw" => raw = Some(PathBuf::from(arg_value(&args, &mut i, "--raw"))),
                    "--object" => {
                        object = Some(PathBuf::from(arg_value(&args, &mut i, "--object")))
                    }
                    "--store" => store = Some(PathBuf::from(arg_value(&args, &mut i, "--store"))),
                    "--theory" => {
                        theory = Some(PathBuf::from(arg_value(&args, &mut i, "--theory")))
                    }
                    "--expected" => {
                        expected = Some(PathBuf::from(arg_value(&args, &mut i, "--expected")))
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
            let theory = match theory {
                Some(p) => match std::fs::read(&p)
                    .map_err(Error::from)
                    .and_then(|b| ValidatedTheory::parse(&b))
                {
                    Ok(t) => Some(t),
                    Err(e) => {
                        eprintln!("finalize: theory: {e}");
                        return ExitCode::FAILURE;
                    }
                },
                None => None,
            };
            match proof_object::finalize(
                &raw,
                &object,
                store.as_deref(),
                theory.as_ref(),
                expected.as_deref(),
            ) {
                Ok(s) => {
                    let out = format!(
                        "{{\"theorem\":\"{}\",\"statement_digest\":\"{}\",\"statement_id\":\"{}\",\"theory_digest\":\"{}\",\"store_digest\":\"{}\",\"derivation_digest\":\"{}\",\"node_count\":{},\"raw_nodes\":{},\"bytes\":{},\"root_id\":{},\"imports\":{},\"inference_total\":{},\"inference_begin\":{},\"inference_delta\":{},\"peak_buffered_payload_bytes\":{},\"peak_rss_kb\":{},\"axioms\":{}}}",
                        json_escape(&s.theorem),
                        s.statement_digest,
                        s.statement_id,
                        s.theory_digest,
                        s.store_digest,
                        s.derivation_digest,
                        s.node_count,
                        s.raw_nodes,
                        s.bytes,
                        s.root_id,
                        s.imports,
                        s.inference_total,
                        s.inference_begin,
                        s.inference_delta(),
                        s.peak_buffered_payload_bytes,
                        self_peak_rss_kb(),
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
        "theory" => {
            let mut store_path = None;
            let mut out = None;
            let mut axioms = BTreeSet::new();
            let mut definitions = BTreeSet::new();
            let mut constants = BTreeSet::new();
            let mut types = BTreeSet::new();
            let mut typedefs = BTreeSet::new();
            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--store" => {
                        store_path = Some(PathBuf::from(arg_value(&args, &mut i, "--store")))
                    }
                    "--out" => out = Some(PathBuf::from(arg_value(&args, &mut i, "--out"))),
                    "--axiom" => {
                        axioms.insert(parse_hex32(&arg_value(&args, &mut i, "--axiom")));
                    }
                    "--definition" => {
                        definitions.insert(parse_hex32(&arg_value(&args, &mut i, "--definition")));
                    }
                    "--constant" => {
                        constants.insert(arg_value(&args, &mut i, "--constant"));
                    }
                    "--type" => {
                        types.insert(arg_value(&args, &mut i, "--type"));
                    }
                    "--typedef" => {
                        let ty = arg_value(&args, &mut i, "--typedef");
                        let abs = arg_value(&args, &mut i, "--typedef");
                        let rep = arg_value(&args, &mut i, "--typedef");
                        typedefs.insert((ty, abs, rep));
                    }
                    other => {
                        eprintln!("unknown argument {other}");
                        usage();
                    }
                }
                i += 1;
            }
            let (Some(store_path), Some(out)) = (store_path, out) else {
                usage();
            };
            let store = match CheckedTheoremStore::load(&store_path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("theory: {e}");
                    return ExitCode::FAILURE;
                }
            };
            let theory = ValidatedTheory::new(
                axioms,
                definitions,
                constants,
                types,
                typedefs,
                store.digest(),
            );
            if let Err(e) = std::fs::write(&out, theory.render()) {
                eprintln!("theory: cannot write {}: {e}", out.display());
                return ExitCode::FAILURE;
            }
            println!(
                "THEORY {{\"digest\":\"{}\",\"store_digest\":\"{}\"}}",
                theory.digest_hex(),
                store.digest_hex()
            );
            ExitCode::SUCCESS
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
            let mut theory = None;
            let mut store = None;
            let mut expected = None;
            let mut json = None;
            let mut budget = 2_000_000u64;
            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--object" => {
                        object = Some(PathBuf::from(arg_value(&args, &mut i, "--object")))
                    }
                    "--theory" => {
                        theory = Some(PathBuf::from(arg_value(&args, &mut i, "--theory")))
                    }
                    "--store" => store = Some(PathBuf::from(arg_value(&args, &mut i, "--store"))),
                    "--expected" => {
                        expected = Some(PathBuf::from(arg_value(&args, &mut i, "--expected")))
                    }
                    "--budget" => {
                        let v = arg_value(&args, &mut i, "--budget");
                        budget = v.parse().unwrap_or_else(|_| usage());
                    }
                    "--json" => json = Some(PathBuf::from(arg_value(&args, &mut i, "--json"))),
                    other => {
                        eprintln!("unknown argument {other}");
                        usage();
                    }
                }
                i += 1;
            }
            let (Some(object), Some(theory), Some(store), Some(expected)) =
                (object, theory, store, expected)
            else {
                usage();
            };
            let theory = match std::fs::read(&theory)
                .map_err(Error::from)
                .and_then(|b| ValidatedTheory::parse(&b))
            {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("validate: theory: {e}");
                    return ExitCode::FAILURE;
                }
            };
            let store = match CheckedTheoremStore::load(&store) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("validate: {e}");
                    return ExitCode::FAILURE;
                }
            };
            let expected = match Expected::load(&expected) {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("validate: {e}");
                    return ExitCode::FAILURE;
                }
            };
            match proof_object::validate(&object, &expected, &theory, &store, budget) {
                Ok(v) => {
                    let out = format!(
                        "{{\"theorem\":\"{}\",\"statement_digest\":\"{}\",\"statement_id\":\"{}\",\"theory_digest\":\"{}\",\"store_digest\":\"{}\",\"derivation_digest\":\"{}\",\"node_count\":{},\"reachable\":{},\"raw_nodes\":{},\"imports\":{},\"inference_total\":{},\"inference_begin\":{},\"peak_rss_kb\":{},\"axioms\":{}}}",
                        json_escape(&v.theorem),
                        v.statement_digest,
                        v.statement_id,
                        v.theory_digest,
                        v.store_digest,
                        v.derivation_digest,
                        v.node_count,
                        v.reachable,
                        v.raw_nodes,
                        v.imports,
                        v.inference_total,
                        v.inference_begin,
                        self_peak_rss_kb(),
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

/// A stable machine-readable class for each refusal.
pub fn refusal_kind(e: &Error) -> &'static str {
    match e {
        Error::DigestMismatch
        | Error::CountMismatch { .. }
        | Error::RawTruncated
        | Error::RawShortRecord
        | Error::BadMagic => "truncated",
        Error::RuleFailure { .. } | Error::AssumptionSetMismatch { .. } => "spliced",
        Error::ConclusionMismatch { .. } | Error::StatementIdentityMismatch { .. } => "conclusion",
        Error::ImportNotJustified { .. } => "import",
        Error::AxiomNotAuthorised { .. }
        | Error::DefinitionNotAuthorised { .. }
        | Error::DefinitionNotFresh { .. }
        | Error::TypeDefinitionNotAuthorised { .. }
        | Error::TypeNotFresh { .. }
        | Error::TheoryDigestMismatch { .. }
        | Error::StoreDigestMismatch { .. }
        | Error::StoreMalformed(_) => "environment",
        Error::BudgetExceeded { .. } => "budget",
        _ => "malformed",
    }
}
