
---

## Orchestrator verification (independent re-run)

Re-ran the decisive command myself rather than accepting the report:

```
$ TEKMERION_ALG_CLOSE=arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT \
    tools/cap.sh 12G -- bash .bench/ix69/run_task.sh arm curve25519 add p25519 .bench/ix82/verify-orch
RUN arm_add_p25519 rc=0 phase_marks=29
STAMP_INF_BEFORE 67646907
STAMP_INFERENCES 5233638
cap.sh: group=167896 rc=0 sampled_group_peak_rss_kb=63056 max_process_vmhwm_kb=62048

$ grep -c "ORACLE close ok"   .../arm_add_p25519.correct   -> 12
$ grep -c "bridge:sat-done"   .../arm_add_p25519.correct   -> 0
$ grep "^STAMP_INFERENCES"    .../arm_add_p25519.correct   -> 5,233,638
$ Running time: 28.000000
```

The headline reproduces on a clean run under a 12 G cap: solver-free, 12 oracle closes, no
bit-blast, **5,233,638** primitive inferences — 21.30× fewer than the sliced SAT route and 2.83×
the human proof. Measured on one host, one pin, one theorem.
