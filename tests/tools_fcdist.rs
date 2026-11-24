//! Tests for the Rust port of `fcdist.py`.
//!
//! The classification rule, the masking and the statistics are exercised on a
//! synthetic checkout. When the pinned s2n-bignum checkout is present, the full
//! report is compared byte-for-byte with the output recorded from the Python
//! tool (`tests/fixtures/fcdist_report.{txt,json}`).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use tekmerion::tools::fcdist::{bench_classify, mask_source, render};

const PINNED_COMMIT: &str = "5e0fc782ae6d587c2ae40faf2c2c4d0b637e6240";

static SEQ: AtomicU32 = AtomicU32::new(0);

struct Tmp(PathBuf);

impl Tmp {
    fn new(tag: &str) -> Tmp {
        let n = SEQ.fetch_add(1, Ordering::SeqCst);
        let p =
            std::env::temp_dir().join(format!("tek-fcdist-{}-{}-{}", std::process::id(), tag, n));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        Tmp(p)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

const PROOFS: &str = "\
let BIGNUM_ADD_CORRECT = prove
  (`!a b. ensures (a + b)`,
   REWRITE_TAC[] THEN
   ARITH_TAC);;

let BIGNUM_LOOP_CORRECT = prove
  (`!n. ensures (n)`,
   ENSURES_WHILE_TAC THEN
   ARITH_TAC);;

let BIGNUM_SUBROUTINE_CORRECT = prove
  (`!a. ensures (a)`,
   REWRITE_TAC[]);;

(* let HIDDEN_CORRECT = prove (`!x. ensures (x)`, ARITH_TAC);; *)
";

fn checkout(tag: &str) -> Tmp {
    let t = Tmp::new(tag);
    let proofs = t.path().join("arm/proofs");
    fs::create_dir_all(&proofs).unwrap();
    fs::write(proofs.join("straight.ml"), PROOFS).unwrap();
    t
}

#[test]
fn mask_source_preserves_offsets_and_newlines() {
    let src = b"(* let HIDDEN = prove (`h`, T);; *)\nlet VISIBLE = prove (`v`, T);;\n\
                let STR = \"(* not a comment\";;\n";
    let masked = mask_source(src);
    assert_eq!(masked.len(), src.len());
    assert_eq!(
        masked.iter().filter(|b| **b == b'\n').count(),
        src.iter().filter(|b| **b == b'\n').count()
    );
    let text = String::from_utf8_lossy(&masked);
    assert!(!text.contains("HIDDEN"));
    assert!(text.contains("VISIBLE"));
    assert!(!text.contains("not a comment"));
}

#[test]
fn bench_classify_mirrors_the_collector_rule() {
    assert_eq!(
        bench_classify("BIGNUM_ADD_CORRECT", "`!x. ensures (x)`"),
        (false, "functional_correctness")
    );
    assert_eq!(
        bench_classify("BIGNUM_ADD_CORRECT", "no backticks here"),
        (true, "the goal is evaluated at runtime")
    );
    assert_eq!(
        bench_classify("T_EQUIV", "`!x. ensures (x)`"),
        (true, "not a proof related to functional correctness")
    );
    assert_eq!(
        bench_classify("T_SUBROUTINE_SAFE", "`!x. ensures (x)`"),
        (true, "not a proof related to functional correctness")
    );
    assert!(bench_classify("BIGNUM_SUBROUTINE_CORRECT", "`!x. ensures (x)`").0);
    assert_eq!(
        bench_classify("BIGNUM_ADD_LEMMA", "`!x. ensures (x)`"),
        (false, "non_fc")
    );
    assert_eq!(
        bench_classify("BIGNUM_ADD_CORRECT", "`!x. 0 <= x`"),
        (false, "non_fc")
    );
}

#[test]
fn synthetic_checkout_report_counts_the_three_declarations() {
    let t = checkout("synthetic");
    let text = render(t.path(), None, false).unwrap();
    assert!(text.contains("files scanned:     1"), "{text}");
    assert!(text.contains("top-level decls:   3"), "{text}");
    assert!(text.contains("  total FC: 2"), "{text}");
    assert!(text.contains("    arm  2"), "{text}");
    assert!(text.contains("  straight-line: 1 (50.0%)"), "{text}");
    assert!(text.contains("  loop:          1 (50.0%)"), "{text}");
    assert!(text.contains("  classified: 1   unclassified: 0"), "{text}");
    assert!(
        text.contains("== named theorems dropped by the rule =="),
        "{text}"
    );
}

#[test]
fn corpus_cross_check_reports_matches_and_examples() {
    let t = checkout("corpus");
    let problems = t.path().join("problems.json");
    fs::write(
        &problems,
        concat!(
            "{\n",
            "  \"arm.bignum_add.BIGNUM_ADD_CORRECT\": {\n",
            "    \"category\": \"functional_correctness_arm\",\n",
            "    \"json\": {\"filename\": \"arm/proofs/straight.ml\"},\n",
            "    \"legacy_id\": \"arm.bignum_add.BIGNUM_ADD_CORRECT\"\n",
            "  },\n",
            "  \"arm.bignum_extra.BIGNUM_EXTRA_CORRECT\": {\n",
            "    \"category\": \"functional_correctness_arm\",\n",
            "    \"json\": {\"filename\": \"arm/proofs/straight.ml\"},\n",
            "    \"legacy_id\": \"arm.bignum_extra.BIGNUM_EXTRA_CORRECT\"\n",
            "  }\n",
            "}\n",
        ),
    )
    .unwrap();

    let text = render(t.path(), Some(&problems), false).unwrap();
    assert!(text.contains("  matched (file, theorem): 1"), "{text}");
    assert!(text.contains("  only direct: 1   only corpus: 1"), "{text}");
    assert!(text.contains("  corpus FC: 2 {'arm': 2}"), "{text}");
    assert!(text.contains("    only-corpus examples:"), "{text}");
}

#[test]
fn non_checkout_is_refused() {
    let t = Tmp::new("not-checkout");
    let error = render(t.path(), None, false).unwrap_err();
    assert!(error.contains("not an s2n-bignum checkout"));
}

#[test]
fn json_output_is_parseable_and_has_the_expected_keys() {
    let t = checkout("json");
    let text = render(t.path(), None, true).unwrap();
    let parsed = tekmerion::tools::json::parse(text.as_bytes()).unwrap();
    assert!(parsed.get("direct").is_some());
    assert!(parsed.get("all_declarations").is_some());
    assert_eq!(
        parsed.get("all_declarations"),
        Some(&tekmerion::tools::json::Json::Num(3.0))
    );
}

fn normalise(text: &str) -> String {
    let cwd = std::env::current_dir().unwrap();
    let s2n = cwd.join(".bench/s2n-bignum").to_string_lossy().into_owned();
    let problems = cwd
        .join(".bench/problems.json")
        .to_string_lossy()
        .into_owned();
    text.replace(&problems, "<PROBLEMS>").replace(&s2n, "<S2N>")
}

/// Differential check against the recorded Python output. Skips when the
/// benchmark checkout is absent or not at the pinned revision.
#[test]
fn pinned_checkout_matches_the_recorded_python_report() {
    let checkout = Path::new(".bench/s2n-bignum");
    if !checkout.join("arm/proofs").is_dir() {
        eprintln!("skipping: .bench/s2n-bignum is absent");
        return;
    }

    let human = render(checkout, Some(Path::new(".bench/problems.json")), false).unwrap();
    if !human.starts_with(&format!("s2n-bignum commit: {PINNED_COMMIT}")) {
        eprintln!("skipping: checkout is not at the pinned commit");
        return;
    }
    let expected =
        fs::read_to_string(manifest_dir().join("tests/fixtures/fcdist_report.txt")).unwrap();
    assert_eq!(normalise(&human), expected, "human report differs");

    let json = render(checkout, Some(Path::new(".bench/problems.json")), true).unwrap();
    let expected_json =
        fs::read_to_string(manifest_dir().join("tests/fixtures/fcdist_report.json")).unwrap();
    assert_eq!(normalise(&json), expected_json, "JSON report differs");
}
