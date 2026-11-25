#!/usr/bin/env bash
# XVI.D3: exercise the shipped failure taxonomy outside HOL Light.
#
# The block between BEGIN/END BRIDGE_FAILURE_STANDALONE in tools/bridge.ml uses
# only the OCaml standard library, so it is extracted verbatim and run with the
# driver in tests/bridge_failure_selftest_driver.ml. No Python, no network, no
# HOL Light.
#
# Usage: tests/bridge_failure_selftest.sh   (exit 0 iff every case passes)
set -u
set -o pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 1

tmp="$(mktemp -d)"
cleanup() { rm -rf "$tmp"; }
trap cleanup EXIT

block="$tmp/bridge_failure.ml"
awk '/BEGIN BRIDGE_FAILURE_STANDALONE/{on=1; next} /END BRIDGE_FAILURE_STANDALONE/{on=0} on' \
  tools/bridge.ml > "$block"

if ! grep -q 'let bridge_slice_retry' "$block"; then
  echo "FAIL: taxonomy block not found in tools/bridge.ml" >&2
  exit 1
fi

# One compilation unit so the toplevel sees both the extracted block and the
# driver.
cat "$block" tests/bridge_failure_selftest_driver.ml > "$tmp/selftest.ml"

cd "$tmp" || exit 1
if ocaml selftest.ml; then
  echo "OK: bridge_failure_selftest"
else
  echo "FAIL: bridge_failure_selftest" >&2
  exit 1
fi
