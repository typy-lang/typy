//! What a build asks for, and the link that turns object code and the runtime into an executable.
#![forbid(unsafe_code)]

mod artifact;
mod emit_stage;
mod linker;
mod mode;
mod request;
mod runtime_archive;

pub use artifact::{Artifact, ArtifactKind};
pub use emit_stage::EmitStage;
pub use linker::Linker;
pub use mode::BuildMode;
pub use request::{BuildRequest, RequestError};
pub use runtime_archive::RuntimeArchive;
