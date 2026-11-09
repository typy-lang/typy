//! Lowers a checked program to TyIR, with reference counting and error returns (the Lowering context).
#![forbid(unsafe_code)]

mod arithmetic;
mod augmented;
mod builtin;
mod call_graph;
mod comparison;
mod comprehension;
mod conversion;
mod dict;
mod expression;
mod function;
mod invoke;
mod iterable_builtin;
mod list;
mod list_method;
mod loop_source;
mod mangle;
mod optional;
mod ownership;
mod program;
mod set;
mod slice;
mod statement;
mod str_method;
mod strings;
mod tuple;
mod types;

pub use program::lower;
