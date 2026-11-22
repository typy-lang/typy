//! Tests of `settings_file::libraries`.

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
