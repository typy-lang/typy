//! Links an object file with the runtime archive into a static or a shared C library, whose
//! only global symbols are its exports (CLI-09, ADR-0044).

use std::fmt::Write as _;
use std::io;

use camino::{Utf8Path, Utf8PathBuf};
use typy_codegen::ObjectFile;
use typy_diagnostics::Diagnostics;

use crate::artifact::{Artifact, ArtifactKind};
use crate::library_kind::LibraryKind;
use crate::library_layout::LibraryLayout;
use crate::linker::Linker;
use crate::tool::{cannot_write, output_directory, run_tool, utf8};

impl Linker {
    /// Links `object` into the library of `kind` that `layout` names, which exports
    /// `exports`, then makes the layout's links to it. The work is done in a temporary
    /// directory beside the library, and the library is renamed into place when every step
    /// succeeds, so a failed build leaves no file.
    pub fn link_library(
        &self,
        object: &ObjectFile,
        layout: &LibraryLayout,
        kind: LibraryKind,
        exports: &[String],
    ) -> Result<Artifact, Diagnostics> {
        let output = layout.path.as_path();
        let directory = output_directory(output);
        let cannot_write = cannot_write(&directory, "the library");
        let work = tempfile::Builder::new()
            .prefix(".typy-")
            .tempdir_in(&directory)
            .map_err(&cannot_write)?;
        let work_path = utf8(work.path());
        let object_path = work_path.join("program.o");
        std::fs::write(&object_path, &object.bytes).map_err(&cannot_write)?;
        let built = match kind {
            LibraryKind::Static => self.static_library(&work_path, &object_path, exports),
            LibraryKind::Shared => {
                self.shared_library(&work_path, &object_path, exports, layout.soname.as_deref())
            }
        }
        .map_err(|error| match error {
            Step::Tool(diagnostics) => diagnostics,
            Step::Write(error) => cannot_write(error),
        })?;
        std::fs::rename(&built, output).map_err(&cannot_write)?;
        for link in &layout.links {
            replace_with_link(&work_path, link, output).map_err(&cannot_write)?;
        }
        Ok(Artifact {
            path: output.to_owned(),
            kind: match kind {
                LibraryKind::Static => ArtifactKind::StaticLibrary,
                LibraryKind::Shared => ArtifactKind::SharedLibrary,
            },
        })
    }

    /// The program object and the runtime archive linked into one object whose global symbols
    /// but the exports are made local, so two TyPy libraries link into one program, in an
    /// archive.
    fn static_library(
        &self,
        work: &Utf8Path,
        object: &Utf8Path,
        exports: &[String],
    ) -> Result<Utf8PathBuf, Step> {
        let combined = work.join("library.o");
        let kept = work.join("exports.txt");
        let archive = work.join("library.a");
        run_tool(
            &self.program,
            &[
                "-r".to_owned(),
                "-nostdlib".to_owned(),
                "-o".to_owned(),
                combined.to_string(),
                object.to_string(),
                self.archive.path().to_string(),
            ],
        )?;
        let listed = exports.iter().fold(String::new(), |mut text, name| {
            let _ = writeln!(text, "{name}");
            text
        });
        std::fs::write(&kept, listed).map_err(Step::Write)?;
        let mut keep = vec![format!("--keep-global-symbols={kept}")];
        if self.strip_debug {
            keep.push("--strip-debug".to_owned());
        }
        keep.push(combined.to_string());
        run_tool("objcopy", &keep)?;
        // `D` writes zero for the times and owners, so a build gives the same bytes each time
        // (NFR-06).
        run_tool(
            "ar",
            &["rcsD".to_owned(), archive.to_string(), combined.to_string()],
        )?;
        Ok(archive)
    }

    /// The program object linked with the runtime archive as a shared object whose version
    /// script exports only the exports.
    fn shared_library(
        &self,
        work: &Utf8Path,
        object: &Utf8Path,
        exports: &[String],
        soname: Option<&str>,
    ) -> Result<Utf8PathBuf, Step> {
        let script = work.join("exports.map");
        let library = work.join("library.so");
        let global = exports.iter().fold(String::new(), |mut text, name| {
            let _ = writeln!(text, "    {name};");
            text
        });
        let text = if exports.is_empty() {
            "{\n  local: *;\n};\n".to_owned()
        } else {
            format!("{{\n  global:\n{global}  local: *;\n}};\n")
        };
        std::fs::write(&script, text).map_err(Step::Write)?;
        let mut arguments = vec!["-shared".to_owned(), "-o".to_owned(), library.to_string()];
        if let Some(soname) = soname {
            arguments.push(format!("-Wl,-soname,{soname}"));
        }
        arguments.extend(self.link_arguments(object));
        arguments.push(format!("-Wl,--version-script={script}"));
        run_tool(&self.program, &arguments)?;
        Ok(library)
    }
}

/// Makes `link` a symbolic link to the file name of `library`, which is beside it: made in
/// the work directory, then renamed over whatever `link` was, so the link is whole at every
/// moment.
fn replace_with_link(work: &Utf8Path, link: &Utf8Path, library: &Utf8Path) -> io::Result<()> {
    let target = library.file_name().unwrap_or_default();
    let made = work.join("link");
    std::os::unix::fs::symlink(target, &made)?;
    std::fs::rename(&made, link)
}

/// Why a step of a library build failed: its program, or writing one of its files.
enum Step {
    Tool(Diagnostics),
    Write(io::Error),
}

impl From<Diagnostics> for Step {
    fn from(diagnostics: Diagnostics) -> Self {
        Self::Tool(diagnostics)
    }
}
