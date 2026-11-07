//! The targets TyPy generates code for (section 2.3 of the specification).

use std::fmt;

/// A supported target triple. 0.1 supports Linux on x86-64, with glibc or musl.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TargetTriple {
    /// `x86_64-unknown-linux-gnu`.
    X86_64LinuxGnu,
    /// `x86_64-unknown-linux-musl`, for static executables.
    X86_64LinuxMusl,
}

impl TargetTriple {
    /// Every supported triple.
    pub const ALL: [Self; 2] = [Self::X86_64LinuxGnu, Self::X86_64LinuxMusl];

    /// Returns the triple as written, such as `x86_64-unknown-linux-gnu`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::X86_64LinuxGnu => "x86_64-unknown-linux-gnu",
            Self::X86_64LinuxMusl => "x86_64-unknown-linux-musl",
        }
    }

    /// Parses a supported triple; any other text gives `None`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|triple| triple.as_str() == text)
    }

    /// Returns the triple LLVM is given, which names the C library as `gnu` or `musl`.
    #[must_use]
    pub(crate) const fn llvm_triple(self) -> &'static str {
        match self {
            Self::X86_64LinuxGnu => "x86_64-unknown-linux-gnu",
            Self::X86_64LinuxMusl => "x86_64-unknown-linux-musl",
        }
    }
}

impl fmt::Display for TargetTriple {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// The machine a build generates code for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Target {
    /// Its triple.
    pub triple: TargetTriple,
}

impl Target {
    /// Returns the target `typy` itself runs on, which builds use unless `--target` is given.
    #[must_use]
    pub fn host() -> Self {
        let triple = if cfg!(target_env = "musl") {
            TargetTriple::X86_64LinuxMusl
        } else {
            TargetTriple::X86_64LinuxGnu
        };
        Self { triple }
    }
}

#[cfg(test)]
mod tests {
    use super::{Target, TargetTriple};

    #[test]
    fn every_triple_parses_from_its_text() {
        for triple in TargetTriple::ALL {
            assert_eq!(TargetTriple::parse(triple.as_str()), Some(triple));
            assert_eq!(triple.to_string(), triple.llvm_triple());
        }
    }

    #[test]
    fn unsupported_triples_are_refused() {
        for text in [
            "",
            "aarch64-unknown-linux-gnu",
            "x86_64-pc-windows-msvc",
            "x86_64",
        ] {
            assert_eq!(TargetTriple::parse(text), None, "{text:?}");
        }
    }

    #[test]
    fn the_host_is_a_supported_target() {
        assert!(TargetTriple::ALL.contains(&Target::host().triple));
    }
}
