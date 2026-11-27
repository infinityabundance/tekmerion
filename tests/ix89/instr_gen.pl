#!/usr/bin/perl
# IX.89: add per-instruction simulation markers and per-tactic arithmetic
# markers to a `--mode phase` generated answer.  The answer is scratch; the
# tracked emitter is not touched.  Every insertion is a `csat_mark`/`csat_mark_at`
# identity, so the proof and its inference count are unchanged (checked by the
# ablation run).
#
# usage: instr_gen.pl <in.answer> <out.answer>
use strict;
use warnings;

my ($in, $out) = @ARGV;
die "usage: instr_gen.pl <in.answer> <out.answer>\n" unless defined $out;
local $/;
open(my $fh, '<', $in) or die "open $in: $!";
my $s = <$fh>;
close $fh;

# 1. Split the single ARM_ACCSTEPS_TAC into one call per instruction, with a
#    marker naming the step and whether it is an accumulated (carry-producing)
#    or a plain step.
my $sim_done = 0;
$s =~ s{csat_mark "(\d\d) play" THEN ARM_ACCSTEPS_TAC (\S+) \[([0-9;]+)\] \(1--(\d+)\) THEN}{
    my ($mk, $exec, $anums, $m) = ($1, $2, $3, $4);
    my %acc = map { $_ => 1 } split /;/, $anums;
    my $o = "csat_mark \"$mk play\" THEN\n";
    for my $n (1 .. $m) {
        my $cls = $acc{$n} ? "acc" : "plain";
        $o .= sprintf("csat_mark \"B:step%02d:%s\" THEN ARM_ACCSTEPS_TAC %s [%s] (%d--%d) THEN\n",
                      $n, $cls, $exec, $anums, $n, $n);
    }
    $o;
}se;
$sim_done = ($s =~ /B:step01:/) ? 1 : 0;
die "simulation line not found\n" unless $sim_done;

sub repl1 {
    my ($src, $old, $new, $what) = @_;
    my $n = ($src =~ s/\Q$old\E/$new/);
    die "anchor for $what occurs $n times (expected 1)\n" unless $n == 1;
    return $src;
}

# 2. Aggregate construction: weighted sum proved by INT_RING.
$s = repl1($s,
"         let th_ring =
           prove(mk_eq(mk_binop sub l r, mk_binop sub l' r'),
                 CONV_TAC INT_RING) in",
"         let _ = csat_mark_at \"G:INT_RING:agg-start\" in
         let th_ring =
           prove(mk_eq(mk_binop sub l r, mk_binop sub l' r'),
                 CONV_TAC INT_RING) in
         let _ = csat_mark_at \"G:INT_RING:agg-end\" in",
"aggregate INT_RING");

# 3. Hypothesis flattening: each integer subterm canonicalised by INT_RING.
$s = repl1($s,
"         else (try prove(mk_eq(tm, flat), CONV_TAC INT_RING)
               with _ -> REFL tm)",
"         else (try
                 let _ = csat_mark_at \"G:INT_RING:flat-start\" in
                 let th = prove(mk_eq(tm, flat), CONV_TAC INT_RING) in
                 let _ = csat_mark_at \"G:INT_RING:flat-end\" in
                 th
               with _ -> REFL tm)",
"flatten INT_RING");

# 4. Carry-band fact: `v < 1' by INT_ARITH.
$s = repl1($s,
"      let th = prove(bridge_mk_imps premises
                       (mk_binop lt_tm v (bridge_int_lit num_1)),
                     INT_ARITH_TAC) in",
"      let _ = csat_mark_at \"G:INT_ARITH:band-start\" in
      let th = prove(bridge_mk_imps premises
                       (mk_binop lt_tm v (bridge_int_lit num_1)),
                     INT_ARITH_TAC) in
      let _ = csat_mark_at \"G:INT_ARITH:band-end\" in",
"band INT_ARITH");

# 5. Bitval case finish: `v < 1 ==> v = 0' by case split + INT_REDUCE.
$s = repl1($s,
"      let th2 = prove(mk_imp (concl th, mk_eq(v, bridge_int_lit num_0)),
                      ASM_CASES_TAC b THEN
                      ASM_REWRITE_TAC[BITVAL_CLAUSES] THEN
                      CONV_TAC INT_REDUCE_CONV) in",
"      let _ = csat_mark_at \"G:INT_REDUCE:finish-start\" in
      let th2 = prove(mk_imp (concl th, mk_eq(v, bridge_int_lit num_0)),
                      ASM_CASES_TAC b THEN
                      ASM_REWRITE_TAC[BITVAL_CLAUSES] THEN
                      CONV_TAC INT_REDUCE_CONV) in
      let _ = csat_mark_at \"G:INT_REDUCE:finish-end\" in",
"finish INT_REDUCE");

# 6. Pinned fact: `v = k' by INT_ARITH.
$s = repl1($s,
"      let th = prove(bridge_mk_imps premises (mk_eq(v, bridge_int_lit k)),
                     INT_ARITH_TAC) in",
"      let _ = csat_mark_at \"G:INT_ARITH:fact-start\" in
      let th = prove(bridge_mk_imps premises (mk_eq(v, bridge_int_lit k)),
                     INT_ARITH_TAC) in
      let _ = csat_mark_at \"G:INT_ARITH:fact-end\" in",
"pinned fact INT_ARITH");

# 7. Linear close: the branch system closed by INT_ARITH.
$s = repl1($s,
"      csat_mark_at \"bridge:pre-close\";
      let th = prove(bridge_mk_imps ants w', INT_ARITH_TAC) in",
"      csat_mark_at \"bridge:pre-close\";
      let _ = csat_mark_at \"G:INT_ARITH:close-start\" in
      let th = prove(bridge_mk_imps ants w', INT_ARITH_TAC) in
      let _ = csat_mark_at \"G:INT_ARITH:close-end\" in",
"linear close INT_ARITH");

open(my $oh, '>', $out) or die "open $out: $!";
print $oh $s;
close $oh;
print "instrumented generated answer -> $out\n";
