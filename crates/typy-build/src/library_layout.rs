//! Where a library build writes its library and the links to it, by the module's name and
//! the release of its version, as Linux and macOS name libraries (CLI-09, CLI-13, CLI-16,
//! ADR-0045, ADR-0056).

use camino::{Utf8Path, Utf8PathBuf};
use typy_codegen::TargetTriple;

use crate::library_kind::LibraryKind;

/// The files of one library: the library, the name a shared library gives the dynamic
/// loader, and the symbolic links to it, each beside it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryLayout {
    /// The library file.
    pub path: Utf8PathBuf,
    /// The name a shared library records, which a program linked with it loads: its soname
    /// on Linux and its install name on macOS; `None` for a static library.
    pub soname: Option<String>,
    /// The links to the library, each naming its file.
    pub links: Vec<Utf8PathBuf>,
    /// The compatibility and current versions a shared library for macOS records, `X.0`
    /// and `X.Y` of the release `X.Y`; `None` without a release, or on Linux.
    pub versions: Option<(String, String)>,
}

impl LibraryLayout {
    /// Returns the layout of the library of module `name` in `directory` for `target`. With
    /// a release `X.Y`, a shared library for Linux is `libNAME.so.X.Y`, whose soname is
    /// `libNAME.so.X`, with links of that name and of `libNAME.so`, and a static library is
    /// `libNAME.X.Y.a` with a link `libNAME.a`; without one, they are `libNAME.so` and
    /// `libNAME.a`. For macOS a shared library is `libNAME.X.dylib`, its own install name,
    /// with a link `libNAME.dylib`, or `libNAME.dylib` without a release, and a static
    /// library is `libNAME.a`.
    #[must_use]
    pub fn beside(
        directory: &Utf8Path,
        kind: LibraryKind,
        name: &str,
        release: Option<&str>,
        target: TargetTriple,
    ) -> Self {
        let file = |file_name: String| directory.join(file_name);
        let layout = |path: Utf8PathBuf, soname: Option<String>, links: Vec<Utf8PathBuf>| Self {
            path,
            soname,
            links,
            versions: None,
        };
        let major = release.map(|release| release.split('.').next().unwrap_or(release));
        if target.is_macos() {
            return match (kind, release, major) {
                (LibraryKind::Shared, Some(release), Some(major)) => {
                    let install_name = format!("lib{name}.{major}.dylib");
                    Self {
                        versions: Some((format!("{major}.0"), release.to_owned())),
                        ..layout(
                            file(install_name.clone()),
                            Some(install_name),
                            vec![file(format!("lib{name}.dylib"))],
                        )
                    }
                }
                (LibraryKind::Shared, ..) => layout(
                    file(format!("lib{name}.dylib")),
                    Some(format!("lib{name}.dylib")),
                    Vec::new(),
                ),
                (LibraryKind::Static, ..) => layout(file(format!("lib{name}.a")), None, Vec::new()),
            };
        }
        match (kind, release, major) {
            (LibraryKind::Static, Some(release), _) => layout(
                file(format!("lib{name}.{release}.a")),
                None,
                vec![file(format!("lib{name}.a"))],
            ),
            (LibraryKind::Static, ..) => layout(file(format!("lib{name}.a")), None, Vec::new()),
            (LibraryKind::Shared, Some(release), Some(major)) => {
                let soname = format!("lib{name}.so.{major}");
                let path = file(format!("lib{name}.so.{release}"));
                let mut links = vec![file(soname.clone()), file(format!("lib{name}.so"))];
                links.retain(|link| *link != path);
                layout(path, Some(soname), links)
            }
            (LibraryKind::Shared, ..) => layout(
                file(format!("lib{name}.so")),
                Some(format!("lib{name}.so")),
                Vec::new(),
            ),
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
            versions: None,
        }
    }
}

#[cfg(test)]
mod tests;
