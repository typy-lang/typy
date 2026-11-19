//! The integration tests of `typy-check`, one module per area, built as one test program so the
//! crate links its tests once.

mod checker;
mod class_bodies;
mod common;
mod ctypes;
mod edge_cases;
mod exception_parts;
mod exports;
mod final_names;
mod function_values;
mod generators;
mod generics;
mod lambda_expressions;
mod lazy_imports;
mod matching;
mod math;
mod modules;
mod nested_functions;
mod nonlocal_statement;
mod open;
mod operators;
mod parameters;
mod percent;
mod protocols;
mod sys;
mod type_aliases;
mod unions;
