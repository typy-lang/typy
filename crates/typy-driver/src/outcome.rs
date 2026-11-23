//! What a successful build produced.

use typy_build::{Artifact, EmitStage};

use crate::emission::Emission;
use crate::sources::Sources;

/// A successful build: the executable it linked, or the output of each stage it was asked
/// to write.
#[derive(Clone, Debug)]
pub enum Outcome {
    /// The build linked an executable.
    Executable(Artifact),
    /// The build wrote the output of each stage `--emit` named, in pipeline order (CLI-29).
    Emitted(Vec<(EmitStage, Emission)>),
    /// The check found no errors; the program and its files are kept.
    Checked(Sources),
}
