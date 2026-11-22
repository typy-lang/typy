//! Tests of `settings_file::target`.

use camino::Utf8PathBuf;

use super::program_name;
use crate::settings::{ProjectSettings, TargetTools};
use crate::settings_file::{SETTINGS_FILE, SettingOverrides};

/// Returns the settings `text` gives, or the message and rule of the first problem.
fn read(text: &str) -> Result<ProjectSettings, (String, String)> {
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
        )
    })
}

#[test]
fn a_program_is_a_name_or_an_absolute_path_never_an_option() {
    for name in [
        "cc",
        "aarch64-linux-gnu-gcc",
        "x86_64-linux-gnu-gcc-14",
        "/opt/cross/bin/aarch64-linux-gnu-ar",
        "clang++",
    ] {
        assert!(program_name(name), "{name}");
    }
    for name in ["", "-fuse-ld=lld", "bin/cc", "../cc", "my cc", "cc;rm"] {
        assert!(!program_name(name), "{name}");
    }
}

#[test]
fn a_target_table_names_the_tools_that_build_for_it() {
    let settings = read(
        "[target.\"aarch64-unknown-linux-gnu\"]\nlinker = \"/opt/cross/bin/gcc\"\n\
             ar = \"llvm-ar\"\nobjcopy = \"llvm-objcopy\"\n\n\
             [target.\"x86_64-unknown-linux-gnu\"]\nlinker = \"clang\"\n",
    )
    .unwrap();
    assert_eq!(
        settings.targets.get("aarch64-unknown-linux-gnu"),
        Some(&TargetTools {
            linker: Some("/opt/cross/bin/gcc".to_owned()),
            archiver: Some("llvm-ar".to_owned()),
            object_copier: Some("llvm-objcopy".to_owned()),
        })
    );
    assert_eq!(
        settings.targets.get("x86_64-unknown-linux-gnu"),
        Some(&TargetTools {
            linker: Some("clang".to_owned()),
            ..TargetTools::default()
        })
    );
}

#[test]
fn a_target_table_that_is_not_valid_is_e0107_of_cli_14() {
    for (text, message) in [
        ("target = 3\n", "`target` is an integer, not a table"),
        (
            "[target.\"x86_64-apple-darwin\"]\nlinker = \"cc\"\n",
            "`x86_64-apple-darwin` is not a target TyPy builds for",
        ),
        (
            "[target]\n\"aarch64-unknown-linux-gnu\" = \"cc\"\n",
            "`target.\"aarch64-unknown-linux-gnu\"` is the string \"cc\", not a table of tools",
        ),
        (
            "[target.\"aarch64-unknown-linux-gnu\"]\nld = \"ld.lld\"\n",
            "`target.\"aarch64-unknown-linux-gnu\"` has no tool `ld`",
        ),
        (
            "[target.\"aarch64-unknown-linux-gnu\"]\nlinker = \"bin/cc\"\n",
            "`target.\"aarch64-unknown-linux-gnu\".linker` is the string \"bin/cc\", not \
                 the name or absolute path of a program",
        ),
    ] {
        assert_eq!(
            read(text).unwrap_err(),
            (message.to_owned(), "CLI-14".to_owned()),
            "{text}"
        );
    }
}
