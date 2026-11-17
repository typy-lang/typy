//! Links an object file with the runtime archive by running the C toolchain (A-01).

use std::io;

use camino::Utf8Path;
use typy_codegen::{ObjectFile, Target, TargetTriple};
use typy_diagnostics::Diagnostics;

use crate::artifact::{Artifact, ArtifactKind};
use crate::runtime_archive::RuntimeArchive;
use crate::tool::{cannot_write, output_directory, utf8};
use crate::toolchain::Toolchain;

/// Links object files for one target with that target's runtime archive.
#[derive(Clone, Debug)]
pub struct Linker {
    pub(crate) tools: Toolchain,
    pub(crate) strip_debug: bool,
    pub(crate) archive: RuntimeArchive,
    /// What follows `-l` for each library, such as `mimalloc`, the allocator's (MEM-08),
    /// or `:libm.so.6`, a `ctypes` declaration's (FFI-01).
    pub(crate) libraries: Vec<String>,
    /// The directories the linker looks in first, as `-L` (FFI-04).
    library_paths: Vec<camino::Utf8PathBuf>,
}

impl Linker {
    /// Returns the linker of `triple` with the tools this host builds it with (ADR-0051).
    #[must_use]
    pub fn new(triple: TargetTriple, archive: RuntimeArchive) -> Self {
        Self {
            tools: Toolchain::of(triple, Target::host().triple),
            strip_debug: false,
            archive,
            libraries: Vec::new(),
            library_paths: Vec::new(),
        }
    }

    /// Returns the linker with each tool `typy.toml` names for its target in place of the one
    /// TyPy would run (CLI-14).
    #[must_use]
    pub fn with_tools(
        mut self,
        linker: Option<&str>,
        archiver: Option<&str>,
        object_copier: Option<&str>,
    ) -> Self {
        self.tools = self.tools.with_overrides(linker, archiver, object_copier);
        self
    }

    /// Returns the linker with the debug information left out of the executable, as a
    /// release build leaves it out; the symbols stay, so a backtrace still names functions.
    #[must_use]
    pub const fn with_strip_debug(mut self, strip: bool) -> Self {
        self.strip_debug = strip;
        self
    }

    /// Returns the linker with each of `libraries` linked as `-l` and the library after the
    /// runtime archive; each is a name `typy.toml` or a declaration has checked, passed as
    /// one argument and never to a shell.
    #[must_use]
    pub fn with_libraries(mut self, libraries: Vec<String>) -> Self {
        self.libraries = libraries;
        self
    }

    /// Returns the linker that looks for libraries in each of `paths` first, as `-L`.
    #[must_use]
    pub fn with_library_paths(mut self, paths: Vec<camino::Utf8PathBuf>) -> Self {
        self.library_paths = paths;
        self
    }

    /// Returns the arguments that link `object` into `output`.
    fn arguments(&self, object: &Utf8Path, output: &Utf8Path) -> Vec<String> {
        let mut arguments = Vec::new();
        if self.tools.static_link {
            arguments.push("-static".to_owned());
        }
        arguments.extend(["-o".to_owned(), output.to_string()]);
        arguments.extend(self.link_arguments(object));
        arguments
    }

    /// Returns the arguments after the output that an executable and a shared library share:
    /// the object, the C maths library, the runtime archive, the project's libraries, the
    /// collection of unused sections, and for a release no debug information.
    pub(crate) fn link_arguments(&self, object: &Utf8Path) -> Vec<String> {
        // The C maths library comes before the runtime archive, whose `compiler_builtins`
        // has weak copies of some of its functions that differ in the last bit; a member of
        // an archive is taken only for a symbol still undefined. `--no-as-needed` keeps it
        // although nothing refers to it yet. The second `-lm` serves a static link, where the
        // order of archives matters the other way.
        let mut arguments: Vec<String> = self
            .library_paths
            .iter()
            .map(|path| format!("-L{path}"))
            .collect();
        arguments.extend([
            object.to_string(),
            "-Wl,--push-state,--no-as-needed".to_owned(),
            "-lm".to_owned(),
            "-Wl,--pop-state".to_owned(),
            self.archive.path().to_string(),
        ]);
        arguments.extend(self.libraries.iter().map(|library| format!("-l{library}")));
        arguments.extend(["-lm".to_owned(), "-Wl,--gc-sections".to_owned()]);
        if self.strip_debug {
            arguments.push("-Wl,--strip-debug".to_owned());
        }
        arguments
    }

    /// Links `object` into the executable `output`. The executable is written under a temporary
    /// name in the same directory and renamed when the link succeeds, so a failed link leaves no
    /// file and an existing file is replaced whole (UC-02, flow 5a).
    pub fn link(&self, object: &ObjectFile, output: &Utf8Path) -> Result<Artifact, Diagnostics> {
        let directory = output_directory(output);
        let cannot_write = cannot_write(&directory, "the executable");
        let mut object_file = tempfile::Builder::new()
            .prefix(".typy-")
            .suffix(".o")
            .tempfile_in(&directory)
            .map_err(&cannot_write)?;
        io::Write::write_all(&mut object_file, &object.bytes).map_err(&cannot_write)?;
        // Created with mode 0755 less the umask, the mode of any new executable, which the
        // linker keeps when it writes the file again.
        let temporary = tempfile::Builder::new()
            .prefix(".typy-")
            .permissions(std::os::unix::fs::PermissionsExt::from_mode(0o755))
            .tempfile_in(&directory)
            .map_err(&cannot_write)?
            .into_temp_path();
        let object_path = utf8(object_file.path());
        let temporary_path = utf8(&temporary);
        let arguments = self.arguments(&object_path, &temporary_path);
        self.tools.run(&self.tools.linker, &arguments)?;
        temporary
            .persist(output)
            .map_err(|error| cannot_write(error.error))?;
        Ok(Artifact {
            path: output.to_owned(),
            kind: ArtifactKind::Executable,
        })
    }
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
        assert_eq!(gnu.tools.linker, "cc");
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
        let pooled = Linker::new(TargetTriple::X86_64LinuxGnu, archive(&root))
            .with_libraries(vec!["mimalloc".to_owned()]);
        let arguments = pooled.arguments(Utf8Path::new("a.o"), Utf8Path::new("out"));
        assert_eq!(
            arguments[6..],
            [
                expected_archive.as_str(),
                "-lmimalloc",
                "-lm",
                "-Wl,--gc-sections"
            ]
        );
        let musl = Linker::new(TargetTriple::X86_64LinuxMusl, archive(&root));
        assert_eq!(musl.tools.linker, "musl-gcc");
        assert_eq!(
            musl.arguments(Utf8Path::new("a.o"), Utf8Path::new("out"))[0],
            "-static"
        );
    }

    #[test]
    fn a_missing_toolchain_program_is_named() {
        let (_guard, root) = directory();
        let mut linker = Linker::new(TargetTriple::X86_64LinuxGnu, archive(&root));
        linker.tools.linker = "typy-no-such-linker".to_owned();
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
