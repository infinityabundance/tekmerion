#!/usr/bin/env bash
# Exercise the shipped backward-closure trimmer outside HOL Light.
#
# The block between BEGIN/END CSAT_TRIM_STANDALONE in tools/csat_lib.ml uses
# only the OCaml standard library, so it is extracted verbatim and run with the
# driver in tests/csat_trim_selftest_driver.ml. No Python, no network.
#
# Usage: tests/csat_trim_selftest.sh   (exit 0 iff every case passes)
set -u
set -o pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 1

tmp="$(mktemp -d)"
cleanup() { rm -rf "$tmp"; }
trap cleanup EXIT

block="$tmp/csat_trim.ml"
awk '/BEGIN CSAT_TRIM_STANDALONE/{on=1; next} /END CSAT_TRIM_STANDALONE/{on=0} on' \
  tools/csat_lib.ml > "$block"

if ! grep -q 'let csat_trim_lrat' "$block"; then
  echo "FAIL: trimmer block not found in tools/csat_lib.ml" >&2
  exit 1
fi

# One CU so the toplevel sees both the extracted block and the driver.
cat "$block" tests/csat_trim_selftest_driver.ml > "$tmp/csat_trim_selftest.ml"

cd "$tmp" || exit 1
if ocaml csat_trim_selftest.ml; then
  echo "OK: csat_trim_selftest"
else
  echo "FAIL: csat_trim_selftest" >&2
  exit 1
fi
