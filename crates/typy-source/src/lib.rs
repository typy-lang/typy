//! Finds, reads and parses the modules of a TyPy program (the Source context).
#![forbid(unsafe_code)]

mod int_literal;
mod line_index;
mod module;
mod parse;
mod project;
mod settings;
mod source_file;
pub mod syntax;

pub use line_index::LineIndex;
pub use module::{Module, ModuleName, ModuleNameError};
pub use parse::parse;
pub use project::{LoadFailure, Project};
pub use settings::{ProjectSettings, PythonFloor, Release};
pub use source_file::{MAX_SOURCE_BYTES, SourceFile};
pub use syntax::{SyntaxTree, dump};
