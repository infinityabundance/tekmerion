| object | bytes | recorded nodes | reachable nodes | checker s (run 1 / 2) | process wall s (run 1 / 2) | peak RSS KB (run 1 / 2) | peak live nodes | replayed nodes/s (1 / 2) | reported inferences/s (1 / 2) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| arm_add_p25519 | 64895202 | 4335020 | 3205212 | 2.753 / 2.863 | 2.766 / 2.875 | 108704 / 108540 | 4400 | 1164262 / 1119529 | 2087093 / 2006904 |
| arm_neg_p25519 | 62436134 | 5119107 | 3075686 | 4.445 / 4.452 | 4.457 / 4.464 | 102776 / 102780 | 2496 | 691943 / 690855 | 1278680 / 1276670 |
| arm_sub_p25519 | 90698353 | 6258132 | 4215004 | 4.364 / 4.326 | 4.376 / 4.339 | 139304 / 139092 | 3946 | 965858 / 974342 | 2069090 / 2087265 |

Determinism: **yes** — statement_id, derivation_digest and reachable node count are identical in both runs.
