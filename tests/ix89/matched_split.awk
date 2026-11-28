# XXX.E: split a dropin-assembled generated proof into the three pieces the
# matched-session run needs, using only markers that `dropin assemble` and
# `inline_answer::render` guarantee.
#
# Input : the generated proof (.ml) written by `dropin assemble --stamp`
#         (the strict artefact, with the answer inlined as nested `let ... in`).
# Output (paths via -v header=... -v phrases=... -v task=...):
#
#   header  : the proof header (needs base.ml, ELF/EXEC/modulus setup).  In the
#             strict run this is evaluated *before* `stamp_before`, so it is
#             outside both L7 and the task total.
#   phrases : every inlined support-library phrase, one top-level `;;` phrase
#             each.  This is the support library, in the exact order and text
#             the strict run inlined.
#   task    : the task-only `let TH = time prove (<query>, <tactic>)` call with
#             the library phrases removed.  It names library values and so only
#             builds after the library has been loaded.
#
# Marker facts (inline_answer.rs render()):
#   answer   = "(\n"  phrase ("\nin\n" phrase)* "\nin\n" "(fun g ->" ... ")" "\n"
#   assemble = "let TH = time prove\n (" query ",\n" answer ");;\n"
# So the answer opens on a line that is exactly "(", the tactic function begins
# on a line that is exactly "(fun g ->", every phrase separator is a line that
# is exactly "in", and the answer closes on the line before the first ");;"
# after the tactic (a ")" line, then the ");;" line).

{ line[NR] = $0 }

END {
  hdr_end = 0; prove = 0; open_par = 0; fun_start = 0; term = 0
  for (i = 1; i <= NR; i++) {
    if (hdr_end == 0 && line[i] ~ /^let stamp_before_/) hdr_end = i - 1
    if (prove == 0 && line[i] ~ /^let .* = time prove$/) prove = i
    if (prove > 0 && open_par == 0 && line[i] == "(") open_par = i
    if (open_par > 0 && fun_start == 0 && line[i] == "(fun g ->") fun_start = i
    if (fun_start > 0 && term == 0 && line[i] == ");;") term = i
  }
  if (hdr_end == 0 || prove == 0 || open_par == 0 || fun_start == 0 || term == 0) {
    print "matched_split.awk: markers not found" > "/dev/stderr"
    exit 3
  }
  if (!(hdr_end < prove && prove < open_par && open_par < fun_start && fun_start + 1 < term)) {
    print "matched_split.awk: marker order violated" > "/dev/stderr"
    exit 4
  }
  if (line[fun_start - 1] != "in" || line[term - 1] != ")") {
    print "matched_split.awk: unexpected boundary lines" > "/dev/stderr"
    exit 4
  }

  for (i = 1; i <= hdr_end; i++) print line[i] > header

  nph = 0
  for (i = open_par + 1; i < fun_start; i++) {
    if (line[i] == "in") { print ";;" > phrases; nph++ }
    else print line[i] > phrases
  }

  for (i = prove; i < open_par; i++) print line[i] > task
  for (i = fun_start; i <= term - 2; i++) print line[i] > task
  print ");;" > task

  printf "matched_split: library_phrases=%d\n", nph > "/dev/stderr"
}
