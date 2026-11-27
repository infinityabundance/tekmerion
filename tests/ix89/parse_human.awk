#!/usr/bin/awk -f
# IX.89: turn one human phase run (.correct) into segment / phase / tactic
# tables.  A segment is charged to the label of the marker that starts it;
# the last segment is closed by HUMAN_CORRECT_AFTER.  The pre-first-marker
# interval is the construction bucket (tactic-argument evaluation), reported
# as phase PRE.
#
# Labels are `<phase>:<item>[:<TACTIC>]`; the tactic token is the text after
# the last ':' with any trailing `.<n>` removed, and is recognised only when
# it is one of the named decision procedures.

function tactic_of(label,  tok) {
  tok = label
  sub(/.*:/, "", tok)
  sub(/\.[0-9]+$/, "", tok)
  if (tok == "ARITH_TAC" || tok == "REAL_ARITH_TAC" ||
      tok == "INT_ARITH_TAC" || tok == "REAL_INTEGER_TAC" ||
      tok == "ASM_ARITH_TAC" || tok == "NUM_REDUCE_CONV" ||
      tok == "BOUNDER_TAC" || tok == "REAL_RAT_REDUCE_CONV")
    return tok
  return ""
}
function phase_of(label,  p) {
  p = label
  sub(/:.*/, "", p)
  return p
}

/HUMAN_CORRECT_BEFORE/ { before = $NF }
/HUMAN_CORRECT_AFTER/  { after  = $NF }
/HUMAN_CORRECT_INF/    { inf    = $NF }
/^HPHASE / { n++; lab[n] = $2; cnt[n] = $3 + 0 }

END {
  # construction bucket
  seglabel[0] = "PRE:construct"; segphase[0] = "PRE"; segtac[0] = ""
  segval[0] = cnt[1] - before
  for (i = 1; i < n; i++) {
    seglabel[i] = lab[i]
    segphase[i] = phase_of(lab[i])
    segtac[i] = tactic_of(lab[i])
    segval[i] = cnt[i + 1] - cnt[i]
  }
  seglabel[n] = lab[n]
  segphase[n] = phase_of(lab[n])
  segtac[n] = tactic_of(lab[n])
  segval[n] = after - cnt[n]

  total = 0
  for (i = 0; i <= n; i++) {
    total += segval[i]
    ph[segphase[i]] += segval[i]
    if (segtac[i] != "") tac[segtac[i]] += segval[i]
  }

  printf "# SUMMARY\n"
  printf "before\t%d\nafter\t%d\ninf\t%d\ncomputed\t%d\ndiff\t%d\n", \
         before, after, inf, total, total - inf
  printf "# PHASE\n"
  split("PRE A B C D E", order, " ")
  for (j = 1; j <= 6; j++) if (order[j] in ph)
    printf "%s\t%d\n", order[j], ph[order[j]]
  printf "# TACTIC\n"
  split("ARITH_TAC REAL_ARITH_TAC INT_ARITH_TAC REAL_INTEGER_TAC ASM_ARITH_TAC NUM_REDUCE_CONV BOUNDER_TAC REAL_RAT_REDUCE_CONV", torder, " ")
  for (j = 1; j <= 8; j++) if (torder[j] in tac)
    printf "%s\t%d\n", torder[j], tac[torder[j]]
  printf "# SEGMENTS\n"
  for (i = 0; i <= n; i++)
    printf "%s\t%s\t%s\t%d\n", seglabel[i], segphase[i], \
           (segtac[i] == "" ? "-" : segtac[i]), segval[i]
}
