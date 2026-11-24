//! Statement challenge files and an independent comparator (Ordeal O1.1/O1.2).
//!
//! Rust port of the earlier Python helper. **Not on the verdict path.**
//!
//! `freeze` writes a plain-text challenge: exact statement bytes, their sha256,
//! the corpus digest, the object digest and the s2n commit. `verify` re-derives
//! every digest and checks that the judge program that ran embeds the statement.
//! `verify --upstream` additionally compares the statement with the goal
//! quotation in the pinned upstream `.ml`.

use crate::tools::json;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const FORMAT: &str = "tnps-challenge v1";
const BEGIN: &[u8] = b"--- statement begin ---\n";
const END: &[u8] = b"--- statement end ---\n";

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    let out = h.finalize();
    let mut s = String::with_capacity(64);
    for b in out {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn find_sub(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || hay.len() < needle.len() {
        return None;
    }
    hay.windows(needle.len()).position(|w| w == needle)
}

struct Challenge {
    fields: BTreeMap<String, String>,
    statement: Vec<u8>,
}

impl Challenge {
    fn get(&self, k: &str) -> Option<&str> {
        self.fields.get(k).map(String::as_str)
    }
}

fn parse_challenge(raw: &[u8]) -> Result<Challenge, String> {
    let b = find_sub(raw, BEGIN).ok_or("missing statement begin marker")?;
    let e = find_sub(&raw[b + BEGIN.len()..], END)
        .map(|x| b + BEGIN.len() + x)
        .ok_or("missing statement end marker")?;
    let header = std::str::from_utf8(&raw[..b]).map_err(|_| "header is not UTF-8")?;
    let mut block = raw[b + BEGIN.len()..e].to_vec();
    if block.last() != Some(&b'\n') {
        return Err("malformed statement block".to_string());
    }
    block.pop();
    let mut lines = header.lines();
    if lines.next() != Some(FORMAT) {
        return Err(format!("not a {FORMAT} file"));
    }
    let mut fields = BTreeMap::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let (k, v) = line.split_once(' ').ok_or("header line without a value")?;
        if fields.insert(k.to_string(), v.to_string()).is_some() {
            return Err(format!("duplicate header key {k:?}"));
        }
    }
    Ok(Challenge {
        fields,
        statement: block,
    })
}

fn parse_task(task: &str) -> Result<(&str, &str, &str), String> {
    let (arch, rest) = task.split_once('.').ok_or("task id has no '.'")?;
    if arch != "arm" && arch != "x86" {
        return Err(format!("unknown architecture {arch:?}"));
    }
    let base = rest.split('.').next().ok_or("task id has no base")?;
    let body = base
        .strip_prefix("bignum_")
        .ok_or("task base does not start bignum_")?;
    let (op, family) = body.split_once('_').ok_or("bignum base has no operation")?;
    if !matches!(op, "add" | "sub" | "neg") {
        return Err(format!("not an add/sub/neg task: {op:?}"));
    }
    Ok((arch, op, family))
}

fn object_for(s2n: &Path, task: &str) -> Result<PathBuf, String> {
    let (arch, op, family) = parse_task(task)?;
    let name = format!("bignum_{op}_{family}.o");
    let mut hits = Vec::new();
    let arch_dir = s2n.join(arch);
    let entries =
        fs::read_dir(&arch_dir).map_err(|e| format!("cannot read {}: {e}", arch_dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let cand = entry.path().join(&name);
        if cand.is_file() {
            hits.push(cand);
        }
    }
    if hits.len() != 1 {
        return Err(format!(
            "expected exactly one {arch} object for {op}/{family}, found {}",
            hits.len()
        ));
    }
    Ok(hits.remove(0))
}

fn git_head(dir: &Path) -> String {
    if !dir.join(".git").exists() {
        return "unknown".to_string();
    }
    match Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "HEAD"])
        .output()
    {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        _ => "unknown".to_string(),
    }
}

fn arg_value<'a>(args: &'a [String], key: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == key)
        .and_then(|i| args.get(i + 1))
        .map(String::as_str)
}

fn has_flag(args: &[String], key: &str) -> bool {
    args.iter().any(|a| a == key)
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn freeze(args: &[String]) -> Result<(), String> {
    let task = arg_value(args, "--task").ok_or("--task is required")?;
    parse_task(task)?;
    let out = arg_value(args, "--out").ok_or("--out is required")?;
    let problems = arg_value(args, "--problems")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_root().join(".bench/problems.json"));
    let s2n = arg_value(args, "--s2n-bignum")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_root().join(".bench/s2n-bignum"));

    let corpus_bytes = fs::read(&problems).map_err(|e| format!("{problems:?}: {e}"))?;
    let corpus = json::parse(&corpus_bytes).map_err(|e| format!("corpus: {e}"))?;
    let entry = corpus
        .get(task)
        .ok_or_else(|| format!("{task:?} is not in the corpus"))?;
    let statement = entry
        .get("query")
        .and_then(|q| q.as_str())
        .ok_or_else(|| format!("{task:?} has no query"))?
        .to_string();
    let category = entry
        .get("category")
        .and_then(|c| c.as_str())
        .unwrap_or("unknown")
        .to_string();

    let obj = object_for(&s2n, task)?;
    let obj_rel = obj
        .strip_prefix(&s2n)
        .map_err(|_| "object is outside the s2n tree")?
        .to_string_lossy()
        .replace('\\', "/");
    let obj_bytes = fs::read(&obj).map_err(|e| format!("{obj:?}: {e}"))?;

    let (frozen_at, pre_search) = match arg_value(args, "--post-hoc") {
        Some(reason) => (format!("POST_HOC: {reason}"), false),
        None => (
            arg_value(args, "--frozen-at")
                .unwrap_or("PRE_SEARCH")
                .to_string(),
            true,
        ),
    };

    let sb = statement.as_bytes();
    let header = format!(
        "{FORMAT}\ntask {task}\ncategory {category}\nstandard tnps/1\n\
         frozen-at {frozen_at}\npre-search {pre_search}\n\
         problems-json-sha256 {}\nstatement-sha256 {}\nstatement-bytes {}\n\
         object {obj_rel}\nobject-sha256 {}\ns2n-bignum-commit {}\n\n",
        sha256_hex(&corpus_bytes),
        sha256_hex(sb),
        sb.len(),
        sha256_hex(&obj_bytes),
        git_head(&s2n),
    );
    let mut bytes = header.into_bytes();
    bytes.extend_from_slice(BEGIN);
    bytes.extend_from_slice(sb);
    bytes.push(b'\n');
    bytes.extend_from_slice(END);

    if let Some(parent) = Path::new(out).parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(out, &bytes).map_err(|e| e.to_string())?;
    println!(
        "froze {out} task={task} statement={}B object={obj_rel} pre_search={pre_search}",
        sb.len()
    );
    Ok(())
}

fn normalise(text: &str) -> String {
    let t = if text.starts_with('`') && text.ends_with('`') && text.len() > 1 {
        &text[1..text.len() - 1]
    } else {
        text
    };
    t.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn upstream_goal(path: &Path, theorem: &str) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    let marker = format!("let {theorem} =");
    let at = text.find(&marker)?;
    let rest = &text[at..];
    let s = rest.find('`')?;
    let t = rest[s + 1..].find('`')?;
    Some(rest[s + 1..s + 1 + t].to_string())
}

fn wrapper_needle(task: &str, statement: &str) -> Vec<u8> {
    format!("bench_run \"{task}\" ({statement}) (fun () ->").into_bytes()
}

pub fn verify(args: &[String], require_pre_search: bool) -> Result<i32, String> {
    let path = arg_value(args, "--challenge").ok_or("--challenge is required")?;
    let raw = fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    let ch = parse_challenge(&raw)?;
    let task = ch.get("task").ok_or("challenge has no task")?.to_string();
    let statement = ch.statement.clone();

    let problems = arg_value(args, "--problems")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_root().join(".bench/problems.json"));
    let s2n = arg_value(args, "--s2n-bignum")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_root().join(".bench/s2n-bignum"));
    let corpus_bytes = fs::read(&problems).map_err(|e| format!("{problems:?}: {e}"))?;
    let corpus = json::parse(&corpus_bytes).map_err(|e| format!("corpus: {e}"))?;

    let mut ok = true;
    let mut incomplete = false;
    let mut check = |name: &str, cond: bool, detail: &str| {
        println!(
            "{} {name}{}",
            if cond { "PASS" } else { "FAIL" },
            if detail.is_empty() {
                String::new()
            } else {
                format!("  {detail}")
            }
        );
        if !cond {
            ok = false;
        }
    };

    let stmt_hash = sha256_hex(&statement);
    let stmt_len = statement.len().to_string();
    let corpus_hash = sha256_hex(&corpus_bytes);
    check(
        "statement-sha256",
        ch.get("statement-sha256") == Some(stmt_hash.as_str()),
        &format!("recorded={}", ch.get("statement-sha256").unwrap_or("")),
    );
    check(
        "statement-bytes",
        ch.get("statement-bytes") == Some(stmt_len.as_str()),
        &format!("recorded={}", ch.get("statement-bytes").unwrap_or("")),
    );
    check(
        "problems-json-sha256",
        ch.get("problems-json-sha256") == Some(corpus_hash.as_str()),
        &format!("recorded={}", ch.get("problems-json-sha256").unwrap_or("")),
    );
    let entry = corpus.get(&task);
    check("task-present", entry.is_some(), "");
    if let Some(entry) = entry {
        let query = entry.get("query").and_then(|q| q.as_str()).unwrap_or("");
        check(
            "statement==problems.json",
            query.as_bytes() == statement.as_slice(),
            "",
        );
        check(
            "category",
            entry.get("category").and_then(|c| c.as_str())
                == Some(ch.get("category").unwrap_or("")),
            "",
        );
    }

    let obj_rel = ch.get("object").unwrap_or("");
    let obj = s2n.join(obj_rel);
    check("object-exists", obj.is_file(), &obj.display().to_string());
    if obj.is_file() {
        let bytes = fs::read(&obj).map_err(|e| e.to_string())?;
        check(
            "object-sha256",
            ch.get("object-sha256") == Some(sha256_hex(&bytes).as_str()),
            "",
        );
    }

    let recorded_commit = ch.get("s2n-bignum-commit").unwrap_or("unknown");
    let live_commit = git_head(&s2n);
    if recorded_commit == "unknown" {
        println!("NOTE s2n-bignum-commit was not recorded");
    } else {
        check(
            "s2n-bignum-commit",
            live_commit == recorded_commit,
            &format!(
                "recorded={} live={}",
                &recorded_commit[..12.min(recorded_commit.len())],
                &live_commit[..12.min(live_commit.len())]
            ),
        );
    }

    if let Some(evaldir) = arg_value(args, "--evaldir") {
        let dir = Path::new(evaldir);
        let mut scanned = 0usize;
        let mut hits = Vec::new();
        let entries = fs::read_dir(dir).map_err(|e| format!("{evaldir}: {e}"))?;
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("ml_files_") || !name.ends_with(".ml") {
                continue;
            }
            scanned += 1;
            let bytes = fs::read(entry.path()).unwrap_or_default();
            if find_sub(
                &bytes,
                &wrapper_needle(&task, &String::from_utf8_lossy(&statement)),
            )
            .is_some()
            {
                hits.push(name);
            }
        }
        check(
            "judge-program-scanned",
            scanned > 0,
            &format!("{scanned} ml_files_*.ml"),
        );
        check(
            "judge-embeds-statement",
            hits.len() == 1,
            &format!(
                "{} of {scanned} judge program(s) embed it{}",
                hits.len(),
                if hits.len() == 1 {
                    format!("; {}", hits[0])
                } else {
                    String::new()
                }
            ),
        );
        let judge = dir.join(format!("{task}.judge.txt"));
        let verdict = fs::read_to_string(&judge).unwrap_or_default();
        check(
            "judge-verdict-ok",
            verdict.trim() == "OK",
            if verdict.is_empty() {
                "missing verdict file"
            } else {
                verdict.trim()
            },
        );
    } else {
        incomplete = true;
        println!("NOTE no --evaldir: verdict evidence missing, result is INCOMPLETE");
    }

    if has_flag(args, "--upstream") {
        if let Some(entry) = entry {
            let j = entry.get("json");
            let file = j
                .and_then(|x| x.get("filename"))
                .and_then(|x| x.as_str())
                .unwrap_or("");
            let name = j
                .and_then(|x| x.get("theorem_name"))
                .and_then(|x| x.as_str())
                .unwrap_or("");
            let goal = if file.is_empty() || name.is_empty() {
                None
            } else {
                upstream_goal(Path::new(file), name)
            };
            check(
                "statement==upstream-goal",
                goal.as_deref().map(normalise)
                    == Some(normalise(&String::from_utf8_lossy(&statement))),
                if goal.is_some() {
                    "goal quotation from the pinned .ml"
                } else {
                    "no goal extracted"
                },
            );
        }
    }

    if require_pre_search {
        check(
            "pre-search",
            ch.get("pre-search") == Some("true"),
            &format!("frozen-at={}", ch.get("frozen-at").unwrap_or("")),
        );
    }

    println!("NOTE NOT CHECKED: kernel-term identity; the axiom whitelist is covered by the upstream drop-in");
    println!("NOTE NOT CHECKED: the statement's upstream provenance unless --upstream is given");

    if !ok {
        println!("RESULT FAIL {task}");
        return Ok(1);
    }
    if incomplete {
        println!("RESULT INCOMPLETE {task}");
        return Ok(3);
    }
    println!("RESULT PASS {task}");
    Ok(0)
}

pub fn run(args: &[String]) -> i32 {
    let cmd = args.first().map(String::as_str).unwrap_or("");
    let rest = if args.is_empty() { args } else { &args[1..] };
    let result = match cmd {
        "freeze" => freeze(rest).map(|()| 0),
        "verify" => verify(rest, has_flag(rest, "--require-pre-search")),
        _ => Err("usage: challenge <freeze|verify> [options]".to_string()),
    };
    match result {
        Ok(code) => code,
        Err(e) => {
            eprintln!("ERROR {e}");
            2
        }
    }
}
