//! XVII.A1: the disassembler is selected by capability, not by name.
//!
//! `find_objdump` used to return the first executable it found on `PATH`.
//! On a runner whose `objdump` is x86-only, the ARM tests then failed inside
//! objdump instead of failing to find a usable tool. These tests pin the new
//! behaviour: a candidate is accepted only after it decodes the committed
//! fixture, an unusable shim is skipped, and failure names every candidate
//! with what it produced.
//!
//! The tests inject the search path instead of mutating `PATH`, so they do not
//! race with each other inside one test binary.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use tekmerion::tools::disasm;

struct TmpDir(PathBuf);

impl Drop for TmpDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn tmp_dir(label: &str) -> TmpDir {
    let dir = std::env::temp_dir().join(format!("tek-disasm-{}-{label}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");
    TmpDir(dir)
}

fn shim(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("write shim");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&path).expect("shim metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).expect("chmod shim");
    }
    path
}

fn path_with(dir: &Path) -> OsString {
    let live = std::env::var_os("PATH").unwrap_or_default();
    let mut combined = OsString::from(dir.as_os_str());
    combined.push(":");
    combined.push(live);
    combined
}

fn fixture(arch: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(match arch {
            "arm" => "aarch64_add.o",
            _ => "x86_add.o",
        })
}

#[test]
fn selection_picks_a_working_arm_tool_and_decodes_the_fixture() {
    let chosen = disasm::disassembler_for("arm").expect("a working AArch64 disassembler");
    eprintln!("chosen arm disassembler: {}", chosen.display());
    // The selected tool must decode the fixture through the production path.
    let steps = disasm::disassemble(
        &fixture("arm"),
        Some(chosen.to_str().expect("utf-8 tool path")),
        Some("arm"),
    )
    .expect("the selected tool disassembles the fixture");
    assert_eq!(steps[0].mnemonic, "add");
    assert_eq!(steps[0].operands, vec!["x0", "x0", "x1"]);
}

#[test]
fn selection_picks_a_working_x86_tool_and_decodes_the_fixture() {
    let chosen = disasm::disassembler_for("x86").expect("a working x86 disassembler");
    eprintln!("chosen x86 disassembler: {}", chosen.display());
    let steps = disasm::disassemble(
        &fixture("x86"),
        Some(chosen.to_str().expect("utf-8 tool path")),
        Some("x86"),
    )
    .expect("the selected tool disassembles the fixture");
    assert_eq!(steps[0].mnemonic, "add");
}

#[test]
fn an_unusable_shim_earlier_on_path_is_skipped() {
    let dir = tmp_dir("shim");
    let bad = shim(
        &dir.0,
        "aarch64-linux-gnu-objdump",
        "echo 'this is not a disassembler'; exit 0",
    );
    let search = path_with(&dir.0);
    let chosen = disasm::disassembler_for_with("arm", None, Some(search.as_os_str()))
        .expect("selection falls through to a working tool");
    assert_ne!(
        chosen, bad,
        "the shim exits 0 but decodes nothing, so it must not be chosen"
    );
    assert!(
        chosen.is_file(),
        "the chosen tool exists: {}",
        chosen.display()
    );
    // And the tool that was chosen really decodes the fixture.
    let steps = disasm::disassemble(
        &fixture("arm"),
        Some(chosen.to_str().expect("utf-8 tool path")),
        Some("arm"),
    )
    .expect("the chosen tool decodes");
    assert_eq!(steps[0].mnemonic, "add");
}

#[test]
fn an_unusable_explicit_tool_is_refused() {
    let dir = tmp_dir("explicit");
    let bad = shim(&dir.0, "objdump", "echo 'still not a disassembler'; exit 0");
    let err = disasm::disassembler_for_with(
        "arm",
        Some(bad.to_str().expect("utf-8 shim path")),
        Some(dir.0.as_os_str()),
    )
    .expect_err("an explicit tool is still probed");
    assert!(
        err.contains(&bad.display().to_string()),
        "the error names the explicit candidate and its output: {err}"
    );
    assert!(err.contains("decoded no `add`"), "{err}");
}

#[test]
fn with_no_usable_tool_the_error_names_every_candidate_and_the_install_step() {
    let dir = tmp_dir("empty");
    let err = disasm::disassembler_for_with("arm", None, Some(dir.0.as_os_str()))
        .expect_err("an empty search path has no disassembler");
    for name in ["aarch64-linux-gnu-objdump", "llvm-objdump", "objdump"] {
        assert!(err.contains(name), "error omits {name}: {err}");
    }
    assert!(err.contains("not found on PATH"), "{err}");
    assert!(err.contains("binutils-aarch64-linux-gnu"), "{err}");
    assert!(err.contains("--objdump PATH"), "{err}");
}

#[test]
fn the_probe_fixtures_are_committed_objects() {
    for arch in ["arm", "x86"] {
        let path = fixture(arch);
        let head = fs::read(&path).expect("fixture is tracked and readable");
        assert_eq!(&head[..4], b"\x7fELF", "{}", path.display());
    }
}
