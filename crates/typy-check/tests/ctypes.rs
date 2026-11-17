//! Tests of how the checker types `ctypes` declarations and calls, through the public API:
//! the program it gives, and the diagnostics of each use TyPy does not compile (FFI-01 to
//! FFI-03, FFI-09, ADR-0047).

mod common;

use camino::Utf8PathBuf;
use typy_check::{CType, TypeChecker};
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
    assert_eq!(program.foreign_functions[0].parameters, [CType::Int32]);
    assert_eq!(program.foreign_functions[0].result, Some(CType::Int32));
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
            "x = ctypes.c_int(5)\n",
            "E0201 a `ctypes` type other than in a function's `argtypes` or `restype` is planned",
        ),
        (
            "x = c_int\n",
            "E0201 a `ctypes` type other than in a function's `argtypes` or `restype` is planned",
        ),
        (
            "x = ctypes.POINTER\n",
            "E0201 `ctypes.POINTER` other than in a function's `argtypes` or `restype` is planned",
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
