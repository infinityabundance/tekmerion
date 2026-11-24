//! Mechanical V7.3 check over the lemma statements of `tools/bridge.ml`.
//!
//! Port of `research/receipts/bridge/curve_check.py`. Reading one file path, it
//! strips HOL Light block comments, extracts the first backtick-delimited term
//! following every `prove`, and reports any curve name, known reduction shape or
//! numeral of five or more digits. It exits 0 only when no statement hits.
//!
//! The port reproduces the original byte for byte on `tools/bridge.ml`; the
//! golden stdout and exit code are recorded in
//! `research/receipts/slice-d-rust-receipts.md`.
//!
//! **Not on the verdict path.**

use std::fs;

/// Curve-name substrings, matched case-insensitively against the statement.
/// Order is significant: it is the order flags are printed in.
const CURVES: [&str; 12] = [
    "p_256",
    "p_384",
    "p_521",
    "nistp",
    "25519",
    "secp256k1",
    "p256k1",
    "sm2",
    "curve25519",
    "p25519",
    "p384",
    "p521",
];

/// Reduction-shape substrings, matched case-sensitively. Order is significant.
const REDUCTION: [&str; 7] = [
    "+ 19",
    "+19",
    "2 EXP 255",
    "2 EXP 384",
    "2 EXP 521",
    "0x",
    "0X",
];

const USAGE: &str = "\
usage: curve_check <file.ml>

  Strips HOL Light block comments, then reports every `prove` statement that
  mentions a curve name, a reduction shape, or a numeral of 5+ digits.
  Exit status is 0 iff no statement hits.
";

/// `\bprove\b`-delimited ASCII word characters.
fn is_word(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Drop `(* ... *)` blocks, preserving everything else verbatim.
///
/// The scan is byte-wise and removes only ASCII delimiter bytes, so a valid
/// UTF-8 input stays valid UTF-8. Nesting is tracked the same way the Python
/// original tracked it, and — like the original — strings are not treated
/// specially.
pub fn strip_comments(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut depth = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i..].starts_with(b"(*") {
            depth += 1;
            i += 2;
        } else if depth > 0 && bytes[i..].starts_with(b"*)") {
            depth -= 1;
            i += 2;
        } else if depth > 0 {
            i += 1;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    // Only whole ASCII bytes were removed, so the result is still UTF-8.
    String::from_utf8(out).unwrap_or_default()
}

/// `(byte-offset-of-prove, statement)` for each `prove` followed by a pair of
/// backticks.
fn lemma_statements(src: &str) -> Vec<(usize, &str)> {
    let bytes = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + 5 <= bytes.len() {
        let boundary_before = i == 0 || !is_word(bytes[i - 1]);
        let boundary_after = i + 5 == bytes.len() || !is_word(bytes[i + 5]);
        if &bytes[i..i + 5] == b"prove" && boundary_before && boundary_after {
            let rest = &src[i + 5..];
            if let Some(q) = rest.find('`') {
                if let Some(e) = rest[q + 1..].find('`') {
                    out.push((i, &rest[q + 1..q + 1 + e]));
                }
            }
        }
        i += 1;
    }
    out
}

/// Maximal ASCII digit runs of length five or more.
fn numerals(stmt: &str) -> Vec<&str> {
    let bytes = stmt.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i - start >= 5 {
                out.push(&stmt[start..i]);
            }
        } else {
            i += 1;
        }
    }
    out
}

/// The first line of `s`, using Python's `str.splitlines()` boundaries.
fn first_line(s: &str) -> &str {
    for (i, c) in s.char_indices() {
        if matches!(
            c,
            '\n' | '\r'
                | '\u{000b}'
                | '\u{000c}'
                | '\u{001c}'
                | '\u{001d}'
                | '\u{001e}'
                | '\u{0085}'
                | '\u{2028}'
                | '\u{2029}'
        ) {
            return &s[..i];
        }
    }
    s
}

/// Render the report for an already-decoded source.
pub fn check_text(src: &str) -> (String, usize, usize) {
    let stripped = strip_comments(src);
    let mut text = String::new();
    let mut hits = 0usize;
    let mut statements = 0usize;

    for (offset, stmt) in lemma_statements(&stripped) {
        statements += 1;
        let line = stripped.as_bytes()[..offset]
            .iter()
            .filter(|&&b| b == b'\n')
            .count()
            + 1;

        let mut flags: Vec<String> = Vec::new();
        let low = stmt.to_lowercase();
        for curve in CURVES {
            if low.contains(curve) {
                flags.push(format!("curve:{curve}"));
            }
        }
        for shape in REDUCTION {
            if stmt.contains(shape) {
                flags.push(format!("shape:{shape}"));
            }
        }
        for num in numerals(stmt) {
            let trimmed: String = num.chars().take(24).collect();
            flags.push(format!("numeral:{trimmed}"));
        }

        if flags.is_empty() {
            text.push_str(&format!("line {line}: CLEAN\n"));
        } else {
            text.push_str(&format!("line {line}: HIT {}\n", flags.join(", ")));
            hits += 1;
        }

        let first: String = first_line(stmt.trim()).chars().take(140).collect();
        text.push_str("  ");
        text.push_str(&first);
        text.push('\n');
    }

    text.push_str(&format!("statements={statements} hits={hits}\n"));
    (text, statements, hits)
}

/// CLI entry point. `args` excludes the program name.
pub fn run(args: &[String]) -> i32 {
    if args.len() == 1 && matches!(args[0].as_str(), "-h" | "--help") {
        print!("{USAGE}");
        return 0;
    }
    if args.len() != 1 {
        eprint!("{USAGE}");
        return 2;
    }
    let src = match fs::read_to_string(&args[0]) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ERROR {}: {e}", args[0]);
            return 2;
        }
    };
    let (text, _statements, hits) = check_text(&src);
    print!("{text}");
    if hits == 0 {
        0
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comments_are_stripped_before_scanning() {
        let src = "(* curve p_256 with 123456 *)\nlet T = prove\n (`!x. x + 0 = x`,\n  REWRITE_TAC[]);;\n";
        let (text, statements, hits) = check_text(src);
        assert_eq!(statements, 1);
        assert_eq!(hits, 0, "{text}");
        assert!(text.contains("line 2: CLEAN"), "{text}");
        assert!(text.contains("statements=1 hits=0"), "{text}");
    }

    #[test]
    fn a_curve_name_in_a_statement_is_a_hit() {
        let src = "let T = prove (`!x. x MOD p_256 = x`, REWRITE_TAC[]);;\n";
        let (text, _statements, hits) = check_text(src);
        assert_eq!(hits, 1);
        assert!(text.contains("HIT curve:p_256"), "{text}");
    }

    #[test]
    fn numerals_of_five_digits_are_flagged_and_truncated() {
        let src = "let T = prove (`!x. x = 123456789012345678901234567890123`, REWRITE_TAC[]);;\n";
        let (text, _statements, hits) = check_text(src);
        assert_eq!(hits, 1);
        assert!(text.contains("numeral:123456789012345678901234"), "{text}");
    }

    #[test]
    fn prove_not_at_a_word_boundary_is_ignored() {
        let src = "let reprove = 1;;\nlet T = prove (`!x. x = x`, REWRITE_TAC[]);;\n";
        let (text, statements, hits) = check_text(src);
        assert_eq!(statements, 1);
        assert_eq!(hits, 0);
        assert!(text.contains("statements=1 hits=0"), "{text}");
    }

    #[test]
    fn nested_comments_are_removed() {
        let src = "a (* outer (* inner *) still *) b";
        assert_eq!(strip_comments(src), "a  b");
    }
}
