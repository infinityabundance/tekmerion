//! Tests for the Slice D Rust ports of the remaining receipt scripts:
//! `curve_check`, `instrument`, `inline_answer`, `sealed_court`, and the
//! `dropin sweep` driver.
//!
//! The rule-level tests use synthetic inputs and need nothing outside the
//! repository. The differential tests compare against artefacts recorded from
//! the Python originals; the `inline_answer` one skips cleanly when the pinned
//! `.bench` sources are absent.

use std::fs;
use std::path::PathBuf;

use tekmerion::tools::inline_answer;
use tekmerion::tools::instrument;
use tekmerion::tools::json;
use tekmerion::tools::sealed_court;
use tekmerion::tools::{curve_check, dropin};

/// sha256 of `tools/bridge.ml` when the recorded goldens were captured.
///
/// A mismatch is a **failure**, not a skip: the goldens below were measured
/// against this exact text, and silently passing while comparing nothing is the
/// defect XVI.A2 removed. Re-record deliberately and update both the digest and
/// the goldens in the same commit, stating why the library moved.
///
/// Boundaries so far: `ca300bd2…` (recorded with the Python-era goldens),
/// `0a135381…` (after IX.69 slicing and IX.70's opt-in algebraic route),
/// `deafeb4d…` (after XVI.D's failure taxonomy), `ace0b868…` (after XVII.A5's
/// mathematical-failure allow-list), `c4b52cc7…` (after XIX.A's interval-sum
/// fix), `65ced317…` (after XIX.C1's weighted-sum aggregate: the chain/weight
/// builder, the negated-literal linear parser, and the aggregate-band fact),
/// `070ed5b4…` (after IX.83's generalisation: the top-bit mask split, the
/// zero-test/OR fold, the x86 `bit` bound, the shared sub/neg entry points,
/// and the non-chain-equality close set), `6c5bc534…` (after XXIII.B1's
/// per-carrier aggregate premise fast path (bridge.ml)), `895a5105…` (after
/// XXIII.B1's per-carrier aggregate premise fast path with `bridge_lf_vars`
/// returning a term list and the band/finish proof shapes restored to the IX.89
/// `a37b4f1c…` (after XXIII.B3's direct-bound slim close: the oracle
/// close first tries the antecedent set without the `bridge_is_direct_bound`
/// members, then falls back to the exact full-set close), `c59985e5…` (after
/// XXIII.G's propositional-refutation fast path: the oracle close first tries
/// to close a branch from an antecedent conjunction `P` and its AC-equivalent
/// negation `~Q`, then falls back to the slim and full closes unchanged),
/// `47eee00a…` (after XXIII.G's range-refutation fast path: the oracle close
/// next tries to refute a branch from `&0 <= &(val x)` and `&(val x) + &k = &0`
/// (or `&(val x) = -- &k`) with numeral `k > 0`, by a hand chain of order rules;
/// detection constructs nothing, so a no-match costs no inferences and falls
/// through to the slim and full closes unchanged), `b9a19268…` (after
/// XXVII.B5's complement and identity fast paths: the propositional matcher now
/// also sees a literal complement `P` / `~P` and a polynomial-AC complement
/// `P` / `~Q` decided by `INT_POLY_CONV`, and a branch whose goal is an exact
/// integer combination of antecedent equations closes from a verified weight
/// certificate instead of an `INT_ARITH` search.  All three are derived-rule
/// only, so the phrase count and the `prove` count are unchanged; the inlined
/// answer text moved because the library text did), `7f686bbd…` (after
/// XXVII.B9's order-antisymmetry refutation and mask-substitution identity
/// search: a branch carrying both `a <= b` and `b < a` (also when a condition
/// arrives negated) is refuted by `INT_LET_ANTISYM`/`INT_LT_ANTISYM` in a
/// handful of derived-rule inferences, and the linear-certificate greedy gains
/// a bounded backtracking search that accepts a step which moves support from a
/// masked atom to its raw form.  Both are derived-rule only; `prove` count and
/// phrase count are unchanged, the inlined answer text moved.  `cae44c88…` is
/// the lazy-negation/reordered final form (literal before order, so `neg`'s
/// literal branches keep their 15-inference route).  `2182ef78…` (after
/// XXVII.B11's sparse Farkas refutation: the oracle close detects a
/// non-negative integer combination of at most three relational facts whose
/// `&(val ..) >= &0` cancellation leaves a negative constant, and proves `F` by
/// hand with `INT_LE_MUL`/`INT_LE_ADD` plus one `INT_RING` identity.  Detection
/// is pure linear-form arithmetic, so a no-match spends no inference; the two
/// new `prove` calls (the ring identity and the ground `&c < &0`) move the
/// statement count 92 -> 94).
const BRIDGE_DIGEST: &str = "ae942c74d63fbe1f3f450bc3ca3199c264960a7653c96a517de8312116fb42ae";

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn bridge_source() -> Option<String> {
    let path = manifest_dir().join("tools/bridge.ml");
    let src = fs::read_to_string(&path).ok()?;
    let got = sha256(src.as_bytes());
    assert_eq!(
        got, BRIDGE_DIGEST,
        "tools/bridge.ml moved away from the digest the goldens were recorded at.\n\
         Re-record the goldens deliberately and update BRIDGE_DIGEST in the same commit,\n\
         stating in the receipt why the library changed (see TODO.md XVI.A2)."
    );
    Some(src)
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tek-receipt-{}-{}", std::process::id(), tag));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

// ---------------------------------------------------------------------------
// curve_check
// ---------------------------------------------------------------------------

#[test]
fn curve_check_strips_comments_before_reporting() {
    let clean = "(* p_256 and 123456 live only in a comment *)\n\
                 let T = prove (`!x. x + 0 = x`, REWRITE_TAC[]);;\n";
    let (text, statements, hits) = curve_check::check_text(clean);
    assert_eq!(statements, 1);
    assert_eq!(hits, 0, "{text}");
    assert!(text.contains("line 2: CLEAN"), "{text}");
    assert!(text.contains("statements=1 hits=0"), "{text}");

    let hidden = "(* outer (* inner *) still p_384 *) let T = prove (`!x. x = x`, T);;";
    assert_eq!(
        curve_check::strip_comments(hidden),
        " let T = prove (`!x. x = x`, T);;"
    );
}

#[test]
fn curve_check_hit_changes_the_exit_code() {
    let dir = scratch("curve-exit");
    let clean = dir.join("clean.ml");
    fs::write(
        &clean,
        "(* 99999 *)\nlet T = prove (`!x. x = x`, REWRITE_TAC[]);;\n",
    )
    .unwrap();
    let hit = dir.join("hit.ml");
    fs::write(
        &hit,
        "let T = prove (`!x. x MOD p_256 = x`, REWRITE_TAC[]);;\n",
    )
    .unwrap();

    let bin = env!("CARGO_BIN_EXE_curve_check");
    let ok = std::process::Command::new(bin)
        .arg(&clean)
        .output()
        .unwrap();
    assert!(ok.status.success(), "clean file should exit 0");
    let bad = std::process::Command::new(bin).arg(&hit).output().unwrap();
    assert_eq!(bad.status.code(), Some(1), "a hit should exit 1");
    assert!(String::from_utf8_lossy(&bad.stdout).contains("HIT curve:p_256"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn curve_check_reproduces_the_recorded_report() {
    let Some(src) = bridge_source() else {
        return;
    };
    let (text, statements, hits) = curve_check::check_text(&src);
    // 76 after IX.77.  IX.78 added five `prove` calls (81) and, in the same
    // commit, the quotation `` `(*):int->int->int` `` which the scanner read as
    // a block-comment opener; it then deleted the rest of the file and the
    // count was silently pinned to 33.  XIX.B4 fixes the scanner (quotations
    // and strings are opaque), so the honest count is 81 again.  XIX.C1's
    // aggregate block adds five `prove` calls: 81 -> 86, hits still 0.
    // XXIII.B3's direct-bound slim close adds one `prove` call (the slim
    // attempt; the full-set close below it is the fallback and was already
    // there): 91 -> 92, hits still 0.  (The library's `prove` word-boundary
    // occurrences go 95 -> 96.)  XXIII.G's propositional-refutation fast path
    // adds no `prove` call (it uses only derived rules), so the count stays at
    // 92; only the recorded line numbers shift.  XXIII.G's range-refutation
    // fast path is likewise derived-rule only (no `prove`), so the count is
    // still 92; its inserted lines shift the recorded line numbers again.
    // XXVII.B5's literal and polynomial-AC complement matchers and the
    // linear-identity certificate call no new `prove` either (`INT_POLY_CONV`
    // is a conversion, and the certificate's identity is a `TRANS` of two of
    // its results), so the statement count is still 92 and hits are still 0;
    // only the reported line numbers move.  XXVII.B11's sparse Farkas refutation
    // adds two `prove` calls (the `INT_RING` sum identity and the ground
    // `&c < &0`), so the count is 94 and hits remain 0.
    assert_eq!(statements, 94);
    assert_eq!(hits, 0);
    let expected =
        fs::read_to_string(manifest_dir().join("tests/fixtures/curve_check_bridge.txt")).unwrap();
    assert_eq!(text, expected, "curve_check report differs");
}

// ---------------------------------------------------------------------------
// instrument
// ---------------------------------------------------------------------------

#[test]
fn instrument_wraps_a_three_phrase_fixture() {
    let src = "let a = 1;;\n\nlet rec b = 2;;\nprint_string \"x\";;\n";
    let (out, phrases) = instrument::instrument_text(src, "sample.ml");
    assert_eq!(phrases, 3);
    assert!(out.contains("let a =\n  bridge_check \"a\" (fun () -> (\nlet a = 1\n   in a));;\n"));
    assert!(
        out.contains("let b =\n  bridge_check \"b\" (fun () -> (\n\nlet rec b = 2\n   in b));;\n")
    );
    assert!(out.ends_with("print_string \"x\";;\n"));
    // The header records the digest of the source, not of the output.
    assert!(out.contains(&format!("sha256(source) = {}", sha256(src.as_bytes()))));
}

#[test]
fn instrument_reproduces_the_recorded_output() {
    let Some(src) = bridge_source() else {
        return;
    };
    let (out, phrases) = instrument::instrument_text(&src, "tools/bridge.ml");
    // Boundaries: 119 (Python-era) -> 127 (IX.69 slicing) -> 154 (IX.70 adds
    // the opt-in algebraic close, BRIDGE_ALGEBRA_TAC and its helpers) -> 157
    // (XVI.D adds bridge_failure_class, bridge_fallback_count and
    // bridge_slice_retry) -> 160 (XVII.A5 adds bridge_failure_has_prefix,
    // bridge_failure_contains and bridge_math_failure_shape, the allow-list)
    // -> 217 (IX.78 adds the oracle close) -> 221 (XIX.A adds the four sum
    // bound helpers) -> 232 (XIX.C1 adds the aggregate builder, its chain and
    // -> 232 (XIX.C1 adds the aggregate builder, its chain and
    // weight helpers, the linear-combination proof and the aggregate-band fact).
    // 249 after XXIII.B1's per-carrier aggregate premise fast path in
    // `bridge_aggregate_facts` (the phrase count is unchanged; only the body of
    // the two fact provers moved).  Still 249 after XXIII.B3's direct-bound
    // slim close, which adds phrases inside the existing `BRIDGE_ORACLE_TAC`
    // phrase.  Still 249 after XXIII.G's propositional-refutation fast path,
    // which adds local `let ... in` bindings inside the same phrase (no new
    // `;;`).  Still 249 after XXIII.G's range-refutation fast path, which adds
    // only local `let` bindings and derived rules in that same phrase.
    // XXIII.G range boundary. The propositional-path digest was
    //   ec2c6c32d7f2f6e2a43904074f364c0254291b12d6ca6a8585bc3e37d479a97d
    // The first range-path draft digest was
    //   eada667fd90522825a62d90a154781396eb9239ac0d5e6f90141e9c49d5cabb0
    // (before the hand proof was completed; kept only as a boundary).
    // XXVII.B5 boundary.  The XXIII.G range-path digest was
    //   e3370c08e02a0f41a600b2f9351541aed36a612c5076859db0b7b44e5931ab90
    // The literal/AC-complement and linear-certificate fast paths add only
    // local `let ... in` bindings inside `BRIDGE_ORACLE_TAC` (no new `;;`
    // phrase), so the phrase count is still 249.
    // XXVII.B9 boundary.  The XXVII.B5 digest was
    //   25b47f58b76f85857447c69da011efe402c45a1be845f76bca26af615f65a6be
    // The order-antisymmetry refutation and the mask-substitution search are
    // likewise locals inside the existing `BRIDGE_ORACLE_TAC` phrase.
    // XXVII.B11 adds ten top-level helper phrases for the sparse Farkas
    // refutation (the shape parser, the positive-term builders, the scaling and
    // addition rules, and the non-negativity rule), so 249 -> 260 and the output
    // digest moved to `f95a579c...`.
    assert_eq!(phrases, 260);
    assert_eq!(
        sha256(out.as_bytes()),
        "f95a579cce4bb6586fa40b75287541e1696672e99c36e8edff7146fafa8abab4"
    );
}

// ---------------------------------------------------------------------------
// inline_answer
// ---------------------------------------------------------------------------

#[test]
fn inline_answer_splits_phrases_and_skips_directives() {
    let src = "#use \"x\";;\nneeds \"base.ml\";;\nloadt \"y.ml\";;\n\
               let a = 1;;\n\nlet b = 2;;\nneedsome = 3;;\n";
    assert_eq!(
        inline_answer::phrases_from(src),
        vec![
            "let a = 1".to_string(),
            "let b = 2".to_string(),
            "needsome = 3".to_string(),
        ]
    );
}

#[test]
fn inline_answer_reproduces_all_four_recorded_digests() {
    let repo = manifest_dir();
    if !repo.join(".bench/hol-light/Cadical/dimacs.ml").is_file() {
        eprintln!("skipping: .bench sources are absent");
        return;
    }
    let parts = inline_answer::collect(&repo).unwrap();
    // 155 before IX.63; the trimmer adds five `let` phrases to csat_lib.ml.
    // 160 after IX.63; the IX.65 phase markers add three more `let` phrases.
    // 163 after IX.65; IX.69 adds eight `let` phrases to bridge.ml (the slice,
    // its diagnostics, and the S3 controls). 171 after IX.69; IX.70 adds
    // twenty-seven `let`/`let rec` phrases (the algebraic close, its
    // normalisation, the integrality split, the bounds, and the entry point).
    // 198 after IX.70; XVI.D adds three (bridge_failure_class,
    // bridge_fallback_count, bridge_slice_retry); XVII.A5 adds three
    // (bridge_failure_has_prefix, bridge_failure_contains and
    // bridge_math_failure_shape); XIX.A adds four to bridge.ml
    // (bridge_bound_add_lo, bridge_bound_add_hi, bridge_bound_contrib and
    // bridge_bounds_of_terms): 261 -> 265.  XIX.C1 adds eleven `let` phrases to
    // bridge.ml (the aggregate builder, its chain/weight helpers, the
    // linear-combination proof, and the aggregate-band fact): 265 -> 276.
    // XXVII.B11 boundary.  The ten new top-level helper phrases in bridge.ml
    // take the inlined count 293 -> 304; every answer carries the new text.
    assert_eq!(parts.len(), 304);
    // IX.63 boundary. These were the Python-era digests and no longer hold:
    //   add-ref   39fd3d64c65063410c02f0a879d505632b4d82042ac3a64f26c137426e23264f
    //   sub       b36a7fd0274396d72bdbe87f6cd8810888ad308bfe415e952f9bfed015c9e176
    //   neg       b5a674d51aeb621f11611200a7e47451b4d6ada2e434728bf60e6eb49143cb6e
    //   add-p384  83d90067847d411d3eee7fb7b6ca6b532d790ab262341e6ba5d248d54db15044
    // tools/csat_lib.ml gained the backward-closure LRAT trim, so the inlined
    // answer text changed. Byte identity with the Python originals is retired
    // as of IX.63.
    //
    // IX.65 boundary. The IX.63 digests were:
    //   add-ref   9e869067312631a3d43c869d6d031209e7a01e793557b25dfe5f6e76233b93c7
    //   sub       4b387ca42073cfbd25c40817b2bcb071da874fbadd1dc7b7454f6cfc33298736
    //   neg       3c57f6e67d72a6b03b3d53213a82a2dabea12afe7cb107292c58fdf8a8127c9d
    //   add-p384  7893ac4a7a2b7245b9661c3932f56def5f6c7cd19ba5b10bf397a11cc5b7ae48
    // `tools/csat_lib.ml` gained the guarded `csat_mark` phrases (IX.65). The
    // tactics are unchanged (the marker block is a no-op unless a measurement
    // answer sets `csat_mark_enabled`), so the change is the appended library
    // text only. The IX.63 values stay here as the boundary, not an assertion.
    // IX.69 boundary. The IX.65 digests were:
    //   add-ref   ce830bce4eea32d0e6e20b25d17bcc2003f39eda44b3fefc45eeebbded7f17f0
    //   sub       0f6d299d9ff7ccd0d3183a7866e6aab2d74ccbf65507cffaa6f5963aca8884c6
    //   neg       a22340aad872514a0cb869982ec8fcc817d5297091481b6d06b74c49f10cace0
    //   add-p384  0de579654cb3e618cd25a80cc7f2cab3a7109967570928c1ed6f8fefaac105ed
    // `tools/bridge.ml` gained the hypothesis-cone slice (XV.B); every inlined
    // answer carries the new library text, exactly as the IX.63 trimmer and the
    // IX.65 markers did. The tactics are unchanged, so the change is the
    // appended library text only.
    // IX.70 boundary. The IX.69 digests were:
    //   add-ref   c0bf89af740d442826a17c0ac8a9b1db04db69333c5a490535203921447fd183
    //   sub       84c13e28b9e944a74ca444b25b9f84b6735d928d631b73784c2321b3c955f894
    //   neg       38792791e9bab492c4644e9d3f29fca3751255a949fc652fa9832a1f483fc884
    //   add-p384  37095cc23aed03d1b19414f7af79c1cbf43604741849d68e417811234d181f5c
    // `tools/bridge.ml` gained the (opt-in, and for now non-closing) algebraic
    // close (IX.70) and the `full` slice control; the shipped tactics are still
    // the SAT ones, so again the change is the appended library text only.
    // XVI.D boundary. The IX.70 digests were:
    //   add-ref   c8b1ca265e8d96bb750726118f7bd5acec2658a1cbcf420a2b06ace512bc9660
    //   sub       18ace53d924feb26a18babd18208a277399cfccd34d060234289742cf2d13e86
    //   neg       30a98a1e679c772390c56883e1f54a5f16edad69233bcbe79237571952abdece
    //   add-p384  1cffe2f64f58d702a416b3326392f81baf3b3194450ed0b6020f09ffb014feb4
    // `tools/bridge.ml` gained the XVI.D failure taxonomy (the classifier, the
    // tested fallback counter, and the retry policy). The shipped SAT tactics
    // are unchanged; the fallback now runs only for a mathematical failure and
    // resource exhaustion is re-raised, so the change is the appended library
    // text plus the rewritten fallback block.
    // XVII.A5 boundary. The XVI.D digests were:
    //   add-ref   8090b1a9f9de4a1a42cfee4d54072c013b64d1a48e4ea02847ecb4d06fea1df7
    //   sub       d2b93b6dab4a1a39ce5b6b549d24ded367234af5b66d547799ab7d76b3bf8d4d
    //   neg       93c457d6de3e754fe822d76b32e02d41edabfdedd99e84060bc716565d75476c
    //   add-p384  3d9547195cb84b7701d313a3953fa195926c604bb7588c5577a3aecb1a66aed6
    // `tools/bridge.ml` gained the XVII.A5 allow-list (bridge_failure_has_prefix,
    // bridge_failure_contains, bridge_math_failure_shape) and the classifier now
    // sends an unrecognised Failure to `internal` instead of `Mathematical`.
    // The shipped SAT tactics are unchanged; only the appended library text and
    // the classifier body moved.
    //
    // XIX.A boundary.  The XVII.A5 digests were:
    //   add-ref   c26c0080c3db52e5f62ba0aa4175d74ef461d50f4de9e575f5daa5923925f1f7
    //   sub       f600e09a4dd9be44cc935ce1386e4613436816cd403a73aba89a746514d01546
    //   neg       821f71d16ad4c8b0620804f5451ffd188a9a5671ed94bf60215f2ecdc8add5ac
    //   add-p384  91083c8163cca271005806aea20d9a7147adbbf247db33f39c93994412a3c640
    // `tools/bridge.ml` gained the four sum-bound helpers (XIX.A), so every
    // inlined answer carries new library text.  The SAT tactics are unchanged.
    // XXIII.B1 boundary. The XIX.C1 digests were:
    //   add-ref   55920f3e4b9cc42d075c22be8d3ad156c20dd2ff191e7d49882a357c8984ee2c
    //   sub       78a05032259a798c035adb9a6b8bd5926bb11ade48835456025cbf891048b50c
    //   neg       f67d0350a4f1ebab2af9e1c62e841a71d8d24188c3f91b8268c531b2dfb18cbe
    //   add-p384  a50237816f2236d585ae80e0a467d3f1e7eff927bc2d8118210e55c6072fbc6d
    // XXIII.B1 changed `bridge_aggregate_facts` to try only the aggregate(s) that
    // mention a carrier before falling back to the full list. The library text
    // moved, so every inlined answer moved; the tactics are unchanged.
    // XXIII.B3 boundary. The XXIII.B1 digests were:
    //   add-ref   813e6af7ed2827139b6e1ba714878bd035bf34ad56c7a605f48f91db6480ec3c
    //   sub       9ca6b923b95771c1f8e1b671af6f73b4f20f90c59108f1db8aeb39ddb9ef7409
    //   neg       e2d676a2c22e6222dfcf0180239ccccd73e7275f85058ff9a836538c436ec802
    //   add-p384  788472d9279b48fb27a615a45f0b1b41fa9db7e1ab343df6563d0e053c3a31a8
    // XXIII.B3's direct-bound slim close adds the slim/fallback attempt pair
    // inside `BRIDGE_ORACLE_TAC`, so every inlined answer carries the new
    // library text. The phrase count is unchanged (249); the tactics are
    // unchanged.
    // XXIII.G boundary. The XXIII.B3 digests were:
    //   add-ref   b4c7b1f990cf397d360f5da6711fa4e2e0da7c7ed7d81f9a0cbd44da59a4c7db
    //   sub       cd64fbf28cf41ad258fb9424802e2a3df7f0e1d4e41ef768989e0d930956d88d
    //   neg       57e3fb43d6e105bac6e95a221afcf5c0857311f372f3ce1c315398018f2542a3
    //   add-p384  2693d797a0ff8b9497e92f14e828a7b6b27000ad56c661e627f9bc602b8a33a9
    // XXIII.G's propositional-refutation fast path adds a block inside
    // `BRIDGE_ORACLE_TAC`, so every inlined answer carries the new library
    // text. The phrase count is unchanged (293); the tactics are unchanged for
    // a branch whose antecedents hold no propositional contradiction.
    // XXIII.G range boundary. The propositional-path digests were:
    //   add-ref   bc3ff5c510352d957fe77f5f283fb8ae90c51f5bb5e793ffcc459444dde69d5f
    //   sub       8d589eced7935e0685db64a269a90fec0b95080b789c993feb6bb52ac0a33589
    //   neg       222bc7267e8efafd79df87aac285563f369843c6945b53d53c112a2a6992945b
    //   add-p384  ed8397b94a58692856a00a65be9671ad5ae7be6d9d04840e5e2ba7afac3cc10b
    // The range-refutation fast path adds derived-rule-only lines inside
    // `BRIDGE_ORACLE_TAC`; the phrase count is unchanged (293).
    // XXVII.B5 boundary.  The range-path digests were the four above.  The
    // complement and certificate fast paths are locals inside the existing
    // `BRIDGE_ORACLE_TAC` phrase, so the count is still 293 and only the
    // inlined library text moved: every answer now carries the new close code.
    // XXVII.B9 boundary.  The XXVII.B5 digests were the four above.  The
    // order-refutation and mask-substitution search are locals inside that
    // phrase, so the count is still 293.  The values below are the lazy-
    // negation/reordered final form.
    for (mode, digest) in [
        (
            "add-ref",
            "488314dbe8f2bdf47b60fa15f2928def64c7d6866253ee7be10dff3f41b42301",
        ),
        (
            "sub",
            "d0ab40a788c028307b821937e4c752d0af071de652ba14f326d1c98021ea7dda",
        ),
        (
            "neg",
            "b81df16e89f3472d3de9df11cf061a41cd675a8de7e78d3776cd3a66dab2b54b",
        ),
        (
            "add-p384",
            "6212ec641d5af3b684658b5a707a1187608c5acaf0b69f5f647c5f478a3bf580",
        ),
    ] {
        let body = inline_answer::render(&parts, mode).unwrap();
        let got = sha256(body.as_bytes());
        eprintln!("inline_answer mode {mode}: sha256 {got}");
        assert_eq!(got, digest, "mode {mode} differs: {got}");
    }
}

// ---------------------------------------------------------------------------
// sealed_court
// ---------------------------------------------------------------------------

fn synthetic_problems() -> json::Json {
    let entries: Vec<String> = sealed_court::TASK_IDS
        .iter()
        .enumerate()
        .map(|(i, task)| {
            format!(
                "\"{task}\": {{\"inlined_locations\": [[\"ml_files/t{i}.ml\", {}]]}}",
                i + 1
            )
        })
        .collect();
    json::parse(format!("{{{}}}", entries.join(",")).as_bytes()).unwrap()
}

#[test]
fn sealed_court_reads_linenum_from_the_first_inlined_location() {
    let plan = sealed_court::template_plan(&synthetic_problems()).unwrap();
    assert_eq!(plan.len(), 9);
    assert_eq!(plan[0].rel, "ml_files/t0.ml");
    assert_eq!(plan[0].linenum, 1);
    assert_eq!(plan[8].linenum, 9);
}

#[test]
fn sealed_court_truncates_templates_to_the_prefix() {
    let (prefix, count) = sealed_court::prefix_bytes(b"l1\nl2\nl3\n", 2);
    assert_eq!(count, 2);
    assert_eq!(prefix.as_slice(), b"l1\nl2\n".as_slice());

    // A prefix longer than the file keeps everything, including the final
    // fragment that has no newline.
    let (all, n) = sealed_court::prefix_bytes(b"l1\nl2\nl3", 10);
    assert_eq!(n, 3);
    assert_eq!(all.as_slice(), b"l1\nl2\nl3".as_slice());

    let (none, zero) = sealed_court::prefix_bytes(b"l1\nl2\n", 0);
    assert!(none.is_empty());
    assert_eq!(zero, 0);
}

// ---------------------------------------------------------------------------
// dropin sweep
// ---------------------------------------------------------------------------

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

#[test]
fn dropin_sweep_argument_parsing() {
    assert!(dropin::parse_sweep_args(&[]).is_err());
    assert!(dropin::parse_sweep_args(&args(&["--arch", "riscv"])).is_err());
    assert!(dropin::parse_sweep_args(&args(&["--arch", "x86", "--bogus"])).is_err());
    assert!(dropin::parse_sweep_args(&args(&["--arch"])).is_err());
    // Both architectures, and the pair, are now in scope.
    assert_eq!(
        dropin::parse_sweep_args(&args(&["--arch", "arm"]))
            .unwrap()
            .arch,
        "arm"
    );
    assert_eq!(
        dropin::parse_sweep_args(&args(&["--arch", "all"]))
            .unwrap()
            .arch,
        "all"
    );

    let parsed = dropin::parse_sweep_args(&args(&[
        "--arch",
        "x86",
        "--dir",
        "/d",
        "--gen",
        "/g",
        "--hol",
        "/h",
        "--answers",
        "/a",
        "--force",
        "--timeout",
        "120",
        "--dry-run",
    ]))
    .unwrap();
    assert_eq!(parsed.arch, "x86");
    assert_eq!(parsed.dropin, PathBuf::from("/d"));
    assert_eq!(parsed.gen, PathBuf::from("/g"));
    assert_eq!(parsed.hol, PathBuf::from("/h"));
    assert_eq!(parsed.answers, Some(PathBuf::from("/a")));
    assert!(parsed.force);
    assert_eq!(parsed.timeout, 120);
    assert!(parsed.dry_run);
}
