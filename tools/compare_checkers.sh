#!/usr/bin/env bash
# Checker-versus-checker harness.
#
# usage: tools/compare_checkers.sh <object-dir> [node-budget]
#
#   <object-dir> must contain, for each <tag>:
#       <tag>.hlpo  <tag>.hlst  <tag>.theory  <tag>.expected
#   [node-budget] defaults to 8000000.
#
# What it does:
#   * runs the Tekmerion Rust checker twice per object, capped at 20G, one at
#     a time, and writes the side-by-side table (CSV + Markdown);
#   * proves two-run determinism by comparing statement_id and
#     derivation_digest across runs;
#   * reports replayed-nodes/s and the *recorded proof's* reported-inferences/s
#     as two separate columns, because they are different numbers;
#   * runs an independent checker only if one is actually supplied, via
#     INDEP_RUNNER (see below).  It never simulates one, and it prints the
#     exact commands a person with HOLTrace would run when it is absent.
#
# Independent-checker contract (opt-in):
#   INDEP_RUNNER=/path/to/runner
#     $INDEP_RUNNER <converted-trace> <out-file>
#   must verify the trace with the independent tool and write exactly one line:
#       ok <seconds> <peak_rss_kb>
#   or
#       fail <reason>
#   TRACE_DIR=<dir> must contain <tag>.50 (HOLTrace's format).  Conversion
#   from .hlpo to .50 is not part of this script; see the BLOCKED notes it
#   prints.  A failed/absent independent run is reported as such, never as a
#   number.
set -euo pipefail

ROOT=/mnt/1tb_kingston/Tekmerion
PO="$ROOT/target/release/proof_object"
CAP="$ROOT/tools/cap.sh"

DIR=${1:?usage: tools/compare_checkers.sh <object-dir> [node-budget]}
BUDGET=${2:-8000000}
case "$DIR" in /*) ;; *) DIR="$ROOT/$DIR" ;; esac
OUT="$DIR/compare"
mkdir -p "$OUT"

CSV="$OUT/rust.csv"
MD="$OUT/rust.md"

echo "object,run,bytes,recorded_nodes,reachable_nodes,external_wall_ms,validate_ms,peak_rss_kb,peak_live_nodes,replayed_nodes_per_s,reported_inferences_per_s,statement_id,derivation_digest" > "$CSV"

for obj in "$DIR"/*.hlpo; do
  [ -e "$obj" ] || { echo "no .hlpo objects in $DIR" >&2; exit 2; }
  tag=$(basename "$obj" .hlpo)
  store="$DIR/$tag.hlst"
  theory="$DIR/$tag.theory"
  expected="$DIR/$tag.expected"
  for f in "$obj" "$store" "$theory" "$expected"; do
    [ -f "$f" ] || { echo "missing $f" >&2; exit 2; }
  done
  bytes=$(stat -c %s "$obj")
  for run in 1 2; do
    json="$OUT/$tag.run$run.json"
    log="$OUT/$tag.run$run.log"
    t0=$(date +%s%N)
    "$CAP" --timeout 1800 20G -- \
      "$PO" validate --object "$obj" --theory "$theory" --store "$store" \
        --expected "$expected" --replay frontier --budget "$BUDGET" --json "$json" \
      > "$log" 2>&1
    t1=$(date +%s%N)
    ext_ms=$(( (t1 - t0) / 1000000 ))
    [ -s "$json" ] || { echo "run failed: $tag run $run (see $log)" >&2; tail -5 "$log" >&2; exit 1; }

    recorded=$(jq -r '.raw_nodes' "$json")
    reachable=$(jq -r '.reachable' "$json")
    vms=$(jq -r '.wall_ms' "$json")
    rss=$(jq -r '.peak_rss_kb' "$json")
    live=$(jq -r '.peak_live_nodes' "$json")
    inf0=$(jq -r '.inference_begin' "$json")
    inf1=$(jq -r '.inference_total' "$json")
    sid=$(jq -r '.statement_id' "$json")
    der=$(jq -r '.derivation_digest' "$json")

    rates=$(awk -v r="$reachable" -v i0="$inf0" -v i1="$inf1" -v ms="$vms" 'BEGIN {
      s = ms / 1000.0;
      if (s <= 0) { printf "NA NA" } else { printf "%.0f %.0f", r / s, (i1 - i0) / s }
    }')
    nodes_s=$(printf '%s' "$rates" | cut -d' ' -f1)
    infs_s=$(printf '%s' "$rates" | cut -d' ' -f2)

    echo "$tag,$run,$bytes,$recorded,$reachable,$ext_ms,$vms,$rss,$live,$nodes_s,$infs_s,$sid,$der" >> "$CSV"
  done
done

# ---- deterministic? --------------------------------------------------------
det_ok=1
for obj in "$DIR"/*.hlpo; do
  tag=$(basename "$obj" .hlpo)
  a=$(jq -r '.statement_id + " " + .derivation_digest + " " + (.reachable|tostring)' "$OUT/$tag.run1.json")
  b=$(jq -r '.statement_id + " " + .derivation_digest + " " + (.reachable|tostring)' "$OUT/$tag.run2.json")
  if [ "$a" != "$b" ]; then det_ok=0; echo "NON-DETERMINISTIC: $tag" >&2; fi
done

# ---- table -----------------------------------------------------------------
{
  echo "| object | bytes | recorded nodes | reachable nodes | checker s (run 1 / 2) | process wall s (run 1 / 2) | peak RSS KB (run 1 / 2) | peak live nodes | replayed nodes/s (1 / 2) | reported inferences/s (1 / 2) |"
  echo "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|"
  for obj in "$DIR"/*.hlpo; do
    tag=$(basename "$obj" .hlpo)
    d1=$(awk -F, -v t="$tag" '$1==t && $2==1 {printf "%.3f", $7/1000}' "$CSV")
    d2=$(awk -F, -v t="$tag" '$1==t && $2==2 {printf "%.3f", $7/1000}' "$CSV")
    w1=$(awk -F, -v t="$tag" '$1==t && $2==1 {printf "%.3f", $6/1000}' "$CSV")
    w2=$(awk -F, -v t="$tag" '$1==t && $2==2 {printf "%.3f", $6/1000}' "$CSV")
    b=$(awk -F, -v t="$tag" '$1==t && $2==1 {print $3}' "$CSV")
    rn=$(awk -F, -v t="$tag" '$1==t && $2==1 {print $4}' "$CSV")
    ra=$(awk -F, -v t="$tag" '$1==t && $2==1 {print $5}' "$CSV")
    rs1=$(awk -F, -v t="$tag" '$1==t && $2==1 {print $8}' "$CSV")
    rs2=$(awk -F, -v t="$tag" '$1==t && $2==2 {print $8}' "$CSV")
    lv=$(awk -F, -v t="$tag" '$1==t && $2==1 {print $9}' "$CSV")
    n1=$(awk -F, -v t="$tag" '$1==t && $2==1 {print $10}' "$CSV")
    n2=$(awk -F, -v t="$tag" '$1==t && $2==2 {print $10}' "$CSV")
    i1=$(awk -F, -v t="$tag" '$1==t && $2==1 {print $11}' "$CSV")
    i2=$(awk -F, -v t="$tag" '$1==t && $2==2 {print $11}' "$CSV")
    echo "| $tag | $b | $rn | $ra | $d1 / $d2 | $w1 / $w2 | $rs1 / $rs2 | $lv | $n1 / $n2 | $i1 / $i2 |"
  done
  if [ "$det_ok" -eq 1 ]; then
    echo ""
    echo "Determinism: **yes** — statement_id, derivation_digest and reachable node count are identical in both runs."
  else
    echo ""
    echo "Determinism: **NO** — see stderr."
  fi
} | tee "$MD"

# ---- independent checker ---------------------------------------------------
echo ""
echo "## Independent checker"
if [ -n "${INDEP_RUNNER:-}" ] && [ -n "${TRACE_DIR:-}" ]; then
  echo "| object | independent ok | independent s | independent peak RSS KB |"
  echo "|---|---:|---:|---:|"
  for obj in "$DIR"/*.hlpo; do
    tag=$(basename "$obj" .hlpo)
    trace="$TRACE_DIR/$tag.50"
    out="$OUT/$tag.indep.txt"
    if [ ! -f "$trace" ]; then
      echo "| $tag | BLOCKED (no $trace) | — | — |"
      continue
    fi
    if "$INDEP_RUNNER" "$trace" "$out" > "$OUT/$tag.indep.log" 2>&1; then
      read -r status secs peak < "$out" || true
      if [ "${status:-}" = "ok" ]; then
        echo "| $tag | yes | $secs | $peak |"
      else
        echo "| $tag | NO (${secs:-failed}) | — | — |"
      fi
    else
      echo "| $tag | NO (runner failed) | — | — |"
    fi
  done
else
  cat <<'EOF'
**BLOCKED — no independent checker was available offline, so no independent
number is claimed.** What was looked for and what failed:

  * `holtrace`, `holtrace_restore`, `holtrace-verify`, `candle`,
    `candle-checker` on `PATH`: not found.
  * The project tree, `/home/one`, `/usr`, `/usr/local`, `/opt`: no HOLTrace or
    Candle source or binary.
  * The installed opam switches (`hol-light`, `hol-light-objects`,
    `hol-light-stamp2`, `dsfb`, `creusot-zic`): neither package installed.
  * `holtrace.cr.cyp.to` (spelling in the task) does not resolve. The real
    host, `holtrace.cr.yp.to`, does resolve and serves HTTP 200 from this
    machine, but nothing was downloaded or built in this run.

Why a `.hlpo` cannot simply be "run through HOLTrace": HOLTrace verifies its
own `.50` trace format inside a HOL Light session (`holtrace_restore`), and
checks axioms/definitions/type definitions against that session's registries.
The Tekmerion object format is different. Running *the same objects* therefore
needs either a `.hlpo` -> `.50` converter or a re-recording with HOLTrace's
tracer (which produces different artefacts). Neither exists here.

Exact commands a person with the tool would run (Linux):

  # 1. obtain and install HOLTrace (network; version 20261122 at the time of
  #    writing).  Follow doc/install.md from the unpacked tree.
  wget -m https://holtrace.cr.yp.to/holtrace-latest-version.txt
  version=$(cat holtrace.cr.yp.to/holtrace-latest-version.txt)
  wget -m https://holtrace.cr.yp.to/holtrace-$version.tar.gz
  tar -xzf holtrace.cr.yp.to/holtrace-$version.tar.gz
  cd holtrace-$version

  # 2. produce <tag>.50 for each object (converter: not written here), then in
  #    a HOL Light session with HOLTrace installed:
  #      holtrace_restore !theorems "<tag>.50";;
  #    To time a whole session, put that line in restore.ml and run the process
  #    under the 20G cap, e.g.:
  printf 'needs "holtrace_restore.ml";;\nholtrace_restore !theorems "%s";;\n' "$PWD/$tag.50" > restore.ml
  tools/cap.sh 1800 20G -- <path-to-hol-light>/ocaml-hol -init hol.ml < restore.ml
  #    and supply a runner to this script:
  #      INDEP_RUNNER=/path/to/runner TRACE_DIR=/path/to/traces \
  #        tools/compare_checkers.sh .bench/ix87/objects

  # 3. Candle: no local installation and no command can be given without the
  #    tool.  Its kernel replay reads Candle's own proof representation, so a
  #    converter would be needed there too.
EOF
fi
