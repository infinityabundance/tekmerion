#!/bin/bash
# IX.89: build and run one instrumented human `_CORRECT` proof, capped and
# strictly -j1, on a scratch copy of the upstream tree.  The proof file is
# restored from `.bench/human-baseline` after the run.
#
# usage: run_human.sh <plain|phase> <add|sub|neg>
set -u
ROOT=/mnt/1tb_kingston/Tekmerion
HOL="$ROOT/.bench/hol-light-stamp2"
cd "$ROOT" || exit 99
eval "$(/home/one/.local/bin/opam env --switch "$HOL" --set-switch)"
export PATH="$ROOT/.bench/bin:/home/one/.elan/toolchains/leanprover--lean4---v4.29.1/bin:$PATH"

mode=$1; op=$2
arch=arm dir=curve25519 fam=p25519
base="bignum_${op}_${fam}"
th="BIGNUM_$(echo "${op}_${fam}" | tr 'a-z' 'A-Z')_CORRECT"
tree="$ROOT/.bench/ix89/human"
ml="$tree/${arch}/proofs/${base}.ml"
pristine="$ROOT/.bench/human-baseline/${arch}/proofs/${base}.ml"
outdir="$ROOT/.bench/ix89/human_out"; mkdir -p "$outdir"
tag="${arch}_${op}_${fam}.${mode}"

cp "$pristine" "$ml" || exit 96
if [ "$mode" = phase ]; then
  perl "$ROOT/tests/ix89/instr_human.pl" phase "$pristine" \
       "$ROOT/tests/ix89/bodies/${op}.tac" "$th" "$ml" || exit 95
else
  perl "$ROOT/tests/ix89/instr_human.pl" plain "$pristine" "" "$th" "$ml" || exit 95
fi

rm -f "$tree/${arch}/${dir}/${base}.correct"
( cd "$tree/${arch}" && \
  "$ROOT/tools/cap.sh" 12G -- make -j1 -B HOLDIR="$HOL" "${dir}/${base}.correct" ) \
  > "$outdir/${tag}.log" 2>&1
rc=$?
cp "$tree/${arch}/${dir}/${base}.correct" "$outdir/${tag}.correct" 2>/dev/null
cp "$pristine" "$ml"      # restore the scratch proof file
echo "HUMAN_RUN ${tag} rc=$rc"
grep -E '^HUMAN_CORRECT_INF|^Running time:' "$outdir/${tag}.correct" 2>/dev/null | head
exit "$rc"
