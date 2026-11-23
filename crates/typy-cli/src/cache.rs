//! Where `typy run` keeps the executables it builds, and the key that names each build.

use std::fmt::Write as _;

use camino::Utf8PathBuf;
use sha2::{Digest as _, Sha256};
use typy_build::BuildRequest;
use typy_diagnostics::WarningLevels;
use typy_source::{PythonFloor, SettingOverrides};

/// Returns the cache directory: `$TYPY_HOME/cache`, else the user's cache directory, which is
/// `$XDG_CACHE_HOME/typy` or `~/.cache/typy`. Its location never changes a build's output.
pub(crate) fn directory() -> Utf8PathBuf {
    let variable = |name: &str| {
        std::env::var(name)
            .ok()
            .filter(|value| !value.is_empty())
            .map(Utf8PathBuf::from)
    };
    if let Some(home) = variable("TYPY_HOME") {
        return home.join("cache");
    }
    if let Some(cache) = variable("XDG_CACHE_HOME") {
        return cache.join("typy");
    }
    if let Some(home) = variable("HOME") {
        return home.join(".cache/typy");
    }
    Utf8PathBuf::from_path_buf(std::env::temp_dir())
        .unwrap_or_else(|_| Utf8PathBuf::from("/tmp"))
        .join("typy-cache")
}

/// Returns the SHA-256, in hexadecimal, of everything a build's output depends on: the compiler
/// version and `compiler`, which tells one build of it from another, the mode, the target, the
/// processor and its features, the main file's name, the settings flags, the level of each
/// warning, which decides whether a build fails (CLI-27), and the path and text of every file
/// of the program, `typy.toml` included (ADR-0015). Each part is preceded by its length, so no two different
/// inputs run together into the same bytes.
pub(crate) fn key(
    request: &BuildRequest,
    files: &[(String, String)],
    overrides: SettingOverrides,
    warning_levels: &WarningLevels,
    compiler: &[u8],
) -> String {
    let name = request.main().file_name().unwrap_or("");
    let floor = overrides.python_floor.map_or("", PythonFloor::as_str);
    // The processor and its features change the code, so they are part of the key (CLI-33).
    let options = request.codegen_options();
    let processor = options.processor.as_deref().unwrap_or("");
    let levels = warning_levels.to_string();
    let mut parts: Vec<&[u8]> = vec![
        env!("CARGO_PKG_VERSION").as_bytes(),
        compiler,
        request.mode().name().as_bytes(),
        request.target().triple.as_str().as_bytes(),
        processor.as_bytes(),
        options.features.as_bytes(),
        name.as_bytes(),
        floor.as_bytes(),
        levels.as_bytes(),
    ];
    for (path, text) in files {
        parts.push(path.as_bytes());
        parts.push(text.as_bytes());
    }
    let mut hash = Sha256::new();
    for part in parts {
        hash.update((part.len() as u64).to_le_bytes());
        hash.update(part);
    }
    let mut text = String::with_capacity(64);
    for byte in hash.finalize() {
        let _ = write!(text, "{byte:02x}");
    }
    text
}

/// Returns what tells this build of the compiler and its runtime from another of the same
/// version: the size and the modification time of the executable and of the runtime archive
/// for `request`'s target, each zero when it cannot be read. The archive counts on its own,
/// since `make runtime` rebuilds it without the compiler.
pub(crate) fn compiler_identity(request: &BuildRequest) -> Vec<u8> {
    let executable = std::env::current_exe().ok();
    let archive = executable.as_ref().and_then(|path| {
        let path = camino::Utf8Path::from_path(path)?;
        typy_build::RuntimeArchive::beside(path, request.target().triple, request.mode())
            .ok()
            .map(|archive| archive.path().as_std_path().to_owned())
    });
    let mut identity = Vec::with_capacity(48);
    for path in [executable, archive] {
        identity.extend_from_slice(&file_identity(path.as_deref()));
    }
    identity
}

/// Returns the size and the modification time of a file, or zeros.
fn file_identity(path: Option<&std::path::Path>) -> [u8; 24] {
    let metadata = path.and_then(|path| std::fs::metadata(path).ok());
    let length = metadata.as_ref().map_or(0, std::fs::Metadata::len);
    let modified = metadata
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |since| since.as_nanos());
    let mut identity = [0; 24];
    identity[..8].copy_from_slice(&length.to_le_bytes());
    identity[8..].copy_from_slice(&modified.to_le_bytes());
    identity
}

#[cfg(test)]
mod tests;
