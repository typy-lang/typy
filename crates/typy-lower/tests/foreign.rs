//! Tests of how lowering calls a C function a `ctypes` declaration describes, through the
//! public API: each argument becomes the value its C type takes (FFI-02, FFI-03).

use camino::Utf8PathBuf;
use typy_check::TypeChecker;
use typy_source::{Project, ProjectSettings};
use typy_tyir::{CType, print};

fn lower(source: &str) -> typy_tyir::TyirModule {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let directory = std::env::temp_dir().join(format!(
        "typy-foreign-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let root = Utf8PathBuf::from_path_buf(directory.clone()).unwrap();
    std::fs::write(root.join("app.py"), source).unwrap();
    let project = Project::load(&root.join("app.py"), ProjectSettings::new(root))
        .expect("the program parses");
    let program = TypeChecker::check(&project)
        .unwrap_or_else(|diagnostics| panic!("rejected: {diagnostics:?}\n{source}"));
    let module = typy_lower::lower(&program)
        .unwrap_or_else(|diagnostics| panic!("{diagnostics:?}\n{source}"));
    let _ = std::fs::remove_dir_all(directory);
    module
}

#[test]
fn every_c_type_reaches_the_module_and_each_call_is_a_foreign_call() {
    let source = "\
import ctypes

lib = ctypes.CDLL(\"libc.so.6\")
lib.f.argtypes = [ctypes.c_int8, ctypes.c_uint8, ctypes.c_int16, ctypes.c_uint16, ctypes.c_int32, ctypes.c_uint32, ctypes.c_int64, ctypes.c_uint64]
lib.f.restype = ctypes.c_uint64
lib.g.argtypes = [ctypes.c_float, ctypes.c_double, ctypes.c_bool, ctypes.c_char_p, ctypes.c_void_p]
lib.g.restype = None
lib.h.argtypes = [ctypes.c_double, ctypes.c_void_p, ctypes.c_char_p, ctypes.c_void_p]
lib.h.restype = ctypes.c_float
print(lib.f(1, 2, 3, 4, 5, 6, True, 8))
lib.g(1.5, 2, [1], b'x', None)
print(lib.h(False, 3, None, b'y'))
";
    let module = lower(source);
    assert_eq!(module.foreign_libraries, ["libc.so.6"]);
    let names: Vec<&str> = module
        .foreign_functions
        .iter()
        .map(|function| function.name.as_str())
        .collect();
    assert_eq!(names, ["f", "g", "h"]);
    assert_eq!(
        module.foreign_functions[0].parameters,
        [
            CType::Int8,
            CType::UInt8,
            CType::Int16,
            CType::UInt16,
            CType::Int32,
            CType::UInt32,
            CType::Int64,
            CType::UInt64
        ]
    );
    assert_eq!(module.foreign_functions[1].result, None);
    let text = print(&module);
    assert_eq!(text.matches("call_foreign ").count(), 3, "{text}");
    assert!(text.contains("rt.ctypes_float_argument"), "{text}");
}
