# C-SAT certified close -- measured

Numbers below come from this run's HOL output; none are hand-typed.

- cadical: `/home/one/.elan/toolchains/leanprover--lean4---v4.29.1/bin/cadical`
- verdict: **not closed: every post-simulation stage left CaDiCaL SATISFIED (num/real/MOD content is not bit-blasted)**

## Tactic sequence

1. `BITBLAST_THEN (CONV_TAC o K CSAT_PROVE)`
1. `CSAT_PROVE = CADICAL_PROVE with CSAT_REFUTE_GEN substituted`
1. `CSAT_REFUTE_GEN: dimacs_of_hol -> cadical --no-binary --lrat -> LRAT_REFUTE_GEN (kernel replay)`

## Probe: raw certified-SAT close on word tautologies

Timeout reference: 120 s (the P-256 add/sub/neg per-task timeout in `.bench/timeout-map.json`).

| lemma | closed | close s | timeout s | headroom s | note |
|---|---|---|---|---|---|
| word64_carry | True | 0.807 | 120 | 119.193 |  |
| word128_add_comm | True | 1.410 | 120 | 118.590 |  |
| word256_add_comm | True | 2.674 | 120 | 117.326 |  |
| word256_carry | True | 4.673 | 120 | 115.327 |  |
| word320_carry | True | 5.858 | 120 | 114.142 |  |

Probe process wall-clock: 77.6s (`/mnt/1tb_kingston/Tekmerion/.bench/verify/csat/csat_probe.log`)

## Post-simulation goals (P-256)

| op | stage | closed | close s | timeout s | headroom s | reason |
|---|---|---|---|---|---|---|
| add | add.raw | False | 0.038 | 120 | n/a | CSAT: cadical did not report UNSAT, exit 10 |
| add | add.ensures | False | 0.003 | 120 | n/a | CSAT: cadical did not report UNSAT, exit 10 |
| add | add.expand | False | 0.007 | 120 | n/a | CSAT: cadical did not report UNSAT, exit 10 |
| sub | sub.raw | False | 0.036 | 120 | n/a | CSAT: cadical did not report UNSAT, exit 10 |
| sub | sub.ensures | False | 0.003 | 120 | n/a | CSAT: cadical did not report UNSAT, exit 10 |
| sub | sub.expand | False | 0.006 | 120 | n/a | CSAT: cadical did not report UNSAT, exit 10 |
| neg | neg.raw | False | 0.036 | 120 | n/a | CSAT: cadical did not report UNSAT, exit 10 |
| neg | neg.ensures | False | 0.003 | 120 | n/a | CSAT: cadical did not report UNSAT, exit 10 |
| neg | neg.expand | False | 0.003 | 120 | n/a | CSAT: cadical did not report UNSAT, exit 10 |

Post process wall-clock: 166.8s

## Notes

- A `closed=0` row is a measured negative: CaDiCaL found the negated goal SATISFIABLE, so the kernel refused it.
- Raw bit-blast replay (probe) is fast enough; the blocker is the num/real/`MOD` bridge lemma (TODO C1), not replay speed.
