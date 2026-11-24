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

fn assemble_core(
    task: &str,
    answer_path: &Path,
    dropin: &Path,
    problems: &Path,
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
                 Printf.printf \"STAMP %s\\n%!\" (stamp_string {theorem});;\n\
                 Printf.printf \"STAMP_INF_BEFORE %d\\n%!\" {before};;\n\
                 Printf.printf \"STAMP_INF_AFTER %d\\n%!\" (inference_count ());;\n\
                 Printf.printf \"STAMP_INFERENCES %d\\n%!\" (inference_count () - {before});;\n\
                 List.iter\n\
                   (fun (id,txt) -> Printf.printf \"STAMP_REGISTRY %d %s\\n%!\" id txt)\n\
                   (axiom_registry ());;\n\
                 List.iter\n\
                   (fun (nm,th) -> Printf.printf \"STAMP_NAME %s %s\\n%!\" nm\n\
                      (String.concat \",\" (List.map string_of_int (stamp_of th))))\n\
                   [\"ETA_AX\",ETA_AX; \"SELECT_AX\",SELECT_AX; \"INFINITY_AX\",INFINITY_AX];;\n"
            ),
        )
    } else {
        (String::new(), String::new())
    };

    let body = if arch == "arm" {
        format!(
            "(* DROP-IN experiment: generated Tekmerion ARM proof in upstream shape. *)\n\
             needs \"arm/proofs/base.ml\";;\n\n\
             let {base}_mc = define_assert_from_elf \"{base}_mc\" \"arm/{dir}/{base}.o\"\n[\n{mc}];;\n\n\
             let {exec_name} = ARM_MK_EXEC_RULE {base}_mc;;\n\n\
             let {mod_name} = new_definition `{mod_name} = {modulus}`;;\n\n\
             {stamp_pre}let {theorem} = time prove\n ({query},\n{answer});;\n\n\
             {stamp_post}Printf.printf \"val {theorem} : thm = %s\\n%!\" (string_of_thm {theorem});;\n",
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
             {stamp_post}Printf.printf \"val {theorem} : thm = %s\\n%!\" (string_of_thm {theorem});;\n",
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
    let stamp = args.iter().any(|a| a == "--stamp");

    let assembled = assemble_core(task, Path::new(answer_path), &dropin, &problems, stamp)?;
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
    /// Wall time of the `time prove` call, parsed from `Running time:`.
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
        self.exit == Some(0)
            && self.check_axioms
            && self.theorem.is_some()
            && (!self.stamp_required || self.stamp_report.as_ref().is_some_and(|r| r.reported))
    }

    fn label(&self) -> String {
        format!("{}/{}_{}", self.dir, self.op, self.fam)
    }

    fn axioms(&self) -> Option<&Vec<String>> {
        self.stamp_report.as_ref().map(|r| &r.axioms)
    }

    fn inference_delta(&self) -> Option<i64> {
        self.stamp_report.as_ref().and_then(|r| r.inference_delta)
    }
}

/// What one stamped proof run printed. Everything here is taken verbatim from
/// the program's own output; nothing is inferred.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StampReport {
    pub begun: bool,
    /// `STAMP` was present (possibly empty).
    pub reported: bool,
    pub axioms: Vec<String>,
    pub inference_before: Option<i64>,
    pub inference_after: Option<i64>,
    pub inference_delta: Option<i64>,
    /// `(id, conclusion)` from `STAMP_REGISTRY`.
    pub registry: Vec<(i64, String)>,
    /// `(name, ids)` from `STAMP_NAME` for the three basic axioms.
    pub names: Vec<(String, Vec<i64>)>,
}

fn parse_i64(s: &str) -> Option<i64> {
    s.trim().parse::<i64>().ok()
}

/// Split a `STAMP` payload into entries. Entries are `id:<n>=|- <term>` joined
/// by `|`, and each entry itself contains `|-`, so a plain split on `|` is not
/// enough: start a new entry only at a part that begins with `id:`.
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
            seen = true;
        } else if let Some(rest) = line.strip_prefix("STAMP ") {
            r.reported = true;
            r.axioms = split_stamp_entries(rest);
            seen = true;
        } else if let Some(rest) = line.strip_prefix("STAMP_INF_BEFORE ") {
            r.inference_before = parse_i64(rest);
        } else if let Some(rest) = line.strip_prefix("STAMP_INF_AFTER ") {
            r.inference_after = parse_i64(rest);
        } else if let Some(rest) = line.strip_prefix("STAMP_INFERENCES ") {
            r.inference_delta = parse_i64(rest);
        } else if let Some(rest) = line.strip_prefix("STAMP_REGISTRY ") {
            if let Some((id, txt)) = rest.split_once(' ') {
                if let Some(id) = parse_i64(id) {
                    r.registry.push((id, txt.trim().to_string()));
                }
            }
        } else if let Some(rest) = line.strip_prefix("STAMP_NAME ") {
            let mut parts = rest.split_whitespace();
            if let (Some(name), Some(ids)) = (parts.next(), parts.next()) {
                let ids: Vec<i64> = ids
                    .split(',')
                    .filter_map(|s| s.trim().parse::<i64>().ok())
                    .collect();
                r.names.push((name.to_string(), ids));
            }
        }
    }
    if seen {
        Some(r)
    } else {
        None
    }
}

/// Registry and basic-axiom naming, unioned across every task that reported it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct StampSummary {
    registry: Vec<(i64, String)>,
    names: Vec<(String, Vec<i64>)>,
}

impl StampSummary {
    fn absorb(&mut self, report: &StampReport) {
        for (id, txt) in &report.registry {
            if !self.registry.iter().any(|(i, _)| i == id) {
                self.registry.push((*id, txt.clone()));
            }
        }
        for (name, ids) in &report.names {
            if !self.names.iter().any(|(n, _)| n == name) {
                self.names.push((name.clone(), ids.clone()));
            }
        }
        self.registry.sort_by_key(|(id, _)| *id);
        self.names.sort_by(|a, b| a.0.cmp(&b.0));
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
fn theorem_marker(text: &str, op: &str, fam: &str) -> Option<String> {
    let name = format!(
        "BIGNUM_{}_{}_CORRECT",
        op.to_uppercase(),
        fam.to_uppercase()
    );
    let marker = format!("val {name} : thm");
    text.lines()
        .any(|l| l.trim_start().starts_with(&marker))
        .then_some(name)
}

/// `logs-<arch>`: `logs-arm` keeps the existing ARM logs in place.
fn log_dir(gen: &Path, arch: &str) -> PathBuf {
    gen.join(format!("logs-{arch}"))
}

fn build_logged(s: &SweepArgs, p: &Planned, logs: &Path) -> Result<TaskResult, String> {
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
    let stamp_required = s.stamp;

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
    for (i, (id, txt)) in summary.registry.iter().enumerate() {
        let comma = if i + 1 == summary.registry.len() {
            ""
        } else {
            ","
        };
        out.push_str(&format!(
            "{}{{\"id\": {id}, \"conclusion\": {}}}{comma}",
            if i == 0 { "" } else { " " },
            json_str(txt)
        ));
    }
    out.push_str("],\n");
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
        out.push_str(&format!(
            "    {{\"arch\": {}, \"dir\": {}, \"op\": {}, \"family\": {}, \"exit\": {}, \"theorem\": {}, \"check_axioms\": {}, \"seconds\": {}, \"timeout\": {}, \"stamp_required\": {}, \"axioms\": {}, \"inference_delta\": {}, \"log\": {}}}{comma}\n",
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
            json_opt_str_array(r.axioms()),
            json_opt_i64(r.inference_delta()),
            json_str(&r.log),
        ));
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
        println!("\nAXIOM REGISTRY (raw conclusions)");
        for (id, txt) in &summary.registry {
            let name = summary
                .names
                .iter()
                .find(|(_, ids)| ids.contains(id))
                .map(|(n, _)| n.as_str())
                .unwrap_or("(not a basic axiom)");
            println!("  id:{id} = {name}: {txt}");
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
        let assembled =
            match assemble_core(&p.task, &answer, &parsed.dropin, &problems, parsed.stamp) {
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
        let result = build_logged(&parsed, p, &logs)?;
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
    let generated_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
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
        "stamp-table" => stamp_table(rest),
        _ => Err(
            "usage: dropin <assemble|build|sweep|stamp-table> [options]\n\
             \n\
             sweep: dropin sweep --arch <x86|arm|all> [--answers <dir>] [--dir <dropin-dir>]
             \x20      [--gen <gen-dir>] [--hol <hol-dir>] [--force] [--timeout <secs>]
             \x20      [--json <path>] [--dry-run] [--stamp]
             \x20      \n\
             stamp-table: dropin stamp-table --json <sweep-json>"
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
                summary.registry.push((id, txt.to_string()));
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
    summary.registry.sort_by_key(|(id, _)| *id);
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
        println!("| {arch} | {task} | {axi} | {inf} | {secs} |");
    }

    println!("\n## Axiom registry (raw conclusions)\n");
    println!("| id | name | conclusion |");
    println!("|---|---|---|");
    for (id, txt) in &summary.registry {
        let name = summary
            .names
            .iter()
            .find(|(_, ids)| ids.contains(id))
            .map(|(n, _)| n.as_str())
            .unwrap_or("*(not a basic axiom)*");
        println!("| {id} | {name} | `{txt}` |");
    }

    println!(
        "\nstamped={stamped} missing_stamp={missing} outside_basic_axioms={outside} total={}",
        tasks.len()
    );
    Ok(if missing == 0 && outside == 0 { 0 } else { 1 })
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
    fn json_writer_emits_a_parseable_stamp_record() {
        let summary = StampSummary {
            registry: vec![(1, "eta-concl".into()), (2, "select-concl".into())],
            names: vec![("ETA_AX".into(), vec![1]), ("SELECT_AX".into(), vec![2])],
        };
        let mut r = fixture("arm", "curve25519", "add", "p25519", 0);
        r.stamp_required = true;
        r.stamp_report = Some(StampReport {
            begun: true,
            reported: true,
            axioms: vec!["id:1=|- eta".into(), "id:2=|- select".into()],
            inference_before: Some(10),
            inference_after: Some(42),
            inference_delta: Some(32),
            registry: summary.registry.clone(),
            names: summary.names.clone(),
        });
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
        let reg = j.get("axiom_registry").unwrap().as_array().unwrap();
        assert_eq!(reg.len(), 2);
        assert_eq!(reg[0].get("id").unwrap(), &json::Json::Num(1.0));
        assert_eq!(
            reg[0].get("conclusion").unwrap().as_str(),
            Some("eta-concl")
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
                    STAMP id:1=|- ETA|id:2=|- SELECT\n\
                    STAMP_INF_BEFORE 100\n\
                    STAMP_INF_AFTER 250\n\
                    STAMP_INFERENCES 150\n\
                    STAMP_REGISTRY 1 (eta)\n\
                    STAMP_REGISTRY 3 (infinity)\n\
                    STAMP_NAME ETA_AX 1\n\
                    STAMP_NAME INFINITY_AX 3\n\
                    val BIGNUM_ADD_P25519_CORRECT : thm = |- T\n";
        let r = parse_stamp(text).expect("stamp lines present");
        assert!(r.begun);
        assert!(r.reported);
        assert_eq!(r.axioms, vec!["id:1=|- ETA", "id:2=|- SELECT"]);
        assert_eq!(r.inference_before, Some(100));
        assert_eq!(r.inference_after, Some(250));
        assert_eq!(r.inference_delta, Some(150));
        assert_eq!(
            r.registry,
            vec![(1, "(eta)".to_string()), (3, "(infinity)".to_string())]
        );
        assert_eq!(
            r.names,
            vec![
                ("ETA_AX".to_string(), vec![1]),
                ("INFINITY_AX".to_string(), vec![3])
            ]
        );
    }

    #[test]
    fn empty_stamp_is_reported_not_missing() {
        // `REFL` prints `STAMP ` with no ids: still a report, with no axioms.
        let r = parse_stamp("STAMP_BEGIN X\nSTAMP \n").expect("present");
        assert!(r.reported);
        assert!(r.axioms.is_empty());
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
        r.stamp_report = Some(StampReport {
            begun: true,
            reported: true,
            ..StampReport::default()
        });
        assert!(r.green());
    }

    #[test]
    fn ids_are_named_through_the_basic_axiom_map() {
        let summary = StampSummary {
            registry: vec![(1, "eta".into()), (2, "select".into()), (3, "inf".into())],
            names: vec![
                ("ETA_AX".into(), vec![1]),
                ("INFINITY_AX".into(), vec![3]),
                ("SELECT_AX".into(), vec![2]),
            ],
        };
        assert_eq!(
            summary.name_ids(&["1".into(), "3".into()]),
            "{ETA_AX, INFINITY_AX}"
        );
        assert_eq!(summary.name_ids(&[]), "{}");
        assert_eq!(summary.name_ids(&["7".into()]), "{id:7}");
        // Real entries carry their conclusion; the id is still recovered.
        assert_eq!(
            summary.name_ids(&["id:2=|- SELECT".into(), "id:9=|- OTHER".into()]),
            "{SELECT_AX, id:9=|- OTHER}"
        );
        assert_eq!(stamp_entry_id("id:12=|- x"), Some(12));
        assert_eq!(stamp_entry_id("nope"), None);
    }
}
