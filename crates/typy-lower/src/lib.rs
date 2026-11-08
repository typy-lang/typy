//! Lowers a checked program to TyIR, with reference counting and error returns (the Lowering context).
#![forbid(unsafe_code)]

mod arithmetic;
mod builtin;
mod call_graph;
mod comparison;
mod conversion;
mod expression;
mod function;
mod invoke;
mod mangle;
mod ownership;
mod program;
mod statement;
mod strings;
mod tuple;
mod types;

pub use program::lower;
