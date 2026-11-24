//! Upstream drop-in assembly and build (F7).
//!
//! Given an already-produced answer file (the emitter is a pre-existing tool),
//! this assembles the upstream proof-file shape for one task and runs
//! s2n-bignum's own `make` target. **Not on the verdict path.**
//!
//! `assemble` writes `<dropin>/<arch>/proofs/bignum_<op>_<fam>.ml`;
//! `build` runs `make -j1 HOLDIR=<hol> <dir>/bignum_<op>_<fam>.correct`.

use crate::tools::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

struct Family {
    dir: &'static str,
    modulus: &'static str,
    modulus_name: &'static str,
}

fn family(name: &str) -> Option<Family> {
    Some(match name {
        "p25519" => Family {
            dir: "curve25519",
            modulus: "57896044618658097711785492504343953926634992332820282019728792003956564819949",
            modulus_name: "p_25519",
        },
        "p256" => Family {
            dir: "p256",
            modulus: "115792089210356248762697446949407573530086143415290314195533631308867097853951",
            modulus_name: "p_256",
        },
        "p256k1" => Family {
            dir: "secp256k1",
            modulus: "115792089237316195423570985008687907853269984665640564039457584007908834671663",
            modulus_name: "p_256k1",
        },
        "p384" => Family {
            dir: "p384",
            modulus: "39402006196394479212279040100143613805079739270465446667948293404245721771496870329047266088258938001861606973112319",
            modulus_name: "p_384",
        },
        "p521" => Family {
            dir: "p521",
            modulus: "6864797660130609714981900799081393217269435300143305409394463459185543183397656052122559640661454554977296311391480858037121987999716643812574028291115057151",
            modulus_name: "p_521",
        },
        "sm2" => Family {
            dir: "sm2",
            modulus: "115792089210356248756420345214020892766250353991924191454421193933289684991999",
            modulus_name: "p_sm2",
        },
        _ => return None,
    })
}

fn parse_task(task: &str) -> Result<(String, String, String, Family), String> {
    let (arch, rest) = task.split_once('.').ok_or("task id has no '.'")?;
    if arch != "arm" && arch != "x86" {
        return Err(format!("unknown architecture {arch:?}"));
    }
    let base = rest.split('.').next().ok_or("task id has no base")?;
    let body = base
        .strip_prefix("bignum_")
        .ok_or("task base does not start bignum_")?;
    let (op, fam) = body.split_once('_').ok_or("bignum base has no operation")?;
    if !matches!(op, "add" | "sub" | "neg") {
        return Err(format!("not an add/sub/neg task: {op:?}"));
    }
    let f = family(fam).ok_or_else(|| format!("unknown family {fam:?}"))?;
    Ok((arch.to_string(), op.to_string(), fam.to_string(), f))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn arg_value<'a>(args: &'a [String], key: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == key)
        .and_then(|i| args.get(i + 1))
        .map(String::as_str)
}

fn copy_text(obj: &Path, arch: &str) -> Result<Vec<u8>, String> {
    let tool = if arch == "arm" {
        "llvm-objcopy"
    } else {
        "objcopy"
    };
    let out = std::env::temp_dir().join(format!(
        "tek-dropin-{}-{}",
        std::process::id(),
        obj.file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default()
    ));
    let status = Command::new(tool)
        .args(["-O", "binary", "--only-section=.text"])
        .arg(obj)
        .arg(&out)
        .status()
        .map_err(|e| format!("{tool}: {e}"))?;
    if !status.success() {
        return Err(format!("{tool} failed on {}", obj.display()));
    }
    let bytes = fs::read(&out).map_err(|e| e.to_string())?;
    let _ = fs::remove_file(&out);
    Ok(bytes)
}

fn mclist(bytes: &[u8], arch: &str) -> Result<String, String> {
    let mut lines = Vec::new();
    if arch == "arm" {
        if bytes.len() % 4 != 0 {
            return Err(".text is not a multiple of 4 bytes".into());
        }
        for chunk in bytes.chunks(4) {
            let w = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            lines.push(format!("0x{w:08x};"));
        }
    } else {
        for b in bytes {
            lines.push(format!("0x{b:02x};"));
        }
    }
    let mut out = String::new();
    for row in lines.chunks(4) {
        out.push_str("  ");
        out.push_str(&row.join(" "));
        out.push('\n');
    }
    Ok(out)
}

pub fn assemble(args: &[String]) -> Result<(), String> {
    let task = arg_value(args, "--task").ok_or("--task is required")?;
    let answer_path = arg_value(args, "--answer").ok_or("--answer is required")?;
    let dropin = arg_value(args, "--dropin")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_root().join(".bench/dropin"));
    let problems = arg_value(args, "--problems")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_root().join(".bench/problems.json"));

    let (arch, op, fam, f) = parse_task(task)?;
    let base = format!("bignum_{op}_{fam}");
    let theorem = format!(
        "BIGNUM_{}_{}_CORRECT",
        op.to_uppercase(),
        fam.to_uppercase()
    );
    let exec_name = theorem.replace("_CORRECT", "_EXEC");

    let corpus = json::parse(&fs::read(&problems).map_err(|e| e.to_string())?)
        .map_err(|e| format!("corpus: {e}"))?;
    let query = corpus
        .get(task)
        .and_then(|e| e.get("query"))
        .and_then(|q| q.as_str())
        .ok_or_else(|| format!("{task:?} has no query"))?;
    let answer = fs::read_to_string(answer_path).map_err(|e| format!("{answer_path}: {e}"))?;

    let obj = dropin.join(&arch).join(f.dir).join(format!("{base}.o"));
    let bytes = copy_text(&obj, &arch)?;
    let mc = mclist(&bytes, &arch)?;

    let body = if arch == "arm" {
        format!(
            "(* DROP-IN experiment: generated Tekmerion ARM proof in upstream shape. *)\n\
             needs \"arm/proofs/base.ml\";;\n\n\
             let {base}_mc = define_assert_from_elf \"{base}_mc\" \"arm/{dir}/{base}.o\"\n[\n{mc}];;\n\n\
             let {exec_name} = ARM_MK_EXEC_RULE {base}_mc;;\n\n\
             let {mod_name} = new_definition `{mod_name} = {modulus}`;;\n\n\
             let {theorem} = time prove\n ({query},\n{answer});;\n\n\
             Printf.printf \"val {theorem} : thm = %s\\n%!\" (string_of_thm {theorem});;\n",
            dir = f.dir, mod_name = f.modulus_name, modulus = f.modulus,
        )
    } else {
        format!(
            "(* DROP-IN experiment: generated Tekmerion x86 proof in upstream shape. *)\n\
             needs \"x86/proofs/base.ml\";;\n\n\
             let {base}_mc = define_assert_from_elf \"{base}_mc\" \"x86/{dir}/{base}.o\"\n[\n{mc}];;\n\
             let {base}_tmc = define_trimmed \"{base}_tmc\" {base}_mc;;\n\
             let {exec_name} = X86_MK_CORE_EXEC_RULE {base}_tmc;;\n\n\
             let {mod_name} = new_definition `{mod_name} = {modulus}`;;\n\n\
             let {theorem} = time prove\n ({query},\n{answer});;\n\n\
             Printf.printf \"val {theorem} : thm = %s\\n%!\" (string_of_thm {theorem});;\n",
            dir = f.dir, mod_name = f.modulus_name, modulus = f.modulus,
        )
    };

    let out = dropin.join(&arch).join("proofs").join(format!("{base}.ml"));
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&out, body).map_err(|e| e.to_string())?;
    println!(
        "assembled {} ({} bytes)",
        out.display(),
        fs::metadata(&out).map(|m| m.len()).unwrap_or(0)
    );
    Ok(())
}

pub fn build(args: &[String]) -> Result<i32, String> {
    let task = arg_value(args, "--task").ok_or("--task is required")?;
    let dropin = arg_value(args, "--dropin")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_root().join(".bench/dropin"));
    let hol = arg_value(args, "--hol-light")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_root().join(".bench/hol-light"));
    let (arch, op, fam, f) = parse_task(task)?;
    let target = format!("{}/bignum_{op}_{fam}.correct", f.dir);
    let status = Command::new("make")
        .args(["-j1"])
        .arg(format!("HOLDIR={}", hol.display()))
        .arg(&target)
        .current_dir(dropin.join(&arch))
        .status()
        .map_err(|e| format!("make: {e}"))?;
    println!("make {target} -> {:?}", status.code());
    Ok(status.code().unwrap_or(1))
}

pub fn run(args: &[String]) -> i32 {
    let cmd = args.first().map(String::as_str).unwrap_or("");
    let rest = if args.is_empty() { args } else { &args[1..] };
    let result = match cmd {
        "assemble" => assemble(rest).map(|()| 0),
        "build" => build(rest),
        _ => Err("usage: dropin <assemble|build> [options]".to_string()),
    };
    match result {
        Ok(code) => code,
        Err(e) => {
            eprintln!("ERROR {e}");
            2
        }
    }
}
