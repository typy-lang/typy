//! A file a build writes.

use camino::Utf8PathBuf;

/// What kind of file a build wrote.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactKind {
    /// A native executable.
    Executable,
    /// An object file, from `--emit=obj`.
    Object,
    /// The text of a stage, from `--emit`.
    Text,
}

/// A file a build wrote, and its kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Artifact {
    /// Where the file is.
    pub path: Utf8PathBuf,
    /// What it holds.
    pub kind: ArtifactKind,
}
