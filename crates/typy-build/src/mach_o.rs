//! Links for macOS with Apple's linker through `cc` (CLI-15, CLI-16, ADR-0056): an executable
//! with its debug information in a `.dSYM` beside it, a static library whose only global
//! symbols are its exports, and a shared library with its install name and versions.
//!
//! Each output is linked in a work directory beside it, then renamed into place, since nothing
//! may change a binary after the linker signed it. The linker signs a binary ad hoc under its
//! file name and records each object's path and time in its debug map, so the bytes depend on
//! neither (NFR-06): an executable is linked as `program`, whichever name `-o` gives it, a
//! library under the name its layout gives, the work directory is named by its canonical
//! path, which `-oso_prefix` takes off, and the object is dated 1970.

use std::fmt::Write as _;

use camino::{Utf8Path, Utf8PathBuf};
use typy_codegen::ObjectFile;
use typy_diagnostics::Diagnostics;

use crate::artifact::{Artifact, ArtifactKind};
use crate::library_layout::LibraryLayout;
use crate::linker::Linker;
use crate::tool::{cannot_write, output_directory, utf8};

/// The oldest macOS a program runs on, the first on Apple silicon (ADR-0055).
const DEPLOYMENT: [&str; 3] = ["-arch", "arm64", "-mmacosx-version-min=11.0"];
/// The tool that copies the debug information of the objects into a `.dSYM`.
const DSYMUTIL: &str = "dsymutil";

/// The name an executable is linked under, and so signed under.
const EXECUTABLE: &str = "program";

/// A directory beside an output, removed when it is dropped, holding the program's object as
/// `program.o`.
struct Work {
    _directory: tempfile::TempDir,
    path: Utf8PathBuf,
}

impl Work {
    fn beside(output: &Utf8Path, object: &ObjectFile, what: &str) -> Result<Self, Diagnostics> {
        let parent = output_directory(output);
        let cannot_write = cannot_write(&parent, what);
        let directory = tempfile::Builder::new()
            .prefix(".typy-")
            .tempdir_in(&parent)
            .map_err(&cannot_write)?;
        // The linker records the object by its real path, without `.` or a link such as
        // macOS's `/var`, so the prefix it takes off is that path too.
        let path = utf8(&std::fs::canonicalize(directory.path()).map_err(&cannot_write)?);
        let object_path = path.join("program.o");
        std::fs::write(&object_path, &object.bytes).map_err(&cannot_write)?;
        std::fs::File::options()
            .write(true)
            .open(&object_path)
            .and_then(|file| file.set_modified(std::time::SystemTime::UNIX_EPOCH))
            .map_err(&cannot_write)?;
        Ok(Self {
            _directory: directory,
            path,
        })
    }

    fn object(&self) -> Utf8PathBuf {
        self.path.join("program.o")
    }

    /// Returns the path in the work directory of a file named as `output` is.
    fn named_as(&self, output: &Utf8Path) -> Utf8PathBuf {
        self.path.join(output.file_name().unwrap_or("output"))
    }
}

impl Linker {
    /// Returns the arguments after the output that an executable and a shared library share:
    /// the target, the library paths, the object, the C maths library, the runtime archive,
    /// the project's libraries, the removal of unused code, and for a release no debug map;
    /// a debug build records the object's path from `work`, so the bytes do not depend on
    /// where the work directory is.
    pub(crate) fn mach_o_arguments(&self, object: &Utf8Path, work: &Utf8Path) -> Vec<String> {
        let mut arguments: Vec<String> = DEPLOYMENT.map(str::to_owned).to_vec();
        arguments.extend(self.library_paths.iter().map(|path| format!("-L{path}")));
        // `libm` is part of `libSystem`; naming it before the runtime archive keeps the
        // system's functions ahead of the weak copies in its `compiler_builtins`, as on Linux.
        arguments.extend([
            object.to_string(),
            "-lm".to_owned(),
            self.archive.path().to_string(),
        ]);
        arguments.extend(self.libraries.iter().cloned());
        arguments.push("-Wl,-dead_strip".to_owned());
        if self.strip_debug {
            arguments.push("-Wl,-S".to_owned());
        } else {
            arguments.push(format!("-Wl,-oso_prefix,{work}/"));
        }
        arguments
    }

    /// Links `object` into the executable `output` and, for a debug build, writes its debug
    /// information into `output.dSYM`, which a debugger finds beside it. `dsymutil` reads the
    /// executable at its own name, which the `.dSYM` names its debug information after, while
    /// the object it reads it from is still in the work directory.
    pub(crate) fn link_mach_o(
        &self,
        object: &ObjectFile,
        output: &Utf8Path,
    ) -> Result<Artifact, Diagnostics> {
        let work = Work::beside(output, object, "the executable")?;
        let parent = output_directory(output);
        let cannot_write = cannot_write(&parent, "the executable");
        let linked = work.path.join(EXECUTABLE);
        let mut arguments = vec!["-o".to_owned(), linked.to_string()];
        arguments.extend(self.mach_o_arguments(&work.object(), &work.path));
        self.tools.run(&self.tools.linker, &arguments)?;
        std::fs::rename(&linked, output).map_err(&cannot_write)?;
        if !self.strip_debug {
            let symbols = work.path.join(format!("{EXECUTABLE}.dSYM"));
            self.tools.run(
                DSYMUTIL,
                &[
                    format!("--oso-prepend-path={}", work.path),
                    "-o".to_owned(),
                    symbols.to_string(),
                    output.to_string(),
                ],
            )?;
            let placed = Utf8PathBuf::from(format!("{output}.dSYM"));
            if placed.is_dir() {
                std::fs::remove_dir_all(&placed).map_err(&cannot_write)?;
            }
            std::fs::rename(&symbols, &placed).map_err(&cannot_write)?;
        }
        Ok(Artifact {
            path: output.to_owned(),
            kind: ArtifactKind::Executable,
        })
    }

    /// Links the program object and the runtime archive into one object with `ld -r`, whose
    /// global symbols but the exports become local, and archives it with `libtool`, so two
    /// TyPy libraries link into one program.
    pub(crate) fn mach_o_static_library(
        &self,
        object: &ObjectFile,
        layout: &LibraryLayout,
        exports: &[String],
    ) -> Result<(), Diagnostics> {
        let work = Work::beside(&layout.path, object, "the library")?;
        let parent = output_directory(&layout.path);
        let cannot_write = cannot_write(&parent, "the library");
        let listed = exports_list(&work.path, exports).map_err(&cannot_write)?;
        let combined = work.path.join("library.o");
        let mut arguments = DEPLOYMENT.map(str::to_owned).to_vec();
        arguments.extend([
            "-r".to_owned(),
            "-nostdlib".to_owned(),
            format!("-Wl,-exported_symbols_list,{listed}"),
            "-o".to_owned(),
            combined.to_string(),
            work.object().to_string(),
            self.archive.path().to_string(),
        ]);
        if self.strip_debug {
            arguments.push("-Wl,-S".to_owned());
        }
        self.tools.run(&self.tools.linker, &arguments)?;
        let archive = work.named_as(&layout.path);
        self.tools.run(
            &self.tools.archiver,
            &[
                "-static".to_owned(),
                "-D".to_owned(),
                "-o".to_owned(),
                archive.to_string(),
                combined.to_string(),
            ],
        )?;
        std::fs::rename(&archive, &layout.path).map_err(&cannot_write)?;
        Ok(())
    }

    /// Links the program object and the runtime archive into a shared library that exports
    /// only the exports, with its install name and, for a release, its versions.
    pub(crate) fn mach_o_shared_library(
        &self,
        object: &ObjectFile,
        layout: &LibraryLayout,
        exports: &[String],
    ) -> Result<(), Diagnostics> {
        let work = Work::beside(&layout.path, object, "the library")?;
        let parent = output_directory(&layout.path);
        let cannot_write = cannot_write(&parent, "the library");
        let listed = exports_list(&work.path, exports).map_err(&cannot_write)?;
        let library = work.named_as(&layout.path);
        let install_name = layout
            .soname
            .clone()
            .unwrap_or_else(|| layout.path.file_name().unwrap_or_default().to_owned());
        let mut arguments = vec![
            "-dynamiclib".to_owned(),
            "-o".to_owned(),
            library.to_string(),
            format!("-Wl,-install_name,{install_name}"),
        ];
        if let Some((compatibility, current)) = &layout.versions {
            arguments.push(format!("-Wl,-compatibility_version,{compatibility}"));
            arguments.push(format!("-Wl,-current_version,{current}"));
        }
        arguments.push(format!("-Wl,-exported_symbols_list,{listed}"));
        arguments.extend(self.mach_o_arguments(&work.object(), &work.path));
        self.tools.run(&self.tools.linker, &arguments)?;
        std::fs::rename(&library, &layout.path).map_err(&cannot_write)?;
        Ok(())
    }
}

/// Writes the list of exports Apple's linker reads, each name with the underscore Mach-O
/// gives a C symbol, and returns its path.
fn exports_list(work: &Utf8Path, exports: &[String]) -> std::io::Result<Utf8PathBuf> {
    let path = work.join("exports.txt");
    let listed = exports.iter().fold(String::new(), |mut text, name| {
        let _ = writeln!(text, "_{name}");
        text
    });
    std::fs::write(&path, listed)?;
    Ok(path)
}

#[cfg(test)]
mod tests;
