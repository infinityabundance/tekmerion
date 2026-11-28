#!/usr/bin/perl
# XXXI.B2: build the A/B hypothesis-identity proof for one ARM task.
#
# Inputs are the three pieces `matched_split.awk` extracts from the strict,
# Tekmerion-backend generated proof (so the support library is loaded once,
# exactly as in the matched-session harness) plus the A/B harness itself:
#
#   header   the proof header (needs base.ml, machine code, EXEC, modulus)
#   phrases  the inlined support library as top-level phrases
#   task     `let TH = time prove (<query>, (fun g -> ...));;`
#   ab_lib   tests/ix89/ab_lib.ml
#
# One `task` copy is spied with the Tekmerion-owned per-step tactic
# (`TEKMERION_ARM_XACCSTEP_TAC`), the other with the upstream one
# (`ARM_XACCSTEP_TAC`).  Every step and the bridge entry are recorded; the
# harness then compares the recordings with `aconv`.
#
# usage: ab_gen.pl <header> <phrases> <task> <ab_lib> <task-id> <theorem> <source> <cache>
use strict;
use warnings;

my ($header, $phrases, $task, $ab_lib, $task_id, $theorem, $source, $cache, $do_statement) = @ARGV;
die "usage: ab_gen.pl <header> <phrases> <task> <ab_lib> <task-id> <theorem> <source> <cache> <do-statement>\n"
  unless defined $do_statement;

sub slurp {
  my ($path) = @_;
  open(my $fh, '<', $path) or die "open $path: $!";
  local $/;
  my $s = <$fh>;
  close $fh;
  return $s;
}

my $hdr = slurp($header);
my $phr = slurp($phrases);
my $tsk = slurp($task);
my $lib = slurp($ab_lib);

# Replace the single simulation call with a per-step, spied composition.  The
# replacement names the same per-step tactic the backend's `*_ACCSTEPS_TAC`
# names, so no inference is added or removed.
sub transform_task {
  my ($text, $prefix, $step_tactic, $target_name) = @_;

  my $n = 0;
  $text =~ s{(?:TEKMERION_ARM_ACCSTEPS_TAC|(?<![A-Za-z_])ARM_ACCSTEPS_TAC)\s+(\S+)\s+\[([0-9;]+)\]\s+\(1--(\d+)\)}{
      $n++;
      "ab_steps \"$prefix\" (fun n -> $step_tactic $1 [`SP`] " .
      "(mem n [$2]) (\"s\"^string_of_int n)) (1--$3)"
    }ge;

  if ($n == 0) {
    # No carry chain at all: both backends emit the plain `ARM_STEPS_TAC`.
    my $m = 0;
    $text =~ s{(?<![A-Za-z_])ARM_STEPS_TAC\s+(\S+)\s+\(1--(\d+)\)}{
        $m++;
        "ab_steps \"$prefix\" (fun n -> ARM_SINGLE_STEP_TAC $1 " .
        "(\"s\"^string_of_int n)) (1--$2)"
      }ge;
    die "ab_gen.pl: $prefix ARM_STEPS_TAC matched $m times (expected 1)\n" if $m != 1;
  } elsif ($n != 1) {
    die "ab_gen.pl: $prefix ACCSTEPS_TAC matched $n times (expected 1)\n";
  }

  my $b = 0;
  $text =~ s{(BRIDGE_[A-Z_]+_TAC \(CONV_TAC)}{$b++; "ab_spy \"$prefix:bridge\" ($1"}ge;
  die "ab_gen.pl: $prefix bridge call matched $b times (expected 1)\n" if $b != 1;

  # The bridge is the last tactic in the wrapper: `... p) g` -> `... p)) g`.
  my $closes = ($text =~ s{\) g\n(\s*with e ->)}{)) g\n$1});
  die "ab_gen.pl: $prefix bridge wrapper close matched $closes times (expected 1)\n"
    if $closes != 1;

  $text =~ s{^let [A-Z0-9_]+ = time prove$}{let $target_name = time prove}m
    or die "ab_gen.pl: no `let ... = time prove` line for $prefix\n";
  return $text;
}

my $tk = transform_task($tsk, "tk", "TEKMERION_ARM_XACCSTEP_TAC", "AB_TK_TH");
my $up = transform_task($tsk, "up", "ARM_XACCSTEP_TAC", "AB_UP_TH");

# The statement cache is keyed on the corpus id (`arm.bignum_neg_p25519`),
# not the full emitted id (`arm.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT`).
my $corpus_id = $task_id;
$corpus_id =~ s/\.[^.]*$//;

# The task-wide statement check strings are OCaml string literals; task ids,
# theorem names and relative paths are ASCII, so a plain quote is exact.
print $hdr;
print "needs \"arm/proofs/ix89_stmt_lib.ml\";;\n";
print $lib, "\n";
print "let ab_lib_before = inference_count ();;\n";
print $phr;
print "let ab_task_tk_before = inference_count ();;\n";
print "let ab_l7 = ab_task_tk_before - ab_lib_before;;\n";
print "Printf.printf \"AB_L7 %d\\n%!\" ab_l7;;\n";
print $tk, "\n";
print "let ab_tk_delta = inference_count () - ab_task_tk_before;;\n";
print "Printf.printf \"AB_INFERENCES backend=tekmerion %d\\n%!\" ab_tk_delta;;\n";
print "Printf.printf \"AB_STRICT backend=tekmerion %d\\n%!\" (ab_l7 + ab_tk_delta);;\n";
print "let ab_task_up_before = inference_count ();;\n";
print $up, "\n";
print "let ab_up_delta = inference_count () - ab_task_up_before;;\n";
print "Printf.printf \"AB_INFERENCES backend=upstream %d\\n%!\" ab_up_delta;;\n";
print "Printf.printf \"AB_STRICT backend=upstream %d\\n%!\" (ab_l7 + ab_up_delta);;\n";
print "ab_check \"$task_id\";;\n";
if ($do_statement) {
  print "ab_statement_check \"tekmerion\" \"$corpus_id\" \"$theorem\" \"$source\" \"$cache\" AB_TK_TH;;\n";
  print "ab_statement_check \"upstream\" \"$corpus_id\" \"$theorem\" \"$source\" \"$cache\" AB_UP_TH;;\n";
  print "Printf.printf \"STATEMENT_CHECK_COMPLETE task=$corpus_id\\n%!\";;\n";
} else {
  print "Printf.printf \"AB_STATEMENT_SKIPPED task=$task_id reason=no-statement-cache\\n%!\";;\n";
}
