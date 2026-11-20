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
mod tests {
    use camino::Utf8PathBuf;
    use typy_codegen::TargetTriple;

    use super::library_argument;

    #[test]
    fn an_archive_name_drops_only_releases_of_numbers() {
        for (stem, archive) in [
            ("libtools.3", "libtools.a"),
            ("libtools.1.2", "libtools.a"),
            ("libtools.a", "libtools.a"),
            ("libtools.beta.3", "libtools.beta.a"),
            ("libtools.", "libtools..a"),
        ] {
            assert_eq!(super::archive_of(stem), archive, "{stem}");
        }
    }

    #[test]
    fn gnu_ld_links_a_library_by_its_file_name_or_its_archive() {
        for target in [
            TargetTriple::X86_64LinuxGnu,
            TargetTriple::X86_64LinuxMusl,
            TargetTriple::Aarch64LinuxGnu,
        ] {
            for (library, statically, argument) in [
                ("libm.so.6", false, "-l:libm.so.6"),
                ("libtools.so.3", true, "-l:libtools.a"),
                ("libz.so", true, "-l:libz.a"),
                ("libpool.a", true, "-l:libpool.a"),
                ("libpool.a", false, "-l:libpool.a"),
            ] {
                assert_eq!(
                    library_argument(target, library, statically, &[]),
                    argument,
                    "{target} {library}"
                );
            }
        }
    }

    #[test]
    fn apples_linker_links_a_dylib_by_its_name() {
        let macos = TargetTriple::Aarch64AppleDarwin;
        for (library, argument) in [
            ("libtools.dylib", "-ltools"),
            ("libtools.3.dylib", "-ltools.3"),
            ("libc.dylib", "-lc"),
            ("libm.so.6", "libm.so.6"),
            ("lib.dylib", "lib.dylib"),
        ] {
            assert_eq!(library_argument(macos, library, false, &[]), argument);
        }
    }

    #[test]
    fn apples_linker_links_an_archive_by_its_path_in_the_library_paths() {
        let directory = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(directory.path().to_owned()).unwrap();
        let (empty, holding) = (root.join("empty"), root.join("lib"));
        std::fs::create_dir_all(&empty).unwrap();
        std::fs::create_dir_all(&holding).unwrap();
        std::fs::write(holding.join("libtools.a"), "!<arch>\n").unwrap();
        let paths = [empty, holding.clone()];
        let macos = TargetTriple::Aarch64AppleDarwin;
        let found = holding.join("libtools.a").to_string();
        for library in ["libtools.dylib", "libtools.3.dylib", "libtools.1.2.dylib"] {
            assert_eq!(library_argument(macos, library, true, &paths), found);
        }
        assert_eq!(library_argument(macos, "libtools.a", true, &paths), found);
        assert_eq!(
            library_argument(macos, "libfoo2.dylib", true, &paths),
            "libfoo2.a"
        );
    }
}
