#!/usr/bin/env bash
# XXX.A4: exhaustive small-integer regression tests for the carry
# certificate's numeric bound layer.
#
# The block between BEGIN/END BRIDGE_CERT_STANDALONE in tools/bridge.ml is
# compiled verbatim against tests/cert_bounds_shim.ml (an exact-rational
# stand-in for HOL Light's `Num') with the driver in
# tests/cert_bounds_selftest_driver.ml.  No Python, no network, no HOL Light,
# no proof build, no bench.
#
# Every case is checked against an independent semantic oracle: the emitted
# bound must be the exact integer endpoint (or the true max/min of the target),
# so a wrong rule is reported as a named counterexample.  The pre-fix join rule
# and the pre-fix `v-1' tightening are required to be rejected.
#
# Usage: tests/cert_bounds_selftest.sh   (exit 0 iff every case passes)
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

block="$tmp/cert_bounds.ml"
awk '/BEGIN BRIDGE_CERT_STANDALONE/{on=1; next} /END BRIDGE_CERT_STANDALONE/{on=0} on' \
  tools/bridge.ml > "$block"

for wanted in bridge_cert_bound_core bridge_cert_join_lo bridge_cert_join_hi \
              bridge_cert_hi bridge_cert_lo; do
  if ! grep -q "let .*$wanted" "$block"; then
    echo "FAIL: $wanted not found in the extracted block" >&2
    exit 1
  fi
done

{
  cat tests/cert_bounds_shim.ml
  cat "$block"
  cat tests/cert_bounds_selftest_driver.ml
} > "$tmp/selftest.ml"

cd "$tmp" || exit 1
if ocaml selftest.ml; then
  echo "OK: cert_bounds_selftest"
else
  echo "FAIL: cert_bounds_selftest" >&2
  exit 1
fi
