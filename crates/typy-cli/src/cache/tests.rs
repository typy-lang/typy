//! Tests of `cache`.

use typy_diagnostics::{WarningLevel, WarningLevels, WarningSelector};
use typy_source::{MainSource, ModuleName, PythonFloor, SettingOverrides};

/// The key without settings flags.
fn key(request: &BuildRequest, files: &[(String, String)], compiler: &[u8]) -> String {
    super::key(
        request,
        files,
        SettingOverrides::default(),
        &WarningLevels::default(),
        compiler,
    )
}
use typy_build::{BuildMode, BuildRequest};

/// Returns the levels that deny the warnings `selector` names.
fn denied(selector: WarningSelector) -> WarningLevels {
    let mut levels = WarningLevels::default();
    levels.set(selector, WarningLevel::Deny);
    levels
}

fn files(texts: &[(&str, &str)]) -> Vec<(String, String)> {
    texts
        .iter()
        .map(|(path, text)| ((*path).to_owned(), (*text).to_owned()))
        .collect()
}

#[test]
fn the_key_changes_with_each_input() {
    let request = BuildRequest::new("app.py").unwrap();
    let program = files(&[("app.py", "import tools\n"), ("tools.py", "x = 1\n")]);
    let base = key(&request, &program, b"build");
    assert_eq!(base.len(), 64);
    assert_eq!(base, key(&request, &program, b"build"));
    let others = [
        key(
            &request,
            &files(&[("app.py", "import tools\n"), ("tools.py", "x = 2\n")]),
            b"build",
        ),
        key(
            &request,
            &files(&[("app.py", "import tools\n"), ("other.py", "x = 1\n")]),
            b"build",
        ),
        key(&request, &program, b"another build"),
        key(
            &request.clone().with_mode(BuildMode::RELEASE),
            &program,
            b"build",
        ),
        key(&BuildRequest::new("other.py").unwrap(), &program, b"build"),
        super::key(
            &request,
            &program,
            SettingOverrides {
                python_floor: Some(PythonFloor::V3_15),
                ..SettingOverrides::default()
            },
            &WarningLevels::default(),
            b"build",
        ),
        super::key(
            &request,
            &program,
            SettingOverrides::default(),
            &denied(WarningSelector::All),
            b"build",
        ),
        super::key(
            &request,
            &program,
            SettingOverrides::default(),
            &denied(WarningSelector::parse("W0101").unwrap()),
            b"build",
        ),
        key(
            &BuildRequest::of_main(MainSource::Code("x = 1\n".to_owned())).unwrap(),
            &program,
            b"build",
        ),
        key(
            &BuildRequest::of_main(MainSource::Module(ModuleName::new("app").unwrap())).unwrap(),
            &program,
            b"build",
        ),
    ];
    for other in others {
        assert_ne!(base, other);
    }
    assert_eq!(
        base,
        key(
            &BuildRequest::new("src/app.py").unwrap(),
            &program,
            b"build"
        )
    );
    assert_eq!(super::compiler_identity(&request).len(), 48);
    assert_eq!(super::file_identity(None), [0; 24]);
}
