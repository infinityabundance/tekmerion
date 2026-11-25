# IX.69: research/receipts/ix69-slicing.json -> the receipt's markdown tables.
#
# usage: awk -f tests/ix69_table.awk research/receipts/ix69-slicing.json
#
# Every printed number is read back out of the JSON; nothing is recomputed from
# the logs here. That is the point: the receipt's table is a rendering of the
# committed JSON, and the JSON is generated from the .correct runs.
function num(key,   s) {
  if (match($0, "\"" key "\": [0-9]+")) {
    s = substr($0, RSTART, RLENGTH);
    sub(/^.*: /, "", s);
    return s + 0;
  }
  return -1;
}
function str(key,   s) {
  if (match($0, "\"" key "\": \"[^\"]*\"")) {
    s = substr($0, RSTART, RLENGTH);
    sub(/^.*: "/, "", s);
    sub(/"$/, "", s);
    return s;
  }
  return "";
}
function wordv(key,   s) {
  if (match($0, "\"" key "\": [a-z]+")) {
    s = substr($0, RSTART, RLENGTH);
    sub(/^.*: /, "", s);
    return s;
  }
  return "";
}
function signed(n) { return (n > 0 ? "+" : "") n }
/^  "tasks": \[/ { section = "tasks"; next }
/^  "controls": \[/ { section = "controls"; next }
/^  "sweep": \[/ { section = "sweep"; next }
/^  "aggregate":/ { agg = $0; next }
section == "tasks" {
  if ($0 ~ /"task":/) task = str("task");
  if ($0 ~ /"baseline":/) {
    b_total = num("total"); b_bb = num("bitblast"); b_dc = num("defn_clausify");
    b_rp = num("replay"); b_blast = num("blast_subterms"); b_clauses = num("cnf_clauses");
    next;
  }
  if ($0 ~ /"sliced":/) {
    s_total = num("total"); s_bb = num("bitblast"); s_dc = num("defn_clausify");
    s_rp = num("replay"); s_blast = num("blast_subterms"); s_clauses = num("cnf_clauses");
    kept = str("kept"); fb = num("fallbacks");
    if (!header) {
      print "| task | total before | total after | delta | off | kept / hypotheses | blasted subterms | CNF clauses | bitblast | defn+clausify | replay | fallbacks |";
      print "|---|---:|---:|---:|---:|---|---:|---:|---:|---:|---:|---:|";
      header = 1;
    }
    printf "| `%s` | %d | %d | %s | %.2f%% | %s | %d -> %d | %d -> %d | %d -> %d | %d -> %d | %d -> %d | %d |\n",
      task, b_total, s_total, signed(s_total - b_total),
      100.0 * (b_total - s_total) / b_total, kept,
      b_blast, s_blast, b_clauses, s_clauses, b_bb, s_bb, b_dc, s_dc, b_rp, s_rp, fb;
    next;
  }
}
section == "controls" {
  if ($0 ~ /"run":/) {
    if (!cheader) {
      print "| run | task | total | kept / hypotheses | fallbacks | accepted | note |";
      print "|---|---|---:|---|---:|---|---|";
      cheader = 1;
    }
    printf "| `%s` | `%s` | %d | %s / %s | %d | %s | %s |\n",
      str("run"), str("task"), num("total"), str("kept"), str("hypotheses"),
      num("fallbacks"), (wordv("accepted") == "true" ? "yes" : "NO"), str("note");
    next;
  }
}
section == "sweep" {
  if ($0 ~ /"task":/) {
    if (!sheader) {
      print "| task | total | kept / hypotheses | fallbacks | accepted |";
      print "|---|---:|---|---:|---|";
      sheader = 1;
    }
    printf "| `%s` | %d | %s / %s | %d | %s |\n",
      str("task"), num("total"), str("kept"), str("hypotheses"),
      num("fallbacks"), (wordv("accepted") == "true" ? "yes" : "NO");
    next;
  }
}
END {
  if (agg != "") {
    $0 = agg;
    print "";
    printf "Six-task aggregate: **%d -> %d** (%s, %.2f%% off).\n",
      num("baseline_total"), num("sliced_total"),
      signed(num("sliced_total") - num("baseline_total")), 100.0 * (num("baseline_total") - num("sliced_total")) / num("baseline_total");
  }
}
