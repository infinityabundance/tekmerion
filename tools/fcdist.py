#!/usr/bin/env python3
"""fcdist.py -- recount functional-correctness (FC) theorems in s2n-bignum.

Reads a pinned s2n-bignum checkout directly from source (no HOL Light, no
build) and applies the benchmark collector's own classification rule, copied
from `.bench/collect-problems.py::categorize`:

  * the goal argument must be a backtick-delimited term, else the theorem is
    dropped ("the goal is evaluated at runtime");
  * a name containing `_EQUIV` or `SUBROUTINE_SAFE` is dropped ("not a proof
    related to functional correctness");
  * a name containing `SUBROUTINE_CORRECT` is dropped ("a top-level
    subroutine that just uses the _CORRECT of the core part");
  * otherwise, `_CORRECT` in the name and the standalone token `ensures` in
    the goal make it FC (arm/x86 by top-level directory).

The goal-token test reproduces `contains_anykw(["ensures"], goal)` exactly,
including the backtick stripping, newline folding and surrounding spaces.

Definitions used here, stated because they are not fixed by the bench:

  proof length   non-blank, non-comment lines in the *tactic* part of the
                 `prove (...)` / `prove_by_refinement (...)` argument list,
                 i.e. everything after the goal term.  Comments and string
                 bodies are masked before counting, so a line that is only a
                 comment never counts.
  loop theorem   the tactic part mentions `ENSURES_WHILE*` (the only place
                 loop machinery appears in s2n-bignum; goals never contain
                 it).  Straight-line is its complement inside FC.
  family         operation token parsed from the theorem name, falling back
                 to the file stem, longest keyword first.

Usage:
  python3 tools/fcdist.py .bench/s2n-bignum
  python3 tools/fcdist.py .bench/s2n-bignum --corpus .bench/problems.json
  python3 tools/fcdist.py .bench/s2n-bignum --json

The measured commit is always printed; a number cannot be read off without
its revision.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import statistics
import subprocess
from collections import Counter, defaultdict

# ---------------------------------------------------------------------------
# Benchmark rule (mirrors .bench/collect-problems.py)
# ---------------------------------------------------------------------------

def contains_anykw(keywords, text):
    """Exact port of collect-problems.py::contains_anykw."""
    for keyword in keywords:
        if keyword.endswith("*"):
            if f" {keyword[:-1]}" in text:
                return True
            continue
        if (
            f" {keyword}" in text
            or f"{keyword} " in text
            or f"({keyword}" in text
            or f"{keyword})" in text
        ):
            return True
    return False


def bench_classify(thm_name, goal):
    """Return (drop?, reason) mirroring categorize() for a goal-only view.

    `goal` is the first argument of the theorem's proof application.  Only the
    parts of categorize() that can be decided without the runtime goal are
    reproduced; the caller still has to supply a backtick-delimited goal.
    """
    goal = goal.strip()
    first = goal.find("`")
    last = goal.rfind("`")
    if first == -1 or last == -1 or first == last:
        return True, "the goal is evaluated at runtime"
    if first != 0 or last != len(goal) - 1:
        goal = goal[first : last + 1]

    if "_EQUIV" in thm_name or "SUBROUTINE_SAFE" in thm_name:
        return True, "not a proof related to functional correctness"
    if "SUBROUTINE_CORRECT" in thm_name:
        return True, "a top-level subroutine that just uses the _CORRECT of the core part"

    folded = " " + goal.replace("\n", " ") + " "
    if "_CORRECT" in thm_name and contains_anykw(["ensures"], folded):
        return False, "functional_correctness"
    return False, "non_fc"


# ---------------------------------------------------------------------------
# Source scanning
# ---------------------------------------------------------------------------

DECL_RE = re.compile(r"(?m)^let[ \t]+([A-Za-z][A-Za-z0-9_']*)[ \t]*=")
APPLY_RE = re.compile(r"\s*(?:time\s+)?(prove_by_refinement|prove)\s*\(")


def mask_source(src):
    """Blank out OCaml comments and string bodies, preserving offsets.

    Newlines are kept so line numbers stay aligned with the original file.
    Handles nested `(* ... *)` (OCaml) and backslash escapes in strings.
    """
    out = list(src)
    i, n = 0, len(src)
    mode = "normal"
    depth = 0
    while i < n:
        c = src[i]
        if mode == "normal":
            if src.startswith("(*", i):
                mode, depth = "comment", 1
                out[i] = out[i + 1] = " "
                i += 2
                continue
            if c == '"':
                mode = "string"
                out[i] = " "
                i += 1
                continue
            i += 1
        elif mode == "comment":
            if src.startswith("(*", i):
                depth += 1
                out[i] = out[i + 1] = " "
                i += 2
                continue
            if src.startswith("*)", i):
                depth -= 1
                out[i] = out[i + 1] = " "
                i += 2
                if depth == 0:
                    mode = "normal"
                continue
            if c != "\n":
                out[i] = " "
            i += 1
        else:  # string
            if c != "\n":
                out[i] = " "
            if c == "\\" and i + 1 < n:
                if src[i + 1] != "\n":
                    out[i + 1] = " "
                i += 2
                continue
            if c == '"':
                mode = "normal"
            i += 1
    return "".join(out)


def match_paren(masked, open_idx):
    """Index of the ')' matching masked[open_idx] == '(', or -1."""
    depth = 0
    for i in range(open_idx, len(masked)):
        ch = masked[i]
        if ch == "(":
            depth += 1
        elif ch == ")":
            depth -= 1
            if depth == 0:
                return i
    return -1


def count_content_lines(masked, start, end):
    if end <= start:
        return 0
    return sum(1 for line in masked[start:end].split("\n") if line.strip())


def scan_file(path, relpath, arch):
    """Yield one dict per top-level `let NAME = prove ...` declaration."""
    with open(path, "r", encoding="utf-8", errors="replace") as fh:
        src = fh.read()
    masked = mask_source(src)
    found = []
    for m in DECL_RE.finditer(masked):
        name = m.group(1)
        am = APPLY_RE.match(masked, m.end())
        if not am:
            continue
        open_idx = am.end() - 1  # points at '('
        close_idx = match_paren(masked, open_idx)
        if close_idx == -1:
            continue
        inner = masked[open_idx + 1 : close_idx]
        # Goal argument: first backtick-delimited term inside the call.
        bt1 = masked.find("`", open_idx + 1, close_idx)
        goal_text = ""
        tactic_start = open_idx + 1
        if bt1 != -1:
            bt2 = masked.find("`", bt1 + 1, close_idx)
            if bt2 != -1:
                goal_text = src[bt1 : bt2 + 1]
                tactic_start = bt2 + 1
        else:
            goal_text = masked[open_idx + 1 : close_idx]
        found.append(
            {
                "file": relpath,
                "arch": arch,
                "name": name,
                "goal": goal_text,
                "has_goal_term": bt1 != -1,
                "tactic_lines": count_content_lines(masked, tactic_start, close_idx),
                "span_lines": count_content_lines(masked, open_idx, close_idx),
                "is_loop": "ENSURES_WHILE" in masked[tactic_start:close_idx],
                "line": src.count("\n", 0, m.start()) + 1,
            }
        )
    return found


def git_head(repo):
    try:
        out = subprocess.run(
            ["git", "-C", repo, "rev-parse", "HEAD"],
            capture_output=True, text=True, check=True,
        )
        return out.stdout.strip()
    except (OSError, subprocess.CalledProcessError) as exc:
        return f"<unavailable: {exc}>"


def scan_checkout(repo):
    if not os.path.isdir(os.path.join(repo, "arm", "proofs")):
        raise SystemExit(f"not an s2n-bignum checkout (no arm/proofs): {repo}")
    rows = []
    for arch in ("arm", "x86"):
        root = os.path.join(repo, arch, "proofs")
        for dirpath, _dirnames, filenames in os.walk(root):
            for fn in sorted(filenames):
                if not fn.endswith(".ml"):
                    continue
                full = os.path.join(dirpath, fn)
                rel = os.path.relpath(full, repo)
                rows.extend(scan_file(full, rel, arch))
    return rows


# ---------------------------------------------------------------------------
# Families
# ---------------------------------------------------------------------------

# Longest-first so 'montmul' wins over 'mul' and 'modsub' resolves explicitly.
# This vocabulary is *ours*; the programme's v3 list is not recorded anywhere,
# so family totals are only comparable up to a stated vocabulary.
FAMILY_PATTERNS = [
    ("montmul", r"montmul"),
    ("montsqr", r"montsqr"),
    ("amontmul", r"amontmul"),
    ("amontsqr", r"amontsqr"),
    ("amontredc", r"amontredc"),
    ("emontredc", r"emontredc"),
    ("montinv", r"montinv"),
    ("tomont", r"tomont"),
    ("demont", r"demont"),
    ("deamont", r"deamont"),
    ("modinv", r"modinv"),
    ("modsub", r"modsub"),
    ("modadd", r"modadd"),
    ("moddouble", r"moddouble"),
    ("optneg", r"optneg"),
    ("optadd", r"optadd"),
    ("optsub", r"optsub"),
    ("cmnegadd", r"cmnegadd"),
    ("cmadd", r"cmadd"),
    ("cmul", r"cmul"),
    ("cmod", r"cmod"),
    ("cdiv", r"cdiv"),
    ("madd", r"madd"),
    ("invsqrt", r"invsqrt"),
    ("triple", r"triple"),
    ("double", r"double"),
    ("half", r"half"),
    ("sqrt", r"sqrt"),
    ("inv", r"inv"),
    ("neg", r"neg"),
    ("add", r"add"),
    ("sub", r"sub"),
    ("mod", r"mod"),
    ("mul", r"mul"),
    ("sqr", r"sqr"),
]

CHUNK_RE = re.compile(r"[a-z0-9]+")


def family_of(name, file, arch):
    """Operation token: theorem name first, then file stem.

    Strips architecture prefixes and curve suffixes so that e.g.
    `LOCAL_MONTMUL_P256_CORRECT` in `p256_montjadd.ml` classifies as montmul.
    """
    stem = os.path.basename(file)[:-3]
    candidates = [name.lower()]
    if stem.lower() != name.lower():
        candidates.append(stem.lower())
    for raw in candidates:
        tokens = CHUNK_RE.findall(raw)
        # Try the whole blob and each token, longest pattern first.
        for _family, pattern in FAMILY_PATTERNS:
            if re.search(pattern, raw):
                # Require the match to be a self-contained operation word:
                # either a token, or a token with a known prefix/suffix glued.
                for tok in tokens:
                    if re.fullmatch(rf"[a-z0-9]*(?:{pattern})[a-z0-9]*", tok):
                        return _family
    return None


# ---------------------------------------------------------------------------
# Statistics
# ---------------------------------------------------------------------------

BANDS = [(0, 24), (25, 49), (50, 99), (100, 199), (200, 299), (300, 599), (600, 10**9)]


def band_label(lo, hi):
    return f"{lo}-{hi}" if hi < 10**9 else f"{lo}+"


def summarize(values):
    if not values:
        return {"n": 0, "median": None, "q1": None, "q3": None, "min": None,
                "max": None, "mean": None, "bands": {}}
    vals = sorted(values)
    q = statistics.quantiles(vals, n=4, method="inclusive") if len(vals) > 1 else [vals[0]] * 3
    bands = Counter()
    for v in vals:
        for lo, hi in BANDS:
            if lo <= v <= hi:
                bands[band_label(lo, hi)] += 1
                break
    return {
        "n": len(vals),
        "median": statistics.median(vals),
        "q1": q[0],
        "q3": q[2],
        "min": vals[0],
        "max": vals[-1],
        "mean": round(sum(vals) / len(vals), 2),
        "bands": {band_label(lo, hi): bands.get(band_label(lo, hi), 0) for lo, hi in BANDS},
    }


def normalise_goal(goal):
    return " ".join(goal.split())


def build_report(repo, corpus_path=None):
    rows = scan_checkout(repo)
    commit = git_head(repo)

    # Classify every top-level theorem that the rule cares about.
    fc, dropped, non_fc = [], [], []
    equiv_names, safe_names, ensures2_goals = [], [], []
    for r in rows:
        if "_EQUIV" in r["name"]:
            equiv_names.append(r)
        if "SUBROUTINE_SAFE" in r["name"]:
            safe_names.append(r)
        if "ensures2" in r["goal"]:
            ensures2_goals.append(r)
        drop, reason = bench_classify(r["name"], r["goal"])
        if drop:
            r["reason"] = reason
            dropped.append(r)
        elif reason == "functional_correctness":
            r["loop"] = r["is_loop"]
            fc.append(r)
        else:
            non_fc.append(r)

    direct = Counter(r["arch"] for r in fc)
    unique_goals = {}
    for r in fc:
        unique_goals.setdefault(normalise_goal(r["goal"]), []).append(r)

    straight = [r for r in fc if not r["loop"]]
    loops = [r for r in fc if r["loop"]]

    length_stats = {
        "tactic": summarize([r["tactic_lines"] for r in fc]),
        "span": summarize([r["span_lines"] for r in fc]),
    }
    arch_lengths = {
        a: summarize([r["tactic_lines"] for r in fc if r["arch"] == a])
        for a in ("arm", "x86")
    }
    shape_by_arch = {
        a: {
            "straight_line": sum(1 for r in fc if r["arch"] == a and not r["loop"]),
            "loop": sum(1 for r in fc if r["arch"] == a and r["loop"]),
        }
        for a in ("arm", "x86")
    }
    length_by_shape = {
        "straight_line": summarize([r["tactic_lines"] for r in straight]),
        "loop": summarize([r["tactic_lines"] for r in loops]),
    }

    families = Counter()
    fam_by_arch = defaultdict(Counter)
    unclassified = 0
    for r in straight:
        fam = family_of(r["name"], r["file"], r["arch"])
        if fam is None:
            unclassified += 1
        else:
            families[fam] += 1
            fam_by_arch[fam][r["arch"]] += 1

    long_tail = [r for r in fc if r["tactic_lines"] >= 300]
    loops_long = sum(1 for r in long_tail if r["loop"])

    report = {
        "repo": os.path.abspath(repo),
        "commit": commit,
        "direct": {
            "fc_total": len(fc),
            "fc_by_arch": dict(sorted(direct.items())),
            "fc_unique_goal": len(unique_goals),
            "fc_duplicate_goal_extra": len(fc) - len(unique_goals),
            "straight_line": len(straight),
            "loop": len(loops),
            "straight_line_share_pct": round(100.0 * len(straight) / len(fc), 2) if fc else None,
            "loop_share_pct": round(100.0 * len(loops) / len(fc), 2) if fc else None,
            "equiv_named": len(equiv_names),
            "subroutine_safe_named": len(safe_names),
            "ensures2_goal": len(ensures2_goals),
            "reason_counts": dict(Counter(r["reason"] for r in dropped).most_common()),
            "named_correct_decls": sum(1 for r in rows if "_CORRECT" in r["name"]),
            "subroutine_correct_decls": sum(1 for r in rows if "SUBROUTINE_CORRECT" in r["name"]),
            "length_tactic": length_stats["tactic"],
            "length_span": length_stats["span"],
            "length_by_arch": arch_lengths,
            "shape_by_arch": shape_by_arch,
            "length_by_shape": length_by_shape,
            "long_tail_300": {
                "n": len(long_tail),
                "loop": loops_long,
                "straight_line": len(long_tail) - loops_long,
                "loop_share_pct": round(100.0 * loops_long / len(long_tail), 2) if long_tail else None,
            },
            "families": {
                "classified": sum(families.values()),
                "unclassified": unclassified,
                "by_family": dict(families.most_common()),
                "by_family_arch": {k: dict(v) for k, v in fam_by_arch.items()},
            },
        },
        "dropped_named": Counter(r["reason"] for r in dropped if "_CORRECT" in r["name"] or "SUBROUTINE" in r["name"]),
        "all_files_scanned": len({r["file"] for r in rows}),
        "all_declarations": len(rows),
    }

    if corpus_path:
        report["corpus"] = compare_corpus(corpus_path, fc, report)
    return report


def compare_corpus(path, fc, report):
    with open(path, encoding="utf-8") as fh:
        problems = json.load(fh)
    cats = Counter(v["category"] for v in problems.values())
    corpus_fc = {
        (arch_of_category(v["category"]), arch_relative(v["json"]["filename"]),
         v["legacy_id"].split(".")[-1])
        for v in problems.values()
        if v["category"].startswith("functional")
    }
    corpus_fc_arch = Counter()
    for v in problems.values():
        if v["category"] == "functional_correctness_arm":
            corpus_fc_arch["arm"] += 1
        elif v["category"] == "functional_correctness_x86":
            corpus_fc_arch["x86"] += 1

    mine = {(r["arch"], arch_relative(r["file"]), r["name"]) for r in fc}
    only_mine = sorted(mine - corpus_fc)
    only_corpus = sorted(corpus_fc - mine)
    return {
        "path": os.path.abspath(path),
        "corpus_total_problems": len(problems),
        "corpus_categories": dict(sorted(cats.items())),
        "corpus_fc_total": sum(corpus_fc_arch.values()),
        "corpus_fc_by_arch": dict(sorted(corpus_fc_arch.items())),
        "direct_fc_total": report["direct"]["fc_total"],
        "direct_fc_by_arch": report["direct"]["fc_by_arch"],
        "matched": len(mine & corpus_fc),
        "only_direct": len(only_mine),
        "only_corpus": len(only_corpus),
        "only_direct_examples": only_mine[:20],
        "only_corpus_examples": only_corpus[:20],
    }


def arch_of_category(category):
    return category.rsplit("_", 1)[-1]


def arch_relative(path):
    """Path below the architecture directory, e.g. 'arm/proofs/x.ml' -> 'proofs/x.ml'."""
    parts = path.replace("\\", "/").split("/")
    for i, part in enumerate(parts):
        if part in ("arm", "x86") and i + 1 < len(parts):
            return "/".join(parts[i + 1 :])
    return os.path.basename(path)


# ---------------------------------------------------------------------------
# Output
# ---------------------------------------------------------------------------

def print_report(rep):
    d = rep["direct"]
    print(f"s2n-bignum commit: {rep['commit']}")
    print(f"checkout:          {rep['repo']}")
    print(f"files scanned:     {rep['all_files_scanned']}")
    print(f"top-level decls:   {rep['all_declarations']}")
    print()
    print("== 1. FC theorem count (bench rule) ==")
    print(f"  total FC: {d['fc_total']}")
    for arch, n in d["fc_by_arch"].items():
        print(f"    {arch:4s} {n}")
    print(f"  unique goals: {d['fc_unique_goal']}  "
          f"(declarations sharing a goal: {d['fc_duplicate_goal_extra']})")
    print()
    print("== 2. shape ==")
    print(f"  straight-line: {d['straight_line']} ({d['straight_line_share_pct']}%)")
    print(f"  loop:          {d['loop']} ({d['loop_share_pct']}%)")
    print(f"  equivalence (name has _EQUIV): {d['equiv_named']}")
    print(f"  SUBROUTINE_SAFE names:          {d['subroutine_safe_named']}")
    print(f"  goals mentioning ensures2:     {d['ensures2_goal']}")
    lt = d["long_tail_300"]
    print(f"  proofs >= 300 lines: {lt['n']}  loop {lt['loop']} / straight {lt['straight_line']}"
          + (f"  (loop share {lt['loop_share_pct']}%)" if lt["n"] else ""))
    print()
    print("== 3. proof length (non-blank, non-comment tactic lines) ==")
    for label, s in (("tactic-only", d["length_tactic"]), ("whole prove span", d["length_span"])):
        if s["n"]:
            print(f"  {label:16s} n={s['n']} median={s['median']} q1={s['q1']} q3={s['q3']} "
                  f"min={s['min']} max={s['max']}")
            print("    bands: " + ", ".join(f"{k}:{v}" for k, v in s["bands"].items()))
    for arch, s in d["length_by_arch"].items():
        if s["n"]:
            print(f"  {arch} tactic-only  n={s['n']} median={s['median']} q1={s['q1']} q3={s['q3']}")
    print("  by shape (tactic-only):")
    for shape, s in d["length_by_shape"].items():
        if s["n"]:
            print(f"    {shape:12s} n={s['n']} median={s['median']} q1={s['q1']} q3={s['q3']}")
            print("      bands: " + ", ".join(f"{k}:{v}" for k, v in s["bands"].items()))
    print("  shape by arch: " + ", ".join(
        f"{a} straight {v['straight_line']} / loop {v['loop']}"
        for a, v in d["shape_by_arch"].items()))
    print()
    print("== 4. families (straight-line FC only) ==")
    fam = d["families"]
    print(f"  classified: {fam['classified']}   unclassified: {fam['unclassified']}")
    for name, n in fam["by_family"].items():
        a = fam["by_family_arch"].get(name, {})
        print(f"    {name:10s} {n:4d}  (arm {a.get('arm', 0)}, x86 {a.get('x86', 0)})")
    listed = ["mod", "montmul", "montsqr", "add", "sub", "neg", "double",
              "half", "triple", "tomont", "demont"]
    sub = sum(fam["by_family"].get(k, 0) for k in listed)
    print(f"  subtotal over the vocabulary the brief lists ({', '.join(listed)}): {sub}")
    print()
    named = rep["dropped_named"]
    if named:
        print("== named theorems dropped by the rule ==")
        for reason, n in named.most_common():
            print(f"  {n:4d}  {reason}")
        print()
    print("== drop decomposition (all top-level declarations) ==")
    for reason, n in d.get("reason_counts", {}).items():
        print(f"  {n:5d}  {reason}")
    print(f"  {d.get('named_correct_decls', 0):5d}  declarations whose name contains _CORRECT")
    print(f"  {d.get('subroutine_correct_decls', 0):5d}  ... of which SUBROUTINE_CORRECT")
    print()
    if "corpus" in rep:
        c = rep["corpus"]
        print("== 5. cross-check against collected corpus ==")
        print(f"  corpus: {c['path']}")
        print(f"  problems total: {c['corpus_total_problems']}  categories: {c['corpus_categories']}")
        print(f"  corpus FC: {c['corpus_fc_total']} {c['corpus_fc_by_arch']}")
        print(f"  direct FC: {c['direct_fc_total']} {c['direct_fc_by_arch']}")
        print(f"  matched (file, theorem): {c['matched']}")
        print(f"  only direct: {c['only_direct']}   only corpus: {c['only_corpus']}")
        if c["only_direct_examples"]:
            print("    only-direct examples:")
            for f, n in c["only_direct_examples"][:10]:
                print(f"      {f}: {n}")
        if c["only_corpus_examples"]:
            print("    only-corpus examples:")
            for f, n in c["only_corpus_examples"][:10]:
                print(f"      {f}: {n}")


def jsonable(obj):
    if isinstance(obj, Counter):
        return dict(obj)
    raise TypeError(type(obj))


def main(argv=None):
    ap = argparse.ArgumentParser(
        description="Recount s2n-bignum FC theorems with the benchmark's rule.",
    )
    ap.add_argument("checkout", help="path to a pinned s2n-bignum checkout")
    ap.add_argument("--corpus", default=None,
                    help="path to the collected problems.json for cross-checking")
    ap.add_argument("--json", action="store_true", help="emit a machine-readable table")
    args = ap.parse_args(argv)

    rep = build_report(args.checkout, args.corpus)
    if args.json:
        print(json.dumps(rep, indent=2, sort_keys=True, default=jsonable))
    else:
        print_report(rep)


if __name__ == "__main__":
    main()
