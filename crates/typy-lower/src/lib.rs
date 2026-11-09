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
mod iterable_builtin;
mod list;
mod list_method;
mod loop_source;
mod mangle;
mod ownership;
mod program;
mod slice;
mod statement;
mod str_method;
mod strings;
mod tuple;
mod types;

pub use program::lower;
