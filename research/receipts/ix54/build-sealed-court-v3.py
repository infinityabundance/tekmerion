#!/usr/bin/env python3
"""Build the pruned, proof-free input tree for the O10.2 structural run.

Reads only problems.json metadata (inlined_locations) and copies the prefix of
each template that combine-answer-and-setup.py actually consumes:
  * check_parsing_error: lines[:linenum]
  * write_template_ml:    template_lines[prevline:linenum]
Nothing at or after `linenum` is copied, so no answer/proof text enters the tree.
"""
import json
import shutil
import subprocess
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
BENCH = REPO / ".bench"
COURT = BENCH / "sealed-court-v3"
R = COURT / "repo"
RB = R / ".bench"

IDS = [
    "x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT",
    "x86.bignum_sub_p25519.BIGNUM_SUB_P25519_CORRECT",
    "x86.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT",
    "x86.bignum_add_p256k1.BIGNUM_ADD_P256K1_CORRECT",
    "x86.bignum_sub_p256k1.BIGNUM_SUB_P256K1_CORRECT",
    "x86.bignum_neg_p256k1.BIGNUM_NEG_P256K1_CORRECT",
    "x86.bignum_add_sm2.BIGNUM_ADD_SM2_CORRECT",
    "x86.bignum_sub_sm2.BIGNUM_SUB_SM2_CORRECT",
    "x86.bignum_neg_sm2.BIGNUM_NEG_SM2_CORRECT",
]

DIR_FOR = {"p25519": "curve25519", "p256k1": "secp256k1", "sm2": "sm2"}


def main():
    if COURT.exists():
        shutil.rmtree(COURT)
    for d in (RB / "ml_files" / "x86",
              RB / "objfiles" / "x86" / "generic",
              RB / "s2n-bignum" / "x86" / "generic",
              R / "tools", R / "research" / "receipts"):
        d.mkdir(parents=True, exist_ok=True)

    problems = json.loads((BENCH / "problems.json").read_text())

    # Bench scripts + data.  template-cache.json is omitted on purpose: it is a
    # rebuildable cache that the harness writes itself.
    for name in ("problems.json", "timeouts.json", "timeout-map.json",
                 "combine-answer-and-setup.py", "run-answers.sh",
                 "collect-verdicts.py", "synchk.sh"):
        shutil.copy2(BENCH / name, RB / name)
    (RB / "run-answers.sh").chmod(0o755)

    # Templates truncated to the prefix the harness reads.
    for task in IDS:
        rel, linenum = problems[task]["inlined_locations"][0]
        with open(BENCH / rel, "r", encoding="utf-8") as fh:
            lines = [line for i, line in enumerate(fh) if i < linenum]
        with open(RB / rel, "w", encoding="utf-8") as fh:
            fh.writelines(lines)
        print(f"template {rel} prefix_lines={len(lines)} of linenum={linenum}")

    # Nine task objects for the emitter, plus the generic madd object that the
    # judge's prelude declares; the judge resolves them relative to objfiles/.
    for task in IDS:
        body = task.split("bignum_", 1)[1]
        op, rest = body.split("_", 1)
        family = rest.split(".", 1)[0]
        name = f"bignum_{op}_{family}.o"
        d = DIR_FOR[family]
        src = BENCH / "s2n-bignum" / "x86" / d / name
        for parent in (RB / "s2n-bignum" / "x86", RB / "objfiles" / "x86"):
            (parent / d).mkdir(parents=True, exist_ok=True)
            shutil.copy2(src, parent / d / name)

    madd = BENCH / "s2n-bignum" / "x86" / "generic" / "bignum_madd.o"
    for parent in (RB / "s2n-bignum" / "x86", RB / "objfiles" / "x86"):
        shutil.copy2(madd, parent / "generic" / "bignum_madd.o")

    # Frozen tools, git metadata, manifest.
    shutil.copytree(REPO / "tools", R / "tools", dirs_exist_ok=True)
    shutil.copytree(REPO / ".git", R / ".git", symlinks=True)
    shutil.copy2(REPO / "research" / "receipts" / "ix54-sealed-manifest.json",
                 R / "research" / "receipts" / "ix54-sealed-manifest.json")

    # HOL Light is bind-mounted read-only at its frozen absolute path; the
    # harness expects ./hol-light next to the bench scripts.
    (RB / "hol-light").symlink_to("/mnt/1tb_kingston/Tekmerion/.bench/hol-light")

    print("built", R)
    print(subprocess.run(["du", "-sh", str(COURT)], capture_output=True,
                         text=True).stdout.strip())


if __name__ == "__main__":
    main()
