//! A build or check that found errors.

use typy_diagnostics::Diagnostics;

use crate::sources::Sources;

/// The diagnostics of a failed build, with the source files they point into.
#[derive(Clone, Debug)]
pub struct Failure {
    /// Every diagnostic, at least one of them an error.
    pub diagnostics: Diagnostics,
    /// The files the diagnostics are rendered against.
    pub sources: Sources,
}

impl Failure {
    /// Returns a failure that read no source file, such as a missing runtime archive.
    #[must_use]
    pub fn without_sources(diagnostics: Diagnostics) -> Self {
        Self {
            diagnostics,
            sources: Sources::None,
        }
    }
}
