//! Tekmerion-owned tooling. **Not on the native verdict path.**
//!
//! These modules replace the Python helpers used to build the assurance
//! receipts. They are ordinary Rust with no new dependencies, and
//! `native::check` never calls them.

pub mod attempt_id;
pub mod attempt_lifecycle;
pub mod challenge;
pub mod curve_check;
pub mod derive;
pub mod disasm;
pub mod dropin;
pub mod emit;
pub mod fcdist;
pub mod guard;
pub mod inline_answer;
pub mod instrument;
pub mod json;
pub mod mutate;
pub mod mutate_guard;
pub mod mutate_lifecycle;
pub mod patch_wrapper;
pub mod run_traces_inject;
pub mod script_gate;
pub mod sealed_court;
pub mod sha256;
pub mod trace_conformance;
