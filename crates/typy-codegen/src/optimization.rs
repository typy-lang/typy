//! How hard LLVM optimises: the levels `-O0` to `-O3`, `-Os` and `-Oz` (CLI-05, CLI-20).

use inkwell::OptimizationLevel;

/// The optimisation level of a build.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Optimization {
    /// `-O0`, for debug builds.
    #[default]
    None,
    /// `-O1`.
    Basic,
    /// `-O2`, for `--release`.
    Speed,
    /// `-O3`.
    Aggressive,
    /// `-Os`: `O2` on functions marked `optsize`.
    Small,
    /// `-Oz`, the smallest code, for `--release-small`: LLVM 23 runs `O2` on functions marked
    /// `minsize`, in place of the removed `Oz` pipeline.
    Size,
}

impl Optimization {
    /// Every level, as `-O` names them.
    pub const ALL: [Self; 6] = [
        Self::None,
        Self::Basic,
        Self::Speed,
        Self::Aggressive,
        Self::Small,
        Self::Size,
    ];

    /// Returns the level `-O` names with `text`: `0`, `1`, `2`, `3`, `s` or `z`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|level| level.letter() == text)
    }

    /// Returns what follows `-O` for the level, such as `2` or `z`.
    #[must_use]
    pub const fn letter(self) -> &'static str {
        match self {
            Self::None => "0",
            Self::Basic => "1",
            Self::Speed => "2",
            Self::Aggressive => "3",
            Self::Small => "s",
            Self::Size => "z",
        }
    }

    /// Returns true for `-Os` and `-Oz`, which link the runtime optimised for size (ADR-0100).
    #[must_use]
    pub const fn favours_size(self) -> bool {
        matches!(self, Self::Small | Self::Size)
    }

    /// Returns the pass pipeline LLVM runs.
    pub(crate) const fn pipeline(self) -> &'static str {
        match self {
            Self::None => "default<O0>",
            Self::Basic => "default<O1>",
            Self::Speed | Self::Small | Self::Size => "default<O2>",
            Self::Aggressive => "default<O3>",
        }
    }

    /// Returns true when every function is marked `optsize`, which `-Os` and `-Oz` ask for.
    pub(crate) const fn optimise_size(self) -> bool {
        self.favours_size()
    }

    /// Returns true when every function is marked `minsize` too, which `-Oz` asks for.
    pub(crate) const fn minimise_size(self) -> bool {
        matches!(self, Self::Size)
    }

    /// Returns the level the code generator uses.
    pub(crate) const fn level(self) -> OptimizationLevel {
        match self {
            Self::None => OptimizationLevel::None,
            Self::Basic => OptimizationLevel::Less,
            Self::Speed | Self::Small | Self::Size => OptimizationLevel::Default,
            Self::Aggressive => OptimizationLevel::Aggressive,
        }
    }
}

#[cfg(test)]
mod tests;
