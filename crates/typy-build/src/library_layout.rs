//! Where a library build writes its library and the links to it, by the module's name and
//! the release of its version, as Linux names libraries (CLI-09, CLI-13, ADR-0045).

use camino::{Utf8Path, Utf8PathBuf};

use crate::library_kind::LibraryKind;

/// The files of one library: the library, the name a shared library gives the dynamic
/// loader, and the symbolic links to it, each beside it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryLayout {
    /// The library file.
    pub path: Utf8PathBuf,
    /// The soname a shared library records, which a program linked with it loads; `None`
    /// for a static library.
    pub soname: Option<String>,
    /// The links to the library, each naming its file.
    pub links: Vec<Utf8PathBuf>,
}

impl LibraryLayout {
    /// Returns the layout of the library of module `name` in `directory`. With a release
    /// `X.Y`, a shared library is `libNAME.so.X.Y`, whose soname is `libNAME.so.X`, with links
    /// of that name and of `libNAME.so`, and a static library is `libNAME.X.Y.a` with a link
    /// `libNAME.a`; without one, they are `libNAME.so` and `libNAME.a`.
    #[must_use]
    pub fn beside(
        directory: &Utf8Path,
        kind: LibraryKind,
        name: &str,
        release: Option<&str>,
    ) -> Self {
        let file = |file_name: String| directory.join(file_name);
        match (kind, release) {
            (LibraryKind::Static, None) => Self {
                path: file(format!("lib{name}.a")),
                soname: None,
                links: Vec::new(),
            },
            (LibraryKind::Static, Some(release)) => Self {
                path: file(format!("lib{name}.{release}.a")),
                soname: None,
                links: vec![file(format!("lib{name}.a"))],
            },
            (LibraryKind::Shared, None) => Self {
                path: file(format!("lib{name}.so")),
                soname: Some(format!("lib{name}.so")),
                links: Vec::new(),
            },
            (LibraryKind::Shared, Some(release)) => {
                let major = release.split('.').next().unwrap_or(release);
                let soname = format!("lib{name}.so.{major}");
                let path = file(format!("lib{name}.so.{release}"));
                let mut links = vec![file(soname.clone()), file(format!("lib{name}.so"))];
                links.retain(|link| *link != path);
                Self {
                    path,
                    soname: Some(soname),
                    links,
                }
            }
        }
    }

    /// Returns the layout of a library written at `path`, which `-o` names: no links, and a
    /// shared library's soname is the file's own name.
    #[must_use]
    pub fn at(path: &Utf8Path, kind: LibraryKind) -> Self {
        Self {
            path: path.to_owned(),
            soname: (kind == LibraryKind::Shared)
                .then(|| path.file_name().unwrap_or_default().to_owned()),
            links: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LibraryLayout;
    use crate::library_kind::LibraryKind;
    use camino::Utf8Path;

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
                Some("5.0")
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
                Some("5.0")
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
        let layout =
            LibraryLayout::beside(Utf8Path::new(""), LibraryKind::Shared, "tools", Some("2"));
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
            let layout = LibraryLayout::beside(Utf8Path::new(""), kind, "tools", None);
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
}
