//! Upstream drop-in assembly and build (F7).
//!
//! Given an already-produced answer file (the emitter is a pre-existing tool),
//! this assembles the upstream proof-file shape for one task and runs
//! s2n-bignum's own `make` target. **Not on the verdict path.**
//!
//! `assemble` writes `<dropin>/<arch>/proofs/bignum_<op>_<fam>.ml`;
//! `build` runs `make -j1 HOLDIR=<hol> <dir>/bignum_<op>_<fam>.correct`.
//!
//! `sweep` drives the generated tasks through the same two code paths,
//! replacing `research/receipts/ix54/build_all.py` and the untracked ARM
//! driver. `--arch x86` is the nine sealed x86 tasks (`add`/`sub`/`neg` on
//! p25519, p256k1, sm2); `--arch arm` is the eighteen ARM tasks (the same
//! three operations on p25519, p256, p256k1, p384, p521, sm2); `--arch all`
//! runs ARM then x86 in one pass.

use crate::tools::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

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

/// The proof-file body and destination for one task, without touching disk.
struct Assembled {
    out: PathBuf,
    body: String,
}

/// Canonicalise a numeral written in HOL's output: `0x4c` and `76` denote the
/// same numeral, so both become `76`. Decimal tokens are left alone.
fn canonical_number(tok: &str) -> String {
    let hex = tok.strip_prefix("0x").or_else(|| tok.strip_prefix("0X"));
    if let Some(hex) = hex {
        if !hex.is_empty() && hex.chars().all(|c| c.is_ascii_hexdigit()) {
            if let Ok(v) = u128::from_str_radix(hex, 16) {
                return v.to_string();
            }
        }
    }
    tok.to_string()
}

/// Tokenise a term so that whitespace and spacing around punctuation do not
/// count, while every identifier, symbol and numeral still has to match. HOL's
/// printer spells the quantifiers `forall`/`exists`; the source quotations use
/// `!`/`?`, and the two denote the same constants.
fn canonical_term(s: &str) -> String {
    let mut tokens: Vec<String> = Vec::new();
    let mut cur = String::new();
    for c in s.chars() {
        if c.is_alphanumeric() || c == '_' || c == '\'' {
            cur.push(c);
        } else {
            if !cur.is_empty() {
                tokens.push(canonical_number(&cur));
                cur.clear();
            }
            if c.is_whitespace() {
                continue;
            }
            tokens.push(match c {
                '!' => "forall".to_string(),
                '?' => "exists".to_string(),
                _ => c.to_string(),
            });
        }
    }
    if !cur.is_empty() {
        tokens.push(canonical_number(&cur));
    }
    tokens.join(" ")
}

/// Extract the HOL quotation that is the statement of `let <name> = time prove`
/// from a pinned upstream proof file. The statement is everything between the
/// first pair of backticks after the declaration.
fn extract_quotation(source: &str, name: &str) -> Result<String, String> {
    let needle = format!("let {name} = time prove");
    let start = source
        .match_indices(&needle)
        .find(|(i, _)| *i == 0 || source.as_bytes()[i - 1] == b'\n')
        .map(|(i, _)| i)
        .ok_or_else(|| format!("upstream has no line starting `{needle}`"))?;
    let rest = &source[start..];
    let open = rest
        .find('`')
        .ok_or_else(|| format!("`{needle}` has no HOL quotation"))?;
    let after = &rest[open + 1..];
    let close = after
        .find('`')
        .ok_or_else(|| format!("`{needle}` has an unterminated quotation"))?;
    Ok(after[..close].to_string())
}

/// Extract the whole `let <name> = time prove ... ;;` block verbatim. This is
/// what makes the emitted derivation upstream's own line rather than a
/// paraphrase: the bytes are copied from the pinned file.
fn extract_theorem_block(source: &str, name: &str) -> Result<String, String> {
    let needle = format!("let {name} = time prove");
    let start = source
        .match_indices(&needle)
        .find(|(i, _)| *i == 0 || source.as_bytes()[i - 1] == b'\n')
        .map(|(i, _)| i)
        .ok_or_else(|| format!("upstream has no line starting `{needle}`"))?;
    let rest = &source[start..];
    let end = rest
        .find(");;\n")
        .map(|i| i + 4)
        .or_else(|| rest.find(");;").map(|i| i + 3))
        .ok_or_else(|| format!("`{needle}` has no terminating `);;`"))?;
    Ok(rest[..end].trim_end().to_string())
}

/// The verbatim upstream subroutine derivation(s) for one task. On ARM this is
/// the single `_SUBROUTINE_CORRECT` block. On x86 the IBT theorem is derived
/// by `ADD_IBT_RULE` from the `_NOIBT_SUBROUTINE_CORRECT` block, so that block
/// is emitted too — both copied from the pinned `.ml`, in upstream order.
fn upstream_subroutine_blocks(
    upstream: &Path,
    arch: &str,
    op: &str,
    fam: &str,
) -> Result<String, String> {
    let base = format!("bignum_{op}_{fam}");
    let root = format!("BIGNUM_{}_{}", op.to_uppercase(), fam.to_uppercase());
    let path = upstream
        .join(arch)
        .join("proofs")
        .join(format!("{base}.ml"));
    let source = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut blocks = Vec::new();
    if arch == "x86" {
        blocks.push(extract_theorem_block(
            &source,
            &format!("{root}_NOIBT_SUBROUTINE_CORRECT"),
        )?);
    }
    let subroutine = format!("{root}_SUBROUTINE_CORRECT");
    let block = extract_theorem_block(&source, &subroutine)?;
    let derives = block.contains("ARM_ADD_RETURN_NOSTACK_TAC")
        || block.contains("X86_PROMOTE_RETURN_NOSTACK_TAC")
        || block.contains("MATCH_ACCEPT_TAC(ADD_IBT_RULE");
    if !derives {
        return Err(format!(
            "{}: {subroutine} does not use a recognised upstream return tactic",
            path.display()
        ));
    }
    blocks.push(block);
    Ok(blocks.join("\n\n"))
}

fn assemble_core(
    task: &str,
    answer_path: &Path,
    dropin: &Path,
    problems: &Path,
    upstream: &Path,
    stamp: bool,
) -> Result<Assembled, String> {
    let (arch, op, fam, f) = parse_task(task)?;
    let base = format!("bignum_{op}_{fam}");
    let theorem = format!(
        "BIGNUM_{}_{}_CORRECT",
        op.to_uppercase(),
        fam.to_uppercase()
    );
    let exec_name = theorem.replace("_CORRECT", "_EXEC");

    let corpus = json::parse(&fs::read(problems).map_err(|e| e.to_string())?)
        .map_err(|e| format!("corpus: {e}"))?;
    let query = corpus
        .get(task)
        .and_then(|e| e.get("query"))
        .and_then(|q| q.as_str())
        .ok_or_else(|| format!("{task:?} has no query"))?;
    let answer =
        fs::read_to_string(answer_path).map_err(|e| format!("{}: {e}", answer_path.display()))?;

    let obj = dropin.join(&arch).join(f.dir).join(format!("{base}.o"));
    let bytes = copy_text(&obj, &arch)?;
    let mc = mclist(&bytes, &arch)?;

    // `--stamp` wraps the proof with the axiom-stamp instrument. The
    // `before` value is taken outside the proof so the reported number is a
    // delta for this derivation, not a session total.
    let (stamp_pre, stamp_post) = if stamp {
        let before = format!("stamp_before_{theorem}");
        (
            format!("let {before} = inference_count ();;\n"),
            format!(
                "Printf.printf \"STAMP_BEGIN %s\\n%!\" \"{theorem}\";;\n\
                 Printf.printf \"STAMP %s %s\\n%!\" \"{theorem}\" (stamp_string {theorem});;\n\
                 Printf.printf \"STAMP_INF_BEFORE %d\\n%!\" {before};;\n\
                 Printf.printf \"STAMP_INF_AFTER %d\\n%!\" (inference_count ());;\n\
                 Printf.printf \"STAMP_INFERENCES %d\\n%!\" (inference_count () - {before});;\n\
                 List.iter\n\
                   (fun (id,dg,txt) -> Printf.printf \"STAMP_REGISTRY %d %s %s\\n%!\" id dg txt)\n\
                   (axiom_registry ());;\n\
                 List.iter\n\
                   (fun (nm,th) -> Printf.printf \"STAMP_NAME %s %s\\n%!\" nm\n\
                      (String.concat \",\" (List.map string_of_int (stamp_of th))))\n\
                   [\"ETA_AX\",ETA_AX; \"SELECT_AX\",SELECT_AX; \"INFINITY_AX\",INFINITY_AX];;\n\
                 Printf.printf \"STAMP_FOOTPRINT_BEGIN statement-level footprint, not the transitive derivation closure\\n%!\";;\n\
                 List.iter\n\
                   (fun (sym,what) -> Printf.printf \"STAMP_FOOTPRINT %s %s\\n%!\" sym what)\n\
                   (footprint_of {theorem});;\n"
            ),
        )
    } else {
        (String::new(), String::new())
    };

    // The subroutine contract is a derived consequence of `_CORRECT`. The
    // blocks are copied verbatim from the pinned upstream file; only the
    // surrounding comment is ours.
    let subroutine_blocks = upstream_subroutine_blocks(upstream, &arch, &op, &fam)?;
    let subroutine = theorem.replace("_CORRECT", "_SUBROUTINE_CORRECT");
    let upstream_rel = format!("{arch}/proofs/{base}.ml");

    let body = if arch == "arm" {
        format!(
            "(* DROP-IN experiment: generated Tekmerion ARM proof in upstream shape. *)\n\
             needs \"arm/proofs/base.ml\";;\n\n\
             let {base}_mc = define_assert_from_elf \"{base}_mc\" \"arm/{dir}/{base}.o\"\n[\n{mc}];;\n\n\
             let {exec_name} = ARM_MK_EXEC_RULE {base}_mc;;\n\n\
             let {mod_name} = new_definition `{mod_name} = {modulus}`;;\n\n\
             {stamp_pre}let {theorem} = time prove\n ({query},\n{answer});;\n\n\
             {stamp_post}Printf.printf \"val {theorem} : thm = %s\\n%!\" (string_of_thm {theorem});;\n\n\
             (* Derived subroutine contract. Verbatim from {upstream_rel}; the\n                trust story is unchanged because nothing new is asserted. *)\n\
             {subroutine_blocks}\n\n\
             Printf.printf \"val {subroutine} : thm = %s\\n%!\" (string_of_thm {subroutine});;\n",
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
             {stamp_pre}let {theorem} = time prove\n ({query},\n{answer});;\n\n\
             {stamp_post}Printf.printf \"val {theorem} : thm = %s\\n%!\" (string_of_thm {theorem});;\n\n\
             (* Derived subroutine contract. Verbatim from {upstream_rel}; the\n                trust story is unchanged because nothing new is asserted. *)\n\
             {subroutine_blocks}\n\n\
             Printf.printf \"val {subroutine} : thm = %s\\n%!\" (string_of_thm {subroutine});;\n",
            dir = f.dir, mod_name = f.modulus_name, modulus = f.modulus,
        )
    };

    let out = dropin.join(&arch).join("proofs").join(format!("{base}.ml"));
    Ok(Assembled { out, body })
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
    let upstream = arg_value(args, "--upstream")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_root().join(".bench/s2n-bignum"));
    let stamp = args.iter().any(|a| a == "--stamp");

    let assembled = assemble_core(
        task,
        Path::new(answer_path),
        &dropin,
        &problems,
        &upstream,
        stamp,
    )?;
    if let Some(parent) = assembled.out.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&assembled.out, assembled.body.as_bytes()).map_err(|e| e.to_string())?;
    println!(
        "assembled {} ({} bytes)",
        assembled.out.display(),
        fs::metadata(&assembled.out).map(|m| m.len()).unwrap_or(0)
    );
    Ok(())
}

fn make_status(
    dropin: &Path,
    arch: &str,
    target: &str,
    hol: &Path,
    log: Option<&Path>,
) -> Result<ExitStatus, String> {
    let mut cmd = Command::new("make");
    cmd.args(["-j1"])
        .arg(format!("HOLDIR={}", hol.display()))
        .arg(target)
        .current_dir(dropin.join(arch));
    if let Some(log) = log {
        let out = fs::File::create(log).map_err(|e| format!("{}: {e}", log.display()))?;
        let err = out
            .try_clone()
            .map_err(|e| format!("{}: {e}", log.display()))?;
        cmd.stdout(out).stderr(err);
    }
    cmd.status().map_err(|e| format!("make: {e}"))
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
    let status = make_status(&dropin, &arch, &target, &hol, None)?;
    println!("make {target} -> {:?}", status.code());
    Ok(status.code().unwrap_or(1))
}

// ---------------------------------------------------------------------------
// sweep
// ---------------------------------------------------------------------------

const SWEEP_OPS: [&str; 3] = ["add", "sub", "neg"];
const ARM_FAMILIES: [&str; 6] = ["p25519", "p256", "p256k1", "p384", "p521", "sm2"];
const X86_FAMILIES: [&str; 3] = ["p25519", "p256k1", "sm2"];

/// Parsed arguments for `dropin sweep`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SweepArgs {
    /// `x86`, `arm` or `all`.
    pub arch: String,
    /// Explicit answer directory, overriding the per-architecture default.
    pub answers: Option<PathBuf>,
    pub dropin: PathBuf,
    pub gen: PathBuf,
    pub hol: PathBuf,
    /// Read-only pin of the human proof tree, the source of the subroutine
    /// contracts and their derivations.
    pub upstream: PathBuf,
    pub dry_run: bool,
    /// Pass `make -B`, so every task rebuilds instead of being skipped.
    pub force: bool,
    /// Per-task wall-clock limit in seconds; 0 is rejected.
    pub timeout: u64,
    pub json: Option<PathBuf>,
    /// Assemble with the axiom-stamp instrument and require a `STAMP` line.
    pub stamp: bool,
}

impl SweepArgs {
    fn arches(&self) -> &'static [&'static str] {
        match self.arch.as_str() {
            "arm" => &["arm"],
            "all" => &["arm", "x86"],
            _ => &["x86"],
        }
    }

    /// Default answer directory: `<gen>/answers` for x86, `<gen>/answers-arm`
    /// for ARM. `--answers` overrides both.
    fn answer_path(&self, arch: &str, base: &str) -> PathBuf {
        let dir = match &self.answers {
            Some(dir) => dir.clone(),
            None if arch == "arm" => self.gen.join("answers-arm"),
            None => self.gen.join("answers"),
        };
        dir.join(format!("{base}.answer.txt"))
    }
}

/// One planned task, before `make` runs.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Planned {
    arch: String,
    op: String,
    fam: String,
    dir: String,
    base: String,
    /// Full corpus id, e.g. `arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT`.
    task: String,
}

impl Planned {
    /// `<dir>/<op>_<fam>` — the label the human summary prints.
    fn label(&self) -> String {
        format!("{}/{}_{}", self.dir, self.op, self.fam)
    }
}

/// The ordered task plan for an architecture. Operations are the outer loop so
/// the x86 sequence is unchanged from the original `sweep`.
fn plan(a: &SweepArgs) -> Vec<Planned> {
    let mut out = Vec::new();
    for arch in a.arches() {
        let fams: &[&str] = if *arch == "arm" {
            &ARM_FAMILIES
        } else {
            &X86_FAMILIES
        };
        for op in SWEEP_OPS {
            for fam in fams {
                let f = match family(fam) {
                    Some(f) => f,
                    None => continue,
                };
                let base = format!("bignum_{op}_{fam}");
                out.push(Planned {
                    arch: (*arch).to_string(),
                    op: op.to_string(),
                    fam: (*fam).to_string(),
                    dir: f.dir.to_string(),
                    task: format!(
                        "{arch}.{base}.BIGNUM_{}_{}_CORRECT",
                        op.to_uppercase(),
                        fam.to_uppercase()
                    ),
                    base,
                });
            }
        }
    }
    out
}

fn sweep_value(args: &[String], i: usize, key: &str) -> Result<String, String> {
    args.get(i + 1)
        .cloned()
        .ok_or_else(|| format!("{key} requires a value"))
}

/// `make` runs with `current_dir(<dropin>/<arch>)`, so a relative `HOLDIR`
/// would be resolved against the wrong directory. Every path argument is
/// therefore anchored at the repository root when it is not already absolute.
fn anchored(repo: &Path, p: PathBuf) -> PathBuf {
    if p.is_absolute() {
        p
    } else {
        repo.join(p)
    }
}

/// Parse `--arch <x86|arm|all> [--answers D] [--dir D] [--gen G] [--hol H]
/// [--force] [--timeout N] [--json P] [--dry-run] [--stamp]`.
pub fn parse_sweep_args(args: &[String]) -> Result<SweepArgs, String> {
    let repo = repo_root();
    let mut arch: Option<String> = None;
    let mut answers: Option<PathBuf> = None;
    let mut dropin = repo.join(".bench/dropin");
    let mut gen = repo.join(".bench/dropin-gen");
    let mut hol = repo.join(".bench/hol-light");
    let mut upstream = repo.join(".bench/s2n-bignum");
    let mut dry_run = false;
    let mut force = false;
    let mut timeout: u64 = 900;
    let mut json: Option<PathBuf> = None;
    let mut stamp = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--arch" => {
                arch = Some(sweep_value(args, i, "--arch")?);
                i += 2;
            }
            "--answers" => {
                answers = Some(PathBuf::from(sweep_value(args, i, "--answers")?));
                i += 2;
            }
            "--dir" => {
                dropin = PathBuf::from(sweep_value(args, i, "--dir")?);
                i += 2;
            }
            "--gen" => {
                gen = PathBuf::from(sweep_value(args, i, "--gen")?);
                i += 2;
            }
            "--hol" => {
                hol = PathBuf::from(sweep_value(args, i, "--hol")?);
                i += 2;
            }
            "--upstream" => {
                upstream = PathBuf::from(sweep_value(args, i, "--upstream")?);
                i += 2;
            }
            "--timeout" => {
                let raw = sweep_value(args, i, "--timeout")?;
                let secs = raw
                    .parse::<u64>()
                    .map_err(|_| format!("--timeout expects whole seconds, got {raw:?}"))?;
                if secs == 0 {
                    return Err("--timeout must be greater than zero".to_string());
                }
                timeout = secs;
                i += 2;
            }
            "--json" => {
                json = Some(PathBuf::from(sweep_value(args, i, "--json")?));
                i += 2;
            }
            "--force" => {
                force = true;
                i += 1;
            }
            "--dry-run" => {
                dry_run = true;
                i += 1;
            }
            "--stamp" => {
                stamp = true;
                i += 1;
            }
            other => return Err(format!("unknown argument {other:?}")),
        }
    }

    let arch = arch.ok_or("--arch is required (x86, arm or all)")?;
    if arch != "x86" && arch != "arm" && arch != "all" {
        return Err(format!(
            "unknown architecture {arch:?} (expected x86, arm or all)"
        ));
    }
    Ok(SweepArgs {
        arch,
        answers: answers.map(|p| anchored(&repo, p)),
        dropin: anchored(&repo, dropin),
        gen: anchored(&repo, gen),
        hol: anchored(&repo, hol),
        upstream: anchored(&repo, upstream),
        dry_run,
        force,
        timeout,
        json: json.map(|p| anchored(&repo, p)),
        stamp,
    })
}

/// One task's recorded outcome. `exit` is `None` only when assembly failed
/// before `make` was invoked; nothing else is ever guessed.
#[derive(Debug, Clone, PartialEq)]
pub struct TaskResult {
    pub arch: String,
    pub dir: String,
    pub op: String,
    pub fam: String,
    pub exit: Option<i32>,
    /// `Some("BIGNUM_<OP>_<FAM>_CORRECT")` iff the `.correct` file carries the
    /// `val <THEOREM> : thm` line.
    pub theorem: Option<String>,
    /// Implied by exit 0: upstream's template runs `check_axioms ()` before it
    /// prints anything, and `run-proof.sh` exits non-zero on failure.
    pub check_axioms: bool,
    /// Wall time printed by upstream's `build-proof.sh` as `Running time:`.
    /// It brackets `loadt <proof>` plus the `check_axioms ()` call that follows
    /// it; it excludes HOL Light load and the native compile.
    pub seconds: Option<f64>,
    pub timeout: bool,
    pub log: String,
    /// The `.correct` or log text carries an exception; display only.
    pub exception: bool,
    /// A `STAMP` line was required for this task (`--stamp`).
    pub stamp_required: bool,
    /// Everything the stamped proof printed, `None` when it printed no stamp.
    pub stamp_report: Option<StampReport>,
}

impl TaskResult {
    fn green(&self) -> bool {
        if self.exit != Some(0) || !self.check_axioms || self.theorem.is_none() {
            return false;
        }
        if !self.stamp_required {
            return true;
        }
        match &self.stamp_report {
            Some(r) => r.validate(self.theorem.as_deref()).is_ok(),
            None => false,
        }
    }

    /// Why a stamped task is red, for the run log. `None` when the stamp is
    /// not required or the record validates.
    fn stamp_error(&self) -> Option<String> {
        if !self.stamp_required {
            return None;
        }
        match &self.stamp_report {
            Some(r) => r.validate(self.theorem.as_deref()).err(),
            None => Some("no STAMP record".to_string()),
        }
    }

    fn label(&self) -> String {
        format!("{}/{}_{}", self.dir, self.op, self.fam)
    }

    fn axioms(&self) -> Option<&Vec<String>> {
        self.stamp_report.as_ref().map(|r| &r.axioms)
    }

    fn inference_delta(&self) -> Option<i64> {
        self.stamp_report
            .as_ref()
            .and_then(|r| r.inference_body_delta.or(r.inference_delta))
    }

    /// The first-line delta, which includes the shared HOL Light bootstrap.
    fn inference_delta_whole_file(&self) -> Option<i64> {
        self.stamp_report.as_ref().and_then(|r| r.inference_delta)
    }
}

/// What one stamped proof run printed. Everything here is taken verbatim from
/// the program's own output; nothing is inferred. `validate` is the only place
/// that decides whether the record is acceptable.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StampReport {
    pub begun: bool,
    /// The theorem name from `STAMP_BEGIN`.
    pub begin_theorem: Option<String>,
    /// `STAMP` was present (possibly empty).
    pub reported: bool,
    /// The theorem name the `STAMP` line itself names.
    pub stamp_theorem: Option<String>,
    pub axioms: Vec<String>,
    pub inference_before: Option<i64>,
    pub inference_after: Option<i64>,
    pub inference_delta: Option<i64>,
    /// Mark taken immediately after the shared `needs "<arch>/proofs/base.ml"`
    /// line, so the proof-body delta excludes the shared bootstrap.
    pub inference_body_before: Option<i64>,
    /// `inference_count () - inference_body_before`.
    pub inference_body_delta: Option<i64>,
    /// `(id, digest, conclusion)` from `STAMP_REGISTRY`.
    pub registry: Vec<(i64, String, String)>,
    /// `(name, ids)` from `STAMP_NAME` for the three basic axioms.
    pub names: Vec<(String, Vec<i64>)>,
    /// `(symbol, introducer)` from `STAMP_FOOTPRINT`.
    pub footprint: Vec<(String, String)>,
    /// The `STAMP_FOOTPRINT_BEGIN` label was present.
    pub footprint_label: bool,
    /// Lines that looked like stamp records but did not parse.
    pub malformed: Vec<String>,
}

/// One `STAMP` entry: `id:<n>#<digest>=|- <canonical conclusion>`.
/// Returns `None` unless the entry has the id, the digest marker and the
/// turnstile, so a loosely-shaped record cannot pass as a resolved entry.
fn stamp_entry_parts(entry: &str) -> Option<(i64, String, String)> {
    let rest = entry.trim().strip_prefix("id:")?;
    let (num, rest) = rest.split_once('#')?;
    let id: i64 = num.trim().parse().ok()?;
    let (digest, concl) = rest.split_once("=|-")?;
    let digest = digest.trim();
    if digest.is_empty() {
        return None;
    }
    Some((id, digest.to_string(), concl.trim().to_string()))
}

fn parse_i64(s: &str) -> Option<i64> {
    s.trim().parse::<i64>().ok()
}

/// Split a `STAMP` payload into entries. Entries are
/// `id:<n>#<dg>=|- <term>` joined by `|`, and each entry itself contains
/// `|-`, so a plain split on `|` is not enough: start a new entry only at a
/// part that begins with `id:`.
fn split_stamp_entries(s: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for part in s.split('|') {
        if out.is_empty() || part.trim_start().starts_with("id:") {
            out.push(part.trim().to_string());
        } else if let Some(last) = out.last_mut() {
            last.push('|');
            last.push_str(part.trim());
        }
    }
    out.retain(|e| !e.is_empty());
    out
}

/// Read the `STAMP*` lines a stamped proof prints. `None` when the run printed
/// no stamp at all (the un-stamped template, or a run that died early).
fn parse_stamp(text: &str) -> Option<StampReport> {
    let mut r = StampReport::default();
    let mut seen = false;
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if let Some(rest) = line.strip_prefix("STAMP_BEGIN ") {
            r.begun = !rest.trim().is_empty();
            r.begin_theorem = Some(rest.trim().to_string());
            seen = true;
        } else if let Some(rest) = line.strip_prefix("STAMP ") {
            // `STAMP <theorem> <payload>`; the payload may be empty.
            let (thm, payload) = rest.split_once(' ').unwrap_or((rest, ""));
            r.reported = true;
            r.stamp_theorem = Some(thm.trim().to_string());
            r.axioms = split_stamp_entries(payload);
            seen = true;
        } else if let Some(rest) = line.strip_prefix("STAMP_INF_BEFORE ") {
            r.inference_before = parse_i64(rest);
        } else if let Some(rest) = line.strip_prefix("STAMP_INF_AFTER ") {
            r.inference_after = parse_i64(rest);
        } else if let Some(rest) = line.strip_prefix("STAMP_INFERENCES ") {
            r.inference_delta = parse_i64(rest);
        } else if let Some(rest) = line.strip_prefix("STAMP_INF_BODY_BEFORE ") {
            r.inference_body_before = parse_i64(rest);
        } else if let Some(rest) = line.strip_prefix("STAMP_INF_BODY ") {
            r.inference_body_delta = parse_i64(rest);
        } else if let Some(rest) = line.strip_prefix("STAMP_REGISTRY ") {
            let mut it = rest.splitn(3, ' ');
            match (it.next(), it.next(), it.next()) {
                (Some(id), Some(digest), Some(txt)) if parse_i64(id).is_some() => {
                    r.registry.push((
                        parse_i64(id).unwrap(),
                        digest.trim().to_string(),
                        txt.trim().to_string(),
                    ));
                }
                _ => r.malformed.push(format!("STAMP_REGISTRY {rest}")),
            }
        } else if let Some(rest) = line.strip_prefix("STAMP_NAME ") {
            let mut parts = rest.split_whitespace();
            if let (Some(name), Some(ids)) = (parts.next(), parts.next()) {
                let ids: Vec<i64> = ids
                    .split(',')
                    .filter_map(|s| s.trim().parse::<i64>().ok())
                    .collect();
                r.names.push((name.to_string(), ids));
            } else {
                r.malformed.push(format!("STAMP_NAME {rest}"));
            }
        } else if let Some(rest) = line.strip_prefix("STAMP_FOOTPRINT_BEGIN ") {
            if rest.trim() == "statement-level footprint, not the transitive derivation closure" {
                r.footprint_label = true;
            } else {
                r.malformed.push(format!("STAMP_FOOTPRINT_BEGIN {rest}"));
            }
        } else if let Some(rest) = line.strip_prefix("STAMP_FOOTPRINT ") {
            match rest.split_once(' ') {
                Some((sym, what)) if !sym.trim().is_empty() && !what.trim().is_empty() => {
                    r.footprint
                        .push((sym.trim().to_string(), what.trim().to_string()));
                }
                _ => r.malformed.push(format!("STAMP_FOOTPRINT {rest}")),
            }
        }
    }
    if seen {
        Some(r)
    } else {
        None
    }
}

impl StampReport {
    /// The single acceptance test for a stamp record. Every clause is required
    /// by XIII.B6: the record names its theorem, the registry is unique, every
    /// id resolves to exactly one registry entry with the same digest, and the
    /// inference triad is arithmetically consistent.
    pub fn validate(&self, theorem: Option<&str>) -> Result<(), String> {
        let thm = theorem.ok_or("no `val <THEOREM> : thm` marker to resolve the stamp against")?;
        if !self.begun {
            return Err("STAMP_BEGIN missing or empty".to_string());
        }
        match self.begin_theorem.as_deref() {
            Some(t) if t == thm => {}
            Some(t) => return Err(format!("STAMP_BEGIN names {t}, theorem is {thm}")),
            None => return Err("STAMP_BEGIN does not name a theorem".to_string()),
        }
        if !self.reported {
            return Err("STAMP line missing".to_string());
        }
        match self.stamp_theorem.as_deref() {
            Some(t) if t == thm => {}
            Some(t) => return Err(format!("STAMP names {t}, theorem is {thm}")),
            None => return Err("STAMP does not name a theorem".to_string()),
        }
        if !self.footprint_label {
            return Err("STAMP_FOOTPRINT_BEGIN (statement-level footprint, not the \
                 transitive derivation closure) missing"
                .to_string());
        }
        if !self.malformed.is_empty() {
            return Err(format!(
                "malformed stamp record(s): {}",
                self.malformed.join("; ")
            ));
        }

        // Registry ids are positive and unique.
        for (i, (id, _, _)) in self.registry.iter().enumerate() {
            if *id < 1 {
                return Err(format!("registry id {id} is not positive"));
            }
            if self.registry[..i].iter().any(|(j, _, _)| j == id) {
                return Err(format!("registry id {id} is not unique"));
            }
        }

        // Every stamped id resolves to exactly one registry entry.
        for entry in &self.axioms {
            let (id, digest, concl) = stamp_entry_parts(entry)
                .ok_or_else(|| format!("malformed STAMP entry {entry:?}"))?;
            let hit = self
                .registry
                .iter()
                .find(|(i, _, _)| *i == id)
                .ok_or_else(|| format!("STAMP id {id} does not resolve in the registry"))?;
            if hit.1.is_empty() || digest != hit.1 {
                return Err(format!(
                    "STAMP id {id} digest {digest} does not match the registry entry"
                ));
            }
            if concl != hit.2 {
                return Err(format!(
                    "STAMP id {id} conclusion does not match the registry entry"
                ));
            }
        }

        // before/after/delta must agree arithmetically.
        let (before, after, delta) = match (
            self.inference_before,
            self.inference_after,
            self.inference_delta,
        ) {
            (Some(b), Some(a), Some(d)) => (b, a, d),
            _ => {
                return Err(
                    "inference before/after/delta triad is incomplete (negative counts are \
                     recorded as null)"
                        .to_string(),
                )
            }
        };
        if after < before {
            return Err(format!(
                "inference counter went backwards: before {before}, after {after}"
            ));
        }
        if after - before != delta {
            return Err(format!(
                "inference delta {delta} != after {after} - before {before} (= {})",
                after - before
            ));
        }
        // The optional proof-body pair must agree with the same final count.
        if let (Some(b), Some(d)) = (self.inference_body_before, self.inference_body_delta) {
            if after < b {
                return Err(format!(
                    "body inference counter went backwards: before {b}, after {after}"
                ));
            }
            if after - b != d {
                return Err(format!(
                    "body inference delta {d} != after {after} - body before {b} (= {})",
                    after - b
                ));
            }
        }
        Ok(())
    }
}

/// Registry and basic-axiom naming, unioned across every task that reported it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct StampSummary {
    /// `(id, digest, conclusion)`, unioned across tasks.
    registry: Vec<(i64, String, String)>,
    names: Vec<(String, Vec<i64>)>,
    /// The union of every task's statement-level footprint.
    footprint: Vec<(String, String)>,
    /// Set when two tasks disagree about the same registry id.
    registry_conflict: Option<String>,
}

impl StampSummary {
    fn absorb(&mut self, report: &StampReport) {
        for (id, digest, txt) in &report.registry {
            match self.registry.iter().find(|(i, _, _)| i == id) {
                Some((_, old_digest, old_txt)) => {
                    if old_digest != digest || old_txt != txt {
                        self.registry_conflict = Some(format!(
                            "registry id {id} has two different entries: {old_digest} {old_txt} / \
                             {digest} {txt}"
                        ));
                    }
                }
                None => self.registry.push((*id, digest.clone(), txt.clone())),
            }
        }
        for (name, ids) in &report.names {
            if !self.names.iter().any(|(n, _)| n == name) {
                self.names.push((name.clone(), ids.clone()));
            }
        }
        for (sym, what) in &report.footprint {
            if !self.footprint.iter().any(|(s, _)| s == sym) {
                self.footprint.push((sym.clone(), what.clone()));
            }
        }
        self.registry.sort_by_key(|(id, _, _)| *id);
        self.names.sort_by(|a, b| a.0.cmp(&b.0));
        self.footprint.sort();
        self.footprint.dedup();
    }

    /// `{ETA_AX, SELECT_AX}` for the ids a theorem's stamp carries.
    fn name_ids(&self, ids: &[String]) -> String {
        if ids.is_empty() {
            return "{}".to_string();
        }
        let named: Vec<String> = ids
            .iter()
            .map(|entry| {
                let hit = stamp_entry_id(entry).and_then(|n| {
                    self.names
                        .iter()
                        .find(|(_, vs)| vs.contains(&n))
                        .map(|(name, _)| name.clone())
                });
                hit.unwrap_or_else(|| {
                    if entry.starts_with("id:") {
                        entry.to_string()
                    } else {
                        format!("id:{entry}")
                    }
                })
            })
            .collect();
        format!("{{{}}}", named.join(", "))
    }
}

/// The numeric id of one `STAMP` entry (`id:3=|- ...` -> 3). A bare integer
/// is accepted too, so the name map is robust to either spelling.
fn stamp_entry_id(entry: &str) -> Option<i64> {
    let t = entry.trim();
    let rest = t.strip_prefix("id:").unwrap_or(t);
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse().ok()
    }
}

fn py_bool(b: bool) -> &'static str {
    if b {
        "True"
    } else {
        "False"
    }
}

/// GNU `timeout` returns 124 when the limit fires, and 137 if `--kill-after`
/// had to send SIGKILL. Anything else is the command's own status.
fn timed_out(exit: i32) -> bool {
    exit == 124 || exit == 137
}

/// Parse `Running time: 46.000000 sec, Start unixtime: ...` out of a
/// `.correct` file. Returns the last such value, or `None` when absent.
fn running_time(text: &str) -> Option<f64> {
    let mut found = None;
    for line in text.lines() {
        if let Some((_, rest)) = line.split_once("Running time:") {
            if let Some((num, _)) = rest.split_once("sec") {
                if let Ok(v) = num.trim().parse::<f64>() {
                    found = Some(v);
                }
            }
        }
    }
    found
}

/// The `val <THEOREM> : thm` line, if the `.correct` file has one.
///
/// Generated proofs print the plain `BIGNUM_<OP>_<FAM>_CORRECT`, while
/// upstream's `collect-specs.sh` only emits the `*_SUBROUTINE_CORRECT` specs,
/// so accept that as the fallback. The plain name is tried first, so a
/// generated tree reports exactly what it did before this fallback existed.
fn theorem_marker(text: &str, op: &str, fam: &str) -> Option<String> {
    let base = format!("BIGNUM_{}_{}", op.to_uppercase(), fam.to_uppercase());
    for name in [
        format!("{base}_CORRECT"),
        format!("{base}_SUBROUTINE_CORRECT"),
    ] {
        let marker = format!("val {name} : thm");
        if text.lines().any(|l| l.trim_start().starts_with(&marker)) {
            return Some(name);
        }
    }
    None
}

/// `logs-<arch>`: `logs-arm` keeps the existing ARM logs in place.
fn log_dir(gen: &Path, arch: &str) -> PathBuf {
    gen.join(format!("logs-{arch}"))
}

/// Restores the original proof file when it goes out of scope, including on
/// an early error return or a panic.
struct ProofFileGuard {
    path: PathBuf,
    original: Vec<u8>,
}

impl Drop for ProofFileGuard {
    fn drop(&mut self) {
        let _ = fs::write(&self.path, &self.original);
    }
}

/// Instrument an existing proof file in place (XIV.B1). The first mark is the
/// literal first line; the body mark is inserted immediately after the shared
/// `needs "<arch>/proofs/base.ml"` line so the comparable delta excludes the
/// shared HOL Light bootstrap that the generated side also excludes.
fn instrument_proof(path: &Path, theorem: &str, base: &str) -> Result<ProofFileGuard, String> {
    let original = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let text = String::from_utf8(original.clone())
        .map_err(|e| format!("{} is not UTF-8: {e}", path.display()))?;
    let mark = format!("stamp_mark_{base}");
    let body = format!("stamp_body_{base}");
    let mut out = String::with_capacity(text.len() + 4096);
    out.push_str(&format!("let {mark} = inference_count ();;\n"));
    let mut inserted = false;
    for line in text.split_inclusive('\n') {
        out.push_str(line);
        if !inserted && line.trim_start().starts_with("needs \"") {
            out.push_str(&format!("let {body} = inference_count ();;\n"));
            inserted = true;
        }
    }
    if !inserted {
        out.push_str(&format!("let {body} = inference_count ();;\n"));
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(&format!(
        r#"
Printf.printf "STAMP_BEGIN %s\n%!" "{theorem}";;
Printf.printf "STAMP %s %s\n%!" "{theorem}" (stamp_string {theorem});;
Printf.printf "STAMP_INF_BEFORE %d\n%!" {mark};;
Printf.printf "STAMP_INF_AFTER %d\n%!" (inference_count ());;
Printf.printf "STAMP_INFERENCES %d\n%!" (inference_count () - {mark});;
Printf.printf "STAMP_INF_BODY_BEFORE %d\n%!" {body};;
Printf.printf "STAMP_INF_BODY %d\n%!" (inference_count () - {body});;
List.iter
(fun (id,dg,txt) -> Printf.printf "STAMP_REGISTRY %d %s %s\n%!" id dg txt)
(axiom_registry ());;
List.iter
(fun (nm,th) -> Printf.printf "STAMP_NAME %s %s\n%!" nm
(String.concat "," (List.map string_of_int (stamp_of th))))
["ETA_AX",ETA_AX; "SELECT_AX",SELECT_AX; "INFINITY_AX",INFINITY_AX];;
Printf.printf "STAMP_FOOTPRINT_BEGIN statement-level footprint, not the transitive derivation closure\n%!";;
List.iter
(fun (sym,what) -> Printf.printf "STAMP_FOOTPRINT %s %s\n%!" sym what)
(footprint_of {theorem});;
Printf.printf "val {theorem} : thm = %s\n%!" (string_of_thm {theorem});;
"#
    ));
    let guard = ProofFileGuard {
        path: path.to_path_buf(),
        original,
    };
    fs::write(path, out.as_bytes()).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(guard)
}

fn build_logged(
    s: &SweepArgs,
    p: &Planned,
    logs: &Path,
    instrument: bool,
) -> Result<TaskResult, String> {
    let target = format!("{}/{}.correct", p.dir, p.base);
    let log = logs.join(format!("{}_{}_{}.log", p.dir, p.op, p.fam));
    let correct = s
        .dropin
        .join(&p.arch)
        .join(&p.dir)
        .join(format!("{}.correct", p.base));

    // With `--force` there must be no chance of reading a `.correct` left over
    // from an earlier run: delete it before make starts.
    if s.force {
        let _ = fs::remove_file(&correct);
    }

    // The guard restores the proof file after `make` (or on any error below).
    let _guard = if instrument {
        let proof = s
            .dropin
            .join(&p.arch)
            .join("proofs")
            .join(format!("{}.ml", p.base));
        let theorem = format!(
            "BIGNUM_{}_{}_SUBROUTINE_CORRECT",
            p.op.to_uppercase(),
            p.fam.to_uppercase()
        );
        Some(instrument_proof(&proof, &theorem, &p.base)?)
    } else {
        None
    };

    let out = fs::File::create(&log).map_err(|e| format!("{}: {e}", log.display()))?;
    let err = out
        .try_clone()
        .map_err(|e| format!("{}: {e}", log.display()))?;

    // GNU `timeout` runs the child in its own process group, so a timed-out
    // `make` does not leave a compiler or proof process behind. Exit 124 (or
    // 137 after `--kill-after`) means the limit fired.
    let mut cmd = Command::new("timeout");
    cmd.arg("--kill-after=30")
        .arg(s.timeout.to_string())
        .arg("make")
        .arg("-j1");
    if s.force {
        cmd.arg("-B");
    }
    cmd.arg(format!("HOLDIR={}", s.hol.display()))
        .arg(&target)
        .current_dir(s.dropin.join(&p.arch))
        .stdout(out)
        .stderr(err);
    let status = cmd.status().map_err(|e| format!("timeout/make: {e}"))?;

    let exit = status.code().unwrap_or(1);
    let hit_timeout = timed_out(exit);
    let log_text = fs::read_to_string(&log).unwrap_or_default();
    let correct_text = fs::read_to_string(&correct).ok();

    let theorem = correct_text
        .as_deref()
        .and_then(|t| theorem_marker(t, &p.op, &p.fam));
    let seconds = correct_text.as_deref().and_then(running_time);
    let exception = log_text.contains("ANSWER_EXCEPTION")
        || log_text.contains("Fatal error")
        || correct_text
            .as_deref()
            .map(|t| t.contains("ANSWER_EXCEPTION") || t.contains("Fatal error"))
            .unwrap_or(false);

    // Stamped output goes to the program's own stdout, i.e. the `.correct`
    // file; fall back to the make log only if that file is missing.
    let stamp_report = correct_text
        .as_deref()
        .and_then(parse_stamp)
        .or_else(|| parse_stamp(&log_text));
    let stamp_required = s.stamp || instrument;

    Ok(TaskResult {
        arch: p.arch.clone(),
        dir: p.dir.clone(),
        op: p.op.clone(),
        fam: p.fam.clone(),
        exit: Some(exit),
        theorem,
        check_axioms: exit == 0,
        seconds,
        timeout: hit_timeout,
        log: log.display().to_string(),
        exception,
        stamp_required,
        stamp_report,
    })
}

fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
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
    out.push('"');
    out
}

fn json_opt_int(v: Option<i32>) -> String {
    v.map(|n| n.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn json_opt_num(v: Option<f64>) -> String {
    v.map(|n| format!("{n:.6}"))
        .unwrap_or_else(|| "null".to_string())
}

fn json_opt_str(v: Option<&str>) -> String {
    v.map(json_str).unwrap_or_else(|| "null".to_string())
}

fn json_opt_i64(v: Option<i64>) -> String {
    v.map(|n| n.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn json_str_array(items: &[String]) -> String {
    let body: Vec<String> = items.iter().map(|s| json_str(s)).collect();
    format!("[{}]", body.join(", "))
}

fn json_opt_str_array(items: Option<&Vec<String>>) -> String {
    match items {
        Some(v) => json_str_array(v),
        None => "null".to_string(),
    }
}

/// One task object, exactly as both the sweep and measure receipts write it.
fn task_json_line(r: &TaskResult) -> String {
    let footprint: Vec<String> = r
        .stamp_report
        .as_ref()
        .map(|s| {
            s.footprint
                .iter()
                .map(|(sym, what)| {
                    format!(
                        "{{\"symbol\": {}, \"introducer\": {}}}",
                        json_str(sym),
                        json_str(what)
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    format!(
        "{{\"arch\": {}, \"dir\": {}, \"op\": {}, \"family\": {}, \"exit\": {}, \"theorem\": {}, \"check_axioms\": {}, \"seconds\": {}, \"timeout\": {}, \"stamp_required\": {}, \"stamp_ok\": {}, \"stamp_theorem\": {}, \"axioms\": {}, \"inference_delta\": {}, \"inference_delta_whole_file\": {}, \"footprint\": [{}], \"log\": {}}}",
        json_str(&r.arch),
        json_str(&r.dir),
        json_str(&r.op),
        json_str(&r.fam),
        json_opt_int(r.exit),
        json_opt_str(r.theorem.as_deref()),
        r.check_axioms,
        json_opt_num(r.seconds),
        r.timeout,
        r.stamp_required,
        r.stamp_required && r.stamp_error().is_none(),
        json_opt_str(r.stamp_report.as_ref().and_then(|s| s.stamp_theorem.as_deref())),
        json_opt_str_array(r.axioms()),
        json_opt_i64(r.inference_delta()),
        json_opt_i64(r.inference_delta_whole_file()),
        footprint.join(", "),
        json_str(&r.log),
    )
}

/// Hand-written serialiser for the sweep receipt. Kept dependency-free on
/// purpose (`cargo build --offline` must keep working).
#[allow(clippy::too_many_arguments)]
fn results_json(
    arch: &str,
    force: bool,
    timeout: u64,
    s2n_pin: &str,
    hol_pin: &str,
    generated_at: u64,
    stamp: bool,
    summary: &StampSummary,
    results: &[TaskResult],
) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!("  \"generated_at\": {generated_at},\n"));
    out.push_str(&format!("  \"arch\": {},\n", json_str(arch)));
    out.push_str(&format!("  \"s2n_bignum_pin\": {},\n", json_str(s2n_pin)));
    out.push_str(&format!("  \"hol_light_pin\": {},\n", json_str(hol_pin)));
    out.push_str("  \"cores\": 1,\n");
    out.push_str(&format!("  \"force\": {force},\n"));
    out.push_str(&format!("  \"stamp\": {stamp},\n"));
    out.push_str(&format!("  \"timeout_seconds\": {timeout},\n"));
    out.push_str("  \"axiom_registry\": [");
    for (i, (id, digest, txt)) in summary.registry.iter().enumerate() {
        let comma = if i + 1 == summary.registry.len() {
            ""
        } else {
            ","
        };
        out.push_str(&format!(
            "{}{{\"id\": {id}, \"digest\": {}, \"conclusion\": {}}}{comma}",
            if i == 0 { "" } else { " " },
            json_str(digest),
            json_str(txt)
        ));
    }
    out.push_str("],\n");
    out.push_str(&format!(
        "  \"axiom_registry_conflict\": {},\n",
        summary.registry_conflict.is_some()
    ));
    out.push_str(
        "  \"footprint_label\": \"statement-level footprint, not the transitive derivation closure\",\n",
    );
    out.push_str("  \"basic_axioms\": {");
    for (i, (name, ids)) in summary.names.iter().enumerate() {
        out.push_str(&format!(
            "{} {}: [{}]",
            if i == 0 { "" } else { "," },
            json_str(name),
            ids.iter()
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
                .join(", "),
        ));
    }
    out.push_str("},\n");
    out.push_str("  \"tasks\": [\n");
    for (i, r) in results.iter().enumerate() {
        let comma = if i + 1 == results.len() { "" } else { "," };
        out.push_str(&format!("    {}{comma}\n", task_json_line(r)));
    }
    out.push_str("  ],\n");
    let ok = results.iter().filter(|r| r.green()).count();
    out.push_str(&format!(
        "  \"ok\": {ok},\n  \"total\": {}\n}}\n",
        results.len()
    ));
    out
}

/// Pin of a frozen checkout, or `"unknown"` if `dir` is not a git repo of its
/// own (a plain copy would otherwise report Tekmerion's own HEAD).
fn git_pin(dir: &Path) -> String {
    if !dir.join(".git").exists() {
        return "unknown".to_string();
    }
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "--short", "HEAD"])
        .output();
    match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => "unknown".to_string(),
    }
}

fn kernel_build_name(dir: &Path) -> String {
    dir.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string()
}

fn print_summary(results: &[TaskResult]) {
    println!("\nSUMMARY");
    println!(
        "{:<4} {:<26} {:<5} {:<27} {:<6} SECONDS",
        "ARCH", "TASK", "EXIT", "THEOREM", "AXIOMS"
    );
    for r in results {
        println!(
            "{:<4} {:<26} {:<5} {:<27} {:<6} {}",
            r.arch,
            r.label(),
            r.exit
                .map(|e| e.to_string())
                .unwrap_or_else(|| "-".to_string()),
            r.theorem.as_deref().unwrap_or("-"),
            if r.check_axioms { "true" } else { "false" },
            r.seconds
                .map(|s| format!("{s:.3}"))
                .unwrap_or_else(|| "-".to_string())
        );
    }
}

/// Per-theorem stamp table: arch, task, axioms actually used, inference delta,
/// check seconds. Nothing here is computed from anything but the parsed logs.
fn print_stamp_table(results: &[TaskResult], summary: &StampSummary) {
    println!("\nSTAMP TABLE");
    println!(
        "{:<4} {:<24} {:<38} {:>13} {:>9}",
        "ARCH", "TASK", "AXIOMS USED", "INFERENCES", "SECONDS"
    );
    for r in results {
        let ax = match r.axioms() {
            Some(ids) => summary.name_ids(ids),
            None => "n/a".to_string(),
        };
        println!(
            "{:<4} {:<24} {:<38} {:>13} {:>9}",
            r.arch,
            r.label(),
            ax,
            r.inference_delta()
                .map(|n| n.to_string())
                .unwrap_or_else(|| "null".to_string()),
            r.seconds
                .map(|s| format!("{s:.3}"))
                .unwrap_or_else(|| "null".to_string())
        );
    }

    if !summary.registry.is_empty() {
        println!("\nAXIOM REGISTRY (canonical typed conclusions)");
        for (id, digest, txt) in &summary.registry {
            let name = summary
                .names
                .iter()
                .find(|(_, ids)| ids.contains(id))
                .map(|(n, _)| n.as_str())
                .unwrap_or("(not a basic axiom)");
            println!("  id:{id} = {name} [{digest}]: {txt}");
        }
    }

    if !summary.footprint.is_empty() {
        println!(
            "\nSTATEMENT-LEVEL FOOTPRINT (not the transitive derivation closure): {} symbols",
            summary.footprint.len()
        );
        for (sym, what) in &summary.footprint {
            println!("  {sym} <- {what}");
        }
    }
}

/// Assemble then build the requested tasks (x86 nine, ARM eighteen, or both).
pub fn sweep(args: &[String]) -> Result<i32, String> {
    let parsed = parse_sweep_args(args)?;
    let problems = repo_root().join(".bench/problems.json");
    let planned = plan(&parsed);
    let total = planned.len();
    let mut results: Vec<TaskResult> = Vec::new();
    let mut summary = StampSummary::default();

    for p in &planned {
        let answer = parsed.answer_path(&p.arch, &p.base);
        let assembled = match assemble_core(
            &p.task,
            &answer,
            &parsed.dropin,
            &problems,
            &parsed.upstream,
            parsed.stamp,
        ) {
            Ok(a) => a,
            Err(e) => {
                // Assembly failed: no make, no exit code. Record it honestly.
                println!("wrote {}.ml (FAILED to assemble)", p.base);
                println!("  {}: assemble error: {e}", p.label());
                results.push(TaskResult {
                    arch: p.arch.clone(),
                    dir: p.dir.clone(),
                    op: p.op.clone(),
                    fam: p.fam.clone(),
                    exit: None,
                    theorem: None,
                    check_axioms: false,
                    seconds: None,
                    timeout: false,
                    log: answer.display().to_string(),
                    exception: true,
                    stamp_required: parsed.stamp,
                    stamp_report: None,
                });
                continue;
            }
        };

        if parsed.dry_run {
            println!("wrote {}.ml (dry run)", p.base);
            let flags = if parsed.force { "-j1 -B" } else { "-j1" };
            println!(
                "  {}: would run make {flags} HOLDIR={} {}/{}.correct",
                p.label(),
                parsed.hol.display(),
                p.dir,
                p.base
            );
            continue;
        }

        if let Some(parent) = assembled.out.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&assembled.out, assembled.body.as_bytes()).map_err(|e| e.to_string())?;
        println!("wrote {}.ml", p.base);

        let logs = log_dir(&parsed.gen, &p.arch);
        fs::create_dir_all(&logs).map_err(|e| format!("{}: {e}", logs.display()))?;
        let result = build_logged(&parsed, p, &logs, false)?;
        if let Some(report) = &result.stamp_report {
            summary.absorb(report);
        }
        println!(
            "  {}: exit={} theorem={} exception={}",
            result.label(),
            result
                .exit
                .map(|e| e.to_string())
                .unwrap_or_else(|| "-".to_string()),
            py_bool(result.theorem.is_some()),
            py_bool(result.exception)
        );
        if let Some(e) = result.stamp_error() {
            println!("  {}: stamp invalid: {e}", result.label());
        }
        results.push(result);

        // Rewrite the JSON after every task so a wall-clock stop still leaves
        // exactly the tasks that really ran, and nothing invented for the rest.
        if let Some(path) = &parsed.json {
            write_json(path, &parsed, &summary, &results)?;
        }
    }

    if parsed.dry_run {
        println!("DRY-RUN {total}/{total} assembled; make not invoked");
        return Ok(0);
    }

    if let Some(path) = &parsed.json {
        write_json(path, &parsed, &summary, &results)?;
        println!("wrote JSON {}", path.display());
    }

    print_summary(&results);
    if parsed.stamp {
        print_stamp_table(&results, &summary);
    }
    let ok = results.iter().filter(|r| r.green()).count();
    println!("OK {ok}/{total}");
    Ok(if ok == total && !results.is_empty() {
        0
    } else {
        1
    })
}

fn write_json(
    path: &Path,
    parsed: &SweepArgs,
    summary: &StampSummary,
    results: &[TaskResult],
) -> Result<(), String> {
    let generated_at = now_unix();
    let text = results_json(
        &parsed.arch,
        parsed.force,
        parsed.timeout,
        &git_pin(&parsed.dropin),
        &git_pin(&parsed.hol),
        generated_at,
        parsed.stamp,
        summary,
        results,
    );
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
    }
    fs::write(path, text.as_bytes()).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn run(args: &[String]) -> i32 {
    let cmd = args.first().map(String::as_str).unwrap_or("");
    let rest = if args.is_empty() { args } else { &args[1..] };
    let result = match cmd {
        "assemble" => assemble(rest).map(|()| 0),
        "build" => build(rest),
        "sweep" => sweep(rest),
        "measure" => measure(rest),
        "stamp-table" => stamp_table(rest),
        "checktime-table" => checktime_table(rest),
        "matched-table" => matched_table(rest),
        "obligation" => obligation(rest),
        "premise" => premise(rest),
        _ => Err(
            "usage: dropin <assemble|build|sweep|measure|stamp-table|checktime-table|matched-table|obligation|premise> [options]\n\
             \n\
             sweep: dropin sweep --arch <x86|arm|all> [--answers <dir>] [--dir <dropin-dir>]
             \x20      [--gen <gen-dir>] [--hol <hol-dir>] [--upstream <pin-dir>] [--force]
             \x20      [--timeout <secs>] [--json <path>] [--dry-run] [--stamp]
             \n\
             assemble: dropin assemble --task <id> --answer <file> [--dir <dropin-dir>]
             \x20        [--problems <json>] [--upstream <pin-dir>] [--stamp]
             \n\
             measure: dropin measure --tree <dir> --arch <x86|arm|all> [--hol <hol-dir>]
             \x20        [--force] [--timeout <secs>] [--json <path>] [--label <name>]
             \x20        [--instrument] [--only arch.dir.op]
             \n\
             stamp-table: dropin stamp-table --json <sweep-json>
             \n\
             checktime-table: dropin checktime-table --human <json> --generated <json>
             \x20        [--human-kernel K] [--generated-kernel K]
             \n\
             matched-table: dropin matched-table --human <json> --generated <json>
             \x20        [--json <path>] [--human-kernel K] [--generated-kernel K]
             \n\
             obligation: dropin obligation --human-tree <dir> --generated-tree <dir>
             \x20        [--generated-theorem <correct|subroutine>] [--json <path>]
             \n\
             premise: dropin premise --upstream-tree <dir> --generated-tree <dir>
             \x20        [--json <path>]"
                .to_string(),
        ),
    };
    match result {
        Ok(code) => code,
        Err(e) => {
            eprintln!("ERROR {e}");
            2
        }
    }
}

fn json_i64(v: &json::Json) -> Option<i64> {
    match v {
        json::Json::Num(n) => Some(*n as i64),
        _ => None,
    }
}

/// Render the receipt's tables from a sweep JSON file. Every number, axiom id
/// and conclusion comes from that file; nothing is retyped.
pub fn stamp_table(args: &[String]) -> Result<i32, String> {
    let path = arg_value(args, "--json").ok_or("--json is required")?;
    let bytes = fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    let j = json::parse(&bytes).map_err(|e| format!("{path}: {e}"))?;

    let mut summary = StampSummary::default();
    if let Some(reg) = j.get("axiom_registry").and_then(|v| v.as_array()) {
        for e in reg {
            if let (Some(id), Some(txt)) = (
                e.get("id").and_then(json_i64),
                e.get("conclusion").and_then(|v| v.as_str()),
            ) {
                let digest = e
                    .get("digest")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                summary.registry.push((id, digest, txt.to_string()));
            }
        }
    }
    if let Some(obj) = j.get("basic_axioms").and_then(|v| v.as_obj()) {
        for (name, ids) in obj {
            let ids: Vec<i64> = ids
                .as_array()
                .map(|a| a.iter().filter_map(json_i64).collect())
                .unwrap_or_default();
            summary.names.push((name.clone(), ids));
        }
    }
    summary.registry.sort_by_key(|(id, _, _)| *id);
    summary.names.sort_by(|a, b| a.0.cmp(&b.0));

    let empty = Vec::new();
    let tasks = j.get("tasks").and_then(|v| v.as_array()).unwrap_or(&empty);

    let basic: Vec<i64> = summary
        .names
        .iter()
        .flat_map(|(_, ids)| ids.iter().copied())
        .collect();

    println!("## Per-theorem axiom stamp\n");
    println!("| arch | task | axioms used | inferences | seconds |");
    println!("|---|---|---|---:|---:|");
    let mut stamped = 0usize;
    let mut outside = 0usize;
    let mut missing = 0usize;
    for t in tasks {
        let arch = t.get("arch").and_then(|v| v.as_str()).unwrap_or("?");
        let dir = t.get("dir").and_then(|v| v.as_str()).unwrap_or("?");
        let op = t.get("op").and_then(|v| v.as_str()).unwrap_or("?");
        let fam = t.get("family").and_then(|v| v.as_str()).unwrap_or("?");
        let task = format!("{dir}/{op}_{fam}");
        let axi = match t.get("axioms") {
            Some(json::Json::Arr(ids)) => {
                stamped += 1;
                let ids: Vec<String> = ids
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(str::to_string)
                    .collect();
                if ids
                    .iter()
                    .filter_map(|s| stamp_entry_id(s))
                    .any(|id| !basic.contains(&id))
                {
                    outside += 1;
                }
                summary.name_ids(&ids)
            }
            _ => {
                missing += 1;
                "n/a".to_string()
            }
        };
        let inf = t
            .get("inference_delta")
            .and_then(json_i64)
            .map(|n| n.to_string())
            .unwrap_or_else(|| "n/a".to_string());
        let secs = match t.get("seconds") {
            Some(json::Json::Num(n)) => format!("{n:.3}"),
            _ => "n/a".to_string(),
        };
        // Footprint rows are only printed in the aggregate section below; the
        // per-theorem table stays the same shape as IX.57.
        if let Some(fp) = t.get("footprint").and_then(|v| v.as_array()) {
            for e in fp {
                if let (Some(sym), Some(what)) = (
                    e.get("symbol").and_then(|v| v.as_str()),
                    e.get("introducer").and_then(|v| v.as_str()),
                ) {
                    if !summary.footprint.iter().any(|(s, _)| s == sym) {
                        summary.footprint.push((sym.to_string(), what.to_string()));
                    }
                }
            }
        }
        println!("| {arch} | {task} | {axi} | {inf} | {secs} |");
    }

    println!("\n## Axiom registry (canonical typed conclusions)\n");
    println!("| id | name | digest | conclusion |");
    println!("|---|---|---|---|");
    for (id, digest, txt) in &summary.registry {
        let name = summary
            .names
            .iter()
            .find(|(_, ids)| ids.contains(id))
            .map(|(n, _)| n.as_str())
            .unwrap_or("*(not a basic axiom)*");
        println!("| {id} | {name} | `{digest}` | `{txt}` |");
    }

    summary.footprint.sort();
    summary.footprint.dedup();
    println!("\n## Statement-level footprint (not the transitive derivation closure)\n");
    println!(
        "Constants and type constructors occurring in the 27 conclusions, each\n\
             mapped to the definition or type definition that introduced it when the\n\
             kernel registry knows it. This is **statement-level footprint, not the\n\
             transitive derivation closure**: it does not follow the proof DAG and it\n\
             says nothing about which definitions a derivation *used*.\n"
    );
    println!("| symbol | introducer |");
    println!("|---|---|");
    for (sym, what) in &summary.footprint {
        println!("| `{sym}` | {what} |");
    }

    println!(
        "\nstamped={stamped} missing_stamp={missing} outside_basic_axioms={outside} total={}",
        tasks.len()
    );
    Ok(if missing == 0 && outside == 0 { 0 } else { 1 })
}

// ---------------------------------------------------------------------------
// measure: run an existing tree through the same recipe as `sweep`
// ---------------------------------------------------------------------------

/// Parsed arguments for `dropin measure`.
///
/// `measure` never assembles anything: the 27 proof files are already in the
/// tree. It walks the same plan as `sweep`, runs `make -j1 [-B]` inside
/// `<tree>/<arch>` and parses `Running time` out of each `.correct` file, so a
/// human tree and a generated tree are measured by the same code path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureArgs {
    pub tree: PathBuf,
    pub arch: String,
    pub hol: PathBuf,
    pub force: bool,
    pub timeout: u64,
    pub json: Option<PathBuf>,
    /// Recorded verbatim in the receipt, e.g. `human`.
    pub label: String,
    /// Log root: `<gen>/logs-<arch>/<dir>_<op>_<fam>.log`.
    pub gen: PathBuf,
    /// Instrument the existing proof file in place before `make`, then restore
    /// it. Used to run the human proof under the stamp kernel (XIV.B1).
    pub instrument: bool,
    /// Optional single-task filter, `arch.dir.op` (e.g. `arm.sm2.neg`).
    pub only: Option<String>,
}

impl MeasureArgs {
    fn arches(&self) -> &'static [&'static str] {
        match self.arch.as_str() {
            "arm" => &["arm"],
            "all" => &["arm", "x86"],
            _ => &["x86"],
        }
    }

    /// `build_logged` already knows how to build one planned task. Reuse it
    /// instead of writing a second build path that could drift from `sweep`.
    fn as_sweep(&self) -> SweepArgs {
        SweepArgs {
            arch: self.arch.clone(),
            answers: None,
            dropin: self.tree.clone(),
            gen: self.gen.clone(),
            hol: self.hol.clone(),
            upstream: repo_root().join(".bench/s2n-bignum"),
            dry_run: false,
            force: self.force,
            timeout: self.timeout,
            json: self.json.clone(),
            stamp: self.instrument,
        }
    }
}

/// Parse `--tree <dir> [--arch <x86|arm|all>] [--hol H] [--force]
/// [--timeout N] [--json P] [--label L]`.
pub fn parse_measure_args(args: &[String]) -> Result<MeasureArgs, String> {
    let repo = repo_root();
    let mut tree: Option<PathBuf> = None;
    let mut arch: Option<String> = None;
    let mut hol = repo.join(".bench/hol-light");
    let mut force = false;
    let mut timeout: u64 = 900;
    let mut json: Option<PathBuf> = None;
    let mut label = String::from("measure");
    let mut instrument = false;
    let mut only: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--tree" => {
                tree = Some(PathBuf::from(sweep_value(args, i, "--tree")?));
                i += 2;
            }
            "--arch" => {
                arch = Some(sweep_value(args, i, "--arch")?);
                i += 2;
            }
            "--hol" => {
                hol = PathBuf::from(sweep_value(args, i, "--hol")?);
                i += 2;
            }
            "--json" => {
                json = Some(PathBuf::from(sweep_value(args, i, "--json")?));
                i += 2;
            }
            "--label" => {
                label = sweep_value(args, i, "--label")?;
                i += 2;
            }
            "--timeout" => {
                let raw = sweep_value(args, i, "--timeout")?;
                let secs = raw
                    .parse::<u64>()
                    .map_err(|_| format!("--timeout expects whole seconds, got {raw:?}"))?;
                if secs == 0 {
                    return Err("--timeout must be greater than zero".to_string());
                }
                timeout = secs;
                i += 2;
            }
            "--force" => {
                force = true;
                i += 1;
            }
            "--instrument" => {
                instrument = true;
                i += 1;
            }
            "--only" => {
                only = Some(sweep_value(args, i, "--only")?);
                i += 2;
            }
            other => return Err(format!("unknown argument {other:?}")),
        }
    }

    let tree = tree.ok_or("--tree is required")?;
    let arch = arch.ok_or("--arch is required (x86, arm or all)")?;
    if arch != "x86" && arch != "arm" && arch != "all" {
        return Err(format!(
            "unknown architecture {arch:?} (expected x86, arm or all)"
        ));
    }
    // The label names a log directory under `.bench/measure-logs`, so it must
    // not be able to escape it.
    if label.is_empty()
        || label == "."
        || label == ".."
        || label.contains('/')
        || label.contains('\\')
    {
        return Err(format!("--label must be a plain name, got {label:?}"));
    }

    let gen = repo.join(".bench/measure-logs").join(&label);
    Ok(MeasureArgs {
        tree: anchored(&repo, tree),
        arch,
        hol: anchored(&repo, hol),
        force,
        timeout,
        json: json.map(|p| anchored(&repo, p)),
        label,
        gen,
        instrument,
        only,
    })
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn ensure_parent(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
    }
    Ok(())
}

/// Same shape as the sweep receipt, plus `label` and `tree` so the reader can
/// tell which tree each `Running time` came from.
fn measure_json(m: &MeasureArgs, generated_at: u64, results: &[TaskResult]) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!("  \"generated_at\": {generated_at},\n"));
    out.push_str(&format!("  \"label\": {},\n", json_str(&m.label)));
    out.push_str(&format!("  \"arch\": {},\n", json_str(&m.arch)));
    out.push_str(&format!(
        "  \"tree\": {},\n",
        json_str(&m.tree.display().to_string())
    ));
    out.push_str(&format!(
        "  \"s2n_bignum_pin\": {},\n",
        json_str(&git_pin(&m.tree))
    ));
    out.push_str(&format!(
        "  \"hol_light_pin\": {},\n",
        json_str(&git_pin(&m.hol))
    ));
    out.push_str("  \"cores\": 1,\n");
    out.push_str(&format!("  \"force\": {},\n", m.force));
    out.push_str(&format!("  \"stamp\": {},\n", m.instrument));
    out.push_str(&format!("  \"instrument\": {},\n", m.instrument));
    out.push_str(&format!(
        "  \"kernel_build\": {},\n",
        json_str(&kernel_build_name(&m.hol))
    ));
    out.push_str(&format!("  \"timeout_seconds\": {},\n", m.timeout));
    out.push_str("  \"tasks\": [\n");
    for (i, r) in results.iter().enumerate() {
        let comma = if i + 1 == results.len() { "" } else { "," };
        out.push_str(&format!("    {}{comma}\n", task_json_line(r)));
    }
    out.push_str("  ],\n");
    let ok = results
        .iter()
        .filter(|r| r.green() && r.seconds.is_some())
        .count();
    out.push_str(&format!(
        "  \"ok\": {ok},\n  \"total\": {}\n}}\n",
        results.len()
    ));
    out
}

fn write_measure_json(path: &Path, m: &MeasureArgs, results: &[TaskResult]) -> Result<(), String> {
    ensure_parent(path)?;
    let text = measure_json(m, now_unix(), results);
    fs::write(path, text.as_bytes()).map_err(|e| format!("{}: {e}", path.display()))
}

/// `dropin measure --tree <dir> --arch <x86|arm|all> [--json P] [--force]`.
pub fn measure(args: &[String]) -> Result<i32, String> {
    let m = parse_measure_args(args)?;
    if !m.tree.is_dir() {
        return Err(format!("--tree is not a directory: {}", m.tree.display()));
    }
    // Fail before touching make, and never panic on a wrong tree.
    for arch in m.arches() {
        if !m.tree.join(arch).join("Makefile").is_file() {
            return Err(format!(
                "--tree {} has no {arch}/Makefile; not an s2n-bignum tree",
                m.tree.display()
            ));
        }
    }

    let s = m.as_sweep();
    let planned: Vec<Planned> = plan(&s)
        .into_iter()
        .filter(|p| match &m.only {
            None => true,
            Some(sel) => {
                let parts: Vec<&str> = sel.split('.').collect();
                parts.len() == 3 && parts[0] == p.arch && parts[1] == p.dir && parts[2] == p.op
            }
        })
        .collect();
    if planned.is_empty() {
        return Err(format!("--only {:?} matches no planned task", m.only));
    }
    let total = planned.len();
    let mut results: Vec<TaskResult> = Vec::new();

    for p in &planned {
        let logs = log_dir(&s.gen, &p.arch);
        fs::create_dir_all(&logs).map_err(|e| format!("{}: {e}", logs.display()))?;
        let result = build_logged(&s, p, &logs, m.instrument)?;
        println!(
            "  {}: exit={} theorem={} seconds={} (label {})",
            result.label(),
            result
                .exit
                .map(|e| e.to_string())
                .unwrap_or_else(|| "-".to_string()),
            py_bool(result.theorem.is_some()),
            result
                .seconds
                .map(|v| format!("{v:.3}"))
                .unwrap_or_else(|| "-".to_string()),
            m.label
        );
        results.push(result);

        // Rewrite after every task: an interrupted run leaves exactly the
        // tasks that really finished, nothing invented for the rest.
        if let Some(path) = &m.json {
            write_measure_json(path, &m, &results)?;
        }
    }

    print_summary(&results);
    let ok = results
        .iter()
        .filter(|r| r.green() && r.seconds.is_some())
        .count();
    println!("MEASURED {ok}/{total} (label {})", m.label);
    Ok(if ok == total && !results.is_empty() {
        0
    } else {
        1
    })
}

// ---------------------------------------------------------------------------
// checktime-table: matched-pair human vs generated check cost
// ---------------------------------------------------------------------------

/// The XII.F9.2 per-task budget, in seconds.
const CHECKTIME_BUDGET_SECONDS: f64 = 120.0;
/// The XII.F9.2 warning line: 70% of the budget.
const CHECKTIME_WARN_FRACTION: f64 = 0.70;

// ---------------------------------------------------------------------------
// Typed comparability (XIV.A1-XIV.A3)
// ---------------------------------------------------------------------------

/// Which tree produced a measurement. The provenance is part of the scope so
/// that a human measurement cannot be silently paired with another generated
/// one, or a tree with itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeKind {
    Human,
    Generated,
}

impl TreeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            TreeKind::Human => "human",
            TreeKind::Generated => "generated",
        }
    }
}

/// Everything about *how* a proof was measured that must agree before two
/// numbers may be put in the same aggregate. `tree_kind` must be the
/// complementary human/generated pair; the remaining fields must be equal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasurementScope {
    pub tree_kind: TreeKind,
    pub s2n_pin: String,
    /// `make -j` value. Every matched run here is `-j1`.
    pub jobs: u32,
    /// `make -B`, so a stale `.correct` cannot contribute a number.
    pub force: bool,
    /// The HOL Light build used (`hol-light-stamp2`, ...).
    pub kernel_build: String,
}

/// One measured task, with the theorem identity and the conditions of
/// measurement carried alongside the numbers (XIV.A2).
#[derive(Debug, Clone, PartialEq)]
pub struct MeasuredProof {
    /// The exact theorem name printed by the run, not a derived label.
    pub theorem: String,
    pub exit: Option<i32>,
    pub check_axioms: bool,
    pub theorem_proved: bool,
    pub seconds: Option<f64>,
    pub inferences: Option<u64>,
    pub scope: MeasurementScope,
}

/// Every reason two measurements may be refused a comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComparisonError {
    TheoremIdentityMismatch {
        human: String,
        generated: String,
    },
    HumanExitNotZero {
        exit: Option<i32>,
    },
    GeneratedExitNotZero {
        exit: Option<i32>,
    },
    HumanAxiomsNotChecked,
    GeneratedAxiomsNotChecked,
    HumanTheoremNotProved,
    GeneratedTheoremNotProved,
    WrongTreeKind {
        expected: TreeKind,
        found: TreeKind,
    },
    ScopeMismatch {
        field: &'static str,
        human: String,
        generated: String,
    },
}

impl std::fmt::Display for ComparisonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComparisonError::TheoremIdentityMismatch { human, generated } => write!(
                f,
                "theorem identity differs: human {human:?}, generated {generated:?}"
            ),
            ComparisonError::HumanExitNotZero { exit } => {
                write!(f, "human judgment failed: exit={}", fmt_exit(*exit))
            }
            ComparisonError::GeneratedExitNotZero { exit } => {
                write!(f, "generated judgment failed: exit={}", fmt_exit(*exit))
            }
            ComparisonError::HumanAxiomsNotChecked => {
                write!(f, "human judgment failed: check_axioms is false")
            }
            ComparisonError::GeneratedAxiomsNotChecked => {
                write!(f, "generated judgment failed: check_axioms is false")
            }
            ComparisonError::HumanTheoremNotProved => {
                write!(f, "human judgment failed: the theorem was not proved")
            }
            ComparisonError::GeneratedTheoremNotProved => {
                write!(f, "generated judgment failed: the theorem was not proved")
            }
            ComparisonError::WrongTreeKind { expected, found } => write!(
                f,
                "measurement tree kind is {} but {} was required",
                found.as_str(),
                expected.as_str()
            ),
            ComparisonError::ScopeMismatch {
                field,
                human,
                generated,
            } => write!(
                f,
                "measurement scope differs in {field}: human {human:?}, generated {generated:?}"
            ),
        }
    }
}

fn fmt_exit(exit: Option<i32>) -> String {
    exit.map(|e| e.to_string())
        .unwrap_or_else(|| "missing".to_string())
}

/// A comparison that may only exist when the two measurements are of the
/// identical theorem, both succeeded, and the measurement conditions agree.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchedComparison {
    pub human: MeasuredProof,
    pub generated: MeasuredProof,
}

impl MatchedComparison {
    pub fn theorem(&self) -> &str {
        &self.human.theorem
    }

    /// `generated / human`, only when both numbers exist and human is non-zero.
    pub fn seconds_ratio(&self) -> Option<f64> {
        ratio(self.human.seconds, self.generated.seconds)
    }

    /// `generated / human` primitive inferences, same guard.
    pub fn inference_ratio(&self) -> Option<f64> {
        match (self.human.inferences, self.generated.inferences) {
            (Some(h), Some(g)) if h > 0 => Some(g as f64 / h as f64),
            _ => None,
        }
    }
}

fn ratio(human: Option<f64>, generated: Option<f64>) -> Option<f64> {
    match (human, generated) {
        (Some(h), Some(g)) if h > 0.0 => Some(g / h),
        _ => None,
    }
}

/// All the reasons the pair is not a `MatchedComparison`, in a fixed order.
/// The first element is what `compare` returns.
pub fn comparison_reasons(
    human: &MeasuredProof,
    generated: &MeasuredProof,
) -> Vec<ComparisonError> {
    let mut out = Vec::new();
    if human.theorem != generated.theorem {
        out.push(ComparisonError::TheoremIdentityMismatch {
            human: human.theorem.clone(),
            generated: generated.theorem.clone(),
        });
    }
    if human.exit != Some(0) {
        out.push(ComparisonError::HumanExitNotZero { exit: human.exit });
    }
    if !human.check_axioms {
        out.push(ComparisonError::HumanAxiomsNotChecked);
    }
    if !human.theorem_proved {
        out.push(ComparisonError::HumanTheoremNotProved);
    }
    if generated.exit != Some(0) {
        out.push(ComparisonError::GeneratedExitNotZero {
            exit: generated.exit,
        });
    }
    if !generated.check_axioms {
        out.push(ComparisonError::GeneratedAxiomsNotChecked);
    }
    if !generated.theorem_proved {
        out.push(ComparisonError::GeneratedTheoremNotProved);
    }
    if human.scope.tree_kind != TreeKind::Human {
        out.push(ComparisonError::WrongTreeKind {
            expected: TreeKind::Human,
            found: human.scope.tree_kind,
        });
    }
    if generated.scope.tree_kind != TreeKind::Generated {
        out.push(ComparisonError::WrongTreeKind {
            expected: TreeKind::Generated,
            found: generated.scope.tree_kind,
        });
    }
    let a = &human.scope;
    let b = &generated.scope;
    if a.s2n_pin != b.s2n_pin {
        out.push(ComparisonError::ScopeMismatch {
            field: "s2n_bignum_pin",
            human: a.s2n_pin.clone(),
            generated: b.s2n_pin.clone(),
        });
    }
    if a.jobs != b.jobs {
        out.push(ComparisonError::ScopeMismatch {
            field: "jobs",
            human: a.jobs.to_string(),
            generated: b.jobs.to_string(),
        });
    }
    if a.force != b.force {
        out.push(ComparisonError::ScopeMismatch {
            field: "force",
            human: a.force.to_string(),
            generated: b.force.to_string(),
        });
    }
    if a.kernel_build != b.kernel_build {
        out.push(ComparisonError::ScopeMismatch {
            field: "kernel_build",
            human: a.kernel_build.clone(),
            generated: b.kernel_build.clone(),
        });
    }
    out
}

/// The typed precondition. A `MatchedComparison` cannot be constructed any
/// other way, so a caller cannot aggregate a non-matching row by accident.
pub fn compare(
    human: &MeasuredProof,
    generated: &MeasuredProof,
) -> Result<MatchedComparison, ComparisonError> {
    match comparison_reasons(human, generated).into_iter().next() {
        Some(reason) => Err(reason),
        None => Ok(MatchedComparison {
            human: human.clone(),
            generated: generated.clone(),
        }),
    }
}

/// One `tasks[]` entry from a sweep/measure receipt. Acceptance is carried
/// through instead of discarded, so a rejected measurement can never
/// contribute a number (XIV.A2).
#[derive(Debug, Clone, PartialEq)]
struct TimedTask {
    arch: String,
    dir: String,
    op: String,
    fam: String,
    theorem: Option<String>,
    exit: Option<i32>,
    check_axioms: bool,
    seconds: Option<f64>,
    inferences: Option<u64>,
    /// Present only for instrumented runs: the delta from the first line,
    /// which includes the shared HOL Light bootstrap.
    inferences_whole_file: Option<u64>,
    log: String,
}

impl TimedTask {
    fn key(&self) -> (String, String, String, String) {
        (
            self.arch.clone(),
            self.dir.clone(),
            self.op.clone(),
            self.fam.clone(),
        )
    }
}

/// A whole receipt file: its measurement scope plus its rows.
#[derive(Debug, Clone, PartialEq)]
struct TimedFile {
    tree: Option<String>,
    scope: MeasurementScope,
    tasks: Vec<TimedTask>,
}

fn json_u64(v: &json::Json) -> Option<u64> {
    match v {
        json::Json::Num(n) if *n >= 0.0 => Some(*n as u64),
        _ => None,
    }
}

fn json_i32(v: &json::Json) -> Option<i32> {
    match v {
        json::Json::Num(n) => Some(*n as i32),
        _ => None,
    }
}

fn json_bool(v: &json::Json) -> Option<bool> {
    match v {
        json::Json::Bool(b) => Some(*b),
        _ => None,
    }
}

/// Read a receipt with its top-level measurement conditions. `kernel_fallback`
/// is used only when the file predates the `kernel_build` field.
fn read_timed_tasks(
    path: &Path,
    kind: TreeKind,
    kernel_fallback: &str,
) -> Result<TimedFile, String> {
    let bytes = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let j = json::parse(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    let tree = j.get("tree").and_then(|v| v.as_str()).map(str::to_string);
    let s2n_pin = j
        .get("s2n_bignum_pin")
        .and_then(|v| v.as_str())
        .unwrap_or("?")
        .to_string();
    let jobs = j.get("cores").and_then(json_u64).unwrap_or(1) as u32;
    let force = j.get("force").and_then(json_bool).unwrap_or(false);
    let kernel_build = j
        .get("kernel_build")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| {
            if kernel_fallback.is_empty() {
                let stamp = j.get("stamp").and_then(json_bool).unwrap_or(false);
                if stamp {
                    "stamp-kernel".to_string()
                } else {
                    "unstamped".to_string()
                }
            } else {
                kernel_fallback.to_string()
            }
        });
    let scope = MeasurementScope {
        tree_kind: kind,
        s2n_pin,
        jobs,
        force,
        kernel_build,
    };
    let empty = Vec::new();
    let tasks = j.get("tasks").and_then(|v| v.as_array()).unwrap_or(&empty);
    let mut out = Vec::with_capacity(tasks.len());
    for t in tasks {
        let field = |k: &str| t.get(k).and_then(|v| v.as_str()).unwrap_or("?").to_string();
        let theorem = t
            .get("theorem")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let exit = t.get("exit").and_then(json_i32);
        let check_axioms = t.get("check_axioms").and_then(json_bool).unwrap_or(false);
        let seconds = match t.get("seconds") {
            Some(json::Json::Num(n)) => Some(*n),
            _ => None,
        };
        let inferences = t.get("inference_delta").and_then(json_u64);
        let inferences_whole_file = t.get("inference_delta_whole_file").and_then(json_u64);
        out.push(TimedTask {
            arch: field("arch"),
            dir: field("dir"),
            op: field("op"),
            fam: field("family"),
            theorem,
            exit,
            check_axioms,
            seconds,
            inferences,
            inferences_whole_file,
            log: field("log"),
        });
    }
    Ok(TimedFile {
        tree,
        scope,
        tasks: out,
    })
}

fn median(mut xs: Vec<f64>) -> Option<f64> {
    if xs.is_empty() {
        return None;
    }
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = xs.len();
    Some(if n % 2 == 1 {
        xs[n / 2]
    } else {
        (xs[n / 2 - 1] + xs[n / 2]) / 2.0
    })
}

fn arch_rank(a: &str) -> usize {
    if a == "arm" {
        0
    } else {
        1
    }
}

fn op_rank(op: &str) -> usize {
    SWEEP_OPS
        .iter()
        .position(|o| *o == op)
        .unwrap_or(SWEEP_OPS.len())
}

fn fam_rank(f: &str) -> usize {
    ARM_FAMILIES
        .iter()
        .position(|x| *x == f)
        .unwrap_or(ARM_FAMILIES.len())
}

/// One joined human/generated row. The verdict is `None` only when a side is
/// absent; otherwise it is exactly what `compare` said.
#[derive(Debug, Clone, PartialEq)]
struct Pair {
    arch: String,
    dir: String,
    op: String,
    fam: String,
    human: Option<TimedTask>,
    generated: Option<TimedTask>,
    human_proof: Option<MeasuredProof>,
    generated_proof: Option<MeasuredProof>,
    verdict: Option<Result<MatchedComparison, ComparisonError>>,
    reasons: Vec<ComparisonError>,
}

impl Pair {
    fn label(&self) -> String {
        format!("{}/{}_{}", self.dir, self.op, self.fam)
    }

    fn matched(&self) -> Option<&MatchedComparison> {
        match &self.verdict {
            Some(Ok(m)) => Some(m),
            _ => None,
        }
    }

    fn human_seconds(&self) -> Option<f64> {
        self.human.as_ref().and_then(|t| t.seconds)
    }

    fn generated_seconds(&self) -> Option<f64> {
        self.generated.as_ref().and_then(|t| t.seconds)
    }

    fn human_inferences(&self) -> Option<u64> {
        self.human.as_ref().and_then(|t| t.inferences)
    }

    fn generated_inferences(&self) -> Option<u64> {
        self.generated.as_ref().and_then(|t| t.inferences)
    }

    fn refused(&self) -> bool {
        !matches!(self.verdict, Some(Ok(_)))
    }

    /// A single line for the unmatched list: the first refusal, with the
    /// remaining reasons named so nothing is hidden.
    fn refusal_reason(&self) -> String {
        if self.human.is_none() {
            return "no human measurement".to_string();
        }
        if self.generated.is_none() {
            return "no generated measurement".to_string();
        }
        let first = match &self.verdict {
            Some(Err(e)) => e.to_string(),
            Some(Ok(_)) => return "matched".to_string(),
            None => "not compared".to_string(),
        };
        let extra: Vec<String> = self
            .reasons
            .iter()
            .skip(1)
            .map(ToString::to_string)
            .collect();
        if extra.is_empty() {
            first
        } else {
            format!("{first}; also: {}", extra.join("; "))
        }
    }
}

/// Build a `MeasuredProof` from a row and the scope of the file it came from.
fn measured(task: &TimedTask, scope: &MeasurementScope) -> MeasuredProof {
    MeasuredProof {
        theorem: task.theorem.clone().unwrap_or_default(),
        exit: task.exit,
        check_axioms: task.check_axioms,
        theorem_proved: task.theorem.is_some(),
        seconds: task.seconds,
        inferences: task.inferences,
        scope: scope.clone(),
    }
}

fn pair_rows(human: &TimedFile, generated: &TimedFile) -> Vec<Pair> {
    use std::collections::BTreeMap;
    let hmap: BTreeMap<_, _> = human.tasks.iter().map(|t| (t.key(), t)).collect();
    let gmap: BTreeMap<_, _> = generated.tasks.iter().map(|t| (t.key(), t)).collect();
    let mut keys: Vec<_> = hmap.keys().chain(gmap.keys()).cloned().collect();
    keys.sort();
    keys.dedup();
    keys.sort_by(|a, b| {
        arch_rank(&a.0)
            .cmp(&arch_rank(&b.0))
            .then(op_rank(&a.2).cmp(&op_rank(&b.2)))
            .then(fam_rank(&a.3).cmp(&fam_rank(&b.3)))
            .then(a.1.cmp(&b.1))
    });
    keys.into_iter()
        .map(|k| {
            let h = hmap.get(&k);
            let g = gmap.get(&k);
            let human_proof = h.map(|t| measured(t, &human.scope));
            let generated_proof = g.map(|t| measured(t, &generated.scope));
            let (verdict, reasons) = match (&human_proof, &generated_proof) {
                (Some(hp), Some(gp)) => (Some(compare(hp, gp)), comparison_reasons(hp, gp)),
                _ => (None, Vec::new()),
            };
            Pair {
                arch: k.0,
                dir: k.1,
                op: k.2,
                fam: k.3,
                human: h.map(|t| (*t).clone()),
                generated: g.map(|t| (*t).clone()),
                human_proof,
                generated_proof,
                verdict,
                reasons,
            }
        })
        .collect()
}

fn fmt_secs(v: Option<f64>) -> String {
    v.map(|x| format!("{x:.3}")).unwrap_or_else(|| "n/a".into())
}

fn fmt_ratio(v: Option<f64>) -> String {
    v.map(|x| format!("{x:.2}")).unwrap_or_else(|| "n/a".into())
}

/// An aggregate over **matched pairs only**. Every `add` takes a
/// `MatchedComparison`, so an unmatched row cannot enter. Each metric has its
/// own denominator: a matched pair contributes to the seconds total only when
/// both times exist, and to the inference total only when both counts exist.
#[derive(Default)]
struct Group {
    n: usize,
    seconds_n: usize,
    inference_n: usize,
    human_seconds: f64,
    generated_seconds: f64,
    human_inferences: u128,
    generated_inferences: u128,
    seconds_ratios: Vec<f64>,
    inference_ratios: Vec<f64>,
}

impl Group {
    fn add(&mut self, m: &MatchedComparison) {
        self.n += 1;
        if let (Some(h), Some(g)) = (m.human.seconds, m.generated.seconds) {
            self.seconds_n += 1;
            self.human_seconds += h;
            self.generated_seconds += g;
            if h > 0.0 {
                self.seconds_ratios.push(g / h);
            }
        }
        if let (Some(h), Some(g)) = (m.human.inferences, m.generated.inferences) {
            self.inference_n += 1;
            self.human_inferences += h as u128;
            self.generated_inferences += g as u128;
            if h > 0 {
                self.inference_ratios.push(g as f64 / h as f64);
            }
        }
    }

    fn seconds_aggregate(&self) -> Option<f64> {
        (self.seconds_n > 0 && self.human_seconds > 0.0)
            .then(|| self.generated_seconds / self.human_seconds)
    }

    fn inference_aggregate(&self) -> Option<f64> {
        (self.inference_n > 0 && self.human_inferences > 0)
            .then(|| self.generated_inferences as f64 / self.human_inferences as f64)
    }
}

fn refusal_code(e: &ComparisonError) -> &'static str {
    match e {
        ComparisonError::TheoremIdentityMismatch { .. } => "theorem identity",
        ComparisonError::HumanExitNotZero { .. } => "human exit",
        ComparisonError::GeneratedExitNotZero { .. } => "generated exit",
        ComparisonError::HumanAxiomsNotChecked => "human check_axioms",
        ComparisonError::GeneratedAxiomsNotChecked => "generated check_axioms",
        ComparisonError::HumanTheoremNotProved => "human theorem",
        ComparisonError::GeneratedTheoremNotProved => "generated theorem",
        ComparisonError::WrongTreeKind { .. } => "tree kind",
        ComparisonError::ScopeMismatch { .. } => "scope",
    }
}

fn fmt_u64(v: Option<u64>) -> String {
    v.map(|x| x.to_string()).unwrap_or_else(|| "n/a".into())
}

fn fmt_match_verdict(p: &Pair) -> String {
    if p.human.is_none() {
        "no human row".to_string()
    } else if p.generated.is_none() {
        "no generated row".to_string()
    } else {
        match &p.verdict {
            Some(Ok(_)) => "matched".to_string(),
            Some(Err(e)) => format!("refused: {}", refusal_code(e)),
            None => "not compared".to_string(),
        }
    }
}

/// Render the matched check-time table. Pure so it can be tested without the
/// `.bench` trees.
fn render_checktime(human: &TimedFile, generated: &TimedFile) -> String {
    let rows = pair_rows(human, generated);
    let mut out = String::new();

    out.push_str("## Matched check-time table (XIV.A)\n\n");
    out.push_str(
        "A row contributes to an aggregate only when `compare` returns a\n\
         `MatchedComparison`: exact theorem identity, both judgments successful,\n\
         and identical measurement scopes. `human s` and `generated s` are the\n\
         `Running time:` value parsed from each task's `.correct` file.\n\n",
    );
    out.push_str(
        "| arch | task | human theorem | generated theorem | human s | generated s | verdict |\n",
    );
    out.push_str("|---|---|---|---|---:|---:|---|\n");
    for p in &rows {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} |\n",
            p.arch,
            p.label(),
            p.human_proof
                .as_ref()
                .map(|m| m.theorem.as_str())
                .unwrap_or("n/a"),
            p.generated_proof
                .as_ref()
                .map(|m| m.theorem.as_str())
                .unwrap_or("n/a"),
            fmt_secs(p.human_seconds()),
            fmt_secs(p.generated_seconds()),
            fmt_match_verdict(p),
        ));
    }

    let matched: Vec<&MatchedComparison> = rows.iter().filter_map(|p| p.matched()).collect();
    let matched_count = matched.len();
    let both_rows = rows
        .iter()
        .filter(|p| p.human.is_some() && p.generated.is_some())
        .count();
    let unmatched: Vec<&Pair> = rows.iter().filter(|p| p.refused()).collect();

    let mut all = Group::default();
    for m in &matched {
        all.add(m);
    }

    out.push_str("\n### Measurement counts (XIV.A3/A4)\n\n");
    out.push_str("| quantity | value |\n|---|---:|\n");
    out.push_str(&format!("| human rows parsed | {} |\n", human.tasks.len()));
    out.push_str(&format!(
        "| generated rows parsed | {} |\n",
        generated.tasks.len()
    ));
    out.push_str(&format!("| task rows (union of keys) | {} |\n", rows.len()));
    out.push_str(&format!("| rows with both sides present | {both_rows} |\n"));
    out.push_str(&format!("| matched pairs (n) | {matched_count} |\n"));
    out.push_str(&format!(
        "| unmatched rows | {} |\n",
        rows.len() - matched_count
    ));

    out.push_str("\n### Matched aggregates (denominator: n matched pairs)\n\n");
    out.push_str("| metric | value |\n|---|---:|\n");
    out.push_str(&format!("| matched pairs (n) | {matched_count} |\n"));
    if matched_count == 0 {
        out.push_str("| human total s (n=0) | n/a (no matched pairs) |\n");
        out.push_str("| generated total s (n=0) | n/a (no matched pairs) |\n");
        out.push_str("| aggregate seconds ratio | n/a (n=0; not published) |\n");
        out.push_str("| per-task seconds ratio median | n/a (n=0) |\n");
        out.push_str("| human total inferences (n=0) | n/a (no matched pairs) |\n");
        out.push_str("| generated total inferences (n=0) | n/a (no matched pairs) |\n");
        out.push_str("| aggregate inference ratio | n/a (n=0; not published) |\n");
        out.push_str("| per-task inference ratio median | n/a (n=0) |\n");
    } else {
        out.push_str(&format!(
            "| human total s (n={}) | {:.3} |\n",
            all.seconds_n, all.human_seconds
        ));
        out.push_str(&format!(
            "| generated total s (n={}) | {:.3} |\n",
            all.seconds_n, all.generated_seconds
        ));
        out.push_str(&format!(
            "| aggregate seconds ratio (n={}) | {} |\n",
            all.seconds_n,
            fmt_ratio(all.seconds_aggregate())
        ));
        out.push_str(&format!(
            "| per-task seconds ratio median (n={}) | {} |\n",
            all.seconds_n,
            fmt_ratio(median(all.seconds_ratios.clone()))
        ));
        out.push_str(&format!(
            "| human total inferences (n={}) | {} |\n",
            all.inference_n, all.human_inferences
        ));
        out.push_str(&format!(
            "| generated total inferences (n={}) | {} |\n",
            all.inference_n, all.generated_inferences
        ));
        out.push_str(&format!(
            "| aggregate inference ratio (n={}) | {} |\n",
            all.inference_n,
            fmt_ratio(all.inference_aggregate())
        ));
        out.push_str(&format!(
            "| per-task inference ratio median (n={}) | {} |\n",
            all.inference_n,
            fmt_ratio(median(all.inference_ratios.clone()))
        ));
    }

    out.push_str("\n### Unmatched rows (excluded from every aggregate)\n\n");
    if unmatched.is_empty() {
        out.push_str("None: every row is a matched pair.\n");
    } else {
        out.push_str(
            "| arch | task | human theorem | generated theorem | human s | generated s | reason |\n",
        );
        out.push_str("|---|---|---|---|---:|---:|---|\n");
        for p in &unmatched {
            out.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} |\n",
                p.arch,
                p.label(),
                p.human_proof
                    .as_ref()
                    .map(|m| m.theorem.as_str())
                    .unwrap_or("n/a"),
                p.generated_proof
                    .as_ref()
                    .map(|m| m.theorem.as_str())
                    .unwrap_or("n/a"),
                fmt_secs(p.human_seconds()),
                fmt_secs(p.generated_seconds()),
                p.refusal_reason(),
            ));
        }
    }

    out.push_str(
        "\n### Descriptive side totals (all parsed rows; NOT an aggregate over matched pairs)\n\n",
    );
    out.push_str("| side | rows | rows with the value | total |\n|---|---:|---:|---:|\n");
    let h_secs: f64 = human.tasks.iter().filter_map(|t| t.seconds).sum();
    let g_secs: f64 = generated.tasks.iter().filter_map(|t| t.seconds).sum();
    let h_inf: u128 = human
        .tasks
        .iter()
        .filter_map(|t| t.inferences)
        .map(|v| v as u128)
        .sum();
    let g_inf: u128 = generated
        .tasks
        .iter()
        .filter_map(|t| t.inferences)
        .map(|v| v as u128)
        .sum();
    out.push_str(&format!(
        "| human seconds | {} | {} | {:.3} s |\n",
        human.tasks.len(),
        human.tasks.iter().filter(|t| t.seconds.is_some()).count(),
        h_secs
    ));
    out.push_str(&format!(
        "| generated seconds | {} | {} | {:.3} s |\n",
        generated.tasks.len(),
        generated
            .tasks
            .iter()
            .filter(|t| t.seconds.is_some())
            .count(),
        g_secs
    ));
    out.push_str(&format!(
        "| human inferences | {} | {} | {} |\n",
        human.tasks.len(),
        human
            .tasks
            .iter()
            .filter(|t| t.inferences.is_some())
            .count(),
        h_inf
    ));
    out.push_str(&format!(
        "| generated inferences | {} | {} | {} |\n",
        generated.tasks.len(),
        generated
            .tasks
            .iter()
            .filter(|t| t.inferences.is_some())
            .count(),
        g_inf
    ));

    let warn = CHECKTIME_BUDGET_SECONDS * CHECKTIME_WARN_FRACTION;
    out.push_str(&format!(
        "\n### XII.F9.2 budget: {:.0} s (70% = {:.0} s)\n\n",
        CHECKTIME_BUDGET_SECONDS, warn
    ));
    let over: Vec<&Pair> = rows
        .iter()
        .filter(|p| p.matched().is_some())
        .filter(|p| p.generated_seconds().is_some_and(|g| g > warn))
        .collect();
    if matched_count == 0 {
        out.push_str("No matched generated task to test (n=0).\n");
    } else if over.is_empty() {
        out.push_str(&format!(
            "No matched generated task exceeds {warn:.0} s (n={matched_count}).\n"
        ));
    } else {
        out.push_str("| arch | task | generated s | % of 120 s | over 70% | over 100% |\n");
        out.push_str("|---|---|---:|---:|---|---|\n");
        for p in over {
            let g = p.generated_seconds().unwrap_or(0.0);
            out.push_str(&format!(
                "| {} | {} | {:.3} | {:.1} | {} | {} |\n",
                p.arch,
                p.label(),
                g,
                g / CHECKTIME_BUDGET_SECONDS * 100.0,
                py_bool(g > warn),
                py_bool(g > CHECKTIME_BUDGET_SECONDS),
            ));
        }
    }

    out.push_str("\n### Running time provenance\n\n");
    out.push_str(
        "`seconds` is the last `Running time: <x> sec, Start unixtime: ...` line of the\n\
         task's `.correct` file. Upstream's `build-proof.sh` prints it around\n\
         `loadt <proof>` plus `check_axioms ()`; it excludes HOL Light load and the\n\
         native compile.\n\n",
    );
    out.push_str("| arch | task | human `.correct` | generated make log |\n");
    out.push_str("|---|---|---|---|\n");
    for p in &rows {
        let correct = match human.tree.as_deref() {
            Some(t) => format!("{t}/{}/{}/bignum_{}_{}.correct", p.arch, p.dir, p.op, p.fam),
            None => "(tree not recorded)".to_string(),
        };
        out.push_str(&format!(
            "| {} | {} | `{}` | `{}` |\n",
            p.arch,
            p.label(),
            correct,
            p.generated
                .as_ref()
                .map(|t| t.log.as_str())
                .unwrap_or("n/a"),
        ));
    }

    out.push_str(&format!(
        "\nMATCHED {matched_count}/{} PAIRS; UNMATCHED {}; HUMAN ROWS {}; GENERATED ROWS {}\n",
        rows.len(),
        rows.len() - matched_count,
        human.tasks.len(),
        generated.tasks.len(),
    ));
    out
}

/// `dropin checktime-table --human <json> --generated <json>
/// [--human-kernel K] [--generated-kernel K]`.
pub fn checktime_table(args: &[String]) -> Result<i32, String> {
    let human_path = arg_value(args, "--human").ok_or("--human is required")?;
    let generated_path = arg_value(args, "--generated").ok_or("--generated is required")?;
    let human_kernel = arg_value(args, "--human-kernel").unwrap_or("");
    let generated_kernel = arg_value(args, "--generated-kernel").unwrap_or("");
    let human = read_timed_tasks(Path::new(human_path), TreeKind::Human, human_kernel)?;
    let generated = read_timed_tasks(
        Path::new(generated_path),
        TreeKind::Generated,
        generated_kernel,
    )?;
    if human.tasks.is_empty() || generated.tasks.is_empty() {
        return Err(format!(
            "need both task lists: human={} generated={}",
            human.tasks.len(),
            generated.tasks.len()
        ));
    }
    let text = render_checktime(&human, &generated);
    print!("{text}");
    let rows = pair_rows(&human, &generated);
    let matched = rows.iter().filter(|p| p.matched().is_some()).count();
    Ok(if !rows.is_empty() && matched == rows.len() {
        0
    } else {
        1
    })
}

fn json_opt_u64(v: Option<u64>) -> String {
    v.map(|n| n.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn json_str_slice(items: &[String]) -> String {
    let body: Vec<String> = items.iter().map(|s| json_str(s)).collect();
    format!("[{}]", body.join(", "))
}

/// Render the inference table. This is the XIV.B2 artefact: only matched
/// pairs are aggregated, and the denominator is printed on every total.
fn render_matched(human: &TimedFile, generated: &TimedFile) -> String {
    let rows = pair_rows(human, generated);
    let matched: Vec<&MatchedComparison> = rows.iter().filter_map(|p| p.matched()).collect();
    let matched_count = matched.len();
    let unmatched: Vec<&Pair> = rows.iter().filter(|p| p.refused()).collect();
    let mut all = Group::default();
    for m in &matched {
        all.add(m);
    }

    let mut out = String::new();
    out.push_str("## Matched inference table (XIV.B2)\n\n");
    out.push_str(
        "A row is published only as a pair. `compare` requires the exact theorem\n\
         identity, both judgments successful (`exit == 0`, `check_axioms`, theorem\n\
         proved), and identical measurement scopes. `human inf` is the proof-body\n\
         primitive-inference delta: the shared HOL Light bootstrap is excluded on\n\
         both sides.\n\n",
    );
    out.push_str(
        "| arch | task | human theorem | generated theorem | human inf | generated inf | human s | generated s | verdict |\n",
    );
    out.push_str("|---|---|---|---|---:|---:|---:|---:|---|\n");
    for p in &rows {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            p.arch,
            p.label(),
            p.human_proof
                .as_ref()
                .map(|m| m.theorem.as_str())
                .unwrap_or("n/a"),
            p.generated_proof
                .as_ref()
                .map(|m| m.theorem.as_str())
                .unwrap_or("n/a"),
            fmt_u64(p.human_inferences()),
            fmt_u64(p.generated_inferences()),
            fmt_secs(p.human_seconds()),
            fmt_secs(p.generated_seconds()),
            fmt_match_verdict(p),
        ));
    }

    out.push_str("\n### Counts\n\n");
    out.push_str("| quantity | value |\n|---|---:|\n");
    out.push_str(&format!("| human rows parsed | {} |\n", human.tasks.len()));
    out.push_str(&format!(
        "| generated rows parsed | {} |\n",
        generated.tasks.len()
    ));
    out.push_str(&format!("| task rows (union of keys) | {} |\n", rows.len()));
    out.push_str(&format!("| matched pairs (n) | {matched_count} |\n"));
    out.push_str(&format!(
        "| unmatched rows | {} |\n",
        rows.len() - matched_count
    ));

    out.push_str("\n### Matched totals (denominator: n matched pairs)\n\n");
    out.push_str("| metric | value |\n|---|---:|\n");
    out.push_str(&format!("| matched pairs (n) | {matched_count} |\n"));
    if matched_count == 0 {
        out.push_str("| human inferences (n=0) | n/a (no matched pairs) |\n");
        out.push_str("| generated inferences (n=0) | n/a (no matched pairs) |\n");
        out.push_str("| aggregate inference ratio | n/a (n=0; not published) |\n");
        out.push_str("| per-task inference ratio median | n/a (n=0) |\n");
        out.push_str("| human seconds (n=0) | n/a (no matched pairs) |\n");
        out.push_str("| generated seconds (n=0) | n/a (no matched pairs) |\n");
        out.push_str("| aggregate seconds ratio | n/a (n=0; not published) |\n");
    } else {
        out.push_str(&format!(
            "| human inferences (n={}) | {} |\n",
            all.inference_n, all.human_inferences
        ));
        out.push_str(&format!(
            "| generated inferences (n={}) | {} |\n",
            all.inference_n, all.generated_inferences
        ));
        out.push_str(&format!(
            "| aggregate inference ratio (n={}) | {} |\n",
            all.inference_n,
            fmt_ratio(all.inference_aggregate())
        ));
        out.push_str(&format!(
            "| per-task inference ratio median (n={}) | {} |\n",
            all.inference_n,
            fmt_ratio(median(all.inference_ratios.clone()))
        ));
        out.push_str(&format!(
            "| human seconds (n={}) | {:.3} |\n",
            all.seconds_n, all.human_seconds
        ));
        out.push_str(&format!(
            "| generated seconds (n={}) | {:.3} |\n",
            all.seconds_n, all.generated_seconds
        ));
        out.push_str(&format!(
            "| aggregate seconds ratio (n={}) | {} |\n",
            all.seconds_n,
            fmt_ratio(all.seconds_aggregate())
        ));
    }

    out.push_str("\n### Unmatched rows (excluded from every aggregate)\n\n");
    if unmatched.is_empty() {
        out.push_str("None: every row is a matched pair.\n");
    } else {
        out.push_str(
            "| arch | task | human theorem | generated theorem | human inf | generated inf | reason |\n",
        );
        out.push_str("|---|---|---|---|---:|---:|---|\n");
        for p in &unmatched {
            out.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} |\n",
                p.arch,
                p.label(),
                p.human_proof
                    .as_ref()
                    .map(|m| m.theorem.as_str())
                    .unwrap_or("n/a"),
                p.generated_proof
                    .as_ref()
                    .map(|m| m.theorem.as_str())
                    .unwrap_or("n/a"),
                fmt_u64(p.human_inferences()),
                fmt_u64(p.generated_inferences()),
                p.refusal_reason(),
            ));
        }
    }

    out.push_str(
        "\n### Descriptive side totals (all parsed rows; NOT an aggregate over matched pairs)\n\n",
    );
    out.push_str(
        "| side | rows | rows with inferences | total inferences |\n|---|---:|---:|---:|\n",
    );
    let h_inf: u128 = human
        .tasks
        .iter()
        .filter_map(|t| t.inferences)
        .map(|v| v as u128)
        .sum();
    let g_inf: u128 = generated
        .tasks
        .iter()
        .filter_map(|t| t.inferences)
        .map(|v| v as u128)
        .sum();
    out.push_str(&format!(
        "| human | {} | {} | {} |\n",
        human.tasks.len(),
        human
            .tasks
            .iter()
            .filter(|t| t.inferences.is_some())
            .count(),
        h_inf
    ));
    out.push_str(&format!(
        "| generated | {} | {} | {} |\n",
        generated.tasks.len(),
        generated
            .tasks
            .iter()
            .filter(|t| t.inferences.is_some())
            .count(),
        g_inf
    ));

    out.push_str(&format!(
        "\nMATCHED {matched_count}/{} PAIRS; UNMATCHED {}; HUMAN ROWS {}; GENERATED ROWS {}\n",
        rows.len(),
        rows.len() - matched_count,
        human.tasks.len(),
        generated.tasks.len(),
    ));
    out
}

fn matched_json(
    human: &TimedFile,
    generated: &TimedFile,
    human_path: &str,
    generated_path: &str,
) -> String {
    let rows = pair_rows(human, generated);
    let matched: Vec<&MatchedComparison> = rows.iter().filter_map(|p| p.matched()).collect();
    let matched_count = matched.len();
    let mut all = Group::default();
    for m in &matched {
        all.add(m);
    }
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!("  \"generated_at\": {},\n", now_unix()));
    out.push_str("  \"label\": \"ix60-matched-inference\",\n");
    out.push_str(&format!("  \"human_file\": {},\n", json_str(human_path)));
    out.push_str(&format!(
        "  \"generated_file\": {},\n",
        json_str(generated_path)
    ));
    out.push_str(&format!(
        "  \"human_kernel_build\": {},\n",
        json_str(&human.scope.kernel_build)
    ));
    out.push_str(&format!(
        "  \"generated_kernel_build\": {},\n",
        json_str(&generated.scope.kernel_build)
    ));
    out.push_str(&format!(
        "  \"human_tree\": {},\n",
        json_opt_str(human.tree.as_deref())
    ));
    out.push_str(&format!(
        "  \"scope\": {{\"s2n_bignum_pin\": {}, \"jobs\": {}, \"force\": {}}},\n",
        json_str(&human.scope.s2n_pin),
        human.scope.jobs,
        human.scope.force
    ));
    out.push_str(&format!(
        "  \"counts\": {{\"human_rows\": {}, \"generated_rows\": {}, \"task_rows\": {}, \"matched_pairs\": {}, \"unmatched\": {}}},\n",
        human.tasks.len(),
        generated.tasks.len(),
        rows.len(),
        matched_count,
        rows.len() - matched_count
    ));
    let (hi, gi, ir, hs, gs, sr) = if matched_count == 0 {
        (None, None, None, None, None, None)
    } else {
        (
            Some(all.human_inferences),
            Some(all.generated_inferences),
            all.inference_aggregate(),
            Some(all.human_seconds),
            Some(all.generated_seconds),
            all.seconds_aggregate(),
        )
    };
    out.push_str(&format!(
        "  \"matched_totals\": {{\"denominator\": {}, \"human_inferences\": {}, \"generated_inferences\": {}, \"inference_ratio\": {}, \"human_seconds\": {}, \"generated_seconds\": {}, \"seconds_ratio\": {}}},\n",
        matched_count,
        hi.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string()),
        gi.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string()),
        json_opt_num(ir),
        json_opt_num(hs),
        json_opt_num(gs),
        json_opt_num(sr),
    ));
    out.push_str("  \"tasks\": [\n");
    for (i, p) in rows.iter().enumerate() {
        let comma = if i + 1 == rows.len() { "" } else { "," };
        let reasons: Vec<String> = p.reasons.iter().map(ToString::to_string).collect();
        out.push_str(&format!(
            "    {{\"arch\": {}, \"dir\": {}, \"op\": {}, \"family\": {}, \"human_theorem\": {}, \"generated_theorem\": {}, \"human_inferences\": {}, \"generated_inferences\": {}, \"human_inferences_whole_file\": {}, \"human_seconds\": {}, \"generated_seconds\": {}, \"matched\": {}, \"verdict\": {}, \"reasons\": {}}}{}\n",
            json_str(&p.arch),
            json_str(&p.dir),
            json_str(&p.op),
            json_str(&p.fam),
            json_opt_str(p.human_proof.as_ref().map(|m| m.theorem.as_str())),
            json_opt_str(p.generated_proof.as_ref().map(|m| m.theorem.as_str())),
            json_opt_u64(p.human_inferences()),
            json_opt_u64(p.generated_inferences()),
            json_opt_u64(p.human.as_ref().and_then(|t| t.inferences_whole_file)),
            json_opt_num(p.human_seconds()),
            json_opt_num(p.generated_seconds()),
            p.matched().is_some(),
            json_str(&p.refusal_reason()),
            json_str_slice(&reasons),
            comma
        ));
    }
    out.push_str("  ],\n");
    out.push_str(&format!(
        "  \"ok\": {},\n  \"total\": {}\n}}\n",
        matched_count,
        rows.len()
    ));
    out
}

/// `dropin matched-table --human <json> --generated <json> [--json <path>]
/// [--human-kernel K] [--generated-kernel K]`.
pub fn matched_table(args: &[String]) -> Result<i32, String> {
    let human_path = arg_value(args, "--human").ok_or("--human is required")?;
    let generated_path = arg_value(args, "--generated").ok_or("--generated is required")?;
    let human_kernel = arg_value(args, "--human-kernel").unwrap_or("");
    let generated_kernel = arg_value(args, "--generated-kernel").unwrap_or("");
    let human = read_timed_tasks(Path::new(human_path), TreeKind::Human, human_kernel)?;
    let generated = read_timed_tasks(
        Path::new(generated_path),
        TreeKind::Generated,
        generated_kernel,
    )?;
    if human.tasks.is_empty() || generated.tasks.is_empty() {
        return Err(format!(
            "need both task lists: human={} generated={}",
            human.tasks.len(),
            generated.tasks.len()
        ));
    }
    let text = render_matched(&human, &generated);
    print!("{text}");
    if let Some(path) = arg_value(args, "--json") {
        let path = Path::new(path);
        ensure_parent(path)?;
        let body = matched_json(&human, &generated, human_path, generated_path);
        fs::write(path, body.as_bytes()).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    let rows = pair_rows(&human, &generated);
    let matched = rows.iter().filter(|p| p.matched().is_some()).count();
    Ok(if !rows.is_empty() && matched == rows.len() {
        0
    } else {
        1
    })
}

// ---------------------------------------------------------------------------
// obligation: decide whether the human and generated proofs prove the same
// statement (XIII.E3). Everything here is a string comparison of the two
// conclusions printed by the built `.correct` files.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ObligationVerdict {
    SameStatement,
    HumanStronger,
    HumanWeaker,
    Incomparable,
    Unclassified,
    Missing,
}

impl ObligationVerdict {
    fn as_str(self) -> &'static str {
        match self {
            ObligationVerdict::SameStatement => "same statement",
            ObligationVerdict::HumanStronger => "different: human strictly stronger",
            ObligationVerdict::HumanWeaker => "different: human strictly weaker",
            ObligationVerdict::Incomparable => {
                "different: neither strictly stronger nor strictly weaker (incomparable)"
            }
            ObligationVerdict::Unclassified => "different: unclassified",
            ObligationVerdict::Missing => "missing conclusion",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Markers {
    human_returnaddress: bool,
    generated_returnaddress: bool,
    human_pc_returnaddress: bool,
    generated_pc_offset: bool,
    human_frame_abi: bool,
    generated_frame_abi: bool,
    human_x30_pre: bool,
    human_rsp_pre: bool,
    human_stack_load_pre: bool,
    human_rsp_post: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FirstDifference {
    /// Character offset (not bytes) of the first mismatch in the normalised
    /// conclusions.
    offset: usize,
    human: String,
    generated: String,
}

#[derive(Debug, Clone, PartialEq)]
struct Obligation {
    arch: String,
    dir: String,
    op: String,
    fam: String,
    human_theorem: String,
    generated_theorem: String,
    human_conclusion: Option<String>,
    generated_conclusion: Option<String>,
    same: bool,
    first_difference: Option<FirstDifference>,
    markers: Markers,
    verdict: ObligationVerdict,
    description: String,
    error: Option<String>,
}

fn normalise_conclusion(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Extract `<prefix><name> ... |- <conclusion>` from a `.correct` file. Two
/// printers are accepted: `val <name> : thm = |- ...` (upstream's printer) and
/// `VAL_UNCHANGED <name> |- ...` (the axiom-stamp instrument).
fn extract_conclusion(text: &str, name: &str) -> Option<String> {
    let prefixes = [
        format!("val {name} : thm = |- "),
        format!("VAL_UNCHANGED {name} |- "),
    ];
    let bare = format!("val {name} : thm =");
    let mut collecting = false;
    let mut buf = String::new();
    for line in text.lines() {
        if !collecting {
            let hit = prefixes
                .iter()
                .find_map(|p| line.strip_prefix(p).map(str::to_string));
            if let Some(rest) = hit {
                collecting = true;
                buf.push_str(&rest);
            } else if line.starts_with(&bare) {
                // The marker exists but carries no `|- ` conclusion.
                return Some(String::new());
            }
            continue;
        }
        if line.starts_with("val ")
            || line.starts_with("VAL_")
            || line.starts_with("T4_DONE")
            || line.starts_with("EQUALS_THM")
            || line.starts_with("Running time:")
            || line.starts_with("STAMP_")
            || line.starts_with("CPU time")
            || line.starts_with("File ")
            || line.starts_with("Exception")
            || line.trim().is_empty()
            // HOL's printer indents every continuation line of a term, so a
            // line at column 0 is the next programme output, not the term.
            || !line.starts_with(char::is_whitespace)
        {
            break;
        }
        buf.push(' ');
        buf.push_str(line);
    }
    if collecting {
        Some(normalise_conclusion(&buf))
    } else {
        None
    }
}

fn first_difference(human: &str, generated: &str) -> Option<FirstDifference> {
    if human == generated {
        return None;
    }
    let h: Vec<char> = human.chars().collect();
    let g: Vec<char> = generated.chars().collect();
    let mut i = 0;
    while i < h.len() && i < g.len() && h[i] == g[i] {
        i += 1;
    }
    let start = i.saturating_sub(60);
    let human_ctx: String = h[start..(i + 60).min(h.len())].iter().collect();
    let generated_ctx: String = g[start..(i + 60).min(g.len())].iter().collect();
    Some(FirstDifference {
        offset: i,
        human: human_ctx,
        generated: generated_ctx,
    })
}

fn markers_of(human: &str, generated: &str) -> Markers {
    Markers {
        human_returnaddress: human.contains("returnaddress"),
        generated_returnaddress: generated.contains("returnaddress"),
        human_pc_returnaddress: human.contains("read PC s = returnaddress")
            || human.contains("read RIP s = returnaddress"),
        generated_pc_offset: generated.contains("read PC s = word (pc +")
            || generated.contains("read RIP s = word (pc +"),
        human_frame_abi: human.contains("MAYCHANGE_REGS_AND_FLAGS_PERMITTED_BY_ABI"),
        generated_frame_abi: generated.contains("MAYCHANGE_REGS_AND_FLAGS_PERMITTED_BY_ABI"),
        human_x30_pre: human.contains("read X30 s = returnaddress"),
        human_rsp_pre: human.contains("read RSP s = stackpointer"),
        human_stack_load_pre: human
            .contains("read (memory :> bytes64 stackpointer) s = returnaddress"),
        human_rsp_post: human.contains("read RSP s = word_add stackpointer (word 8)"),
    }
}

fn describe_obligation(m: &Markers) -> String {
    let mut parts: Vec<String> = Vec::new();
    if !m.human_returnaddress && !m.generated_returnaddress {
        parts.push("neither conclusion mentions `returnaddress`".to_string());
    } else if m.human_returnaddress && !m.generated_returnaddress {
        parts.push("the human conclusion quantifies a caller-provided `returnaddress` (and, on x86, `stackpointer`) that the generated conclusion does not mention".to_string());
    }
    if m.human_x30_pre {
        parts.push("the human precondition assumes `read X30 s = returnaddress`".to_string());
    }
    if m.human_rsp_pre || m.human_stack_load_pre {
        parts.push("the human precondition assumes the stack pointer and the saved return address in memory".to_string());
    }
    if m.human_pc_returnaddress {
        parts.push("the human postcondition asserts `PC = returnaddress`".to_string());
    }
    if m.generated_pc_offset {
        parts.push("the generated postcondition asserts `PC = pc + N` for a fixed N".to_string());
    }
    if m.human_rsp_post {
        parts.push("the human postcondition asserts `RSP = stackpointer + 8`".to_string());
    }
    if m.human_frame_abi && !m.generated_frame_abi {
        parts.push("the human frame is `MAYCHANGE_REGS_AND_FLAGS_PERMITTED_BY_ABI` (broader: it permits every ABI-clobberable register), while the generated frame is an explicit `MAYCHANGE` list (narrower)".to_string());
    }
    parts.join("; ")
}

fn obligation_verdict(human: &str, generated: &str, m: &Markers) -> ObligationVerdict {
    if human == generated {
        return ObligationVerdict::SameStatement;
    }
    let human_extra_post = m.human_pc_returnaddress && !m.generated_returnaddress;
    let human_extra_pre = m.human_x30_pre || m.human_rsp_pre || m.human_stack_load_pre;
    let human_broader_frame = m.human_frame_abi && !m.generated_frame_abi;
    match (human_extra_post, human_extra_pre, human_broader_frame) {
        (true, true, _) | (true, _, true) => ObligationVerdict::Incomparable,
        (true, false, false) => ObligationVerdict::HumanStronger,
        (false, true, _) | (false, _, true) => ObligationVerdict::HumanWeaker,
        (false, false, false) => ObligationVerdict::Unclassified,
    }
}

fn build_obligation(
    arch: &str,
    dir: &str,
    op: &str,
    fam: &str,
    human_path: &Path,
    generated_path: &Path,
    generated_subroutine: bool,
) -> Obligation {
    let human_theorem = format!(
        "BIGNUM_{}_{}_SUBROUTINE_CORRECT",
        op.to_uppercase(),
        fam.to_uppercase()
    );
    let generated_theorem = if generated_subroutine {
        human_theorem.clone()
    } else {
        format!(
            "BIGNUM_{}_{}_CORRECT",
            op.to_uppercase(),
            fam.to_uppercase()
        )
    };
    let mut o = Obligation {
        arch: arch.to_string(),
        dir: dir.to_string(),
        op: op.to_string(),
        fam: fam.to_string(),
        human_theorem: human_theorem.clone(),
        generated_theorem: generated_theorem.clone(),
        human_conclusion: None,
        generated_conclusion: None,
        same: false,
        first_difference: None,
        markers: Markers::default(),
        verdict: ObligationVerdict::Missing,
        description: String::new(),
        error: None,
    };
    let human_text = match fs::read_to_string(human_path) {
        Ok(t) => t,
        Err(e) => {
            o.error = Some(format!("{}: {e}", human_path.display()));
            return o;
        }
    };
    let generated_text = match fs::read_to_string(generated_path) {
        Ok(t) => t,
        Err(e) => {
            o.error = Some(format!("{}: {e}", generated_path.display()));
            return o;
        }
    };
    let h = extract_conclusion(&human_text, &human_theorem);
    let g = extract_conclusion(&generated_text, &generated_theorem);
    if h.is_none() || g.is_none() {
        o.error = Some(format!(
            "conclusion not found: human={} generated={}",
            h.is_some(),
            g.is_some()
        ));
        o.human_conclusion = h;
        o.generated_conclusion = g;
        return o;
    }
    let h = h.unwrap();
    let g = g.unwrap();
    o.same = h == g;
    o.first_difference = first_difference(&h, &g);
    o.markers = markers_of(&h, &g);
    o.verdict = obligation_verdict(&h, &g, &o.markers);
    o.description = describe_obligation(&o.markers);
    o.human_conclusion = Some(h);
    o.generated_conclusion = Some(g);
    o
}

fn obligation_json(obligations: &[Obligation], human_tree: &str, generated_tree: &str) -> String {
    let same = obligations.iter().filter(|o| o.same).count();
    let missing = obligations
        .iter()
        .filter(|o| o.verdict == ObligationVerdict::Missing)
        .count();
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!("  \"generated_at\": {},\n", now_unix()));
    out.push_str(&format!("  \"human_tree\": {},\n", json_str(human_tree)));
    out.push_str(&format!(
        "  \"generated_tree\": {},\n",
        json_str(generated_tree)
    ));
    out.push_str(&format!(
        "  \"counts\": {{\"tasks\": {}, \"same\": {}, \"different\": {}, \"missing\": {}}},\n",
        obligations.len(),
        same,
        obligations.len() - same - missing,
        missing
    ));
    out.push_str("  \"tasks\": [\n");
    for (i, o) in obligations.iter().enumerate() {
        let comma = if i + 1 == obligations.len() { "" } else { "," };
        let fd = o
            .first_difference
            .as_ref()
            .map(|d| {
                format!(
                    "{{\"offset\": {}, \"human\": {}, \"generated\": {}}}",
                    d.offset,
                    json_str(&d.human),
                    json_str(&d.generated)
                )
            })
            .unwrap_or_else(|| "null".to_string());
        out.push_str(&format!(
            "    {{\"arch\": {}, \"dir\": {}, \"op\": {}, \"family\": {}, \"human_theorem\": {}, \"generated_theorem\": {}, \"same\": {}, \"verdict\": {}, \"first_difference\": {}, \"description\": {}, \"human_conclusion\": {}, \"generated_conclusion\": {}, \"error\": {}}}{}\n",
            json_str(&o.arch),
            json_str(&o.dir),
            json_str(&o.op),
            json_str(&o.fam),
            json_str(&o.human_theorem),
            json_str(&o.generated_theorem),
            o.same,
            json_str(o.verdict.as_str()),
            fd,
            json_str(&o.description),
            json_opt_str(o.human_conclusion.as_deref()),
            json_opt_str(o.generated_conclusion.as_deref()),
            json_opt_str(o.error.as_deref()),
            comma
        ));
    }
    out.push_str("  ]\n}\n");
    out
}

/// `dropin obligation --human-tree <dir> --generated-tree <dir> [--json P]`.
pub fn obligation(args: &[String]) -> Result<i32, String> {
    let human_tree =
        PathBuf::from(arg_value(args, "--human-tree").ok_or("--human-tree is required")?);
    let generated_tree =
        PathBuf::from(arg_value(args, "--generated-tree").ok_or("--generated-tree is required")?);
    let generated_subroutine = arg_value(args, "--generated-theorem")
        .map(|v| v == "subroutine")
        .unwrap_or(false);
    let planned = plan(&SweepArgs {
        arch: "all".to_string(),
        answers: None,
        dropin: repo_root().join(".bench/dropin"),
        gen: repo_root().join(".bench/dropin-gen"),
        hol: repo_root().join(".bench/hol-light-stamp2"),
        upstream: repo_root().join(".bench/s2n-bignum"),
        dry_run: false,
        force: true,
        timeout: 900,
        json: None,
        stamp: false,
    });
    let mut obligations = Vec::with_capacity(planned.len());
    for p in &planned {
        let human_path = human_tree
            .join(&p.arch)
            .join(&p.dir)
            .join(format!("{}.correct", p.base));
        let generated_path = generated_tree
            .join(&p.arch)
            .join(&p.dir)
            .join(format!("{}.correct", p.base));
        obligations.push(build_obligation(
            &p.arch,
            &p.dir,
            &p.op,
            &p.fam,
            &human_path,
            &generated_path,
            generated_subroutine,
        ));
    }

    let same = obligations.iter().filter(|o| o.same).count();
    let missing = obligations
        .iter()
        .filter(|o| o.verdict == ObligationVerdict::Missing)
        .count();
    println!("## Obligation comparison (XIII.E3)\n");
    println!("| arch | task | human theorem | generated theorem | same? | verdict |");
    println!("|---|---|---|---|---|---|");
    for o in &obligations {
        println!(
            "| {} | {}/{}_{} | {} | {} | {} | {} |",
            o.arch,
            o.dir,
            o.op,
            o.fam,
            o.human_theorem,
            o.generated_theorem,
            py_bool(o.same),
            o.verdict.as_str()
        );
    }
    println!("\n### Per-task evidence\n");
    for o in &obligations {
        println!("#### {}/{} {}", o.arch, o.dir, o.op);
        if let Some(err) = &o.error {
            println!("- error: {err}\n");
            continue;
        }
        if let Some(d) = &o.first_difference {
            println!("- first difference at character {}:", d.offset);
            println!("  - human:     `{}`", d.human);
            println!("  - generated: `{}`", d.generated);
        }
        println!("- description: {}", o.description);
        println!("- verdict: {}\n", o.verdict.as_str());
    }
    println!(
        "OBLIGATIONS total={} same={} different={} missing={}",
        obligations.len(),
        same,
        obligations.len() - same - missing,
        missing
    );

    if let Some(path) = arg_value(args, "--json") {
        let path = Path::new(path);
        ensure_parent(path)?;
        let body = obligation_json(
            &obligations,
            &human_tree.display().to_string(),
            &generated_tree.display().to_string(),
        );
        fs::write(path, body.as_bytes()).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(if missing == 0 { 0 } else { 1 })
}

/// One precondition check: is the generated `_CORRECT` conclusion the same
/// statement as the quotation of upstream's `_CORRECT`?
#[derive(Debug, Clone, PartialEq, Eq)]
struct PremiseCheck {
    arch: String,
    dir: String,
    op: String,
    fam: String,
    upstream: Option<String>,
    built: Option<String>,
    same: bool,
    error: Option<String>,
}

fn build_premise(
    arch: &str,
    dir: &str,
    op: &str,
    fam: &str,
    upstream_file: &Path,
    generated_correct: &Path,
) -> PremiseCheck {
    let name = format!(
        "BIGNUM_{}_{}_CORRECT",
        op.to_uppercase(),
        fam.to_uppercase()
    );
    let mut c = PremiseCheck {
        arch: arch.to_string(),
        dir: dir.to_string(),
        op: op.to_string(),
        fam: fam.to_string(),
        upstream: None,
        built: None,
        same: false,
        error: None,
    };
    let source = match fs::read_to_string(upstream_file) {
        Ok(t) => t,
        Err(e) => {
            c.error = Some(format!("{}: {e}", upstream_file.display()));
            return c;
        }
    };
    let text = match fs::read_to_string(generated_correct) {
        Ok(t) => t,
        Err(e) => {
            c.error = Some(format!("{}: {e}", generated_correct.display()));
            return c;
        }
    };
    let up = match extract_quotation(&source, &name) {
        Ok(q) => q,
        Err(e) => {
            c.error = Some(e);
            return c;
        }
    };
    let built = match extract_conclusion(&text, &name) {
        Some(b) => b,
        None => {
            c.error = Some(format!(
                "{} has no `{name}` conclusion",
                generated_correct.display()
            ));
            return c;
        }
    };
    let up_c = canonical_term(&up);
    let built_c = canonical_term(&built);
    c.same = up_c == built_c;
    c.upstream = Some(up_c);
    c.built = Some(built_c);
    c
}

/// `dropin premise --upstream-tree <dir> --generated-tree <dir> [--json P]`.
///
/// G1a of IX.64: our built `_CORRECT` conclusion equals upstream's `_CORRECT`
/// quotation, after canonicalising numeral notation and whitespace.
pub fn premise(args: &[String]) -> Result<i32, String> {
    let upstream_tree =
        PathBuf::from(arg_value(args, "--upstream-tree").ok_or("--upstream-tree is required")?);
    let generated_tree =
        PathBuf::from(arg_value(args, "--generated-tree").ok_or("--generated-tree is required")?);
    let planned = plan(&SweepArgs {
        arch: "all".to_string(),
        answers: None,
        dropin: repo_root().join(".bench/dropin"),
        gen: repo_root().join(".bench/dropin-gen"),
        hol: repo_root().join(".bench/hol-light"),
        upstream: repo_root().join(".bench/s2n-bignum"),
        dry_run: false,
        force: false,
        timeout: 900,
        json: None,
        stamp: false,
    });
    let checks: Vec<PremiseCheck> = planned
        .iter()
        .map(|p| {
            build_premise(
                &p.arch,
                &p.dir,
                &p.op,
                &p.fam,
                &upstream_tree
                    .join(&p.arch)
                    .join("proofs")
                    .join(format!("{}.ml", p.base)),
                &generated_tree
                    .join(&p.arch)
                    .join(&p.dir)
                    .join(format!("{}.correct", p.base)),
            )
        })
        .collect();
    let same = checks.iter().filter(|c| c.same).count();
    let missing = checks.iter().filter(|c| c.error.is_some()).count();
    println!("## G1a premise: generated _CORRECT vs upstream _CORRECT quotation\n");
    println!("| arch | task | same? | note |");
    println!("|---|---|---|---|");
    for c in &checks {
        println!(
            "| {} | {}/{}_{} | {} | {} |",
            c.arch,
            c.dir,
            c.op,
            c.fam,
            py_bool(c.same),
            c.error.as_deref().unwrap_or("")
        );
    }
    println!(
        "\nPREMISE total={} same={} different={} missing={}",
        checks.len(),
        same,
        checks.len() - same - missing,
        missing
    );
    for c in &checks {
        if !c.same && c.error.is_none() {
            println!("DIFF {} {}/{}_{}", c.arch, c.dir, c.op, c.fam);
            if let (Some(u), Some(b)) = (&c.upstream, &c.built) {
                let d = first_difference(u, b);
                if let Some(d) = d {
                    println!("  offset {}", d.offset);
                    println!("  upstream: {}", d.human);
                    println!("  built:    {}", d.generated);
                }
            }
        }
    }
    if let Some(path) = arg_value(args, "--json") {
        let path = Path::new(path);
        ensure_parent(path)?;
        let mut out = String::new();
        out.push_str("{\n");
        out.push_str(&format!(
            "  \"counts\": {{\"tasks\": {}, \"same\": {}, \"different\": {}, \"missing\": {}}},\n",
            checks.len(),
            same,
            checks.len() - same - missing,
            missing
        ));
        out.push_str("  \"tasks\": [\n");
        for (i, c) in checks.iter().enumerate() {
            let comma = if i + 1 == checks.len() { "" } else { "," };
            out.push_str(&format!(
                "    {{\"arch\": {}, \"dir\": {}, \"op\": {}, \"family\": {}, \"same\": {}, \"error\": {}}}{}\n",
                json_str(&c.arch),
                json_str(&c.dir),
                json_str(&c.op),
                json_str(&c.fam),
                c.same,
                json_opt_str(c.error.as_deref()),
                comma
            ));
        }
        out.push_str("  ]\n}\n");
        fs::write(path, out.as_bytes()).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(if same == checks.len() { 0 } else { 1 })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn sweep_requires_arch() {
        let err = parse_sweep_args(&[]).unwrap_err();
        assert!(err.contains("--arch is required"), "{err}");
    }

    #[test]
    fn sweep_rejects_unknown_architectures() {
        let err = parse_sweep_args(&args(&["--arch", "riscv"])).unwrap_err();
        assert!(err.contains("expected x86, arm or all"), "{err}");
    }

    #[test]
    fn sweep_rejects_unknown_flags() {
        let err = parse_sweep_args(&args(&["--arch", "x86", "--bogus"])).unwrap_err();
        assert!(err.contains("unknown argument"), "{err}");
    }

    #[test]
    fn sweep_parses_overrides() {
        let parsed = parse_sweep_args(&args(&[
            "--arch",
            "x86",
            "--dir",
            "/d",
            "--gen",
            "/g",
            "--hol",
            "/h",
            "--dry-run",
        ]))
        .unwrap();
        assert_eq!(parsed.arch, "x86");
        assert!(parsed.answers.is_none());
        assert_eq!(parsed.dropin, PathBuf::from("/d"));
        assert_eq!(parsed.gen, PathBuf::from("/g"));
        assert_eq!(parsed.hol, PathBuf::from("/h"));
        assert!(parsed.dry_run);
        assert!(!parsed.force);
        assert_eq!(parsed.timeout, 900);
        assert!(parsed.json.is_none());
    }

    #[test]
    fn sweep_parses_arm_and_all() {
        let arm = parse_sweep_args(&args(&["--arch", "arm"])).unwrap();
        assert_eq!(arm.arch, "arm");
        assert_eq!(arm.arches(), &["arm"]);
        let all = parse_sweep_args(&args(&["--arch", "all"])).unwrap();
        assert_eq!(all.arch, "all");
        assert_eq!(all.arches(), &["arm", "x86"]);
        let x86 = parse_sweep_args(&args(&["--arch", "x86"])).unwrap();
        assert_eq!(x86.arches(), &["x86"]);
    }

    #[test]
    fn sweep_parses_answers_force_timeout_json() {
        let parsed = parse_sweep_args(&args(&[
            "--arch",
            "all",
            "--answers",
            "/a",
            "--force",
            "--timeout",
            "120",
            "--json",
            "/j/out.json",
        ]))
        .unwrap();
        assert_eq!(parsed.answers, Some(PathBuf::from("/a")));
        assert!(parsed.force);
        assert_eq!(parsed.timeout, 120);
        assert_eq!(parsed.json, Some(PathBuf::from("/j/out.json")));
        // `--answers` overrides both architecture defaults.
        assert_eq!(
            parsed.answer_path("arm", "bignum_add_p521"),
            PathBuf::from("/a/bignum_add_p521.answer.txt")
        );
    }

    #[test]
    fn sweep_rejects_bad_timeout() {
        for raw in ["0", "-1", "soon"] {
            let err = parse_sweep_args(&args(&["--arch", "all", "--timeout", raw])).unwrap_err();
            assert!(err.contains("--timeout"), "{raw}: {err}");
        }
    }

    #[test]
    fn running_time_is_parsed() {
        let text = "val X : thm = |- T\n\
                    Running time: 46.000000 sec, Start unixtime: 1795501903.000000, \
                    End unixtime: 1795501949.000000\n";
        assert_eq!(running_time(text), Some(46.0));
        let two = "Running time: 1.500000 sec, Start unixtime: 1, End unixtime: 2\n\
                   Running time: 2.250000 sec, Start unixtime: 2, End unixtime: 4\n";
        assert_eq!(running_time(two), Some(2.25));
    }

    #[test]
    fn running_time_absent_is_none() {
        let text = "val X : thm = |- T\nFatal error: exception Failure(\"boom\")\n";
        assert_eq!(running_time(text), None);
    }

    #[test]
    fn theorem_marker_prefers_the_plain_name_and_falls_back_upstream() {
        let generated = "val BIGNUM_ADD_P521_CORRECT : thm = |- T\n";
        assert_eq!(
            theorem_marker(generated, "add", "p521").as_deref(),
            Some("BIGNUM_ADD_P521_CORRECT")
        );
        // Upstream's `collect-specs.sh` prints only the subroutine specs.
        let upstream = "val BIGNUM_ADD_P521_SUBROUTINE_CORRECT : thm = |- T\n\
                        val BIGNUM_ADD_P521_SUBROUTINE_SAFE : thm = |- T\n";
        assert_eq!(
            theorem_marker(upstream, "add", "p521").as_deref(),
            Some("BIGNUM_ADD_P521_SUBROUTINE_CORRECT")
        );
        assert_eq!(theorem_marker("nothing here\n", "add", "p521"), None);
    }

    #[test]
    fn timeout_exit_codes_are_classified() {
        assert!(timed_out(124));
        assert!(timed_out(137));
        assert!(!timed_out(0));
        assert!(!timed_out(2));
    }

    fn fixture(arch: &str, dir: &str, op: &str, fam: &str, exit: i32) -> TaskResult {
        TaskResult {
            arch: arch.to_string(),
            dir: dir.to_string(),
            op: op.to_string(),
            fam: fam.to_string(),
            exit: Some(exit),
            theorem: if exit == 0 {
                Some(format!(
                    "BIGNUM_{}_{}_CORRECT",
                    op.to_uppercase(),
                    fam.to_uppercase()
                ))
            } else {
                None
            },
            check_axioms: exit == 0,
            seconds: if exit == 0 { Some(46.0) } else { None },
            timeout: false,
            log: format!("logs-{arch}/{dir}_{op}_{fam}.log"),
            exception: exit != 0,
            stamp_required: false,
            stamp_report: None,
        }
    }

    #[test]
    fn json_writer_shape_on_two_results() {
        let results = vec![
            fixture("arm", "curve25519", "add", "p25519", 0),
            fixture("x86", "sm2", "neg", "sm2", 2),
        ];
        let text = results_json(
            "all",
            true,
            900,
            "5e0fc782",
            "d30e823",
            1795501903,
            false,
            &StampSummary::default(),
            &results,
        );
        let j = json::parse(text.as_bytes()).expect("writer output must parse");
        assert_eq!(
            j.get("generated_at").unwrap(),
            &json::Json::Num(1795501903.0)
        );
        assert_eq!(j.get("arch").unwrap().as_str(), Some("all"));
        assert_eq!(j.get("s2n_bignum_pin").unwrap().as_str(), Some("5e0fc782"));
        assert_eq!(j.get("hol_light_pin").unwrap().as_str(), Some("d30e823"));
        assert_eq!(j.get("cores").unwrap(), &json::Json::Num(1.0));
        let tasks = j.get("tasks").unwrap().as_array().unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].get("arch").unwrap().as_str(), Some("arm"));
        assert_eq!(tasks[0].get("exit").unwrap(), &json::Json::Num(0.0));
        assert_eq!(
            tasks[0].get("theorem").unwrap().as_str(),
            Some("BIGNUM_ADD_P25519_CORRECT")
        );
        assert_eq!(
            tasks[0].get("check_axioms").unwrap(),
            &json::Json::Bool(true)
        );
        assert_eq!(tasks[0].get("seconds").unwrap(), &json::Json::Num(46.0));
        assert_eq!(tasks[0].get("timeout").unwrap(), &json::Json::Bool(false));
        // A failed task carries a real exit code but no invented timing.
        assert_eq!(tasks[1].get("exit").unwrap(), &json::Json::Num(2.0));
        assert_eq!(tasks[1].get("theorem"), Some(&json::Json::Null));
        assert_eq!(tasks[1].get("seconds"), Some(&json::Json::Null));
        assert_eq!(j.get("ok").unwrap(), &json::Json::Num(1.0));
        assert_eq!(j.get("total").unwrap(), &json::Json::Num(2.0));
    }

    #[test]
    fn dry_run_plan_over_all_reports_27_assemblies() {
        // This is the list the `--dry-run` path walks before it prints
        // "DRY-RUN 27/27 assembled; make not invoked"; nothing here touches make.
        let parsed = parse_sweep_args(&args(&["--arch", "all", "--dry-run"])).unwrap();
        let planned = plan(&parsed);
        assert_eq!(planned.len(), 27);
        assert_eq!(planned.iter().filter(|p| p.arch == "arm").count(), 18);
        assert_eq!(planned.iter().filter(|p| p.arch == "x86").count(), 9);
        assert_eq!(planned[0].arch, "arm");
        assert_eq!(planned[0].label(), "curve25519/add_p25519");
        assert_eq!(planned[17].label(), "sm2/neg_sm2");
        assert_eq!(planned[18].arch, "x86");
        assert_eq!(planned[26].label(), "sm2/neg_sm2");
        // Per-architecture answer defaults are the ones the sweep actually uses.
        assert!(parsed
            .answer_path("arm", "bignum_add_p521")
            .ends_with(".bench/dropin-gen/answers-arm/bignum_add_p521.answer.txt"));
        assert!(parsed
            .answer_path("x86", "bignum_add_p521")
            .ends_with(".bench/dropin-gen/answers/bignum_add_p521.answer.txt"));
        // Single-architecture plans keep the historical x86 order.
        let x86 = plan(&parse_sweep_args(&args(&["--arch", "x86"])).unwrap());
        assert_eq!(x86.len(), 9);
        assert_eq!(x86[0].label(), "curve25519/add_p25519");
        assert_eq!(x86[1].label(), "secp256k1/add_p256k1");
        assert_eq!(x86[2].label(), "sm2/add_sm2");
        assert_eq!(x86[8].label(), "sm2/neg_sm2");
    }

    #[test]
    fn sweep_help_is_accepted_by_the_dispatcher() {
        // `--help` is not a sweep flag; the dispatcher must still reject it
        // cleanly rather than panicking.
        assert_eq!(run(&args(&["sweep", "--help"])), 2);
    }

    #[test]
    fn proof_body_mark_is_parsed_and_reported_separately() {
        let text = "STAMP_BEGIN X\nSTAMP X \nSTAMP_INF_BEFORE 10\n\
                    STAMP_INF_AFTER 250\nSTAMP_INFERENCES 240\n\
                    STAMP_INF_BODY_BEFORE 100\nSTAMP_INF_BODY 150\n\
                    STAMP_FOOTPRINT_BEGIN statement-level footprint, not the transitive derivation closure\n";
        let r = parse_stamp(text).expect("stamp lines present");
        assert_eq!(r.inference_delta, Some(240));
        assert_eq!(r.inference_body_before, Some(100));
        assert_eq!(r.inference_body_delta, Some(150));
        assert!(r.validate(Some("X")).is_ok());
        let mut bad = r;
        bad.inference_body_delta = Some(151);
        let err = bad.validate(Some("X")).unwrap_err();
        assert!(err.contains("body inference delta"), "{err}");
    }

    #[test]
    fn instrument_proof_marks_the_first_line_and_the_body_then_restores() {
        let dir = std::env::temp_dir().join(format!("tek-ix60-instr-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let p = dir.join("bignum_add_p256.ml");
        let original = "(* c *)\nneeds \"arm/proofs/base.ml\";;\nlet T = 1;;\n";
        fs::write(&p, original).unwrap();
        {
            let _g = instrument_proof(&p, "BIGNUM_ADD_P256_SUBROUTINE_CORRECT", "bignum_add_p256")
                .unwrap();
            let text = fs::read_to_string(&p).unwrap();
            assert!(text.starts_with("let stamp_mark_bignum_add_p256 = inference_count ();;\n"));
            let body_at = text
                .find("let stamp_body_bignum_add_p256 = inference_count ();;")
                .unwrap();
            let needs_at = text.find("needs \"arm/proofs/base.ml\";;").unwrap();
            assert!(
                body_at > needs_at,
                "body mark must follow the shared bootstrap"
            );
            assert!(text.contains("STAMP_INF_BODY_BEFORE"));
            assert!(text.contains("(string_of_thm BIGNUM_ADD_P256_SUBROUTINE_CORRECT)"));
        }
        assert_eq!(fs::read_to_string(&p).unwrap(), original);
        let _ = fs::remove_dir_all(&dir);
    }

    fn valid_stamp(theorem: &str) -> StampReport {
        StampReport {
            begun: true,
            begin_theorem: Some(theorem.to_string()),
            reported: true,
            stamp_theorem: Some(theorem.to_string()),
            axioms: vec![
                "id:1#aaaa=|- (eta : bool)".to_string(),
                "id:2#bbbb=|- (select : bool)".to_string(),
            ],
            inference_before: Some(100),
            inference_after: Some(250),
            inference_delta: Some(150),
            inference_body_before: None,
            inference_body_delta: None,
            registry: vec![
                (1, "aaaa".to_string(), "(eta : bool)".to_string()),
                (2, "bbbb".to_string(), "(select : bool)".to_string()),
            ],
            names: vec![
                ("ETA_AX".to_string(), vec![1]),
                ("SELECT_AX".to_string(), vec![2]),
            ],
            footprint: vec![(
                "const:=".to_string(),
                "primitive or unregistered".to_string(),
            )],
            footprint_label: true,
            malformed: Vec::new(),
        }
    }

    #[test]
    fn json_writer_emits_a_parseable_stamp_record() {
        let summary = StampSummary {
            registry: vec![
                (1, "aaaa".into(), "(eta : bool)".into()),
                (2, "bbbb".into(), "(select : bool)".into()),
            ],
            names: vec![("ETA_AX".into(), vec![1]), ("SELECT_AX".into(), vec![2])],
            ..StampSummary::default()
        };
        let mut r = fixture("arm", "curve25519", "add", "p25519", 0);
        r.stamp_required = true;
        let mut stamp = valid_stamp("BIGNUM_ADD_P25519_CORRECT");
        stamp.inference_before = Some(10);
        stamp.inference_after = Some(42);
        stamp.inference_delta = Some(32);
        stamp.registry = summary.registry.clone();
        stamp.names = summary.names.clone();
        r.stamp_report = Some(stamp);
        assert!(r.green(), "{:?}", r.stamp_error());
        let text = results_json(
            "arm",
            true,
            900,
            "5e0fc782",
            "d30e823",
            1,
            true,
            &summary,
            &[r],
        );
        let j = json::parse(text.as_bytes()).expect("stamp record must parse");
        assert_eq!(j.get("stamp").unwrap(), &json::Json::Bool(true));
        assert_eq!(
            j.get("axiom_registry_conflict").unwrap(),
            &json::Json::Bool(false)
        );
        let reg = j.get("axiom_registry").unwrap().as_array().unwrap();
        assert_eq!(reg.len(), 2);
        assert_eq!(reg[0].get("id").unwrap(), &json::Json::Num(1.0));
        assert_eq!(reg[0].get("digest").unwrap().as_str(), Some("aaaa"));
        assert_eq!(
            reg[0].get("conclusion").unwrap().as_str(),
            Some("(eta : bool)")
        );
        let basic = j.get("basic_axioms").unwrap().as_obj().unwrap();
        assert_eq!(basic.len(), 2);
        assert_eq!(
            basic.get("SELECT_AX").unwrap().as_array().unwrap()[0],
            json::Json::Num(2.0)
        );
        let task = &j.get("tasks").unwrap().as_array().unwrap()[0];
        assert_eq!(task.get("axioms").unwrap().as_array().unwrap().len(), 2);
        assert_eq!(task.get("inference_delta").unwrap(), &json::Json::Num(32.0));
        assert_eq!(task.get("stamp_ok").unwrap(), &json::Json::Bool(true));
        let fp = task.get("footprint").unwrap().as_array().unwrap();
        assert_eq!(fp[0].get("symbol").unwrap().as_str(), Some("const:="));
    }

    #[test]
    fn sweep_parses_stamp_flag() {
        let parsed = parse_sweep_args(&args(&["--arch", "all", "--stamp"])).unwrap();
        assert!(parsed.stamp);
        assert!(!parse_sweep_args(&args(&["--arch", "all"])).unwrap().stamp);
    }

    #[test]
    fn relative_paths_are_anchored_at_the_repo_root() {
        // `make` is run from `<dropin>/<arch>`; a relative `--hol` would break.
        let parsed = parse_sweep_args(&args(&[
            "--arch",
            "arm",
            "--hol",
            ".bench/hol-light-stamp",
            "--json",
            "research/receipts/x.json",
        ]))
        .unwrap();
        assert!(parsed.hol.is_absolute());
        assert!(parsed.hol.ends_with(".bench/hol-light-stamp"));
        assert!(parsed.json.unwrap().is_absolute());
        assert!(parsed.dropin.is_absolute());
    }

    #[test]
    fn stamp_lines_are_parsed_verbatim() {
        let text = "noise\n\
                    STAMP_BEGIN BIGNUM_ADD_P25519_CORRECT\n\
                    STAMP BIGNUM_ADD_P25519_CORRECT id:1#aaaa=|- (eta : bool)|id:2#bbbb=|- (select : bool)\n\
                    STAMP_INF_BEFORE 100\n\
                    STAMP_INF_AFTER 250\n\
                    STAMP_INFERENCES 150\n\
                    STAMP_REGISTRY 1 aaaa (eta : bool)\n\
                    STAMP_REGISTRY 2 bbbb (select : bool)\n\
                    STAMP_NAME ETA_AX 1\n\
                    STAMP_NAME SELECT_AX 2\n\
                    STAMP_FOOTPRINT_BEGIN statement-level footprint, not the transitive derivation closure\n\
                    STAMP_FOOTPRINT const:= primitive or unregistered\n\
                    val BIGNUM_ADD_P25519_CORRECT : thm = |- T\n";
        let r = parse_stamp(text).expect("stamp lines present");
        assert!(r.begun);
        assert!(r.reported);
        assert_eq!(
            r.begin_theorem.as_deref(),
            Some("BIGNUM_ADD_P25519_CORRECT")
        );
        assert_eq!(
            r.stamp_theorem.as_deref(),
            Some("BIGNUM_ADD_P25519_CORRECT")
        );
        assert_eq!(
            r.axioms,
            vec!["id:1#aaaa=|- (eta : bool)", "id:2#bbbb=|- (select : bool)"]
        );
        assert_eq!(r.inference_before, Some(100));
        assert_eq!(r.inference_after, Some(250));
        assert_eq!(r.inference_delta, Some(150));
        assert_eq!(
            r.registry,
            vec![
                (1, "aaaa".to_string(), "(eta : bool)".to_string()),
                (2, "bbbb".to_string(), "(select : bool)".to_string())
            ]
        );
        assert_eq!(
            r.names,
            vec![
                ("ETA_AX".to_string(), vec![1]),
                ("SELECT_AX".to_string(), vec![2])
            ]
        );
        assert!(r.footprint_label);
        assert_eq!(
            r.footprint,
            vec![(
                "const:=".to_string(),
                "primitive or unregistered".to_string()
            )]
        );
        assert!(r.malformed.is_empty());
        assert!(r.validate(Some("BIGNUM_ADD_P25519_CORRECT")).is_ok());
    }

    #[test]
    fn empty_stamp_is_reported_not_missing() {
        // `REFL` prints `STAMP <name> ` with no ids: still a report, no axioms.
        let r = parse_stamp("STAMP_BEGIN X\nSTAMP X \n").expect("present");
        assert!(r.reported);
        assert!(r.axioms.is_empty());
        assert_eq!(r.stamp_theorem.as_deref(), Some("X"));
    }

    #[test]
    fn unstamped_output_has_no_report() {
        assert_eq!(parse_stamp("val X : thm = |- T\n"), None);
    }

    #[test]
    fn stamp_required_turns_a_missing_stamp_red() {
        let mut r = fixture("arm", "p256", "add", "p256", 0);
        assert!(r.green());
        r.stamp_required = true;
        assert!(!r.green());
        r.stamp_report = Some(valid_stamp("BIGNUM_ADD_P256_CORRECT"));
        assert!(r.green(), "{:?}", r.stamp_error());
    }

    #[test]
    fn a_stamp_from_another_theorem_is_red() {
        let mut r = fixture("arm", "p256", "add", "p256", 0);
        r.stamp_required = true;
        r.stamp_report = Some(valid_stamp("BIGNUM_ADD_P521_CORRECT"));
        assert!(!r.green());
        let err = r.stamp_error().unwrap();
        assert!(err.contains("STAMP_BEGIN names"), "{err}");
    }

    #[test]
    fn a_dangling_axiom_id_is_red() {
        let mut r = fixture("arm", "p256", "add", "p256", 0);
        r.stamp_required = true;
        let mut s = valid_stamp("BIGNUM_ADD_P256_CORRECT");
        s.axioms.push("id:9#ffff=|- OTHER".to_string());
        r.stamp_report = Some(s);
        assert!(!r.green());
        let err = r.stamp_error().unwrap();
        assert!(err.contains("does not resolve"), "{err}");
    }

    #[test]
    fn a_non_unique_registry_is_red() {
        let mut r = fixture("arm", "p256", "add", "p256", 0);
        r.stamp_required = true;
        let mut s = valid_stamp("BIGNUM_ADD_P256_CORRECT");
        s.registry
            .push((1, "cccc".to_string(), "OTHER".to_string()));
        r.stamp_report = Some(s);
        assert!(!r.green());
        let err = r.stamp_error().unwrap();
        assert!(err.contains("not unique"), "{err}");
    }

    #[test]
    fn an_inconsistent_inference_delta_is_red() {
        let mut r = fixture("arm", "p256", "add", "p256", 0);
        r.stamp_required = true;
        let mut s = valid_stamp("BIGNUM_ADD_P256_CORRECT");
        s.inference_delta = Some(151);
        r.stamp_report = Some(s);
        assert!(!r.green());
        let err = r.stamp_error().unwrap();
        assert!(err.contains("inference delta"), "{err}");
    }

    #[test]
    fn a_digest_that_does_not_match_the_registry_is_red() {
        let mut r = fixture("arm", "p256", "add", "p256", 0);
        r.stamp_required = true;
        let mut s = valid_stamp("BIGNUM_ADD_P256_CORRECT");
        s.axioms[0] = "id:1#dead=|- (eta : bool)".to_string();
        r.stamp_report = Some(s);
        assert!(!r.green());
        let err = r.stamp_error().unwrap();
        assert!(err.contains("digest"), "{err}");
    }

    #[test]
    fn a_registry_conflict_across_tasks_is_recorded() {
        let mut summary = StampSummary::default();
        summary.absorb(&valid_stamp("A"));
        let mut other = valid_stamp("B");
        other.registry[0] = (1, "zzzz".to_string(), "ETA".to_string());
        summary.absorb(&other);
        assert!(summary.registry_conflict.is_some());
    }

    #[test]
    fn ids_are_named_through_the_basic_axiom_map() {
        let summary = StampSummary {
            registry: vec![
                (1, "a".into(), "eta".into()),
                (2, "b".into(), "select".into()),
                (3, "c".into(), "inf".into()),
            ],
            names: vec![
                ("ETA_AX".into(), vec![1]),
                ("INFINITY_AX".into(), vec![3]),
                ("SELECT_AX".into(), vec![2]),
            ],
            ..StampSummary::default()
        };
        assert_eq!(
            summary.name_ids(&["1".into(), "3".into()]),
            "{ETA_AX, INFINITY_AX}"
        );
        assert_eq!(summary.name_ids(&[]), "{}");
        assert_eq!(summary.name_ids(&["7".into()]), "{id:7}");
        // Real entries carry their conclusion; the id is still recovered.
        assert_eq!(
            summary.name_ids(&["id:2#b=|- SELECT".into(), "id:9#f=|- OTHER".into()]),
            "{SELECT_AX, id:9#f=|- OTHER}"
        );
        assert_eq!(stamp_entry_id("id:12#dead=|- x"), Some(12));
        assert_eq!(stamp_entry_id("nope"), None);
    }

    // --- measure ---------------------------------------------------------

    #[test]
    fn measure_requires_tree_and_arch() {
        let err = parse_measure_args(&args(&["--arch", "all"])).unwrap_err();
        assert!(err.contains("--tree is required"), "{err}");
        let err = parse_measure_args(&args(&["--tree", "/t"])).unwrap_err();
        assert!(err.contains("--arch is required"), "{err}");
    }

    #[test]
    fn measure_parses_and_anchors_arguments() {
        let m = parse_measure_args(&args(&[
            "--tree",
            ".bench/human-baseline",
            "--arch",
            "all",
            "--force",
            "--json",
            "research/receipts/ix58-checktime.json",
            "--label",
            "human",
        ]))
        .unwrap();
        assert_eq!(m.tree, repo_root().join(".bench/human-baseline"));
        assert_eq!(m.arch, "all");
        assert_eq!(m.arches(), &["arm", "x86"]);
        assert!(m.force);
        assert_eq!(m.timeout, 900);
        assert_eq!(m.label, "human");
        assert_eq!(m.gen, repo_root().join(".bench/measure-logs/human"));
        assert_eq!(
            m.json,
            Some(repo_root().join("research/receipts/ix58-checktime.json"))
        );
    }

    #[test]
    fn measure_rejects_unknown_arch_label_and_flag() {
        let err = parse_measure_args(&args(&["--tree", "/t", "--arch", "riscv"])).unwrap_err();
        assert!(err.contains("expected x86, arm or all"), "{err}");
        for bad in ["", ".", "..", "a/b", "a\\b"] {
            let err = parse_measure_args(&args(&["--tree", "/t", "--arch", "arm", "--label", bad]))
                .unwrap_err();
            assert!(err.contains("--label"), "{bad:?}: {err}");
        }
        let err =
            parse_measure_args(&args(&["--tree", "/t", "--arch", "arm", "--bogus"])).unwrap_err();
        assert!(err.contains("unknown argument"), "{err}");
        let err = parse_measure_args(&args(&["--tree", "/t", "--arch", "arm", "--timeout", "0"]))
            .unwrap_err();
        assert!(err.contains("--timeout"), "{err}");
    }

    /// A wrong `--tree` must produce an error, not a panic and not a make run.
    #[test]
    fn measure_on_nonexistent_tree_fails_cleanly() {
        let missing = std::env::temp_dir().join("tekmerion-ix58-does-not-exist");
        let _ = fs::remove_dir_all(&missing);
        let err = measure(&args(&[
            "--tree",
            missing.to_str().unwrap(),
            "--arch",
            "all",
        ]))
        .unwrap_err();
        assert!(err.contains("not a directory"), "{err}");
        assert_eq!(run(&args(&["measure", "--tree", "", "--arch", "all"])), 2);
    }

    #[test]
    fn measure_on_tree_without_makefile_fails_cleanly() {
        let empty = std::env::temp_dir().join(format!("tek-ix58-empty-{}", std::process::id()));
        fs::create_dir_all(empty.join("arm")).unwrap();
        let err =
            measure(&args(&["--tree", empty.to_str().unwrap(), "--arch", "arm"])).unwrap_err();
        assert!(err.contains("no arm/Makefile"), "{err}");
        let _ = fs::remove_dir_all(&empty);
    }

    #[test]
    fn measure_json_records_label_tree_and_shares_the_task_shape() {
        let m = parse_measure_args(&args(&[
            "--tree", "/t", "--arch", "arm", "--label", "human",
        ]))
        .unwrap();
        let results = vec![fixture("arm", "p521", "add", "p521", 0)];
        let text = measure_json(&m, 1795501903, &results);
        let j = json::parse(text.as_bytes()).expect("measure JSON must parse");
        assert_eq!(j.get("label").unwrap().as_str(), Some("human"));
        assert_eq!(j.get("tree").unwrap().as_str(), Some("/t"));
        assert_eq!(j.get("cores").unwrap(), &json::Json::Num(1.0));
        let tasks = j.get("tasks").unwrap().as_array().unwrap();
        assert_eq!(tasks[0].get("family").unwrap().as_str(), Some("p521"));
        assert_eq!(tasks[0].get("seconds").unwrap(), &json::Json::Num(46.0));
        assert_eq!(j.get("ok").unwrap(), &json::Json::Num(1.0));
    }

    // --- typed comparability (XIV.A) -------------------------------------

    fn scope_of(kind: TreeKind) -> MeasurementScope {
        MeasurementScope {
            tree_kind: kind,
            s2n_pin: "5e0fc782".to_string(),
            jobs: 1,
            force: true,
            kernel_build: "hol-light-stamp2".to_string(),
        }
    }

    fn timed(arch: &str, dir: &str, op: &str, fam: &str, secs: Option<f64>) -> TimedTask {
        TimedTask {
            arch: arch.to_string(),
            dir: dir.to_string(),
            op: op.to_string(),
            fam: fam.to_string(),
            theorem: Some(format!(
                "BIGNUM_{}_{}_CORRECT",
                op.to_uppercase(),
                fam.to_uppercase()
            )),
            exit: Some(0),
            check_axioms: true,
            seconds: secs,
            inferences: secs.map(|s| (s * 1_000_000.0) as u64),
            inferences_whole_file: None,
            log: format!("logs-{arch}/{dir}_{op}_{fam}.log"),
        }
    }

    fn tfile(kind: TreeKind, tasks: Vec<TimedTask>) -> TimedFile {
        TimedFile {
            tree: Some("/h".to_string()),
            scope: scope_of(kind),
            tasks,
        }
    }

    fn proof(theorem: &str, kind: TreeKind) -> MeasuredProof {
        MeasuredProof {
            theorem: theorem.to_string(),
            exit: Some(0),
            check_axioms: true,
            theorem_proved: true,
            seconds: Some(10.0),
            inferences: Some(1_000),
            scope: scope_of(kind),
        }
    }

    fn matched() -> (MeasuredProof, MeasuredProof) {
        (
            proof("BIGNUM_ADD_P521_CORRECT", TreeKind::Human),
            proof("BIGNUM_ADD_P521_CORRECT", TreeKind::Generated),
        )
    }

    #[test]
    fn compare_accepts_the_positive_case() {
        let (h, g) = matched();
        let m = compare(&h, &g).expect("identical theorem, success, same scope");
        assert_eq!(m.theorem(), "BIGNUM_ADD_P521_CORRECT");
        assert_eq!(m.inference_ratio(), Some(1.0));
        assert!(comparison_reasons(&h, &g).is_empty());
    }

    #[test]
    fn compare_refuses_a_different_theorem_identity() {
        let (h, mut g) = matched();
        g.theorem = "BIGNUM_ADD_P521_SUBROUTINE_CORRECT".to_string();
        assert_eq!(
            compare(&h, &g),
            Err(ComparisonError::TheoremIdentityMismatch {
                human: "BIGNUM_ADD_P521_CORRECT".to_string(),
                generated: "BIGNUM_ADD_P521_SUBROUTINE_CORRECT".to_string(),
            })
        );
    }

    #[test]
    fn compare_refuses_each_failed_judgment() {
        let (h, g) = matched();
        let mut bad = h.clone();
        bad.exit = Some(2);
        assert_eq!(
            compare(&bad, &g),
            Err(ComparisonError::HumanExitNotZero { exit: Some(2) })
        );

        let (h, g) = matched();
        let mut bad = g.clone();
        bad.exit = Some(124);
        assert_eq!(
            compare(&h, &bad),
            Err(ComparisonError::GeneratedExitNotZero { exit: Some(124) })
        );

        let (h, g) = matched();
        let mut bad = h.clone();
        bad.check_axioms = false;
        assert_eq!(
            compare(&bad, &g),
            Err(ComparisonError::HumanAxiomsNotChecked)
        );

        let (h, g) = matched();
        let mut bad = g.clone();
        bad.check_axioms = false;
        assert_eq!(
            compare(&h, &bad),
            Err(ComparisonError::GeneratedAxiomsNotChecked)
        );

        let (h, g) = matched();
        let mut bad = h.clone();
        bad.theorem_proved = false;
        assert_eq!(
            compare(&bad, &g),
            Err(ComparisonError::HumanTheoremNotProved)
        );

        let (h, g) = matched();
        let mut bad = g.clone();
        bad.theorem_proved = false;
        assert_eq!(
            compare(&h, &bad),
            Err(ComparisonError::GeneratedTheoremNotProved)
        );
    }

    #[test]
    fn compare_refuses_a_wrong_tree_kind() {
        let (h, g) = matched();
        let mut bad = h.clone();
        bad.scope.tree_kind = TreeKind::Generated;
        assert_eq!(
            compare(&bad, &g),
            Err(ComparisonError::WrongTreeKind {
                expected: TreeKind::Human,
                found: TreeKind::Generated,
            })
        );

        let (h, g) = matched();
        let mut bad = g.clone();
        bad.scope.tree_kind = TreeKind::Human;
        assert_eq!(
            compare(&h, &bad),
            Err(ComparisonError::WrongTreeKind {
                expected: TreeKind::Generated,
                found: TreeKind::Human,
            })
        );
    }

    #[test]
    fn compare_refuses_each_scope_difference() {
        let (h, g) = matched();
        let mut bad = g.clone();
        bad.scope.s2n_pin = "deadbeef".to_string();
        assert_eq!(
            compare(&h, &bad),
            Err(ComparisonError::ScopeMismatch {
                field: "s2n_bignum_pin",
                human: "5e0fc782".into(),
                generated: "deadbeef".into(),
            })
        );

        let (h, g) = matched();
        let mut bad = g.clone();
        bad.scope.jobs = 2;
        assert_eq!(
            compare(&h, &bad),
            Err(ComparisonError::ScopeMismatch {
                field: "jobs",
                human: "1".into(),
                generated: "2".into(),
            })
        );

        let (h, g) = matched();
        let mut bad = g.clone();
        bad.scope.force = false;
        assert_eq!(
            compare(&h, &bad),
            Err(ComparisonError::ScopeMismatch {
                field: "force",
                human: "true".into(),
                generated: "false".into(),
            })
        );

        let (h, g) = matched();
        let mut bad = g.clone();
        bad.scope.kernel_build = "hol-light".to_string();
        assert_eq!(
            compare(&h, &bad),
            Err(ComparisonError::ScopeMismatch {
                field: "kernel_build",
                human: "hol-light-stamp2".into(),
                generated: "hol-light".into(),
            })
        );
    }

    #[test]
    fn median_handles_odd_even_and_empty() {
        assert_eq!(median(vec![]), None);
        assert_eq!(median(vec![3.0]), Some(3.0));
        assert_eq!(median(vec![10.0, 40.0]), Some(25.0));
        assert_eq!(median(vec![5.0, 1.0, 3.0]), Some(3.0));
    }

    #[test]
    fn checktime_table_excludes_a_refused_row_from_every_aggregate() {
        let human = tfile(
            TreeKind::Human,
            vec![
                timed("arm", "p521", "add", "p521", Some(3.0)),
                timed("arm", "p521", "sub", "p521", Some(1.0)),
            ],
        );
        let mut gt = vec![
            timed("arm", "p521", "add", "p521", Some(12.0)),
            timed("arm", "p521", "sub", "p521", Some(5.0)),
        ];
        gt[1].theorem = Some("BIGNUM_SUB_P521_SUBROUTINE_CORRECT".to_string());
        let generated = tfile(TreeKind::Generated, gt);
        let text = render_checktime(&human, &generated);
        assert!(
            text.contains(
                "| arm | p521/add_p521 | BIGNUM_ADD_P521_CORRECT | BIGNUM_ADD_P521_CORRECT | 3.000 | 12.000 | matched |"
            ),
            "{text}"
        );
        assert!(
            text.contains(
                "| arm | p521/sub_p521 | BIGNUM_SUB_P521_CORRECT | BIGNUM_SUB_P521_SUBROUTINE_CORRECT | 1.000 | 5.000 | refused: theorem identity |"
            ),
            "{text}"
        );
        assert!(text.contains("| matched pairs (n) | 1 |"), "{text}");
        assert!(text.contains("| human total s (n=1) | 3.000 |"), "{text}");
        assert!(
            text.contains("| generated total s (n=1) | 12.000 |"),
            "{text}"
        );
        assert!(
            text.contains("| aggregate seconds ratio (n=1) | 4.00 |"),
            "{text}"
        );
        assert!(text.contains("| unmatched rows | 1 |"), "{text}");
        assert!(text.contains("theorem identity differs"), "{text}");
        assert!(
            text.contains("MATCHED 1/2 PAIRS; UNMATCHED 1; HUMAN ROWS 2; GENERATED ROWS 2"),
            "{text}"
        );
        // The refused row's 1 s / 5 s must not appear in a total.
        assert!(text.contains("| human total s (n=1) | 3.000 |"), "{text}");
    }

    #[test]
    fn a_missing_row_cannot_yield_a_pair_or_a_ratio() {
        let human = tfile(
            TreeKind::Human,
            vec![
                timed("x86", "sm2", "add", "sm2", Some(2.0)),
                timed("x86", "sm2", "neg", "sm2", Some(3.0)),
            ],
        );
        let generated = tfile(
            TreeKind::Generated,
            vec![timed("x86", "sm2", "add", "sm2", Some(4.0))],
        );
        let text = render_checktime(&human, &generated);
        assert!(text.contains("| human rows parsed | 2 |"), "{text}");
        assert!(text.contains("| generated rows parsed | 1 |"), "{text}");
        assert!(text.contains("| task rows (union of keys) | 2 |"), "{text}");
        assert!(text.contains("| matched pairs (n) | 1 |"), "{text}");
        assert!(text.contains("| unmatched rows | 1 |"), "{text}");
        assert!(text.contains("no generated row"), "{text}");
        assert!(text.contains("| human total s (n=1) | 2.000 |"), "{text}");
        assert!(text.contains("MATCHED 1/2 PAIRS"), "{text}");
        assert!(!text.contains("human total s (n=2)"), "{text}");
    }

    #[test]
    fn checktime_budget_only_uses_matched_pairs() {
        let human = tfile(
            TreeKind::Human,
            vec![timed("x86", "sm2", "add", "sm2", Some(2.0))],
        );
        let generated = tfile(
            TreeKind::Generated,
            vec![timed("x86", "sm2", "add", "sm2", Some(100.0))],
        );
        let text = render_checktime(&human, &generated);
        assert!(
            text.contains("| x86 | sm2/add_sm2 | 100.000 | 83.3 | True | False |"),
            "{text}"
        );
        assert!(text.contains("MATCHED 1/1 PAIRS"), "{text}");
        assert!(
            text.contains("`/h/x86/sm2/bignum_add_sm2.correct`"),
            "{text}"
        );
    }

    #[test]
    fn zero_matched_pairs_publishes_no_ratio() {
        let human = tfile(
            TreeKind::Human,
            vec![timed("arm", "p256", "add", "p256", Some(3.0))],
        );
        let mut gt = timed("arm", "p256", "add", "p256", Some(9.0));
        gt.theorem = Some("BIGNUM_ADD_P256_SUBROUTINE_CORRECT".to_string());
        let generated = tfile(TreeKind::Generated, vec![gt]);
        let text = render_matched(&human, &generated);
        assert!(text.contains("| matched pairs (n) | 0 |"), "{text}");
        assert!(
            text.contains("aggregate inference ratio | n/a (n=0; not published)"),
            "{text}"
        );
        assert!(text.contains("MATCHED 0/1 PAIRS"), "{text}");

        let j = matched_json(&human, &generated, "h.json", "g.json");
        let v = json::parse(j.as_bytes()).expect("matched JSON must parse");
        assert_eq!(
            v.get("counts").unwrap().get("matched_pairs").unwrap(),
            &json::Json::Num(0.0)
        );
        assert_eq!(
            v.get("matched_totals").unwrap().get("inference_ratio"),
            Some(&json::Json::Null)
        );
        let task = &v.get("tasks").unwrap().as_array().unwrap()[0];
        assert_eq!(
            task.get("human_theorem").unwrap().as_str(),
            Some("BIGNUM_ADD_P256_CORRECT")
        );
        assert_eq!(
            task.get("generated_theorem").unwrap().as_str(),
            Some("BIGNUM_ADD_P256_SUBROUTINE_CORRECT")
        );
        assert_eq!(task.get("matched").unwrap(), &json::Json::Bool(false));
    }

    // --- obligation (XIII.E3) --------------------------------------------

    #[test]
    fn extract_conclusion_normalises_and_stops_at_the_next_marker() {
        // HOL's printer indents every continuation line of a term.
        let text = "noise\nval BIGNUM_ADD_P256_CORRECT : thm = |- forall z x.\n      a /\\ b\n      ==> c\nval OTHER : thm = |- T\nRunning time: 1.0 sec\n";
        let c = extract_conclusion(text, "BIGNUM_ADD_P256_CORRECT").unwrap();
        assert_eq!(c, "forall z x. a /\\ b ==> c");
        assert_eq!(extract_conclusion(text, "MISSING"), None);
        assert_eq!(
            extract_conclusion(text, "BIGNUM_ADD_P256_SUBROUTINE_CORRECT"),
            None
        );
    }

    #[test]
    fn first_difference_is_the_first_character_mismatch() {
        let d = first_difference("abcdef", "abXdef").expect("different");
        assert_eq!(d.offset, 2);
        assert_eq!(first_difference("same", "same"), None);
    }

    #[test]
    fn obligation_verdict_is_incomparable_when_the_human_adds_a_post_and_a_broader_frame() {
        let h = "forall z x pc returnaddress. \
                 read X30 s = returnaddress /\\ ensures arm (read PC s = returnaddress) \
                 (MAYCHANGE_REGS_AND_FLAGS_PERMITTED_BY_ABI ,, MAYCHANGE [memory])";
        let g = "forall z x pc. ensures arm (read PC s = word (pc + 88)) \
                 (MAYCHANGE [PC; X3] ,, MAYCHANGE [memory])";
        let m = markers_of(h, g);
        assert!(m.human_x30_pre);
        assert!(m.human_pc_returnaddress);
        assert!(m.generated_pc_offset);
        assert!(m.human_frame_abi && !m.generated_frame_abi);
        assert_eq!(
            obligation_verdict(h, g, &m),
            ObligationVerdict::Incomparable
        );
        assert_eq!(
            obligation_verdict(h, h, &markers_of(h, h)),
            ObligationVerdict::SameStatement
        );
        let d = describe_obligation(&m);
        assert!(d.contains("returnaddress"), "{d}");
        assert!(
            d.contains("MAYCHANGE_REGS_AND_FLAGS_PERMITTED_BY_ABI"),
            "{d}"
        );
    }

    #[test]
    fn obligation_json_records_the_verdict_and_counts() {
        let different = Obligation {
            arch: "arm".into(),
            dir: "p256".into(),
            op: "add".into(),
            fam: "p256".into(),
            human_theorem: "BIGNUM_ADD_P256_SUBROUTINE_CORRECT".into(),
            generated_theorem: "BIGNUM_ADD_P256_CORRECT".into(),
            human_conclusion: Some("h".into()),
            generated_conclusion: Some("g".into()),
            same: false,
            first_difference: Some(FirstDifference {
                offset: 0,
                human: "h".into(),
                generated: "g".into(),
            }),
            markers: Markers::default(),
            verdict: ObligationVerdict::Incomparable,
            description: "different".into(),
            error: None,
        };
        let mut same = different.clone();
        same.same = true;
        same.verdict = ObligationVerdict::SameStatement;
        let body = obligation_json(&[different, same], "/h", "/g");
        let j = json::parse(body.as_bytes()).unwrap();
        assert_eq!(
            j.get("counts").unwrap().get("same").unwrap(),
            &json::Json::Num(1.0)
        );
        assert_eq!(
            j.get("counts").unwrap().get("different").unwrap(),
            &json::Json::Num(1.0)
        );
        let tasks = j.get("tasks").unwrap().as_array().unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(
            tasks[0].get("verdict").unwrap().as_str(),
            Some("different: neither strictly stronger nor strictly weaker (incomparable)")
        );
    }

    #[test]
    fn canonical_term_erases_notation_and_spacing() {
        // The built conclusion spells `!` as `forall`, writes numerals in
        // decimal, and differs only in whitespace from the quotation.
        let built = "forall z x y m n pc.\n  nonoverlapping (word pc,80) (z,8 * 4)\n  ==> read PC s = word (pc + 76)";
        let quoted =
            "!z x y m n pc. nonoverlapping (word pc,0x50) (z,8*4)\n ==> read PC s = word (pc+0x4c)";
        assert_eq!(canonical_term(built), canonical_term(quoted));
    }

    #[test]
    fn canonical_term_keeps_genuine_differences() {
        assert_ne!(
            canonical_term("read PC s = word (pc + 0x4c)"),
            canonical_term("read PC s = word (pc + 0x4d)")
        );
        assert_ne!(
            canonical_term("MAYCHANGE [events] ,, MAYCHANGE [memory :> bignum(z,4)]"),
            canonical_term(
                "MAYCHANGE_REGS_AND_FLAGS_PERMITTED_BY_ABI ,, MAYCHANGE [memory :> bignum(z,4)]"
            )
        );
    }

    const UPSTREAM_SAMPLE: &str = "\
let BIGNUM_ADD_P25519_CORRECT = time prove\n \
 (`!z x y m n pc. nonoverlapping (word pc,0x50) (z,8 * 4) ==> P`,\n \
  BODY_TAC);;\n\
\
let BIGNUM_ADD_P25519_SUBROUTINE_CORRECT = time prove\n \
 (`!z x y m n pc returnaddress. nonoverlapping (word pc,0x50) (z,8 * 4) ==> Q`,\n \
  ARM_ADD_RETURN_NOSTACK_TAC BIGNUM_ADD_P25519_EXEC BIGNUM_ADD_P25519_CORRECT);;\n";

    #[test]
    fn extract_quotation_reads_the_statement() {
        let q = extract_quotation(UPSTREAM_SAMPLE, "BIGNUM_ADD_P25519_SUBROUTINE_CORRECT").unwrap();
        assert!(q.starts_with("!z x y m n pc returnaddress."));
        assert!(q.ends_with("==> Q"));
    }

    #[test]
    fn extract_quotation_does_not_confuse_noibt_name() {
        let src =
            "let BIGNUM_ADD_P25519_NOIBT_SUBROUTINE_CORRECT = time prove\n (`NOIBT`,\n  T);;\n\
let BIGNUM_ADD_P25519_SUBROUTINE_CORRECT = time prove\n (`IBT`,\n  T);;\n";
        assert_eq!(
            extract_quotation(src, "BIGNUM_ADD_P25519_SUBROUTINE_CORRECT").unwrap(),
            "IBT"
        );
    }

    #[test]
    fn extract_theorem_block_is_verbatim() {
        let block =
            extract_theorem_block(UPSTREAM_SAMPLE, "BIGNUM_ADD_P25519_SUBROUTINE_CORRECT").unwrap();
        assert!(block.starts_with("let BIGNUM_ADD_P25519_SUBROUTINE_CORRECT = time prove"));
        assert!(block.contains(
            "ARM_ADD_RETURN_NOSTACK_TAC BIGNUM_ADD_P25519_EXEC BIGNUM_ADD_P25519_CORRECT);;"
        ));
        assert!(!block.contains("BIGNUM_ADD_P25519_CORRECT = time prove"));
    }

    #[test]
    fn upstream_blocks_emit_noibt_then_ibt_for_x86() {
        let dir = std::env::temp_dir().join(format!("tek-ix64-{}", std::process::id()));
        let proofs = dir.join("x86/proofs");
        fs::create_dir_all(&proofs).unwrap();
        let src = "let BIGNUM_ADD_SM2_NOIBT_SUBROUTINE_CORRECT = time prove\n (`NOIBT`,\n  X86_PROMOTE_RETURN_NOSTACK_TAC bignum_add_sm2_tmc BIGNUM_ADD_SM2_CORRECT);;\n\n\
let BIGNUM_ADD_SM2_SUBROUTINE_CORRECT = time prove\n (`IBT`,\n  MATCH_ACCEPT_TAC(ADD_IBT_RULE BIGNUM_ADD_SM2_NOIBT_SUBROUTINE_CORRECT));;\n";
        fs::write(proofs.join("bignum_add_sm2.ml"), src).unwrap();
        let out = upstream_subroutine_blocks(&dir, "x86", "add", "sm2").unwrap();
        let noibt = out.find("NOIBT_SUBROUTINE_CORRECT = time prove").unwrap();
        let ibt = out
            .find("let BIGNUM_ADD_SM2_SUBROUTINE_CORRECT = time prove")
            .unwrap();
        assert!(noibt < ibt);
        assert!(out.contains("ADD_IBT_RULE"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn upstream_blocks_reject_an_unknown_derivation() {
        let dir = std::env::temp_dir().join(format!("tek-ix64-bad-{}", std::process::id()));
        let proofs = dir.join("arm/proofs");
        fs::create_dir_all(&proofs).unwrap();
        let src = "let BIGNUM_NEG_P256_SUBROUTINE_CORRECT = time prove\n (`Q`,\n  ALL_TAC);;\n";
        fs::write(proofs.join("bignum_neg_p256.ml"), src).unwrap();
        let err = upstream_subroutine_blocks(&dir, "arm", "neg", "p256").unwrap_err();
        assert!(err.contains("recognised upstream return tactic"), "{err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn sweep_parses_upstream_override() {
        let parsed = parse_sweep_args(&args(&["--arch", "arm", "--upstream", "/pin"])).unwrap();
        assert_eq!(parsed.upstream, PathBuf::from("/pin"));
    }

    #[test]
    fn obligation_compares_the_subroutine_name_when_asked() {
        let dir = std::env::temp_dir().join(format!("tek-ix64-ob-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let human = dir.join("human.correct");
        let generated = dir.join("generated.correct");
        let concl = "forall z x n pc. read PC s = returnaddress";
        fs::write(
            &human,
            format!("val BIGNUM_NEG_P256_SUBROUTINE_CORRECT : thm = |- {concl}\n"),
        )
        .unwrap();
        fs::write(
            &generated,
            format!("val BIGNUM_NEG_P256_SUBROUTINE_CORRECT : thm = |- {concl}\n"),
        )
        .unwrap();
        let o = build_obligation("arm", "p256", "neg", "p256", &human, &generated, true);
        assert_eq!(o.generated_theorem, "BIGNUM_NEG_P256_SUBROUTINE_CORRECT");
        assert_eq!(o.verdict, ObligationVerdict::SameStatement);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn conclusion_extraction_stops_at_column_zero_output() {
        let text = "val BIGNUM_NEG_P256_CORRECT : thm = |- forall z.\n       P z\nWarning: inventing type variables\nStepping to state s1\n";
        assert_eq!(
            extract_conclusion(text, "BIGNUM_NEG_P256_CORRECT").unwrap(),
            "forall z. P z"
        );
    }
}
