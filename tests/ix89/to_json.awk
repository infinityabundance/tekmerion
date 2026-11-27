#!/usr/bin/awk -f
# IX.89: convert the sectioned TSV emitted by parse_human.awk / parse_gen.awk
# / parse_sat.awk into one JSON object.  Every number therefore comes from the
# parser, which reads the `.correct` file; nothing is typed by hand.
function esc(s) { gsub(/\\/, "\\\\", s); gsub(/"/, "\\\"", s); return s }
/^# SUMMARY/ { sec = "S"; next }
/^# PHASE/   { sec = "P"; next }
/^# TACTIC/  { sec = "T"; next }
/^# SEGMENTS/{ sec = "G"; next }
/^# ITEM/    { sec = "I"; next }
{
  if (sec == "S") { sval[$1] = $2 }
  else if (sec == "P") { pval[$1] = $2; porder[++np] = $1 }
  else if (sec == "T") { tval[$1] = $2; torder[++nt] = $1 }
  else if (sec == "G") { ng++; glab[ng] = $1; gph[ng] = $2; gtac[ng] = $3; gval[ng] = $4 }
  else if (sec == "I") { ni++; ilab[ni] = $1; ival[ni] = $2 }
}
END {
  printf "{";
  printf "\"before\":%d,\"after\":%d,\"inf\":%d,\"computed\":%d,\"diff\":%d,",
         sval["before"], sval["after"], sval["inf"], sval["computed"], sval["diff"];
  printf "\"phases\":{";
  for (i = 1; i <= np; i++)
    printf "%s\"%s\":%d", (i > 1 ? "," : ""), porder[i], pval[porder[i]];
  printf "},\"tactics\":{";
  for (i = 1; i <= nt; i++)
    printf "%s\"%s\":%d", (i > 1 ? "," : ""), torder[i], tval[torder[i]];
  printf "},\"items\":{";
  for (i = 1; i <= ni; i++)
    printf "%s\"%s\":%d", (i > 1 ? "," : ""), esc(ilab[i]), ival[i];
  printf "},\"segments\":[";
  for (i = 1; i <= ng; i++)
    printf "%s{\"label\":\"%s\",\"phase\":\"%s\",\"tactic\":\"%s\",\"inferences\":%d}",
           (i > 1 ? "," : ""), esc(glab[i]), gph[i],
           (gtac[i] == "-" ? "" : gtac[i]), gval[i];
  printf "]}\n";
}
