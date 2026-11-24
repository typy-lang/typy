//! Tests of `library_argument`.

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
