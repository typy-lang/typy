//! Where `typy run` keeps the executables it builds, and the key that names each build.

use std::fmt::Write as _;

use camino::Utf8PathBuf;
use sha2::{Digest as _, Sha256};
use typy_build::BuildRequest;

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
/// version, the mode, the target, the main file's name and its text. Each part is preceded by
/// its length, so no two different inputs run together into the same bytes.
pub(crate) fn key(request: &BuildRequest, source: &[u8]) -> String {
    let name = request.main().file_name().unwrap_or("");
    let parts: [&[u8]; 5] = [
        env!("CARGO_PKG_VERSION").as_bytes(),
        request.mode().name().as_bytes(),
        request.target().triple.as_str().as_bytes(),
        name.as_bytes(),
        source,
    ];
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

#[cfg(test)]
mod tests {
    use super::key;
    use typy_build::{BuildMode, BuildRequest};

    #[test]
    fn the_key_changes_with_each_input() {
        let request = BuildRequest::new("app.py").unwrap();
        let base = key(&request, b"print(1)\n");
        assert_eq!(base.len(), 64);
        assert_eq!(base, key(&request, b"print(1)\n"));
        assert_ne!(base, key(&request, b"print(2)\n"));
        assert_ne!(
            base,
            key(
                &request.clone().with_mode(BuildMode::Release),
                b"print(1)\n"
            )
        );
        assert_ne!(
            base,
            key(&BuildRequest::new("other.py").unwrap(), b"print(1)\n")
        );
        assert_eq!(
            base,
            key(&BuildRequest::new("src/app.py").unwrap(), b"print(1)\n")
        );
    }
}
