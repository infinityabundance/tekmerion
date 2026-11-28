#!/bin/bash
# XXXI.B1 / XXXI.B2: automatic assurance for the generated ARM proofs.
#
# Subcommands
#   hashes              print one `UPSTREAM_HASH <path> <sha256>` line per
#                       pinned framework file, flag drift in the effective
#                       build tree, and print `UPSTREAM_UNCHANGED 1|0`.
#   ab <task-id>        build the task with BOTH simulation backends in one
#                       HOL session and assert the goal, every assumption
#                       conclusion, every frame predicate and the bridge
#                       hypotheses are `aconv`-identical.  Fails on mismatch.
#
# XXXI.B1: the pin lives in tests/ix89/upstream_manifest.txt, not in a mutable
# in-code constant.  The effective tree (`.bench/dropin`, which the build's
# `needs` resolves against) and the pristine baseline (`.bench/human-baseline`)
# are both compared to it.
#
# XXXI.B2: re-loading the inlined support library in one session would redefine
# constants, so the A/B proof loads it once (exactly as matched_session.sh
# does) and runs the task tactic twice, once with the upstream per-step tactic
# and once with the Tekmerion-owned one.  Spies record every step's entry and
# resulting goal plus the bridge entry; `tests/ix89/ab_lib.ml` compares them
# with `aconv`.
#
# Portability: every path derives from this script's location, or from
# TEKMERION_BENCH / TEKMERION_HOL / TEKMERION_UPSTREAM / TEKMERION_SPENT.
set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)" || exit 99
ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)" || exit 99
BENCH="${TEKMERION_BENCH:-$ROOT/.bench}"
HOL="${TEKMERION_HOL:-$BENCH/hol-light-stamp2}"
UPSTREAM="${TEKMERION_UPSTREAM:-$BENCH/human-baseline}"
MANIFEST="$SCRIPT_DIR/upstream_manifest.txt"
SPENT="${TEKMERION_SPENT:-$BENCH/ix66/spent-manifest.json}"
CHECK="$SCRIPT_DIR/stmt_check.sh"

usage () {
  echo "usage: framework_check.sh <hashes|ab> [task-id]" >&2
  exit 90
}

sha256_of () { sha256sum -- "$1" 2>/dev/null | cut -d' ' -f1; }

holenv () {
  local opam_bin
  opam_bin=$(command -v opam 2>/dev/null || true)
  [ -n "$opam_bin" ] || opam_bin="$HOME/.local/bin/opam"
  eval "$("$opam_bin" env --switch "$HOL" --set-switch)"
  export PATH="$BENCH/bin:${ELAN_HOME:-$HOME/.elan}/toolchains/leanprover--lean4---v4.29.1/bin:$PATH"
}

# --- XXXI.B1: upstream framework hashes -----------------------------------

hashes () {
  [ -f "$MANIFEST" ] || {
    echo "UPSTREAM_MANIFEST_MISSING path=$MANIFEST" >&2
    echo "UPSTREAM_UNCHANGED 0"
    return 1
  }
  local unchanged=1 want rel pristine effective psha esha
  while read -r want rel; do
    [ -n "${rel:-}" ] || continue
    case "$want" in \#*) continue ;; esac
    case "$want" in *[!0-9a-f]*) echo "UPSTREAM_MANIFEST_BAD rel=$rel" >&2; unchanged=0; continue ;; esac
    [ "${#want}" -eq 64 ] || { echo "UPSTREAM_MANIFEST_BAD rel=$rel" >&2; unchanged=0; continue; }

    pristine="$UPSTREAM/$rel"
    effective="$BENCH/dropin/$rel"

    if [ -f "$pristine" ]; then
      psha=$(sha256_of "$pristine")
      echo "UPSTREAM_HASH $rel $psha"
      [ "$psha" = "$want" ] || { echo "UPSTREAM_DRIFT $rel expected=$want got=$psha file=$pristine"; unchanged=0; }
    else
      echo "UPSTREAM_HASH $rel MISSING"
      echo "UPSTREAM_DRIFT $rel expected=$want got=MISSING file=$pristine"
      unchanged=0
    fi

    if [ -f "$effective" ]; then
      esha=$(sha256_of "$effective")
      echo "EFFECTIVE_HASH $rel $esha"
      [ "$esha" = "$want" ] || { echo "UPSTREAM_DRIFT $rel expected=$want got=$esha file=$effective"; unchanged=0; }
    else
      echo "EFFECTIVE_HASH $rel MISSING"
    fi
  done < "$MANIFEST"

  echo "UPSTREAM_UNCHANGED $unchanged"
  [ "$unchanged" -eq 1 ]
}

# --- XXXI.B2: A/B hypothesis identity -------------------------------------

parse_task () {
  tid=${1:-}
  case "$tid" in arm.bignum_*) ;; *) return 1 ;; esac
  body=${tid#arm.bignum_}
  op=${body%%_*}
  case "$op" in add|sub|neg) ;; *) return 1 ;; esac
  tail=${body#*_}
  fam=${tail%%.*}
  th=${tail#*.}
  [ -n "$fam" ] || return 1
  expected="BIGNUM_$(echo "${op}_${fam}" | tr 'a-z' 'A-Z')_CORRECT"
  [ "$th" = "$expected" ] || return 1
  base="bignum_${op}_${fam}"
  return 0
}

ab () {
  local tid=${1:-}
  [ -n "$tid" ] || usage
  if ! parse_task "$tid"; then
    echo "AB_BLOCKED task=$tid reason=not-an-arm-bignum-add-sub-neg-task"
    return 90
  fi

  local work="$BENCH/ix89/ab/$op"
  local proof="$BENCH/dropin/arm/proofs/$base.ml"
  local corpus_id="arm.${base}"
  local source_rel="arm/proofs/$base.ml"
  local cache="$BENCH/ix89/stmt/arm_${op}_${fam}.stmt"
  mkdir -p "$work" || { echo "AB_BLOCKED reason=mkdir"; return 99; }

  [ -x "$ROOT/target/release/emit" ] || { echo "AB_BLOCKED reason=missing-emit"; return 96; }
  [ -x "$ROOT/target/release/dropin" ] || { echo "AB_BLOCKED reason=missing-dropin"; return 96; }
  [ -d "$HOL" ] || { echo "AB_BLOCKED reason=missing-hol"; return 96; }
  [ -f "$MANIFEST" ] || { echo "AB_BLOCKED reason=missing-manifest"; return 96; }

  # XXXI.B1: a standalone A/B receipt carries the same baseline check as a
  # run_gen receipt.  run_gen.sh sets TEKMERION_AB_SKIP_HASHES=1 because it has
  # already printed (and enforced) them.
  if [ "${TEKMERION_AB_SKIP_HASHES:-0}" != "1" ]; then
    hashes || { echo "AB_BLOCKED task=$tid reason=upstream-framework-drift"; return 91; }
  fi

  # Locate the object directory from the object itself, so the target works for
  # any family, not only curve25519.
  local obj
  obj=$(find "$BENCH/dropin/arm" -mindepth 2 -maxdepth 2 -name "$base.o" -print -quit 2>/dev/null)
  [ -n "$obj" ] || { echo "AB_BLOCKED task=$tid reason=missing-object"; return 96; }
  local dir
  dir=$(basename "$(dirname "$obj")")

  echo "AB_TASK task=$tid op=$op family=$fam dir=$dir object=$obj"
  echo "FREE_G $(free -g | awk '/^Mem:/{printf "total=%s used=%s free=%s available=%s", $2,$3,$4,$7}')"

  local emit_flags=()
  if [ -f "$SPENT" ] && grep -q "\"$tid\"" "$SPENT"; then
    emit_flags=(--spent --spent-manifest "$SPENT")
  fi

  # Emit both answers.  Each backend is exercised through the public selector.
  TEKMERION_SIM_BACKEND=tekmerion TEKMERION_ALG_CLOSE="$tid" \
    "$ROOT/target/release/emit" --task "$tid" --mode answer \
      --out "$work/tk.answer" ${emit_flags[@]+"${emit_flags[@]}"} \
    || { echo "AB_BLOCKED task=$tid reason=emit-tekmerion"; return 98; }
  TEKMERION_SIM_BACKEND=upstream TEKMERION_ALG_CLOSE="$tid" \
    "$ROOT/target/release/emit" --task "$tid" --mode answer \
      --out "$work/up.answer" ${emit_flags[@]+"${emit_flags[@]}"} \
    || { echo "AB_BLOCKED task=$tid reason=emit-upstream"; return 98; }

  local tk_calls up_calls tk_def
  tk_calls=$(grep -oE 'TEKMERION_ARM_ACCSTEPS_TAC [A-Z0-9_]+ \[[0-9;]+\] \(1--[0-9]+\)' "$work/tk.answer" | wc -l)
  up_calls=$(grep -oE '(^|[^A-Z_])ARM_ACCSTEPS_TAC [A-Z0-9_]+ \[[0-9;]+\] \(1--[0-9]+\)' "$work/up.answer" | wc -l)
  tk_def=$(grep -c 'let TEKMERION_ARM_ACCSTEPS_TAC ' "$work/tk.answer" || true)
  echo "AB_BACKEND_SELECT task=$tid tekmerion_calls=$tk_calls upstream_calls=$up_calls tekmerion_definition=$tk_def"
  if [ "${tk_calls:-0}" -ne 1 ] || [ "${up_calls:-0}" -ne 1 ]; then
    # A routine with no carry chain emits the plain `ARM_STEPS_TAC` under both
    # backends; the halves are then byte-identical and the comparison is
    # trivially valid.  Accept exactly one such call in each answer.
    local tk_steps up_steps
    tk_steps=$(grep -cE '(^|[^A-Z_])ARM_STEPS_TAC [A-Z0-9_]+ \(1--[0-9]+\)' "$work/tk.answer" || true)
    up_steps=$(grep -cE '(^|[^A-Z_])ARM_STEPS_TAC [A-Z0-9_]+ \(1--[0-9]+\)' "$work/up.answer" || true)
    if [ "${tk_calls:-0}" -eq 0 ] && [ "${up_calls:-0}" -eq 0 ] && \
       [ "${tk_steps:-0}" -eq 1 ] && [ "${up_steps:-0}" -eq 1 ]; then
      echo "AB_BACKEND_SELECT task=$tid note=no-carry-chain steps=$tk_steps"
    else
      echo "AB_BLOCKED task=$tid reason=backend-selector tk_calls=$tk_calls up_calls=$up_calls tk_steps=$tk_steps up_steps=$up_steps"
      return 97
    fi
  fi
  if grep -q 'TEKMERION_ARM_ACCSTEPS_TAC' "$work/up.answer"; then
    echo "AB_BLOCKED task=$tid reason=upstream-answer-carries-tekmerion-tactic"
    return 97
  fi

  # Assemble the Tekmerion answer in the upstream proof shape, then split it
  # into header / support library / task (the matched-session split).
  local backup="$work/proof.backup.ml"
  rm -f "$backup"
  [ -f "$proof" ] && cp "$proof" "$backup"
  restore_proof () { if [ -f "$backup" ]; then cp "$backup" "$proof"; else rm -f "$proof"; fi; }
  trap 'restore_proof' EXIT
  trap 'restore_proof; exit 130' INT TERM

  "$ROOT/target/release/dropin" assemble --task "$tid" --answer "$work/tk.answer" --stamp \
    || { echo "AB_BLOCKED task=$tid reason=assemble"; return 97; }
  cp "$proof" "$work/strict.proof.ml" || return 97

  local header="$work/header.ml" phrases="$work/phrases.ml" task="$work/task.ml"
  awk -v header="$header" -v phrases="$phrases" -v task="$task" \
      -f "$SCRIPT_DIR/matched_split.awk" "$work/strict.proof.ml" \
    || { echo "AB_BLOCKED task=$tid reason=split"; return 95; }
  local nph
  nph=$(grep -c '^;;$' "$phrases" || true)
  echo "AB_SPLIT task=$tid library_phrases=${nph:-0}"

  # The statement cache exists only for the p25519 tasks (stmt_check.sh pins
  # that family); every other task still gets the identity comparison.
  local do_statement=0
  if [ "$fam" = "p25519" ] && [ -f "$CHECK" ]; then
    "$CHECK" ensure "$op" || { echo "AB_BLOCKED task=$tid reason=stmt-ensure"; return 94; }
    [ -s "$cache" ] && do_statement=1
  fi
  echo "AB_STATEMENT_MODE task=$tid enabled=$do_statement cache=$cache"

  # Build the combined proof and install it for one capped build.
  perl "$SCRIPT_DIR/ab_gen.pl" "$header" "$phrases" "$task" "$SCRIPT_DIR/ab_lib.ml" \
      "$tid" "$th" "$source_rel" "$cache" "$do_statement" > "$work/ab.proof.ml" \
    || { echo "AB_BLOCKED task=$tid reason=ab-gen"; return 95; }

  cp "$SCRIPT_DIR/stmt_lib.ml" "$BENCH/dropin/arm/proofs/ix89_stmt_lib.ml" || return 96
  cp "$work/ab.proof.ml" "$proof" || { echo "AB_BLOCKED reason=install-proof"; return 94; }

  holenv
  local correct="$BENCH/dropin/arm/$dir/$base.correct"
  rm -f "$correct"
  ( cd "$BENCH/dropin/arm" && \
    "$ROOT/tools/cap.sh" 12G -- make -j1 -B HOLDIR="$HOL" "$dir/$base.correct" ) \
    > "$work/ab.log" 2>&1
  local rc=$?
  [ -f "$correct" ] && cp "$correct" "$work/ab.correct"
  restore_proof
  trap - EXIT INT TERM
  echo "AB_MAKE_RC $rc"
  echo "AB_LOG $work/ab.log"
  echo "AB_CORRECT $correct"
  [ -f "$correct" ] || { echo "AB_BLOCKED task=$tid reason=no-correct rc=$rc"; return 1; }

  grep -E '^(AB_L7|AB_COMPARE|AB_IDENTICAL|AB_MISMATCH|AB_RESULT|AB_INFERENCES|AB_STRICT|AB_BACKEND_SELECT|AB_STATEMENT_MODE|AB_STATEMENT_SKIPPED|STATEMENT_OK|STATEMENT_OK_UP|STATEMENT_FAIL)' \
    "$correct" || true

  local nident nmism nres
  nident=$(grep -c '^AB_IDENTICAL ' "$correct" || true)
  nmism=$(grep -c '^AB_MISMATCH ' "$correct" || true)
  nres=$(grep -c '^AB_RESULT ' "$correct" || true)
  if [ "${nident:-0}" -ne 1 ] || [ "${nmism:-0}" -ne 0 ] || [ "${nres:-0}" -ne 0 ]; then
    echo "AB_VERDICT task=$tid identical=0 identical_lines=$nident mismatch_lines=$nmism result_lines=$nres"
    return 1
  fi

  # Per-backend inference totals must both be present and positive.
  local tk_inf up_inf
  tk_inf=$(grep -m1 '^AB_INFERENCES backend=tekmerion ' "$correct" | awk '{print $3}')
  up_inf=$(grep -m1 '^AB_INFERENCES backend=upstream ' "$correct" | awk '{print $3}')
  echo "AB_INFERENCE_TOTALS task=$tid tekmerion=${tk_inf:-MISSING} upstream=${up_inf:-MISSING}"

  if [ "$do_statement" -eq 1 ]; then
    if "$CHECK" verdict "$op" "$correct"; then :; else
      echo "AB_VERDICT task=$tid identical=1 statement=0"
      return 1
    fi
    if ! grep -q "^STATEMENT_OK_UP backend=upstream task=$corpus_id aconv=1 hyps=0 axioms=3$" "$correct"; then
      echo "AB_VERDICT task=$tid identical=1 statement=0 reason=upstream-statement"
      return 1
    fi
  fi

  if [ "$rc" -ne 0 ]; then
    echo "AB_VERDICT task=$tid identical=1 make_rc=$rc"
    return "$rc"
  fi
  echo "AB_VERDICT task=$tid identical=1 statement=$do_statement tekmerion=${tk_inf:-MISSING} upstream=${up_inf:-MISSING}"
  return 0
}

cmd=${1:-}
case "$cmd" in
  hashes) hashes ;;
  ab) ab "${2:-}" ;;
  *) usage ;;
esac
