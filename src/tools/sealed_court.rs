//! Build the pruned, proof-free input tree for the O10.2 structural run.
//!
//! Port of `research/receipts/ix54/build-sealed-court-v3.py`. The builder reads
//! only `problems.json` metadata (`inlined_locations`) and copies the prefix of
//! each template that `combine-answer-and-setup.py` consumes:
//!
//! * `check_parsing_error`: `lines[:linenum]`
//! * `write_template_ml`:    `template_lines[prevline:linenum]`
//!
//! Nothing at or after `linenum` is copied, so no answer/proof text enters the
//! tree. Nine task objects, the bench scripts, the frozen `tools/`, `.git/` and
//! the sealed manifest are copied; HOL Light is symlinked read-only by its frozen
//! absolute path.
//!
//! Defaults: the repository is the one this binary was built in
//! (`CARGO_MANIFEST_DIR`), and the output tree is `.bench/sealed-court-v3` under
//! it. Both are overridable. Output is byte-identical to the Python original
//! (`diff -r --no-dereference` clean). **Not on the verdict path.**

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::tools::json::{self, Json};

const USAGE: &str = "\
usage: sealed_court [--repo <repo-root>] [--court <court-dir>]

  Builds the pruned, proof-free sealed input tree.

  --repo   repository root (default: the repository this binary was built in,
           i.e. CARGO_MANIFEST_DIR)
  --court  output tree (default: <repo>/.bench/sealed-court-v3)
";

/// The nine sealed x86 tasks, in the original order.
pub const TASK_IDS: [&str; 9] = [
    "x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT",
    "x86.bignum_sub_p25519.BIGNUM_SUB_P25519_CORRECT",
    "x86.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT",
    "x86.bignum_add_p256k1.BIGNUM_ADD_P256K1_CORRECT",
    "x86.bignum_sub_p256k1.BIGNUM_SUB_P256K1_CORRECT",
    "x86.bignum_neg_p256k1.BIGNUM_NEG_P256K1_CORRECT",
    "x86.bignum_add_sm2.BIGNUM_ADD_SM2_CORRECT",
    "x86.bignum_sub_sm2.BIGNUM_SUB_SM2_CORRECT",
    "x86.bignum_neg_sm2.BIGNUM_NEG_SM2_CORRECT",
];

/// Bench scripts and data copied verbatim into `repo/.bench`.
const BENCH_FILES: [&str; 7] = [
    "problems.json",
    "timeouts.json",
    "timeout-map.json",
    "combine-answer-and-setup.py",
    "run-answers.sh",
    "collect-verdicts.py",
    "synchk.sh",
];

/// A template path and the line at which the harness stops reading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    pub rel: String,
    pub linenum: usize,
}

fn family_dir(family: &str) -> Option<&'static str> {
    Some(match family {
        "p25519" => "curve25519",
        "p256k1" => "secp256k1",
        "sm2" => "sm2",
        _ => return None,
    })
}

fn mkdir_all(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|e| format!("{}: {e}", path.display()))
}

/// Make `path` absolute against the current directory, without resolving `..`.
/// `std::path::absolute` is newer than this crate's minimum Rust version.
fn absolute(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    }
}

fn copy_file(from: &Path, to: &Path) -> Result<(), String> {
    if let Some(parent) = to.parent() {
        mkdir_all(parent)?;
    }
    fs::copy(from, to)
        .map(|_| ())
        .map_err(|e| format!("{} -> {}: {e}", from.display(), to.display()))
}

/// Recursively copy `src` into `dst`. When `follow_symlinks` is false, symlinks
/// are recreated as symlinks; otherwise their targets are copied.
pub fn copy_tree(src: &Path, dst: &Path, follow_symlinks: bool) -> Result<(), String> {
    mkdir_all(dst)?;
    let entries = fs::read_dir(src).map_err(|e| format!("{}: {e}", src.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("{}: {e}", src.display()))?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        let file_type = entry
            .file_type()
            .map_err(|e| format!("{}: {e}", from.display()))?;
        if file_type.is_symlink() {
            if follow_symlinks {
                let target = fs::metadata(&from).map_err(|e| format!("{}: {e}", from.display()))?;
                if target.is_dir() {
                    copy_tree(&from, &to, follow_symlinks)?;
                } else {
                    copy_file(&from, &to)?;
                }
            } else {
                let link = fs::read_link(&from).map_err(|e| format!("{}: {e}", from.display()))?;
                std::os::unix::fs::symlink(&link, &to)
                    .map_err(|e| format!("{}: {e}", to.display()))?;
            }
        } else if file_type.is_dir() {
            copy_tree(&from, &to, follow_symlinks)?;
        } else {
            copy_file(&from, &to)?;
        }
    }
    Ok(())
}

/// Read `inlined_locations[0]` for each of the nine tasks.
pub fn template_plan(problems: &Json) -> Result<Vec<Template>, String> {
    let mut out = Vec::new();
    for task in TASK_IDS {
        let entry = problems
            .get(task)
            .ok_or_else(|| format!("problems.json has no entry {task:?}"))?;
        let locations = entry
            .get("inlined_locations")
            .and_then(Json::as_array)
            .ok_or_else(|| format!("{task:?} has no inlined_locations array"))?;
        let first = locations
            .first()
            .ok_or_else(|| format!("{task:?} has an empty inlined_locations array"))?;
        let pair = first
            .as_array()
            .ok_or_else(|| format!("{task:?} inlined_locations[0] is not a pair"))?;
        let rel = pair
            .first()
            .and_then(Json::as_str)
            .ok_or_else(|| format!("{task:?} inlined_locations[0][0] is not a string"))?;
        let linenum = match pair.get(1) {
            Some(Json::Num(n)) if *n >= 0.0 && n.fract() == 0.0 => *n as usize,
            _ => {
                return Err(format!(
                    "{task:?} inlined_locations[0][1] is not a non-negative integer"
                ))
            }
        };
        out.push(Template {
            rel: rel.to_string(),
            linenum,
        });
    }
    Ok(out)
}

/// The first `linenum` lines of `bytes`, and how many lines that actually was.
///
/// Lines keep their `\n`, matching iterating a file in the original. A trailing
/// fragment without a newline counts as one line.
pub fn prefix_bytes(bytes: &[u8], linenum: usize) -> (Vec<u8>, usize) {
    let mut out = Vec::new();
    let mut count = 0usize;
    let mut start = 0usize;
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'\n' {
            if count >= linenum {
                return (out, count);
            }
            out.extend_from_slice(&bytes[start..=i]);
            count += 1;
            start = i + 1;
        }
    }
    if count < linenum && start < bytes.len() {
        out.extend_from_slice(&bytes[start..]);
        count += 1;
    }
    (out, count)
}

fn split_task(task: &str) -> Result<(String, String), String> {
    let rest = task
        .strip_prefix("x86.")
        .ok_or_else(|| format!("task {task:?} is not an x86 task"))?;
    let base = rest.split('.').next().ok_or("task id has no base")?;
    let body = base
        .strip_prefix("bignum_")
        .ok_or_else(|| format!("task base {base:?} does not start bignum_"))?;
    let (op, family) = body
        .split_once('_')
        .ok_or_else(|| format!("bignum base {body:?} has no operation"))?;
    Ok((op.to_string(), family.to_string()))
}

/// Build the tree at `court` from `repo`.
pub fn build(repo: &Path, court: &Path) -> Result<(), String> {
    let bench = repo.join(".bench");
    let r = court.join("repo");
    let rb = r.join(".bench");

    if court.exists() {
        fs::remove_dir_all(court).map_err(|e| format!("{}: {e}", court.display()))?;
    }
    for dir in [
        rb.join("ml_files/x86"),
        rb.join("objfiles/x86/generic"),
        rb.join("s2n-bignum/x86/generic"),
        r.join("tools"),
        r.join("research/receipts"),
    ] {
        mkdir_all(&dir)?;
    }

    let problems_path = bench.join("problems.json");
    let problems = json::parse(
        &fs::read(&problems_path).map_err(|e| format!("{}: {e}", problems_path.display()))?,
    )
    .map_err(|e| format!("{}: {e}", problems_path.display()))?;

    // Bench scripts and data. `template-cache.json` is omitted on purpose: it is
    // a rebuildable cache that the harness writes itself.
    for name in BENCH_FILES {
        copy_file(&bench.join(name), &rb.join(name))?;
    }
    let run_answers = rb.join("run-answers.sh");
    fs::set_permissions(&run_answers, fs::Permissions::from_mode(0o755))
        .map_err(|e| format!("{}: {e}", run_answers.display()))?;

    // Templates truncated to the prefix the harness reads.
    for template in template_plan(&problems)? {
        let source = bench.join(&template.rel);
        let bytes = fs::read(&source).map_err(|e| format!("{}: {e}", source.display()))?;
        let (prefix, count) = prefix_bytes(&bytes, template.linenum);
        let dest = rb.join(&template.rel);
        if let Some(parent) = dest.parent() {
            mkdir_all(parent)?;
        }
        fs::write(&dest, &prefix).map_err(|e| format!("{}: {e}", dest.display()))?;
        println!(
            "template {} prefix_lines={} of linenum={}",
            template.rel, count, template.linenum
        );
    }

    // Nine task objects for the emitter, plus the generic madd object that the
    // judge's prelude declares.
    for task in TASK_IDS {
        let (op, family) = split_task(task)?;
        let dir = family_dir(&family).ok_or_else(|| format!("unknown family {family:?}"))?;
        let name = format!("bignum_{op}_{family}.o");
        let source = bench.join("s2n-bignum/x86").join(dir).join(&name);
        for parent in [rb.join("s2n-bignum/x86"), rb.join("objfiles/x86")] {
            let dest_dir = parent.join(dir);
            mkdir_all(&dest_dir)?;
            copy_file(&source, &dest_dir.join(&name))?;
        }
    }
    let madd = bench.join("s2n-bignum/x86/generic/bignum_madd.o");
    for parent in [
        rb.join("s2n-bignum/x86/generic"),
        rb.join("objfiles/x86/generic"),
    ] {
        copy_file(&madd, &parent.join("bignum_madd.o"))?;
    }

    // Frozen tools, git metadata, manifest.
    copy_tree(&repo.join("tools"), &r.join("tools"), true)?;
    copy_tree(&repo.join(".git"), &r.join(".git"), false)?;
    copy_file(
        &repo.join("research/receipts/ix54-sealed-manifest.json"),
        &r.join("research/receipts/ix54-sealed-manifest.json"),
    )?;

    // HOL Light is bind-mounted read-only at its frozen absolute path; the
    // harness expects ./hol-light next to the bench scripts.
    let hol = absolute(&bench.join("hol-light"));
    let link = rb.join("hol-light");
    std::os::unix::fs::symlink(&hol, &link).map_err(|e| format!("{}: {e}", link.display()))?;

    println!("built {}", r.display());
    let du = Command::new("du")
        .arg("-sh")
        .arg(court)
        .output()
        .map_err(|e| format!("du: {e}"))?;
    println!("{}", String::from_utf8_lossy(&du.stdout).trim());
    Ok(())
}

/// CLI entry point. `args` excludes the program name.
pub fn run(args: &[String]) -> i32 {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print!("{USAGE}");
        return 0;
    }
    let mut repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut court: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--repo" => match args.get(i + 1) {
                Some(v) => {
                    repo = PathBuf::from(v);
                    i += 2;
                }
                None => {
                    eprint!("{USAGE}");
                    return 2;
                }
            },
            "--court" => match args.get(i + 1) {
                Some(v) => {
                    court = Some(PathBuf::from(v));
                    i += 2;
                }
                None => {
                    eprint!("{USAGE}");
                    return 2;
                }
            },
            other => {
                eprintln!("ERROR unknown argument {other:?}");
                eprint!("{USAGE}");
                return 2;
            }
        }
    }
    let court = court.unwrap_or_else(|| repo.join(".bench/sealed-court-v3"));
    match build(&repo, &court) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("ERROR {e}");
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_problems() -> Json {
        json::parse(
            br#"{
              "x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT": {
                "query": "q",
                "inlined_locations": [["ml_files/x86/bignum_add_p25519.ml", 3]]
              }
            }"#,
        )
        .unwrap()
    }

    #[test]
    fn template_plan_reads_the_first_location() {
        let plan = template_plan(&synthetic_problems());
        assert!(plan.is_err(), "the other eight tasks must be required");
        assert!(plan.unwrap_err().contains("sub_p25519"));
    }

    #[test]
    fn prefix_bytes_takes_exactly_linenum_lines() {
        let (out, n) = prefix_bytes(b"l1\nl2\nl3\n", 2);
        assert_eq!(out, b"l1\nl2\n");
        assert_eq!(n, 2);
    }

    #[test]
    fn prefix_bytes_keeps_everything_when_linenum_exceeds_the_file() {
        let (out, n) = prefix_bytes(b"l1\nl2\nl3", 10);
        assert_eq!(out, b"l1\nl2\nl3");
        assert_eq!(n, 3);
    }

    #[test]
    fn prefix_bytes_of_zero_is_empty() {
        let (out, n) = prefix_bytes(b"l1\nl2\n", 0);
        assert!(out.is_empty());
        assert_eq!(n, 0);
    }

    #[test]
    fn split_task_recovers_op_and_family() {
        assert_eq!(
            split_task("x86.bignum_neg_sm2.BIGNUM_NEG_SM2_CORRECT").unwrap(),
            ("neg".to_string(), "sm2".to_string())
        );
    }
}
