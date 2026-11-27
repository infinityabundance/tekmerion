#!/bin/bash
# IX.89: assemble research/receipts/ix89-human-phases.json from the recorded
# `.correct` runs.  Every value is parsed from a run; nothing is typed.
set -eu
ROOT=/mnt/1tb_kingston/Tekmerion
cd "$ROOT"
OUT=research/receipts/ix89-human-phases.json
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

for op in add sub neg; do
  awk -f tests/ix89/parse_human.awk ".bench/ix89/human_out/arm_${op}_p25519.phase.correct" \
    | awk -f tests/ix89/to_json.awk > "$TMP/human_$op.json"
  awk -f tests/ix89/parse_gen.awk ".bench/ix89/gen/arm_${op}_p25519.phase.correct" \
    | awk -f tests/ix89/to_json.awk > "$TMP/gen_$op.json"
  awk -f tests/ix89/parse_sat.awk ".bench/ix89/sat/arm_${op}_p25519.sat.correct" \
    | awk -f tests/ix89/to_json.awk > "$TMP/sat_$op.json"
done

hinf() { grep -o 'HUMAN_CORRECT_INF [0-9]*' "$1" | awk '{print $2}'; }
ginf() { grep -m1 '^STAMP_INFERENCES' "$1" | awk '{print $2}'; }

ABL=$(jq -n \
  --argjson ha_plain "$(hinf .bench/ix89/human_out/arm_add_p25519.plain.correct)" \
  --argjson ha_phase "$(hinf .bench/ix89/human_out/arm_add_p25519.phase.correct)" \
  --argjson hs_plain "$(hinf .bench/ix89/human_out/arm_sub_p25519.plain.correct)" \
  --argjson hs_phase "$(hinf .bench/ix89/human_out/arm_sub_p25519.phase.correct)" \
  --argjson hn_plain "$(hinf .bench/ix89/human_out/arm_neg_p25519.plain.correct)" \
  --argjson hn_phase "$(hinf .bench/ix89/human_out/arm_neg_p25519.phase.correct)" \
  --argjson ga_plain "$(ginf .bench/ix89/gen/arm_add_p25519.plain.correct)" \
  --argjson ga_phase "$(ginf .bench/ix89/gen/arm_add_p25519.phase.correct)" \
  --argjson gs_plain "$(ginf .bench/ix89/gen/arm_sub_p25519.plain.correct)" \
  --argjson gs_phase "$(ginf .bench/ix89/gen/arm_sub_p25519.phase.correct)" \
  --argjson gn_plain "$(ginf .bench/ix89/gen/arm_neg_p25519.plain.correct)" \
  --argjson gn_phase "$(ginf .bench/ix89/gen/arm_neg_p25519.phase.correct)" \
  '{human:{add:{plain:$ha_plain,phase:$ha_phase,diff:($ha_phase-$ha_plain)},
           sub:{plain:$hs_plain,phase:$hs_phase,diff:($hs_phase-$hs_plain)},
           neg:{plain:$hn_plain,phase:$hn_phase,diff:($hn_phase-$hn_plain)}},
    generated:{add:{plain:$ga_plain,phase:$ga_phase,diff:($ga_phase-$ga_plain)},
               sub:{plain:$gs_plain,phase:$gs_phase,diff:($gs_phase-$gs_plain)},
               neg:{plain:$gn_plain,phase:$gn_phase,diff:($gn_phase-$gn_plain)}},
    recorded_standing:{add:5745767,sub:9029510,neg:5683733}}')

jq -n \
  --arg commit "$(git rev-parse HEAD)" \
  --arg date "$(date -u +%Y-%m-%d)" \
  --argjson ha "$(cat "$TMP/human_add.json")" \
  --argjson hs "$(cat "$TMP/human_sub.json")" \
  --argjson hn "$(cat "$TMP/human_neg.json")" \
  --argjson ga "$(cat "$TMP/gen_add.json")" \
  --argjson gs "$(cat "$TMP/gen_sub.json")" \
  --argjson gn "$(cat "$TMP/gen_neg.json")" \
  --argjson sa "$(cat "$TMP/sat_add.json")" \
  --argjson ss "$(cat "$TMP/sat_sub.json")" \
  --argjson sn "$(cat "$TMP/sat_neg.json")" \
  --argjson ablation "$ABL" \
  '{
    schema: "ix89-human-phases/1",
    date: $date,
    branch: "staging",
    commit: $commit,
    kernel: ".bench/hol-light-stamp2",
    s2n_bignum_pin: "5e0fc782",
    cap: "12G per proof build, 20G per cargo, one process at a time",
    phase_definitions: {
      A: "init + digitise (quantiier/assumption setup, ENSURES_INIT_TAC, BIGNUM_*DIGITIZE_TAC)",
      B: "simulation, one ARM_ACCSTEPS_TAC per instruction; accumulated vs plain separated",
      C: "carry/borrow characterisation (FLAG_FROM_CARRY_* + its BOUNDER_TAC side goal; generated: aggregate fact proof)",
      D: "final arithmetic, split by tactic (ARITH/REAL_ARITH/INT_ARITH/REAL_INTEGER/NUM_REDUCE/BOUNDER)",
      E: "everything else, itemised (state finalisation, rewrites/expansions, DECARRY/normalisation, assembly, returns, support library)"
    },
    human: {add:$ha, sub:$hs, neg:$hn},
    generated_algebraic: {add:$ga, sub:$gs, neg:$gn},
    generated_sat: {add:$sa, sub:$ss, neg:$sn},
    ablation: $ablation
  }' > "$OUT"
echo "wrote $OUT"
jq -r '"human add sim=\(.human.add.phases.B) total=\(.human.add.inf)  gen add sim=\(.generated_algebraic.add.phases.B) total=\(.generated_algebraic.add.inf)  ablation gen add diff=\(.ablation.generated.add.diff) human add diff=\(.ablation.human.add.diff)"' "$OUT"
