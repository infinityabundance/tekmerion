//! XIX.B3: the committed emitted answer must be accepted by the real OCaml
//! front end.  A Rust suite cannot notice that HOL Light refuses to parse an
//! emitted answer; this gate runs `ocamlc` itself, and a missing `ocamlc` is a
//! **failure**, never a skip.
//!
//! Two arms:
//!
//! 1. Always, offline, in CI: `ocamlc -stop-after parsing -i` on the committed
//!    answer, with HOL Light quotations replaced by OCaml string literals.
//!    Quotations are opaque to OCaml anyway (HOL's own preprocessor `pa_j.cmo`
//!    turns them into terms), so this exercises every byte that OCaml parses:
//!    the class of defect that broke the library — camlp5r rejecting a
//!    polymorphic variant, a stray token — fails here.
//!
//! 2. When this host has the bench's HOL Light (`_opam` + `hol.sh` + a combined
//!    template), also run the bench's own full elaboration
//!    (`ocamlc -pp "$(hol.sh -pp)" -I <hol-light> -i`) over the combined file.
//!    The committed answer alone cannot be `-i`-typechecked without the bench
//!    template: it names `BITBLAST_THEN` and other helpers defined by the
//!    template prefix, and the fragment has no way to bind them.  The combined
//!    file is the bench's artefact, not a repository input, so CI cannot have
//!    it; the always-on arm is the gate.
//!
//! Why the answer is preprocessed rather than passed raw: HOL Light source is
//! not stock OCaml syntax.  It uses backtick quotations (opaque to OCaml —
//! HOL's own preprocessor `pa_j.cmo` turns them into terms) and upper-case
//! value names (`let rec GCONJUNCTS th acc = ...`), which camlp5r accepts and
//! the OCaml grammar does not.  The gate applies exactly two *lexical*
//! normalisations — quotations to string literals, upper-case value names to
//! lower case — and then runs the real parser.  It does not repair structure,
//! so a stray backtick (the XVI.D polymorphic-variant regression, which broke
//! every emitted answer) or a malformed phrase still fails.  What it cannot
//! check offline is the *content* of a well-formed quotation; that requires
//! HOL Light's own preprocessor, which is the optional arm below.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const ANSWER: &str = "tests/fixtures/answers/arm/bignum_add_p25519.answer.txt";

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn runs(tool: &Path) -> Option<String> {
    let out = Command::new(tool).arg("-version").output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string(),
    )
}

/// The `ocamlc` used for the parse gate: the system one (CI installs
/// `ocaml-nox`), falling back to the bench switch on a host that has one.
fn parse_ocamlc() -> (PathBuf, String) {
    let system = PathBuf::from("ocamlc");
    if let Some(version) = runs(&system) {
        return (system, version);
    }
    let switch = manifest_dir().join(".bench/hol-light/_opam/bin/ocamlc");
    if let Some(version) = runs(&switch) {
        return (switch, version);
    }
    panic!(
        "XIX.B3: ocamlc is required for the answer-elaboration gate, but neither \
         `ocamlc` on PATH nor `.bench/hol-light/_opam/bin/ocamlc` runs. \
         CI installs `ocaml-nox`; a local host can install OCaml too. \
         This is a failure by design: a missing front end must not look like a pass."
    )
}

/// Replace every HOL Light quotation (backtick-delimited, outside OCaml string
/// literals) with an equivalent OCaml string literal.  Returns the rewritten
/// text and the number of quotations replaced.  An unterminated quotation is
/// an error: the committed answer must not contain one.
pub fn neutralize_quotations(src: &str) -> Result<(String, usize), String> {
    let bytes = src.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut count = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'(' if bytes[i..].starts_with(b"(*") => {
                // Copy comments verbatim: ocamlc strips them itself.  Skipping
                // them here matters because comment prose contains quotes and
                // apostrophes that are not OCaml strings; treating one as a
                // string would swallow real quotations.
                out.extend_from_slice(b"(*");
                let mut depth = 1usize;
                i += 2;
                while i < bytes.len() && depth > 0 {
                    if bytes[i..].starts_with(b"(*") {
                        depth += 1;
                        out.extend_from_slice(b"(*");
                        i += 2;
                    } else if bytes[i..].starts_with(b"*)") {
                        depth -= 1;
                        out.extend_from_slice(b"*)");
                        i += 2;
                    } else {
                        out.push(bytes[i]);
                        i += 1;
                    }
                }
            }
            b'"' => {
                // Copy the OCaml string verbatim, escapes included.
                out.push(bytes[i]);
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' {
                    if bytes[i] == b'\\' && i + 1 < bytes.len() {
                        out.push(bytes[i]);
                        out.push(bytes[i + 1]);
                        i += 2;
                    } else {
                        out.push(bytes[i]);
                        i += 1;
                    }
                }
                if i == bytes.len() {
                    return Err("unterminated OCaml string literal in the answer".into());
                }
                out.push(bytes[i]);
                i += 1;
            }
            b'`' => {
                let start = i + 1;
                let mut end = start;
                while end < bytes.len() && bytes[end] != b'`' {
                    end += 1;
                }
                if end == bytes.len() {
                    return Err(format!(
                        "unterminated HOL quotation starting at byte {i} in the answer"
                    ));
                }
                out.push(b'"');
                for &b in &bytes[start..end] {
                    match b {
                        b'\\' | b'"' => {
                            out.push(b'\\');
                            out.push(b);
                        }
                        b'\n' | b'\r' | b'\t' => out.push(b),
                        _ => out.push(b),
                    }
                }
                out.push(b'"');
                count += 1;
                i = end + 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    let text = String::from_utf8(out).map_err(|e| format!("answer is not UTF-8: {e}"))?;
    Ok((text, count))
}

/// Identifiers the parse gate must keep upper case.
///
/// Two groups: (a) OCaml/HOL constructors, which are legal in patterns and
/// cannot be lowercased; (b) upper-case names whose lower-case spelling is an
/// OCaml keyword (`THEN` is HOL's tactic combinator; `then` is syntax), which
/// cannot be lowercased either.  Qualified names (`List.map`, `Sys.Break`) are
/// preserved by the `.` rule below, not by this list.
const KEEP_UPPER: &[&str] = &[
    // term and type constructors
    "Comb",
    "Const",
    "Var",
    "Abs",
    "Tyapp",
    "Tyvar",
    // option/result/list and standard exceptions
    "Some",
    "None",
    "Ok",
    "Error",
    "Result",
    "Cons",
    "Nil",
    "Inl",
    "Inr",
    "Left",
    "Right",
    "Failure",
    "Not_found",
    "Sys_error",
    "Match_failure",
    "Exit",
    "Invalid_argument",
    "Division_by_zero",
    "End_of_file",
    "Stack_overflow",
    "Out_of_memory",
    "Break",
    "Signal_handle",
    "EQ",
    "LT",
    "GT",
];

/// OCaml keywords.  An upper-case name whose lower-case form is one of these is
/// kept as-is; `THEN` is the only one that occurs as a value in the answer, and
/// it parses as an ordinary (upper-case) identifier in infix position.
const OCAML_KEYWORDS: &[&str] = &[
    "and",
    "as",
    "assert",
    "asr",
    "begin",
    "class",
    "constraint",
    "do",
    "done",
    "downto",
    "else",
    "end",
    "exception",
    "external",
    "false",
    "for",
    "fun",
    "function",
    "functor",
    "if",
    "in",
    "include",
    "inherit",
    "initializer",
    "land",
    "lazy",
    "let",
    "lor",
    "lsl",
    "lsr",
    "lxor",
    "match",
    "method",
    "mod",
    "module",
    "mutable",
    "new",
    "nonrec",
    "object",
    "of",
    "open",
    "or",
    "private",
    "rec",
    "sig",
    "struct",
    "then",
    "to",
    "true",
    "try",
    "type",
    "val",
    "virtual",
    "when",
    "while",
    "with",
];

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'\''
}

/// Lower-case HOL Light's upper-case value names so that the stock OCaml
/// grammar can parse the answer.
///
/// HOL Light writes `let rec GCONJUNCTS th acc = ...` and applies upper-case
/// functions to several arguments; the OCaml grammar reads an upper-case
/// identifier as a constructor and refuses both.  This pass lower-cases every
/// upper-case identifier except constructors, keyword collisions and names
/// that are part of a dotted path.  It is a lexical normalisation only: no
/// structure is repaired, and any malformed code stays malformed.  Comments
/// and string literals are copied verbatim.  Returns the text and the number of
/// identifiers lowered.
pub fn lower_hol_identifiers(src: &str) -> (String, usize) {
    let bytes = src.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut lowered = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i..].starts_with(b"(*") {
            let mut depth = 1usize;
            out.extend_from_slice(b"(*");
            i += 2;
            while i < bytes.len() && depth > 0 {
                if bytes[i..].starts_with(b"(*") {
                    depth += 1;
                    out.extend_from_slice(b"(*");
                    i += 2;
                } else if bytes[i..].starts_with(b"*)") {
                    depth -= 1;
                    out.extend_from_slice(b"*)");
                    i += 2;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
        } else if bytes[i] == b'"' {
            out.push(bytes[i]);
            i += 1;
            while i < bytes.len() && bytes[i] != b'"' {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    out.push(bytes[i]);
                    out.push(bytes[i + 1]);
                    i += 2;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            if i < bytes.len() {
                out.push(bytes[i]);
                i += 1;
            }
        } else if bytes[i].is_ascii_uppercase() {
            let start = i;
            let mut end = i;
            while end < bytes.len() && is_ident_byte(bytes[end]) {
                end += 1;
            }
            let word = &src[start..end];
            let prev_is_dot = start > 0 && bytes[start - 1] == b'.';
            let next_is_dot = end < bytes.len() && bytes[end] == b'.';
            let lower = word.to_ascii_lowercase();
            let keep = prev_is_dot
                || next_is_dot
                || KEEP_UPPER.contains(&word)
                || OCAML_KEYWORDS.contains(&lower.as_str());
            if keep {
                out.extend_from_slice(word.as_bytes());
            } else {
                out.extend_from_slice(lower.as_bytes());
                lowered += 1;
            }
            i = end;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    (String::from_utf8(out).unwrap_or_default(), lowered)
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tek-answer-{}-{tag}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

fn first_lines(text: &str, n: usize) -> String {
    text.lines().take(n).collect::<Vec<_>>().join("\n")
}

/// The bench's HOL Light environment (PATH for camlp5r, library path for the
/// switch's stubs).  Only used by the optional full-elaboration arm.
fn hol_env() -> Vec<(String, String)> {
    let hol = manifest_dir().join(".bench/hol-light");
    let mut env: Vec<(String, String)> = std::env::vars().collect();
    let path = env
        .iter()
        .find(|(k, _)| k == "PATH")
        .map(|(_, v)| v.clone())
        .unwrap_or_default();
    let stublibs = format!(
        "{}:{}:{}",
        hol.join("_opam/lib/stublibs").display(),
        hol.join("_opam/lib/ocaml/stublibs").display(),
        hol.join("_opam/lib/ocaml").display()
    );
    env.retain(|(k, _)| k != "PATH" && k != "CAML_LD_LIBRARY_PATH");
    env.push((
        "PATH".into(),
        format!("{}:{}", hol.join("_opam/bin").display(), path),
    ));
    env.push(("CAML_LD_LIBRARY_PATH".into(), stublibs));
    env
}

/// Newest `eval-*/ml_files_arm_bignum_add_p25519.ml` under `.bench`, if any.
fn bench_combined_task() -> Option<PathBuf> {
    let bench = manifest_dir().join(".bench");
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in fs::read_dir(&bench).ok()?.flatten() {
        let name = entry.file_name();
        if !name.to_string_lossy().starts_with("eval-") {
            continue;
        }
        let candidate = entry.path().join("ml_files_arm_bignum_add_p25519.ml");
        if let Ok(meta) = fs::metadata(&candidate) {
            if let Ok(modified) = meta.modified() {
                if best.as_ref().map(|(t, _)| modified > *t).unwrap_or(true) {
                    best = Some((modified, candidate));
                }
            }
        }
    }
    best.map(|(_, p)| p)
}

#[test]
fn the_committed_answer_parses_with_the_real_ocaml_front_end() {
    let (ocamlc, version) = parse_ocamlc();
    let answer_path = manifest_dir().join(ANSWER);
    let answer = fs::read_to_string(&answer_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", answer_path.display()));

    let (text, quotations) = neutralize_quotations(&answer).expect("neutralise quotations");
    assert!(
        quotations > 0,
        "the answer contains no HOL quotation; the gate would be vacuous"
    );
    let (text, lowered) = lower_hol_identifiers(&text);

    let dir = scratch("parse");
    let file = dir.join("answer.ml");
    fs::write(&file, &text).expect("write neutralised answer");

    let out = Command::new(&ocamlc)
        .args(["-stop-after", "parsing", "-i"])
        .arg(&file)
        .output()
        .unwrap_or_else(|e| panic!("cannot run {}: {e}", ocamlc.display()));
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let _ = fs::remove_dir_all(&dir);

    eprintln!(
        "ocamlc: {} (version {version})\nanswer: {} ({} lines, {quotations} quotations, \
         {lowered} upper-case identifiers lowered)",
        ocamlc.display(),
        answer_path.display(),
        answer.lines().count()
    );
    eprintln!("ocamlc -stop-after parsing -i: exit {}", out.status);
    if !stdout.trim().is_empty() {
        eprintln!("first lines of output:\n{}", first_lines(&stdout, 5));
    }
    assert!(
        out.status.success(),
        "the committed answer does not parse with the real OCaml front end \
         ({}):\n{stderr}{stdout}",
        ocamlc.display()
    );

    // Optional full arm: the bench's own check, where the bench exists.
    let hol = manifest_dir().join(".bench/hol-light");
    let switch_ocamlc = hol.join("_opam/bin/ocamlc");
    let Some(combined) = bench_combined_task() else {
        eprintln!(
            "full elaboration: not run (no .bench/eval-*/combined task on this host; \
             the parse gate above is the CI gate)"
        );
        return;
    };
    if !hol.join("hol.sh").is_file() || !switch_ocamlc.is_file() {
        eprintln!("full elaboration: not run (HOL Light switch absent)");
        return;
    }
    // Only use a combined file that actually carries this answer's tactic.
    let combined_text = fs::read_to_string(&combined).unwrap_or_default();
    let marker = "BRIDGE_ADD_REF_TAC";
    if !combined_text.contains(marker) {
        eprintln!(
            "full elaboration: not run ({} predates the current answer)",
            combined.display()
        );
        return;
    }
    let pp = Command::new(hol.join("hol.sh"))
        .arg("-pp")
        .output()
        .expect("hol.sh -pp");
    let pp = String::from_utf8_lossy(&pp.stdout).trim().to_string();
    assert!(!pp.is_empty(), "hol.sh -pp produced nothing");
    let full = Command::new(&switch_ocamlc)
        .arg("-pp")
        .arg(&pp)
        .arg("-I")
        .arg(&hol)
        .arg("-i")
        .arg(&combined)
        .env_clear()
        .envs(hol_env())
        .output()
        .expect("full elaboration");
    eprintln!(
        "full elaboration (bench template + answer): exit {}",
        full.status
    );
    assert!(
        full.status.success(),
        "the combined bench task does not elaborate:\n{}",
        String::from_utf8_lossy(&full.stderr)
    );
}

/// XVI.D's regression was a polymorphic-variant value (`\`Closed`) in code.
/// HOL source has no stray backticks, so an odd one is not a quotation and the
/// gate must refuse rather than pair it silently with a later delimiter.
#[test]
fn a_stray_backtick_is_refused() {
    let answer = "let f = `Closed in\nlet T = prove (`!x. x = x`, REWRITE_TAC[]);;\n";
    let err = neutralize_quotations(answer).expect_err("a stray backtick is a failure");
    assert!(err.contains("unterminated HOL quotation"), "{err}");
}

#[test]
fn upper_case_value_names_are_lowered_but_constructors_are_kept() {
    let src = "let rec GCONJUNCTS th acc =\n  match th with\n  | Comb(p,q) -> PROVE_HYP p q\n  | _ -> List.map (fun x -> TAC x) acc\n";
    let (out, n) = lower_hol_identifiers(src);
    assert!(n >= 3, "lowered {n}: {out}");
    assert!(out.contains("let rec gconjuncts"), "{out}");
    assert!(out.contains("prove_hyp"), "{out}");
    assert!(out.contains("tac"), "{out}");
    // Constructors and qualified module names keep their spelling.
    assert!(out.contains("Comb(p,q)"), "{out}");
    assert!(out.contains("List.map"), "{out}");
    // A name whose lower-case form is a keyword is kept (THEN is infix).
    let (out2, _) = lower_hol_identifiers("let t = a THEN b\n");
    assert!(out2.contains("THEN"), "{out2}");
}
