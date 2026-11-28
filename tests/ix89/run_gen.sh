#!/bin/bash
# IX.89: emit, (optionally) further-instrument, assemble, build and run one
# generated ARM Curve25519 task under the algebraic close.  Capped, -j1.
#
# usage: run_gen.sh <plain|phase> <add|sub|neg>
#
# XXIX.A4: the repository root is derived from this script's location; every
# other path is relative to it (or to TEKMERION_BENCH when a caller supplies
# an alternate bench tree).  No absolute project path is hard-coded here.
set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)" || exit 99
ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)" || exit 99
BENCH="${TEKMERION_BENCH:-$ROOT/.bench}"
HOL="${TEKMERION_HOL:-$BENCH/hol-light-stamp2}"
cd "$ROOT" || exit 99

OPAM_BIN=$(command -v opam 2>/dev/null || true)
[ -n "$OPAM_BIN" ] || OPAM_BIN="$HOME/.local/bin/opam"
eval "$("$OPAM_BIN" env --switch "$HOL" --set-switch)"
export PATH="$BENCH/bin:${ELAN_HOME:-$HOME/.elan}/toolchains/leanprover--lean4---v4.29.1/bin:$PATH"

mode=$1; op=$2
arch=arm dir=curve25519 fam=p25519
base="bignum_${op}_${fam}"
th="BIGNUM_$(echo "${op}_${fam}" | tr 'a-z' 'A-Z')_CORRECT"
tid="$arch.bignum_${op}_${fam}.$th"
outdir="$BENCH/ix89/gen"; mkdir -p "$outdir"
tag="${arch}_${op}_${fam}.${mode}"
check="$ROOT/tests/ix89/stmt_check.sh"
fwcheck="$ROOT/tests/ix89/framework_check.sh"

# XXXI.B1: print the pinned upstream framework digests and their verdict on
# every run, before the build, so a drifted baseline is visible even if the
# proof later fails.  The exit status is enforced below.
"$fwcheck" hashes
hash_rc=$?
echo "GEN_UPSTREAM_HASH_RC $hash_rc"

# XXVIII.A / XXIX.A: the upstream conclusion is cached once per task and only
# accepted with valid provenance; only a missing or invalid cache pays for the
# human proof build (capped, -j1, sequential).
"$check" ensure "$op" || exit 94

export TEKMERION_ALG_CLOSE="$tid"
./target/release/emit --task "$tid" --mode phase --out "$outdir/${tag}.raw.answer" \
  --spent --spent-manifest "$BENCH/ix66/spent-manifest.json" || exit 98

if [ "$mode" = phase ]; then
  perl "$ROOT/tests/ix89/instr_gen.pl" "$outdir/${tag}.raw.answer" "$outdir/${tag}.answer" || exit 95
else
  cp "$outdir/${tag}.raw.answer" "$outdir/${tag}.answer"
fi

./target/release/dropin assemble --task "$tid" --answer "$outdir/${tag}.answer" --stamp || exit 97

# XXVIII.A / XXIX.A: append the same-session statement/hypothesis/axiom check
# to the generated proof.  It runs after the proof's own STAMP block, so it
# cannot change STAMP_INFERENCES.
"$check" inject "$op" || exit 93

rm -f "$BENCH/dropin/${arch}/${dir}/${base}.correct"
( cd "$BENCH/dropin/${arch}" && \
  "$ROOT/tools/cap.sh" 12G -- make -j1 -B HOLDIR="$HOL" "${dir}/${base}.correct" ) \
  > "$outdir/${tag}.log" 2>&1
rc=$?
cp "$BENCH/dropin/${arch}/${dir}/${base}.correct" "$outdir/${tag}.correct" 2>/dev/null
n=$(grep -c '^PHASE_MARK' "$outdir/${tag}.correct" 2>/dev/null || echo 0)
"$check" verdict "$tag" "$outdir/${tag}.correct"
sc=$?
echo "GEN_RUN ${tag} rc=$rc phase_marks=$n statement_rc=$sc"
if [ "$hash_rc" -eq 0 ]; then
  echo "GEN_UPSTREAM_UNCHANGED 1"
else
  echo "GEN_UPSTREAM_UNCHANGED 0"
fi
grep -E '^(STAMP_INF_BEFORE|STAMP_INFERENCES|Running time)' "$outdir/${tag}.correct" 2>/dev/null | head

# XXXI.B2: with TEKMERION_AB=1, build the same task under both simulation
# backends and fail the run on any goal/assumption/frame/bridge mismatch.
ab_rc=0
if [ "${TEKMERION_AB:-0}" = "1" ]; then
  TEKMERION_AB_SKIP_HASHES=1 "$fwcheck" ab "$tid"
  ab_rc=$?
  echo "GEN_AB_RC $ab_rc"
fi

[ "$rc" -eq 0 ] || exit "$rc"
[ "$sc" -eq 0 ] || exit "$sc"
[ "$hash_rc" -eq 0 ] || exit "$hash_rc"
[ "$ab_rc" -eq 0 ] || exit "$ab_rc"
exit 0
