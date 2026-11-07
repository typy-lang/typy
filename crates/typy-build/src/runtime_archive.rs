//! Finds the runtime archive of a target and checks that it comes from this TyPy version.

use camino::{Utf8Path, Utf8PathBuf};
use typy_codegen::TargetTriple;
use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics};

const MARKER: &[u8] = b"typy-runtime-version:";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const FILE_NAME: &str = "libtypy_runtime.a";

/// A runtime archive whose version matches the compiler's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeArchive {
    path: Utf8PathBuf,
}

fn refused(message: String) -> Diagnostics {
    Diagnostics::from(
        Diagnostic::without_location(DiagnosticCode::error(703), message).with_help(
            "reinstall TyPy; in a TyPy checkout, `make runtime` builds and stages the archive",
        ),
    )
}

impl RuntimeArchive {
    /// Returns the archive installed with the compiler at `executable`, for `triple`:
    /// `<prefix>/lib/typy/<triple>/libtypy_runtime.a` for `<prefix>/bin/typy`.
    pub fn beside(executable: &Utf8Path, triple: TargetTriple) -> Result<Self, Diagnostics> {
        let prefix = executable
            .parent()
            .and_then(Utf8Path::parent)
            .unwrap_or_else(|| Utf8Path::new("."));
        Self::open(
            &prefix
                .join("lib/typy")
                .join(triple.as_str())
                .join(FILE_NAME),
        )
    }

    /// Returns the archive at `path`, after checking that its version is the compiler's.
    pub fn open(path: &Utf8Path) -> Result<Self, Diagnostics> {
        let bytes = std::fs::read(path)
            .map_err(|error| refused(format!("cannot read the runtime archive {path}: {error}")))?;
        let Some(start) = bytes
            .windows(MARKER.len())
            .position(|window| window == MARKER)
        else {
            return Err(refused(format!("{path} is not a TyPy runtime archive")));
        };
        let rest = &bytes[start + MARKER.len()..];
        let end = rest
            .iter()
            .position(|byte| *byte == b'\n')
            .unwrap_or(rest.len());
        let found = String::from_utf8_lossy(&rest[..end]);
        if found != VERSION {
            return Err(refused(format!(
                "the runtime archive {path} is from TyPy {found}, not {VERSION}"
            )));
        }
        Ok(Self {
            path: path.to_owned(),
        })
    }

    /// Returns the archive's path.
    #[must_use]
    pub fn path(&self) -> &Utf8Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::{RuntimeArchive, VERSION};
    use camino::Utf8PathBuf;
    use typy_codegen::TargetTriple;

    fn directory() -> (tempfile::TempDir, Utf8PathBuf) {
        let directory = tempfile::tempdir().unwrap();
        let path = Utf8PathBuf::from_path_buf(directory.path().to_owned()).unwrap();
        (directory, path)
    }

    fn message(result: Result<RuntimeArchive, typy_diagnostics::Diagnostics>) -> String {
        let diagnostics = result.unwrap_err();
        let diagnostic = diagnostics.iter().next().unwrap();
        assert_eq!(diagnostic.code().to_string(), "E0703");
        diagnostic.message().to_owned()
    }

    #[test]
    fn the_archive_beside_the_compiler_is_found() {
        let (_guard, root) = directory();
        let archive = root.join("lib/typy/x86_64-unknown-linux-gnu/libtypy_runtime.a");
        std::fs::create_dir_all(archive.parent().unwrap()).unwrap();
        std::fs::write(
            &archive,
            format!("!<arch>\ntypy-runtime-version:{VERSION}\nrest"),
        )
        .unwrap();
        let found =
            RuntimeArchive::beside(&root.join("bin/typy"), TargetTriple::X86_64LinuxGnu).unwrap();
        assert_eq!(found.path(), archive);
    }

    #[test]
    fn a_missing_or_foreign_archive_is_refused() {
        let (_guard, root) = directory();
        let path = root.join("libtypy_runtime.a");
        assert!(
            message(RuntimeArchive::open(&path)).starts_with("cannot read the runtime archive")
        );
        std::fs::write(&path, "!<arch>\nnothing").unwrap();
        assert_eq!(
            message(RuntimeArchive::open(&path)),
            format!("{path} is not a TyPy runtime archive")
        );
        std::fs::write(&path, "typy-runtime-version:9.9.9\n").unwrap();
        assert_eq!(
            message(RuntimeArchive::open(&path)),
            format!("the runtime archive {path} is from TyPy 9.9.9, not {VERSION}")
        );
        let missing = RuntimeArchive::beside(&root.join("typy"), TargetTriple::X86_64LinuxMusl);
        assert!(message(missing).contains("x86_64-unknown-linux-musl"));
    }
}
