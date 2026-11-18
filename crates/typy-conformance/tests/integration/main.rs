//! The integration tests of `typy-conformance`, one module per area, built as one test program so the
//! crate links its tests once.

mod determinism;
mod documentation;
// The other architecture runs under qemu-user, which only Linux has (NFR-16).
#[cfg(target_os = "linux")]
mod other_architecture;
// LeakSanitizer, which the sanitizer run needs, is not on macOS for Apple silicon (NFR-07).
#[cfg(target_os = "linux")]
mod sanitized;
mod suite;
mod verdicts;
