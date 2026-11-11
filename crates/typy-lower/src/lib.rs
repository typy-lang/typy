//! Lowers a checked program to TyIR, with reference counting and error returns (the Lowering context).
#![forbid(unsafe_code)]

mod arithmetic;
mod augmented;
mod borrow;
mod builtin;
mod call_graph;
mod class;
mod comparison;
mod comprehension;
mod conversion;
mod dict;
mod dict_call;
mod exception;
mod expression;
mod fstring;
mod function;
mod import;
mod invoke;
mod iterable_builtin;
mod iterator;
mod lazy;
mod list;
mod list_method;
mod loop_source;
mod mangle;
mod module_variable;
mod optional;
mod ownership;
mod program;
mod set;
mod slice;
mod statement;
mod str_method;
mod strings;
mod tuple;
mod type_object;
mod types;
mod view_set;

pub use program::lower;
