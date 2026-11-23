//! Tests of `request`.

use super::{BuildRequest, RequestError};
use crate::{BuildMode, EmitStage, LibraryKind};
use typy_codegen::{Target, TargetTriple};
use typy_source::MainSource;

#[test]
fn a_new_request_is_a_debug_build_of_an_executable_for_the_host() {
    let request = BuildRequest::new("src/app.py").unwrap();
    assert_eq!(request.main(), &MainSource::File("src/app.py".into()));
    assert_eq!(request.directory(), "src");
    assert_eq!(request.mode(), BuildMode::DEBUG);
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
        .with_mode(BuildMode::RELEASE_SMALL)
        .with_target(musl)
        .with_emit(EmitStage::Tyir)
        .with_output("out/app");
    assert_eq!(request.mode(), BuildMode::RELEASE_SMALL);
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

#[test]
fn a_module_or_code_is_built_under_the_programs_name() {
    let module = MainSource::Module(typy_source::ModuleName::new("tools.run").unwrap());
    let request = BuildRequest::of_main(module.clone()).unwrap();
    assert_eq!(
        (request.main(), request.directory(), request.output_path()),
        (&module, camino::Utf8Path::new(""), "tools.run".into())
    );
    let code = BuildRequest::of_main(MainSource::Code("print(1)".to_owned())).unwrap();
    assert_eq!(
        (code.directory(), code.output_path()),
        (camino::Utf8Path::new(""), "c".into())
    );
}
