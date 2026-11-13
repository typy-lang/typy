//! Runs the compiler's stages in order: load, check, lower, generate code and link.
#![forbid(unsafe_code)]

mod emission;
mod failure;
mod outcome;
mod pipeline;
mod sources;
mod success;

pub use emission::Emission;
pub use failure::Failure;
pub use outcome::Outcome;
pub use pipeline::Pipeline;
pub use sources::Sources;
pub use success::{Success, Warnings};
