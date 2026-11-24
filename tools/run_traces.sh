#!/usr/bin/env bash
# run_traces.sh -- build or reuse one instrumented s2n-bignum proof and run it.
#
# TacticTrace's own build-proof.sh only compiles the proof binary; it never
# executes it, and it pre-creates the dump directory that exptrace_dump then
# tries to Sys.mkdir. This runner executes the binary and guarantees that the
# success directory does not exist until the binary itself creates it.
#
# Usage:
#   tools/run_traces.sh --success-dir DIR [options] THEOREM
#   tools/run_traces.sh --success-dir DIR [options] --proof PATH
#
# Options:
#   --arch arm|x86        architecture (default: arm)
#   --success-dir DIR     where the success-channel <TAC>.json files go (required)
#   --failure-dir DIR     where the failure journals go
#                         (default: $TEKMERION_FAILURE_DIR, else <success>.failures)
#   --attempt-dir DIR     where <attempt_id>.jsonl and its manifest go
#                         (default: <success>.attempts; patched runs only)
#   --proof PATH          run this proof file instead of an s2n-bignum theorem
#   --timeout SECONDS     stop a patched run after SECONDS and record the
#                         terminal outcome as `timeout` (default: no limit)
#   --work-dir DIR        build directory (default: .bench/.runner/<arch>-<name>-<mode>)
#   --reuse               reuse an existing binary only if it was built for the
#                         same proof, injection, mode, wrapper, attempt and
#                         success dir
#   --stock               unpatched TacticTrace wrapper, no failure module
#   --inject TACTIC       insert `<TACTIC> THEN` at the start of the first proof
#   --force               remove an existing non-empty success/failure directory
#   -h, --help            show this help
#
# Exit status: 0 only when the proof ran to completion and wrote at least one
# success JSON file. A failed proof exits non-zero, ends the attempt stream
# with outcome `failed`, and leaves its failure journals in place.
#
# --proof takes a path to a proof file. Relative paths are resolved against the
# repository root, the s2n-bignum root, then .bench, exactly like a theorem
# name that contains a slash.

set -u
set -o pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BENCH="$ROOT/.bench"
HOL="$BENCH/hol-light"
TT="$HOL/TacticTrace"
S2N="$BENCH/s2n-bignum"
INJECT_SRC="$ROOT/src/tools/run_traces_inject.rs"
PATCHER_SRC="$ROOT/src/tools/patch_wrapper.rs"
MODULE="$ROOT/trace/exptrace_failure.ml"
ATTEMPT_ID_SRC="$ROOT/src/tools/attempt_id.rs"
ATTEMPT_LIFECYCLE_SRC="$ROOT/src/tools/attempt_lifecycle.rs"
INJECT=(cargo run --offline --quiet --manifest-path "$ROOT/Cargo.toml" --bin run_traces_inject --)
PATCHER=(cargo run --offline --quiet --manifest-path "$ROOT/Cargo.toml" --bin patch_wrapper --)
ATTEMPT_ID_TOOL=(cargo run --offline --quiet --manifest-path "$ROOT/Cargo.toml" --bin attempt_id --)
ATTEMPT_LIFECYCLE=(cargo run --offline --quiet --manifest-path "$ROOT/Cargo.toml" --bin attempt_lifecycle --)

usage() {
  sed -n '3,44p' "${BASH_SOURCE[0]}"
}

die() {
  printf 'run_traces: error: %s\n' "$*" >&2
  exit 2
}

arch=arm
theorem=
proof=
success_dir=
failure_dir=
attempt_dir=
work_dir=
timeout_s=0
reuse=false
stock=false
inject_text=
force=false

while [ "$#" -gt 0 ]; do
  case "$1" in
    --arch)
      [ "$#" -ge 2 ] || die "--arch needs a value"
      arch="$2"; shift 2 ;;
    --success-dir)
      [ "$#" -ge 2 ] || die "--success-dir needs a value"
      success_dir="$2"; shift 2 ;;
    --failure-dir)
      [ "$#" -ge 2 ] || die "--failure-dir needs a value"
      failure_dir="$2"; shift 2 ;;
    --attempt-dir)
      [ "$#" -ge 2 ] || die "--attempt-dir needs a value"
      attempt_dir="$2"; shift 2 ;;
    --proof)
      [ "$#" -ge 2 ] || die "--proof needs a value"
      proof="$2"; shift 2 ;;
    --timeout)
      [ "$#" -ge 2 ] || die "--timeout needs a value"
      timeout_s="$2"; shift 2 ;;
    --work-dir)
      [ "$#" -ge 2 ] || die "--work-dir needs a value"
      work_dir="$2"; shift 2 ;;
    --inject)
      [ "$#" -ge 2 ] || die "--inject needs a value"
      inject_text="$2"; shift 2 ;;
    --reuse) reuse=true; shift ;;
    --stock) stock=true; shift ;;
    --force) force=true; shift ;;
    -h|--help) usage; exit 0 ;;
    --*) die "unknown option: $1" ;;
    *)
      if [ -n "$theorem" ] || [ -n "$proof" ]; then
        die "give exactly one theorem name or --proof PATH"
      fi
      theorem="$1"; shift ;;
  esac
done

[ "$arch" = arm ] || [ "$arch" = x86 ] || die "--arch must be arm or x86"
[ -n "$success_dir" ] || die "--success-dir is required"
case "$timeout_s" in
  ''|*[!0-9]*) die "--timeout must be a nonnegative integer" ;;
esac

resolve_path() {
  candidate="$1"
  if [ -f "$candidate" ]; then
    printf '%s\n' "$candidate"
    return 0
  fi
  for base in "$ROOT" "$S2N" "$BENCH"; do
    if [ -f "$base/$candidate" ]; then
      printf '%s\n' "$base/$candidate"
      return 0
    fi
  done
  return 1
}

if [ -n "$proof" ] && [ -n "$theorem" ]; then
  die "give exactly one theorem name or --proof PATH"
fi

if [ -n "$proof" ]; then
  resolved="$(resolve_path "$proof")" || die "proof not found: $proof"
elif [ -n "$theorem" ]; then
  case "$theorem" in
    *.ml|*/*)
      resolved="$(resolve_path "$theorem")" || die "proof not found: $theorem" ;;
    *)
      resolved="$(resolve_path "$arch/proofs/$theorem.ml")" ||
        die "theorem not found: $theorem" ;;
  esac
else
  die "give a theorem name (e.g. word_max) or --proof PATH"
fi

proof="$(cd "$(dirname "$resolved")" && pwd)/$(basename "$resolved")"
name="$(basename "$proof" .ml)"
success_dir="$(realpath -m "$success_dir")"

if [ -z "$failure_dir" ]; then
  if [ -n "${TEKMERION_FAILURE_DIR:-}" ]; then
    failure_dir="$TEKMERION_FAILURE_DIR"
  else
    failure_dir="${success_dir}.failures"
  fi
fi
failure_dir="$(realpath -m "$failure_dir")"

if [ "$stock" = true ]; then mode=stock; else mode=patched; fi

if [ -z "$attempt_dir" ]; then
  attempt_dir="${success_dir}.attempts"
fi
attempt_dir="$(realpath -m "$attempt_dir")"

if [ -z "$work_dir" ]; then
  work_dir="$BENCH/.runner/${arch}-${name}-${mode}"
fi
work_dir="$(realpath -m "$work_dir")"
mkdir -p "$work_dir" || die "cannot create work directory: $work_dir"

[ -x "$HOL/hol.sh" ] || die "missing $HOL/hol.sh"
[ -x "$TT/tracer" ] || die "missing $TT/tracer (build TacticTrace first)"
[ -s "$TT/kernel_wrapper.ml" ] || die "missing $TT/kernel_wrapper.ml"
[ -s "$TT/exportTrace.ml" ] || die "missing $TT/exportTrace.ml"
[ -s "$TT/hol_lib_inlined.mli" ] || die "missing $TT/hol_lib_inlined.mli"
[ -s "$MODULE" ] || die "missing $MODULE"
[ -f "$S2N/$arch/proofs/base.ml" ] || die "missing $S2N/$arch/proofs/base.ml"
[ -f "$INJECT_SRC" ] || die "missing $INJECT_SRC"
[ -f "$PATCHER_SRC" ] || die "missing $PATCHER_SRC"
[ -f "$ATTEMPT_ID_SRC" ] || die "missing $ATTEMPT_ID_SRC"
[ -f "$ATTEMPT_LIFECYCLE_SRC" ] || die "missing $ATTEMPT_LIFECYCLE_SRC"

export HOLLIGHT_DIR="$HOL"
export TACLOGGER_DIR="$TT"
if [ -d "$HOL/_opam" ]; then
  command -v opam >/dev/null 2>&1 || die "opam not found"
  eval "$(opam env --switch "$HOL" --set-switch)"
fi

prepare_fresh_dir() {
  target="$1"
  label="$2"
  case "$target" in
    ""|/) die "refusing to use '$target' as a $label directory" ;;
  esac
  if [ -e "$target" ]; then
    if [ -d "$target" ] && [ -z "$(ls -A "$target")" ]; then
      rmdir "$target" || die "cannot remove empty $label directory: $target"
    elif [ "$force" = true ]; then
      rm -rf -- "$target" || die "cannot remove $label directory: $target"
    else
      die "$label directory already exists and is not empty: $target (use --force)"
    fi
  fi
  mkdir -p "$(dirname "$target")" ||
    die "cannot create parent of $label directory: $target"
}

build_proof="$proof"
if [ -n "$inject_text" ]; then
  build_proof="$work_dir/proof.injected.ml"
  "${INJECT[@]}" "$proof" "$build_proof" "$inject_text" \
    > "$work_dir/inject.log" 2>&1 ||
    { cat "$work_dir/inject.log" >&2; die "injection failed"; }
fi

proof_hash="$(sha256sum "$proof" | awk '{print $1}')"
wrapper_hash="$(sha256sum "$TT/kernel_wrapper.ml" | awk '{print $1}')"
module_hash="$(sha256sum "$MODULE" | awk '{print $1}')"
patcher_hash="$(sha256sum "$PATCHER_SRC" | awk '{print $1}')"
inject_helper_hash="$(sha256sum "$INJECT_SRC" | awk '{print $1}')"
lifecycle_hash="$(sha256sum "$ATTEMPT_LIFECYCLE_SRC" | awk '{print $1}')"

case "$proof" in
  "$S2N"/*) theorem_id="${proof#"$S2N"/}" ;;
  *) theorem_id="$proof" ;;
esac

attempt_id=
if [ "$mode" = patched ]; then
  attempt_id="$(
    "${ATTEMPT_ID_TOOL[@]}" \
      --theorem-id "$theorem_id" \
      --proof "$proof" \
      --inject "$inject_text" \
      --wrapper-sha "$wrapper_hash" \
      --module-sha "$module_hash"
  )" || die "attempt_id failed"
  case "$attempt_id" in
    *[!0-9a-f]*|'')
      die "attempt_id produced an unexpected id: $attempt_id" ;;
  esac
  [ "${#attempt_id}" -eq 64 ] ||
    die "attempt_id did not produce a 64-character digest: $attempt_id"
fi

build_key="$(
  printf '%s\n%s\n%s\n%s\n%s\n%s\n%s\n%s\n%s\n%s\n%s\n%s\n' \
    "$proof_hash" "$inject_text" "$mode" "$wrapper_hash" "$module_hash" \
    "$patcher_hash" "$inject_helper_hash" "$success_dir" "$attempt_id" \
    "$attempt_dir" "$timeout_s" "$lifecycle_hash" |
    sha256sum | awk '{print $1}'
)"

native="$work_dir/proof.native"
reuse_build=false
if [ "$reuse" = true ] && [ -x "$native" ] && [ -f "$work_dir/.build-key" ] &&
   [ "$(cat "$work_dir/.build-key")" = "$build_key" ]; then
  reuse_build=true
fi

if [ "$reuse_build" = true ]; then
  printf 'reusing binary: %s\n' "$native"
else
  rm -f -- "$native" "$work_dir/proof.cmx" "$work_dir/wrapped.ml" \
    "$work_dir/wrapped.org" "$work_dir/wrapped.org.scaffolded"

  template="$work_dir/template.ml"
  inlined="$work_dir/inlined.ml"

  if [ "$mode" = patched ]; then
    cat > "$template" <<EOF
let rt_start = Unix.time();;
(* TEKMERION_ATTEMPT_START *)
(* TEKMERION_ATTEMPT_TIMEOUT *)
loadt "$build_proof";;
check_axioms ();;
(* TEKMERION_ATTEMPT_END *)
Printf.printf "RUN_TRACES_PROOF_OK\\n";;
let rt_end = Unix.time();;
Printf.printf "RUN_TRACES_ELAPSED %f s\\n" (rt_end -. rt_start);;
EOF
  else
    cat > "$template" <<EOF
let rt_start = Unix.time();;
loadt "$build_proof";;
check_axioms ();;
Printf.printf "RUN_TRACES_PROOF_OK\\n";;
let rt_end = Unix.time();;
Printf.printf "RUN_TRACES_ELAPSED %f s\\n" (rt_end -. rt_start);;
EOF
  fi

  printf 'inline-loading %s\n' "$build_proof"
  ( cd "$S2N" && ocaml "$HOL/inline_load.ml" "$template" "$inlined" ) \
    > "$work_dir/inline.log" 2>&1 ||
    { tail -40 "$work_dir/inline.log" >&2; die "inline_load failed"; }
  [ -s "$inlined" ] || die "inline_load produced no output"

  nolinedir="$work_dir/inlined.nolinedir.ml"
  python3 "$TT/remove-linenum-dirs.py" "$inlined" "$nolinedir" ||
    die "remove-linenum-dirs.py failed"

  ast="${nolinedir%.ml}.marshalled.bin"
  mli="${nolinedir%.ml}.mli"
  printf 'collecting the AST\n'
  "$TT/get-ast.sh" "$nolinedir" > "$work_dir/get-ast.log" 2>&1 ||
    { tail -40 "$work_dir/get-ast.log" >&2; die "get-ast.sh failed"; }
  [ -s "$ast" ] && [ -s "$mli" ] || die "get-ast.sh did not produce $ast and $mli"

  "$TT/tracer" modify "$inlined" "$ast" "$mli" "$TT/hol_lib_inlined.mli" \
    "$work_dir/wrapped.org" > "$work_dir/tracer.log" 2>&1 ||
    { tail -40 "$work_dir/tracer.log" >&2; die "tracer modify failed"; }
  [ -s "$work_dir/wrapped.org" ] || die "tracer modify produced no output"

  if [ "$mode" = patched ]; then
    "${ATTEMPT_LIFECYCLE[@]}" "$work_dir/wrapped.org" \
      "$work_dir/wrapped.org.scaffolded" > "$work_dir/lifecycle.log" 2>&1 ||
      { cat "$work_dir/lifecycle.log" >&2; die "attempt_lifecycle failed"; }
    wrapped_org="$work_dir/wrapped.org.scaffolded"
  else
    wrapped_org="$work_dir/wrapped.org"
  fi

  if [ "$mode" = stock ]; then
    wrapper="$TT/kernel_wrapper.ml"
  else
    wrapper="$work_dir/kernel_wrapper.patched.ml"
    "${PATCHER[@]}" "$TT/kernel_wrapper.ml" "$wrapper" \
      > "$work_dir/patch.log" 2>&1 ||
      { cat "$work_dir/patch.log" >&2; die "patch_wrapper failed"; }
  fi

  wrapped="$work_dir/wrapped.ml"
  {
    head -2 "$wrapped_org"
    echo "(* --- exportTrace.ml --- *)"
    cat "$TT/exportTrace.ml"
    if [ "$mode" = patched ]; then
      echo "(* --- exptrace_failure.ml --- *)"
      cat "$MODULE"
    fi
    echo "(* --- kernel_wrapper.ml --- *)"
    cat "$wrapper"
    echo ""
    tail -n +3 "$wrapped_org"
    echo "exptrace_dump \"$success_dir\";;"
  } > "$wrapped"

  printf 'compiling %s\n' "$wrapped"
  ( cd "$work_dir" && OCAMLRUNPARAM=l=2000000000 ocamlopt.byte \
      -pp "$("$HOL/hol.sh" -pp)" -I "$HOL" -I +unix -c \
      hol_lib.cmxa "$wrapped" -o "$work_dir/proof.cmx" -w -a ) \
    > "$work_dir/compile.log" 2>&1 ||
    { tail -40 "$work_dir/compile.log" >&2; die "compile failed"; }

  printf 'linking %s\n' "$native"
  ( cd "$work_dir" && ocamlfind ocamlopt -package zarith,unix -linkpkg \
      hol_lib.cmxa -I "$HOL" "$work_dir/proof.cmx" -o "$native" ) \
    > "$work_dir/link.log" 2>&1 ||
    { tail -40 "$work_dir/link.log" >&2; die "link failed"; }

  printf '%s\n' "$build_key" > "$work_dir/.build-key"
fi

prepare_fresh_dir "$success_dir" "success"
prepare_fresh_dir "$failure_dir" "failure"
if [ "$mode" = patched ]; then
  prepare_fresh_dir "$attempt_dir" "attempt"
fi

printf 'running %s\n' "$native"
if [ "$mode" = patched ]; then
  ( cd "$S2N" &&
    TEKMERION_FAILURE_DIR="$failure_dir" \
    TEKMERION_ATTEMPT_DIR="$attempt_dir" \
    TEKMERION_ATTEMPT_ID="$attempt_id" \
    TEKMERION_THEOREM_ID="$theorem_id" \
    TEKMERION_PROOF_SHA="$proof_hash" \
    TEKMERION_INJECTION="$inject_text" \
    TEKMERION_WRAPPER_SHA="$wrapper_hash" \
    TEKMERION_MODULE_SHA="$module_hash" \
    TEKMERION_TIMEOUT_S="$timeout_s" \
    "$native" ) > "$work_dir/run.log" 2>&1
else
  ( cd "$S2N" && TEKMERION_FAILURE_DIR="$failure_dir" "$native" ) \
    > "$work_dir/run.log" 2>&1
fi
run_status=$?

success_count=0
if [ -d "$success_dir" ]; then
  success_count="$(find "$success_dir" -maxdepth 1 -type f -name '*.json' | wc -l)"
fi

journal_count=0
journal_lines=0
if [ -d "$failure_dir" ]; then
  journal_count="$(find "$failure_dir" -maxdepth 1 -type f -name '*.jsonl' | wc -l)"
  journal_lines="$(
    find "$failure_dir" -maxdepth 1 -type f -name '*.jsonl' -exec cat {} + |
      wc -l
  )"
fi

printf 'success JSON files: %s\n' "$success_count"
printf 'failure journals:   %s (%s lines)\n' "$journal_count" "$journal_lines"
printf 'success dir:        %s\n' "$success_dir"
printf 'failure dir:        %s\n' "$failure_dir"
if [ "$mode" = patched ]; then
  stream_path="$attempt_dir/$attempt_id.jsonl"
  manifest_path="$attempt_dir/$attempt_id.manifest.json"
  stream_bytes=0
  if [ -f "$stream_path" ]; then
    stream_bytes="$(wc -c < "$stream_path")"
  fi
  printf 'attempt id:         %s\n' "$attempt_id"
  printf 'attempt stream:     %s (%s bytes)\n' "$stream_path" "$stream_bytes"
  printf 'attempt manifest:   %s\n' "$manifest_path"
  printf 'attempt dir:        %s\n' "$attempt_dir"
fi
printf 'run log:            %s\n' "$work_dir/run.log"

if [ "$run_status" -ne 0 ]; then
  tail -40 "$work_dir/run.log" >&2
  printf 'run_traces: proof exited %s; failure journals (if any) are in %s\n' \
    "$run_status" "$failure_dir" >&2
  exit "$run_status"
fi

if ! grep -q 'RUN_TRACES_PROOF_OK' "$work_dir/run.log"; then
  tail -40 "$work_dir/run.log" >&2
  die "proof did not report RUN_TRACES_PROOF_OK"
fi

if [ "$success_count" -eq 0 ]; then
  die "proof succeeded but wrote no success JSON under $success_dir"
fi

printf 'run_traces: OK\n'
exit 0
