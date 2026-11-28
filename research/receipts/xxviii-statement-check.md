# XXVIII.A — statement, hypothesis and axiom check in the measurement session

Date: 2026-11-28. Repository: `/mnt/1tb_kingston/Tekmerion`, branch `staging`,
HEAD `ce44abc`. Nothing was committed or pushed; `main` was not touched.

This receipt closes the gap named in `TODO.md` **XXVIII.0.1 / XXVIII.A1–A4**:
IX.94–IX.97 report inference counts but said `equals_thm`/`axioms()` had not
been re-run. The check below runs **in the same HOL session that produces the
generated theorem**, so no receipt has to say "inherited" again.

## 1. Memory at the start of the work

`free -g` (2026-11-27 23:49, before any build):

```
               total        used        free      shared  buff/cache   available
Mem:             125          53          33           3          30          71
Swap:            125           5         119
```

Before the `add` extraction: free 41, available 71. Before the `sub`
extraction: free 43, available 72. `df -h /mnt/1tb_kingston`: 265 G free.
Every proof build ran alone, through `tools/cap.sh 12G --`, never raised, with
`-j1`. No two proof builds overlapped.

## 2. Extraction method — route (a)

The upstream `_CORRECT` conclusion is extracted **once per task** from
`.bench/human-baseline/arm/proofs/bignum_{add,sub,neg}_p25519.ml` by building
that proof in its own HOL session and printing `string_of_term (concl th)`.
Route (a) of the brief, quoted:

> save the pretty-printed term and re-parse it in the generated session with
> HOL Light's term parser (the same base is loaded, so the constants exist),
> then `aconv`

The printed term is stored hex-encoded in a line-oriented cache under
`.bench/ix89/stmt/` so whitespace and newlines survive. Extraction also
re-parses the saved string **in the extracting session** and records the answer
(`REPARSE 1`), so a printer that does not round-trip is a hard failure, not a
silent one.

Cache format (one task):

```
TASK arm.bignum_neg_p25519
THEOREM BIGNUM_NEG_P25519_CORRECT
SOURCE arm/proofs/bignum_neg_p25519.ml
REPARSE 1
STMT <hex of string_of_term (concl th)>
AXIOM <hex>            (one line per entry of axioms())
AXIOM <hex>
AXIOM <hex>
```

Extraction commands and output (each is a capped, `-j1` human proof build):

```
$ tests/ix89/stmt_check.sh extract neg
STMT_REPARSE 1
STMT_TEXT_LEN 597
STMT_AXIOM exists f. ONE_ONE f /\ ~ONTO f
STMT_AXIOM forall P x. P x ==> P ((@) P)
STMT_AXIOM forall t. (\x. t x) = t
STMT_EXTRACT task=arm.bignum_neg_p25519 reparse=1 axioms=3
$ echo $?
0

$ tests/ix89/stmt_check.sh extract add
STMT_REPARSE 1
STMT_TEXT_LEN 668
STMT_AXIOM exists f. ONE_ONE f /\ ~ONTO f
STMT_AXIOM forall P x. P x ==> P ((@) P)
STMT_AXIOM forall t. (\x. t x) = t
STMT_EXTRACT task=arm.bignum_add_p25519 reparse=1 axioms=3
$ echo $?
0

$ tests/ix89/stmt_check.sh extract sub
STMT_REPARSE 1
STMT_TEXT_LEN 665
STMT_AXIOM exists f. ONE_ONE f /\ ~ONTO f
STMT_AXIOM forall P x. P x ==> P ((@) P)
STMT_AXIOM forall t. (\x. t x) = t
STMT_EXTRACT task=arm.bignum_sub_p25519 reparse=1 axioms=3
$ echo $?
0
```

Peak RSS of the extraction (build step, then run step), from
`.bench/ix89/stmt/*.extract.log`; all under `tools/cap.sh 12G`:

```
neg  build max_process_vmhwm_kb=1667136   run max_process_vmhwm_kb=165508
add  build max_process_vmhwm_kb=1556784   run max_process_vmhwm_kb=162436
sub  build max_process_vmhwm_kb=1559756   run max_process_vmhwm_kb=161180
```

The extracted `neg` statement decodes to (this is what `aconv` compares
against; add/sub have the same shape with `+` / `-` and their own maychanges):

```
forall z x n pc.
    nonoverlapping (word pc,80) (z,8 * 4)
    ==> ensures arm
        (\s.
             aligned_bytes_loaded s (word pc) bignum_neg_p25519_mc /\
             read PC s = word pc /\
             C_ARGUMENTS [z; x] s /\
             bignum_from_memory (x,4) s = n)
        (\s.
             read PC s = word (pc + 76) /\
             (n <= p_25519
              ==> bignum_from_memory (z,4) s = (p_25519 - n) MOD p_25519))
        (MAYCHANGE [PC; X2; X3; X4; X5; X6; X7] ,,
         MAYCHANGE SOME_FLAGS ,,
         MAYCHANGE [events] ,,
         MAYCHANGE [memory :> bignum (z,4)])
```

SHA-256 of the decoded statement text:

```
neg 67a5ac76b04fbf7bd89b0376c68caf39a600d2a2147211fb220b82972dca12f7
add 0a00efb00e9144b3c8e93aa8241716b6c972db4f8df6b1d0a41ae94087d1828f
sub 539d59792e8745a132ef938f8404c237a907871add821634ffc59d9c37dc3e01
```

The caches are git-ignored build artefacts, not part of the trusted base. If a
cache is absent, `stmt_check.sh ensure <op>` rebuilds it (see §5).

## 3. The in-session check

`tests/ix89/stmt_check.sh inject <op>` appends a block to the generated proof
file after `dropin assemble` has written it. The block runs after the proof's
own `STAMP`/`STAMP_INFERENCES` prints. Verbatim tail of
`.bench/dropin/arm/proofs/bignum_neg_p25519.ml` (the load-bearing part):

```ocaml
(* ---- XXVIII.A: statement / hypothesis / axiom check, same session ---- *)
needs "arm/proofs/ix89_stmt_lib.ml";;
let ix89_task = "arm.bignum_neg_p25519";;
let ix89_th = BIGNUM_NEG_P25519_CORRECT;;
let ix89_stmt_check () =
  let entries =
    try ix89_read_cache "/mnt/1tb_kingston/Tekmerion/.bench/ix89/stmt/arm_neg_p25519.stmt" with _ -> [] in
  let stmt =
    match List.find_opt (fun (k, _) -> k = "STMT") entries with
      None -> None
    | Some (_, h) -> (try Some (parse_term (ix89_string_of_hex h)) with _ -> None) in
  let cached_axes =
    List.map (fun (_, h) -> ix89_string_of_hex h)
      (List.filter (fun (k, _) -> k = "AXIOM") entries) in
  let hyps = hyp ix89_th in
  let actual_axes = List.map (fun t -> string_of_term (concl t)) (axioms ()) in
  let expected_axes =
    List.map (fun t -> string_of_term (concl t)) [INFINITY_AX; SELECT_AX; ETA_AX] in
  let sorted l = List.sort compare l in
  let ax_ok = (sorted actual_axes = sorted expected_axes) in
  let cached_ok =
    (cached_axes = []) || (sorted actual_axes = sorted cached_axes) in
  let aconv_ok =
    match stmt with Some s -> aconv (concl ix89_th) s | None -> false in
  let nhyps = List.length hyps in
  if aconv_ok && nhyps = 0 && ax_ok && cached_ok then
    Printf.printf "STATEMENT_OK task=%s aconv=1 hyps=0 axioms=%d\n%!"
      ix89_task (List.length actual_axes)
  else begin
    Printf.printf "STATEMENT_FAIL task=%s aconv=%d hyps=%d axioms=%d axioms_ok=%d cached_ok=%d\n%!"
      ix89_task (if aconv_ok then 1 else 0) nhyps
      (List.length actual_axes) (if ax_ok then 1 else 0) (if cached_ok then 1 else 0);
    (match stmt with
       None -> Printf.printf "STATEMENT_REF <cache-missing-or-unparseable>\n%!"
     | Some s ->
         Printf.printf "STATEMENT_REF %s\n%!"
           (ix89_truncate 1200 (ix89_oneline (string_of_term s)));
         Printf.printf "STATEMENT_GOT %s\n%!"
           (ix89_truncate 1200 (ix89_oneline (string_of_term (concl ix89_th)))))
  end;
  List.iter (fun a -> Printf.printf "STATEMENT_AXIOM %s\n%!" a) actual_axes;
  List.iter (fun h -> Printf.printf "STATEMENT_HYP %s\n%!"
                        (ix89_truncate 400 (ix89_oneline (string_of_term h)))) hyps;;
ix89_stmt_check ();;
Printf.printf "STATEMENT_CHECK_DONE task=%s\n%!" ix89_task;;
```

The three checks are exactly the brief's:

1. `aconv (concl th) stmt` — alpha-equivalence against the cached upstream
   conclusion, never string equality;
2. `hyp th = []` (`hyps=0`);
3. `axioms()` compared, as a sorted set of conclusions, with
   `[INFINITY_AX; SELECT_AX; ETA_AX]`, and also with the axiom list captured at
   extraction time. Both lists are printed on any failure (`STATEMENT_AXIOM`);
   the reference and actual statements are printed (`STATEMENT_REF`,
   `STATEMENT_GOT`) as single lines.

`tests/ix89/stmt_lib.ml` defines `ix89_hex_of_string`, `ix89_string_of_hex`,
`ix89_read_cache`, `ix89_write_cache`, `ix89_truncate`, `ix89_oneline`. It
defines values only; no inference rule is invoked, so loading it cannot move
`inference_count`.

## 4. Results

Commands (`tests/ix89/run_gen.sh` runs `ensure`, `inject`, then `verdict`; each
proof build is capped and `-j1`):

```
$ tests/ix89/run_gen.sh plain neg   → STATEMENT_OK ... ; exit 0
$ tests/ix89/run_gen.sh plain add   → STATEMENT_OK ... ; exit 0
$ tests/ix89/run_gen.sh plain sub   → STATEMENT_OK ... ; exit 0
$ tests/ix89/run_gen.sh phase neg   → STATEMENT_OK ... ; exit 0   (measurement mode)
```

Lines taken from the `.correct` logs (the `phase` run reproduces `plain`
exactly, so only its `STAMP_INFERENCES` is repeated below):

| task | verdict line | `hyp th` | `axioms()` list | `STAMP_INFERENCES` before → after |
|---|---|---|---|---|
| `arm.neg_p25519` | `STATEMENT_OK task=arm.bignum_neg_p25519 aconv=1 hyps=0 axioms=3` | 0 | `exists f. ONE_ONE f /\ ~ONTO f`; `forall P x. P x ==> P ((@) P)`; `forall t. (\x. t x) = t` | 4,121,086 → **4,121,086** |
| `arm.add_p25519` | `STATEMENT_OK task=arm.bignum_add_p25519 aconv=1 hyps=0 axioms=3` | 0 | `exists f. ONE_ONE f /\ ~ONTO f`; `forall P x. P x ==> P ((@) P)`; `forall t. (\x. t x) = t` | 4,396,406 → **4,396,406** |
| `arm.sub_p25519` | `STATEMENT_OK task=arm.bignum_sub_p25519 aconv=1 hyps=0 axioms=3` | 0 | `exists f. ONE_ONE f /\ ~ONTO f`; `forall P x. P x ==> P ((@) P)`; `forall t. (\x. t x) = t` | 6,259,618 → **6,259,618** |

The `axioms()` list is the three permitted kernel axioms (`INFINITY_AX`,
`SELECT_AX`, `ETA_AX`, printed in the reverse-creation order that
`the_axioms` returns). No fourth axiom appears.

`STAMP_INFERENCES` "before" values: observed at session start with

```
$ grep -H '^STAMP_INFERENCES' .bench/ix89/gen/*.plain.correct .bench/ix89/gen/*.phase.correct
.bench/ix89/gen/arm_add_p25519.plain.correct:STAMP_INFERENCES 4396406
.bench/ix89/gen/arm_neg_p25519.plain.correct:STAMP_INFERENCES 4121086
.bench/ix89/gen/arm_sub_p25519.plain.correct:STAMP_INFERENCES 6259618
.bench/ix89/gen/arm_add_p25519.phase.correct:STAMP_INFERENCES 4396406
.bench/ix89/gen/arm_neg_p25519.phase.correct:STAMP_INFERENCES 4121086
.bench/ix89/gen/arm_sub_p25519.phase.correct:STAMP_INFERENCES 6259618
```

(those artefacts were then overwritten by the verified rebuilds; the values
also appear in `TODO.md:5161-5162` and `TODO.md:5268-5269`, and in
`CHANGELOG.md:19` and `CHANGELOG.md:26-28`, as the IX.97 totals). The `emit`
and `dropin` binaries were not rebuilt; the only change to the generated proof
is text appended after the `STAMP_INFERENCES` print, and the counts reproduced
to the unit.

Group peak RSS of the three generated builds (from the `.log` files), all
`rc=0`:

```
neg plain  sampled_group_peak_rss_kb=2013124  max_process_vmhwm_kb=1823516
add plain  sampled_group_peak_rss_kb=1901524  max_process_vmhwm_kb=1818960
sub plain  sampled_group_peak_rss_kb=2009700  max_process_vmhwm_kb=1820056
neg phase  sampled_group_peak_rss_kb=2011200  max_process_vmhwm_kb=1821396
```

## 5. Negative control (the check bites)

The cached `neg` statement was mutated in place — `word pc,80` →
`word pc,84`, nothing else — and the generated proof was rebuilt. The theorem
is unchanged, so only `aconv` can fire:

```
$ .bench/ix89/stmt/neg_control.sh
...
STATEMENT_FAIL task=arm.bignum_neg_p25519 aconv=0 hyps=0 axioms=3 axioms_ok=1 cached_ok=1
GEN_RUN arm_neg_p25519.plain rc=0 phase_marks=85 statement_rc=1
STAMP_INF_BEFORE 67649313
STAMP_INFERENCES 4121086
MUTANT_RUN rc=1
```

`aconv=0` and exit 1, while `STAMP_INFERENCES` stays `4,121,086`: the check is
sensitive to a wrong reference and its failure does not perturb the proof. The
real cache was then restored (`cmp` clean) and `neg` rebuilt:
`STATEMENT_OK`, `STAMP_INFERENCES 4121086`.

## 6. Where it is wired in

`tests/ix89/run_gen.sh` (the IX.89 generated-side measurement entry point) now
does, in order:

```bash
check="$ROOT/tests/ix89/stmt_check.sh"
"$check" ensure "$op" || exit 94           # extract once if the cache is absent
...
./target/release/dropin assemble ... || exit 97
"$check" inject "$op" || exit 93           # append the same-session check
make ... <target>.correct
"$check" verdict "$tag" "$outdir/${tag}.correct"
```

`ensure` is a no-op when `.bench/ix89/stmt/<arch>_<op>_p25519.stmt` exists with
`REPARSE 1`; otherwise it builds the upstream proof once (capped, `-j1`) and
caches it. A fresh checkout therefore gets the verdict automatically, with no
human proof text needed at measurement time.

`inject` rewrites only the generated `.ml` in `.bench/dropin/` (regenerated by
`dropin assemble` on every run); nothing under `src/`, `tools/bridge.ml` or the
fixtures is touched. `verdict` writes `$outdir/${tag}.correct.statement` beside
the `.correct` log and echoes the `STATEMENT_OK`/`STATEMENT_FAIL` line;
`run_gen.sh` exits non-zero if the verdict is not `STATEMENT_OK`.

Sidecar files produced (`339` bytes each, e.g.
`.bench/ix89/gen/arm_neg_p25519.plain.correct.statement`):

```
STATEMENT_CHECK_FILE ...
STATEMENT_OK task=arm.bignum_neg_p25519 aconv=1 hyps=0 axioms=3
STATEMENT_AXIOM exists f. ONE_ONE f /\ ~ONTO f
STATEMENT_AXIOM forall P x. P x ==> P ((@) P)
STATEMENT_AXIOM forall t. (\x. t x) = t
STATEMENT_CHECK_DONE task=arm.bignum_neg_p25519
```

Tracked files added/changed (not committed):

```
M tests/ix89/run_gen.sh
?? tests/ix89/stmt_check.sh
?? tests/ix89/stmt_lib.ml
```

SHA-256 of the four final `.correct` logs (local, git-ignored):

```
bb19894ddccf6dfb1060fbebb44ee5713e03b43dba775647e8797a0fa17be269  arm_neg_p25519.plain.correct
eb5df37e4e387fd5dbd195f8b6ba9e4139e1c58b5b41420703638e569fdc199f  arm_add_p25519.plain.correct
559e91551fc6278060f5cf29574bfa86672868e97f6426c5a3c1582a2f0a50dc  arm_sub_p25519.plain.correct
c400c45534a55e0a7e457163ec712ee1e5c7d2ff69a95903a489db5208afb74a  arm_neg_p25519.phase.correct
```

## 7. Honest notes

- No task failed `aconv`; there is no failing pair of terms to quote.
- The negative control mutates the *reference* statement. `aconv` is
  symmetric, so a wrong theorem statement fails the same way; what this control
  demonstrates is that the reference path and the failure record are live, not
  that the generated theorem was deliberately broken.
- The cache is a build artefact under `.bench/` (git-ignored). Its
  reproducibility rests on `stmt_check.sh extract`, which rebuilds it from the
  pinned upstream file `.bench/human-baseline/arm/proofs/bignum_<op>_p25519.ml`.
- This receipt does not update any ratio in README/CHANGELOG. It only clears
  the blocking condition for doing so (`XXVIII.0.1`).

## 8. Verdict

```
STATEMENT CHECK: add OK, sub OK, neg OK (axioms=[INFINITY_AX: exists f. ONE_ONE f /\ ~ONTO f; SELECT_AX: forall P x. P x ==> P ((@) P); ETA_AX: forall t. (\x. t x) = t])
```
