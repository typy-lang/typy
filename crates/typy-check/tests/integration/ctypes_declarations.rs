//! The `ctypes` declarations a module makes: libraries, function types and structures.

use crate::common::{app, messages};

#[test]
fn declared_foreign_functions_are_accepted() {
    let accepted = [
        "from ctypes import CDLL, c_int\n\nlibc = CDLL(None)\nlibc.abs.argtypes = [c_int]\n\
        libc.abs.restype = c_int\nprint(libc.abs(-3))\n",
    ];
    for source in accepted {
        assert_eq!(app(source), Vec::<String>::new(), "{source}");
    }
}

#[test]
fn a_declaration_ctypes_reads_otherwise_is_refused() {
    let refused = [
        (
            "import ctypes\n\n\nclass Point(ctypes.Structure):\n    _pack_ = 1\n    \
            _fields_ = [(\"x\", ctypes.c_int)]\n",
            "E0201",
            "a `Structure` subclass with methods or class attributes other than one \
            `_fields_` is planned for TyPy 0.13 (FFI-11)",
        ),
        (
            "import ctypes\n\n\nclass Point(ctypes.Structure):\n    _fields_ = [(\"x\",)]\n",
            "E0430",
            "each item of `_fields_` is a `(name, type)` pair",
        ),
        (
            "import ctypes\n\nlibc = ctypes.pydll.LoadLibrary(None)\n",
            "E0201",
            "`ctypes.pydll` is planned for TyPy 0.13 (FFI-10)",
        ),
    ];
    for (source, code, message) in refused {
        let found = messages(source);
        assert!(
            found
                .iter()
                .any(|(each, text, _)| each == code && text == message),
            "{source}\n{found:?}"
        );
    }
}
