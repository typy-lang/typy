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

/// The C functions a program allocates its memory with, in place of `malloc`, `realloc` and
/// `free`, and the library that defines them (MEM-08, ADR-0043).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Allocator {
    /// The function that allocates, as `malloc` does.
    pub allocate: String,
    /// The function that resizes an allocation, as `realloc` does.
    pub reallocate: String,
    /// The function that frees an allocation, as `free` does.
    pub free: String,
    /// The library the linker finds as it finds `-lname`, or `None` when a library the
    /// program links anyway defines the functions.
    pub library: Option<String>,
}

/// How a C library a `ctypes` declaration names is linked (FFI-04, ADR-0047); the build's
/// `library_argument` gives the linker's argument for each target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibraryLink {
    /// The shared library, which the system loads when the program starts.
    Dynamic,
    /// The archive of the same name up to `.so` or `.dylib`, linked into the program.
    Static,
}

/// The operating system a program is built for, which `sys.platform` names (LIB-22).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    /// Linux, whose `sys.platform` is `linux`.
    Linux,
    /// macOS, whose `sys.platform` is `darwin`.
    MacOs,
}

impl Platform {
    /// Returns the operating system `typy` itself runs on.
    #[must_use]
    pub const fn host() -> Self {
        if cfg!(target_os = "macos") {
            Self::MacOs
        } else {
            Self::Linux
        }
    }

    /// Returns `sys.platform` on the operating system, as CPython gives it.
    #[must_use]
    pub const fn sys_platform(self) -> &'static str {
        match self {
            Self::Linux => "linux",
            Self::MacOs => "darwin",
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
    /// The allocator the project names, or `None` for `malloc`, `realloc` and `free`
    /// (MEM-08).
    pub allocator: Option<Allocator>,
    /// The main file is built as a C library: its module is named after its file, and its
    /// `__all__` names the functions it exports (CLI-09, FFI-05, ADR-0044).
    pub library: bool,
    /// The version of a library whose module has no `__version__`, which its file names
    /// carry (CLI-13).
    pub version: Option<String>,
    /// The directories, relative to the root, where the linker looks for libraries first
    /// (FFI-04).
    pub library_paths: Vec<Utf8PathBuf>,
    /// How each C library a `ctypes` declaration names is linked, by its name; one not
    /// named is linked dynamically (FFI-04).
    pub libraries: std::collections::BTreeMap<String, LibraryLink>,
    /// The C tools `typy.toml` names for a target, by its triple, in place of those TyPy
    /// would run (CLI-14).
    pub targets: std::collections::BTreeMap<String, TargetTools>,
    /// The operating system of the target, the host's unless a build or a check names
    /// another target (LIB-22).
    pub platform: Platform,
}

/// The C tools `typy.toml` names for one target; each absent one is TyPy's choice (CLI-14).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TargetTools {
    /// The C compiler driver that links, `linker`.
    pub linker: Option<String>,
    /// The archiver of static libraries, `ar`.
    pub archiver: Option<String>,
    /// The tool that edits an object's symbols, `objcopy`.
    pub object_copier: Option<String>,
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
            allocator: None,
            library: false,
            version: None,
            library_paths: Vec::new(),
            libraries: std::collections::BTreeMap::new(),
            targets: std::collections::BTreeMap::new(),
            platform: Platform::host(),
        }
    }
}

#[cfg(test)]
mod tests;
