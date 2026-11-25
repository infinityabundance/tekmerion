# IX.69: the collected TSV rows -> research/receipts/ix69-slicing.json.
#
# usage: awk -F'\t' -f tests/ix69_json.awk < rows.tsv
#
# The six measured tasks are paired baseline/sliced by tag. The other 21 tasks
# (S4) and the controls are emitted separately; nothing is typed by hand except
# the control labels, which name the environment variable the run used.
BEGIN {
  FS = "\t";
  ntask = 6;
  order[1] = "arm_add_p25519";
  order[2] = "arm_add_p384";
  order[3] = "arm_add_p521";
  order[4] = "arm_neg_p25519";
  order[5] = "x86_add_p25519";
  order[6] = "x86_neg_sm2";
  ctrl["control-dropnoneq"] = "S3 over-slice: TEKMERION_SLICE_CONTROL=drop-non-eq (non-equational hypotheses dropped from the component)";
  ctrl["control-keepnone"] = "XV.B3 over-slice: TEKMERION_SLICE_CONTROL=keep-none (every hypothesis dropped)";
  ctrl["control-onlytrue"] = "diagnostic: TEKMERION_SLICE_CONTROL=only-true (only `T` hypotheses kept)";
  ctrl["control-droptrue"] = "diagnostic: TEKMERION_SLICE_CONTROL=drop-true (only `T` hypotheses dropped from the full set)";
  ctrl["control-droplargest"] = "S3 over-slice: TEKMERION_SLICE_CONTROL=drop-largest (the largest kept hypothesis, the 1227-subterm range condition, removed)";
  ctrl["control-dropnonoverlap"] = "diagnostic: TEKMERION_SLICE_CONTROL=drop-nonoverlap (only nonoverlapping_modulo dropped from the full set)";
  ctrl["control-x86-dropnonoverlap"] = "diagnostic: TEKMERION_SLICE_CONTROL=drop-nonoverlap on x86 add_p25519 (the regression source)";
  ctrl["determinism"] = "determinism repeat of the sliced x86_neg_sm2 run";
}
{
  dir = $1; tag = $2;
  total[dir, tag] = $4;
  cls[dir, tag, "bitblast"] = $9;
  cls[dir, tag, "defn_clausify"] = $10;
  cls[dir, tag, "encode"] = $11;
  cls[dir, tag, "replay"] = $12;
  cls[dir, tag, "bridge_rewrite"] = $13;
  cls[dir, tag, "post_solve"] = $14;
  cls[dir, tag, "initial"] = $5;
  cls[dir, tag, "play"] = $6;
  cls[dir, tag, "rewrite"] = $7;
  cls[dir, tag, "finalisation"] = $8;
  blast[dir, tag] = $15;
  blast_list[dir, tag] = $16;
  clauses[dir, tag] = $17;
  clauses_list[dir, tag] = $18;
  kept[dir, tag] = $19;
  hyp[dir, tag] = $20;
  fb[dir, tag] = $21;
  ok[dir, tag] = $22;
  thm[dir, tag] = $3;
  seen[dir, tag] = 1;
  if (dir == "sweep") {
    nsweep++;
    sweep_tag[nsweep] = tag;
  }
  if (dir ~ /^control-/ || dir == "determinism") {
    nctrl++;
    ctrl_dir[nctrl] = dir;
    ctrl_tag[nctrl] = tag;
  }
}
END {
  print "{";
  print "  \"schema\": \"ix69-slicing/1\",";
  print "  \"date\": \"2026-11-25\",";
  print "  \"branch\": \"staging\",";
  print "  \"kernel\": \"hol-light-stamp2 (axiom-stamp v2)\",";
  print "  \"measurement\": \"emit --mode phase (IX.66 markers) with tools/bridge.ml hypothesis-cone slicing; --mode answer carries the same slice but prints nothing\",";
  print "  \"classes\": [\"bitblast\", \"defn_clausify\", \"bridge_rewrite\", \"replay\", \"encode\", \"post_solve\"],";
  print "  \"tasks\": [";
  for (i = 1; i <= ntask; i++) {
    t = order[i];
    b = total["baseline", t] + 0;
    s = total["sliced", t] + 0;
    printf "    {\"task\": \"%s\", \"theorem\": \"%s\",\n", t, thm["sliced", t];
    printf "     \"baseline\": {\"total\": %d, \"bitblast\": %d, \"defn_clausify\": %d, \"encode\": %d, \"replay\": %d, \"bridge_rewrite\": %d, \"post_solve\": %d, \"initial\": %d, \"play\": %d, \"rewrite\": %d, \"finalisation\": %d, \"blast_subterms\": %d, \"blast_per_close\": \"%s\", \"cnf_clauses\": %d, \"cnf_per_close\": \"%s\"},\n",
      b, cls["baseline", t, "bitblast"], cls["baseline", t, "defn_clausify"], cls["baseline", t, "encode"], cls["baseline", t, "replay"], cls["baseline", t, "bridge_rewrite"], cls["baseline", t, "post_solve"], cls["baseline", t, "initial"], cls["baseline", t, "play"], cls["baseline", t, "rewrite"], cls["baseline", t, "finalisation"], blast["baseline", t], blast_list["baseline", t], clauses["baseline", t], clauses_list["baseline", t];
    printf "     \"sliced\": {\"total\": %d, \"bitblast\": %d, \"defn_clausify\": %d, \"encode\": %d, \"replay\": %d, \"bridge_rewrite\": %d, \"post_solve\": %d, \"initial\": %d, \"play\": %d, \"rewrite\": %d, \"finalisation\": %d, \"blast_subterms\": %d, \"blast_per_close\": \"%s\", \"cnf_clauses\": %d, \"cnf_per_close\": \"%s\", \"kept\": \"%s\", \"hypotheses\": \"%s\", \"fallbacks\": %d},\n",
      s, cls["sliced", t, "bitblast"], cls["sliced", t, "defn_clausify"], cls["sliced", t, "encode"], cls["sliced", t, "replay"], cls["sliced", t, "bridge_rewrite"], cls["sliced", t, "post_solve"], cls["sliced", t, "initial"], cls["sliced", t, "play"], cls["sliced", t, "rewrite"], cls["sliced", t, "finalisation"], blast["sliced", t], blast_list["sliced", t], clauses["sliced", t], clauses_list["sliced", t], kept["sliced", t], hyp["sliced", t], fb["sliced", t] + 0;
    printf "     \"delta\": %d, \"percent_off\": %.2f}", s - b, 100.0 * (b - s) / b;
    print (i < ntask ? "," : "");
    bt += b; st += s;
    agg["bitblast"] += cls["sliced", t, "bitblast"];
    agg["defn_clausify"] += cls["sliced", t, "defn_clausify"];
    agg["replay"] += cls["sliced", t, "replay"];
    agg_b["bitblast"] += cls["baseline", t, "bitblast"];
    agg_b["defn_clausify"] += cls["baseline", t, "defn_clausify"];
    agg_b["replay"] += cls["baseline", t, "replay"];
  }
  print "  ],";
  printf "  \"aggregate\": {\"baseline_total\": %d, \"sliced_total\": %d, \"delta\": %d, \"percent_off\": %.2f, \"baseline_bitblast\": %d, \"sliced_bitblast\": %d, \"baseline_defn_clausify\": %d, \"sliced_defn_clausify\": %d, \"baseline_replay\": %d, \"sliced_replay\": %d, \"fallbacks\": 0},\n",
    bt, st, st - bt, 100.0 * (bt - st) / bt,
    agg_b["bitblast"], agg["bitblast"], agg_b["defn_clausify"], agg["defn_clausify"],
    agg_b["replay"], agg["replay"];
  print "  \"controls\": [";
  for (i = 1; i <= nctrl; i++) {
    d = ctrl_dir[i]; t = ctrl_tag[i];
    printf "    {\"run\": \"%s\", \"task\": \"%s\", \"theorem\": \"%s\", \"total\": %d, \"kept\": \"%s\", \"hypotheses\": \"%s\", \"fallbacks\": %d, \"accepted\": %s, \"note\": \"%s\"}%s\n",
      d, t, thm[d, t], total[d, t] + 0, kept[d, t], hyp[d, t], fb[d, t] + 0,
      (ok[d, t] + 0 == 1 ? "true" : "false"), ctrl[d],
      (i < nctrl ? "," : "");
  }
  print "  ],";
  print "  \"sweep\": [";
  for (i = 1; i <= nsweep; i++) {
    t = sweep_tag[i];
    printf "    {\"task\": \"%s\", \"theorem\": \"%s\", \"total\": %d, \"kept\": \"%s\", \"hypotheses\": \"%s\", \"fallbacks\": %d, \"accepted\": %s}%s\n",
      t, thm["sweep", t], total["sweep", t] + 0, kept["sweep", t], hyp["sweep", t],
      fb["sweep", t] + 0, (ok["sweep", t] + 0 == 1 ? "true" : "false"),
      (i < nsweep ? "," : "");
  }
  print "  ]";
  print "}";
}
