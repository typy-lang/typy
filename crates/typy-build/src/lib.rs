//! What a build asks for, and the link that turns object code and the runtime into an executable
//! or a C library.
#![forbid(unsafe_code)]

mod artifact;
mod emit_stage;
mod library_kind;
mod library_layout;
mod library_link;
mod linker;
mod mode;
mod request;
mod runtime_archive;
mod tool;

pub use artifact::{Artifact, ArtifactKind};
pub use emit_stage::EmitStage;
pub use library_kind::LibraryKind;
pub use library_layout::LibraryLayout;
pub use linker::Linker;
pub use mode::BuildMode;
pub use request::{BuildRequest, RequestError};
pub use runtime_archive::RuntimeArchive;
