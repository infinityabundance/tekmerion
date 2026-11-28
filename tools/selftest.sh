#!/usr/bin/env bash
# Offline scaffold self-test for the failure-side TacticTrace extension.
#
# It does NOT build HOL Light or s2n-bignum. The OCaml module is syntax-checked
# only; resolving `goal`, `pp_print_goal`, `term`, and `string_of_term` requires
# loading it inside HOL Light, which is deliberately out of scope here. The real
# HOL Light load check and the proof runner are separate commands
# (research/03-INSTRUMENT.md section 9).

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
section "1/7 OCaml syntax check (syntax only; type-checking needs HOL Light)"

# Compile a copy inside the temp directory so .cmo/.cmi artifacts never land
# in the repository, wherever ocamlc decides to put them.
cp trace/exptrace_failure.ml "$tmp/"
if ( cd "$tmp" && ocamlc -stop-after parsing -c exptrace_failure.ml ); then
  printf '%s\n' \
    "OK: trace/exptrace_failure.ml parses." \
    "NOTE: this is a syntax check only. It does not type-check 'goal'," \
    "      'pp_print_goal', 'term', or 'string_of_term'; those resolve only" \
    "      inside HOL Light after exportTrace.ml is loaded."
else
  fail "OCaml syntax check failed"
fi

# ---------------------------------------------------------------------------
section "1b/7 carry-certificate bound selftest (exact rationals, no HOL Light)"

# XXX.A4: the numeric core of the carry certificate is extracted verbatim and
# checked against a semantic oracle over a small grid.  No proof build, no
# bench; the pre-fix join rule and `v-1' tightening are required to fail.
if bash tests/cert_bounds_selftest.sh; then
  printf 'OK: cert_bounds_selftest passed\n'
else
  fail "cert_bounds_selftest failed"
fi

# ---------------------------------------------------------------------------
section "2/7 runner script syntax"

if bash -n tools/run_traces.sh; then
  printf 'OK: tools/run_traces.sh parses\n'
else
  fail "tools/run_traces.sh syntax check failed"
fi

if cargo run --offline --quiet --bin run_traces_inject -- --self-test; then
  printf 'OK: run_traces_inject self-test passed\n'
else
  fail "cargo run --bin run_traces_inject -- --self-test failed"
fi

section "2b/7 attempt_id self-test"
if cargo run --offline --quiet --bin attempt_id -- --self-test; then
  printf 'OK: attempt_id self-test passed\n'
else
  fail "cargo run --bin attempt_id -- --self-test failed"
fi

section "2c/7 attempt_lifecycle self-test"
if cargo run --offline --quiet --bin attempt_lifecycle -- --self-test; then
  printf 'OK: attempt_lifecycle self-test passed\n'
else
  fail "cargo run --bin attempt_lifecycle -- --self-test failed"
fi

section "2d/7 run_traces.sh argument parsing"
output="$(bash tools/run_traces.sh --success-dir "$tmp/s" --bogus 2>&1)"
if printf '%s' "$output" | grep -q 'unknown option: --bogus'; then
  printf 'OK: an unknown option is rejected\n'
else
  fail "run_traces.sh did not reject --bogus (output: $output)"
fi
output="$(bash tools/run_traces.sh --success-dir "$tmp/s" \
  --proof "$tmp/no-such-proof.ml" 2>&1)"
if printf '%s' "$output" | grep -q 'proof not found:'; then
  printf 'OK: --proof is parsed and a missing path is reported\n'
else
  fail "run_traces.sh did not parse --proof (output: $output)"
fi
output="$(bash tools/run_traces.sh --success-dir "$tmp/s" \
  --proof "$tmp/no-such-proof.ml" word_max 2>&1)"
if printf '%s' "$output" | grep -q 'exactly one theorem name or --proof PATH'; then
  printf 'OK: --proof and a theorem name are mutually exclusive\n'
else
  fail "run_traces.sh did not reject --proof plus theorem (output: $output)"
fi

# ---------------------------------------------------------------------------
section "3/7 patch_wrapper --self-test"
if cargo run --offline --quiet --bin patch_wrapper -- --self-test; then
  printf 'OK: patch_wrapper self-test passed\n'
else
  fail "cargo run --bin patch_wrapper -- --self-test failed"
fi

# ---------------------------------------------------------------------------
section "4/7 trace_conformance --self-test"
if cargo run --offline --quiet --bin trace_conformance -- --self-test; then
  printf 'OK: trace_conformance self-test passed\n'
else
  fail "cargo run --bin trace_conformance -- --self-test failed"
fi

# ---------------------------------------------------------------------------
section "5/7 write-through self-test (module compiled against stubs, no dump)"
if cargo test --offline --test write_through -- --nocapture; then
  printf 'OK: write-through self-test passed\n'
else
  fail "write-through self-test failed"
fi

# ---------------------------------------------------------------------------
# Sections 6 and 7 are the heavy, duplicated checks: CI runs `cargo test` and
# each mutation harness as its own job/step. SELFTEST_SKIP_HEAVY=1 lets the
# scaffold job run only the checks that are unique to this script (1-5). The
# skip is explicit in the log and counted as neither a pass nor a failure; the
# heavy checks are still required, just elsewhere.
if [ "${SELFTEST_SKIP_HEAVY:-0}" = "1" ]; then
  section "6/7 heavy checks skipped (SELFTEST_SKIP_HEAVY=1)"
  printf 'SKIP: cargo test --offline (run as its own CI job)\n'
  printf 'SKIP: cargo run --bin mutate (run as its own CI job)\n'
  printf 'SKIP: cargo run --bin mutate_lifecycle (run as its own CI job)\n'
else
  section "6/7a existing Rust suite (cargo test --offline)"
  if cargo test --offline; then
    printf 'OK: cargo test --offline passed\n'
  else
    fail "cargo test --offline failed"
  fi

  section "6/7b existing mutation harness (cargo run --bin mutate)"
  if cargo run --offline --quiet --bin mutate; then
    printf 'OK: cargo run --bin mutate passed\n'
  else
    fail "cargo run --bin mutate failed"
  fi

  section "7/7 lifecycle mutation harness (cargo run --bin mutate_lifecycle)"
  if cargo run --offline --quiet --bin mutate_lifecycle; then
    printf 'OK: cargo run --bin mutate_lifecycle passed\n'
  else
    fail "cargo run --bin mutate_lifecycle failed"
  fi
fi

# ---------------------------------------------------------------------------
printf '\n'
if [ "$failures" -ne 0 ]; then
  printf 'SELF-TEST FAILED: %d check(s) failed\n' "$failures"
  exit 1
fi
printf 'SELF-TEST PASSED\n'
