//! Finds, reads and parses the modules of a TyPy program (the Source context).
#![forbid(unsafe_code)]

mod int_literal;
mod lazy;
mod line_index;
mod loader;
mod module;
mod parse;
mod project;
mod settings;
mod settings_file;
mod source_file;
mod stdlib_names;
pub mod syntax;

pub use lazy::{is_lazy, lazy_modules};
pub use line_index::LineIndex;
pub use loader::{ANNOTATION_MODULES, Import, absolute_name};
pub use module::{Module, ModuleName, ModuleNameError};
pub use parse::{MAX_DEPTH, Parsed, parse, parse_with_warnings};
pub use project::{LoadFailure, Project};
pub use settings::{ProjectSettings, PythonFloor, Release};
pub use settings_file::{SETTINGS_FILE, SettingOverrides};
pub use source_file::{MAX_SOURCE_BYTES, SourceFile};
pub use syntax::{SyntaxTree, dump};
