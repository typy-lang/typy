//! Tests of how the checker types `ctypes` declarations, calls and objects, through the
//! public API: the program it gives, and the diagnostics of each use TyPy does not compile
//! (FFI-01 to FFI-03, FFI-09, FFI-10, ADR-0047, ADR-0048).

mod common;

use camino::Utf8PathBuf;
use typy_check::{CData, CType, TypeChecker};
use typy_source::{Project, ProjectSettings};

const DECLARED: &str = "\
import ctypes
from ctypes import c_int

libc = ctypes.CDLL(\"libc.so.6\")
libc.abs.argtypes = [c_int]
libc.abs.restype = c_int
libc.puts.argtypes = [ctypes.c_char_p]
libc.puts.restype = None
";

/// Returns the code and message of each diagnostic of `source` after `DECLARED`.
fn refused(source: &str) -> Vec<String> {
    let root = Utf8PathBuf::from_path_buf(tempfile_root()).unwrap();
    std::fs::write(root.join("app.py"), format!("{DECLARED}{source}")).unwrap();
    let project = Project::load(&root.join("app.py"), ProjectSettings::new(root.clone())).unwrap();
    let found = match TypeChecker::check(&project) {
        Ok(_) => Vec::new(),
        Err(diagnostics) => diagnostics
            .iter()
            .map(|diagnostic| format!("{} {}", diagnostic.code(), diagnostic.message()))
            .collect(),
    };
    let _ = std::fs::remove_dir_all(&root);
    found
}

fn tempfile_root() -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let root = std::env::temp_dir().join(format!(
        "typy-ctypes-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn a_declared_function_is_a_typed_c_call() {
    assert_eq!(
        common::app(&format!("{DECLARED}print(libc.abs(-3), libc.puts(b'x'))\n")),
        Vec::<String>::new()
    );
    let root = Utf8PathBuf::from_path_buf(tempfile_root()).unwrap();
    std::fs::write(
        root.join("app.py"),
        format!("{DECLARED}print(libc.abs(-3))\n"),
    )
    .unwrap();
    let project = Project::load(&root.join("app.py"), ProjectSettings::new(root.clone())).unwrap();
    let program = TypeChecker::check(&project).unwrap();
    let _ = std::fs::remove_dir_all(&root);
    assert_eq!(program.foreign_libraries.len(), 1);
    assert_eq!(program.foreign_libraries[0].name, "libc.so.6");
    assert_eq!(program.foreign_functions.len(), 1);
    assert_eq!(program.foreign_functions[0].name, "abs");
    assert_eq!(
        program.foreign_functions[0].parameters,
        [CData::Scalar(CType::Int32)]
    );
    assert_eq!(
        program.foreign_functions[0].result,
        Some(CData::Scalar(CType::Int32))
    );
}

#[test]
fn a_pointer_parameter_is_declared_by_pointer_or_by_a_name_bound_to_one() {
    let root = Utf8PathBuf::from_path_buf(tempfile_root()).unwrap();
    let source = "IntPointer = ctypes.POINTER(c_int)\n\
                  libc.frexp.argtypes = [ctypes.c_double, IntPointer]\n\
                  libc.frexp.restype = ctypes.c_double\n\
                  libc.__errno_location.argtypes = []\n\
                  libc.__errno_location.restype = ctypes.POINTER(ctypes.c_int32)\n\
                  e = c_int()\n\
                  print(libc.frexp(2.0, ctypes.byref(e)), libc.frexp(2.0, e), libc.frexp(1.0, None))\n\
                  print(libc.__errno_location()[0])\n";
    std::fs::write(root.join("app.py"), format!("{DECLARED}{source}")).unwrap();
    let project = Project::load(&root.join("app.py"), ProjectSettings::new(root.clone())).unwrap();
    let program = TypeChecker::check(&project).unwrap();
    let _ = std::fs::remove_dir_all(&root);
    let pointer = CData::Pointer(Box::new(CData::Scalar(CType::Int32)));
    assert_eq!(
        program.foreign_functions[0].parameters,
        [CData::Scalar(CType::Double), pointer.clone()]
    );
    assert_eq!(program.foreign_functions[1].result, Some(pointer));
}

#[test]
fn each_call_ctypes_would_refuse_is_an_error_where_it_is_compiled() {
    for (source, message) in [
        (
            "libc.abs(x=1)\n",
            "E0404 `libc.abs` is a C function, which takes positional arguments only",
        ),
        (
            "libc.abs(1, 2)\n",
            "E0404 `libc.abs` takes 1 argument, as its `argtypes` declare, and 2 were given",
        ),
        (
            "libc.puts()\n",
            "E0404 `libc.puts` takes 1 argument, as its `argtypes` declare, and 0 were given",
        ),
        (
            "libc.abs(1.5)\n",
            "E0404 argument 1 of `libc.abs` is a float, where its `argtypes` take an `int`",
        ),
        (
            "libc.puts('text')\n",
            "E0404 argument 1 of `libc.puts` is a str, where its `argtypes` take `bytes` or `None`",
        ),
        (
            "libc.strlen(b'x')\n",
            "E0430 `libc.strlen` is called without both its `argtypes` and its `restype`",
        ),
        (
            "print(libc)\n",
            "E0201 the library `libc` used other than to call its functions is planned for TyPy 1.0 (FFI-09)",
        ),
        (
            "print(c_int)\n",
            "E0201 the class `c_int` used as a value is planned for TyPy 1.0 (FFI-10)",
        ),
        (
            "x = ctypes.POINTER\n",
            "E0201 using `ctypes.POINTER` as a value is planned for TyPy 1.0 (FUN-05)",
        ),
        (
            "def load() -> None:\n    other = ctypes.CDLL(\"libm.so.6\")\n",
            "E0430 a library is declared at module level and bound to a name",
        ),
    ] {
        let found = refused(source);
        assert!(
            found.iter().any(|each| each.starts_with(message)),
            "{source}: {found:?}"
        );
    }
}

#[test]
fn a_local_name_hides_a_library_of_the_same_name() {
    let found = refused("def f(libc: int) -> int:\n    return libc.abs(1)\n");
    assert!(
        found.iter().any(|each| each.starts_with("E0412")),
        "{found:?}"
    );
    assert_eq!(
        refused("def f() -> int:\n    return libc.abs(-1)\n"),
        Vec::<String>::new()
    );
}

#[test]
fn a_c_bool_argument_takes_any_value_with_a_truth() {
    let source = "libc.isdigit.argtypes = [ctypes.c_bool]\nlibc.isdigit.restype = ctypes.c_bool\nprint(libc.isdigit([1]), libc.isdigit(None), libc.isdigit('x'))\n";
    assert_eq!(refused(source), Vec::<String>::new());
}

#[test]
fn the_typed_dump_shows_a_library_and_a_c_call() {
    let root = Utf8PathBuf::from_path_buf(tempfile_root()).unwrap();
    std::fs::write(
        root.join("app.py"),
        format!("{DECLARED}print(libc.abs(-3))\n"),
    )
    .unwrap();
    let project = Project::load(&root.join("app.py"), ProjectSettings::new(root.clone())).unwrap();
    let program = TypeChecker::check(&project).unwrap();
    let dump = typy_check::dump(&program, &project);
    let _ = std::fs::remove_dir_all(&root);
    for node in ["ForeignLibrary libc.so.6", "ForeignCall abs: int"] {
        assert!(dump.contains(node), "{node} missing from\n{dump}");
    }
}

#[test]
#[allow(clippy::too_many_lines, reason = "one row per refused use")]
fn each_use_of_a_ctypes_object_ctypes_would_refuse_is_an_error_where_it_is_compiled() {
    for (source, message) in [
        (
            "ctypes.c_int(1.5)\n",
            "E0404 the value of `c_int` is a float, where it takes an `int`",
        ),
        (
            "ctypes.c_int(1, 2)\n",
            "E0404 `c_int` takes 0 or 1 positional arguments, and 2 are given",
        ),
        (
            "ctypes.c_int(value=1)\n",
            "E0404 `c_int` takes positional arguments only",
        ),
        (
            "ctypes.c_void_p(b'x')\n",
            "E0404 the value of `c_void_p` is a bytes, where it takes an `int` or `None`",
        ),
        (
            "(ctypes.c_double * 2)(1, 'x')\n",
            "E0404 an item of `c_double_Array_2` is a str, where it takes an `int` or a `float`",
        ),
        (
            "x = c_int(1)\nx.value = 'a'\n",
            "E0402 the value of `c_int` is assigned a str, where it takes an `int`",
        ),
        (
            "a = (c_int * 2)()\na[0] = 1.5\n",
            "E0402 an item of `c_int_Array_2` is assigned a float, where it takes an `int`",
        ),
        (
            "x = c_int(1)\nx.value += 0.5\n",
            "E0402 the value of `c_int` is assigned a float, where it takes an `int`",
        ),
        (
            "x = c_int(1)\nprint(x.contents)\n",
            "E0412 a `c_int` has no attribute `contents` in TyPy",
        ),
        (
            "x = c_int(1)\nprint(x[0])\n",
            "E0401 a `c_int` cannot be indexed",
        ),
        (
            "x = c_int(1)\nx[0] = 1\n",
            "E0401 a `c_int` cannot be indexed",
        ),
        (
            "p = ctypes.pointer(c_int(1))\nprint(p.value)\n",
            "E0412 a `LP_c_int` has no attribute `value` in TyPy",
        ),
        (
            "p = ctypes.pointer(c_int(1))\np.size = 1\n",
            "E0412 a `LP_c_int` has no attribute `size` in TyPy",
        ),
        (
            "p = ctypes.pointer(c_int(1))\np.contents = c_int(2)\n",
            "E0201 assigning a pointer's `contents` is planned for TyPy 1.0 (FFI-10)",
        ),
        (
            "a = (c_int * 2)()\nprint(a[0:1])\n",
            "E0201 a slice of a `ctypes` array or pointer is planned for TyPy 1.0 (FFI-10)",
        ),
        (
            "a = (c_int * 2)()\na[0:1] = [1]\n",
            "E0201 a slice of a `ctypes` array or pointer is planned for TyPy 1.0 (FFI-10)",
        ),
        (
            "n = 2\nA = c_int * n\n",
            "E0201 an array whose length is not an integer literal is planned for TyPy 1.0",
        ),
        (
            "A = c_int * 5000000000\n",
            "E0404 an array of 5000000000 items is too large for TyPy",
        ),
        (
            "A = (c_int * 2) * 3\n",
            "E0201 an array of pointers or arrays is planned for TyPy 1.0 (FFI-10)",
        ),
        (
            "ctypes.pointer(ctypes.pointer(c_int(1)))\n",
            "E0201 a pointer to a pointer or to an array is planned for TyPy 1.0 (FFI-10)",
        ),
        (
            "ctypes.POINTER(c_int * 2)\n",
            "E0201 a pointer to a pointer or to an array is planned for TyPy 1.0 (FFI-10)",
        ),
        (
            "ctypes.byref(c_int(1))\n",
            "E0201 `ctypes.byref` other than as a C function's argument is planned for TyPy 1.0",
        ),
        (
            "ctypes.sizeof(5)\n",
            "E0404 `ctypes.sizeof` takes a `ctypes` class or object, and an int has no size",
        ),
        (
            "ctypes.pointer(5)\n",
            "E0404 `ctypes.pointer` takes a `ctypes` object, and an int is given",
        ),
        (
            "ctypes.POINTER(5)\n",
            "E0404 `ctypes.POINTER` takes a `ctypes` class, and an int is given",
        ),
        (
            "ctypes.pointer()\n",
            "E0404 `ctypes.pointer` takes 1 positional argument, and 0 are given",
        ),
        (
            "ctypes.POINTER(c_int)(ctypes.c_long(1))\n",
            "E0404 `LP_c_int` takes a `c_int`, and a c_long is given",
        ),
        (
            "ctypes.POINTER(c_int)(1, 2)\n",
            "E0404 `LP_c_int` takes 0 or 1 positional arguments, and 2 are given",
        ),
        (
            "x = ctypes.c_char(1)\n",
            "E0201 `ctypes.c_char` is planned for TyPy 1.0 (FFI-10)",
        ),
        (
            "x = ctypes.CFUNCTYPE(None)\n",
            "E0201 `ctypes.CFUNCTYPE` is planned for TyPy 1.0 (FFI-06)",
        ),
        (
            "x = ctypes.Structure\n",
            "E0201 `ctypes.Structure` is planned for TyPy 0.5 (FFI-03)",
        ),
        ("s = {c_int(1)}\n", "E0413"),
        (
            "if c_int:\n    pass\n",
            "E0201 the class `c_int` used as a value",
        ),
        (
            "A = c_int * c_int\n",
            "E0201 the class `c_int` used as a value",
        ),
        (
            "ctypes.sizeof(x=c_int)\n",
            "E0404 `ctypes.sizeof` takes 1 positional argument, and 1 is given",
        ),
        ("ctypes.sizeof(missing)\n", "E0205"),
        ("ctypes.POINTER(c_int)(missing)\n", "E0205"),
        ("c_int(missing)\n", "E0205"),
        ("A = c_int * missing\n", "E0205"),
        (
            "from typing import Iterable\n\
             def f(values: Iterable[int]) -> None:\n    flag = ctypes.c_bool()\n    \
             flag.value = values\n",
            "E0304 the value of `c_bool` is assigned an Iterable[int], whose truth cannot be tested",
        ),
        (
            "x = (1 * 2)(3)\n",
            "E0201 calls of values that are not names are planned for TyPy 1.0 (FUN-05)",
        ),
    ] {
        let found = refused(source);
        assert!(
            found.iter().any(|each| each.starts_with(message)),
            "{source}: {found:?}"
        );
    }
}

#[test]
fn a_c_function_takes_an_object_by_reference_as_ctypes_does() {
    let declared = "libc.frexp.argtypes = [ctypes.c_double, ctypes.POINTER(c_int)]\n\
                    libc.frexp.restype = ctypes.c_double\n\
                    libc.memset.argtypes = [ctypes.c_void_p, c_int, ctypes.c_size_t]\n\
                    libc.memset.restype = None\n";
    assert_eq!(
        refused(&format!(
            "{declared}e = c_int()\na = (c_int * 2)()\n\
             libc.frexp(1.0, e)\nlibc.frexp(1.0, ctypes.byref(e))\nlibc.frexp(1.0, a)\n\
             libc.frexp(1.0, ctypes.pointer(e))\nlibc.frexp(1.0, None)\n\
             libc.memset(a, 0, 8)\nlibc.memset(ctypes.byref(e), 0, 4)\n\
             libc.memset(ctypes.pointer(e), 0, 4)\nlibc.memset(None, 0, 0)\n\
             import math\nlibc.abs(math.floor(2.5))\nlibc.abs(len(b'ab'))\n"
        )),
        Vec::<String>::new()
    );
    for (source, message) in [
        (
            "libc.frexp(1.0, ctypes.c_long(1))\n",
            "E0404 argument 2 of `libc.frexp` is a c_long, where its `argtypes` take a pointer to `c_int`",
        ),
        (
            "libc.frexp(1.0, 5)\n",
            "E0404 argument 2 of `libc.frexp` is an int, where its `argtypes` take a pointer",
        ),
        (
            "libc.frexp(1.0, ctypes.byref(ctypes.c_long(1)))\n",
            "E0404 argument 2 of `libc.frexp` is `byref` of a c_long, where",
        ),
        (
            "libc.memset(c_int(1), 0, 4)\n",
            "E0404 argument 1 of `libc.memset` is a c_int, where its `argtypes` take an `int`, \
             `bytes`, `None`, a pointer, an array or `byref` of an object",
        ),
        (
            "libc.abs(ctypes.byref(c_int(1)))\n",
            "E0404 argument 1 of `libc.abs` is `byref` of a c_int, where its `argtypes` take",
        ),
        (
            "libc.abs(ctypes.byref(ctypes.pointer(c_int(1))))\n",
            "E0201 `byref` of a pointer is planned for TyPy 1.0 (FFI-10)",
        ),
        (
            "libc.abs(ctypes.byref(5))\n",
            "E0404 `ctypes.byref` takes a `ctypes` object, and an int is given",
        ),
        (
            "libc.abs(ctypes.byref())\n",
            "E0404 `ctypes.byref` takes 1 positional argument, and 0 are given",
        ),
        (
            "def f() -> int:\n    byref = c_int\n    return libc.abs(byref(5))\n",
            "E0404 argument 1 of `libc.abs` is a c_int, where its `argtypes` take an `int`",
        ),
    ] {
        let found = refused(&format!("{declared}{source}"));
        assert!(
            found.iter().any(|each| each.starts_with(message)),
            "{source}: {found:?}"
        );
    }
}

#[test]
fn a_ctypes_object_is_made_read_and_written_as_ctypes_does() {
    let source = "x = c_int(5)\nx.value = True\nx.value += 1\n\
                  b = ctypes.c_bool([1])\nb.value = 'text'\n\
                  d = ctypes.c_double(1)\nd.value = 2\n\
                  s = ctypes.c_char_p()\ns.value = b'x'\ns.value = None\n\
                  v = ctypes.c_void_p(16)\nv.value = None\nv.value = True\n\
                  A = c_int * 3\na = A(1, 2)\na[2] = a[0] + len(a)\n\
                  p = ctypes.pointer(x)\np[0] = 3\np.contents.value = 4\nq = ctypes.POINTER(c_int)()\n\
                  r = ctypes.POINTER(c_int)(x)\n\
                  print(x, x.value, b.value, d.value, s.value, v.value, ctypes.sizeof(a), ctypes.sizeof(A))\n\
                  print(bool(q), x == x, x is x, [i for i in a], 2 in a, list(reversed(a)), r[0])\n\
                  def f() -> int:\n    return A(7)[0] + ctypes.sizeof(c_int)\n";
    assert_eq!(refused(source), Vec::<String>::new());
}
