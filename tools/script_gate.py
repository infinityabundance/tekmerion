#!/usr/bin/env python3
"""Closed-grammar gate for emitted proof scripts (mentorship finding F1).

HOL Light's guarantee is enforced by OCaml's type system, and a generated proof
script is arbitrary OCaml. This gate parses a candidate with the **real OCaml
front end**, through HOL Light's own quotation preprocessor, and refuses:

  * `Obj`, `Marshal`, `Unix`, `Sys.command` (denylist profile);
  * any `external` declaration;
  * any binding or parameter named after a kernel/judge primitive.

The strict profile additionally refuses `Sys`, references (`ref`, `:=`) and the
file-I/O primitives — the complete target grammar. The current inlined answers do
not pass it; that is measured, not hidden.

Two modes:
  * authoritative  — `--hol-light DIR` (default `.bench/hol-light` if present):
    `ocamlc -pp "<hol.sh -pp>" -stop-after parsing -dparsetree`.
  * token fallback — no HOL available: a regex denylist over the text. Weaker
    (comments and strings can hide matches); the report says which mode ran.

Exit: 0 allowed, 1 refused by policy, 2 parse error, 3 tool unavailable.
"""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

KERNEL_NAMES = {
    "prove", "axioms", "check_axioms", "new_axiom", "new_definition",
    "mk_thm", "mk_axiom", "UNDISCH", "TAUT", "GEN", "INST", "EQ_MP",
}

DENY_PREFIX = ("Obj", "Marshal", "Unix")
DENY_EXACT = {"Sys.command"}
STRICT_PREFIX = ("Sys",)
STRICT_EXACT = {"ref", ":=", "open_in", "open_out", "input_line",
                "output_string", "really_input", "seek_in", "close_in",
                "close_out", "open_in_bin", "open_out_bin"}

IDENT = re.compile(r'Pexp_ident "([^"]+)"')
PATVAR = re.compile(r'Ppat_var "([^"]+)"')


class GateError(Exception):
    pass


def hol_env(hol: Path) -> dict:
    env = dict(os.environ)
    env["PATH"] = str(hol / "_opam" / "bin") + ":" + env.get("PATH", "")
    lib = hol / "_opam" / "lib"
    env["CAML_LD_LIBRARY_PATH"] = ":".join([
        str(lib / "stublibs"), str(lib / "ocaml" / "stublibs"), str(lib / "ocaml"),
    ])
    return env


def parsetree(text: str, as_expr: bool, hol: Path | None) -> str:
    with tempfile.TemporaryDirectory() as td:
        src = Path(td) / "candidate.ml"
        src.write_text(("let _tek_candidate = (\n" + text + "\n);;\n")
                       if as_expr else text, encoding="utf-8")
        cmd = ["ocamlc"]
        env = dict(os.environ)
        if hol is not None and (hol / "hol.sh").is_file():
            pp = subprocess.run([str(hol / "hol.sh"), "-pp"],
                                capture_output=True, text=True).stdout.strip()
            if not pp:
                raise GateError("hol.sh -pp produced nothing")
            cmd += ["-pp", pp]
            env = hol_env(hol)
        cmd += ["-stop-after", "parsing", "-dparsetree", "-impl", str(src)]
        proc = subprocess.run(cmd, capture_output=True, text=True, env=env)
        out = proc.stdout + proc.stderr
        if proc.returncode != 0:
            raise GateError("parse failed:\n" + out[-2000:])
        return out


def scan_tree(tree: str, strict: bool) -> list[str]:
    bad: list[str] = []
    for ident in IDENT.findall(tree):
        head = ident.split(".")[0]
        if head in DENY_PREFIX or ident in DENY_EXACT:
            bad.append(f"forbidden identifier {ident}")
        if strict and (head in STRICT_PREFIX or ident in STRICT_EXACT):
            bad.append(f"strict-profile identifier {ident}")
    for name in PATVAR.findall(tree):
        if name in KERNEL_NAMES:
            bad.append(f"rebinding of kernel name {name}")
    if "Pstr_primitive" in tree:
        bad.append("external declaration")
    return sorted(set(bad))


def scan_tokens(text: str, strict: bool) -> list[str]:
    bad: list[str] = []
    for p in DENY_PREFIX:
        if re.search(rf"\b{p}\b", text):
            bad.append(f"forbidden token {p}")
    for t in DENY_EXACT:
        if t in text:
            bad.append(f"forbidden token {t}")
    if re.search(r"^\s*external\b", text, re.M):
        bad.append("external declaration")
    for name in KERNEL_NAMES:
        if re.search(rf"let\s+(rec\s+)?{re.escape(name)}\b", text):
            bad.append(f"rebinding of kernel name {name}")
        if re.search(rf"\(\s*{re.escape(name)}\s*:", text):
            bad.append(f"rebinding of kernel name {name} (parameter)")
    if strict:
        for p in STRICT_PREFIX:
            if re.search(rf"\b{p}\.", text):
                bad.append(f"strict-profile token {p}")
        for t in STRICT_EXACT:
            if t in text:
                bad.append(f"strict-profile token {t}")
    return sorted(set(bad))


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("path")
    ap.add_argument("--expr", action="store_true")
    ap.add_argument("--profile", choices=["denylist", "strict"], default="denylist")
    ap.add_argument("--hol-light", default=".bench/hol-light")
    ap.add_argument("--tokens-only", action="store_true")
    args = ap.parse_args(argv)

    try:
        text = Path(args.path).read_text(encoding="utf-8")
    except OSError as exc:
        print(f"ERROR {exc}", file=sys.stderr)
        return 3
    strict = args.profile == "strict"

    hol = Path(args.hol_light)
    if not args.tokens_only and (hol / "hol.sh").is_file():
        try:
            bad = scan_tree(parsetree(text, args.expr, hol), strict)
            mode = "front-end"
        except FileNotFoundError:
            bad, mode = scan_tokens(text, strict), "tokens(no ocamlc)"
        except GateError as exc:
            print(f"REFUSED parse  {args.path}")
            print(exc)
            return 2
    else:
        bad, mode = scan_tokens(text, strict), "tokens(no HOL)"

    if bad:
        print(f"REFUSED {args.profile} [{mode}]  {args.path}")
        for b in bad[:20]:
            print(f"  {b}")
        if len(bad) > 20:
            print(f"  ... {len(bad) - 20} more")
        return 1
    print(f"ALLOWED {args.profile} [{mode}]  {args.path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
