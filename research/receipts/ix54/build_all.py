#!/usr/bin/env python3
"""Generate and build the nine x86 drop-in proofs in upstream s2n-bignum.

Scratch tool: lives under .bench/ and is not part of the trusted or shipped tree.
No sealed proof file is read. The proof term is the sealed-eval emitter's answer;
the wrapper is the upstream file shape (learned from a non-sealed proof).
"""
import json
import re
import shutil
import subprocess
from pathlib import Path

ROOT = Path("/mnt/1tb_kingston/Tekmerion")
DROP = ROOT / ".bench/dropin"
PROOFS = DROP / "x86/proofs"
GEN = ROOT / ".bench/dropin-gen"
LOGS = GEN / "logs"
LOGS.mkdir(parents=True, exist_ok=True)

FAMILIES = {
    "p25519": ("curve25519", 2**255 - 19, "p_25519"),
    "p256k1": ("secp256k1", 2**256 - 2**32 - 977, "p_256k1"),
    "sm2": ("sm2", 2**256 - 2**224 - 2**96 + 2**64 - 1, "p_sm2"),
}
OPS = ("add", "sub", "neg")

problems = json.loads((ROOT / ".bench/problems.json").read_text())

PATH = (":".join([
    "/home/one/.elan/toolchains/leanprover--lean4---v4.29.1/bin",
    "/home/one/.wasmtime/bin", "/home/one/.local/bin", "/home/one/.cargo/bin",
    "/home/one/.dotnet/tools", "/home/one/.opencode/bin", "/home/one/bin",
    "/opt/cuda/bin", "/opt/rocm/bin", "/usr/local/sbin", "/usr/local/bin",
    "/usr/bin", "/usr/bin/site_perl", "/usr/bin/vendor_perl",
    "/usr/bin/core_perl", "/home/one/.lmstudio/bin",
]))


def text_bytes(obj: Path) -> bytes:
    out = GEN / (obj.stem + ".text.bin")
    subprocess.run(["objcopy", "-O", "binary", "--only-section=.text",
                    str(obj), str(out)], check=True)
    return out.read_bytes()


def write_dropin(op: str, fam: str) -> tuple[str, Path]:
    d = FAMILIES[fam][0]
    theorem = f"BIGNUM_{op.upper()}_{fam.upper()}_CORRECT"
    task = f"x86.bignum_{op}_{fam}.{theorem}"
    query = problems[task]["query"]
    answer = (GEN / "answers" / f"bignum_{op}_{fam}.answer.txt").read_text()
    obj = DROP / "x86" / d / f"bignum_{op}_{fam}.o"
    raw = text_bytes(obj)
    mc = "\n".join("  " + " ".join(f"0x{b:02x};" for b in raw[i:i + 8])
                   for i in range(0, len(raw), 8))
    base = f"bignum_{op}_{fam}"
    mod = FAMILIES[fam][2]
    body = f'''(* DROP-IN experiment: generated Tekmerion proof in upstream shape. *)
needs "x86/proofs/base.ml";;

let {base}_mc = define_assert_from_elf "{base}_mc" "x86/{d}/{base}.o"
[
{mc}
];;

let {base}_tmc = define_trimmed "{base}_tmc" {base}_mc;;
let {theorem.replace('_CORRECT','_EXEC')} = X86_MK_CORE_EXEC_RULE {base}_tmc;;

let {mod} = new_definition `{mod} = {FAMILIES[fam][1]}`;;

let {theorem} = time prove
 ({query},
{answer});;

Printf.printf "val {theorem} : thm = %s\\n%!" (string_of_thm {theorem});;
'''
    out = PROOFS / f"{base}.ml"
    out.write_text(body)
    return task, out


def build(op: str, fam: str) -> dict:
    d = FAMILIES[fam][0]
    target = f"{d}/bignum_{op}_{fam}.correct"
    log = LOGS / f"{d}_{op}_{fam}.log"
    env = {"PATH": PATH, "HOME": "/home/one", "HOLDIR": str(ROOT / ".bench/hol-light")}
    with log.open("w") as fh:
        proc = subprocess.run(
            ["make", "-j1", f"HOLDIR={env['HOLDIR']}", target],
            cwd=DROP / "x86", stdout=fh, stderr=subprocess.STDOUT, env=env)
    txt = log.read_text(errors="replace")
    correct = DROP / "x86" / d / f"bignum_{op}_{fam}.correct"
    has_thm = correct.exists() and f"val BIGNUM_{op.upper()}_{fam.upper()}_CORRECT : thm" in correct.read_text(errors="replace")
    return {"task": f"{d}/{op}_{fam}", "exit": proc.returncode,
            "theorem": has_thm,
            "exception": "ANSWER_EXCEPTION" in txt or "Fatal error" in txt,
            "log": str(log)}


def main() -> int:
    results = []
    for op in OPS:
        for fam in FAMILIES:
            task, path = write_dropin(op, fam)
            print(f"wrote {path.name}", flush=True)
            r = build(op, fam)
            results.append(r)
            print(f"  {r['task']}: exit={r['exit']} theorem={r['theorem']} exception={r['exception']}", flush=True)
    print("\nSUMMARY")
    ok = sum(1 for r in results if r["exit"] == 0 and r["theorem"])
    for r in results:
        print(f"{r['task']:28s} exit={r['exit']} theorem={r['theorem']}")
    print(f"OK {ok}/{len(results)}")
    return 0 if ok == len(results) else 1


if __name__ == "__main__":
    raise SystemExit(main())
