#!/bin/bash
# XXVIII.A / XXIX.A: statement / hypothesis / axiom check on the IX.89 path.
#
# The check runs in the same HOL session that produces the generated theorem:
#
#   aconv (concl th) stmt      -- stmt cached from the upstream `_CORRECT` proof
#   hyp th = []                -- no hypotheses
#   axioms() = the three standard axioms (eta, select, infinity)
#
# Subcommands (one task at a time; every proof build is capped and -j1):
#   ensure    <add|sub|neg>        extract once, if no valid cache exists
#   extract   <add|sub|neg>        force extraction (builds the human `_CORRECT`)
#   inject    <add|sub|neg>        append the in-session check to the generated proof
#   verdict   <op|tag> <log>       validate the STATEMENT_* verdict in a run log
#   validate  <add|sub|neg> [file] validate a cache without building anything
#   provenance <add|sub|neg>       print the cache provenance fields
#
# XXIX.A1/A2: a cache is accepted only if it has exactly one REPARSE 1, one
# copy of every mandatory field, a well-formed term payload and length, and
# provenance that still matches the upstream commit, the upstream proof file,
# the theory closure used to parse, and the task's ELF bytes.
#
# XXIX.A4: every path is derived from this script's location (or from
# TEKMERION_BENCH / TEKMERION_HOL / TEKMERION_UPSTREAM when a caller needs a
# fixture); there are no absolute project paths in this file.
set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)" || exit 99
ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)" || exit 99
BENCH="${TEKMERION_BENCH:-$ROOT/.bench}"
HOL="${TEKMERION_HOL:-$BENCH/hol-light-stamp2}"
UPSTREAM="${TEKMERION_UPSTREAM:-$BENCH/human-baseline}"

ARCH=arm
DIR=curve25519
FAM=p25519

usage () {
  echo "usage: stmt_check.sh <ensure|extract|inject|validate|provenance|verdict> <op> [args]" >&2
  exit 90
}

# --- task-dependent paths -------------------------------------------------

set_op () {
  op=$1
  base="bignum_${op}_${FAM}"
  th="BIGNUM_$(echo "${op}_${FAM}" | tr 'a-z' 'A-Z')_CORRECT"
  task="$ARCH.bignum_${op}_${FAM}"
  source_rel="arm/proofs/${base}.ml"
  cachedir="$BENCH/ix89/stmt"
  cache="$cachedir/${ARCH}_${op}_${FAM}.stmt"
  human="$BENCH/ix89/human"
  pristine="$UPSTREAM/$source_rel"
  upstream_obj="$UPSTREAM/$ARCH/$DIR/${base}.o"
  human_obj="$human/$ARCH/$DIR/${base}.o"
  driver="$human/$ARCH/proofs/ix89_stmt_extract_${op}.ml"
  native_rel="ix89_stmt_extract_${op}.native"
  log="$cachedir/${op}.extract.log"
}

holenv () {
  local opam_bin
  opam_bin=$(command -v opam 2>/dev/null || true)
  [ -n "$opam_bin" ] || opam_bin="$HOME/.local/bin/opam"
  eval "$("$opam_bin" env --switch "$HOL" --set-switch)"
  export PATH="$BENCH/bin:${ELAN_HOME:-$HOME/.elan}/toolchains/leanprover--lean4---v4.29.1/bin:$PATH"
}

# --- provenance -----------------------------------------------------------

sha256_of () { sha256sum -- "$1" 2>/dev/null | cut -d' ' -f1; }

resolve_rel () {
  local rel=$1
  if [ -f "$UPSTREAM/$rel" ]; then printf '%s\n' "$UPSTREAM/$rel"
  elif [ -f "$HOL/$rel" ]; then printf '%s\n' "$HOL/$rel"
  else printf 'MISSING\n'; fi
}

# Digest of the transitive `needs` closure rooted at the upstream proof file.
# Paths are made relative before hashing, so the digest is independent of the
# checkout location.  A missing dependency is recorded as MISSING rather than
# skipped, so the digest changes if the closure breaks.
theory_closure_digest () {
  local tmp rel f digest
  tmp=$(mktemp -d "${TMPDIR:-/tmp}/ix89-theory.XXXXXX") || return 1
  : > "$tmp/seen"
  printf '%s\n' "$source_rel" > "$tmp/queue"
  while [ -s "$tmp/queue" ]; do
    rel=$(head -n 1 "$tmp/queue")
    sed -i 1d "$tmp/queue"
    grep -qxF -- "$rel" "$tmp/seen" && continue
    printf '%s\n' "$rel" >> "$tmp/seen"
    f=$(resolve_rel "$rel")
    [ "$f" = "MISSING" ] || \
      sed -n 's/.*needs "\([^"]*\)".*/\1/p' "$f" >> "$tmp/queue"
  done
  digest=$(sort -u "$tmp/seen" | while IFS= read -r rel; do
      f=$(resolve_rel "$rel")
      if [ "$f" = "MISSING" ]; then
        printf 'MISSING %s\n' "$rel"
      else
        sha256sum "$f" | sed "s#  .*#  $rel#"
      fi
    done | sha256sum | cut -d' ' -f1)
  rm -rf "$tmp"
  printf '%s\n' "$digest"
}

# Populate PROV_* globals; return non-zero with PROV_ERROR if the upstream
# sources themselves are unavailable.
provenance_values () {
  PROV_COMMIT=""; PROV_SOURCE=""; PROV_THEORY=""; PROV_OBJECT=""; PROV_ERROR=""
  [ -f "$pristine" ] || { PROV_ERROR="upstream-proof-missing"; return 1; }
  [ -f "$upstream_obj" ] || { PROV_ERROR="upstream-object-missing"; return 1; }
  PROV_COMMIT=$(git -C "$UPSTREAM" rev-parse HEAD 2>/dev/null) || PROV_COMMIT=""
  [ -n "$PROV_COMMIT" ] || { PROV_ERROR="upstream-commit-unavailable"; return 1; }
  PROV_SOURCE=$(sha256_of "$pristine")
  PROV_THEORY=$(theory_closure_digest)
  PROV_OBJECT=$(sha256_of "$upstream_obj")
  [ -n "$PROV_SOURCE" ] && [ -n "$PROV_THEORY" ] && [ -n "$PROV_OBJECT" ] || {
    PROV_ERROR="provenance-unreadable"; return 1; }
  return 0
}

# --- cache validation -----------------------------------------------------

is_hex () {
  local h=$1
  [ -n "$h" ] && [ $(( ${#h} % 2 )) -eq 0 ] || return 1
  case "$h" in *[!0-9a-f]*) return 1 ;; esac
  return 0
}

is_hex_n () {
  local n=$1 h=$2
  [ "${#h}" -eq "$n" ] || return 1
  case "$h" in *[!0-9a-f]*) return 1 ;; esac
  return 0
}

is_digits () {
  local s=$1
  [ -n "$s" ] || return 1
  case "$s" in *[!0-9]*) return 1 ;; esac
  return 0
}

# validate_cache <op> <file>; sets VALIDATE_REASON and returns 1 on rejection.
# Reason vocabulary is shared with stmt_lib.ml so failures are greppable.
validate_cache () {
  local vop=$1 file=$2
  VALIDATE_REASON=""
  local vtask="$ARCH.bignum_${vop}_${FAM}"
  local vth="BIGNUM_$(echo "${vop}_${FAM}" | tr 'a-z' 'A-Z')_CORRECT"
  local vsource="arm/proofs/bignum_${vop}_${FAM}.ml"
  [ -f "$file" ] || { VALIDATE_REASON="cache-missing"; return 1; }

  local line n_task=0 n_theorem=0 n_source=0 n_reparse=0 n_len=0 n_stmt=0
  local n_commit=0 n_src_dig=0 n_th_dig=0 n_obj_dig=0 n_axiom=0 unknown=0
  local v_task="" v_theorem="" v_source="" v_reparse="" v_len="" v_stmt=""
  local v_commit="" v_src_dig="" v_th_dig="" v_obj_dig=""
  local -a v_axioms=()
  while IFS= read -r line || [ -n "$line" ]; do
    case "$line" in
      "TASK "*)              n_task=$((n_task+1));         v_task=${line#TASK } ;;
      "THEOREM "*)           n_theorem=$((n_theorem+1));   v_theorem=${line#THEOREM } ;;
      "SOURCE "*)            n_source=$((n_source+1));     v_source=${line#SOURCE } ;;
      "REPARSE "*)           n_reparse=$((n_reparse+1));   v_reparse=${line#REPARSE } ;;
      "STMT_LEN "*)          n_len=$((n_len+1));           v_len=${line#STMT_LEN } ;;
      "STMT "*)              n_stmt=$((n_stmt+1));         v_stmt=${line#STMT } ;;
      "UPSTREAM_COMMIT "*)   n_commit=$((n_commit+1));     v_commit=${line#UPSTREAM_COMMIT } ;;
      "SOURCE_DIGEST "*)     n_src_dig=$((n_src_dig+1));   v_src_dig=${line#SOURCE_DIGEST } ;;
      "THEORY_DIGEST "*)     n_th_dig=$((n_th_dig+1));     v_th_dig=${line#THEORY_DIGEST } ;;
      "OBJECT_DIGEST "*)     n_obj_dig=$((n_obj_dig+1));   v_obj_dig=${line#OBJECT_DIGEST } ;;
      "AXIOM "*)             n_axiom=$((n_axiom+1));       v_axioms+=("${line#AXIOM }") ;;
      "TASK"|"THEOREM"|"SOURCE"|"REPARSE"|"STMT_LEN"|"STMT"|"UPSTREAM_COMMIT"|\
      "SOURCE_DIGEST"|"THEORY_DIGEST"|"OBJECT_DIGEST"|"AXIOM")
                             unknown=$((unknown+1)) ;;
      *)                     unknown=$((unknown+1)) ;;
    esac
  done < "$file"

  if [ "$n_task" -ne 1 ]; then VALIDATE_REASON="task-field"; return 1; fi
  if [ "$v_task" != "$vtask" ]; then VALIDATE_REASON="task-mismatch"; return 1; fi
  if [ "$n_theorem" -ne 1 ]; then VALIDATE_REASON="theorem-field"; return 1; fi
  if [ "$v_theorem" != "$vth" ]; then VALIDATE_REASON="theorem-mismatch"; return 1; fi
  if [ "$n_source" -ne 1 ]; then VALIDATE_REASON="source-field"; return 1; fi
  if [ "$v_source" != "$vsource" ]; then VALIDATE_REASON="source-mismatch"; return 1; fi
  if [ "$n_reparse" -ne 1 ]; then VALIDATE_REASON="reparse-field"; return 1; fi
  if [ "$v_reparse" != "1" ]; then VALIDATE_REASON="reparse-not-1"; return 1; fi

  if [ "$n_stmt" -ne 1 ]; then VALIDATE_REASON="stmt-field"; return 1; fi
  if ! is_hex "$v_stmt"; then VALIDATE_REASON="stmt-encoding"; return 1; fi
  if [ "$n_len" -ne 1 ]; then VALIDATE_REASON="stmt-length-field"; return 1; fi
  if ! is_digits "$v_len"; then VALIDATE_REASON="stmt-length-malformed"; return 1; fi
  if [ "$v_len" -ne $(( ${#v_stmt} / 2 )) ]; then
    VALIDATE_REASON="stmt-length-mismatch"; return 1; fi

  if [ "$n_axiom" -ne 3 ]; then VALIDATE_REASON="axiom-count"; return 1; fi
  local a
  for a in "${v_axioms[@]}"; do
    if ! is_hex "$a"; then VALIDATE_REASON="axiom-encoding"; return 1; fi
  done

  if [ "$n_commit" -ne 1 ]; then VALIDATE_REASON="upstream-commit-field"; return 1; fi
  if ! is_hex_n 40 "$v_commit"; then VALIDATE_REASON="upstream-commit-malformed"; return 1; fi
  if [ "$n_src_dig" -ne 1 ]; then VALIDATE_REASON="source-digest-field"; return 1; fi
  if ! is_hex_n 64 "$v_src_dig"; then VALIDATE_REASON="source-digest-malformed"; return 1; fi
  if [ "$n_th_dig" -ne 1 ]; then VALIDATE_REASON="theory-digest-field"; return 1; fi
  if ! is_hex_n 64 "$v_th_dig"; then VALIDATE_REASON="theory-digest-malformed"; return 1; fi
  if [ "$n_obj_dig" -ne 1 ]; then VALIDATE_REASON="object-digest-field"; return 1; fi
  if ! is_hex_n 64 "$v_obj_dig"; then VALIDATE_REASON="object-digest-malformed"; return 1; fi
  if [ "$unknown" -ne 0 ]; then VALIDATE_REASON="unknown-field"; return 1; fi

  provenance_values || { VALIDATE_REASON=$PROV_ERROR; return 1; }
  if [ "$v_commit" != "$PROV_COMMIT" ]; then
    VALIDATE_REASON="upstream-commit-changed"; return 1; fi
  if [ "$v_src_dig" != "$PROV_SOURCE" ]; then
    VALIDATE_REASON="source-digest-changed"; return 1; fi
  if [ "$v_th_dig" != "$PROV_THEORY" ]; then
    VALIDATE_REASON="theory-digest-changed"; return 1; fi
  if [ "$v_obj_dig" != "$PROV_OBJECT" ]; then
    VALIDATE_REASON="object-digest-changed"; return 1; fi
  return 0
}

# --- extraction -----------------------------------------------------------

extract () {
  mkdir -p "$cachedir" "$human/$ARCH/proofs"
  [ -f "$pristine" ] || { echo "stmt_check.sh: missing $pristine" >&2; return 96; }
  [ -f "$upstream_obj" ] || { echo "stmt_check.sh: missing $upstream_obj" >&2; return 96; }
  [ -f "$human_obj" ] || { echo "stmt_check.sh: missing $human_obj" >&2; return 96; }
  provenance_values || {
    echo "STMT_EXTRACT_FAIL task=${task} provenance=${PROV_ERROR}" >&2; return 95; }
  # The extraction builds against the human tree's ELF; it must be the same
  # object whose digest is recorded in the cache.
  if [ "$(sha256_of "$human_obj")" != "$PROV_OBJECT" ]; then
    echo "STMT_EXTRACT_FAIL task=${task} reason=object-mismatch" >&2; return 95
  fi
  cp "$pristine" "$human/$ARCH/proofs/${base}.ml" || return 96
  cp "$ROOT/tests/ix89/stmt_lib.ml" "$human/$ARCH/proofs/ix89_stmt_lib.ml" || return 96
  cat > "$driver" <<EOF
(* XXVIII.A / XXIX.A: cache concl of the upstream ${th}.  Generated; do not edit. *)
needs "arm/proofs/${base}.ml";;
needs "arm/proofs/ix89_stmt_lib.ml";;

let ix89_stmt = concl ${th};;
let ix89_text = string_of_term ix89_stmt;;
let ix89_reparse =
  (try aconv (parse_term ix89_text) ix89_stmt with _ -> false);;
Printf.printf "STMT_REPARSE %d\n%!" (if ix89_reparse then 1 else 0);;
Printf.printf "STMT_TEXT_LEN %d\n%!" (String.length ix89_text);;
let ix89_axes = List.map (fun t -> string_of_term (concl t)) (axioms ());;
ix89_write_cache "${cache}"
  (["TASK ${task}";
    "THEOREM ${th}";
    "SOURCE arm/proofs/${base}.ml";
    "REPARSE " ^ (if ix89_reparse then "1" else "0");
    "STMT_LEN " ^ string_of_int (String.length ix89_text);
    "STMT " ^ ix89_hex_of_string ix89_text;
    "UPSTREAM_COMMIT ${PROV_COMMIT}";
    "SOURCE_DIGEST ${PROV_SOURCE}";
    "THEORY_DIGEST ${PROV_THEORY}";
    "OBJECT_DIGEST ${PROV_OBJECT}"]
   @ List.map (fun h -> "AXIOM " ^ h) (List.map ix89_hex_of_string ix89_axes));;
List.iter (fun a -> Printf.printf "STMT_AXIOM %s\n%!" a) ix89_axes;;
Printf.printf "STMT_EXTRACT task=${task} reparse=%d axioms=%d\n%!"
  (if ix89_reparse then 1 else 0) (List.length ix89_axes);;
EOF
  holenv
  ( cd "$human/$ARCH" && \
    "$ROOT/tools/cap.sh" 12G -- ../tools/build-proof.sh \
      "proofs/ix89_stmt_extract_${op}.ml" "$HOL/hol.sh" "$native_rel" ) \
    > "$log" 2>&1
  local rc=$?
  if [ $rc -ne 0 ]; then
    echo "STMT_EXTRACT_FAIL task=${task} stage=build rc=$rc log=$log" >&2
    return $rc
  fi
  ( cd "$human" && "$ROOT/tools/cap.sh" 12G -- "./$ARCH/$native_rel" ) \
    >> "$log" 2>&1
  rc=$?
  if [ $rc -ne 0 ]; then
    echo "STMT_EXTRACT_FAIL task=${task} stage=run rc=$rc log=$log" >&2
    return $rc
  fi
  if ! grep -q '^STMT_REPARSE 1$' "$log"; then
    echo "STMT_EXTRACT_FAIL task=${task} reparse=0 log=$log" >&2
    return 95
  fi
  [ -s "$cache" ] || { echo "STMT_EXTRACT_FAIL task=${task} cache-missing" >&2; return 95; }
  if ! validate_cache "$op" "$cache"; then
    echo "STMT_EXTRACT_FAIL task=${task} cache-invalid reason=${VALIDATE_REASON} log=$log" >&2
    return 95
  fi
  grep -E '^STMT_(EXTRACT|REPARSE|TEXT_LEN|AXIOM)' "$log"
}

inject () {
  local dst="$BENCH/dropin/$ARCH/proofs/${base}.ml"
  local lib="$BENCH/dropin/$ARCH/proofs/ix89_stmt_lib.ml"
  [ -f "$dst" ] || {
    echo "stmt_check.sh: $dst missing (run dropin assemble first)" >&2; return 92; }
  if ! validate_cache "$op" "$cache"; then
    echo "stmt_check.sh: $cache rejected reason=${VALIDATE_REASON} (run 'stmt_check.sh extract $op')" >&2
    return 92
  fi
  cp "$ROOT/tests/ix89/stmt_lib.ml" "$lib" || return 96
  cat >> "$dst" <<EOF

(* ---- XXVIII.A / XXIX.A: statement / hypothesis / axiom check, same session ---- *)
needs "arm/proofs/ix89_stmt_lib.ml";;
let ix89_task = "${task}";;
let ix89_theorem = "${th}";;
let ix89_source = "arm/proofs/${base}.ml";;
let ix89_cache = "${cache}";;
let ix89_th = ${th};;
let ix89_stmt_check () =
  let cache_errors =
    try ix89_validate_cache ~task:ix89_task ~theorem:ix89_theorem
          ~source:ix89_source ix89_cache
    with _ -> ["cache-read-error"] in
  let entries =
    if cache_errors = [] then (try ix89_read_cache ix89_cache with _ -> []) else [] in
  let stmt =
    match List.find_opt (fun (k, _) -> k = "STMT") entries with
      None -> None
    | Some (_, h) -> (try Some (parse_term (ix89_string_of_hex h)) with _ -> None) in
  let cached_axes =
    List.map (fun (_, h) -> ix89_string_of_hex h)
      (List.filter (fun (k, _) -> k = "AXIOM") entries) in
  let hyps = hyp ix89_th in
  let actual_axes = List.map (fun t -> string_of_term (concl t)) (axioms ()) in
  let expected_axes =
    List.map (fun t -> string_of_term (concl t)) [INFINITY_AX; SELECT_AX; ETA_AX] in
  let sorted l = List.sort compare l in
  let ax_ok = (sorted actual_axes = sorted expected_axes) in
  let cached_ok =
    (cache_errors = []) && (sorted actual_axes = sorted cached_axes) in
  let aconv_ok =
    match stmt with Some s -> aconv (concl ix89_th) s | None -> false in
  let nhyps = List.length hyps in
  if cache_errors = [] && aconv_ok && nhyps = 0 && ax_ok && cached_ok then
    Printf.printf "STATEMENT_OK task=%s aconv=1 hyps=0 axioms=%d\n%!"
      ix89_task (List.length actual_axes)
  else begin
    Printf.printf "STATEMENT_FAIL task=%s aconv=%d hyps=%d axioms=%d axioms_ok=%d cached_ok=%d cache_errors=%s\n%!"
      ix89_task (if aconv_ok then 1 else 0) nhyps
      (List.length actual_axes) (if ax_ok then 1 else 0) (if cached_ok then 1 else 0)
      (String.concat "," cache_errors);
    (match stmt with
       | None -> Printf.printf "STATEMENT_REF <cache-missing-or-unparseable>\n%!"
     | Some s ->
         Printf.printf "STATEMENT_REF %s\n%!"
           (ix89_truncate 1200 (ix89_oneline (string_of_term s)));
         Printf.printf "STATEMENT_GOT %s\n%!"
           (ix89_truncate 1200 (ix89_oneline (string_of_term (concl ix89_th)))))
  end;
  List.iter (fun a -> Printf.printf "STATEMENT_AXIOM %s\n%!" a) actual_axes;
  List.iter (fun h -> Printf.printf "STATEMENT_HYP %s\n%!"
                        (ix89_truncate 400 (ix89_oneline (string_of_term h)))) hyps;;
ix89_stmt_check ();;
Printf.printf "STATEMENT_CHECK_COMPLETE task=%s\n%!" ix89_task;;
EOF
  echo "STATEMENT_INJECT task=${task} proof=$dst cache=valid"
}

# --- verdict --------------------------------------------------------------

# verdict <op|tag> <log>
verdict () {
  local spec=${1:-} file=${2:-}
  [ -n "$spec" ] && [ -n "$file" ] || usage
  local vop=""
  case "$spec" in
    add|sub|neg) vop=$spec ;;
    *)
      case "$spec" in
        *add*) vop=add ;;
        *sub*) vop=sub ;;
        *neg*) vop=neg ;;
        *) usage ;;
      esac ;;
  esac
  local vtask="$ARCH.bignum_${vop}_${FAM}"
  local out="$file.statement"
  local reject
  reject () {
    echo "STATEMENT_REJECT task=${vtask} tag=${spec} reason=$1 file=${out}" >&2
    return 1
  }

  [ -f "$file" ] || { reject missing-log; return 1; }
  {
    echo "STATEMENT_CHECK_FILE $file"
    grep -E '^(STATEMENT|STMT_EXTRACT_FAIL)' "$file" 2>/dev/null
  } > "$out"

  # No failure marker may appear anywhere in the run log.
  if grep -Eq 'STATEMENT_FAIL|STMT_EXTRACT_FAIL|STATEMENT_CHECK_ABORT' "$file"; then
    reject stray-STATEMENT_FAIL
    grep -E 'STATEMENT_FAIL|STMT_EXTRACT_FAIL|STATEMENT_CHECK_ABORT' "$out" >&2 || true
    return 1
  fi

  local n_ok n_done
  n_ok=$(grep -c '^STATEMENT_OK ' "$file" 2>/dev/null || true)
  n_ok=${n_ok:-0}
  if [ "$n_ok" -eq 0 ]; then reject missing-STATEMENT_OK; return 1; fi
  if [ "$n_ok" -gt 1 ]; then
    reject duplicate-STATEMENT_OK
    grep '^STATEMENT_OK ' "$out" >&2 || true
    return 1
  fi

  # Exactly the expected task, and the line must have the canonical shape.
  if ! grep -Eq "^STATEMENT_OK task=${vtask} aconv=1 hyps=0 axioms=3\$" "$file"; then
    local got
    got=$(sed -n 's/^STATEMENT_OK task=\([^ ]*\).*/\1/p' "$file" | head -n 1)
    if [ "$got" != "$vtask" ]; then
      reject wrong-task
    else
      reject malformed-STATEMENT_OK
    fi
    grep '^STATEMENT_OK ' "$out" >&2 || true
    return 1
  fi

  n_done=$(grep -c "^STATEMENT_CHECK_COMPLETE task=${vtask}\$" "$file" 2>/dev/null || true)
  n_done=${n_done:-0}
  if [ "$n_done" -eq 0 ]; then reject missing-completion-marker; return 1; fi
  if [ "$n_done" -gt 1 ]; then reject duplicate-completion-marker; return 1; fi

  grep '^STATEMENT_OK ' "$out"
  return 0
}

# --- dispatch -------------------------------------------------------------

cmd=${1:-}
case "$cmd" in
  ensure|extract|inject|validate|provenance)
    op=${2:-}
    case "$op" in add|sub|neg) ;; *) usage ;; esac
    set_op "$op" ;;
  verdict) ;;
  *) usage ;;
esac

case "$cmd" in
  extract) extract ;;
  ensure)
    if [ -e "$cache" ]; then
      if validate_cache "$op" "$cache"; then
        echo "STATEMENT_CACHE_HIT task=${task} cache=$cache"
        exit 0
      fi
      echo "STATEMENT_CACHE_REJECT task=${task} reason=${VALIDATE_REASON} action=rebuild cache=$cache" >&2
    fi
    extract ;;
  inject) inject ;;
  validate)
    file=${3:-$cache}
    if validate_cache "$op" "$file"; then
      echo "STATEMENT_CACHE_VALID task=${task} cache=$file"
      exit 0
    fi
    echo "STATEMENT_CACHE_REJECT task=${task} reason=${VALIDATE_REASON} cache=$file" >&2
    exit 91 ;;
  provenance)
    provenance_values || {
      echo "STATEMENT_PROVENANCE_FAIL task=${task} reason=${PROV_ERROR}" >&2; exit 91; }
    echo "UPSTREAM_COMMIT ${PROV_COMMIT}"
    echo "SOURCE_DIGEST ${PROV_SOURCE}"
    echo "THEORY_DIGEST ${PROV_THEORY}"
    echo "OBJECT_DIGEST ${PROV_OBJECT}" ;;
  verdict) verdict "${2:-}" "${3:-}" ;;
  *) usage ;;
esac
