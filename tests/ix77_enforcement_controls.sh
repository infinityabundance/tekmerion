#!/usr/bin/env bash
# XVII.A3/A4 firing controls, at the binary level.
#
# Builds a throw-away git repository with a real freeze closure (fake runtime),
# then:
#
#   A3  invokes the *sealed emit* with TEKMERION_ALG_CLOSE set, where the
#       closure records it unset. The sealed policy must refuse.
#   A4  alters one non-digest field (runtime.rustc) in the manifest committed at
#       the tag, then runs `freeze check-tag` against the original manifest. The
#       complete-manifest binding must refuse and name runtime.rustc.
#
# Nothing here touches the real tree, its tags, or `.bench/`. No network.
#
# Prerequisite: target/debug/{freeze,emit} built from the current tree.
set -u
set -o pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FREEZE="$ROOT/target/debug/freeze"
EMIT="$ROOT/target/debug/emit"

for bin in "$FREEZE" "$EMIT"; do
  if [ ! -x "$bin" ]; then
    echo "FAIL: $bin is not built (cargo build --offline --bins)" >&2
    exit 1
  fi
done

TMP="$(mktemp -d)"
cleanup() { rm -rf "$TMP"; }
trap cleanup EXIT

REPO="$TMP/repo"
HOL="$TMP/hol"
mkdir -p "$REPO/src" "$REPO/tools" "$REPO/runtime" "$REPO/receipts"
printf 'fn main() {}\n' > "$REPO/src/main.rs"
printf 'let x = 1\n' > "$REPO/tools/bridge.ml"
printf '[package]\nname = "fixture"\n' > "$REPO/Cargo.toml"
printf '# lock\n' > "$REPO/Cargo.lock"
printf '\177ELF not really\n' > "$REPO/runtime/emit"
printf '{}\n' > "$REPO/runtime/problems.json"
printf '#!/bin/sh\necho 2.1.2\n' > "$REPO/runtime/cadical"
chmod +x "$REPO/runtime/cadical"
mkdir -p "$HOL"
printf 'cmxa fixture\n' > "$HOL/hol_lib.cmxa"

git_commit() {
  git -C "$1" -c user.name=infinityabundance \
    -c user.email=255699974+infinityabundance@users.noreply.github.com \
    -c commit.gpgsign=false add -A
  git -C "$1" -c user.name=infinityabundance \
    -c user.email=255699974+infinityabundance@users.noreply.github.com \
    -c commit.gpgsign=false commit -q -m "$2"
}

git -C "$REPO" init -q
git_commit "$REPO" freeze
git -C "$HOL" init -q
git_commit "$HOL" hol
TAG=freeze-ix77-control

# The generation step must see exactly the environment the manifest records.
unset TEKMERION_ALG_CLOSE TEKMERION_FAULT TEKMERION_PROBLEMS \
      TEKMERION_RECORD_ANSWERS TEKMERION_SIM_BACKEND \
      TEKMERION_SLICE_CONTROL TEKMERION_SLICE_DEBUG

"$FREEZE" manifest --repo "$REPO" --tag "$TAG" --runtime \
  --executable runtime/emit --hol "$HOL" \
  --cadical "$REPO/runtime/cadical" \
  --benchmark "$REPO/runtime/problems.json" \
  --out "$REPO/receipts/closure.json" >/dev/null
git_commit "$REPO" manifest
git -C "$REPO" -c tag.gpgsign=false tag -f "$TAG" HEAD
TAG_COMMIT="$(git -C "$REPO" rev-parse HEAD)"

printf '{"freeze_commit":"%s","freeze_tag":"%s",' "$TAG_COMMIT" "$TAG" \
  > "$REPO/receipts/sealed.json"
printf '"sealed_ids":["x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT"],' \
  >> "$REPO/receipts/sealed.json"
printf '"freeze_closure":"receipts/closure.json"}\n' >> "$REPO/receipts/sealed.json"

echo "=== A3: sealed emit, TEKMERION_ALG_CLOSE set where the manifest records unset ==="
echo '$ TEKMERION_ALG_CLOSE=ix77-live-control target/debug/emit --sealed-eval \'
echo '      --sealed-manifest receipts/sealed.json --freeze-tag '"$TAG"' \'
echo '      --task x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT --mode answer \'
echo '      --out /tmp/ix77-answer.txt'
(
  cd "$REPO" || exit 1
  TEKMERION_ALG_CLOSE=ix77-live-control "$EMIT" --sealed-eval \
    --sealed-manifest receipts/sealed.json --freeze-tag "$TAG" \
    --task x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT --mode answer \
    --out "$TMP/ix77-answer.txt"
)
echo "exit=$?"

echo
echo "=== A4: check-tag, runtime.rustc changed in the manifest committed at the tag ==="
sed -i 's/"rustc": "rustc [^"]*"/"rustc": "ix77-changed-rustc-version"/' \
  "$REPO/receipts/closure.json"
grep '"rustc"' "$REPO/receipts/closure.json"
git_commit "$REPO" changed-rustc
git -C "$REPO" -c tag.gpgsign=false tag -f "$TAG" HEAD
# The manifest under verification is the original, restored from the manifest
# commit; the tag now carries the one-field change.
git -C "$REPO" checkout "$TAG_COMMIT" -- receipts/closure.json
echo '$ target/debug/freeze check-tag --manifest receipts/closure.json --repo <fixture>'
"$FREEZE" check-tag --manifest "$REPO/receipts/closure.json" --repo "$REPO"
echo "exit=$?"
