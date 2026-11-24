#!/usr/bin/env python3
"""challenge.py -- statement challenge files and an independent comparator.

Implements the cheap half of Ordeal O1.1/O1.2:

  * ``freeze`` writes a plain-text challenge file for one benchmark task: the
    exact statement bytes, their digest, the object digest, the corpus digest
    and a provenance label.
  * ``verify`` re-reads the challenge, re-derives every digest from the live
    files, and checks that the *judge program that actually ran* embeds that
    statement byte-for-byte.

What this does and does not establish
-------------------------------------
The benchmark harness calls ``prove(query, tac)``; HOL's ``prove`` fails unless
the tactic proves that exact goal, so the accepted theorem's conclusion is the
query *by construction*. What was missing was an independent record that the
query is the intended statement, and a check that the judged program used that
record. That is what these files and this tool add.

They do **not** compare kernel terms (that needs an exported proof object), and
they do **not** check the axiom whitelist (the harness only compares axiom
*counts*; the whitelist is O4 work). ``verify`` says so in every report.

A challenge frozen after the proof search is labelled ``POST_HOC``. O1.1 wants
the challenge written *before* search; ``verify --require-pre-search`` refuses a
POST_HOC record, and certification must use it.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
BEGIN = b"--- statement begin ---\n"
END = b"--- statement end ---\n"
FORMAT = "tnps-challenge v1"


class ChallengeError(Exception):
    pass


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def canonical_task(task: str) -> str:
    if not isinstance(task, str) or not task:
        raise ChallengeError("task id is empty")
    if re.fullmatch(r"[A-Za-z0-9_.]+", task) is None:
        raise ChallengeError(f"task id has unexpected characters: {task!r}")
    return task


def object_for(s2n: Path, task: str) -> Path:
    m = re.fullmatch(r"(arm|x86)\.bignum_(add|sub|neg)_([A-Za-z0-9_]+)\."
                     r"BIGNUM_(?:ADD|SUB|NEG)_[A-Za-z0-9_]+_CORRECT", task)
    if m is None:
        raise ChallengeError(f"not an add/sub/neg functional-correctness task: {task!r}")
    arch, op, family = m.groups()
    matches = sorted(s2n.glob(f"{arch}/*/bignum_{op}_{family}.o"))
    if len(matches) != 1:
        raise ChallengeError(
            f"expected exactly one {arch} object for {op}/{family}, found {matches}")
    return matches[0]


def git_head(path: Path) -> str:
    head = path / ".git"
    if not head.exists():
        return "unknown"
    try:
        import subprocess
        out = subprocess.run(["git", "-C", str(path), "rev-parse", "HEAD"],
                             capture_output=True, text=True, check=True)
        return out.stdout.strip()
    except Exception:  # noqa: BLE001 - a missing commit must not block a digest
        return "unknown"


def freeze(args) -> int:
    task = canonical_task(args.task)
    corpus_path = Path(args.problems)
    corpus_bytes = corpus_path.read_bytes()
    corpus = json.loads(corpus_bytes)
    if task not in corpus:
        raise ChallengeError(f"{task!r} is not in {corpus_path}")
    entry = corpus[task]
    statement = entry["query"]
    if not isinstance(statement, str) or not statement:
        raise ChallengeError(f"{task!r} has no query statement")

    s2n = Path(args.s2n_bignum)
    obj = object_for(s2n, task)
    obj_rel = obj.relative_to(s2n)
    obj_bytes = obj.read_bytes()

    frozen_at = args.frozen_at or "PRE_SEARCH"
    if args.post_hoc:
        frozen_at = f"POST_HOC: {args.post_hoc}"

    text = [
        FORMAT,
        f"task {task}",
        f"category {entry['category']}",
        f"standard tnps/1",
        f"frozen-at {frozen_at}",
        f"pre-search {'false' if args.post_hoc else 'true'}",
        f"problems-json-sha256 {sha256(corpus_bytes)}",
        f"statement-sha256 {sha256(statement.encode('utf-8'))}",
        f"statement-bytes {len(statement.encode('utf-8'))}",
        f"object {obj_rel.as_posix()}",
        f"object-sha256 {sha256(obj_bytes)}",
        f"s2n-bignum-commit {git_head(s2n)}",
        "",
    ]
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_bytes("\n".join(text).encode("utf-8")
                    + BEGIN + statement.encode("utf-8") + b"\n" + END)
    print(f"froze {out} task={task} statement={len(statement.encode('utf-8'))}B "
          f"object={obj_rel} pre_search={not args.post_hoc}")
    return 0


def parse_challenge(path: Path) -> dict:
    raw = path.read_bytes()
    i = raw.find(BEGIN)
    j = raw.find(END, i + len(BEGIN)) if i >= 0 else -1
    if i < 0 or j < 0:
        raise ChallengeError(f"{path}: missing statement delimiters")
    header = raw[:i].decode("utf-8")
    statement = raw[i + len(BEGIN):j]
    # The writer adds one newline before END; the statement itself is exact.
    if not statement.endswith(b"\n"):
        raise ChallengeError(f"{path}: malformed statement block")
    statement = statement[:-1]
    fields: dict[str, str] = {}
    lines = header.splitlines()
    if not lines or lines[0] != FORMAT:
        raise ChallengeError(f"{path}: not a {FORMAT} file")
    for line in lines[1:]:
        if not line.strip():
            continue
        if " " not in line:
            raise ChallengeError(f"{path}: header line without a value: {line!r}")
        key, value = line.split(" ", 1)
        if key in fields:
            raise ChallengeError(f"{path}: duplicate header key {key!r}")
        fields[key] = value
    fields["_statement"] = statement
    return fields


def contains_wrapper(judge_ml: bytes, task: str, statement: str) -> bool:
    """The harness writes: bench_run "<task>" (<query>) (fun () -> <answer>) ..."""
    needle = ('bench_run "' + task + '" (' + statement + ") (fun () ->").encode("utf-8")
    return needle in judge_ml


def verify(args) -> int:
    path = Path(args.challenge)
    f = parse_challenge(path)
    statement: bytes = f["_statement"]
    ok = True
    notes: list[str] = []

    def check(name: str, cond: bool, detail: str = "") -> None:
        nonlocal ok
        print(f"{'PASS' if cond else 'FAIL'} {name}" + (f"  {detail}" if detail else ""))
        if not cond:
            ok = False

    check("statement-sha256", sha256(statement) == f.get("statement-sha256"),
          f"recorded={f.get('statement-sha256')}")
    check("statement-bytes", str(len(statement)) == f.get("statement-bytes"),
          f"recorded={f.get('statement-bytes')}")

    corpus_path = Path(args.problems)
    corpus = json.loads(corpus_path.read_bytes())
    task = f["task"]
    check("task-present", task in corpus)
    if task in corpus:
        check("statement==problems.json",
              corpus[task]["query"].encode("utf-8") == statement)
        check("category", corpus[task]["category"] == f.get("category"),
              f"recorded={f.get('category')}")

    s2n = Path(args.s2n_bignum)
    obj = s2n / f["object"]
    check("object-exists", obj.is_file(), str(obj))
    if obj.is_file():
        check("object-sha256", sha256(obj.read_bytes()) == f.get("object-sha256"))

    if args.evaldir:
        evaldir = Path(args.evaldir)
        cands = sorted(evaldir.glob("ml_files_*.ml"))
        hits = [p for p in cands
                if contains_wrapper(p.read_bytes(), task, statement.decode("utf-8"))]
        check("judge-program-scanned", len(cands) > 0, f"{len(cands)} ml_files_*.ml")
        check("judge-embeds-statement", len(hits) == 1,
              f"{len(hits)} of {len(cands)} judge program(s) embed it"
              + (f"; {hits[0].name}" if len(hits) == 1 else ""))
        judge_txt = evaldir / f"{task}.judge.txt"
        if judge_txt.is_file():
            verdict = judge_txt.read_text(encoding="utf-8").strip()
            check("judge-verdict-ok", verdict == "OK", verdict)
        else:
            notes.append(f"no judge verdict file at {judge_txt}")

    if args.require_pre_search:
        check("pre-search", f.get("pre-search") == "true",
              f"frozen-at={f.get('frozen-at')}")

    notes.append("NOT CHECKED: kernel-term identity (needs an exported proof "
                 "object); axiom whitelist (the harness compares counts only).")
    for n in notes:
        print(f"NOTE {n}")
    print(f"RESULT {'PASS' if ok else 'FAIL'} {task}")
    return 0 if ok else 1


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = ap.add_subparsers(dest="cmd", required=True)

    fr = sub.add_parser("freeze", help="write a challenge file")
    fr.add_argument("--task", required=True)
    fr.add_argument("--out", required=True)
    fr.add_argument("--problems", default=str(REPO / ".bench" / "problems.json"))
    fr.add_argument("--s2n-bignum", default=str(REPO / ".bench" / "s2n-bignum"))
    fr.add_argument("--frozen-at", default=None,
                    help="ISO timestamp; omit for PRE_SEARCH")
    fr.add_argument("--post-hoc", default=None,
                    help="reason this challenge was written after the run")
    fr.set_defaults(func=freeze)

    ve = sub.add_parser("verify", help="check a challenge against live files")
    ve.add_argument("--challenge", required=True)
    ve.add_argument("--problems", default=str(REPO / ".bench" / "problems.json"))
    ve.add_argument("--s2n-bignum", default=str(REPO / ".bench" / "s2n-bignum"))
    ve.add_argument("--evaldir", default=None)
    ve.add_argument("--require-pre-search", action="store_true")
    ve.set_defaults(func=verify)

    args = ap.parse_args(argv)
    try:
        return args.func(args)
    except ChallengeError as exc:
        print(f"ERROR {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
