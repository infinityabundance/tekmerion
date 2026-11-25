#!/usr/bin/env bash
# XIX.A3: exhaustive small-domain tests for the interval engine.
#
# The block between BEGIN/END BRIDGE_INTERVAL_STANDALONE in tools/bridge.ml is
# compiled verbatim (against tests/interval_engine_shim.ml, an exact-rational
# stand-in for HOL Light's `Num') with the driver in
# tests/interval_engine_selftest_driver.ml.  No Python, no network, no
# HOL Light, no proof build.
#
# Every bound pair in a small family is compared with brute-force enumeration
# of rational points in the two intervals; a wrong rule is reported as a
# named counterexample, and the retired join-based rule is required to fail.
#
# Usage: tests/interval_engine_selftest.sh   (exit 0 iff every case passes)
set -u
set -o pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 1

if ! command -v ocaml >/dev/null 2>&1; then
  echo "FAIL: the OCaml toplevel (ocaml) is required for this test" >&2
  exit 1
fi

tmp="$(mktemp -d)"
cleanup() { rm -rf "$tmp"; }
trap cleanup EXIT

block="$tmp/interval.ml"
awk '/BEGIN BRIDGE_INTERVAL_STANDALONE/{on=1; next} /END BRIDGE_INTERVAL_STANDALONE/{on=0} on' \
  tools/bridge.ml > "$block"

if ! grep -q 'let bridge_bounds_of_terms' "$block"; then
  echo "FAIL: interval block not found in tools/bridge.ml" >&2
  exit 1
fi

{
  cat tests/interval_engine_shim.ml
  cat "$block"
  cat tests/interval_engine_selftest_driver.ml
} > "$tmp/selftest.ml"

cd "$tmp" || exit 1
if ocaml selftest.ml; then
  echo "OK: interval_engine_selftest"
else
  echo "FAIL: interval_engine_selftest" >&2
  exit 1
fi
