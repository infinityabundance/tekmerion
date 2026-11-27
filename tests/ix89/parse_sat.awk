#!/usr/bin/awk -f
# IX.89: split one SAT-route (.correct) run into the same coarse phases used for
# the algebraic route (A init+digitize, B simulation, E everything else) plus
# the SAT-close items bounding-boxed by the IX.66 markers.
#
# The skeleton is the same as the algebraic answer, so the `play` mark bounds
# phase B; there is no per-instruction split on the SAT route (its simulation
# tactic is one ARM_ACCSTEPS_TAC call and it is not the lever under test).
function phase_mark(lab) {
  if (lab ~ / play$/) return "B"
  if (lab ~ /^[0-9][0-9] / || lab ~ /^init /) return (seen_play ? "E" : "A")
  return "E"
}
/STAMP_INF_BEFORE/ { before = $NF }
/STAMP_INF_AFTER/  { after  = $NF }
/STAMP_INFERENCES/ { inf    = $NF }
/^PHASE_MARK / {
  label = ""
  for (i = 2; i < NF; i++) label = label (i > 2 ? " " : "") $i
  n++; lab[n] = label; cnt[n] = $NF + 0
}
END {
  # Segment 0: construction (tactic-argument evaluation).
  sphase[0] = "PRE"; sval[0] = cnt[1] - before
  seen_play = 0
  preamble_done = 0
  for (i = 1; i < n; i++) {
    if (lab[i] ~ / play$/) seen_play = 1
    sphase[i] = phase_mark(lab[i])
    sval[i] = cnt[i + 1] - cnt[i]
    if (!preamble_done && lab[i + 1] == "bridge:pre-bitblast") {
      # preamble = everything from the first mark to the first SAT close entry
      item["preamble"] = cnt[i + 1] - cnt[1]
      preamble_done = 1
    }
    if (lab[i] == "bridge:pre-bitblast" && lab[i + 1] == "bridge:prove-entry")
      item["bitblast"] += sval[i]
    if (lab[i] == "bridge:prove-entry" && lab[i + 1] == "bridge:cnf-built")
      item["clausify"] += sval[i]
    if (lab[i] == "bridge:cnf-built" && lab[i + 1] == "bridge:pre-replay")
      item["dimacs+search"] += sval[i]
    if (lab[i] == "bridge:pre-replay" && lab[i + 1] == "bridge:post-replay")
      item["replay"] += sval[i]
    if (lab[i] == "bridge:post-replay" && lab[i + 1] == "bridge:sat-done")
      item["assemble"] += sval[i]
  }
  sphase[n] = phase_mark(lab[n]); sval[n] = after - cnt[n]
  total = 0
  for (i = 0; i <= n; i++) { total += sval[i]; ph[sphase[i]] += sval[i] }
  item["tail"] = after - cnt[n]
  printf "# SUMMARY\n"
  printf "before\t%d\nafter\t%d\ninf\t%d\ncomputed\t%d\ndiff\t%d\n",
         before, after, inf, total, total - inf
  printf "# PHASE\n"
  split("PRE A B E", order, " ")
  for (j = 1; j <= 4; j++) if (order[j] in ph) printf "%s\t%d\n", order[j], ph[order[j]]
  printf "# TACTIC\n"
  printf "# ITEM\n"
  split("preamble bitblast clausify dimacs+search replay assemble tail", iorder, " ")
  for (j = 1; j <= 7; j++) printf "%s\t%d\t-1\n", iorder[j], item[iorder[j]] + 0
}
