//! Where a program's main module comes from: a file, a module of the project, or code given
//! on the command line (CLI-02, CLI-28).

use std::fmt;

use camino::Utf8PathBuf;

use crate::module::ModuleName;

/// The path the main module has when it is code, as CPython names code given with `-c`.
pub const CODE_PATH: &str = "<string>";

/// Where a program's main module comes from.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MainSource {
    /// A `.py` file, whose directory is the project's root.
    File(Utf8PathBuf),
    /// A module of the project rooted at the current directory, run as `python -m` runs it:
    /// the packages that hold it run first, and a package runs its `__main__` module.
    Module(ModuleName),
    /// Code, run as `python -c` runs it, with the project rooted at the current directory;
    /// diagnostics name it `<string>`.
    Code(String),
}

impl MainSource {
    /// Returns the project's root: the file's directory, or `.` for a module or code.
    #[must_use]
    pub fn root(&self) -> Utf8PathBuf {
        match self {
            Self::File(path) => match path.parent() {
                Some(parent) if !parent.as_str().is_empty() => parent.to_owned(),
                _ => Utf8PathBuf::from("."),
            },
            Self::Module(_) | Self::Code(_) => Utf8PathBuf::from("."),
        }
    }

    /// Returns the name of the program's executable: the file's name without `.py`, the
    /// module's name, or `c` for code.
    #[must_use]
    pub fn program_name(&self) -> &str {
        match self {
            Self::File(path) => path.file_stem().unwrap_or("program"),
            Self::Module(name) => name.as_str(),
            Self::Code(_) => "c",
        }
    }
}

/// Writes the source as the command line gives it: the file's path, `-m NAME`, or `-c`.
impl fmt::Display for MainSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::File(path) => write!(formatter, "{path}"),
            Self::Module(name) => write!(formatter, "-m {name}"),
            Self::Code(_) => formatter.write_str("-c"),
        }
    }
}

#[cfg(test)]
mod tests;
