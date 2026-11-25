#!/bin/bash
# IX.69 receipt generator. Bash + awk only; there is no Python in this tree.
#
#   tests/ix69_report.sh json  <scratch-dir>   > research/receipts/ix69-slicing.json
#   tests/ix69_report.sh table <json-file>
#
# `json` reads the per-run `.correct` logs under <scratch-dir>/{baseline,sliced,
# sweep,control-*,determinism}; it (re)derives each run's phase blocks with
# tests/ix69_phases.awk (IX.66's splitter, copied here so the receipt does not
# depend on git-ignored scratch) and emits the JSON. `table` renders the JSON.
#
# Every number in the JSON is printed by tests/ix69_collect.awk from a line of a
# log or of a derived block table; nothing is typed by hand.
set -eu
LC_ALL=C
root=$(cd "$(dirname "$0")/.." && pwd)

collect () { # <dir> <tag>
  local dir=$1 tag=$2 base="$root/.bench/ix69/$1/$2"
  [ -f "$base.correct" ] || return 0
  if [ ! -f "$base.blocks" ] || [ ! -f "$base.aux" ]; then
    awk -f "$root/tests/ix69_phases.awk" "$base.correct" > "$base.blocks" 2> "$base.aux"
  fi
  awk -v dir="$dir" -v tag="$tag" -f "$root/tests/ix69_collect.awk" \
    "$base.correct" "$base.blocks" "$base.aux"
}

case "${1:-}" in
  json)
    scratch=${2:-"$root/.bench/ix69"}
    {
      for d in baseline sliced sweep; do
        for f in "$scratch/$d"/*.correct; do
          [ -e "$f" ] || continue
          collect "$d" "$(basename "$f" .correct)"
        done
      done
      for d in "$scratch"/control-* "$scratch"/determinism; do
        [ -d "$d" ] || continue
        for f in "$d"/*.correct; do
          [ -e "$f" ] || continue
          collect "$(basename "$d")" "$(basename "$f" .correct)"
        done
      done
    } | awk -F'\t' -f "$root/tests/ix69_json.awk"
    ;;
  table)
    [ "$#" -eq 2 ] || { echo "usage: $0 table <json-file>" >&2; exit 2; }
    awk -f "$root/tests/ix69_table.awk" "$2"
    ;;
  *)
    echo "usage: $0 json <scratch-dir> | $0 table <json-file>" >&2
    exit 2
    ;;
esac
