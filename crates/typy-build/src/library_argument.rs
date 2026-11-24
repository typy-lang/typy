//! The linker argument that links a C library a `ctypes` declaration names, as the target's
//! linker takes it (FFI-04, ADR-0047, ADR-0056).

use camino::Utf8PathBuf;
use typy_codegen::TargetTriple;

/// Returns the argument that links `library`, a file name such as `libtools.so.3` or
/// `libtools.dylib`: dynamically, or when `statically` its archive, the name up to `.so` or
/// `.dylib` and a release, with `.a`. GNU ld takes `-l:FILE` and `-l:BASE.a`; Apple's linker
/// takes `-lNAME` for `libNAME.dylib`, and the path of the archive in the first of `paths`
/// that holds it, since it has no `-l:`. A name the linker cannot find is passed as it is, so
/// the linker's error names it.
#[must_use]
pub fn library_argument(
    target: TargetTriple,
    library: &str,
    statically: bool,
    paths: &[Utf8PathBuf],
) -> String {
    if !target.is_macos() {
        return format!("-l:{}", gnu_file(library, statically));
    }
    if statically {
        let archive = archive_of(library.strip_suffix(".dylib").unwrap_or(library));
        return paths
            .iter()
            .map(|path| path.join(&archive))
            .find(|path| path.is_file())
            .map_or(archive, |path| path.to_string());
    }
    match library
        .strip_prefix("lib")
        .and_then(|name| name.strip_suffix(".dylib"))
    {
        Some(name) if !name.is_empty() => format!("-l{name}"),
        _ => library.to_owned(),
    }
}

/// Returns the file GNU ld links for `library`: itself, or the archive of the name up to `.so`.
fn gnu_file(library: &str, statically: bool) -> String {
    let base = library.split(".so").next().unwrap_or(library);
    if !statically || base.len() == library.len() {
        return library.to_owned();
    }
    format!("{base}.a")
}

/// Returns the archive of the file name `stem` without `.dylib`: itself when it is one, and
/// otherwise the name without a release of numbers, with `.a`, as `libtools.3` gives
/// `libtools.a`.
fn archive_of(stem: &str) -> String {
    if camino::Utf8Path::new(stem).extension() == Some("a") {
        return stem.to_owned();
    }
    let mut base = stem;
    while let Some((before, release)) = base.rsplit_once('.')
        && !release.is_empty()
        && release.bytes().all(|byte| byte.is_ascii_digit())
    {
        base = before;
    }
    format!("{base}.a")
}

#[cfg(test)]
mod tests;
