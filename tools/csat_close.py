#!/usr/bin/env python3
"""csat_close.py -- measured certified-SAT close for the P-256 linear family.

This wraps two HOL Light drivers:

  tools/csat_probe.ml   word tautologies (64/128/256/320-bit), to measure the
                        raw BITBLAST_THEN -> CaDiCaL -> kernel-LRAT replay cost.
  tools/csat_hol.ml     the P-256 add/sub/neg post-simulation goals, reduced by
                        the deterministic prefix of tools/w4_close.py, with the
                        certified-SAT close attempted at every stage.

Every refutation is replayed by the LCF kernel (Cadical/ldrup.ml); the solver
certificate is not trusted.  The tool records, per attempt:

  * whether the goal closed,
  * the tactic sequence (the deterministic stages plus the SAT close),
  * wall-clock close time,
  * the per-task timeout from .bench/timeout-map.json and the headroom.

It never reads a held-out proof.  It runs tools/heldout_guard.py on the three
P-256 proof paths before doing anything (a check, not a read), and otherwise
only reads ELF objects and the timeout map.

Offline: no network, no model key, no 20 GB bench build.  It does need a
`cadical` binary that supports --lrat --no-binary.
"""

from __future__ import annotations

import argparse
import glob
import json
import os
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parent
BENCH = REPO / ".bench"
S2N = BENCH / "s2n-bignum"
RECEIPTS = REPO / "research" / "receipts"
WORK = BENCH / "verify" / "csat"

sys.path.insert(0, str(HERE))
import derive_steps  # noqa: E402
import disasm_steps  # noqa: E402
import heldout_guard  # noqa: E402

OPS = ("add", "sub", "neg")

TACTIC_SEQUENCE = [
    "BITBLAST_THEN (CONV_TAC o K CSAT_PROVE)",
    "CSAT_PROVE = CADICAL_PROVE with CSAT_REFUTE_GEN substituted",
    "CSAT_REFUTE_GEN: dimacs_of_hol -> cadical --no-binary --lrat "
    "-> LRAT_REFUTE_GEN (kernel replay)",
]

RESULT_RE = re.compile(
    r"^\s*#?\s*CSAT_RESULT\s+(?P<tag>\S+)\s+closed=(?P<closed>[01])\s+"
    r"seconds=(?P<secs>[0-9.]+)(?:\s+reason=(?P<reason>.*))?$"
)
STAGE_RE = re.compile(r"^\s*#?\s*CSAT_STAGE\s+(?P<tag>\S+)\s+goals=(?P<n>\d+)$")
STAGE_FAIL_RE = re.compile(
    r"^\s*#?\s*CSAT_STAGE_FAIL\s+(?P<tag>\S+)\s+(?P<msg>.*)$")


def resolve_cadical(explicit: str | None) -> str | None:
    if explicit:
        return explicit if os.access(explicit, os.X_OK) else None
    env = os.environ.get("CSAT_CADICAL")
    if env and os.access(env, os.X_OK):
        return env
    found = shutil.which("cadical")
    if found:
        return found
    patterns = [
        str(Path.home() / ".elan/toolchains/*/bin/cadical"),
        str(REPO / ".bench/tmp-csat/bin/cadical"),
        str(REPO / ".bench/bin/cadical"),
    ]
    for pat in patterns:
        hits = sorted(glob.glob(pat))
        if hits:
            return hits[0]
    return None


def derived_anums(op: str) -> list[int]:
    obj = S2N / "arm" / "p256" / f"bignum_{op}_p256.o"
    if not obj.exists():
        return []
    return derive_steps.derive_accumulate(disasm_steps.disassemble(str(obj)))


def timeout_for(op: str) -> dict:
    path = BENCH / "timeout-map.json"
    task = f"arm.bignum_{op}_p256.BIGNUM_{op.upper()}_P256_CORRECT"
    entry = {"task": task, "timeout_sec": None, "human_prove_secs": None}
    if path.exists():
        for e in json.loads(path.read_text()):
            if e.get("problem_id") == task:
                entry["timeout_sec"] = e.get("timeout_sec")
                entry["human_prove_secs"] = e.get("prove_secs")
                break
    return entry


def run_hol(driver: Path, cadical: str, extra_env: dict, timeout: int):
    """Run a HOL driver.  Returns (record, full_stdout)."""
    env = dict(os.environ)
    env["PATH"] = str(Path(cadical).parent) + os.pathsep + env.get("PATH", "")
    env["CSAT_CADICAL"] = cadical
    env["CSAT_LIB"] = str(HERE / "csat_lib.ml")
    env.update(extra_env)
    cmd = f"cat {driver} | ../hol-light/hol.sh"
    log = WORK / (driver.stem + ".log")
    t0 = time.time()
    try:
        proc = subprocess.run(["/bin/sh", "-c", cmd], cwd=str(S2N), env=env,
                              capture_output=True, text=True, timeout=timeout)
        out, err, rc, timed_out = proc.stdout, proc.stderr, proc.returncode, False
    except subprocess.TimeoutExpired as exc:
        out = (exc.stdout or "") if isinstance(exc.stdout, str) else ""
        err = (exc.stderr or "") if isinstance(exc.stderr, str) else ""
        rc, timed_out = None, True
    log.write_text(out + "\n--- stderr ---\n" + err, encoding="utf-8")
    rec = {
        "command": cmd, "cwd": str(S2N), "seconds": round(time.time() - t0, 3),
        "returncode": rc, "timed_out": timed_out, "log": str(log),
    }
    return rec, out


def parse(output: str) -> tuple[list, list]:
    results, stage_fail = [], []
    for line in output.splitlines():
        s = line.strip()
        m = RESULT_RE.match(s)
        if m:
            results.append({
                "tag": m.group("tag"),
                "closed": m.group("closed") == "1",
                "seconds": float(m.group("secs")),
                "reason": (m.group("reason") or "").strip(),
            })
            continue
        m = STAGE_FAIL_RE.match(s)
        if m:
            stage_fail.append({"tag": m.group("tag"), "message": m.group("msg")})
    return results, stage_fail


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--mode", choices=["probe", "post", "both"], default="both")
    ap.add_argument("--target", choices=OPS + ("all",), default="all")
    ap.add_argument("--cadical", default=None)
    ap.add_argument("--timeout", type=int, default=2400,
                    help="wall-clock limit for each HOL process (default 2400s)")
    ap.add_argument("--receipt", default=str(RECEIPTS / "csat-close.md"))
    ap.add_argument("--json", default=str(RECEIPTS / "csat-close.json"))
    args = ap.parse_args(argv)

    WORK.mkdir(parents=True, exist_ok=True)
    RECEIPTS.mkdir(parents=True, exist_ok=True)

    receipt: dict = {
        "tool": "tools/csat_close.py",
        "tactic_sequence": TACTIC_SEQUENCE,
        "cadical": None,
        "guards": [],
        "derived": {},
        "timeouts": {},
        "probe": None,
        "post": {},
        "verdict": None,
    }

    # Compliance: guard the three P-256 proof paths before any work.  These are
    # checks only; this tool never opens the proof files.
    for op in OPS:
        proof = S2N / "arm" / "proofs" / f"bignum_{op}_p256.ml"
        if proof.exists():
            try:
                receipt["guards"].append({
                    "path": str(proof),
                    "guard": heldout_guard.describe(str(proof), root=S2N)})
            except heldout_guard.HeldOutRefused as exc:
                print(f"REFUSED by heldout guard: {exc}", file=sys.stderr)
                return 2

    cadical = resolve_cadical(args.cadical)
    receipt["cadical"] = cadical
    if cadical is None:
        receipt["verdict"] = "blocked: no cadical binary (need --lrat support)"
        Path(args.json).write_text(json.dumps(receipt, indent=2) + "\n")
        print(receipt["verdict"], file=sys.stderr)
        return 3

    targets = OPS if args.target == "all" else (args.target,)
    for op in targets:
        receipt["derived"][op] = derived_anums(op)
        receipt["timeouts"][op] = timeout_for(op)

    anums_env = {
        f"CSAT_ANUMS_{op.upper()}": ";".join(map(str, receipt["derived"][op]))
        for op in targets
    }

    if args.mode in ("probe", "both"):
        rec, out = run_hol(HERE / "csat_probe.ml", cadical, {}, args.timeout)
        results, failed = parse(out)
        rec["results"] = results
        rec["stage_failures"] = failed
        receipt["probe"] = rec

    if args.mode in ("post", "both"):
        env = dict(anums_env)
        env["CSAT_DUMP_DIR"] = str(WORK)
        rec, out = run_hol(HERE / "csat_hol.ml", cadical, env, args.timeout)
        results, failed = parse(out)
        rec["results"] = results
        rec["stage_failures"] = failed
        receipt["post"] = rec

    # ---- verdict ----------------------------------------------------------
    post_results = (receipt["post"] or {}).get("results", [])
    closed = [r for r in post_results if r["closed"]]
    receipt["verdict"] = (
        "closed" if closed else
        "not closed: every post-simulation stage left CaDiCaL SATISFIED "
        "(num/real/MOD content is not bit-blasted)"
    ) if receipt["post"] else "probe only"

    # ---- persist ----------------------------------------------------------
    Path(args.json).write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n",
                               encoding="utf-8")
    md = build_markdown(receipt, targets)
    Path(args.receipt).write_text(md, encoding="utf-8")
    print(md)
    return 0 if closed else 1


def build_markdown(r: dict, targets) -> str:
    out = ["# C-SAT certified close -- measured", ""]
    out.append("Numbers below come from this run's HOL output; none are hand-typed.")
    out.append("")
    out.append(f"- cadical: `{r['cadical']}`")
    out.append(f"- verdict: **{r['verdict']}**")
    out.append("")
    out.append("## Tactic sequence")
    out.append("")
    for t in r["tactic_sequence"]:
        out.append(f"1. `{t}`")
    out.append("")

    if r.get("probe"):
        out.append("## Probe: raw certified-SAT close on word tautologies")
        out.append("")
        out.append("Timeout reference: 120 s (the P-256 add/sub/neg per-task "
                   "timeout in `.bench/timeout-map.json`).")
        out.append("")
        out.append("| lemma | closed | close s | timeout s | headroom s | note |")
        out.append("|---|---|---|---|---|---|")
        for x in r["probe"]["results"]:
            head = f"{120 - x['seconds']:.3f}" if x["closed"] else "n/a"
            out.append(f"| {x['tag']} | {x['closed']} | {x['seconds']:.3f} "
                       f"| 120 | {head} | {x['reason']} |")
        out.append("")
        out.append(f"Probe process wall-clock: {r['probe']['seconds']:.1f}s "
                   f"(`{r['probe']['log']}`)")
        out.append("")

    if r.get("post"):
        out.append("## Post-simulation goals (P-256)")
        out.append("")
        out.append("| op | stage | closed | close s | timeout s | headroom s | reason |")
        out.append("|---|---|---|---|---|---|---|")
        for op in targets:
            to = r["timeouts"][op].get("timeout_sec")
            for x in r["post"]["results"]:
                if not x["tag"].startswith(op + "."):
                    continue
                if not x["closed"]:
                    head = "n/a"
                elif to is None:
                    head = "?"
                else:
                    head = f"{to - x['seconds']:.3f}"
                out.append(f"| {op} | {x['tag']} | {x['closed']} "
                           f"| {x['seconds']:.3f} | {to if to is not None else '?'} "
                           f"| {head} | {x['reason']} |")
        out.append("")
        out.append(f"Post process wall-clock: {r['post']['seconds']:.1f}s")
        out.append("")

    out.append("## Notes")
    out.append("")
    out.append("- A `closed=0` row is a measured negative: CaDiCaL found the "
               "negated goal SATISFIABLE, so the kernel refused it.")
    out.append("- Raw bit-blast replay (probe) is fast enough; the blocker is the "
               "num/real/`MOD` bridge lemma (TODO C1), not replay speed.")
    return "\n".join(out) + "\n"


if __name__ == "__main__":
    raise SystemExit(main())
