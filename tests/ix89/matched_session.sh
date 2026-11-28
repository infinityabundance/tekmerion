#!/bin/bash
# XXX.E: measured matched-session total for arm/{add,sub,neg}_p25519.
#
# Definition (TODO XXIII.P3 / XXIX.E): the support library is built and
# kernel-checked *before* the counter starts; its cost is recorded once as L7;
# then the counter is reset and the task-only proof is counted.  The task
# theorem is still produced and the same-session statement check still runs.
#
# One HOL session, one task at a time, under `tools/cap.sh 12G -- make -j1`,
# cap never raised.  Nothing here touches the strict path: the emitted answer,
# `tools/bridge.ml`, `src/` and the fixtures are read-only.  Only the generated
# proof file `.bench/dropin/arm/proofs/bignum_<op>_p25519.ml` is replaced for
# the duration of the run and restored afterwards.
#
# usage: matched_session.sh <add|sub|neg> [--reuse]
#
#   --reuse  skip emit+assemble and reuse <work>/arm_<op>_p25519.strict.proof.ml
#
# Outputs (under $BENCH/ix89/matched/):
#   arm_<op>_p25519.raw.answer        the emitted answer (identical to the
#                                     plain run's raw.answer)
#   arm_<op>_p25519.strict.proof.ml   the strict generated proof (pre-split)
#   arm_<op>_p25519.header.ml         needs + ELF/EXEC/modulus setup
#   arm_<op>_p25519.phrases.ml        the support library as top-level phrases
#   arm_<op>_p25519.library.ml        header + library-only; prints MATCHED_L7
#   arm_<op>_p25519.task.ml           task-only prove call (query + tactic)
#   arm_<op>_p25519.matched.ml        library + task in one session (what runs)
#   arm_<op>_p25519.correct           stdout of the matched run
#   arm_<op>_p25519.log               make log
set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)" || exit 99
ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)" || exit 99
BENCH="${TEKMERION_BENCH:-$ROOT/.bench}"
HOL="${TEKMERION_HOL:-$BENCH/hol-light-stamp2}"
cd "$ROOT" || exit 99

op=${1:-}
case "$op" in add|sub|neg) ;; *) echo "usage: matched_session.sh <add|sub|neg> [--reuse]" >&2; exit 90 ;; esac
reuse=0
[ "${2:-}" = "--reuse" ] && reuse=1

arch=arm dir=curve25519 fam=p25519
base="bignum_${op}_${fam}"
th="BIGNUM_$(echo "${op}_${fam}" | tr 'a-z' 'A-Z')_CORRECT"
tid="${arch}.${base}.${th}"
tag="${arch}_${op}_${fam}"
work="$BENCH/ix89/matched"
proof="$BENCH/dropin/$arch/proofs/${base}.ml"
correct="$BENCH/dropin/$arch/$dir/${base}.correct"
check="$SCRIPT_DIR/stmt_check.sh"
mkdir -p "$work" || exit 99

OPAM_BIN=$(command -v opam 2>/dev/null || true)
[ -n "$OPAM_BIN" ] || OPAM_BIN="$HOME/.local/bin/opam"
eval "$("$OPAM_BIN" env --switch "$HOL" --set-switch)"
export PATH="$BENCH/bin:${ELAN_HOME:-$HOME/.elan}/toolchains/leanprover--lean4---v4.29.1/bin:$PATH"

log () { printf '%s\n' "$*"; }

log "MATCHED_SESSION task=$tid"
log "FREE_G $(free -g | awk '/^Mem:/{printf "total=%s used=%s free=%s available=%s", $2,$3,$4,$7}')"
log "HOL=$HOL"

# --- 0. prerequisites -----------------------------------------------------
[ -x ./target/release/emit ]   || { log "MATCHED_BLOCKED reason=missing-emit"; exit 96; }
[ -x ./target/release/dropin ] || { log "MATCHED_BLOCKED reason=missing-dropin"; exit 96; }
[ -d "$HOL" ]                  || { log "MATCHED_BLOCKED reason=missing-hol"; exit 96; }

# --- 1. emit the strict answer (same flags as run_gen.sh's plain path) -----
if [ "$reuse" -eq 0 ]; then
  TEKMERION_ALG_CLOSE="$tid" ./target/release/emit --task "$tid" --mode phase \
    --out "$work/${tag}.raw.answer" --spent \
    --spent-manifest "$BENCH/ix66/spent-manifest.json" || { log "MATCHED_BLOCKED reason=emit rc=$?"; exit 98; }
fi
[ -s "$work/${tag}.raw.answer" ] || { log "MATCHED_BLOCKED reason=missing-raw-answer"; exit 98; }
log "RAW_ANSWER_SHA256 $(sha256sum "$work/${tag}.raw.answer" | awk '{print $1}')"

# --- 2. assemble the strict generated proof -------------------------------
backup="$work/${tag}.proof.backup.ml"
rm -f "$backup"
if [ -f "$proof" ]; then cp "$proof" "$backup" || exit 97; fi
restore_proof () { [ -f "$backup" ] && cp "$backup" "$proof"; }
trap 'restore_proof' EXIT
trap 'restore_proof; exit 130' INT TERM

if [ "$reuse" -eq 0 ]; then
  ./target/release/dropin assemble --task "$tid" --answer "$work/${tag}.raw.answer" --stamp \
    || { log "MATCHED_BLOCKED reason=assemble"; exit 97; }
  cp "$proof" "$work/${tag}.strict.proof.ml" || exit 97
fi
[ -s "$work/${tag}.strict.proof.ml" ] || { log "MATCHED_BLOCKED reason=missing-strict-proof"; exit 97; }

# --- 3. split at the library/task boundary --------------------------------
awk -v header="$work/${tag}.header.ml" \
    -v phrases="$work/${tag}.phrases.ml" \
    -v task="$work/${tag}.task.ml" \
    -f "$SCRIPT_DIR/matched_split.awk" "$work/${tag}.strict.proof.ml" \
  || { log "MATCHED_BLOCKED reason=split"; exit 95; }
nph=$(grep -c '^;;$' "$work/${tag}.phrases.ml" || true)
log "SPLIT library_phrases=${nph:-0}"

# The task-only file must contain no library phrase and start with the prove.
if ! head -n1 "$work/${tag}.task.ml" | grep -Eq '^let [A-Z0-9_]+ = time prove$'; then
  log "MATCHED_BLOCKED reason=task-head-not-prove"; exit 95
fi
if grep -q 'Reading and writing DIMACS' "$work/${tag}.task.ml"; then
  log "MATCHED_BLOCKED reason=library-leaked-into-task"; exit 95
fi

# --- 4. assemble library-only, task-only and matched session --------------
{
  cat "$work/${tag}.header.ml"
  printf 'let ix89_lib_before = inference_count ();;\n'
  cat "$work/${tag}.phrases.ml"
  printf 'Printf.printf "MATCHED_LIB_AFTER %%d\\n%%!" (inference_count ());;\n'
  printf 'Printf.printf "MATCHED_L7 %%d\\n%%!" (inference_count () - ix89_lib_before);;\n'
} > "$work/${tag}.library.ml" || exit 95

{
  printf 'let ix89_task_before = inference_count ();;\n'
  cat "$work/${tag}.task.ml"
  printf 'Printf.printf "MATCHED_TASK %%d\\n%%!" (inference_count () - ix89_task_before);;\n'
  printf 'Printf.printf "MATCHED_L7 %%d\\n%%!" (ix89_task_before - ix89_lib_before);;\n'
  printf 'Printf.printf "MATCHED_TOTAL %%d\\n%%!" (inference_count () - ix89_lib_before);;\n'
  printf 'Printf.printf "STAMP_INF_BEFORE %%d\\n%%!" ix89_task_before;;\n'
  printf 'Printf.printf "STAMP_INF_AFTER %%d\\n%%!" (inference_count ());;\n'
  printf 'Printf.printf "STAMP_INFERENCES %%d\\n%%!" (inference_count () - ix89_task_before);;\n'
  printf 'List.iter (fun (id,dg,txt) -> Printf.printf "STAMP_REGISTRY %%d %%s %%s\\n%%!" id dg txt) (axiom_registry ());;\n'
  printf 'Printf.printf "val %s : thm = %%s\\n%%!" (string_of_thm %s);;\n' "$th" "$th"
} > "$work/${tag}.taskrun.ml" || exit 95

cat "$work/${tag}.library.ml" "$work/${tag}.taskrun.ml" > "$work/${tag}.matched.ml" || exit 95

# --- 5. statement-check cache (validated, rebuilt only if invalid) --------
"$check" ensure "$op" || { log "MATCHED_BLOCKED reason=stmt-ensure rc=$?"; exit 94; }

# --- 6. install the matched session at the generated proof path, inject ---
cp "$work/${tag}.matched.ml" "$proof" || { log "MATCHED_BLOCKED reason=install-proof"; exit 94; }
"$check" inject "$op" || { log "MATCHED_BLOCKED reason=stmt-inject rc=$?"; exit 93; }

# --- 7. run: one session, capped at 12G, -j1 ------------------------------
rm -f "$correct"
( cd "$BENCH/dropin/$arch" && \
  "$ROOT/tools/cap.sh" 12G -- make -j1 -B HOLDIR="$HOL" "${dir}/${base}.correct" ) \
  > "$work/${tag}.log" 2>&1
rc=$?
log "MAKE_RC $rc"
[ -f "$correct" ] && cp "$correct" "$work/${tag}.correct"

# --- 8. verdicts ----------------------------------------------------------
"$check" verdict "$tag" "$work/${tag}.correct"
sc=$?
log "STATEMENT_RC $sc"

log "MATCHED_RESULT task=$tid l7=$(grep -m1 '^MATCHED_L7 ' "$work/${tag}.correct" 2>/dev/null | awk '{print $2}')" \
    "task=$(grep -m1 '^MATCHED_TASK ' "$work/${tag}.correct" 2>/dev/null | awk '{print $2}')" \
    "total=$(grep -m1 '^MATCHED_TOTAL ' "$work/${tag}.correct" 2>/dev/null | awk '{print $2}')" \
    "strict=$(grep -m1 '^STAMP_INFERENCES ' "$work/${tag}.correct" 2>/dev/null | awk '{print $2}')"

[ "$rc" -eq 0 ] || exit "$rc"
exit "$sc"
