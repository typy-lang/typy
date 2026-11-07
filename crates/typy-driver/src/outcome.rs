//! What a successful build produced.

use typy_build::Artifact;

use crate::emission::Emission;
use crate::sources::Sources;

/// A successful build: the executable it linked, or the output of the stage it stopped at.
#[derive(Clone, Debug)]
pub enum Outcome {
    /// The build linked an executable.
    Executable(Artifact),
    /// The build stopped after the stage `--emit` named.
    Emitted(Emission),
    /// The check found no errors; the program and its files are kept.
    Checked(Sources),
}
