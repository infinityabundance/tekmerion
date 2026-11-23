# Receipt: Programme v3 FC recount at the bench pin

Measured revision (pinned checkout): `5e0fc782ae6d587c2ae40faf2c2c4d0b637e6240`.

The second measurement is at `0f792fe2e8a63a0bbcf384ddf73e67cafb537ca4`, the
revision v3's figures came from. It is **not** in the pinned fork, so it was
fetched into a throwaway clone outside the project, only to measure the delta:

    git init /tmp/s2n-head
    git -C /tmp/s2n-head fetch --depth 1 \
        https://github.com/awslabs/s2n-bignum.git \
        0f792fe2e8a63a0bbcf384ddf73e67cafb537ca4
    git -C /tmp/s2n-head checkout -q FETCH_HEAD

`/tmp/innocent_looking.ml` is a symlink to
`.bench/s2n-bignum/arm/proofs/secp256k1_jadd.ml`, used to show that the guard
resolves links before opening.

Everything below is verbatim subprocess output, including return codes.

$ git -C .bench/s2n-bignum rev-parse HEAD
5e0fc782ae6d587c2ae40faf2c2c4d0b637e6240
[return code 0]

$ git -C .bench/s2n-bignum log -1 --format=%H %ci %s
5e0fc782ae6d587c2ae40faf2c2c4d0b637e6240 2026-08-09 11:16:13 -0400 Merge branch 'awslabs:main' into main
[return code 0]

$ python3 tools/fcdist.py .bench/s2n-bignum --corpus .bench/problems.json
s2n-bignum commit: 5e0fc782ae6d587c2ae40faf2c2c4d0b637e6240
checkout:          /mnt/1tb_kingston/Tekmerion/.bench/s2n-bignum
files scanned:     689
top-level decls:   5811

== 1. FC theorem count (bench rule) ==
  total FC: 898
    arm  463
    x86  435
  unique goals: 898  (declarations sharing a goal: 0)

== 2. shape ==
  straight-line: 675 (75.17%)
  loop:          223 (24.83%)
  equivalence (name has _EQUIV): 57
  SUBROUTINE_SAFE names:          394
  goals mentioning ensures2:     0
  proofs >= 300 lines: 164  loop 131 / straight 33  (loop share 79.88%)

== 3. proof length (non-blank, non-comment tactic lines) ==
  tactic-only      n=898 median=87.5 q1=40.25 q3=188.75 min=2 max=4119
    bands: 0-24:185, 25-49:97, 50-99:213, 100-199:190, 200-299:49, 300-599:69, 600+:95
  whole prove span n=898 median=104.0 q1=53.0 q3=211.25 min=12 max=4140
    bands: 0-24:108, 25-49:106, 50-99:223, 100-199:217, 200-299:78, 300-599:69, 600+:97
  arm tactic-only  n=463 median=92 q1=41.0 q3=254.5
  x86 tactic-only  n=435 median=77 q1=34.5 q3=161.5
  by shape (tactic-only):
    straight_line n=675 median=64 q1=20.0 q3=115.5
      bands: 0-24:185, 25-49:90, 50-99:196, 100-199:155, 200-299:16, 300-599:27, 600+:6
    loop         n=223 median=383 q1=188.0 q3=781.0
      bands: 0-24:0, 25-49:7, 50-99:17, 100-199:35, 200-299:33, 300-599:42, 600+:89
  shape by arch: arm straight 338 / loop 125, x86 straight 337 / loop 98

== 4. families (straight-line FC only) ==
  classified: 569   unclassified: 106
    add          87  (arm 44, x86 43)
    double       59  (arm 31, x86 28)
    mul          58  (arm 32, x86 26)
    mod          53  (arm 13, x86 40)
    montmul      47  (arm 31, x86 16)
    montsqr      47  (arm 31, x86 16)
    sqr          36  (arm 20, x86 16)
    sub          25  (arm 13, x86 12)
    inv          20  (arm 10, x86 10)
    cmul         18  (arm 6, x86 12)
    tomont       15  (arm 6, x86 9)
    triple       15  (arm 5, x86 10)
    deamont      12  (arm 5, x86 7)
    demont       12  (arm 5, x86 7)
    montinv      12  (arm 6, x86 6)
    neg          12  (arm 6, x86 6)
    optneg       12  (arm 6, x86 6)
    half         10  (arm 5, x86 5)
    invsqrt       4  (arm 2, x86 2)
    madd          4  (arm 2, x86 2)
    modinv        4  (arm 2, x86 2)
    sqrt          4  (arm 2, x86 2)
    emontredc     3  (arm 3, x86 0)
  subtotal over the vocabulary the brief lists (mod, montmul, montsqr, add, sub, neg, double, half, triple, tomont, demont): 382

== named theorems dropped by the rule ==
  1817  a top-level subroutine that just uses the _CORRECT of the core part
   394  not a proof related to functional correctness

== drop decomposition (all top-level declarations) ==
   1817  a top-level subroutine that just uses the _CORRECT of the core part
    451  not a proof related to functional correctness
      3  the goal is evaluated at runtime
   2721  declarations whose name contains _CORRECT
   1817  ... of which SUBROUTINE_CORRECT

== 5. cross-check against collected corpus ==
  corpus: /mnt/1tb_kingston/Tekmerion/.bench/problems.json
  problems total: 2622  categories: {'bit_vector': 437, 'functional_correctness_arm': 463, 'functional_correctness_x86': 435, 'generic': 686, 'program_state': 601}
  corpus FC: 898 {'arm': 463, 'x86': 435}
  direct FC: 898 {'arm': 463, 'x86': 435}
  matched (file, theorem): 898
  only direct: 0   only corpus: 0
[return code 0]

$ python3 tools/fcdist.py /tmp/s2n-head
s2n-bignum commit: 0f792fe2e8a63a0bbcf384ddf73e67cafb537ca4
checkout:          /tmp/s2n-head
files scanned:     718
top-level decls:   7096

== 1. FC theorem count (bench rule) ==
  total FC: 925
    arm  478
    x86  447
  unique goals: 925  (declarations sharing a goal: 0)

== 2. shape ==
  straight-line: 699 (75.57%)
  loop:          226 (24.43%)
  equivalence (name has _EQUIV): 61
  SUBROUTINE_SAFE names:          445
  goals mentioning ensures2:     0
  proofs >= 300 lines: 164  loop 131 / straight 33  (loop share 79.88%)

== 3. proof length (non-blank, non-comment tactic lines) ==
  tactic-only      n=925 median=84 q1=37.0 q3=188.0 min=2 max=4119
    bands: 0-24:192, 25-49:111, 50-99:218, 100-199:190, 200-299:50, 300-599:69, 600+:95
  whole prove span n=925 median=101 q1=53.0 q3=207.0 min=12 max=4140
    bands: 0-24:108, 25-49:111, 50-99:240, 100-199:221, 200-299:78, 300-599:70, 600+:97
  arm tactic-only  n=478 median=91.0 q1=41.0 q3=242.5
  x86 tactic-only  n=447 median=74 q1=33.5 q3=152.5
  by shape (tactic-only):
    straight_line n=699 median=62 q1=20.0 q3=111.5
      bands: 0-24:192, 25-49:102, 50-99:201, 100-199:155, 200-299:16, 300-599:27, 600+:6
    loop         n=226 median=373.5 q1=181.25 q3=778.25
      bands: 0-24:0, 25-49:9, 50-99:17, 100-199:35, 200-299:34, 300-599:42, 600+:89
  shape by arch: arm straight 352 / loop 126, x86 straight 347 / loop 100

== 4. families (straight-line FC only) ==
  classified: 570   unclassified: 129
    add          88  (arm 45, x86 43)
    double       59  (arm 31, x86 28)
    mul          58  (arm 32, x86 26)
    mod          53  (arm 13, x86 40)
    montmul      47  (arm 31, x86 16)
    montsqr      47  (arm 31, x86 16)
    sqr          36  (arm 20, x86 16)
    sub          25  (arm 13, x86 12)
    inv          20  (arm 10, x86 10)
    cmul         18  (arm 6, x86 12)
    tomont       15  (arm 6, x86 9)
    triple       15  (arm 5, x86 10)
    deamont      12  (arm 5, x86 7)
    demont       12  (arm 5, x86 7)
    montinv      12  (arm 6, x86 6)
    neg          12  (arm 6, x86 6)
    optneg       12  (arm 6, x86 6)
    half         10  (arm 5, x86 5)
    invsqrt       4  (arm 2, x86 2)
    madd          4  (arm 2, x86 2)
    modinv        4  (arm 2, x86 2)
    sqrt          4  (arm 2, x86 2)
    emontredc     3  (arm 3, x86 0)
  subtotal over the vocabulary the brief lists (mod, montmul, montsqr, add, sub, neg, double, half, triple, tomont, demont): 383

== named theorems dropped by the rule ==
  1870  a top-level subroutine that just uses the _CORRECT of the core part
   445  not a proof related to functional correctness

== drop decomposition (all top-level declarations) ==
   1870  a top-level subroutine that just uses the _CORRECT of the core part
    506  not a proof related to functional correctness
    125  the goal is evaluated at runtime
   2824  declarations whose name contains _CORRECT
   1870  ... of which SUBROUTINE_CORRECT

[return code 0]

$ python3 tools/heldout_guard.py .bench/s2n-bignum/arm/proofs/secp256k1_jadd.ml
REFUSED: held-out marker 'secp256k1' in .bench/s2n-bignum/arm/proofs/secp256k1_jadd.ml
[return code 2]

$ python3 tools/heldout_guard.py .bench/s2n-bignum/arm/proofs/curve25519_ladderstep.ml
REFUSED: held-out marker '25519' in .bench/s2n-bignum/arm/proofs/curve25519_ladderstep.ml
[return code 2]

$ python3 tools/heldout_guard.py .bench/s2n-bignum/x86/proofs/sm2_montjmixadd.ml
REFUSED: held-out marker 'sm2' in .bench/s2n-bignum/x86/proofs/sm2_montjmixadd.ml
[return code 2]

$ python3 tools/heldout_guard.py .bench/s2n-bignum/arm/proofs/p256_montjadd.ml
allowed: /mnt/1tb_kingston/Tekmerion/.bench/s2n-bignum/arm/proofs/p256_montjadd.ml sha256=f471b44633b9a985042f6c7bc4d62385409a7a5c2aa8e8327f72e698b17e7ade lines=1455
[return code 0]

$ python3 tools/heldout_guard.py .bench/s2n-bignum/arm/proofs/secp256k1_jadd.ml --root .bench/s2n-bignum
REFUSED: held-out marker 'secp256k1' in .bench/s2n-bignum/arm/proofs/secp256k1_jadd.ml
[return code 2]

$ python3 tools/heldout_guard.py /tmp/innocent_looking.ml
REFUSED: held-out marker 'secp256k1' in /mnt/1tb_kingston/Tekmerion/.bench/s2n-bignum/arm/proofs/secp256k1_jadd.ml
[return code 2]

$ python3 tools/heldout_guard.py .bench/s2n-bignum/arm/proofs
REFUSED: not a regular file: .bench/s2n-bignum/arm/proofs
[return code 2]

$ python3 tools/heldout_guard.py
REFUSED: no path given (fail closed)
[return code 2]

