#!/usr/bin/env bash
# Export one solver-free aggregate-route derivation as a replayable proof object.
#
# usage: tools/export_proof_objects.sh <arch> <op> <family> <aggregate-answer> <outdir>
#
#   arch    arm | x86
#   op      add | sub | neg | ...
#   family  p25519 | p256 | sm2 | ...
#   answer  the emitter's ".answer" file for the aggregate route
#   outdir  where the driver, raw stream, object, store, theory and expected go
#
# The driver is assembled from the upstream proof file (its ELF words, EXEC
# rule and goal) plus the emitted aggregate answer.  Recording is enabled only
# by TEKMERION_PROOF_RAW in the recording kernel (`.bench/hol-light-objects`).
# Every heavy step runs through tools/cap.sh, one at a time.
set -euo pipefail

ROOT=/mnt/1tb_kingston/Tekmerion
HOL="$ROOT/.bench/hol-light-objects"
PROOFS="$ROOT/.bench/s2n-bignum"
OPAM_LIB="$ROOT/.bench/hol-light/_opam/lib"

arch=$1
op=$2
fam=$3
answer=$4
out=$5

case "$answer" in /*) ;; *) answer="$ROOT/$answer" ;; esac
case "$out" in /*) ;; *) out="$ROOT/$out" ;; esac

tag="${arch}_${op}_${fam}"
TH="BIGNUM_$(printf '%s' "${op}_${fam}" | tr 'a-z' 'A-Z')_CORRECT"
base="bignum_${op}_${fam}"
src="$PROOFS/$arch/proofs/${base}.ml"

[ -f "$src" ] || { echo "missing upstream proof $src" >&2; exit 2; }
[ -f "$answer" ] || { echo "missing aggregate answer $answer" >&2; exit 2; }
mkdir -p "$out"

driver="$out/${tag}.ml"
raw="$out/${tag}.raw"
obj="$out/${tag}.hlpo"
store="$out/${tag}.hlst"
theory="$out/${tag}.theory"
expected="$out/${tag}.expected"
json="$out/${tag}.final.json"
runlog="$out/${tag}.run.log"
finlog="$out/${tag}.finalize.log"

# ---- 1. assemble the driver ------------------------------------------------
{
  echo '#load "unix.cma";;'
  grep -m1 '^needs ' "$src"
  awk -v b="$base" '
    $0 ~ "^let " b "_mc = define_assert_from_elf" { p = 1 }
    p { print }
    p && /^\];;/ { exit }
  ' "$src"
  awk '/ARM_MK_EXEC_RULE/ { print; exit }' "$src"
  awk '/^let p_25519 = new_definition/ { print; exit }' "$src"
  echo "let _ = proof_record_begin ();;"
  echo "let stamp_before_${TH} = inference_count ();;"
  echo "let ${TH} = time prove"
  awk '
    /^let .*_CORRECT = time prove/ { p = 1; next }
    p && /^[[:space:]]*\(`/ { q = 1 }
    q { print }
    q && /`,[[:space:]]*$/ { exit }
  ' "$src"
} > "$driver"

cat "$answer" >> "$driver"

{
  echo ");;"
  printf 'let _ = Printf.printf "proof_nodes=%%d\\n%%!" (proof_node_count ());;\n'
  printf 'let _ = proof_export "%s" %s;;\n' "$TH" "$TH"
  printf 'let _ = Printf.printf "exported %s\\n%%!";;\n' "$TH"
} >> "$driver"

echo "DRIVER $driver ($(wc -l < "$driver") lines)"

# ---- 2. record (capped proof run) -----------------------------------------
free -g | sed -n '1,2p'
(
  cd "$PROOFS" || exit 99
  TEKMERION_PROOF_RAW="$raw" \
  CAML_LD_LIBRARY_PATH="$OPAM_LIB/stublibs:$OPAM_LIB/ocaml/stublibs:$OPAM_LIB/ocaml" \
  HOLLIGHT_DIR="$HOL" HOLLIGHT_USE_MODULE=1 \
    "$ROOT/tools/cap.sh" --timeout 3600 12G -- \
      "$HOL/ocaml-hol" -init "$HOL/hol.ml" -I "$HOL" < "$driver"
) > "$runlog" 2>&1 || {
  echo "RECORD_FAILED $tag (see $runlog)" >&2
  tail -20 "$runlog" >&2
  exit 1
}

grep -E 'proof_nodes|exported|Running time|cap\.sh:' "$runlog" | tail -6

if grep -Eq 'Parse error|Error:|ANSWER_EXCEPTION|proof_nodes=0' "$runlog"; then
  echo "RECORD_FAILED $tag (toplevel error; see $runlog)" >&2
  grep -nE 'Parse error|Error:|ANSWER_EXCEPTION|proof_nodes=' "$runlog" | head -5 >&2
  exit 1
fi

# ---- 3. finalise (capped) --------------------------------------------------
"$ROOT/tools/cap.sh" --timeout 3600 20G -- \
  "$ROOT/target/release/proof_object" finalize \
    --raw "$raw" --object "$obj" --store "$store" \
    --theory-out "$theory" --expected "$expected" --json "$json" \
  > "$finlog" 2>&1 || {
    echo "FINALIZE_FAILED $tag (see $finlog)" >&2
    tail -20 "$finlog" >&2
    exit 1
  }

grep -E 'FINALIZED|cap\.sh:' "$finlog" | tail -3
echo "OBJECT $obj"
