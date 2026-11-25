# IX.69: one measured run -> one TSV row. No Python.
#
# usage: awk -f tests/ix69_collect.awk -v dir=<dir> -v tag=<tag> \
#          <tag>.correct <tag>.blocks <tag>.aux
#
# Columns:
#  1 dir            2 tag              3 theorem
#  4 total          5 initial          6 play            7 rewrite
#  8 finalisation   9 bitblast       10 defn_clausify  11 encode
# 12 replay        13 bridge_rewrite 14 post_solve     15 blast_sum
# 16 blast_list    17 clauses_sum    18 clauses_list   19 kept_str
# 20 hyp_str       21 fallbacks      22 accepted
#
# Every number comes from a line of one of the three inputs:
#   total          <- STAMP_INFERENCES  (or ".correct" absence -> accepted=0)
#   classes        <- <.tag>.blocks (tests/phases66.awk over PHASE_MARK pairs)
#   blast, clauses <- <.tag>.aux (BLAST and CNF rows of the same awk)
#   kept/total     <- "SLICE kept=k/n task=..." (tools/bridge.ml)
#   fallbacks      <- count of "SLICE fallback=..." lines
{
  if (FILENAME ~ /\.correct$/) {
    if ($1 == "STAMP_BEGIN") thm = $2;
    else if ($1 == "STAMP_INFERENCES") { total = $2 + 0; accepted = 1 }
    else if ($1 == "SLICE") {
      if ($2 ~ /^kept=/) {
        split($2, kv, "=");
        split(kv[2], kn, "/");
        nk++;
        kept_str = kept_str (nk > 1 ? " " : "") kv[2];
        hyp_str = hyp_str (nk > 1 ? " " : "") kn[2];
      } else if ($2 ~ /^fallback=/) nfb++;
    }
    next;
  }
  if (FILENAME ~ /\.blocks$/) {
    n = split($0, f, "\t");
    if (n == 5) cls[f[2]] += f[5] + 0;
    next;
  }
  if (FILENAME ~ /\.aux$/) {
    if ($1 == "BLAST") {
      nb++;
      blast_sum += $4 + 0;
      blast_list = blast_list (nb > 1 ? " " : "") $4;
    } else if ($1 == "CNF") {
      nc++;
      clauses_sum += $5 + 0;
      clauses_list = clauses_list (nc > 1 ? " " : "") $5;
    }
    next;
  }
}
END {
  printf "%s\t%s\t%s\t%d\t%d\t%d\t%d\t%d\t%d\t%d\t%d\t%d\t%d\t%d\t%d\t%s\t%d\t%s\t%s\t%s\t%d\t%d\n",
    dir, tag, thm, total,
    cls["initial"], cls["play"], cls["rewrite"], cls["finalisation"],
    cls["bitblast"], cls["defn_clausify"], cls["encode"], cls["replay"],
    cls["bridge_rewrite"], cls["post_solve"],
    blast_sum, blast_list, clauses_sum, clauses_list,
    kept_str, hyp_str, nfb, accepted + 0;
}
