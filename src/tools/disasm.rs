//! Turn an AArch64 or x86-64 ELF object into a step list.
//!
//! The step list is the sole input to [`crate::tools::derive::derive_accumulate`].
//! Nothing here reads a proof file or a tactic script: the bytes come from the
//! object file that the proof's `define_assert_from_elf` binds, and the text
//! comes from `objdump`.
//!
//! AArch64 and x86 differ in two ways that matter to the step numbering:
//!
//!  * s2n-bignum's x86 proofs bind a *trimmed* machine code constant
//!    (`define_trimmed` = `TRIM_LIST(4,0)`), which drops the four-byte
//!    `ENDBR64` prefix. The x86 path therefore requires that prefix, drops it
//!    and reports addresses relative to the trimmed constant. A code object
//!    without it is refused rather than silently numbered against the wrong
//!    base.
//!  * an x86 `ensures` precondition may enter the code after a prologue, e.g.
//!    `read RIP s = word (pc + 3)`. [`enter_at`] keeps only the instructions at
//!    or after that entry and renumbers them from 1.
//!
//! The x86 disassembly is Intel syntax (`objdump -M intel`) so that operands
//! read destination-first, matching the operand indices in `derive`.

use crate::tools::derive::Step;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_OBJDUMP_NAMES_ARM: &[&str] =
    &["aarch64-linux-gnu-objdump", "llvm-objdump", "objdump"];
const DEFAULT_OBJDUMP_NAMES_X86: &[&str] = &["objdump", "llvm-objdump", "x86_64-linux-gnu-objdump"];

// ELF e_machine values we can name.
const EM_386: u16 = 0x03;
const EM_X86_64: u16 = 0x3E;
const EM_AARCH64: u16 = 0xB7;

/// `define_trimmed` in s2n-bignum's x86 proofs is `TRIM_LIST(4,0)`.
pub const X86_TRIM_BYTES: i64 = 4;
const X86_ENDBR64: [u8; 4] = [0xf3, 0x0f, 0x1e, 0xfa];

fn arch_by_machine(machine: u16) -> Option<&'static str> {
    match machine {
        EM_X86_64 | EM_386 => Some("x86"),
        EM_AARCH64 => Some("arm"),
        _ => None,
    }
}

/// Return `"x86"`/`"arm"` from the ELF header, or `None`.
pub fn elf_arch(path: &Path) -> Option<&'static str> {
    let head = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(_) => return None,
    };
    if head.len() < 20 || &head[..4] != b"\x7fELF" {
        return None;
    }
    if head[5] != 1 {
        // EI_DATA: only little-endian objects are used here.
        return None;
    }
    let machine = u16::from_le_bytes([head[18], head[19]]);
    arch_by_machine(machine)
}

/// The four bytes an x86 trimmed constant starts with, for callers that need
/// to check the prefix themselves.
pub fn x86_endbr64() -> &'static [u8] {
    &X86_ENDBR64
}

fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        match fs::metadata(path) {
            Ok(m) => m.is_file() && (m.permissions().mode() & 0o111) != 0,
            Err(_) => false,
        }
    }
    #[cfg(not(unix))]
    {
        path.is_file()
    }
}

/// A `shutil.which` equivalent: an explicit path is checked directly, a bare
/// name is searched on `PATH`.
fn which(name: &str) -> Option<PathBuf> {
    if name.contains('/') {
        let p = PathBuf::from(name);
        return if is_executable(&p) { Some(p) } else { None };
    }
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if is_executable(&candidate) {
            return Some(candidate);
        }
    }
    None
}

fn find_objdump(explicit: Option<&str>, arch: &str) -> Result<Vec<String>, String> {
    let defaults = match arch {
        "arm" => DEFAULT_OBJDUMP_NAMES_ARM,
        "x86" => DEFAULT_OBJDUMP_NAMES_X86,
        other => return Err(format!("disasm_steps: unknown architecture {other:?}")),
    };
    let candidates: Vec<&str> = match explicit {
        Some(name) => vec![name],
        None => defaults.to_vec(),
    };
    for name in &candidates {
        if name.is_empty() {
            continue;
        }
        if let Some(path) = which(name) {
            return Ok(vec![path.display().to_string()]);
        }
    }
    let tried: Vec<&str> = candidates
        .iter()
        .copied()
        .filter(|c| !c.is_empty())
        .collect();
    Err(format!(
        "disasm_steps: no {arch} objdump found; pass --objdump PATH (tried: {})",
        tried.join(", ")
    ))
}

/// Split an operand list on commas that are not inside `[...]`.
pub fn split_operands(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut depth: usize = 0;
    let mut current = String::new();
    for ch in text.chars() {
        match ch {
            '[' => depth += 1,
            ']' => depth = depth.saturating_sub(1),
            _ => {}
        }
        if ch == ',' && depth == 0 {
            out.push(current.trim().to_string());
            current.clear();
        } else {
            current.push(ch);
        }
    }
    let tail = current.trim();
    if !tail.is_empty() {
        out.push(tail.to_string());
    }
    out.into_iter().map(|o| o.to_lowercase()).collect()
}

fn parse_line(line: &str) -> Option<(i64, String)> {
    // ^\s*([0-9a-fA-F]+):\s+(.*?)\s*$
    let trimmed = line.trim_start();
    let mut hex_len = 0;
    for b in trimmed.bytes() {
        if b.is_ascii_hexdigit() {
            hex_len += 1;
        } else {
            break;
        }
    }
    if hex_len == 0 {
        return None;
    }
    let rest = &trimmed[hex_len..];
    let rest = rest.strip_prefix(':')?;
    let after = rest.trim_start_matches(|c: char| c.is_whitespace());
    if after.len() == rest.len() {
        // The pattern requires at least one whitespace character after ':'.
        return None;
    }
    let address = i64::from_str_radix(&trimmed[..hex_len], 16).ok()?;
    Some((address, after.trim_end().to_string()))
}

fn split_instruction(body: &str) -> Option<(String, Option<String>)> {
    // ^([a-z][a-z0-9._]*)(?:\s+(.*))?$
    let first = *body.as_bytes().first()?;
    if !first.is_ascii_lowercase() {
        return None;
    }
    let mut end = 1;
    for b in body.bytes().skip(1) {
        if b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_') {
            end += 1;
        } else {
            break;
        }
    }
    let mnemonic = body[..end].to_string();
    if end == body.len() {
        return Some((mnemonic, None));
    }
    let tail = &body[end..];
    if !tail.starts_with(|c: char| c.is_whitespace()) {
        return None;
    }
    Some((mnemonic, Some(tail.trim().to_string())))
}

/// Parse `objdump -d` output into [`Step`] records, in program order.
pub fn parse_disassembly(text: &str) -> Vec<Step> {
    let mut steps: Vec<Step> = Vec::new();
    for line in text.lines() {
        let (address, raw_body) = match parse_line(line) {
            Some(v) => v,
            None => continue,
        };
        // Drop objdump's trailing comment, including the "// #123" hint.
        let body = match raw_body.split_once("//") {
            Some((head, _)) => head.trim(),
            None => raw_body.as_str(),
        };
        if body.is_empty() {
            continue;
        }
        let (mnemonic, operand_text) = match split_instruction(body) {
            Some(v) => v,
            None => continue,
        };
        let operands = match operand_text {
            Some(t) if !t.is_empty() => split_operands(&t),
            _ => Vec::new(),
        };
        steps.push(Step {
            index: steps.len() + 1,
            mnemonic: mnemonic.to_lowercase(),
            operands,
            raw: body.to_string(),
            address,
        });
    }
    steps
}

fn renumber(steps: Vec<Step>) -> Vec<Step> {
    steps
        .into_iter()
        .enumerate()
        .map(|(i, s)| Step {
            index: i + 1,
            mnemonic: s.mnemonic,
            operands: s.operands,
            raw: s.raw,
            address: s.address,
        })
        .collect()
}

/// Drop the four-byte `ENDBR64` that `define_trimmed` removes.
///
/// The returned addresses are relative to the trimmed machine code constant (so
/// the first real instruction is at address 0). Fail closed when the object
/// does not start with exactly that prefix: trimming a different number of
/// bytes would shift every step number without any check.
pub fn trim_x86_prefix(steps: Vec<Step>, path: &str) -> Result<Vec<Step>, String> {
    let first = match steps.first() {
        Some(s) => s,
        None => {
            return Err(format!(
                "disasm_steps: no instructions in {}",
                display(path)
            ))
        }
    };
    let next_address = steps.get(1).map(|s| s.address);
    if first.mnemonic != "endbr64" || first.address != 0 || next_address != Some(X86_TRIM_BYTES) {
        return Err(format!(
            "disasm_steps: x86 object must start with a 4-byte endbr64 (found {:?} at {}, next at {}); {} is not a trimmed x86 s2n-bignum code constant",
            first.mnemonic,
            first.address,
            match next_address {
                Some(a) => a.to_string(),
                None => "None".to_string(),
            },
            display(path)
        ));
    }
    let shifted: Vec<Step> = steps
        .into_iter()
        .skip(1)
        .map(|s| Step {
            index: s.index,
            mnemonic: s.mnemonic,
            operands: s.operands,
            raw: s.raw,
            address: s.address - X86_TRIM_BYTES,
        })
        .collect();
    Ok(renumber(shifted))
}

fn display(path: &str) -> &str {
    if path.is_empty() {
        "object"
    } else {
        path
    }
}

/// Keep instructions at or after `entry` (a trimmed-code byte offset).
///
/// `X86_STEPS_TAC` starts numbering at the instruction the precondition's
/// `read RIP` names, so a prologue before that entry is not simulated and must
/// not occupy a step number. The entry must land on an instruction boundary;
/// otherwise the numbering cannot be produced and we refuse.
pub fn enter_at(steps: Vec<Step>, entry: i64) -> Result<Vec<Step>, String> {
    if entry < 0 {
        return Err(format!("disasm_steps: negative entry offset {entry}"));
    }
    let kept: Vec<Step> = steps.into_iter().filter(|s| s.address >= entry).collect();
    let first_address = match kept.first() {
        Some(s) => s.address,
        None => {
            return Err(format!(
                "disasm_steps: no instruction at or after entry {entry}"
            ))
        }
    };
    if first_address != entry {
        return Err(format!(
            "disasm_steps: entry {entry} is not an instruction boundary (first kept instruction is at {first_address})"
        ));
    }
    Ok(renumber(kept))
}

/// Disassemble `path` into a step list.
///
/// `arch` is read from the ELF header when `None`; a mismatch between the
/// header and an explicit `arch` is refused.
pub fn disassemble(
    path: &Path,
    objdump: Option<&str>,
    arch: Option<&str>,
) -> Result<Vec<Step>, String> {
    if !path.is_file() {
        return Err(format!("disasm_steps: not a file: {}", path.display()));
    }
    let detected = elf_arch(path);
    let resolved = match arch {
        Some(a) => Some(a.to_string()),
        None => detected.map(str::to_string),
    };
    let arch = match resolved {
        Some(a) => a,
        None => {
            return Err(format!(
                "disasm_steps: cannot detect ELF architecture of {}; pass --arch arm|x86",
                path.display()
            ))
        }
    };
    if let Some(d) = detected {
        if d != arch {
            return Err(format!(
                "disasm_steps: {} is a {d} object but --arch {arch} was requested",
                path.display()
            ));
        }
    }
    let mut cmd = find_objdump(objdump, &arch)?;
    cmd.push("-d".to_string());
    cmd.push("--no-show-raw-insn".to_string());
    if arch == "x86" {
        cmd.push("-M".to_string());
        cmd.push("intel".to_string());
    }
    cmd.push(path.display().to_string());

    let mut command = Command::new(&cmd[0]);
    command.args(&cmd[1..]);
    let output = command
        .output()
        .map_err(|e| format!("disasm_steps: cannot run {}: {e}", cmd[0]))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "disasm_steps: {} failed rc={}\n{}",
            cmd.join(" "),
            output.status.code().unwrap_or(-1),
            stderr.trim()
        ));
    }
    let text = String::from_utf8(output.stdout)
        .map_err(|_| "disasm_steps: objdump output is not UTF-8".to_string())?;
    let mut steps = parse_disassembly(&text);
    if arch == "x86" {
        steps = trim_x86_prefix(steps, &path.display().to_string())?;
    }
    Ok(steps)
}
