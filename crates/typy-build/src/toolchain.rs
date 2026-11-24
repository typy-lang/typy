//! The C tools that link for a target: the linker driver, the archiver and the object copier
//! (CLI-14, ADR-0051).

use typy_codegen::TargetTriple;
use typy_diagnostics::Diagnostics;

use crate::tool::run_tool;

/// The programs that link objects for one target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Toolchain {
    /// The target they build for.
    target: TargetTriple,
    /// The C compiler driver that links, such as `cc` or `aarch64-linux-gnu-gcc`.
    pub(crate) linker: String,
    /// The archiver of static libraries, such as `ar`, or `libtool` on macOS.
    pub(crate) archiver: String,
    /// The tool that edits an object's symbols, such as `objcopy`; empty on macOS, whose
    /// linker keeps only the exports itself (ADR-0056).
    pub(crate) object_copier: String,
    /// Whether programs link statically, as they do for musl.
    pub(crate) static_link: bool,
    /// Whether each command is written to standard error before it runs (CLI-23).
    verbose: bool,
}

impl Toolchain {
    /// Returns the tools that build for `target` on a `host`: `cc`, `ar` and `objcopy` for the
    /// host's architecture, with `musl-gcc` as the linker for musl, and for another
    /// architecture the tools named after the target's GNU triple, as Linux distributions
    /// package cross toolchains. macOS, which only a macOS host builds for, links with `cc`
    /// and archives with `libtool` (ADR-0055).
    #[must_use]
    pub fn of(target: TargetTriple, host: TargetTriple) -> Self {
        if target.is_macos() {
            return Self::named(target, ["cc", "libtool", ""], false);
        }
        let static_link = target.is_musl();
        if target.architecture() == host.architecture() {
            let linker = if static_link && !host.is_musl() {
                "musl-gcc"
            } else {
                "cc"
            };
            return Self::named(target, [linker, "ar", "objcopy"], static_link);
        }
        let prefix = target.gnu_triple();
        Self::named(
            target,
            [
                &format!("{prefix}-gcc"),
                &format!("{prefix}-ar"),
                &format!("{prefix}-objcopy"),
            ],
            static_link,
        )
    }

    /// Returns the tools with each one `typy.toml` names for the target in place of TyPy's
    /// choice (CLI-14).
    #[must_use]
    pub fn with_overrides(
        mut self,
        linker: Option<&str>,
        archiver: Option<&str>,
        object_copier: Option<&str>,
    ) -> Self {
        for (slot, name) in [
            (&mut self.linker, linker),
            (&mut self.archiver, archiver),
            (&mut self.object_copier, object_copier),
        ] {
            if let Some(name) = name {
                name.clone_into(slot);
            }
        }
        self
    }

    /// Returns the tools, writing each command to standard error before it runs when
    /// `verbose`, as `typy -v` asks (CLI-23).
    #[must_use]
    pub const fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Returns the target the tools build for.
    pub(crate) const fn target(&self) -> TargetTriple {
        self.target
    }

    /// Runs one of the tools with `arguments`; a tool that is not installed is E0701, which
    /// names the target it builds for (UC-08, flow 3a).
    pub(crate) fn run(&self, program: &str, arguments: &[String]) -> Result<(), Diagnostics> {
        if self.verbose {
            eprintln!("{}", command_line(program, arguments));
        }
        run_tool(program, arguments, self.target)
    }

    fn named(
        target: TargetTriple,
        [linker, archiver, object_copier]: [&str; 3],
        static_link: bool,
    ) -> Self {
        Self {
            target,
            linker: linker.to_owned(),
            archiver: archiver.to_owned(),
            object_copier: object_copier.to_owned(),
            static_link,
            verbose: false,
        }
    }
}

/// Returns a command as a shell would read it, each word that holds anything but letters,
/// digits and `-_./=:,+@%` in single quotes, so a user can run it again.
#[must_use]
pub fn command_line(program: &str, arguments: &[String]) -> String {
    std::iter::once(program)
        .chain(arguments.iter().map(String::as_str))
        .map(|word| {
            let plain = !word.is_empty()
                && word.chars().all(|character| {
                    character.is_ascii_alphanumeric() || "-_./=:,+@%".contains(character)
                });
            if plain {
                word.to_owned()
            } else {
                format!("'{}'", word.replace('\'', "'\\''"))
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests;
