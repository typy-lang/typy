//! A build or check that found no errors, with the warnings it found (DIA-07).

use typy_diagnostics::Diagnostics;

use crate::outcome::Outcome;
use crate::sources::Sources;

/// What a build or check produced, and its warnings with the files they point into.
#[derive(Clone, Debug)]
pub struct Success {
    /// The executable, the stage output, or the checked program.
    pub outcome: Outcome,
    /// The warnings, which never fail a build unless warnings are denied.
    pub warnings: Warnings,
}

/// Warnings with the source files they are rendered against.
#[derive(Clone, Debug, Default)]
pub struct Warnings {
    /// Every warning, in the order the stages found them.
    pub diagnostics: Diagnostics,
    /// The files the warnings point into; none when there are no warnings.
    pub sources: Sources,
}
