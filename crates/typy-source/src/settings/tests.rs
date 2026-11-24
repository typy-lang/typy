//! Tests of `settings`.

use super::{Platform, ProjectSettings, PythonFloor, Release};

#[test]
fn sys_platform_is_linux_on_linux_and_darwin_on_macos() {
    assert_eq!(Platform::Linux.sys_platform(), "linux");
    assert_eq!(Platform::MacOs.sys_platform(), "darwin");
}

#[test]
fn the_platform_is_the_hosts_until_a_target_names_another() {
    let expected = if cfg!(target_os = "macos") {
        Platform::MacOs
    } else {
        Platform::Linux
    };
    assert_eq!(Platform::host(), expected);
    assert_eq!(ProjectSettings::new("app".into()).platform, expected);
}

#[test]
fn every_floor_parses_from_its_own_text() {
    for floor in PythonFloor::ALL {
        assert_eq!(PythonFloor::parse(floor.as_str()), Some(floor));
        assert_eq!(floor.as_str(), format!("3.{}", floor.minor()));
    }
}

#[test]
fn floors_outside_the_oracle_range_are_refused() {
    for text in ["", "3.11", "3.16", "3", "3.12.0", " 3.12", "312"] {
        assert_eq!(PythonFloor::parse(text), None, "{text:?}");
    }
}

#[test]
fn default_settings_use_release_0_1_and_floor_3_12() {
    let settings = ProjectSettings::new("/work".into());
    assert_eq!(settings.release, Release::V0_1);
    assert_eq!(settings.release.as_str(), "0.1");
    assert_eq!(settings.python_floor, PythonFloor::V3_12);
}
