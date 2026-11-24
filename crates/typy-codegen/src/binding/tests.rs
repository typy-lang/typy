//! Tests of `binding`.

use super::{handle_name, python_binding, python_string};
use typy_tyir::{CType, FunctionId, TyirExport, TyirLibrary};

fn export(name: &str, parameters: &[(&str, CType)], result: Option<CType>) -> TyirExport {
    TyirExport {
        name: name.to_owned(),
        function: FunctionId(0),
        parameters: parameters.iter().map(|(_, ty)| *ty).collect(),
        parameter_names: parameters
            .iter()
            .map(|(name, _)| (*name).to_owned())
            .collect(),
        result,
    }
}

fn library(exports: Vec<TyirExport>) -> TyirLibrary {
    TyirLibrary {
        name: "tools".to_owned(),
        version: Some("5.0".to_owned()),
        exports,
    }
}

#[test]
fn the_binding_declares_each_export_and_wraps_it_in_a_typed_function() {
    let text = python_binding(
        &library(vec![
            export(
                "add",
                &[("a", CType::Int64), ("b", CType::Int64)],
                Some(CType::Int64),
            ),
            export("reset", &[], None),
            export("ready", &[("ratio", CType::Double)], Some(CType::Bool)),
        ]),
        "libtools.so.5",
    );
    for line in [
        "import ctypes",
        "__all__ = [\"add\", \"reset\", \"ready\"]",
        "_library = ctypes.CDLL(\"libtools.so.5\")",
        "_library.add.argtypes = [ctypes.c_int64, ctypes.c_int64]",
        "_library.add.restype = ctypes.c_int64",
        "_library.reset.argtypes = []",
        "_library.reset.restype = None",
        "_library.ready.restype = ctypes.c_bool",
        "def add(a: int, b: int) -> int:",
        "    return _library.add(a, b)",
        "def reset() -> None:",
        "    _library.reset()",
        "def ready(ratio: float) -> bool:",
    ] {
        assert!(text.lines().any(|each| each == line), "{line}\n{text}");
    }
}

#[test]
fn the_library_variable_never_takes_a_name_the_module_uses() {
    let clashing = library(vec![
        export("_library", &[], None),
        export("f", &[("_library_", CType::Int64)], None),
    ]);
    assert_eq!(handle_name(&clashing), "_library__");
}

#[test]
fn a_string_literal_escapes_what_python_would_read_otherwise() {
    assert_eq!(python_string("a\"b\\c\n"), "\"a\\\"b\\\\c\\u000a\"");
    assert_eq!(python_string("libtools.so.5"), "\"libtools.so.5\"");
}
