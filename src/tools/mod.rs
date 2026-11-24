//! Tekmerion-owned tooling. **Not on the native verdict path.**
//!
//! These modules replace the Python helpers used to build the assurance
//! receipts. They are ordinary Rust with no new dependencies, and
//! `native::check` never calls them.

pub mod challenge;
pub mod json;
pub mod mutate_guard;
pub mod script_gate;
