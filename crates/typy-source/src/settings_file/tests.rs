//! Tests of `settings_file`.

use super::{SETTINGS_FILE, SettingOverrides};
use crate::settings::{ProjectSettings, PythonFloor};
use camino::Utf8PathBuf;

fn root(settings: Option<&str>) -> (tempfile::TempDir, Utf8PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(directory.path().to_owned()).unwrap();
    if let Some(text) = settings {
        std::fs::write(root.join(SETTINGS_FILE), text).unwrap();
    }
    std::fs::create_dir(root.join("lib")).unwrap();
    (directory, root)
}

#[test]
fn without_typy_toml_the_defaults_apply() {
    let (_guard, root) = root(None);
    let settings = ProjectSettings::read(&root, SettingOverrides::default()).unwrap();
    assert_eq!(settings, ProjectSettings::new(root));
}

#[test]
fn each_setting_is_read_and_a_flag_wins_over_the_file() {
    let (_guard, root) = root(Some(
        "# The project's settings.\npython-floor = \"3.14\"\nmodule-paths = [\"lib\"]\n",
    ));
    let settings = ProjectSettings::read(&root, SettingOverrides::default()).unwrap();
    assert_eq!(settings.python_floor, PythonFloor::V3_14);
    assert_eq!(settings.module_paths, [Utf8PathBuf::from("lib")]);
    let overrides = SettingOverrides {
        python_floor: Some(PythonFloor::V3_15),
        ..SettingOverrides::default()
    };
    let settings = ProjectSettings::read(&root, overrides).unwrap();
    assert_eq!(settings.python_floor, PythonFloor::V3_15);
}

#[test]
fn every_wrong_setting_is_reported_once() {
    let (_guard, root) = root(Some(
        "python-floor = 3\nmodule-paths = [\"/abs\", \"lib\"]\nother = true\n",
    ));
    let failure = ProjectSettings::read(&root, SettingOverrides::default()).unwrap_err();
    let codes: Vec<String> = failure
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code().to_string())
        .collect();
    assert_eq!(codes, ["E0107"; 3]);
}

#[test]
fn each_wrong_value_names_what_it_is() {
    for (text, message) in [
        (
            "python-floor = \"3.11\"\n",
            "`python-floor` is the string \"3.11\"",
        ),
        ("python-floor = [1]\n", "`python-floor` is an array"),
        (
            "module-paths = \"lib\"\n",
            "`module-paths` is the string \"lib\"",
        ),
        (
            "module-paths = [true]\n",
            "a path of `module-paths` is a boolean",
        ),
        ("module-paths = [\"lib\", \"\"]\n", "the path `` is not"),
        (
            "module-paths = [\"missing\"]\n",
            "`missing` is not a directory",
        ),
        ("[python-floor\n", "is not valid TOML"),
    ] {
        let (_guard, root) = root(Some(text));
        let failure = ProjectSettings::read(&root, SettingOverrides::default()).unwrap_err();
        let found = failure
            .diagnostics
            .iter()
            .next()
            .unwrap()
            .message()
            .to_owned();
        assert!(found.contains(message), "{text}: {found}");
    }
}

#[test]
fn version_names_a_librarys_release() {
    let (_guard, project) = root(Some("version = \"5.0rc1\"\n"));
    let settings = ProjectSettings::read(&project, SettingOverrides::default()).unwrap();
    assert_eq!(settings.version.as_deref(), Some("5.0rc1"));
    for (text, message) in [
        (
            "version = 5.0\n",
            "`version` is a float, not a string that starts with a release",
        ),
        (
            "version = \"latest\"\n",
            "`version` is the string \"latest\", not a string",
        ),
    ] {
        let (_wrong_guard, wrong) = root(Some(text));
        let failure = ProjectSettings::read(&wrong, SettingOverrides::default()).unwrap_err();
        let diagnostic = failure.diagnostics.iter().next().unwrap();
        assert!(
            diagnostic.message().starts_with(message),
            "{}",
            diagnostic.message()
        );
        assert_eq!(
            diagnostic.rule().map(typy_diagnostics::RuleId::as_str),
            Some("CLI-13")
        );
    }
}

#[test]
fn a_settings_file_that_cannot_be_read_is_e0107() {
    let (_guard, root) = root(None);
    std::fs::create_dir(root.join(SETTINGS_FILE)).unwrap();
    let failure = ProjectSettings::read(&root, SettingOverrides::default()).unwrap_err();
    let diagnostic = failure.diagnostics.iter().next().unwrap();
    assert_eq!(diagnostic.code().to_string(), "E0107");
    assert!(diagnostic.message().starts_with("cannot read "));
}
