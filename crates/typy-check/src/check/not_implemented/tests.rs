//! Tests of `check::not_implemented`.

use camino::Utf8PathBuf;
use typy_diagnostics::FileId;
use typy_source::syntax::Stmt;
use typy_source::{PythonFloor, SourceFile, parse};

use super::{gives_way, returns_not_implemented};

fn body(source: &str) -> Vec<Stmt> {
    let file = SourceFile::from_text(FileId(0), Utf8PathBuf::from("t.py"), source);
    parse(&file, PythonFloor::default()).unwrap().body
}

#[test]
fn operator_and_comparison_methods_give_way_and_others_do_not() {
    for name in [
        "__eq__", "__ge__", "__add__", "__radd__", "__iadd__", "__rpow__",
    ] {
        assert!(gives_way(name), "{name}");
    }
    for name in [
        "__hash__",
        "__len__",
        "__neg__",
        "__contains__",
        "add",
        "__ipos__",
    ] {
        assert!(!gives_way(name), "{name}");
    }
}

#[test]
fn a_return_of_not_implemented_is_found_in_any_block_but_a_nested_definition() {
    let cases = [
        ("return NotImplemented\n", true),
        ("if x:\n    return NotImplemented\nreturn 1\n", true),
        (
            "for x in y:\n    pass\nelse:\n    return NotImplemented\n",
            true,
        ),
        ("while x:\n    return NotImplemented\n", true),
        ("with x:\n    return NotImplemented\n", true),
        (
            "try:\n    pass\nexcept E:\n    return NotImplemented\n",
            true,
        ),
        (
            "match x:\n    case 1:\n        return NotImplemented\n",
            true,
        ),
        ("def inner():\n    return NotImplemented\nreturn 1\n", false),
        ("return Missing\n", false),
        ("return\n", false),
        ("x = NotImplemented\nreturn x\n", false),
    ];
    for (source, expected) in cases {
        assert_eq!(returns_not_implemented(&body(source)), expected, "{source}");
    }
}
