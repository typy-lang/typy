//! Tests of `request`.

use super::{BuildRequest, RequestError};
use crate::{BuildMode, EmitStage, LibraryKind};
use typy_codegen::{Target, TargetTriple};

#[test]
fn a_new_request_is_a_debug_build_of_an_executable_for_the_host() {
    let request = BuildRequest::new("src/app.py").unwrap();
    assert_eq!(request.main(), "src/app.py");
    assert_eq!(request.mode(), BuildMode::Debug);
    assert_eq!(request.target(), Target::host());
    assert_eq!((request.emit(), request.output()), (None, None));
    assert_eq!(request.output_path(), "src/app");
}

#[test]
fn every_setting_can_be_changed() {
    let musl = Target {
        triple: TargetTriple::X86_64LinuxMusl,
    };
    let request = BuildRequest::new("app.py")
        .unwrap()
        .with_mode(BuildMode::ReleaseSmall)
        .with_target(musl)
        .with_emit(EmitStage::Tyir)
        .with_output("out/app");
    assert_eq!(request.mode(), BuildMode::ReleaseSmall);
    assert_eq!(request.target(), musl);
    assert_eq!(request.emit(), Some(EmitStage::Tyir));
    assert_eq!(request.output_path(), "out/app");
}

#[test]
fn a_request_may_ask_for_a_library() {
    let request = BuildRequest::new("src/tools.py").unwrap();
    assert_eq!(request.library(), None);
    let request = request.with_library(LibraryKind::Static);
    assert_eq!(request.library(), Some(LibraryKind::Static));
}

#[test]
fn a_main_file_that_is_not_python_is_refused() {
    for path in ["app", "app.ty", "app.py.txt", ""] {
        let error = BuildRequest::new(path).unwrap_err();
        assert_eq!(error, RequestError::NotPython(path.into()));
        assert_eq!(error.to_string(), format!("{path} is not a `.py` file"));
    }
}
