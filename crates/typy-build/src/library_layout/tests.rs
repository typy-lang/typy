//! Tests of `library_layout`.

use super::LibraryLayout;
use crate::library_kind::LibraryKind;
use camino::Utf8Path;
use typy_codegen::TargetTriple;

const LINUX: TargetTriple = TargetTriple::X86_64LinuxGnu;
const MACOS: TargetTriple = TargetTriple::Aarch64AppleDarwin;

fn names(layout: &LibraryLayout) -> (String, Option<String>, Vec<String>) {
    (
        layout.path.to_string(),
        layout.soname.clone(),
        layout.links.iter().map(ToString::to_string).collect(),
    )
}

#[test]
fn a_versioned_library_is_named_as_linux_names_libraries() {
    let directory = Utf8Path::new("out");
    assert_eq!(
        names(&LibraryLayout::beside(
            directory,
            LibraryKind::Shared,
            "django",
            Some("5.0"),
            LINUX
        )),
        (
            "out/libdjango.so.5.0".to_owned(),
            Some("libdjango.so.5".to_owned()),
            vec![
                "out/libdjango.so.5".to_owned(),
                "out/libdjango.so".to_owned()
            ]
        )
    );
    assert_eq!(
        names(&LibraryLayout::beside(
            directory,
            LibraryKind::Static,
            "django",
            Some("5.0"),
            LINUX
        )),
        (
            "out/libdjango.5.0.a".to_owned(),
            None,
            vec!["out/libdjango.a".to_owned()]
        )
    );
}

#[test]
fn a_release_of_one_number_has_one_link() {
    let layout = LibraryLayout::beside(
        Utf8Path::new(""),
        LibraryKind::Shared,
        "tools",
        Some("2"),
        LINUX,
    );
    assert_eq!(
        names(&layout),
        (
            "libtools.so.2".to_owned(),
            Some("libtools.so.2".to_owned()),
            vec!["libtools.so".to_owned()]
        )
    );
}

#[test]
fn a_library_without_a_version_has_no_links() {
    for (kind, path, soname) in [
        (LibraryKind::Static, "libtools.a", None),
        (
            LibraryKind::Shared,
            "libtools.so",
            Some("libtools.so".to_owned()),
        ),
    ] {
        let layout = LibraryLayout::beside(Utf8Path::new(""), kind, "tools", None, LINUX);
        assert_eq!(names(&layout), (path.to_owned(), soname, Vec::new()));
    }
}

#[test]
fn an_output_path_is_the_library_with_its_own_name_as_soname() {
    let shared = LibraryLayout::at(Utf8Path::new("build/tools.so"), LibraryKind::Shared);
    assert_eq!(
        names(&shared),
        (
            "build/tools.so".to_owned(),
            Some("tools.so".to_owned()),
            Vec::new()
        )
    );
    let archive = LibraryLayout::at(Utf8Path::new("tools.a"), LibraryKind::Static);
    assert_eq!(archive.soname, None);
}

#[test]
fn a_library_for_macos_is_named_as_macos_names_libraries() {
    let directory = Utf8Path::new("out");
    let shared =
        LibraryLayout::beside(directory, LibraryKind::Shared, "django", Some("5.1"), MACOS);
    assert_eq!(
        names(&shared),
        (
            "out/libdjango.5.dylib".to_owned(),
            Some("libdjango.5.dylib".to_owned()),
            vec!["out/libdjango.dylib".to_owned()]
        )
    );
    assert_eq!(shared.versions, Some(("5.0".to_owned(), "5.1".to_owned())));
    let unversioned = LibraryLayout::beside(directory, LibraryKind::Shared, "django", None, MACOS);
    assert_eq!(
        names(&unversioned),
        (
            "out/libdjango.dylib".to_owned(),
            Some("libdjango.dylib".to_owned()),
            Vec::new()
        )
    );
    assert_eq!(unversioned.versions, None);
    for release in [None, Some("5.1")] {
        let archive =
            LibraryLayout::beside(directory, LibraryKind::Static, "django", release, MACOS);
        assert_eq!(
            names(&archive),
            ("out/libdjango.a".to_owned(), None, Vec::new())
        );
        assert_eq!(archive.versions, None);
    }
}

#[test]
fn a_library_for_linux_records_no_versions() {
    let shared = LibraryLayout::beside(
        Utf8Path::new(""),
        LibraryKind::Shared,
        "tools",
        Some("2.1"),
        LINUX,
    );
    assert_eq!(shared.versions, None);
}
