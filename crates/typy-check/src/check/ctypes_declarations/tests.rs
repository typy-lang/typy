//! Tests of reading a module's `ctypes` declarations.

use typy_diagnostics::{Diagnostics, FileId};
use typy_source::{PythonFloor, SourceFile, parse};

use super::{ModuleDeclarations, Restype, read};
use crate::ctypes::{CData, CType};

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
    assert_eq!(
        cbrt.parameters.as_deref(),
        Some(&[CData::Scalar(CType::Double)][..])
    );
    assert_eq!(cbrt.result, Some(Restype::Of(CData::Scalar(CType::Double))));
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
        Some(&[CData::Scalar(CType::CharPointer)][..])
    );
    assert_eq!(
        strlen.result,
        Some(Restype::Of(CData::Scalar(CType::UInt64)))
    );
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

#[test]
fn statements_that_are_not_declarations_stay_in_the_module() {
    let (found, messages, left) = declarations(
        "import ctypes\n\
         import ctypes.util as helpers\n\
         from .ctypes import CDLL as Relative\n\
         lib = ctypes.CDLL(\"libc.so.6\")\n\
         other.f.argtypes = [ctypes.c_int]\n\
         lib.f.g.argtypes = [ctypes.c_int]\n\
         lib.f.signature = [ctypes.c_int]\n\
         lib.argtypes = [ctypes.c_int]\n\
         first = second = ctypes.CDLL(\"libm.so.6\")\n\
         name = ctypes.c_int\n",
    );
    assert_eq!(messages, Vec::<String>::new());
    assert_eq!(found.libraries.len(), 1);
    assert_eq!(found.functions, Vec::new());
    assert_eq!(left, 10);
}

#[test]
fn a_declaration_whose_value_is_refused_still_counts_as_made() {
    let (found, messages, _) = declarations(
        "import ctypes\n\
         lib = ctypes.CDLL(\"libc.so.6\")\n\
         lib.f.argtypes = [ctypes.c_wchar_p]\n\
         lib.f.restype = ctypes.c_longdouble\n",
    );
    assert_eq!(messages.len(), 2, "{messages:?}");
    let f = found.function("lib", "f").unwrap();
    assert_eq!(f.parameters.as_deref(), Some(&[][..]));
    assert_eq!(f.result, Some(Restype::Void));
}

#[test]
fn a_pointer_type_is_read_by_pointer_or_by_a_name_bound_to_one_before() {
    let (found, messages, left) = declarations(
        "import ctypes\n\
         from ctypes import POINTER, c_double\n\
         libm = ctypes.CDLL(\"libm.so.6\")\n\
         IntPointer = ctypes.POINTER(ctypes.c_int)\n\
         Real = c_double\n\
         libm.frexp.argtypes = [Real, IntPointer]\n\
         libm.frexp.restype = Real\n\
         libm.find.argtypes = [POINTER(c_double)]\n\
         libm.find.restype = POINTER(ctypes.c_int32)\n",
    );
    assert_eq!(messages, Vec::<String>::new());
    // The two bindings of names stay, with the imports and the library.
    assert_eq!(left, 5);
    let int_pointer = CData::Pointer(Box::new(CData::Scalar(CType::Int32)));
    let frexp = found.function("libm", "frexp").unwrap();
    assert_eq!(
        frexp.parameters.as_deref(),
        Some(&[CData::Scalar(CType::Double), int_pointer.clone()][..])
    );
    let find = found.function("libm", "find").unwrap();
    assert_eq!(
        find.parameters.as_deref(),
        Some(&[CData::Pointer(Box::new(CData::Scalar(CType::Double)))][..])
    );
    assert_eq!(find.result, Some(Restype::Of(int_pointer)));
}

#[test]
fn a_pointer_type_ctypes_has_that_typy_does_not_compile_is_refused() {
    for (source, message) in [
        (
            "lib.f.argtypes = [ctypes.POINTER(ctypes.POINTER(ctypes.c_int))]\n",
            "E0201 a pointer to a pointer or to an array is planned for TyPy 1.0 (FFI-10)",
        ),
        (
            "lib.f.argtypes = [ctypes.POINTER(ctypes.c_int, 2)]\n",
            "E0430 `POINTER` takes one `ctypes` type",
        ),
        (
            "lib.f.argtypes = [ctypes.POINTER(ctypes.c_char)]\n",
            "E0430 this is not one of the `ctypes` types TyPy compiles",
        ),
        (
            "lib.f.restype = ctypes.c_int * 3\n",
            "E0430 this is not one of the `ctypes` types TyPy compiles",
        ),
    ] {
        let (_, messages, _) = declarations(&format!(
            "import ctypes\nlib = ctypes.CDLL(\"lib.so\")\n{source}"
        ));
        assert!(
            messages.iter().any(|each| each.starts_with(message)),
            "{source}: {messages:?}"
        );
    }
}
