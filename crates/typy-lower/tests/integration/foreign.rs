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
    assert_eq!(module.foreign_libraries, [Some("libc.so.6".to_owned())]);
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

#[test]
fn each_ctypes_class_is_one_entry_of_the_module_and_a_pointer_is_of_its_class() {
    use typy_tyir::{CDataShape, TyirCDataClass};
    let source = "\
import ctypes

lib = ctypes.CDLL(\"libm.so.6\")
lib.frexp.argtypes = [ctypes.c_double, ctypes.POINTER(ctypes.c_int)]
lib.frexp.restype = ctypes.c_double
lib.find.argtypes = [ctypes.c_void_p]
lib.find.restype = ctypes.POINTER(ctypes.c_int32)
found = ctypes.c_int()
numbers = (ctypes.c_int * 3)(1, 2, 3)
print(lib.frexp(2.0, ctypes.byref(found)), lib.frexp(1.0, numbers), lib.frexp(1.0, None))
print(lib.find(numbers)[0], numbers[1], len(numbers))
numbers[0] += 1
";
    let module = lower(source);
    let int = TyirCDataClass {
        name: "c_int".to_owned(),
        shape: CDataShape::Scalar(CType::Int32),
        size: 4,
        align: 4,
    };
    let pointer = TyirCDataClass {
        name: "LP_c_int".to_owned(),
        shape: CDataShape::Pointer(0),
        size: 8,
        align: 8,
    };
    let array = TyirCDataClass {
        name: "c_int_Array_3".to_owned(),
        shape: CDataShape::Array(0, 3),
        size: 12,
        align: 4,
    };
    assert_eq!(module.cdata_classes, [int, array, pointer]);
    assert_eq!(
        module.foreign_functions[0].parameters,
        [CType::Double, CType::Pointer(2)]
    );
    assert_eq!(module.foreign_functions[1].result, Some(CType::Pointer(2)));
    let text = print(&module);
    for shown in [
        "cdata_class#0",
        "rt.cdata_new",
        "rt.cdata_item",
        "rt.cdata_set_int",
        "rt.cdata_get_int",
        "rt.cdata_length",
        "call_foreign frexp",
    ] {
        assert!(text.contains(shown), "{shown} missing from\n{text}");
    }
}

#[test]
fn a_structure_is_a_class_of_its_fields_and_passes_by_value_as_its_own_c_type() {
    use typy_tyir::{CDataShape, TyirField};
    let source = "\
import ctypes

class Point(ctypes.Structure):
    _fields_ = [(\"x\", ctypes.c_int), (\"y\", ctypes.c_double)]

lib = ctypes.CDLL(\"libc.so.6\")
lib.f.argtypes = [Point, ctypes.POINTER(Point)]
lib.f.restype = Point
p = Point(1, y=2.0)
p.x += 1
q = lib.f(p, p)
print(q.y, (Point * 2)(p)[0].x)
";
    let module = lower(source);
    let point = module
        .cdata_classes
        .iter()
        .position(|class| class.name == "__main__.Point")
        .unwrap();
    let class = &module.cdata_classes[point];
    assert_eq!((class.size, class.align), (16, 8));
    let CDataShape::Structure(fields) = &class.shape else {
        panic!("{class:?}")
    };
    assert_eq!(fields[1].offset, 8);
    assert_eq!(
        fields
            .iter()
            .map(|field: &TyirField| module.cdata_classes[field.class as usize].name.as_str())
            .collect::<Vec<_>>(),
        ["c_int", "c_double"]
    );
    let point = u32::try_from(point).unwrap();
    assert_eq!(
        module.foreign_functions[0].parameters[0],
        CType::Structure(point)
    );
    assert_eq!(
        module.foreign_functions[0].result,
        Some(CType::Structure(point))
    );
    let text = print(&module);
    for shown in ["rt.cdata_field", "rt.cdata_new", "rt.cdata_get_float"] {
        assert!(text.contains(shown), "{shown} missing from\n{text}");
    }
}
