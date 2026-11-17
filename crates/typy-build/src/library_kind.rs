//! The kinds of C library `typy build --lib` writes (CLI-09, ADR-0044).

/// A static or a shared C library.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LibraryKind {
    /// An archive that the host links into its executable.
    Static,
    /// A shared object that the host links or loads when it runs.
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
}

#[cfg(test)]
mod tests {
    use super::LibraryKind;

    #[test]
    fn each_kind_parses_from_its_name() {
        for kind in LibraryKind::ALL {
            assert_eq!(LibraryKind::parse(kind.name()), Some(kind));
        }
        assert_eq!(LibraryKind::parse("dynamic"), None);
    }
}
