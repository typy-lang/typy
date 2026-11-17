//! Reads the `[libraries]` table of `typy.toml`: how each C library a `ctypes` declaration
//! names is linked (FFI-04, ADR-0047).

use std::collections::BTreeMap;

use toml_edit::Item;
use typy_diagnostics::RuleId;

use super::{Problem, shown};
use crate::settings::LibraryLink;

fn problem(range: std::ops::Range<usize>, message: String, help: Option<String>) -> Problem {
    Problem {
        range,
        message,
        help,
        rule: RuleId::FFI_04,
    }
}

/// Returns how each library the table names is linked, or what is wrong with it.
pub(super) fn libraries(item: &Item) -> Result<BTreeMap<String, LibraryLink>, Problem> {
    let Some(table) = item.as_table_like() else {
        return Err(problem(
            item.span().unwrap_or(0..0),
            format!("`libraries` is {}, not a table", shown(item)),
            Some(
                "write a table for each library, such as `[libraries.\"libtools.so.3\"]` and \
                 `link = \"static\"`"
                    .to_owned(),
            ),
        ));
    };
    let mut libraries = BTreeMap::new();
    for (name, value) in table.iter() {
        let range = value.span().unwrap_or(0..0);
        if !library_name(name) {
            return Err(problem(
                table
                    .get_key_value(name)
                    .and_then(|(key, _)| key.span())
                    .unwrap_or(range),
                format!("`{name}` is not the file name of a library"),
                Some(
                    "name the library as its `ctypes` declaration does, such as `libm.so.6`"
                        .to_owned(),
                ),
            ));
        }
        let link = value
            .as_table_like()
            .and_then(|entry| {
                (entry.len() == 1)
                    .then(|| entry.get("link"))
                    .flatten()
                    .and_then(Item::as_str)
            })
            .and_then(|link| match link {
                "static" => Some(LibraryLink::Static),
                "dynamic" => Some(LibraryLink::Dynamic),
                _ => None,
            });
        let Some(link) = link else {
            return Err(problem(
                range,
                format!(
                    "`libraries.\"{name}\"` is {}, not a table of one `link`, \"static\" or \
                     \"dynamic\"",
                    shown(value)
                ),
                Some("write `link = \"static\"` or `link = \"dynamic\"`".to_owned()),
            ));
        };
        libraries.insert(name.to_owned(), link);
    }
    Ok(libraries)
}

/// Returns whether `name` is a library's file name, which the linker reads after `-l:` and
/// never as a path or an option.
fn library_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('-')
        && name
            .chars()
            .all(|each| each.is_ascii_alphanumeric() || "_.+-".contains(each))
}

#[cfg(test)]
mod tests {
    use camino::Utf8PathBuf;

    use super::library_name;
    use crate::settings::ProjectSettings;
    use crate::settings_file::{SETTINGS_FILE, SettingOverrides};

    /// Returns the settings `text` gives, or the message and rule of the first problem.
    fn read(text: &str) -> Result<ProjectSettings, (String, String, Option<String>)> {
        let directory = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(directory.path().to_owned()).unwrap();
        std::fs::write(root.join(SETTINGS_FILE), text).unwrap();
        ProjectSettings::read(&root, SettingOverrides::default()).map_err(|failure| {
            let diagnostic = failure.diagnostics.iter().next().unwrap();
            (
                diagnostic.message().to_owned(),
                diagnostic
                    .rule()
                    .map(|rule| rule.as_str().to_owned())
                    .unwrap_or_default(),
                diagnostic.help().map(str::to_owned),
            )
        })
    }

    #[test]
    fn a_library_name_is_a_file_name_never_a_path_or_an_option() {
        for name in ["libm.so.6", "libtools.so.3", "libc++.so.1", "libfoo.a"] {
            assert!(library_name(name), "{name}");
        }
        for name in ["", "-lm", "lib/foo.so", "../libc.so", "lib foo.so"] {
            assert!(!library_name(name), "{name}");
        }
    }

    #[test]
    fn libraries_name_how_each_c_library_is_linked() {
        let directory = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(directory.path().to_owned()).unwrap();
        std::fs::create_dir(root.join("vendor")).unwrap();
        std::fs::write(
            root.join(SETTINGS_FILE),
            "library-paths = [\"vendor\"]\n\n[libraries.\"libtools.so.3\"]\nlink = \"static\"\n\n\
             [libraries.\"libm.so.6\"]\nlink = \"dynamic\"\n",
        )
        .unwrap();
        let settings = ProjectSettings::read(&root, SettingOverrides::default()).unwrap();
        assert_eq!(settings.library_paths, [Utf8PathBuf::from("vendor")]);
        assert_eq!(
            settings.libraries.get("libtools.so.3"),
            Some(&crate::settings::LibraryLink::Static)
        );
        assert_eq!(
            settings.libraries.get("libm.so.6"),
            Some(&crate::settings::LibraryLink::Dynamic)
        );
    }

    #[test]
    fn each_wrong_library_setting_is_e0107_under_ffi_04() {
        for (text, message) in [
            ("libraries = 1\n", "`libraries` is an integer, not a table"),
            (
                "[libraries.\"-lm\"]\nlink = \"static\"\n",
                "`-lm` is not the file name of a library",
            ),
            (
                "[libraries.\"libm.so.6\"]\nlink = \"shared\"\n",
                "not a table of one `link`",
            ),
            (
                "[libraries.\"libm.so.6\"]\nlink = \"static\"\nmode = 1\n",
                "not a table of one `link`",
            ),
            (
                "library-paths = [\"../lib\"]\n",
                "is not a directory under the project root",
            ),
            (
                "library-paths = \"lib\"\n",
                "`library-paths` is the string \"lib\"",
            ),
        ] {
            let (found, rule, _) = read(text).unwrap_err();
            assert!(found.contains(message), "{text}: {found}");
            assert_eq!(rule, "FFI-04", "{text}");
        }
    }
}
