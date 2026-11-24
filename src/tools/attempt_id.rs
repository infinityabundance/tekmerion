//! Content-addressed `attempt_id` for one traced proof attempt.
//!
//! Rust port of the earlier Python helper, with the same CLI and the same
//! digest. **Not on the verdict path.**
//!
//! The id is the SHA-256 of the canonical tuple, in this order:
//!
//! ```text
//! theorem_id, proof_text, injection, wrapper_sha256, module_sha256
//! ```
//!
//! Each component is encoded as an 8-byte big-endian length followed by its
//! bytes, so no redistribution of bytes between components can collide.
//! `proof_text` is the exact bytes of the proof file (not a digest), which is
//! why the id changes when the proof changes even if its name does not.
//!
//! The id is the file stem of the attempt stream (`<attempt_id>.jsonl`) and the
//! manifest (`<attempt_id>.manifest.json`). It is printed as 64 lowercase hex
//! characters.

use sha2::{Digest, Sha256};
use std::fs;

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

/// The digest of the canonical tuple, as 64 lowercase hex characters.
pub fn compute_attempt_id(
    theorem_id: &str,
    proof_text: &[u8],
    injection: &str,
    wrapper_sha256: &str,
    module_sha256: &str,
) -> String {
    let mut h = Sha256::new();
    for component in [
        theorem_id.as_bytes(),
        proof_text,
        injection.as_bytes(),
        wrapper_sha256.as_bytes(),
        module_sha256.as_bytes(),
    ] {
        h.update((component.len() as u64).to_be_bytes());
        h.update(component);
    }
    hex(&h.finalize())
}

/// Ported `--self-test`: determinism, every component, and the length framing.
pub fn self_test() -> i32 {
    let mut ok = true;

    let base = compute_attempt_id("arm/proofs/word_max.ml", b"proof", "", "aa", "bb");
    let again = compute_attempt_id("arm/proofs/word_max.ml", b"proof", "", "aa", "bb");
    if base == again
        && base.len() == 64
        && base
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    {
        println!("  ok: id is deterministic, 64 lowercase hex characters");
    } else {
        println!("  FAIL: id is not deterministic 64-character lowercase hex");
        ok = false;
    }

    let mutations = [
        (
            "theorem_id",
            compute_attempt_id("arm/proofs/word_min.ml", b"proof", "", "aa", "bb"),
        ),
        (
            "proof_text",
            compute_attempt_id("arm/proofs/word_max.ml", b"prooF", "", "aa", "bb"),
        ),
        (
            "injection",
            compute_attempt_id(
                "arm/proofs/word_max.ml",
                b"proof",
                "TRY (FAIL_TAC \"probe\")",
                "aa",
                "bb",
            ),
        ),
        (
            "wrapper",
            compute_attempt_id("arm/proofs/word_max.ml", b"proof", "", "ab", "bb"),
        ),
        (
            "module",
            compute_attempt_id("arm/proofs/word_max.ml", b"proof", "", "aa", "bc"),
        ),
    ];
    for (name, value) in mutations {
        if value != base {
            println!("  ok: changing {name} changes the id");
        } else {
            println!("  FAIL: changing {name} did not change the id");
            ok = false;
        }
    }

    // Length framing: moving a byte between adjacent components must not collide.
    let left = compute_attempt_id("ab", b"c", "", "", "");
    let right = compute_attempt_id("a", b"bc", "", "", "");
    if left != right {
        println!("  ok: length framing prevents component-boundary collisions");
    } else {
        println!("  FAIL: component-boundary collision");
        ok = false;
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
Usage: attempt_id --theorem-id ID --proof PATH [--inject TEXT] \
--wrapper-sha SHA --module-sha SHA
       attempt_id --self-test

Content-addressed attempt id for a traced proof attempt.";

#[derive(Default)]
struct Opts {
    theorem_id: Option<String>,
    proof: Option<String>,
    inject: String,
    wrapper_sha: Option<String>,
    module_sha: Option<String>,
    self_test: bool,
    help: bool,
}

fn usage_error(message: &str) -> i32 {
    eprintln!("attempt_id: error: {message}");
    eprintln!("{USAGE}");
    2
}

fn parse(args: &[String]) -> Result<Opts, String> {
    let mut o = Opts::default();
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        let (name, inline) = match arg.split_once('=') {
            Some((n, v)) if n.starts_with("--") => (n, Some(v.to_string())),
            _ => (arg.as_str(), None),
        };
        match name {
            "--self-test" | "-h" | "--help" if inline.is_some() => {
                return Err(format!("unrecognized argument: {arg}"));
            }
            "--self-test" => o.self_test = true,
            "-h" | "--help" => o.help = true,
            "--theorem-id" | "--proof" | "--inject" | "--wrapper-sha" | "--module-sha" => {
                let value = match inline {
                    Some(v) => v,
                    None => {
                        i += 1;
                        match args.get(i) {
                            Some(v) => v.clone(),
                            None => return Err(format!("{name} needs a value")),
                        }
                    }
                };
                match name {
                    "--theorem-id" => o.theorem_id = Some(value),
                    "--proof" => o.proof = Some(value),
                    "--inject" => o.inject = value,
                    "--wrapper-sha" => o.wrapper_sha = Some(value),
                    _ => o.module_sha = Some(value),
                }
            }
            _ => return Err(format!("unrecognized argument: {arg}")),
        }
        i += 1;
    }
    Ok(o)
}

pub fn run(args: &[String]) -> i32 {
    let opts = match parse(args) {
        Ok(o) => o,
        Err(message) => return usage_error(&message),
    };
    if opts.help {
        println!("{USAGE}");
        return 0;
    }
    if opts.self_test {
        return self_test();
    }

    let mut missing = Vec::new();
    if opts.theorem_id.is_none() {
        missing.push("--theorem-id");
    }
    if opts.proof.is_none() {
        missing.push("--proof");
    }
    if opts.wrapper_sha.is_none() {
        missing.push("--wrapper-sha");
    }
    if opts.module_sha.is_none() {
        missing.push("--module-sha");
    }
    if !missing.is_empty() {
        return usage_error(&format!(
            "missing required option(s): {}",
            missing.join(", ")
        ));
    }

    let proof = opts.proof.unwrap_or_default();
    let proof_text = match fs::read(&proof) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("attempt_id: error: {e}");
            return 1;
        }
    };

    println!(
        "{}",
        compute_attempt_id(
            opts.theorem_id.as_deref().unwrap_or_default(),
            &proof_text,
            &opts.inject,
            opts.wrapper_sha.as_deref().unwrap_or_default(),
            opts.module_sha.as_deref().unwrap_or_default(),
        )
    );
    0
}
