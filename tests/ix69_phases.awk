# IX.66: turn one PHASE_* stream into block costs with the bridge block split.
#
# Emits TSV rows: <tag> <class> <opening-label> <closing-label> <inferences>
# Class of an interval is decided by the *pair* of labels that bound it, because
# the labels alone cannot tell a SAT close from one BITBLAST_THEN's final
# REWRITE_TAC closed (the second has no `bridge:prove-entry').
#
#   (bridge:pre-bitblast, bridge:prove-entry)  bitblast       BITBLAST_THEN
#   (bridge:pre-bitblast, bridge:sat-done)     bitblast       closed by rewriting
#   (bridge:prove-entry,  bridge:cnf-built)    defn_clausify definitionalize+CNF
#   (bridge:cnf-built,    bridge:pre-replay)   encode         DIMACS+solver+trim
#   (bridge:pre-replay,   bridge:post-replay)  replay         LRAT_REFUTE_GEN
#   (bridge:post-replay,  bridge:sat-done)     post_solve     assembly after replay
#   (bridge:sat-done,     ...)                 bridge_rewrite condition split, expand,
#   (NN bridge,           ...)                 bridge_rewrite BRIDGE_TO_WORD_CONV
#   (NN play/finalisation/rewrite, ...)        as IX.65
#
# Sizes come from PHASE_GOAL / PHASE_CNF lines and are reported per close.
# On stderr: TOTALS (before, after, total, goal, attempts, sat_closes),
# then one BLAST row per SAT close: close, blast_size, word_goal_size,
# then one CNF row per SAT close: close, atoms, clauses, cnf_term_size.

FNR == 1 {
  tag = FILENAME
  sub(/^.*\//, "", tag)
  sub(/\.correct$/, "", tag)
  n = 0; attempts = 0; closes = 0
}

function norm(l) {
  # The emitter's own marks start with two digits and a space.
  if (l ~ /^[0-9][0-9] /) return substr(l, 4)
  return l
}

function klass(a, b) {
  a = norm(a); b = norm(b)
  if (a == "initial") return "initial"
  if (a == "play") return "play"
  if (a == "finalisation" || a == "init finalisation") return "finalisation"
  if (a == "rewrite") return "rewrite"
  if (a == "bridge") return "bridge_rewrite"
  if (a == "bridge:sat-done")
    return (b == "STAMP_INF_AFTER") ? "post_solve" : "bridge_rewrite"
  if (a == "bridge:pre-bitblast") return "bitblast"
  if (a == "bridge:prove-entry") return "defn_clausify"
  if (a == "bridge:cnf-built") return "encode"
  if (a == "bridge:pre-replay") return "replay"
  if (a == "bridge:post-replay") return "post_solve"
  return "unmatched"
}

/^STAMP_INF_BEFORE / { before = $2 + 0; next }
/^STAMP_INF_AFTER /  { after = $2 + 0; next }
/^PHASE_CNF /        { cnf_atoms[closes] = $2 + 0; cnf_clauses[closes] = $3 + 0; cnf_size[closes] = $4 + 0; next }
/^PHASE_GOAL / {
  label = ""
  for (i = 2; i < NF; i++) label = label (i > 2 ? " " : "") $i
  if (label == "bridge:goal") goal = $NF + 0
  else if (label == "bridge:blast") blast[closes] = $NF + 0
  else if (label == "bridge:word-goal") wordgoal[attempts] = $NF + 0
  next
}
/^PHASE_MARK / {
  label = ""
  for (i = 2; i < NF; i++) label = label (i > 2 ? " " : "") $i
  if (norm(label) == "bridge:pre-bitblast") attempts++
  if (norm(label) == "bridge:prove-entry") closes++
  n++; lab[n] = label; cnt[n] = $NF + 0
}

END {
  # initial: STAMP_INF_BEFORE -> first mark
  print tag "\tinitial\tinitial\t" lab[1] "\t" (cnt[1] - before)
  for (i = 1; i < n; i++)
    print tag "\t" klass(lab[i], lab[i + 1]) "\t" lab[i] "\t" lab[i + 1] "\t" (cnt[i + 1] - cnt[i])
  print tag "\t" klass(lab[n], "STAMP_INF_AFTER") "\t" lab[n] "\tSTAMP_INF_AFTER\t" (after - cnt[n])
  printf "TOTAL\t%s\t%d\t%d\t%d\t%d\t%d\t%d\n", tag, before, after, after - before, goal, attempts, closes > "/dev/stderr"
  for (i = 1; i <= attempts; i++)
    printf "ATT\t%s\t%d\t%d\n", tag, i, wordgoal[i] > "/dev/stderr"
  for (i = 1; i <= closes; i++)
    printf "BLAST\t%s\t%d\t%d\n", tag, i, blast[i] > "/dev/stderr"
  for (i = 1; i <= closes; i++)
    printf "CNF\t%s\t%d\t%d\t%d\n", tag, i, cnf_atoms[i], cnf_clauses[i] > "/dev/stderr"
}
