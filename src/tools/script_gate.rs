//! Closed-grammar gate for emitted proof scripts (mentorship finding F1).
//!
//! Parses a candidate with the real OCaml front end through HOL Light's own
//! quotation preprocessor and refuses dangerous constructs. Token fallback when
//! no HOL switch is present. **Not on the verdict path.**
//!
//! Exit: 0 allowed, 1 refused, 2 parse error, 3 tool unavailable.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const KERNEL_NAMES: &[&str] = &[
    "prove",
    "axioms",
    "check_axioms",
    "new_axiom",
    "new_definition",
    "mk_thm",
    "mk_axiom",
    "UNDISCH",
    "TAUT",
    "GEN",
    "INST",
    "EQ_MP",
];
const DENY_PREFIX: &[&str] = &["Obj", "Marshal", "Unix"];
const DENY_EXACT: &[&str] = &["Sys.command"];
const STRICT_PREFIX: &[&str] = &["Sys"];
const STRICT_EXACT: &[&str] = &[
    "ref",
    ":=",
    "open_in",
    "open_out",
    "input_line",
    "output_string",
    "really_input",
    "seek_in",
    "close_in",
    "close_out",
    "open_in_bin",
    "open_out_bin",
];

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'\''
}

/// True if `word` occurs in `text` delimited by non-word bytes.
fn contains_word(text: &str, word: &str) -> bool {
    let b = text.as_bytes();
    let w = word.as_bytes();
    let mut i = 0;
    while let Some(p) = b[i..].windows(w.len()).position(|x| x == w) {
        let at = i + p;
        let before_ok = at == 0 || !is_word_byte(b[at - 1]);
        let after = at + w.len();
        let after_ok = after >= b.len() || !is_word_byte(b[after]);
        if before_ok && after_ok {
            return true;
        }
        i = at + 1;
        if i >= b.len() {
            break;
        }
    }
    false
}

fn quoted_after<'a>(text: &'a str, marker: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find(marker) {
        let after = &rest[at + marker.len()..];
        if let Some(end) = after.find('"') {
            out.push(&after[..end]);
            rest = &after[end..];
        } else {
            break;
        }
    }
    out
}

fn scan_tree(tree: &str, strict: bool) -> Vec<String> {
    let mut bad = Vec::new();
    for ident in quoted_after(tree, "Pexp_ident \"") {
        let head = ident.split('.').next().unwrap_or(ident);
        if DENY_PREFIX.contains(&head) || DENY_EXACT.contains(&ident) {
            bad.push(format!("forbidden identifier {ident}"));
        }
        if strict && (STRICT_PREFIX.contains(&head) || STRICT_EXACT.contains(&ident)) {
            bad.push(format!("strict-profile identifier {ident}"));
        }
    }
    for name in quoted_after(tree, "Ppat_var \"") {
        if KERNEL_NAMES.contains(&name) {
            bad.push(format!("rebinding of kernel name {name}"));
        }
    }
    if tree.contains("Pstr_primitive") {
        bad.push("external declaration".to_string());
    }
    bad.sort();
    bad.dedup();
    bad
}

fn scan_tokens(text: &str, strict: bool) -> Vec<String> {
    let mut bad = Vec::new();
    for p in DENY_PREFIX {
        if contains_word(text, p) {
            bad.push(format!("forbidden token {p}"));
        }
    }
    for t in DENY_EXACT {
        if text.contains(t) {
            bad.push(format!("forbidden token {t}"));
        }
    }
    for line in text.lines() {
        if line.trim_start().starts_with("external ") {
            bad.push("external declaration".to_string());
            break;
        }
    }
    for name in KERNEL_NAMES {
        for line in text.lines() {
            let l = line.trim_start();
            for lead in ["let ", "let rec "] {
                if let Some(rest) = l.strip_prefix(lead) {
                    if rest.starts_with(&format!("{name} "))
                        || rest.starts_with(&format!("{name}="))
                    {
                        bad.push(format!("rebinding of kernel name {name}"));
                    }
                }
            }
            if l.contains(&format!("({name} :")) {
                bad.push(format!("rebinding of kernel name {name} (parameter)"));
            }
        }
    }
    if strict {
        for p in STRICT_PREFIX {
            if text.contains(&format!("{p}.")) {
                bad.push(format!("strict-profile token {p}"));
            }
        }
        for t in STRICT_EXACT {
            if contains_word(text, t) {
                bad.push(format!("strict-profile token {t}"));
            }
        }
    }
    bad.sort();
    bad.dedup();
    bad
}

fn hol_env(hol: &Path) -> Vec<(String, String)> {
    let mut env: Vec<(String, String)> = std::env::vars().collect();
    let bin = hol.join("_opam/bin");
    let lib = hol.join("_opam/lib");
    let path = env
        .iter()
        .find(|(k, _)| k == "PATH")
        .map(|(_, v)| v.clone())
        .unwrap_or_default();
    let new_path = format!("{}:{}", bin.display(), path);
    let new_ld = format!(
        "{}:{}:{}",
        lib.join("stublibs").display(),
        lib.join("ocaml/stublibs").display(),
        lib.join("ocaml").display()
    );
    env.retain(|(k, _)| k != "PATH" && k != "CAML_LD_LIBRARY_PATH");
    env.push(("PATH".into(), new_path));
    env.push(("CAML_LD_LIBRARY_PATH".into(), new_ld));
    env
}

fn parsetree(text: &str, as_expr: bool, hol: &Path) -> Result<String, String> {
    let dir = std::env::temp_dir().join(format!("tek-gate-{}", std::process::id()));
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let src = dir.join("candidate.ml");
    let body = if as_expr {
        format!("let _tek_candidate = (\n{text}\n);;\n")
    } else {
        text.to_string()
    };
    fs::write(&src, body).map_err(|e| e.to_string())?;

    let pp = Command::new(hol.join("hol.sh"))
        .arg("-pp")
        .output()
        .map_err(|e| format!("hol.sh: {e}"))?;
    let pp = String::from_utf8_lossy(&pp.stdout).trim().to_string();
    if pp.is_empty() {
        return Err("hol.sh -pp produced nothing".into());
    }
    let out = Command::new("ocamlc")
        .arg("-pp")
        .arg(&pp)
        .args(["-stop-after", "parsing", "-dparsetree", "-impl"])
        .arg(&src)
        .env_clear()
        .envs(hol_env(hol))
        .output()
        .map_err(|e| format!("ocamlc: {e}"))?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = fs::remove_dir_all(&dir);
    if !out.status.success() {
        return Err(format!("parse failed:\n{text}"));
    }
    Ok(text)
}

fn arg_value<'a>(args: &'a [String], key: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == key)
        .and_then(|i| args.get(i + 1))
        .map(String::as_str)
}

pub fn run(args: &[String]) -> i32 {
    let Some(path) = args
        .iter()
        .find(|a| !a.starts_with("--") && Path::new(a).is_file())
    else {
        eprintln!("usage: script_gate <file> [--expr] [--profile denylist|strict] [--hol-light DIR] [--tokens-only]");
        return 3;
    };
    let as_expr = args.iter().any(|a| a == "--expr");
    let strict = arg_value(args, "--profile") == Some("strict");
    let tokens_only = args.iter().any(|a| a == "--tokens-only");
    let hol = PathBuf::from(
        arg_value(args, "--hol-light")
            .unwrap_or(concat!(env!("CARGO_MANIFEST_DIR"), "/.bench/hol-light")),
    );

    let Ok(text) = fs::read_to_string(path) else {
        eprintln!("ERROR cannot read {path}");
        return 3;
    };

    let (bad, mode) = if !tokens_only && hol.join("hol.sh").is_file() {
        match parsetree(&text, as_expr, &hol) {
            Ok(tree) => (scan_tree(&tree, strict), "front-end"),
            Err(e) if e.starts_with("parse failed") => {
                println!("REFUSED parse  {path}");
                println!("{e}");
                return 2;
            }
            Err(_) => (scan_tokens(&text, strict), "tokens(no ocamlc)"),
        }
    } else {
        (scan_tokens(&text, strict), "tokens(no HOL)")
    };

    let profile = if strict { "strict" } else { "denylist" };
    if bad.is_empty() {
        println!("ALLOWED {profile} [{mode}]  {path}");
        0
    } else {
        println!("REFUSED {profile} [{mode}]  {path}");
        for b in bad.iter().take(20) {
            println!("  {b}");
        }
        if bad.len() > 20 {
            println!("  ... {} more", bad.len() - 20);
        }
        1
    }
}
