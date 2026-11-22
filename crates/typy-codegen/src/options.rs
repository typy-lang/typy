//! What a build asks of code generation beyond its target: how hard to optimise, and for
//! which processor (CLI-20, CLI-21, CLI-30, ADR-0104).

use crate::optimization::Optimization;

/// The options of code generation.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct CodegenOptions {
    /// How hard LLVM optimises.
    pub optimization: Optimization,
    /// The processor to generate code for; `None` for the target's baseline, which every
    /// machine of the target runs (NFR-06).
    pub processor: Option<String>,
    /// The features to turn on or off beside the processor's, as LLVM writes them, such as
    /// `+avx2,-sse4a`; empty for the processor's own.
    pub features: String,
}

impl CodegenOptions {
    /// Returns the options of `optimization` for the baseline processor.
    #[must_use]
    pub fn new(optimization: Optimization) -> Self {
        Self {
            optimization,
            ..Self::default()
        }
    }
}
