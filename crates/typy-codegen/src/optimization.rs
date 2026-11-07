//! How much LLVM optimises: one level per build mode (CLI-05).

use inkwell::OptimizationLevel;

/// The optimisation level of a build.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Optimization {
    /// `-O0`, for debug builds.
    None,
    /// `-O2`, for `--release`.
    Speed,
    /// The smallest code, for `--release-small`: LLVM 23 runs `O2` on functions marked `minsize`, in place of the removed `Oz` pipeline.
    Size,
}

impl Optimization {
    /// Returns the pass pipeline LLVM runs.
    pub(crate) const fn pipeline(self) -> &'static str {
        match self {
            Self::None => "default<O0>",
            Self::Speed | Self::Size => "default<O2>",
        }
    }

    /// Returns true when every function is marked `minsize` and `optsize`.
    pub(crate) const fn minimise_size(self) -> bool {
        matches!(self, Self::Size)
    }

    /// Returns the level the code generator uses.
    pub(crate) const fn level(self) -> OptimizationLevel {
        match self {
            Self::None => OptimizationLevel::None,
            Self::Speed | Self::Size => OptimizationLevel::Default,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Optimization;

    #[test]
    fn each_level_has_its_pipeline() {
        assert_eq!(Optimization::None.pipeline(), "default<O0>");
        assert_eq!(Optimization::Speed.pipeline(), "default<O2>");
        assert_eq!(Optimization::Size.pipeline(), "default<O2>");
        assert!(Optimization::Size.minimise_size());
        assert!(!Optimization::Speed.minimise_size());
        assert_eq!(Optimization::None.level(), inkwell::OptimizationLevel::None);
        assert_eq!(
            Optimization::Size.level(),
            inkwell::OptimizationLevel::Default
        );
    }
}
