# IX.73 — Failure taxonomy and the cap's documented scope (XVI.D, XVI.E)

- **Date:** 2026-11-25
- **Branch:** `staging` (one commit)
- **Scope:** `tools/bridge.ml` (XVI.D), `tools/cap.sh`, `AGENTS.md` §7a,
  `tests/bridge_failure_selftest.{sh,ml}`, `tests/bridge_failure_taxonomy.rs`,
  `tests/cap_group_kill.sh`, `tests/tools_emit.rs`, `tests/tools_receipt.rs`,
  `tests/fixtures/answers/**`, `tests/fixtures/curve_check_bridge.txt`, the
  `XVI.D*`/`XVI.E*` lines of `TODO.md`, and this receipt. No other receipt, no
  `tools/csat_lib.ml`, no `tools/hol-light/axiom-stamp.diff`, no `src/native/`,
  no `verification/`, no other `src/tools/*.rs`, no `.bench` write, no tag.
- **Environment:** OCaml 5.5.0, util-linux `setsid` 2.42.4, rustc/cargo 1.98.0,
  one host.

## Resource check (AGENTS.md §7a)

The brief fixes the cap at 20 G; the 3× rule therefore needs ≥ 60 G available.
`/usr/bin/time` is **absent** on this host (`command -v /usr/bin/time` prints
nothing); this matters for the reporting claim below.

Before the runs:

```text
$ free -g | head -2
               total        used        free      shared  buff/cache   available
Mem:             125          29          87           2          11          95
$ df -h /mnt/1tb_kingston | tail -1
/dev/nvme1n1p1  1.3T  1.3T  7.0G 100% /mnt/1tb_kingston
```

95 G available is 4.75× the 20 G cap; 7.0 G free on the volume is above the 6 G
floor. Every cargo run below went through `tools/cap.sh 20G --`. **No HOL Light
proof build was started**, so the 12 G proof cap was not used. After the runs:

```text
$ free -g | head -2
               total        used        free      shared  buff/cache   available
Mem:             125          29          91           2           7          96
$ df -h /mnt/1tb_kingston | tail -1
/dev/nvme1n1p1  1.3T  1.3T  7.0G 100% /mnt/1tb_kingston
```

## 1. The defect (XVI.D)

`BRIDGE_CSAT_TAC` tried the sliced hypothesis set first and, on any exception
other than `Out_of_memory`, retried the **full** set. `Stack_overflow` — the
exception that ended IX.70's algebraic route — was therefore caught by the
catch-all and followed by a larger obligation. That is backwards: a
resource-exhausted attempt must not fall back to a larger instance.

## 2. The five outcomes and their policies

The policy is one pure function, `bridge_failure_class` (`tools/bridge.ml`,
marked `BEGIN/END BRIDGE_FAILURE_STANDALONE`), called by
`bridge_slice_retry`. `closed` is the success path of the retry; the other four
are exception classes.

| class | trigger | message | policy |
|---|---|---|---|
| **closed** | the sliced attempt returned a theorem | none | use it |
| **mathematical** | a `Failure` from the close that is not a sanity failure, a timeout or a signal death (e.g. `CSAT: cadical did not report UNSAT, exit 10`: the stronger slice was unprovable) | `SLICE fallback=<task> reason=mathematical kept=k/n` | run the full set; increment `bridge_fallback_count` |
| **unsupported input** | `Invalid_argument s`, or `Failure "unsupported:..."` | `CSAT unsupported=<what> kept=k/n` | report, re-raise, **no fallback** |
| **resource exhausted** | `Out_of_memory`, `Stack_overflow`, `Sys.Break`; `Failure "... exit 124"` (timeout wrapper); `Failure "... exit N"` with `N >= 128` (signal death, including OOM-killed solvers); or `bridge_failure_class ~timeout:true` | `CSAT capped=<which> limit=<what> kept=k/n` | report, re-raise, **no fallback** |
| **internal error** | `Failure "CSAT_PROVE: Sanity check"`, `Failure "CSAT_REFUTE: Sanity check failure"`, or any other exception | `CSAT internal=<why>` | report, re-raise, do not swallow |

The mapping is deliberately explicit rather than a `try ... with _`:

- `Out_of_memory` and `Stack_overflow` are matched by constructor, not by text.
- A solver killed by a signal is `exit 128+N`; 124 is the `timeout(1)`
  convention. Both are resource/interrupt outcomes.
- A generic `Failure` remains the mathematical fallback, which is today's
  behaviour for a slice that was too small.
- The classifier reports through `bridge_slice_note`, the same phase-mode
  channel as the existing `SLICE` lines, so a normal answer stays silent; the
  re-raised exception is the report that always reaches the caller.

`bridge_slice_retry` is the only place the fallback can run. The shipped
`BRIDGE_CSAT_TAC` body is now:

```ocaml
    let th' =
      if keptn = total then close hths
      else
        bridge_slice_retry
          ~task:(!bridge_slice_task) ~kept:keptn ~total
          ~attempt:(fun () -> close kept)
          ~fallback:(fun () -> close hths)
          ~note:bridge_slice_note in
    ACCEPT_TAC th' (asl,w);;
```

The two IX.66 marker anchors in `src/tools/emit.rs` were preserved verbatim
(`let BRIDGE_CSAT_TAC : tactic = ... let hths = map snd asl in` and
`ACCEPT_TAC th' (asl,w);;`), so phase mode still instruments the same
boundaries.

## 3. Tests (XVI.D3)

`tests/bridge_failure_selftest.sh` extracts the marked block verbatim and runs
it with plain OCaml; `tests/bridge_failure_taxonomy.rs` runs that script from
`cargo test` (and skips, naming the reason, if `ocaml` is absent). The
resource cases use real exceptions — `raise Stack_overflow`, `raise
Out_of_memory` — not `failwith`, and assert the fallback counter stays at zero.

```text
$ bash tests/bridge_failure_selftest.sh
OK closed: result
OK closed: no fallback
OK closed: counter zero
OK closed: no note
OK mathematical: fallback ran
OK mathematical: result is the fallback's
OK mathematical: counter one
OK mathematical: note
OK mathematical-generic: fallback ran
OK mathematical-generic: counter one
OK mathematical-generic: note
OK unsupported: re-raised
OK unsupported: no fallback
OK unsupported: counter zero
OK unsupported: note
OK resource-memory: re-raised
OK resource-memory: no fallback
OK resource-memory: counter zero
OK resource-memory: note
OK resource-stack: re-raised
OK resource-stack: no fallback
OK resource-stack: counter zero
OK resource-stack: note
OK classify timeout flag
OK classify exit 124
OK classify signal 137
OK resource-signal: re-raised
OK resource-signal: no fallback
OK resource-signal: counter zero
OK resource-signal: note
OK internal: re-raised
OK internal: no fallback
OK internal: counter zero
OK internal: note
OK internal-unknown: re-raised
OK internal-unknown: no fallback
OK internal-unknown: counter zero
OK internal-unknown: note
OK bridge_failure_taxonomy
```

The decisive stack-overflow assertion is `OK resource-stack: no fallback`;
`OK resource-memory: no fallback` is the same property for OOM. The
mathematical case is the control that proves the counter and fallback still
work (`counter one`, `fallback ran`).

`tests/tools_emit.rs` was updated from a textual grep for the old catch-all to
`ix69_slice_falls_back_only_on_a_mathematical_failure`, which pins: the sliced
attempt is passed before the full-set fallback; there is exactly one
`reason=mathematical` report; and `Out_of_memory`/`Stack_overflow` map to
`` `Resource``.

## 4. `cap.sh` scope (XVI.E1, XVI.E2)

### What is guaranteed

- **Per process:** every process in the tree inherits `RLIMIT_AS = <MEM>` via
  `ulimit -v`, so no single process can exceed it (mechanism verified to bite).
- **Group isolation and kill:** the command runs in its own session/process
  group (`setsid`). With `--timeout <secs>` the whole group receives TERM and
  then KILL on expiry, not just the leaf. The timeout is additive: the existing
  `tools/cap.sh <MEM> -- <cmd>` form is unchanged.

### What is not guaranteed

- **Aggregate memory:** `RLIMIT_AS` caps one process, not a tree. N processes
  can consume about N × the limit.
- cgroups would cap the aggregate, but this host accepts and ignores
  `systemd-run -p MemoryMax=` (existing recorded verification: the scope
  reports `MemoryMax=infinity` and a 2 GB allocation succeeds under a 256 M
  setting). **I did not re-run that cgroup probe in this task**; it is the
  finding already recorded in `AGENTS.md` §7a and TODO XVI.E.
- A descendant that deliberately calls `setsid` itself leaves the group, so
  neither the group kill nor the group RSS sample includes it. Common build
  tools do not do this.

### Reporting

One stderr line after every run:

```text
cap.sh: group=<pgid> rc=<rc> sampled_group_peak_rss_kb=<n> max_process_vmhwm_kb=<n>
```

`sampled_group_peak_rss_kb` is the maximum over 0.5 s samples of the summed RSS
of processes whose PGID is `<pgid>`; it can miss a shorter peak.
`max_process_vmhwm_kb` is the highest per-process `VmHWM` observed alive.
`/usr/bin/time -v` is absent here; where it exists it reports the **direct
child's** `ru_maxrss`, not the tree's, so it is not used as tree accounting.

### Exact wording added to `AGENTS.md` §7a

The old `systemd-run` bullet was replaced by the following three bullets
(verbatim):

```markdown
- `tools/cap.sh` uses `RLIMIT_AS` via `ulimit -v`, which was verified to bite.
  It caps **one process**, not a tree: every process the command starts inherits
  the limit, so no single process can exceed it, but the aggregate is not
  capped — N processes can consume about N × the limit. cgroups would cap the
  aggregate, but `systemd-run -p MemoryMax=` is **not** an acceptable mechanism
  on this host: it is accepted but not applied (verified — the scope reports
  `MemoryMax=infinity` and a 2 GB allocation succeeds under a 256 M setting).
  The group kill below is the strongest tree-wide stop available here, and it
  is time-based, not memory-based.
- The command runs in its own process group (`setsid`). `--timeout <secs>` is an
  additive option: `tools/cap.sh --timeout 600 12G -- make -j1 … <target>.correct`.
  On expiry the whole group receives TERM and then KILL, not just the leaf. A
  descendant that deliberately calls `setsid` itself leaves the group; common
  build tools do not do this. The existing `tools/cap.sh <MEM> -- <cmd>` form is
  unchanged.
- After the run the wrapper prints one stderr line:
  `cap.sh: group=<pgid> rc=<rc> sampled_group_peak_rss_kb=<n> max_process_vmhwm_kb=<n>`.
  `sampled_group_peak_rss_kb` is a 0.5 s sample of the summed RSS of the group
  (it can miss a shorter peak); `max_process_vmhwm_kb` is the highest
  per-process `VmHWM` seen alive. `/usr/bin/time -v` is absent on this host;
  where present it gives the direct child's maximum only, never tree accounting.
  The group-kill control is `tests/cap_group_kill.sh` (XVI.E3).
```

`tools/cap.sh`'s header states the same guarantees, the same gap, the absent
`/usr/bin/time`, and the `setsid`-escape case.

## 5. Aggregate-gap demonstration (honest scope of the cap)

Three OCaml processes, each holding a 700 MiB `Bytes` object, under a **2 G
per-process** cap. No single process is near 2 G; the tree is above it. The
sampler sees `2208416` KiB ≈ 2.106 GiB, i.e. the aggregate exceeds the cap.
The timeout then kills the whole group. The helper used for the demonstration
was:

```ocaml
(* /tmp/alloc_hold700.ml *)
let b = Bytes.make (700 * 1024 * 1024) (Char.chr 120) in
let _ = Sys.command "sleep 30" in
ignore b
```

```text
$ tools/cap.sh --timeout 6 2G -- bash -c \
    'ocaml /tmp/alloc_hold700.ml & ocaml /tmp/alloc_hold700.ml & ocaml /tmp/alloc_hold700.ml & wait'
cap.sh: TIMEOUT after 6s; process group 2181499 signalled (TERM then KILL)
cap.sh: group=2181499 rc=124 sampled_group_peak_rss_kb=2208416 max_process_vmhwm_kb=732052
```

`2208416` KiB is greater than the 2 G cap's `2097152` KiB. That is the
aggregate-memory gap, measured rather than asserted; the per-process cap did not
fire, and the group kill is what stopped the run.

## 6. The E3 control: both processes gone

`tests/cap_group_kill.sh` runs a parent under `--timeout 3 512M`; the parent
spawns a child that tries to allocate 4 GiB and therefore fails the cap, then
the parent keeps sleeping. The child's fate and the parent's termination are
recorded by the script, and `ps` afterwards is empty.

```text
$ bash tests/cap_group_kill.sh
Out of memory during evaluation.
Terminated                 sleep 300
cap.sh: TIMEOUT after 3s; process group 2182138 signalled (TERM then KILL)
cap.sh: group=2182138 rc=124 sampled_group_peak_rss_kb=16520 max_process_vmhwm_kb=4652
CONTROL cap_rc=124
CONTROL parent_pid=2182138 child_pid=2182140
CONTROL recorded:
CONTROL   CHILD_STATUS=2
CONTROL   PARENT_SLEEPING=2182138
CONTROL   PARENT_TERM=143
CONTROL alive_after=0
CONTROL ps_after:
    PID    PGID STAT COMMAND
CONTROL PASS
```

Read: `cap.sh` returns **124**; the child died of the cap (`CHILD_STATUS=2`,
after `Out of memory during evaluation.`); the parent was still alive until the
timeout (`PARENT_SLEEPING`), then its TERM trap recorded `PARENT_TERM=143`; and
both PIDs are absent (`alive_after=0`, empty `ps`). The control asserts exactly
those three facts and exits non-zero otherwise.

## 7. The library moved; fixtures and goldens re-recorded deliberately

`tools/bridge.ml` changed (the taxonomy). `tools/csat_lib.ml` did not.
`src/tools/emit.rs` did not change; its two IX.66 anchors were preserved.

- New `tools/bridge.ml` digest:
  `deafeb4de6129686f7a521a0b4afee08bdd15b78aaf703f2bc0dbc39a5d1c072`
  (`BRIDGE_DIGEST` re-pinned; the old `0a135381…` stays recorded as a boundary).
- Instrumented bridge phrase count: **154 → 157** (three new `let`s:
  `bridge_failure_class`, `bridge_fallback_count`, `bridge_slice_retry`).
  Instrumented output digest:
  `cd52a52efb927b656079a786256d3aa4d5da96a24de7fd7cee5fd56a3255556f`
  (was `6037ca7f…`).
- Inline-answer phrase count: **198 → 201**. The four `inline_answer` digests
  were re-recorded:
  - `add-ref`  `8090b1a9f9de4a1a42cfee4d54072c013b64d1a48e4ea02847ecb4d06fea1df7`
  - `sub`      `d2b93b6dab4a1a39ce5b6b549d24ded367234af5b66d547799ab7d76b3bf8d4d`
  - `neg`      `93c457d6de3e754fe822d76b32e02d41edabfdedd99e84060bc716565d75476c`
  - `add-p384` `3d9547195cb84b7701d313a3953fa195926c604bb7588c5577a3aecb1a66aed6`
  The IX.70 values remain in the test as the boundary.
- All **27** tracked reference answers under `tests/fixtures/answers/` were
  re-recorded with the exact command from the brief:

  ```text
  $ TEKMERION_RECORD_ANSWERS=1 tools/cap.sh 20G -- cargo test --offline --test tools_emit
  ```

  This is a deliberate re-record, not a test edit: every answer inlines the
  bridge library text, and the text gained the taxonomy block.
- `tests/fixtures/curve_check_bridge.txt` was regenerated because the inserted
  block moved the line numbers of the later `prove` statements. The statement
  count is unchanged (`statements=76 hits=0`), so no statement was added or
  changed; only the reported line numbers moved. The report was produced with
  the shipped binary:

  ```text
  $ tools/cap.sh 20G -- cargo run --offline --release --bin curve_check -- tools/bridge.ml \
      > tests/fixtures/curve_check_bridge.txt
  ...
  statements=76 hits=0
  ```

## 8. Final validation (verbatim)

```text
$ free -g | head -2
               total        used        free      shared  buff/cache   available
Mem:             125          29          87           2          11          95
$ df -h /mnt/1tb_kingston | tail -1
/dev/nvme1n1p1  1.3T  1.3T  7.0G 100% /mnt/1tb_kingston

$ tools/cap.sh 20G -- cargo test --offline
...
test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
... (42 `test result: ok` blocks in total; see /tmp/ix73-cargo-test-final.txt)
     Running tests/bridge_failure_taxonomy.rs (...)
test bridge_failure_taxonomy_selftest_passes ... ok
...
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
cap.sh: group=2182266 rc=0 sampled_group_peak_rss_kb=177044 max_process_vmhwm_kb=191764

$ tools/cap.sh 20G -- cargo clippy --offline --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.02s
cap.sh: group=2181287 rc=0 sampled_group_peak_rss_kb=75972 max_process_vmhwm_kb=54920

$ cargo fmt --check
(no output; exit 0)

$ git ls-files '*.py'
(no output; exit 0)

$ df -h /mnt/1tb_kingston | tail -1
/dev/nvme1n1p1  1.3T  1.3T  7.0G 100% /mnt/1tb_kingston
```

The existing interface was exercised unchanged throughout: every cargo command
above used `tools/cap.sh 20G -- cargo ...`, with no `--timeout`.

## 9. What I could not verify

- **The cgroup claim was not re-probed in this task.** The statement that
  `systemd-run -p MemoryMax=` is accepted but not applied is the recorded
  finding in `AGENTS.md` §7a; I did not create a fresh scope here.
- **No end-to-end HOL Light proof run.** The taxonomy was tested outside HOL
  Light (as intended: the block uses only the standard library) and the shape
  is pinned in `tests/tools_emit.rs`. A live proof that spills the OCaml stack
  was not re-run under the new code, so the `Stack_overflow` path is proven by
  the synthetic exception plus the classifier, not by replaying IX.70's proof.
- **The timeout branch inside the tactic has no in-tree producer.** There is no
  wall-clock timer in `BRIDGE_CSAT_TAC`; a timeout in practice arrives as the
  process-group kill, or as a solver `Failure "... exit 124"`/signal from an
  enclosing runner. The `~timeout:true` classifier input and exit-124/≥128
  mapping are tested, but no shipped call site passes `~timeout`.
- **`Invalid_argument` as the "unsupported input" signal is a policy**, chosen
  because the current CSAT close raises `Failure`, not `Invalid_argument`, for
  mathematical failure. The class and its no-fallback policy are tested; no
  current code path is known to raise it.
