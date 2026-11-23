#!/usr/bin/env python3
"""curve_check.py -- mechanical V7.3 check over the lemma statements of bridge.ml.

Extract the first backtick-delimited term following every `prove` and report any
occurrence of a curve name, a concrete large numeral/hex constant, or a known
reduction shape.  Comments are stripped first so the header prose cannot mask a
hit.  Exit 1 if any hit is found.
"""

import re
import sys

CURVES = ("p_256", "p_384", "p_521", "nistp", "25519", "secp256k1",
          "p256k1", "sm2", "curve25519", "p25519", "p384", "p521")
REDUCTION = ("+ 19", "+19", "2 EXP 255", "2 EXP 384", "2 EXP 521",
             "0x", "0X")


def strip_comments(src: str) -> str:
    out, depth = [], 0
    i = 0
    while i < len(src):
        if src.startswith("(*", i):
            depth += 1
            i += 2
        elif src.startswith("*)", i) and depth:
            depth -= 1
            i += 2
        elif depth:
            i += 1
        else:
            out.append(src[i])
            i += 1
    return "".join(out)


def lemma_statements(src: str):
    """Yield (offset, statement-text) for each prove(...) call."""
    for m in re.finditer(r"\bprove\b", src):
        rest = src[m.end():]
        q = rest.find("`")
        if q < 0:
            continue
        e = rest.find("`", q + 1)
        if e < 0:
            continue
        yield m.start(), rest[q + 1:e]


def main(path):
    src = strip_comments(open(path, encoding="utf-8").read())
    hits = 0
    for off, stmt in lemma_statements(src):
        line = src[:off].count("\n") + 1
        flags = []
        low = stmt.lower()
        for c in CURVES:
            if c in low:
                flags.append(f"curve:{c}")
        for r in REDUCTION:
            if r in stmt:
                flags.append(f"shape:{r}")
        for num in re.findall(r"\d{5,}", stmt):
            flags.append(f"numeral:{num[:24]}")
        print(f"line {line}: {'CLEAN' if not flags else 'HIT ' + ', '.join(flags)}")
        print(f"  {stmt.strip().splitlines()[0][:140]}")
        if flags:
            hits += 1
    print(f"statements={sum(1 for _ in lemma_statements(src))} hits={hits}")
    return 0 if hits == 0 else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1]))
