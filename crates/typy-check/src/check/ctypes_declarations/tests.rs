//! Tests of reading a module's `ctypes` declarations.

use typy_diagnostics::{Diagnostics, FileId};
use typy_source::{PythonFloor, SourceFile, parse};

use super::{ModuleDeclarations, Restype, read};
use crate::ctypes::CType;

/// Returns the declarations of `source`, the codes and messages of its diagnostics, and how
/// many statements stay in the module.
fn declarations(source: &str) -> (ModuleDeclarations, Vec<String>, usize) {
    let file = SourceFile::from_text(FileId(0), "app.py".into(), source);
    let mut tree = parse(&file, PythonFloor::V3_12).unwrap();
    let mut diagnostics = Diagnostics::new();
    let found = read(&mut tree.body, &mut diagnostics);
    let messages = diagnostics
        .iter()
        .map(|diagnostic| format!("{} {}", diagnostic.code(), diagnostic.message()))
        .collect();
    (found, messages, tree.body.len())
}

#[test]
fn a_library_and_its_functions_are_read_and_their_declarations_leave_the_module() {
    let (found, messages, left) = declarations(
        "import ctypes\n\
         libm = ctypes.CDLL(\"libm.so.6\")\n\
         libm.cbrt.argtypes = [ctypes.c_double]\n\
         libm.cbrt.restype = ctypes.c_double\n\
         libm.labs.argtypes = (ctypes.c_long,)\n\
         libm.labs.restype = None\n\
         print(libm.cbrt(27.0))\n",
    );
    assert_eq!(messages, Vec::<String>::new());
    assert_eq!(left, 3);
    assert_eq!(found.libraries.len(), 1);
    assert_eq!(found.libraries[0].variable, "libm");
    assert_eq!(found.libraries[0].name, "libm.so.6");
    let cbrt = found.function("libm", "cbrt").unwrap();
    assert_eq!(cbrt.parameters.as_deref(), Some(&[CType::Double][..]));
    assert_eq!(cbrt.result, Some(Restype::Of(CType::Double)));
    assert_eq!(
        found.function("libm", "labs").unwrap().result,
        Some(Restype::Void)
    );
}

#[test]
fn every_way_of_naming_ctypes_and_loading_a_library_is_read() {
    let (found, messages, _) = declarations(
        "import ctypes as C\n\
         from ctypes import CDLL as Library, cdll, c_int, c_char_p as text\n\
         one = Library(\"libc.so.6\")\n\
         two = cdll.LoadLibrary(\"libm.so.6\")\n\
         three = C.cdll.LoadLibrary(\"libz.so.1\")\n\
         one.strlen.argtypes = [text]\n\
         one.strlen.restype = C.c_size_t\n\
         two.abs.argtypes = [c_int]\n",
    );
    assert_eq!(messages, Vec::<String>::new());
    let names: Vec<&str> = found
        .libraries
        .iter()
        .map(|library| library.name.as_str())
        .collect();
    assert_eq!(names, ["libc.so.6", "libm.so.6", "libz.so.1"]);
    let strlen = found.function("one", "strlen").unwrap();
    assert_eq!(
        strlen.parameters.as_deref(),
        Some(&[CType::CharPointer][..])
    );
    assert_eq!(strlen.result, Some(Restype::Of(CType::UInt64)));
    assert_eq!(found.function("two", "abs").unwrap().result, None);
}

#[test]
fn each_declaration_ctypes_would_not_run_alike_is_e0430() {
    for (source, message) in [
        (
            "lib = ctypes.CDLL(name)\n",
            "a library is named by one string literal",
        ),
        (
            "lib = ctypes.CDLL(\"a\", mode=1)\n",
            "a library is named by one string literal",
        ),
        (
            "lib = ctypes.CDLL(\"a\")\nlib.f.argtypes = [ctypes.c_char]\n",
            "this is not one of the `ctypes` types TyPy compiles",
        ),
        (
            "lib = ctypes.CDLL(\"a\")\nlib.f.argtypes = types\n",
            "`argtypes` is assigned a list or tuple of `ctypes` types",
        ),
        (
            "lib = ctypes.CDLL(\"a\")\nlib.f.restype = int\n",
            "this is not one of the `ctypes` types TyPy compiles",
        ),
        (
            "lib = ctypes.CDLL(\"a\")\nlib.f.restype = None\nlib.f.restype = None\n",
            "`lib.f.restype` is assigned again",
        ),
    ] {
        let (_, messages, _) = declarations(&format!("import ctypes\n{source}"));
        assert_eq!(messages.len(), 1, "{source}: {messages:?}");
        assert!(messages[0].starts_with("E0430 "), "{messages:?}");
        assert!(messages[0].contains(message), "{source}: {messages:?}");
    }
}

#[test]
fn a_module_without_ctypes_is_left_alone() {
    let (found, messages, left) = declarations("lib = CDLL(\"a\")\nlib.f.argtypes = []\n");
    assert_eq!(found, ModuleDeclarations::default());
    assert_eq!(messages, Vec::<String>::new());
    assert_eq!(left, 2);
}
