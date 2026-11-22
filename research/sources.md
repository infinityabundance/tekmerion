# Sources and reproduction

All repository facts below were measured on 2026-11-22, not taken from prose.

## Pinned checkouts

| Repository | Commit / ref | Used for |
|---|---|---|
| `github.com/awslabs/s2n-bignum` | `1d097f630f0ca91cc2271491b9dd57dc9c9023a5` (`--depth 1`) | proof line counts, corpus size, absence of byte-digest checks, `SOUNDNESS.md` |
| `github.com/jrh13/hol-light` | `master`, file `fusion.ml` fetched directly | kernel line count, primitives, axioms |
| `github.com/hol-light-family/TacticTrace` | `main`, `--depth 1` | record schema, sampling, failure policy, wrapper generator |
| `github.com/kings-crown/s2n-bignum-bench` | `main`, `--depth 1` | task format, verdicts, cheating oracle, baseline prompt, obfuscation |
| `github.com/kings-crown/s2n-bignum-leaderboard` | `master` | category counts, submission rules, leaderboard state |

## Commands used for the repository claims

```sh
# Kernel size, two legitimate definitions
curl -sSL https://raw.githubusercontent.com/jrh13/hol-light/master/fusion.ml -o fusion.ml
wc -l fusion.ml
grep -vE '^\s*$|^\s*\(\*' fusion.ml | wc -l
grep -nE 'new_axiom' fusion.ml

# Proof size for the flagship routine
git clone --depth 1 https://github.com/awslabs/s2n-bignum
cd s2n-bignum && git rev-parse HEAD
for f in arm/proofs/bignum_inv_p25519.ml x86/proofs/bignum_inv_p25519.ml; do
  printf '%s code / %s total  %s\n' \
    "$(grep -vcE '^\s*$|^\s*\(\*' $f)" "$(wc -l < $f)" "$f"
done
grep -nE 'BIGNUM_INV_P25519_(SUBROUTINE_CORRECT|SAFE|SUBROUTINE_SAFE)' \
  arm/proofs/bignum_inv_p25519.ml

# Corpus size and the missing byte-digest checks
ls arm/proofs/*.ml x86/proofs/*.ml | wc -l
grep -rniE 'sha256|md5|digest|checksum|watermark' tools/ Makefile arm/Makefile

# Trace record schema and the failure policy
grep -n 'type tac_record' -A 8 tactictrace/exportTrace.ml
grep -n 'fails writes no traces' tactictrace/README.md

# Cheating oracle and baseline prompt
grep -rn 'axioms_before' bench/assess_answer.py
cat bench/prompt-basic.txt
```

## Documents fetched

- s2n-bignum `SOUNDNESS.md`:
  <https://raw.githubusercontent.com/awslabs/s2n-bignum/main/SOUNDNESS.md>
  — B1 co-simulation, A2/D2/D3 C-level gap, A3 sequential-only constant time, A4 stack
  discipline unchecked, B3 speculative execution excluded, C1 trusts OCaml, reassembly
  caveat.
- s2n-bignum-bench NeurIPS 2026 poster:
  <https://neurips.cc/virtual/2026/poster/139651>
  — 2,301 tasks, 6.35% single attempt, 10.26% Pass@5 with feedback, 0/864 functional
  correctness, v1.1 2,622 tasks and 5 FC successes.
- Benchmark repository: <https://github.com/kings-crown/s2n-bignum-bench> — workflow,
  verdicts, obfuscation, checkpointed Pass@K.
- Leaderboard: <https://kings-crown.github.io/s2n-bignum-leaderboard/> and
  <https://github.com/kings-crown/s2n-bignum-leaderboard> — category table, submission
  process, and the absence of `results.json` on `master`.
- CVE-2026-72844 advisory:
  <https://openwall.com/lists/oss-security/2026/10/26/2>
  — projection type confusion (CWE-843) plus unchecked nested inductive parameters
  (CWE-20); Nanoda and Lean4Lean equally affected; Docker PoC; CVSS 3.1 7.1.
- `TacticTrace`: <https://github.com/hol-light-family/TacticTrace> — success-only tracing,
  sampling, wrapper generation.

## Literature consulted through the arXiv API

- `2611.13627` TopoVerus — curriculum proof repair with learned counterexamples;
  61.6/72.2 Safe-Pass@1/@3 on 471 Verus problems.
- `2611.00287` VerusWeave — 75–85% focused, 29–47% full-module proof repair.
- `2610.36669` Proof repair as (co)algebra equivalence — foundational treatment.
- `2608.13459` CAPRI — contract-aware repair; 138/180 valid repairs, 32/36 frozen
  iterative; catches repairs that modified protected text.
- `2608.18822` Contract-aware rescue of a drifted Isabelle development — a weakened
  assurance theorem found by audit.
- `2608.26183` Refusal-site mutation testing of an evidence-bundle verifier — 112 refusal
  sites, 75 deletable unnoticed, score 0.330; and the explicit warning that self-measured
  numbers over a closed loop of one's own repositories are the credibility, not a caveat.
- `2606.28841` LAMP; `2606.26442` AXLE; `2606.13473` MaxProof; `2602.17016` M2F;
  `2601.04653` Isabellm; `2605.14549` CSLibPremiseBench — the density of the 2026 LLM-proving
  field.
- `2602.02990` APRIL — 260k supervised proof-repair tuples.
- `2310.06959` Baldur — whole-proof generation plus repair; +8.7% over Thor.
- `2010.00774` Proof repair across type equivalences (PUMPKIN Pi).
- `2503.01538` NACT — formal specification mutation for protocol testing.

## Not verified

- The `2,633`-line figure for `bignum_inv_p25519`. It does not reproduce at `1d097f6`
  under either counting rule (3,624/3,346 arm; 4,617/4,090 x86). Do not cite until pinned.
- Metamath Zero's <200 ms `set.mm` check and hol2dk's ~4 min `hol.ml` check. Taken from the
  brief; not measured here.
- Per-category leaderboard values (client-side rendering; no results file on `master`).
- Candle's supported fragment of the HOL Light libraries s2n-bignum depends on.
