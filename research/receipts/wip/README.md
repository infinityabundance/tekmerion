# `ix58-checktime.partial.json` — not a receipt

This file is a **partial working measurement**, not a receipt.

- It contains **25** task rows (18 ARM + 7 x86), not 27: x86 `secp256k1/neg` and
  `sm2/neg` are absent. Verified with `grep -c '"dir"'` = 25 and, after the
  comparison machinery was fixed, with `checktime-table`, which prints
  `HUMAN ROWS 25`.
- It records `stamp: false` and `inference_delta: null` everywhere; it was
  measured without the axiom-stamp kernel.
- As a metric it is **superseded by `../ix60-matched-inference.md`**. IX.60
  measures the human proofs under the stamp kernel and compares the two theorem
  statements. All 27 human `BIGNUM_*_SUBROUTINE_CORRECT` statements differ from
  the generated `BIGNUM_*_CORRECT` statements (incomparable contracts), so the
  comparison machinery refuses a `MatchedComparison` for every row and no ratio
  is published.
- Its wall-clock times are retained as the **only wall-clock data**, clearly
  labelled partial, in IX.60 §4. They are not a same-obligation ratio.
