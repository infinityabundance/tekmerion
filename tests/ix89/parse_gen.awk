#!/usr/bin/awk -f
# IX.89: turn one generated (algebraic-close) phase run into phase / tactic /
# item tables.  Segments are charged to the label of the marker that starts
# them; the last is closed by STAMP_INF_AFTER.  The interval before the first
# marker is the support-library construction bucket (PRE).
#
# Label sources: the emitter's `NN <class>` skeleton marks, the bridge's
# `bridge:*` oracle marks, the per-instruction `B:stepNN:<acc|plain>` marks and
# the per-tactic `G:<TACTIC>:<site>-<start|end>` marks added by instr_gen.pl.

function mid_of(label,  t) {
  t = label
  sub(/^G:/, "", t)
  sub(/:.*/, "", t)
  return t
}
function is_tac(t) {
  return (t == "INT_RING" || t == "INT_ARITH" || t == "INT_REDUCE")
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
  # Segment 0: support-library construction, from before to the first mark.
  slabel[0] = "PRE:lib"; sphase[0] = "PRE"; stac[0] = ""; sval[0] = cnt[1] - before
  seen_play = 0
  for (i = 1; i < n; i++) {
    slabel[i] = lab[i]
    stac[i] = ""
    if (lab[i] ~ /^B:step/ || lab[i] ~ / play$/) {
      sphase[i] = "B"
      if (lab[i] ~ / play$/) seen_play = 1
    } else if (lab[i] ~ /^G:/) {
      t = mid_of(lab[i]); stac[i] = t
      if (t == "INT_RING") sphase[i] = "E"
      else if (t == "INT_REDUCE") sphase[i] = "C"
      else if (t == "INT_ARITH") {
        if (lab[i] ~ /:(band|fact)/) sphase[i] = "C"; else sphase[i] = "D"
      } else sphase[i] = "E"
    } else if (lab[i] == "bridge:post-flat") { sphase[i] = "C"
    } else if (lab[i] == "bridge:pre-close") { sphase[i] = "D"
    } else if (lab[i] ~ /^[0-9][0-9] / || lab[i] ~ /^init /) {
      sphase[i] = seen_play ? "E" : "A"
    } else { sphase[i] = "E" }
    sval[i] = cnt[i + 1] - cnt[i]
  }
  slabel[n] = lab[n]; stac[n] = ""; sval[n] = after - cnt[n]
  if (lab[n] ~ /^G:/) { stac[n] = mid_of(lab[n]); sphase[n] = "E" }
  else if (lab[n] == "bridge:pre-close") sphase[n] = "D"
  else if (lab[n] == "bridge:post-flat") sphase[n] = "C"
  else sphase[n] = "E"

  total = 0
  for (i = 0; i <= n; i++) {
    total += sval[i]
    ph[sphase[i]] += sval[i]
    if (is_tac(stac[i])) tac[stac[i]] += sval[i]
    if (!(slabel[i] in seen)) { seen[slabel[i]] = 1; iorder[++ni] = slabel[i] }
    item[slabel[i]] += sval[i]
  }

  printf "# SUMMARY\n"
  printf "before\t%d\nafter\t%d\ninf\t%d\ncomputed\t%d\ndiff\t%d\n", before, after, inf, total, total - inf
  printf "# PHASE\n"
  split("PRE A B C D E", order, " ")
  for (j = 1; j <= 6; j++) if (order[j] in ph) printf "%s\t%d\n", order[j], ph[order[j]]
  printf "# TACTIC\n"
  split("INT_ARITH INT_RING INT_REDUCE", torder, " ")
  for (j = 1; j <= 3; j++) printf "%s\t%d\n", torder[j], tac[torder[j]] + 0
  printf "# ITEM\n"
  for (j = 1; j <= ni; j++) printf "%s\t%d\t%d\n", iorder[j], item[iorder[j]], -1
  # preamble: before -> first bridge:algebra (support library + skeleton + entry)
  for (i = 1; i < n; i++) if (lab[i] == "bridge:algebra") { preamble = cnt[i] - before; break }
  printf "preamble\t%d\t-1\n", preamble + 0
}
