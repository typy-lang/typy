//! The source files a build read, which diagnostics are rendered against.

use typy_diagnostics::{FileId, Position, SourceLookup, TextOffset};
use typy_source::{LoadFailure, Project};

/// The source files of a build: a loaded project, the files of one that failed to load, or none
/// when the build stopped before reading any.
#[derive(Clone, Debug, Default)]
pub enum Sources {
    /// Nothing was read; only diagnostics without a location occur.
    #[default]
    None,
    /// The project loaded.
    Project(Box<Project>),
    /// The project did not load; its files are kept for the diagnostics.
    Unloaded(Box<LoadFailure>),
}

impl SourceLookup for Sources {
    fn path(&self, file: FileId) -> &str {
        match self {
            Self::None => "",
            Self::Project(project) => project.path(file),
            Self::Unloaded(failure) => failure.path(file),
        }
    }

    fn position(&self, file: FileId, offset: TextOffset) -> Position {
        match self {
            Self::None => Position { line: 1, column: 1 },
            Self::Project(project) => project.position(file, offset),
            Self::Unloaded(failure) => failure.position(file, offset),
        }
    }

    fn line_text(&self, file: FileId, line: u32) -> &str {
        match self {
            Self::None => "",
            Self::Project(project) => project.line_text(file, line),
            Self::Unloaded(failure) => failure.line_text(file, line),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Sources;
    use typy_diagnostics::{FileId, Position, SourceLookup, TextOffset};

    #[test]
    fn no_sources_answer_with_empty_text() {
        let sources = Sources::None;
        assert_eq!(sources.path(FileId(0)), "");
        assert_eq!(sources.line_text(FileId(0), 1), "");
        assert_eq!(
            sources.position(FileId(0), TextOffset(9)),
            Position { line: 1, column: 1 }
        );
    }
}
