//! The integration tests of `typy-conformance`, one module per area, built as one test program so the
//! crate links its tests once.

mod determinism;
mod documentation;
mod other_architecture;
mod sanitized;
mod suite;
mod verdicts;
