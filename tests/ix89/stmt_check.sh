#!/bin/bash
# XXVIII.A: statement / hypothesis / axiom check on the IX.89 measurement path.
#
# The check runs in the same HOL session that produces the generated theorem:
#
#   aconv (concl th) stmt      -- stmt cached from the upstream `_CORRECT` proof
#   hyp th = []                -- no hypotheses
#   axioms() = the three standard axioms (eta, select, infinity)
#
# Subcommands (one task at a time; every proof build is capped and -j1):
#   ensure  <add|sub|neg>   extract the upstream conclusion once, if not cached
#   extract <add|sub|neg>   force extraction (builds the human `_CORRECT` proof)
#   inject  <add|sub|neg>   append the in-session check to the generated proof
#   verdict <tag> <correct> write the STATEMENT_* verdict next to the log and
#                           echo the OK/FAIL line
#
# Extraction route (a) from the XXVIII.A brief: build the upstream proof, save
# `string_of_term (concl th)` (hex, line safe), re-parse it with HOL Light's
# own term parser in the generated session and compare with `aconv`.
set -u
ROOT=/mnt/1tb_kingston/Tekmerion
HOL="$ROOT/.bench/hol-light-stamp2"
ARCH=arm
DIR=curve25519
FAM=p25519

usage () {
  echo "usage: stmt_check.sh <ensure|extract|inject|verdict> <op> [args]" >&2
  exit 90
}

cmd=${1:-}
op=${2:-}
case "$cmd" in
  ensure|extract|inject)
    case "$op" in add|sub|neg) ;; *) usage ;; esac ;;
  verdict) ;;
  *) usage ;;
esac

base="bignum_${op}_${FAM}"
th="BIGNUM_$(echo "${op}_${FAM}" | tr 'a-z' 'A-Z')_CORRECT"
task="$ARCH.bignum_${op}_${FAM}"
cachedir="$ROOT/.bench/ix89/stmt"
cache="$cachedir/${ARCH}_${op}_${FAM}.stmt"
human="$ROOT/.bench/ix89/human"
pristine="$ROOT/.bench/human-baseline/${ARCH}/proofs/${base}.ml"
driver="$human/${ARCH}/proofs/ix89_stmt_extract_${op}.ml"
native_rel="ix89_stmt_extract_${op}.native"
log="$cachedir/${op}.extract.log"

holenv () {
  eval "$(/home/one/.local/bin/opam env --switch "$HOL" --set-switch)"
  export PATH="$ROOT/.bench/bin:/home/one/.elan/toolchains/leanprover--lean4---v4.29.1/bin:$PATH"
}

extract () {
  mkdir -p "$cachedir"
  [ -f "$pristine" ] || { echo "stmt_check.sh: missing $pristine" >&2; return 96; }
  cp "$pristine" "$human/${ARCH}/proofs/${base}.ml" || return 96
  cp "$ROOT/tests/ix89/stmt_lib.ml" "$human/${ARCH}/proofs/ix89_stmt_lib.ml" || return 96
  cat > "$driver" <<EOF
(* XXVIII.A: cache concl of the upstream ${th}.  Generated; do not edit. *)
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
    "STMT " ^ ix89_hex_of_string ix89_text]
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
  grep -E '^STMT_(EXTRACT|REPARSE|TEXT_LEN|AXIOM)' "$log"
}

inject () {
  local dst="$ROOT/.bench/dropin/$ARCH/proofs/${base}.ml"
  local lib="$ROOT/.bench/dropin/$ARCH/proofs/ix89_stmt_lib.ml"
  [ -f "$dst" ] || {
    echo "stmt_check.sh: $dst missing (run dropin assemble first)" >&2; return 92; }
  [ -s "$cache" ] || {
    echo "stmt_check.sh: $cache missing (run 'stmt_check.sh extract $op')" >&2; return 92; }
  cp "$ROOT/tests/ix89/stmt_lib.ml" "$lib" || return 96
  cat >> "$dst" <<EOF

(* ---- XXVIII.A: statement / hypothesis / axiom check, same session ---- *)
needs "arm/proofs/ix89_stmt_lib.ml";;
let ix89_task = "${task}";;
let ix89_th = ${th};;
let ix89_stmt_check () =
  let entries =
    try ix89_read_cache "${cache}" with _ -> [] in
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
    (cached_axes = []) || (sorted actual_axes = sorted cached_axes) in
  let aconv_ok =
    match stmt with Some s -> aconv (concl ix89_th) s | None -> false in
  let nhyps = List.length hyps in
  if aconv_ok && nhyps = 0 && ax_ok && cached_ok then
    Printf.printf "STATEMENT_OK task=%s aconv=1 hyps=0 axioms=%d\n%!"
      ix89_task (List.length actual_axes)
  else begin
    Printf.printf "STATEMENT_FAIL task=%s aconv=%d hyps=%d axioms=%d axioms_ok=%d cached_ok=%d\n%!"
      ix89_task (if aconv_ok then 1 else 0) nhyps
      (List.length actual_axes) (if ax_ok then 1 else 0) (if cached_ok then 1 else 0);
    (match stmt with
       None -> Printf.printf "STATEMENT_REF <cache-missing-or-unparseable>\n%!"
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
Printf.printf "STATEMENT_CHECK_DONE task=%s\n%!" ix89_task;;
EOF
  echo "STATEMENT_INJECT task=${task} proof=$dst"
}

verdict () {
  local tag=${1:-} file=${2:-}
  [ -n "$tag" ] && [ -n "$file" ] || usage
  {
    echo "STATEMENT_CHECK_FILE $file"
    grep -E '^STATEMENT_(OK|FAIL|AXIOM|HYP|REF|GOT|CHECK_DONE|INJECT)' "$file" 2>/dev/null
  } > "$file.statement"
  if grep -q '^STATEMENT_OK ' "$file.statement"; then
    grep '^STATEMENT_OK ' "$file.statement"
    return 0
  fi
  echo "STATEMENT_FAIL task=${tag} no-STATEMENT_OK-line (see $file.statement)" >&2
  grep -E '^STATEMENT_FAIL ' "$file.statement" || true
  return 1
}

case "$cmd" in
  extract) extract ;;
  ensure)
    if [ -s "$cache" ] && ! grep -q '^REPARSE 0$' "$cache"; then
      echo "STATEMENT_CACHE_HIT task=${task} cache=$cache"
    else
      extract
    fi ;;
  inject) inject ;;
  verdict) verdict "${2:-}" "${3:-}" ;;
  *) usage ;;
esac
