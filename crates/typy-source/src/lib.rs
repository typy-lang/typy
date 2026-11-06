//! Finds, reads and parses the modules of a TyPy program (the Source context).
#![forbid(unsafe_code)]

mod int_literal;
mod module;
mod parse;
mod project;
mod settings;
mod source_file;
pub mod syntax;

pub use module::{Module, ModuleName, ModuleNameError};
pub use parse::parse;
pub use project::{LoadFailure, Project};
pub use settings::{ProjectSettings, PythonFloor, Release};
pub use source_file::{LineIndex, MAX_SOURCE_BYTES, SourceFile};
pub use syntax::{SyntaxTree, dump};
