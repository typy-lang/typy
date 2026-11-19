//! The end-to-end tests of `typy-cli`, which run `typy` as a subprocess, one module per area,
//! built as one test program so the crate links its tests once.

mod allocator;
mod cli;
mod common;
mod files;
mod generators;
mod library;
mod memory;
mod nesting;
mod structures;
mod sys;
