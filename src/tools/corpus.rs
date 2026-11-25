//! The task set and committed input fixtures for the emitter parity suite.
//!
//! `tests/tools_emit.rs` compares the Rust emitters with the tracked reference
//! answers for 27 tasks. The answers are in git (`tests/fixtures/answers/`), but
//! the inputs the emitter reads are not: `.bench/` is git-ignored, so a CI
//! checkout has neither the 42 MB `problems.json` nor the small ELF objects or
//! the three Cadical sources the answer inliner reads.
//!
//! This module is the single definition of *which* 27 tasks and *where* their
//! committed inputs live. `src/bin/record_problems_fixture.rs` writes them and
//! the tests read them, so the two cannot drift apart.
//!
//! **Not on the verdict path.**

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The 18 ARM tasks the parity suite covers.
pub const ARM_TASKS: [&str; 18] = [
    "arm.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT",
    "arm.bignum_add_p256.BIGNUM_ADD_P256_CORRECT",
    "arm.bignum_add_p256k1.BIGNUM_ADD_P256K1_CORRECT",
    "arm.bignum_add_p384.BIGNUM_ADD_P384_CORRECT",
    "arm.bignum_add_p521.BIGNUM_ADD_P521_CORRECT",
    "arm.bignum_add_sm2.BIGNUM_ADD_SM2_CORRECT",
    "arm.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT",
    "arm.bignum_neg_p256.BIGNUM_NEG_P256_CORRECT",
    "arm.bignum_neg_p256k1.BIGNUM_NEG_P256K1_CORRECT",
    "arm.bignum_neg_p384.BIGNUM_NEG_P384_CORRECT",
    "arm.bignum_neg_p521.BIGNUM_NEG_P521_CORRECT",
    "arm.bignum_neg_sm2.BIGNUM_NEG_SM2_CORRECT",
    "arm.bignum_sub_p25519.BIGNUM_SUB_P25519_CORRECT",
    "arm.bignum_sub_p256.BIGNUM_SUB_P256_CORRECT",
    "arm.bignum_sub_p256k1.BIGNUM_SUB_P256K1_CORRECT",
    "arm.bignum_sub_p384.BIGNUM_SUB_P384_CORRECT",
    "arm.bignum_sub_p521.BIGNUM_SUB_P521_CORRECT",
    "arm.bignum_sub_sm2.BIGNUM_SUB_SM2_CORRECT",
];

/// The 9 x86 tasks the parity suite covers.
pub const X86_TASKS: [&str; 9] = [
    "x86.bignum_add_p25519.BIGNUM_ADD_P25519_CORRECT",
    "x86.bignum_add_p256k1.BIGNUM_ADD_P256K1_CORRECT",
    "x86.bignum_add_sm2.BIGNUM_ADD_SM2_CORRECT",
    "x86.bignum_neg_p25519.BIGNUM_NEG_P25519_CORRECT",
    "x86.bignum_neg_p256k1.BIGNUM_NEG_P256K1_CORRECT",
    "x86.bignum_neg_sm2.BIGNUM_NEG_SM2_CORRECT",
    "x86.bignum_sub_p25519.BIGNUM_SUB_P25519_CORRECT",
    "x86.bignum_sub_p256k1.BIGNUM_SUB_P256K1_CORRECT",
    "x86.bignum_sub_sm2.BIGNUM_SUB_SM2_CORRECT",
];

/// The Cadical sources every answer inlines. They are read from
/// `.bench/hol-light/Cadical/` (live) or `tests/fixtures/bench/hol-light/`
/// (committed), not from `tools/`.
pub const CADICAL_SOURCES: [&str; 3] = ["dimacs.ml", "cnf.ml", "ldrup.ml"];

/// Every task id the parity suite covers, ARM first then x86.
pub fn tasks() -> impl Iterator<Item = &'static str> {
    ARM_TASKS.into_iter().chain(X86_TASKS)
}

/// `tests/fixtures/problems.json`: the committed 27-entry corpus excerpt.
pub fn problems_fixture(repo: &Path) -> PathBuf {
    problems_fixture_in(&repo.join("tests/fixtures"))
}

/// `<fixtures>/problems.json` for a caller that already has the fixtures dir.
pub fn problems_fixture_in(fixtures: &Path) -> PathBuf {
    fixtures.join("problems.json")
}

/// `tests/fixtures/bench`: the committed object and Cadical inputs, laid out as
/// they appear under `.bench/`.
pub fn bench_fixture(repo: &Path) -> PathBuf {
    bench_fixture_in(&repo.join("tests/fixtures"))
}

/// `<fixtures>/bench` for a caller that already has the fixtures dir.
pub fn bench_fixture_in(fixtures: &Path) -> PathBuf {
    fixtures.join("bench")
}

/// Copy the committed bench inputs into `root/.bench`, creating the layout the
/// emitter and guard expect. `problems.json` is deliberately not copied: callers
/// point at it with `TEKMERION_PROBLEMS`, which is the override under test.
pub fn materialise_bench_inputs(fixtures: &Path, root: &Path) -> io::Result<()> {
    let bench = fixtures.join("bench");
    copy_tree(&bench.join("s2n-bignum"), &root.join(".bench/s2n-bignum"))?;
    copy_tree(&bench.join("hol-light"), &root.join(".bench/hol-light"))?;
    Ok(())
}

/// Recursively copy `src` into `dst`, creating directories as needed.
pub fn copy_tree(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}
