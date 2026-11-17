//! The kinds of C library `typy build --lib` writes (CLI-09, ADR-0044).

/// A static or a shared C library.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LibraryKind {
    /// An archive, `libNAME.a`, that the host links into its executable.
    Static,
    /// A shared object, `libNAME.so`, that the host links or loads when it runs.
    Shared,
}

impl LibraryKind {
    /// Every kind, as `--lib` names them.
    pub const ALL: [Self; 2] = [Self::Static, Self::Shared];

    /// Returns the kind as `--lib` names it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Static => "static",
            Self::Shared => "shared",
        }
    }

    /// Parses `static` or `shared`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.name() == text)
    }

    /// Returns the file name of the library of module `name`: `libNAME.a` or `libNAME.so`.
    #[must_use]
    pub fn file_name(self, name: &str) -> String {
        match self {
            Self::Static => format!("lib{name}.a"),
            Self::Shared => format!("lib{name}.so"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LibraryKind;

    #[test]
    fn each_kind_parses_from_its_name_and_names_its_file() {
        for kind in LibraryKind::ALL {
            assert_eq!(LibraryKind::parse(kind.name()), Some(kind));
        }
        assert_eq!(LibraryKind::parse("dynamic"), None);
        assert_eq!(LibraryKind::Static.file_name("tools"), "libtools.a");
        assert_eq!(LibraryKind::Shared.file_name("tools"), "libtools.so");
    }
}
