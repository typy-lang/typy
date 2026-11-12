//! The settings a command builds once, when it starts.

use camino::Utf8PathBuf;

/// A TyPy release, which selects the features the subset checker accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Release {
    /// TyPy 0.1.
    V0_1,
}

impl Release {
    /// Returns the release as it is written, such as `0.1`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::V0_1 => "0.1",
        }
    }
}

/// The oldest CPython version a program supports (ADR-0008).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PythonFloor {
    /// CPython 3.12, the default.
    #[default]
    V3_12,
    /// CPython 3.13.
    V3_13,
    /// CPython 3.14.
    V3_14,
    /// CPython 3.15.
    V3_15,
}

impl PythonFloor {
    /// Every floor, oldest first.
    pub const ALL: [Self; 4] = [Self::V3_12, Self::V3_13, Self::V3_14, Self::V3_15];

    /// Returns the floor as it is written in `typy.toml`, such as `3.12`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::V3_12 => "3.12",
            Self::V3_13 => "3.13",
            Self::V3_14 => "3.14",
            Self::V3_15 => "3.15",
        }
    }

    /// Parses `3.12` to `3.15`, and nothing else (CLI-12).
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|floor| floor.as_str() == text)
    }

    /// Returns the minor version, such as 12 for 3.12.
    #[must_use]
    pub const fn minor(self) -> u8 {
        match self {
            Self::V3_12 => 12,
            Self::V3_13 => 13,
            Self::V3_14 => 14,
            Self::V3_15 => 15,
        }
    }
}

/// The settings of one command; immutable once the project is loaded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectSettings {
    /// The directory of the main file; every module path is relative to it.
    pub root: Utf8PathBuf,
    /// The release whose feature table the subset checker uses.
    pub release: Release,
    /// The parser's target version and the lowest oracle version.
    pub python_floor: PythonFloor,
    /// The directories, relative to the root, searched for modules after the root itself,
    /// in order (MOD-01).
    pub module_paths: Vec<Utf8PathBuf>,
}

impl ProjectSettings {
    /// Returns the settings for a project rooted at `root`, with the defaults of 0.1.
    #[must_use]
    pub fn new(root: Utf8PathBuf) -> Self {
        Self {
            root,
            release: Release::V0_1,
            python_floor: PythonFloor::default(),
            module_paths: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ProjectSettings, PythonFloor, Release};

    #[test]
    fn every_floor_parses_from_its_own_text() {
        for floor in PythonFloor::ALL {
            assert_eq!(PythonFloor::parse(floor.as_str()), Some(floor));
            assert_eq!(floor.as_str(), format!("3.{}", floor.minor()));
        }
    }

    #[test]
    fn floors_outside_the_oracle_range_are_refused() {
        for text in ["", "3.11", "3.16", "3", "3.12.0", " 3.12", "312"] {
            assert_eq!(PythonFloor::parse(text), None, "{text:?}");
        }
    }

    #[test]
    fn default_settings_use_release_0_1_and_floor_3_12() {
        let settings = ProjectSettings::new("/work".into());
        assert_eq!(settings.release, Release::V0_1);
        assert_eq!(settings.release.as_str(), "0.1");
        assert_eq!(settings.python_floor, PythonFloor::V3_12);
    }
}
