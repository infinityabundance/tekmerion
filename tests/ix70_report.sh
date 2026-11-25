#!/bin/bash
# IX.70 receipt generator. Bash + awk only; no Python.
#
#   tests/ix70_report.sh [scratch-dir] > research/receipts/ix70-algebraic-close.json
#
# Reads:
#   <scratch>/prefull/<tag>.correct   SAT close, full (pre-IX.69) hypothesis set
#   <scratch>/sliced/<tag>.correct    SAT close, IX.69 slice (shipped default)
#   <scratch>/alg/<tag>.correct       algebraic close (TEKMERION_ALG_CLOSE)
#   <scratch>/c5/<tag>.correct        three other tasks (scope spot check)
#   <scratch>/c4-sat/cadical-invocations.txt
#   <scratch>/c4-alg/cadical-invocations.txt
#
# Every number is read from a line of one of those files; nothing is typed.
set -eu
LC_ALL=C
root=$(cd "$(dirname "$0")/.." && pwd)
scratch=${1:-"$root/.bench/ix70"}
tag=arm_add_p25519

total () { grep '^STAMP_INFERENCES' "$1" 2>/dev/null | awk '{print $2}' | tail -1; }
closes () { grep -c '^PHASE_MARK bridge:prove-entry' "$1" 2>/dev/null || true; }
lastmark () { grep '^PHASE_MARK' "$1" 2>/dev/null | awk '{print $NF}' | tail -1; }
lastlabel () {
  grep '^PHASE_MARK' "$1" 2>/dev/null \
    | sed -e 's/^PHASE_MARK //' -e 's/ [0-9]*$//' | tail -1
}
# phase-class sum, from IX.66's splitter
classsum () { # <file> <class>
  awk -f "$root/tests/ix69_phases.awk" "$1" 2>/dev/null \
    | awk -v c="$2" -F'\t' '$2 == c {s += $5} END {print s + 0}'
}
everyclass () { # <file> -> "bitblast defn_clausify encode replay other"
  local f=$1 bb dc en rp rest
  bb=$(classsum "$f" bitblast)
  dc=$(classsum "$f" defn_clausify)
  en=$(classsum "$f" encode)
  rp=$(classsum "$f" replay)
  rest=$(awk -f "$root/tests/ix69_phases.awk" "$f" 2>/dev/null \
    | awk -F'\t' '$2 == "initial" || $2 == "play" || $2 == "rewrite" || \
                   $2 == "finalisation" || $2 == "bridge_rewrite" || \
                   $2 == "post_solve" {s += $5} END {print s + 0}')
  printf '%s %s %s %s %s' "$bb" "$dc" "$en" "$rp" "$rest"
}

pre="$scratch/prefull/$tag.correct"
sli="$scratch/sliced/$tag.correct"
alg="$scratch/alg/$tag.correct"

read -r pbb pdc pen prp prest <<EOF
$(everyclass "$pre")
EOF
read -r sbb sdc sen srp srest <<EOF
$(everyclass "$sli")
EOF
read -r abb adc aen arp arest <<EOF
$(everyclass "$alg")
EOF

c4sat=$(wc -l < "$scratch/c4-sat/cadical-invocations.txt" 2>/dev/null || echo 0)
c4alg=$(wc -l < "$scratch/c4-alg/cadical-invocations.txt" 2>/dev/null || echo 0)
# the ALG marker is the literal line `0`; subtract it from the line count
if grep -qx '0' "$scratch/c4-alg/cadical-invocations.txt" 2>/dev/null; then c4alg=0; fi

# Work performed before the algebraic path aborted: the last mark minus the
# pre-theorem counter.  The pre-theorem counter is identical in the SAT run
# (same library and simulation chain), so it is read from the sliced log.
before=$(grep '^STAMP_INF_BEFORE' "$sli" | awk '{print $2}' | tail -1)
algwork=$(( $(lastmark "$alg") - before ))

printf '{\n'
printf '  "schema": "ix70-algebraic-close/1",\n'
printf '  "date": "2026-11-25",\n'
printf '  "branch": "staging",\n'
printf '  "kernel": "hol-light-stamp2 (axiom-stamp v2)",\n'
printf '  "measurement": "emit --mode phase (IX.66 markers); tools/cap.sh 12G; make -j1",\n'
printf '  "task": "%s",\n' "$tag"
printf '  "verdict": "does-not-prove",\n'
printf '  "baseline_full": {"total": %s, "sat_closes": %s, "bitblast": %s, "defn_clausify": %s, "encode": %s, "replay": %s, "other": %s},\n' \
  "$(total "$pre")" "$(closes "$pre")" "$pbb" "$pdc" "$pen" "$prp" "$prest"
printf '  "baseline_sliced": {"total": %s, "sat_closes": %s, "bitblast": %s, "defn_clausify": %s, "encode": %s, "replay": %s, "other": %s},\n' \
  "$(total "$sli")" "$(closes "$sli")" "$sbb" "$sdc" "$sen" "$srp" "$srest"
printf '  "algebraic": {"total": null, "sat_closes": %s, "bitblast": %s, "defn_clausify": %s, "encode": %s, "replay": %s, "work_before_abort": %s, "pre_theorem_inferences": %s, "last_mark": "%s", "last_mark_inferences": %s, "aborted": true},\n' \
  "$(closes "$alg")" "$abb" "$adc" "$aen" "$arp" "$algwork" "$before" "$(lastlabel "$alg")" "$(lastmark "$alg")"
printf '  "cadical": {"sat_path_invocations": %s, "algebraic_path_invocations": %s},\n' "$c4sat" "$c4alg"
printf '  "scope_spot_check": [\n'
first=1
for t in arm_add_p256 arm_neg_p25519 x86_neg_sm2; do
  f="$scratch/c5/$t.correct"
  [ -f "$f" ] || continue
  [ "$first" -eq 0 ] && printf ',\n'
  printf '    {"task": "%s", "total": %s, "accepted": %s}' "$t" "$(total "$f")" \
    "$(grep -q '^STAMP_BEGIN' "$f" && echo true || echo false)"
  first=0
done
printf '\n  ]\n'
printf '}\n'
