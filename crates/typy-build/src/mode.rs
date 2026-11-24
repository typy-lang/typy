//! The build modes: an optimisation level, `-O0` to `-O3`, `-Os` or `-Oz`, of which the
//! debug build, `--release` and `--release-small` are three (CLI-05, CLI-20).

use typy_codegen::Optimization;

/// How a build trades compile time against the speed and size of the program.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct BuildMode {
    optimization: Optimization,
}

impl BuildMode {
    /// No optimisation, the default: `-O0`.
    pub const DEBUG: Self = Self::of(Optimization::None);
    /// Optimised for speed, by `--release`: `-O2`.
    pub const RELEASE: Self = Self::of(Optimization::Speed);
    /// Optimised for size, by `--release-small`: `-Oz`.
    pub const RELEASE_SMALL: Self = Self::of(Optimization::Size);

    /// Returns the mode of an optimisation level.
    #[must_use]
    pub const fn of(optimization: Optimization) -> Self {
        Self { optimization }
    }

    /// Returns the optimisation code generation runs for the mode.
    #[must_use]
    pub const fn optimization(self) -> Optimization {
        self.optimization
    }

    /// Returns true for the debug build, `-O0`, which keeps debug information (CLI-05).
    #[must_use]
    pub const fn is_debug(self) -> bool {
        matches!(self.optimization, Optimization::None)
    }

    /// Returns the directory of `<prefix>/lib/` whose runtime archive the mode links: the one
    /// optimised for size for `-Os` and `-Oz`, `--release-small` among them, and the one
    /// optimised for speed otherwise (ADR-0100).
    #[must_use]
    pub const fn runtime_directory(self) -> &'static str {
        if self.optimization.favours_size() {
            "typy-small"
        } else {
            "typy"
        }
    }

    /// Returns the mode's name as the command line and the build cache write it: `debug`,
    /// `release` and `release-small` for those three, and `O1`, `O3` and `Os` for the others.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self.optimization {
            Optimization::None => "debug",
            Optimization::Speed => "release",
            Optimization::Size => "release-small",
            Optimization::Basic => "O1",
            Optimization::Aggressive => "O3",
            Optimization::Small => "Os",
        }
    }
}

#[cfg(test)]
mod tests;
