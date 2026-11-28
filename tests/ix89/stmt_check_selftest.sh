#!/bin/bash
# XXIX.A5: mutation controls for the IX.89 statement harness itself.
#
# Five hostile cases are fed to the hardened harness.  Each must be rejected
# with a non-zero exit and a distinct named reason:
#
#   1. a run log with no completion marker,
#   2. a STATEMENT_OK for the wrong task,
#   3. a cache whose REPARSE marker is 0,
#   4. two STATEMENT_OK lines,
#   5. a stray STATEMENT_FAIL alongside a valid STATEMENT_OK.
#
# The self-test exits 0 only when all five are rejected, with five distinct
# reasons.  It uses a throwaway fixture bench under .bench/ix89/ (git-ignored)
# so that no real cache, proof, or object is touched.  Bash and coreutils
# only; no Python (Tekmerion-owned tooling rule).
set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)" || exit 99
ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)" || exit 99
STMT="$ROOT/tests/ix89/stmt_check.sh"
[ -x "$STMT" ] || { echo "SELFTEST_FAIL reason=missing-stmt-check" >&2; exit 98; }

mkdir -p "$ROOT/.bench/ix89" || exit 99
TMP_ROOT=$(mktemp -d "$ROOT/.bench/ix89/selftest.XXXXXX") || exit 99
cleanup () { rm -rf "$TMP_ROOT"; }
trap cleanup EXIT HUP INT TERM

B="$TMP_ROOT/bench"
D="$B/ix89/stmt"
mkdir -p "$B/human-baseline/arm/proofs" \
         "$B/human-baseline/arm/curve25519" \
         "$B/hol-light-stamp2" "$D" || exit 99

# --- fixture upstream: one proof, one theory file, one ELF -----------------
printf 'needs "arm/proofs/base.ml";;\n' \
  > "$B/human-baseline/arm/proofs/bignum_add_p25519.ml"
printf '(* selftest fixture theory *)\n' \
  > "$B/human-baseline/arm/proofs/base.ml"
printf 'fixture-object-bignum-add-p25519\n' \
  > "$B/human-baseline/arm/curve25519/bignum_add_p25519.o"

( cd "$B/human-baseline" && git init -q && git add -A && \
  git -c user.email=selftest@invalid -c user.name=selftest commit -q --no-verify -m fixture ) \
  >/dev/null 2>&1 || { echo "SELFTEST_FAIL reason=fixture-git" >&2; exit 98; }

export TEKMERION_BENCH="$B"

PROV=$("$STMT" provenance add 2>&1) || {
  echo "SELFTEST_FAIL reason=fixture-provenance output=$PROV" >&2; exit 98; }

# --- a valid synthetic cache (sanity: must be accepted) --------------------
TERM='forall z x. P (z,x)'
HEX=$(printf '%s' "$TERM" | od -An -tx1 | tr -d ' \n')
CACHE_OK="$D/valid.stmt"
{
  echo "TASK arm.bignum_add_p25519"
  echo "THEOREM BIGNUM_ADD_P25519_CORRECT"
  echo "SOURCE arm/proofs/bignum_add_p25519.ml"
  echo "REPARSE 1"
  echo "STMT_LEN ${#TERM}"
  echo "STMT $HEX"
  printf '%s\n' "$PROV"
  echo "AXIOM 6f6e65"
  echo "AXIOM 74776f"
  echo "AXIOM 7468726565"
} > "$CACHE_OK"

if ! "$STMT" validate add "$CACHE_OK" >/dev/null 2>&1; then
  echo "SELFTEST_FAIL reason=valid-cache-rejected" >&2
  exit 98
fi
echo "SELFTEST_BASELINE_VALID cache=$CACHE_OK"

# --- hostile fixtures ------------------------------------------------------
OK_ADD='STATEMENT_OK task=arm.bignum_add_p25519 aconv=1 hyps=0 axioms=3'
OK_NEG='STATEMENT_OK task=arm.bignum_neg_p25519 aconv=1 hyps=0 axioms=3'
DONE_ADD='STATEMENT_CHECK_COMPLETE task=arm.bignum_add_p25519'
FAIL_ADD='STATEMENT_FAIL task=arm.bignum_add_p25519 aconv=0 hyps=0 axioms=3 axioms_ok=1 cached_ok=1 cache_errors='

printf '%s\n' "$OK_ADD" > "$D/no_complete.log"
printf '%s\n%s\n' "$OK_NEG" "$DONE_ADD" > "$D/wrong_task.log"
printf '%s\n%s\n%s\n' "$OK_ADD" "$OK_ADD" "$DONE_ADD" > "$D/two_ok.log"
printf '%s\n%s\n%s\n' "$OK_ADD" "$DONE_ADD" "$FAIL_ADD" > "$D/stray_fail.log"

CACHE_REPARSE0="$D/reparse0.stmt"
sed 's/^REPARSE 1$/REPARSE 0/' "$CACHE_OK" > "$CACHE_REPARSE0"
[ -s "$CACHE_REPARSE0" ] || { echo "SELFTEST_FAIL reason=fixture-reparse0" >&2; exit 98; }
if cmp -s "$CACHE_OK" "$CACHE_REPARSE0"; then
  echo "SELFTEST_FAIL reason=reparse-mutation-did-not-apply" >&2; exit 98
fi

# --- runner ----------------------------------------------------------------
TOTAL=0
REJECTED=0
BAD=0
SEEN=""
run_case () {
  local name=$1 expected=$2
  shift 2
  local out rc reason result
  out=$("$@" 2>&1)
  rc=$?
  reason=$(printf '%s\n' "$out" | sed -n 's/.*reason=\([^ ]*\).*/\1/p' | head -n 1)
  TOTAL=$((TOTAL + 1))
  if [ "$rc" -eq 0 ]; then
    result=ACCEPTED
    BAD=$((BAD + 1))
  elif [ "$reason" != "$expected" ]; then
    result=wrong-reason
    BAD=$((BAD + 1))
  else
    result=rejected
    REJECTED=$((REJECTED + 1))
  fi
  printf 'SELFTEST_CASE name=%s rc=%s reason=%s expected=%s result=%s\n' \
    "$name" "$rc" "${reason:-<none>}" "$expected" "$result"
  case " $SEEN " in
    *" $reason "*) BAD=$((BAD + 1)); echo "SELFTEST_DUPLICATE_REASON reason=$reason" ;;
  esac
  SEEN="$SEEN $reason"
}

run_case missing-completion-marker missing-completion-marker \
  "$STMT" verdict add "$D/no_complete.log"
run_case wrong-task wrong-task \
  "$STMT" verdict add "$D/wrong_task.log"
run_case cache-reparse-0 reparse-not-1 \
  "$STMT" validate add "$CACHE_REPARSE0"
run_case duplicate-STATEMENT_OK duplicate-STATEMENT_OK \
  "$STMT" verdict add "$D/two_ok.log"
run_case stray-STATEMENT_FAIL stray-STATEMENT_FAIL \
  "$STMT" verdict add "$D/stray_fail.log"

DISTINCT=$(printf '%s\n' $SEEN | grep -v '^$' | sort -u | wc -l | tr -d ' ')

printf 'SELFTEST_SUMMARY cases=%d rejected=%d bad=%d distinct_reasons=%s\n' \
  "$TOTAL" "$REJECTED" "$BAD" "$DISTINCT"

if [ "$TOTAL" -eq 5 ] && [ "$REJECTED" -eq 5 ] && [ "$BAD" -eq 0 ] && [ "$DISTINCT" -eq 5 ]; then
  echo "SELFTEST_OK 5/5 hostile cases rejected with distinct reasons"
  exit 0
fi
echo "SELFTEST_FAIL rejected=$REJECTED/5 bad=$BAD distinct=$DISTINCT" >&2
exit 1
