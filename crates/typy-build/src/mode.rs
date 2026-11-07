//! The three build modes and the optimisation each one asks of code generation (CLI-05).

use typy_codegen::Optimization;

/// How a build trades compile time against the speed and size of the program.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BuildMode {
    /// No optimisation, the default.
    #[default]
    Debug,
    /// Optimised for speed, by `--release`.
    Release,
    /// Optimised for size, by `--release-small`.
    ReleaseSmall,
}

impl BuildMode {
    /// Returns the optimisation code generation runs for the mode.
    #[must_use]
    pub const fn optimization(self) -> Optimization {
        match self {
            Self::Debug => Optimization::None,
            Self::Release => Optimization::Speed,
            Self::ReleaseSmall => Optimization::Size,
        }
    }

    /// Returns the mode's name as the command line and the build cache write it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Release => "release",
            Self::ReleaseSmall => "release-small",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BuildMode;
    use typy_codegen::Optimization;

    #[test]
    fn each_mode_runs_its_optimisation() {
        let cases = [
            (BuildMode::Debug, Optimization::None, "debug"),
            (BuildMode::Release, Optimization::Speed, "release"),
            (BuildMode::ReleaseSmall, Optimization::Size, "release-small"),
        ];
        for (mode, optimization, name) in cases {
            assert_eq!((mode.optimization(), mode.name()), (optimization, name));
        }
        assert_eq!(BuildMode::default(), BuildMode::Debug);
    }
}
