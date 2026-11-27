#!/usr/bin/env bash
# Build a replay copy of the recording kernel with the trusted-input primitive
# of tools/hol-light/replay-import.diff (XXII.B).  The read-only recording
# kernel in .bench/hol-light-objects is never modified.
#
# usage: tools/hol-light/build-replay-kernel.sh
set -euo pipefail

ROOT=/mnt/1tb_kingston/Tekmerion
SRC="$ROOT/.bench/hol-light-objects"
DST="$ROOT/.bench/ix88-hol-replay"
OPAM_BIN="$ROOT/.bench/hol-light/_opam/bin"

[ -d "$SRC" ] || { echo "missing recording kernel $SRC" >&2; exit 2; }

free -g | sed -n '1,2p'
df -h /mnt/1tb_kingston | tail -1

if [ ! -d "$DST" ]; then
  cp -a "$SRC" "$DST"
fi

# Idempotent: only patch a pristine fusion.ml.
if ! grep -q "val proof_type" "$DST/fusion.ml"; then
  patch -p1 -d "$DST" < "$ROOT/tools/hol-light/replay-import.diff"
fi

# Rebuild the module the toplevel loads (HOLLIGHT_USE_MODULE=1).  The inliner
# regenerates hol_lib_inlined.ml from the patched fusion.ml.
PATH="$OPAM_BIN:$PATH" \
CAML_LD_LIBRARY_PATH="$ROOT/.bench/hol-light/_opam/lib/stublibs:$ROOT/.bench/hol-light/_opam/lib/ocaml/stublibs:$ROOT/.bench/hol-light/_opam/lib/ocaml" \
  "$ROOT/tools/cap.sh" --timeout 1800 12G -- \
    make -C "$DST" hol_lib.cmo > "$ROOT/.bench/ix88-replay-build.log" 2>&1

tail -5 "$ROOT/.bench/ix88-replay-build.log"
grep -c "proof_import" "$DST/hol_lib_inlined.ml"
echo "REPLAY_KERNEL $DST"
