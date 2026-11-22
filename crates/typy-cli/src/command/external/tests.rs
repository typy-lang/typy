//! Tests of `command::external`.

use super::{find, is_command_name};

#[test]
fn a_command_name_is_a_word_and_never_a_path() {
    for name in ["fmt", "test", "bench-compare", "my_tool", "x2"] {
        assert!(is_command_name(name), "{name}");
    }
    for name in ["", "-v", "../tool", "a/b", "tool.sh", "a b", "\u{e9}"] {
        assert!(!is_command_name(name), "{name:?}");
    }
}

#[cfg(unix)]
#[test]
fn the_first_executable_program_on_the_path_is_found() {
    use std::os::unix::fs::PermissionsExt as _;
    let root = std::env::temp_dir().join(format!("typy-external-{}", std::process::id()));
    let (first, second) = (root.join("first"), root.join("second"));
    for directory in [&first, &second] {
        std::fs::create_dir_all(directory).unwrap();
    }
    // A file that is not executable is passed over for one that is, later on the path.
    std::fs::write(first.join("typy-tool"), "").unwrap();
    std::fs::write(second.join("typy-tool"), "#!/bin/sh\n").unwrap();
    std::fs::set_permissions(
        second.join("typy-tool"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    let path = std::env::join_paths([&first, &second]).unwrap();
    assert_eq!(find("typy-tool", &path), Some(second.join("typy-tool")));
    assert_eq!(find("typy-missing", &path), None);
    std::fs::remove_dir_all(root).unwrap();
}
