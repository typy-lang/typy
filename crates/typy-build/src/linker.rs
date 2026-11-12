//! Links an object file with the runtime archive by running the C toolchain (A-01).

use std::io;
use std::process::Command;

use camino::{Utf8Path, Utf8PathBuf};
use typy_codegen::{ObjectFile, TargetTriple};
use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics, RuleId};

use crate::artifact::{Artifact, ArtifactKind};
use crate::runtime_archive::RuntimeArchive;

/// Links object files for one target with that target's runtime archive.
#[derive(Clone, Debug)]
pub struct Linker {
    program: String,
    static_link: bool,
    strip_debug: bool,
    archive: RuntimeArchive,
}

fn link_failed(message: String, help: String) -> Diagnostics {
    Diagnostics::from(
        Diagnostic::without_location(DiagnosticCode::error(702), message)
            .with_rule(RuleId::A_01)
            .with_help(help),
    )
}

impl Linker {
    /// Returns the linker of `triple`: `cc` for glibc, and `musl-gcc` with `-static` for musl.
    #[must_use]
    pub fn new(triple: TargetTriple, archive: RuntimeArchive) -> Self {
        let (program, static_link) = match triple {
            TargetTriple::X86_64LinuxGnu => ("cc", false),
            TargetTriple::X86_64LinuxMusl => ("musl-gcc", true),
        };
        Self {
            program: program.to_owned(),
            static_link,
            strip_debug: false,
            archive,
        }
    }

    /// Returns the linker with the debug information left out of the executable, as a
    /// release build leaves it out; the symbols stay, so a backtrace still names functions.
    #[must_use]
    pub const fn with_strip_debug(mut self, strip: bool) -> Self {
        self.strip_debug = strip;
        self
    }

    /// Returns the arguments that link `object` into `output`.
    fn arguments(&self, object: &Utf8Path, output: &Utf8Path) -> Vec<String> {
        let mut arguments = Vec::new();
        if self.static_link {
            arguments.push("-static".to_owned());
        }
        // The C maths library comes before the runtime archive, whose `compiler_builtins`
        // has weak copies of some of its functions that differ in the last bit; a member of
        // an archive is taken only for a symbol still undefined. `--no-as-needed` keeps it
        // although nothing refers to it yet. The second `-lm` serves a static link, where the
        // order of archives matters the other way.
        arguments.extend([
            "-o".to_owned(),
            output.to_string(),
            object.to_string(),
            "-Wl,--push-state,--no-as-needed".to_owned(),
            "-lm".to_owned(),
            "-Wl,--pop-state".to_owned(),
            self.archive.path().to_string(),
            "-lm".to_owned(),
            "-Wl,--gc-sections".to_owned(),
        ]);
        if self.strip_debug {
            arguments.push("-Wl,--strip-debug".to_owned());
        }
        arguments
    }

    /// Links `object` into the executable `output`. The executable is written under a temporary
    /// name in the same directory and renamed when the link succeeds, so a failed link leaves no
    /// file and an existing file is replaced whole (UC-02, flow 5a).
    pub fn link(&self, object: &ObjectFile, output: &Utf8Path) -> Result<Artifact, Diagnostics> {
        let directory = match output.parent() {
            Some(parent) if !parent.as_str().is_empty() => parent.to_owned(),
            _ => Utf8PathBuf::from("."),
        };
        let cannot_write = |error: io::Error| {
            Diagnostics::from(
                Diagnostic::without_location(
                    DiagnosticCode::error(106),
                    format!("cannot write the executable in {directory}: {error}"),
                )
                .with_help("check that the directory exists and is writable"),
            )
        };
        let mut object_file = tempfile::Builder::new()
            .prefix(".typy-")
            .suffix(".o")
            .tempfile_in(&directory)
            .map_err(cannot_write)?;
        io::Write::write_all(&mut object_file, &object.bytes).map_err(cannot_write)?;
        // Created with mode 0755 less the umask, the mode of any new executable, which the
        // linker keeps when it writes the file again.
        let temporary = tempfile::Builder::new()
            .prefix(".typy-")
            .permissions(std::os::unix::fs::PermissionsExt::from_mode(0o755))
            .tempfile_in(&directory)
            .map_err(cannot_write)?
            .into_temp_path();
        let object_path = utf8(object_file.path());
        let temporary_path = utf8(&temporary);
        let arguments = self.arguments(&object_path, &temporary_path);
        let result = Command::new(&self.program).args(&arguments).output();
        let run = match result {
            Ok(run) => run,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err(Diagnostics::from(
                    Diagnostic::without_location(
                        DiagnosticCode::error(701),
                        format!("the C toolchain program `{}` was not found", self.program),
                    )
                    .with_rule(RuleId::A_01)
                    .with_help(format!(
                        "install `{}`, which TyPy runs to link the program",
                        self.program
                    )),
                ));
            }
            Err(error) => {
                return Err(link_failed(
                    format!("cannot run `{}`: {error}", self.program),
                    format!("the command was `{} {}`", self.program, arguments.join(" ")),
                ));
            }
        };
        if !run.status.success() {
            return Err(link_failed(
                format!("`{}` failed with {}", self.program, run.status),
                format!(
                    "the command was `{} {}`; it wrote:\n{}",
                    self.program,
                    arguments.join(" "),
                    String::from_utf8_lossy(&run.stderr).trim_end()
                ),
            ));
        }
        temporary
            .persist(output)
            .map_err(|error| cannot_write(error.error))?;
        Ok(Artifact {
            path: output.to_owned(),
            kind: ArtifactKind::Executable,
        })
    }
}

/// Temporary paths are made from a UTF-8 directory and an ASCII name, so they are UTF-8.
fn utf8(path: &std::path::Path) -> Utf8PathBuf {
    Utf8PathBuf::from(path.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::Linker;
    use crate::runtime_archive::RuntimeArchive;
    use camino::{Utf8Path, Utf8PathBuf};
    use typy_codegen::{ObjectFile, Target, TargetTriple};

    fn archive(directory: &Utf8Path) -> RuntimeArchive {
        let path = directory.join("libtypy_runtime.a");
        std::fs::write(
            &path,
            format!("typy-runtime-version:{}\n", env!("CARGO_PKG_VERSION")),
        )
        .unwrap();
        RuntimeArchive::open(&path).unwrap()
    }

    fn directory() -> (tempfile::TempDir, Utf8PathBuf) {
        let directory = tempfile::tempdir().unwrap();
        let path = Utf8PathBuf::from_path_buf(directory.path().to_owned()).unwrap();
        (directory, path)
    }

    fn object() -> ObjectFile {
        ObjectFile {
            target: Target::host(),
            bytes: b"not an object".to_vec(),
        }
    }

    #[test]
    fn each_target_links_with_its_toolchain() {
        let (_guard, root) = directory();
        let gnu = Linker::new(TargetTriple::X86_64LinuxGnu, archive(&root));
        let arguments = gnu.arguments(Utf8Path::new("a.o"), Utf8Path::new("out"));
        let expected_archive = root.join("libtypy_runtime.a").to_string();
        assert_eq!(gnu.program, "cc");
        assert_eq!(
            arguments,
            [
                "-o",
                "out",
                "a.o",
                "-Wl,--push-state,--no-as-needed",
                "-lm",
                "-Wl,--pop-state",
                expected_archive.as_str(),
                "-lm",
                "-Wl,--gc-sections"
            ]
        );
        let stripped =
            Linker::new(TargetTriple::X86_64LinuxGnu, archive(&root)).with_strip_debug(true);
        let arguments = stripped.arguments(Utf8Path::new("a.o"), Utf8Path::new("out"));
        assert_eq!(
            arguments.last().map(String::as_str),
            Some("-Wl,--strip-debug")
        );
        let musl = Linker::new(TargetTriple::X86_64LinuxMusl, archive(&root));
        assert_eq!(musl.program, "musl-gcc");
        assert_eq!(
            musl.arguments(Utf8Path::new("a.o"), Utf8Path::new("out"))[0],
            "-static"
        );
    }

    #[test]
    fn a_missing_toolchain_program_is_named() {
        let (_guard, root) = directory();
        let mut linker = Linker::new(TargetTriple::X86_64LinuxGnu, archive(&root));
        linker.program = "typy-no-such-linker".to_owned();
        let diagnostics = linker.link(&object(), &root.join("app")).unwrap_err();
        let diagnostic = diagnostics.iter().next().unwrap();
        assert_eq!(diagnostic.code().to_string(), "E0701");
        assert!(diagnostic.message().contains("`typy-no-such-linker`"));
    }

    #[test]
    fn a_failed_link_leaves_no_file_behind() {
        let (_guard, root) = directory();
        let linker = Linker::new(TargetTriple::X86_64LinuxGnu, archive(&root));
        let output = root.join("app");
        let diagnostics = linker.link(&object(), &output).unwrap_err();
        let diagnostic = diagnostics.iter().next().unwrap();
        assert_eq!(diagnostic.code().to_string(), "E0702");
        assert!(
            diagnostic
                .help()
                .unwrap()
                .contains("the command was `cc -o")
        );
        let left: Vec<String> = std::fs::read_dir(&root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(left, ["libtypy_runtime.a"]);
    }

    #[test]
    fn an_output_in_a_missing_directory_is_refused() {
        let (_guard, root) = directory();
        let linker = Linker::new(TargetTriple::X86_64LinuxGnu, archive(&root));
        let diagnostics = linker
            .link(&object(), &root.join("absent/app"))
            .unwrap_err();
        let diagnostic = diagnostics.iter().next().unwrap();
        assert_eq!(diagnostic.code().to_string(), "E0106");
        assert!(
            diagnostic
                .message()
                .starts_with("cannot write the executable in")
        );
    }
}
