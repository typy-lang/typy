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
    /// The archiver of static libraries, such as `ar`.
    pub(crate) archiver: String,
    /// The tool that edits an object's symbols, such as `objcopy`.
    pub(crate) object_copier: String,
    /// Whether programs link statically, as they do for musl.
    pub(crate) static_link: bool,
}

impl Toolchain {
    /// Returns the tools that build for `target` on a `host`: `cc`, `ar` and `objcopy` for the
    /// host's architecture, with `musl-gcc` as the linker for musl, and for another
    /// architecture the tools named after the target's GNU triple, as Linux distributions
    /// package cross toolchains.
    #[must_use]
    pub fn of(target: TargetTriple, host: TargetTriple) -> Self {
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

    /// Runs one of the tools with `arguments`; a tool that is not installed is E0701, which
    /// names the target it builds for (UC-08, flow 3a).
    pub(crate) fn run(&self, program: &str, arguments: &[String]) -> Result<(), Diagnostics> {
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
        }
    }
}

#[cfg(test)]
mod tests {
    use typy_codegen::TargetTriple;

    use super::Toolchain;

    #[test]
    fn each_target_gets_the_tools_its_host_builds_it_with() {
        let gnu = TargetTriple::X86_64LinuxGnu;
        let musl = TargetTriple::X86_64LinuxMusl;
        let arm = TargetTriple::Aarch64LinuxGnu;
        let cases = [
            (gnu, gnu, ["cc", "ar", "objcopy"], false),
            (musl, gnu, ["musl-gcc", "ar", "objcopy"], true),
            (musl, musl, ["cc", "ar", "objcopy"], true),
            (
                arm,
                gnu,
                [
                    "aarch64-linux-gnu-gcc",
                    "aarch64-linux-gnu-ar",
                    "aarch64-linux-gnu-objcopy",
                ],
                false,
            ),
            (arm, arm, ["cc", "ar", "objcopy"], false),
            (
                gnu,
                arm,
                [
                    "x86_64-linux-gnu-gcc",
                    "x86_64-linux-gnu-ar",
                    "x86_64-linux-gnu-objcopy",
                ],
                false,
            ),
            (
                musl,
                arm,
                [
                    "x86_64-linux-musl-gcc",
                    "x86_64-linux-musl-ar",
                    "x86_64-linux-musl-objcopy",
                ],
                true,
            ),
        ];
        for (target, host, [linker, archiver, object_copier], static_link) in cases {
            let tools = Toolchain::of(target, host);
            assert_eq!(
                (
                    tools.linker.as_str(),
                    tools.archiver.as_str(),
                    tools.object_copier.as_str(),
                    tools.static_link
                ),
                (linker, archiver, object_copier, static_link),
                "{target} on {host}"
            );
        }
    }

    #[test]
    fn typy_toml_names_tools_in_place_of_the_defaults() {
        let tools = Toolchain::of(TargetTriple::Aarch64LinuxGnu, TargetTriple::X86_64LinuxGnu)
            .with_overrides(Some("/opt/cross/bin/gcc"), None, Some("llvm-objcopy"));
        assert_eq!(
            (
                tools.linker.as_str(),
                tools.archiver.as_str(),
                tools.object_copier.as_str()
            ),
            ("/opt/cross/bin/gcc", "aarch64-linux-gnu-ar", "llvm-objcopy")
        );
    }

    #[test]
    fn typy_toml_knows_the_targets_the_code_generator_supports() {
        let triples: Vec<&str> = TargetTriple::ALL
            .iter()
            .map(|triple| triple.as_str())
            .collect();
        assert_eq!(triples, typy_source::TARGET_TRIPLES);
    }
}
