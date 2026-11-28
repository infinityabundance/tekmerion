#!/bin/bash
# IX.89: emit, (optionally) further-instrument, assemble, build and run one
# generated ARM Curve25519 task under the algebraic close.  Capped, -j1.
#
# usage: run_gen.sh <plain|phase> <add|sub|neg>
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
tid="$arch.bignum_${op}_${fam}.$th"
outdir="$ROOT/.bench/ix89/gen"; mkdir -p "$outdir"
tag="${arch}_${op}_${fam}.${mode}"
check="$ROOT/tests/ix89/stmt_check.sh"

# XXVIII.A: the upstream conclusion is cached once per task; only the first
# run of a task pays for the human proof build (capped, -j1, sequential).
"$check" ensure "$op" || exit 94

export TEKMERION_ALG_CLOSE="$tid"
./target/release/emit --task "$tid" --mode phase --out "$outdir/${tag}.raw.answer" \
  --spent --spent-manifest .bench/ix66/spent-manifest.json || exit 98

if [ "$mode" = phase ]; then
  perl "$ROOT/tests/ix89/instr_gen.pl" "$outdir/${tag}.raw.answer" "$outdir/${tag}.answer" || exit 95
else
  cp "$outdir/${tag}.raw.answer" "$outdir/${tag}.answer"
fi

./target/release/dropin assemble --task "$tid" --answer "$outdir/${tag}.answer" --stamp || exit 97

# XXVIII.A: append the same-session statement/hypothesis/axiom check to the
# generated proof.  It runs after the proof's own STAMP block, so it cannot
# change STAMP_INFERENCES.
"$check" inject "$op" || exit 93

rm -f ".bench/dropin/${arch}/${dir}/${base}.correct"
( cd ".bench/dropin/${arch}" && \
  "$ROOT/tools/cap.sh" 12G -- make -j1 -B HOLDIR="$HOL" "${dir}/${base}.correct" ) \
  > "$outdir/${tag}.log" 2>&1
rc=$?
cp ".bench/dropin/${arch}/${dir}/${base}.correct" "$outdir/${tag}.correct" 2>/dev/null
n=$(grep -c '^PHASE_MARK' "$outdir/${tag}.correct" 2>/dev/null || echo 0)
"$check" verdict "$tag" "$outdir/${tag}.correct"
sc=$?
echo "GEN_RUN ${tag} rc=$rc phase_marks=$n statement_rc=$sc"
grep -E '^(STAMP_INF_BEFORE|STAMP_INFERENCES|Running time)' "$outdir/${tag}.correct" 2>/dev/null | head
[ "$rc" -eq 0 ] || exit "$rc"
exit "$sc"
