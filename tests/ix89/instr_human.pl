#!/usr/bin/perl
# IX.89: instrument the upstream human `_CORRECT` proof body for phase
# measurement.  The theorem statement is copied verbatim; only the tactic is
# replaced (phase mode) or kept as-is (plain mode).  `plain` re-emits the
# untouched tactic wrapped in before/after inference counters, so it is the
# ablation baseline for the phase run.
#
# usage: instr_human.pl <plain|phase> <pristine.ml> <body.tac> <TH> <out.ml>
use strict;
use warnings;

my ($mode, $ml, $bodyf, $th, $out) = @ARGV;
die "usage: instr_human.pl <plain|phase> <pristine.ml> <body.tac> <TH> <out.ml>\n"
    unless defined $out;

local $/;
open(my $fh, '<', $ml) or die "open $ml: $!";
my $src = <$fh>;
close $fh;

my ($term, $orig);
my $pat = qr/let \Q$th\E = time prove\s*\(\s*(`[^`]*`)\s*,(.*?)\)\s*;;/s;
{
    my $copy = $src;
    $copy =~ $pat or die "theorem block not found: $th";
    $term = $1;
    $orig = $2;
}

my $helper = <<'H';
let ix89_mark label : tactic =
  fun (asl,w) ->
    (Printf.printf "HPHASE %s %d\n%!" label (inference_count ());
     ALL_TAC (asl,w));;

H

my $tac;
if ($mode eq 'plain') {
    $tac = $orig;
} elsif ($mode eq 'phase') {
    open(my $bh, '<', $bodyf) or die "open $bodyf: $!";
    $tac = <$bh>;
    close $bh;
    $tac =~ s/\s+\z//;
} else {
    die "mode must be plain or phase\n";
}

my $repl =
    ($mode eq 'phase' ? $helper : '') .
    "let ix89_stamp_before_$th = inference_count ();;\n" .
    "let $th = time prove\n ($term,\n$tac);;\n" .
    "let ix89_stamp_after_$th = inference_count ();;\n" .
    "Printf.printf \"HUMAN_CORRECT_BEFORE %d\\n%!\" ix89_stamp_before_$th;;\n" .
    "Printf.printf \"HUMAN_CORRECT_AFTER %d\\n%!\" ix89_stamp_after_$th;;\n" .
    "Printf.printf \"HUMAN_CORRECT_INF %d\\n%!\" " .
    "(ix89_stamp_after_$th - ix89_stamp_before_$th);;\n";

$src =~ s/$pat/$repl/ or die "substitution failed for $th";
open(my $oh, '>', $out) or die "open $out: $!";
print $oh $src;
close $oh;
print "instrumented $mode $th -> $out\n";
