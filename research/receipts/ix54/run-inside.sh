#!/usr/bin/bash
# Runs inside the sealed court container.  Every path below is inside the
# container's mount namespace.
set -u

IDS="x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT
x86.bignum_sub_p25519.BIGNUM_SUB_P25519_CORRECT
x86.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT
x86.bignum_add_p256k1.BIGNUM_ADD_P256K1_CORRECT
x86.bignum_sub_p256k1.BIGNUM_SUB_P256K1_CORRECT
x86.bignum_neg_p256k1.BIGNUM_NEG_P256K1_CORRECT
x86.bignum_add_sm2.BIGNUM_ADD_SM2_CORRECT
x86.bignum_sub_sm2.BIGNUM_SUB_SM2_CORRECT
x86.bignum_neg_sm2.BIGNUM_NEG_SM2_CORRECT"

echo "=== IDENTITY ==="
id
python3 -V
git --version
command -v objdump cadical
mkdir -p /work/home /work/bin /work/answers
cp -a /court/repo /work/repo
cd /work/repo
echo "cwd=$(pwd)"

echo "=== ISOLATION PROBE (Step 2/4) ==="
find / -path '*proofs*' -name '*.ml' 2>/dev/null > /work/proofs-find.txt
echo "find_proofs_ml_lines=$(wc -l < /work/proofs-find.txt)"
cat /work/proofs-find.txt
find / -type d -name proofs 2>/dev/null > /work/proofs-dirs.txt
echo "find_proofs_dirs_lines=$(wc -l < /work/proofs-dirs.txt)"
cat /work/proofs-dirs.txt
echo "--- scoped: s2n-bignum .ml anywhere in view ---"
find / -path '*s2n-bignum*' -name '*.ml' 2>/dev/null | tee /work/s2n-ml.txt
echo "s2n_bignum_ml_lines=$(wc -l < /work/s2n-ml.txt)"
echo "--- scoped: all .ml in the court ---"
find /court /work/repo -name '*.ml' -not -path '*/hol-light/*' 2>/dev/null | sort
echo "court_ml_lines=$(find /court /work/repo -name '*.ml' -not -path '*/hol-light/*' 2>/dev/null | wc -l)"
echo "--- mounts ---"
mount | grep -E ' on /(work|tmp|court|usr|lib64|etc) ' | sed 's/,relatime.*//'
test -e /work/repo/.bench/s2n-bignum/x86/proofs/bignum_add_p25519.ml \
  && echo "TEST_E_PRESENT" || echo "TEST_E_ABSENT"
test -e /work/repo/.bench/ml_files/x86/bignum_add_p25519.ml \
  && echo "TEMPLATE_PRESENT" || echo "TEMPLATE_MISSING"
echo "template_line_count=$(wc -l < /work/repo/.bench/ml_files/x86/bignum_add_p25519.ml)"

echo "=== O10.2 FIRING CONTROL ==="
cat /work/repo/.bench/s2n-bignum/x86/proofs/bignum_add_p25519.ml
echo "cat_exit=$?"
cat /mnt/1tb_kingston/Tekmerion/.bench/s2n-bignum/x86/proofs/bignum_add_p25519.ml
echo "cat_hostpath_exit=$?"

export HOME=/work/home

cat > /work/opam-env.sh <<'ENVEOF'
OPAM_SWITCH_PREFIX='/mnt/1tb_kingston/Tekmerion/.bench/hol-light/_opam'; export OPAM_SWITCH_PREFIX;
OCAMLTOP_INCLUDE_PATH='/mnt/1tb_kingston/Tekmerion/.bench/hol-light/_opam/lib/toplevel'; export OCAMLTOP_INCLUDE_PATH;
CAML_LD_LIBRARY_PATH='/mnt/1tb_kingston/Tekmerion/.bench/hol-light/_opam/lib/stublibs:/mnt/1tb_kingston/Tekmerion/.bench/hol-light/_opam/lib/ocaml/stublibs:/mnt/1tb_kingston/Tekmerion/.bench/hol-light/_opam/lib/ocaml'; export CAML_LD_LIBRARY_PATH;
OCAML_TOPLEVEL_PATH='/mnt/1tb_kingston/Tekmerion/.bench/hol-light/_opam/lib/toplevel'; export OCAML_TOPLEVEL_PATH;
PATH='/work/bin:/mnt/1tb_kingston/Tekmerion/.bench/hol-light/_opam/bin:/home/one/.elan/toolchains/leanprover--lean4---v4.29.1/bin:/usr/local/sbin:/usr/local/bin:/usr/bin:/bin'; export PATH;
ENVEOF
cat > /work/bin/opam <<'SHIMEOF'
#!/bin/sh
# Faithful replay of `opam env --switch <hol-light> --set-switch` output,
# captured on the host; opam itself cannot initialise a read-only root.
cat /work/opam-env.sh
SHIMEOF
chmod +x /work/bin/opam
export PATH=/work/bin:/home/one/.elan/toolchains/leanprover--lean4---v4.29.1/bin:$PATH
eval "$(/work/bin/opam env)"

echo "=== EMIT ==="
for id in $IDS; do
  python3 tools/emit_ref_proof_x86.py \
    --sealed-eval \
    --sealed-manifest research/receipts/ix54-sealed-manifest.json \
    --freeze-tag freeze-x86-sealed-v3 \
    --task "$id" --mode answer --out "/work/answers/$id/answer.txt" \
    || echo "EMIT_FAILED $id"
done 2>&1 | tee /work/emit.log

if [ "${MODE:-full}" = "preflight" ]; then
  echo "PREFLIGHT_DONE"
  exit 0
fi

cd /work/repo/.bench
echo "=== COMBINE ==="
python3 combine-answer-and-setup.py /work/answers 1 2>&1 | tee /work/combine.log
EVALDIR=$(tail -n 1 /work/combine.log)
echo "EVALDIR=$EVALDIR"
ls -la "$EVALDIR" | head -40

echo "=== RUN ==="
./run-answers.sh "$EVALDIR" 1 > /work/run-answers.log 2>&1
echo "run_answers_exit=$?"

echo "=== COLLECT ==="
python3 collect-verdicts.py "$EVALDIR" /work/verdicts.csv 2>&1 | tee /work/collect.log
echo "collect_exit=$?"
echo "=== VERDICTS CSV ==="
cat /work/verdicts.csv
echo "=== JUDGE FILES ==="
for f in "$EVALDIR"/*.judge.txt; do
  echo "$(basename "$f")=$(cat "$f")"
done
echo "=== PROFILES ==="
grep -h '^PROFILE' "$EVALDIR"/*.run.outlog | sort
echo "=== RUN ERRLOGS (non-empty only) ==="
for f in "$EVALDIR"/*.run.errlog; do
  if [ -s "$f" ]; then echo "--- $f"; cat "$f"; fi
done
echo "=== RUN OUTLOG TAILS ==="
for f in "$EVALDIR"/*.run.outlog; do
  echo "--- $(basename "$f") $(wc -l < "$f") lines"
  tail -n 3 "$f"
done
echo "DONE"
