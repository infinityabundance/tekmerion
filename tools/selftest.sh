#!/usr/bin/env bash
# Offline scaffold self-test for the failure-side TacticTrace extension.
#
# It does NOT build HOL Light or s2n-bignum. The OCaml module is syntax-checked
# only; resolving `goal`, `pp_print_goal`, and ExportTrace requires loading it
# inside HOL Light, which is deliberately out of scope here.

set -u
set -o pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 1

failures=0
fail() {
  printf 'FAIL: %s\n' "$1"
  failures=$((failures + 1))
}
section() {
  printf '\n=== %s ===\n' "$1"
}

tmp="$(mktemp -d)"
cleanup() { rm -rf "$tmp"; }
trap cleanup EXIT

# ---------------------------------------------------------------------------
section "1/6 OCaml syntax check (syntax only; type-checking needs HOL Light)"

# Compile a copy inside the temp directory so .cmo/.cmi artifacts never land
# in the repository, wherever ocamlc decides to put them.
cp trace/exptrace_failure.ml "$tmp/"
if ( cd "$tmp" && ocamlc -stop-after parsing -c exptrace_failure.ml ); then
  printf '%s\n' \
    "OK: trace/exptrace_failure.ml parses." \
    "NOTE: this is a syntax check only. It does not type-check 'goal'," \
    "      'pp_print_goal', or 'ExportTrace'; those resolve only inside" \
    "      HOL Light after exportTrace.ml is loaded."
else
  fail "OCaml syntax check failed"
fi

# ---------------------------------------------------------------------------
section "2/6 patch_wrapper.py --self-test"
if python3 tools/patch_wrapper.py --self-test; then
  printf 'OK: patch_wrapper self-test passed\n'
else
  fail "patch_wrapper.py --self-test failed"
fi

# ---------------------------------------------------------------------------
section "3/6 trace_conformance.py --self-test"
if python3 tools/trace_conformance.py --self-test; then
  printf 'OK: trace_conformance self-test passed\n'
else
  fail "trace_conformance.py --self-test failed"
fi

# ---------------------------------------------------------------------------
section "4/6 write-through self-test (module compiled against stubs, no dump)"
if python3 tools/test_write_through.py; then
  printf 'OK: write-through self-test passed\n'
else
  fail "write-through self-test failed"
fi

# ---------------------------------------------------------------------------
section "5/6a existing Rust suite (cargo test --offline)"
if cargo test --offline; then
  printf 'OK: cargo test --offline passed\n'
else
  fail "cargo test --offline failed"
fi

section "5/6b existing mutation harness (tools/mutate.py)"
if python3 tools/mutate.py; then
  printf 'OK: tools/mutate.py passed\n'
else
  fail "tools/mutate.py failed"
fi

section "6/6 lifecycle mutation harness (tools/mutate_lifecycle.py)"
if python3 tools/mutate_lifecycle.py; then
  printf 'OK: tools/mutate_lifecycle.py passed\n'
else
  fail "tools/mutate_lifecycle.py failed"
fi

# ---------------------------------------------------------------------------
printf '\n'
if [ "$failures" -ne 0 ]; then
  printf 'SELF-TEST FAILED: %d check(s) failed\n' "$failures"
  exit 1
fi
printf 'SELF-TEST PASSED\n'
