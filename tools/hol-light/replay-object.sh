#!/usr/bin/env bash
# Replay one exported proof object with HOL Light's own kernel (XXII.B).
#
# usage: tools/hol-light/replay-object.sh <object.hlpo> [stats|replay]
#
# The object is parsed from the bytes its digest covers and rebuilt node by
# node with the primitive rules of fusion.ml.  No derived rule and no tactic
# is used for the reconstruction.  The whole run is one process group under
# tools/cap.sh (default 12G), and the cap line is printed for the receipt.
set -euo pipefail

ROOT=/mnt/1tb_kingston/Tekmerion
HOL="${HOL_REPLAY:-$ROOT/.bench/ix88-hol-replay}"
OPAM_LIB="$ROOT/.bench/hol-light/_opam/lib"

obj=$1
mode=${2:-replay}
cap=${CAP:-12G}

case "$obj" in /*) ;; *) obj="$ROOT/$obj" ;; esac
[ -x "$HOL/ocaml-hol" ] || { echo "missing replay kernel $HOL (run tools/hol-light/build-replay-kernel.sh)" >&2; exit 2; }

free -g | sed -n '1,2p'
df -h /mnt/1tb_kingston | tail -1

TEKMERION_OBJECT="$obj" TEKMERION_PO_MODE="$mode" \
CAML_LD_LIBRARY_PATH="$OPAM_LIB/stublibs:$OPAM_LIB/ocaml/stublibs:$OPAM_LIB/ocaml" \
HOLLIGHT_DIR="$HOL" HOLLIGHT_USE_MODULE=1 \
  "$ROOT/tools/cap.sh" --timeout 3600 "$cap" -- \
    "$HOL/ocaml-hol" -init "$HOL/hol.ml" -I "$HOL" \
    < "$ROOT/tools/hol-light/replay-proof-object.ml"
