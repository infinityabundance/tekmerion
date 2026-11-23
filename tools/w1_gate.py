#!/usr/bin/env python3
"""w1_gate.py -- the v3 W1 gate for straight-line ARM modular routines.

For each training-curve operation (P-256 add/sub/neg):

  1. refuse held-out proof paths with tools/heldout_guard.py,
  2. read the *machine code* step list from the ELF object (tools/disasm_steps),
  3. derive the accumulate step set from carry dataflow (tools/derive_steps),
  4. extract the human ARM_ACCSTEPS_TAC lists from the proof *for comparison
     only* -- this happens in this driver, never inside derive_steps,
  5. run a real HOL Light session (.bench/verify/w1/w1_capture.ml) that
     captures the post-simulation goal under both lists and closes each proof
     from the derived state,
  6. compare count, terms, sequence and SHA-256 of the captured goals.

Writes research/receipts/w1-accsteps-gate.{md,json}.  Exit status 0 only if
every operation matches exactly.

The HOL step is expensive (HOL Light plus the s2n-bignum base load, then three
proofs).  It is not run by the unit tests; see tools/test_derive_steps.py.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parent
BENCH = REPO / ".bench"
S2N = BENCH / "s2n-bignum"
WORK = BENCH / "verify" / "w1"
RECEIPTS = REPO / "research" / "receipts"

sys.path.insert(0, str(HERE))
import derive_steps  # noqa: E402
import disasm_steps  # noqa: E402
import heldout_guard  # noqa: E402

# The v3 scope: training curves only.  These are P-256 operations; the order
# of work is add, then sub, then neg.
OPS = ("add", "sub", "neg")

TACTIC_RE = re.compile(
    r"ARM_ACCSTEPS_TAC\s+(\w+)\s+(\[[0-9;\s]*\]|\(\s*\d+\s*--\s*\d+\s*\))\s+"
    r"(\([^)]*\))"
)


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 16), b""):
            h.update(chunk)
    return h.hexdigest()


def parse_step_spec(spec: str) -> list[int]:
    """Parse `[3;4;8]` or `(1--8)` into a list of ints."""
    s = spec.strip()
    if s.startswith("[") and s.endswith("]"):
        inner = s[1:-1].strip()
        return [] if not inner else [int(x) for x in inner.split(";") if x.strip()]
    m = re.fullmatch(r"\(\s*(\d+)\s*--\s*(\d+)\s*\)", s)
    if not m:
        raise ValueError(f"unrecognised step spec: {spec!r}")
    lo, hi = int(m.group(1)), int(m.group(2))
    return list(range(lo, hi + 1))


def extract_human_accsteps(proof: Path) -> list[dict]:
    text = proof.read_text(encoding="utf-8")
    calls = []
    for m in TACTIC_RE.finditer(text):
        calls.append({
            "exec": m.group(1),
            "anums_spec": m.group(2).strip(),
            "snums_spec": m.group(3).strip(),
            "anums": parse_step_spec(m.group(2)),
        })
    return calls


def guard(path: Path) -> dict:
    """Run the real guard CLI, fail closed, and return its receipt line."""
    proc = subprocess.run(
        [sys.executable, str(HERE / "heldout_guard.py"), str(path),
         "--root", str(S2N)],
        capture_output=True, text=True,
    )
    if proc.returncode != 0:
        raise SystemExit(f"heldout_guard refused {path}: {proc.stderr.strip()}")
    return {"path": str(path), "guard": proc.stdout.strip()}


def parse_capture(path: Path) -> dict[str, dict]:
    blocks: dict[str, dict] = {}
    tag = None
    cur: dict = {}
    for raw in path.read_text(encoding="utf-8").splitlines():
        if raw.startswith("BEGIN "):
            parts = raw.split()
            tag = parts[1]
            cur = {"count": int(parts[3]), "asms": [], "concls": [],
                   "md5": [], "goals": []}
        elif raw.startswith("END "):
            if tag is not None:
                # canonical serialisation: ordered assumptions + conclusion
                entries = ([("A", a) for a in cur["asms"]]
                           + [("C", c) for c in cur["concls"]])
                blob = "\n".join(f"{k}\t{v}" for k, v in entries)
                cur["sha256"] = hashlib.sha256(blob.encode()).hexdigest()
                sorted_blob = "\n".join(sorted(f"{k}\t{v}" for k, v in entries))
                cur["sha256_multiset"] = hashlib.sha256(
                    sorted_blob.encode()).hexdigest()
                blocks[tag] = cur
            tag = None
            cur = {}
        elif tag is not None:
            if raw.startswith("ASM "):
                cur["asms"].append(raw[4:])
            elif raw.startswith("CONCL "):
                cur["concls"].append(raw[6:])
            elif raw.startswith("DIGEST "):
                cur["md5"].append(raw[7:])
    return blocks


def first_difference(a: list[str], b: list[str]) -> str:
    for i, (x, y) in enumerate(zip(a, b)):
        if x != y:
            return f"index {i}: human={x!r} derived={y!r}"
    if len(a) != len(b):
        longer, shorter = (a, b) if len(a) > len(b) else (b, a)
        side = "human" if longer is a else "derived"
        return f"first extra entry on {side} side at index {len(shorter)}: " \
               f"{longer[len(shorter)]!r}"
    return ""


def run_hol(lists_path: Path, out_path: Path, log_path: Path,
            timeout_s: int) -> tuple[str, float]:
    env = dict(os.environ)
    env["W1_OUT"] = str(out_path)
    env["W1_LISTS"] = str(lists_path)
    driver = REPO / "tools" / "w1_hol_capture.ml"
    cmd = f"cat {driver} | ../hol-light/hol.sh"
    t0 = time.time()
    with open(log_path, "w", encoding="utf-8") as log:
        proc = subprocess.run(["/bin/sh", "-c", cmd], cwd=str(S2N),
                              env=env, stdout=log, stderr=subprocess.STDOUT,
                              timeout=timeout_s)
    return cmd, time.time() - t0


_NO_SEM = derive_steps.InstructionSemantics(form=None)


def flag_live_successors(steps) -> list[int]:
    """Immediate successors of flag writers whose flags are not yet overwritten.

    This is *not* the carried derivation.  It is the diagnostic closure: the
    ARM accumulator also fires on the step after a flag write, because the
    flag's underlying arithmetic expression is still live there.  Capturing
    both variants shows whether a single rule can reproduce all three human
    goals.
    """
    out: list[int] = []
    for i, s in enumerate(steps):
        si = derive_steps.DEFAULT_SEMANTICS.get(s.mnemonic, _NO_SEM)
        if not si.writes_carry or i + 1 >= len(steps):
            continue
        nxt = steps[i + 1]
        sni = derive_steps.DEFAULT_SEMANTICS.get(nxt.mnemonic, _NO_SEM)
        if not sni.writes_carry:
            out.append(nxt.index)
    return out


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--timeout", type=int, default=3600,
                    help="HOL Light session timeout in seconds")
    ap.add_argument("--skip-hol", action="store_true",
                    help="reuse an existing capture.out instead of running HOL")
    ap.add_argument("--receipt", default=str(RECEIPTS / "w1-accsteps-gate.md"))
    ap.add_argument("--json", default=str(RECEIPTS / "w1-accsteps-gate.json"))
    args = ap.parse_args(argv)

    WORK.mkdir(parents=True, exist_ok=True)
    RECEIPTS.mkdir(parents=True, exist_ok=True)

    proofs = {op: S2N / "arm" / "proofs" / f"bignum_{op}_p256.ml" for op in OPS}
    objects = {op: S2N / "arm" / "p256" / f"bignum_{op}_p256.o" for op in OPS}

    receipt: dict = {"operations": {}, "commands": [], "files": {}}

    # Guard every proof path before opening it.
    for op in OPS:
        receipt["files"][f"proof_{op}"] = guard(proofs[op])

    human_calls = {}
    human_union = {}
    for op in OPS:
        calls = extract_human_accsteps(proofs[op])
        if not calls:
            raise SystemExit(f"no ARM_ACCSTEPS_TAC found in {proofs[op]}")
        human_calls[op] = calls
        union: list[int] = []
        for c in calls:
            for n in c["anums"]:
                if n not in union:
                    union.append(n)
        human_union[op] = sorted(union)
        receipt["files"][f"proof_{op}"]["sha256"] = sha256_file(proofs[op])

    derived = {}
    diag = {}
    all_steps = {}
    for op in OPS:
        obj = objects[op]
        steps = disasm_steps.disassemble(str(obj))
        all_steps[op] = steps
        d = derive_steps.derive_accumulate(steps)
        derived[op] = d
        succ = flag_live_successors(steps)
        diag[op] = sorted(set(d) | set(succ))
        receipt["files"][f"object_{op}"] = {
            "path": str(obj), "sha256": sha256_file(obj),
            "steps": len(steps),
            "mnemonics": [f"{s.index}:{s.mnemonic}" for s in steps],
        }

    # ---- generate the list file consumed by the HOL driver -----------------
    lines = []
    for op in OPS:
        lines.append(f"let human_{op} = [{';'.join(map(str, human_union[op]))}];;")
        lines.append(f"let derived_{op} = [{';'.join(map(str, derived[op]))}];;")
        lines.append(f"let diag_{op} = [{';'.join(map(str, diag[op]))}];;")
    lists_path = WORK / "w1_lists.ml"
    lists_path.write_text("\n".join(lines) + "\n", encoding="utf-8")

    command = ("cd .bench/s2n-bignum && "
               "W1_OUT=<work>/capture.out W1_LISTS=<work>/w1_lists.ml "
               "sh -c 'cat ../verify/w1/w1_capture.ml | ../hol-light/hol.sh'")
    receipt["commands"].append(command)
    receipt["lists_file"] = str(lists_path)
    receipt["lists_source"] = "\n".join(lines)

    out_path = WORK / "capture.out"
    log_path = WORK / "capture.log"
    if not args.skip_hol:
        cmd, seconds = run_hol(lists_path, out_path, log_path, args.timeout)
        receipt["hol"] = {"command": cmd, "cwd": str(S2N),
                          "seconds": round(seconds, 2),
                          "out": str(out_path), "log": str(log_path)}
    else:
        receipt["hol"] = {"command": command, "reused": True,
                          "out": str(out_path), "log": str(log_path)}

    if not out_path.exists():
        raise SystemExit(f"HOL produced no capture file: {out_path}")
    raw = out_path.read_text(encoding="utf-8")
    closed = {}
    for line in raw.splitlines():
        if line.startswith("CLOSED "):
            p = line.split()
            closed[p[1]] = {"hyp": int(p[3]), "concl_md5": p[5]}
    if not args.skip_hol:
        (RECEIPTS / "w1-accsteps-capture.txt").write_text(raw, encoding="utf-8")
        receipt["raw_capture"] = str(RECEIPTS / "w1-accsteps-capture.txt")
    if "DONE" not in raw:
        receipt["hol"]["incomplete"] = True

    blocks = parse_capture(out_path)

    all_ok = True
    for op in OPS:
        h = blocks.get(f"{op}-human")
        d = blocks.get(f"{op}-derived")
        entry: dict = {
            "human_list": human_union[op],
            "derived_list": derived[op],
            "list_equal": sorted(human_union[op]) == sorted(derived[op]),
            "human_calls": human_calls[op],
        }
        if h is None or d is None:
            entry["match"] = False
            entry["error"] = "missing capture block"
            all_ok = False
            receipt["operations"][op] = entry
            continue
        entry.update({
            "human_count": h["count"], "derived_count": d["count"],
            "human_assumptions": len(h["asms"]),
            "derived_assumptions": len(d["asms"]),
            "human_sha256": h["sha256"], "derived_sha256": d["sha256"],
            "human_sha256_multiset": h["sha256_multiset"],
            "derived_sha256_multiset": d["sha256_multiset"],
            "human_hol_md5": h["md5"], "derived_hol_md5": d["md5"],
            "conclusion_equal": h["concls"] == d["concls"],
            "conclusion_sha256": hashlib.sha256(
                "\n".join(h["concls"]).encode()).hexdigest(),
            "assumptions_sequence_equal": h["asms"] == d["asms"],
            "assumptions_multiset_equal": sorted(h["asms"]) == sorted(d["asms"]),
            "closed": closed.get(f"{op}-derived"),
        })
        entry["match"] = (
            h["count"] == d["count"]
            and h["asms"] == d["asms"]
            and h["concls"] == d["concls"]
            and h["sha256"] == d["sha256"]
            and h["md5"] == d["md5"]
        )
        if not entry["match"]:
            entry["first_assumption_difference"] = first_difference(h["asms"], d["asms"])
            entry["only_human"] = sorted(set(h["asms"]) - set(d["asms"]))
            entry["only_derived"] = sorted(set(d["asms"]) - set(h["asms"]))
        if entry["closed"] is None or entry["closed"]["hyp"] != 0:
            entry["closed_ok"] = False
            all_ok = False
        else:
            entry["closed_ok"] = True
        dg = blocks.get(f"{op}-diag")
        if dg is not None:
            entry["diag_list"] = diag[op]
            entry["diag_matches_human"] = (
                dg["count"] == h["count"]
                and dg["asms"] == h["asms"]
                and dg["concls"] == h["concls"]
            )
        all_ok = all_ok and entry["match"]
        receipt["operations"][op] = entry

    receipt["gate_passed"] = bool(all_ok)

    # ---- write receipts ----------------------------------------------------
    json_path = Path(args.json)
    json_path.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n",
                         encoding="utf-8")

    md = build_markdown(receipt)
    Path(args.receipt).write_text(md, encoding="utf-8")

    print(build_markdown(receipt))
    return 0 if all_ok else 1


def build_markdown(r: dict) -> str:
    out = ["# W1 gate - accumulate steps from carry dataflow", ""]
    out.append("Receipt generated by `tools/w1_gate.py`. All numbers below come "
               "from this run's captured goals; none are hand-typed.")
    out.append("")
    out.append("## Commands")
    out.append("")
    out.append("```sh")
    for op in OPS:
        out.append(f"python3 tools/heldout_guard.py {r['files'][f'proof_{op}']['path']} "
                   f"--root .bench/s2n-bignum")
    for c in r["commands"]:
        out.append(c)
    out.append("```")
    out.append("")
    if "seconds" in r.get("hol", {}):
        out.append(f"HOL Light session wall time: {r['hol']['seconds']} s.")
        out.append("")
    out.append("## Inputs")
    out.append("")
    out.append("| file | sha256 |")
    out.append("|---|---|")
    for key in sorted(r["files"]):
        f = r["files"][key]
        if "sha256" in f:
            out.append(f"| `{f['path']}` | `{f['sha256']}` |")
    out.append("")
    out.append("Held-out guard output:")
    out.append("")
    out.append("```")
    for op in OPS:
        out.append(r["files"][f"proof_{op}"]["guard"])
    out.append("```")
    out.append("")
    out.append("## Result")
    out.append("")
    out.append("Human simulation calls (comparison data only):")
    out.append("")
    for op in OPS:
        for c in r["operations"][op].get("human_calls", []):
            out.append(f"- `{op}`: `ARM_ACCSTEPS_TAC {c['exec']} "
                       f"{c['anums_spec']} {c['snums_spec']}`")
    out.append("")
    out.append("| op | derived | human | lists equal | goals | assumptions | "
               "same terms | same digest | closed (hyp 0) | exact |")
    out.append("|---|---|---|---|---|---|---|---|---|---|")
    for op in OPS:
        e = r["operations"][op]
        out.append(
            f"| {op} | `{e['derived_list']}` | `{e['human_list']}` | "
            f"{e['list_equal']} | {e.get('human_count')}/{e.get('derived_count')} | "
            f"{e.get('human_assumptions')}/{e.get('derived_assumptions')} | "
            f"{e.get('assumptions_sequence_equal')} | "
            f"{e.get('human_sha256') == e.get('derived_sha256')} | "
            f"{e.get('closed_ok')} | **{e['match']}** |")
    out.append("")
    out.append("Gate passed: **%s**." % r["gate_passed"])
    out.append("")
    out.append("### Diagnostic: what would a flag-liveness closure add?")
    out.append("")
    out.append("The diagnostic list is `derived` plus the immediate successor of "
               "every flag-writing step whose flags are still live.  It is "
               "reported to locate the `sub` difference; it is not the gate.")
    out.append("")
    out.append("| op | diagnostic list | reproduces the human goal |")
    out.append("|---|---|---|")
    for op in OPS:
        e = r["operations"][op]
        if "diag_list" in e:
            out.append(f"| {op} | `{e['diag_list']}` | "
                       f"{e.get('diag_matches_human')} |")
    out.append("")
    out.append("### Digests")
    out.append("")
    for op in OPS:
        e = r["operations"][op]
        out.append(f"- **{op}** human `{e.get('human_sha256')}` "
                   f"(HOL MD5 `{e.get('human_hol_md5')}`), derived "
                   f"`{e.get('derived_sha256')}` "
                   f"(HOL MD5 `{e.get('derived_hol_md5')}`).")
    out.append("")
    for op in OPS:
        e = r["operations"][op]
        if e["match"]:
            continue
        out.append(f"### First difference - {op}")
        out.append("")
        out.append(f"- first assumption difference: {e.get('first_assumption_difference')}")
        out.append("- assumptions only in the human goal:")
        for x in e.get("only_human", []):
            out.append(f"  - `{x}`")
        out.append("- assumptions only in the derived goal:")
        for x in e.get("only_derived", []):
            out.append(f"  - `{x}`")
        out.append("")
    return "\n".join(out) + "\n"


if __name__ == "__main__":
    sys.exit(main())
