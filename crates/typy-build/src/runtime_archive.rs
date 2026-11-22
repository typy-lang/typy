//! Finds the runtime archive of a target and checks that it comes from this TyPy version.

use camino::{Utf8Path, Utf8PathBuf};
use typy_codegen::TargetTriple;
use typy_diagnostics::{Diagnostic, DiagnosticCode, Diagnostics};

use crate::BuildMode;

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
    /// Returns the archive installed with the compiler at `executable`, for `triple` and a build
    /// in `mode`: `<prefix>/lib/typy/<triple>/libtypy_runtime.a` for `<prefix>/bin/typy`, or
    /// `lib/typy-small/` for `--release-small` (ADR-0100).
    pub fn beside(
        executable: &Utf8Path,
        triple: TargetTriple,
        mode: BuildMode,
    ) -> Result<Self, Diagnostics> {
        let prefix = executable
            .parent()
            .and_then(Utf8Path::parent)
            .unwrap_or_else(|| Utf8Path::new("."));
        Self::open(
            &prefix
                .join("lib")
                .join(mode.runtime_directory())
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
mod tests;
