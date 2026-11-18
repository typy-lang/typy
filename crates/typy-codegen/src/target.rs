//! The targets TyPy generates code for (section 2.3 of the specification).

use std::fmt;

/// A supported target triple: Linux on x86-64 with glibc or musl from 0.1, Linux on AArch64
/// with glibc from 0.6 (ADR-0051), and macOS on Apple silicon from 0.7 (ADR-0055).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TargetTriple {
    /// `x86_64-unknown-linux-gnu`.
    X86_64LinuxGnu,
    /// `x86_64-unknown-linux-musl`, for static executables.
    X86_64LinuxMusl,
    /// `aarch64-unknown-linux-gnu`.
    Aarch64LinuxGnu,
    /// `aarch64-apple-darwin`, macOS 11 or later on Apple silicon.
    Aarch64AppleDarwin,
}

impl TargetTriple {
    /// Every supported triple.
    pub const ALL: [Self; 4] = [
        Self::X86_64LinuxGnu,
        Self::X86_64LinuxMusl,
        Self::Aarch64LinuxGnu,
        Self::Aarch64AppleDarwin,
    ];

    /// Returns the triple as written, such as `x86_64-unknown-linux-gnu`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::X86_64LinuxGnu => "x86_64-unknown-linux-gnu",
            Self::X86_64LinuxMusl => "x86_64-unknown-linux-musl",
            Self::Aarch64LinuxGnu => "aarch64-unknown-linux-gnu",
            Self::Aarch64AppleDarwin => "aarch64-apple-darwin",
        }
    }

    /// Parses a supported triple; any other text gives `None`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|triple| triple.as_str() == text)
    }

    /// Returns the triple LLVM is given, which names the C library as `gnu` or `musl`, and for
    /// macOS the oldest release a program runs on, 11.0, the first on Apple silicon.
    #[must_use]
    pub(crate) const fn llvm_triple(self) -> &'static str {
        match self {
            Self::X86_64LinuxGnu => "x86_64-unknown-linux-gnu",
            Self::X86_64LinuxMusl => "x86_64-unknown-linux-musl",
            Self::Aarch64LinuxGnu => "aarch64-unknown-linux-gnu",
            Self::Aarch64AppleDarwin => "arm64-apple-macosx11.0.0",
        }
    }

    /// Returns the architecture, as the triple names it.
    #[must_use]
    pub const fn architecture(self) -> &'static str {
        match self {
            Self::X86_64LinuxGnu | Self::X86_64LinuxMusl => "x86_64",
            Self::Aarch64LinuxGnu | Self::Aarch64AppleDarwin => "aarch64",
        }
    }

    /// Returns the triple GNU tools are named after, such as `aarch64-linux-gnu` in
    /// `aarch64-linux-gnu-gcc`; macOS, which only its own host builds for, has none and gives
    /// its triple.
    #[must_use]
    pub const fn gnu_triple(self) -> &'static str {
        match self {
            Self::X86_64LinuxGnu => "x86_64-linux-gnu",
            Self::X86_64LinuxMusl => "x86_64-linux-musl",
            Self::Aarch64LinuxGnu => "aarch64-linux-gnu",
            Self::Aarch64AppleDarwin => "aarch64-apple-darwin",
        }
    }

    /// Returns whether the triple is macOS's, whose programs are Mach-O and link with Apple's
    /// linker (ADR-0056).
    #[must_use]
    pub const fn is_macos(self) -> bool {
        matches!(self, Self::Aarch64AppleDarwin)
    }

    /// Returns whether a `host` builds for the triple: a Linux host for every Linux triple, and
    /// a macOS host for macOS alone, since linking for macOS needs Apple's SDK (ADR-0055).
    #[must_use]
    pub const fn builds_on(self, host: Self) -> bool {
        self.is_macos() == host.is_macos()
    }

    /// Returns whether the triple's C library is musl, whose programs TyPy links statically.
    #[must_use]
    pub const fn is_musl(self) -> bool {
        matches!(self, Self::X86_64LinuxMusl)
    }

    /// Returns the baseline CPU of the triple's architecture, so every machine of the target runs
    /// the output and the bytes do not depend on the machine that builds them (NFR-06).
    #[must_use]
    pub(crate) const fn cpu(self) -> &'static str {
        match self {
            Self::X86_64LinuxGnu | Self::X86_64LinuxMusl => "x86-64",
            Self::Aarch64LinuxGnu => "generic",
            Self::Aarch64AppleDarwin => "apple-m1",
        }
    }

    /// Returns whether a C integer narrower than 32 bits, or a `_Bool`, reaches the code that
    /// reads it extended to 32 bits, so clang marks it `signext` or `zeroext`: on x86-64 and
    /// Apple's arm64, but not under AAPCS64 on Linux, where the reader extends it (ADR-0058).
    #[must_use]
    pub(crate) const fn extends_narrow_integers(self) -> bool {
        !matches!(self, Self::Aarch64LinuxGnu)
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
        let triple = if cfg!(target_os = "macos") {
            TargetTriple::Aarch64AppleDarwin
        } else if cfg!(target_arch = "aarch64") {
            TargetTriple::Aarch64LinuxGnu
        } else if cfg!(target_env = "musl") {
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
            assert_eq!(triple.to_string(), triple.as_str());
        }
    }

    #[test]
    fn llvm_is_given_linux_triples_as_written_and_macos_with_its_oldest_release() {
        for triple in TargetTriple::ALL {
            let expected = if triple.is_macos() {
                "arm64-apple-macosx11.0.0"
            } else {
                triple.as_str()
            };
            assert_eq!(triple.llvm_triple(), expected);
        }
    }

    #[test]
    fn unsupported_triples_are_refused() {
        for text in [
            "",
            "aarch64-unknown-linux-musl",
            "x86_64-apple-darwin",
            "arm64-apple-macosx11.0.0",
            "x86_64-pc-windows-msvc",
            "x86_64",
        ] {
            assert_eq!(TargetTriple::parse(text), None, "{text:?}");
        }
    }

    #[test]
    fn narrow_integers_are_extended_on_x86_64_and_not_under_aapcs64() {
        assert!(TargetTriple::X86_64LinuxGnu.extends_narrow_integers());
        assert!(TargetTriple::X86_64LinuxMusl.extends_narrow_integers());
        assert!(!TargetTriple::Aarch64LinuxGnu.extends_narrow_integers());
        assert!(TargetTriple::Aarch64AppleDarwin.extends_narrow_integers());
    }

    #[test]
    fn a_linux_host_builds_for_linux_and_a_macos_host_for_macos() {
        let linux = [
            TargetTriple::X86_64LinuxGnu,
            TargetTriple::X86_64LinuxMusl,
            TargetTriple::Aarch64LinuxGnu,
        ];
        for host in linux {
            for target in linux {
                assert!(target.builds_on(host), "{target} on {host}");
            }
            assert!(!TargetTriple::Aarch64AppleDarwin.builds_on(host));
            assert!(!host.builds_on(TargetTriple::Aarch64AppleDarwin));
        }
        let macos = TargetTriple::Aarch64AppleDarwin;
        assert!(macos.builds_on(macos));
    }

    #[test]
    fn the_host_is_a_supported_target() {
        assert!(TargetTriple::ALL.contains(&Target::host().triple));
    }
}
