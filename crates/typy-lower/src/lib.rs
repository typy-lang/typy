//! Lowers a checked program to TyIR, with reference counting and error returns (the Lowering context).
#![forbid(unsafe_code)]

mod arithmetic;
mod augmented;
mod builtin;
mod call_graph;
mod comparison;
mod conversion;
mod expression;
mod function;
mod invoke;
mod list;
mod list_method;
mod mangle;
mod ownership;
mod program;
mod slice;
mod statement;
mod strings;
mod tuple;
mod types;

pub use program::lower;
