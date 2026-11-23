//! `TypeGuard[T]` and `TypeIs[T]`: a test of a call narrows its first argument, a variable,
//! to `T` where it holds, and for `TypeIs` to the rest where it fails (TYP-31, ADR-0114).

use crate::common::{messages, messages_at};
use typy_source::PythonFloor;

/// Returns the diagnostics of `source` at the Python floor 3.13, which `TypeIs` needs.
fn at_3_13(source: &str) -> Vec<(String, String, String)> {
    messages_at(source, PythonFloor::V3_13)
}

const GUARDS: &str = "from typing import TypeGuard, TypeIs\n\n\nclass Shape:\n    pass\n\n\n\
                      class Circle(Shape):\n    def label(self) -> str:\n        \
                      return 'circle'\n\n\ndef is_text(value: object) -> TypeIs[str]:\n    \
                      return isinstance(value, str)\n\n\n\
                      def is_circle(shape: Shape) -> TypeGuard[Circle]:\n    \
                      return isinstance(shape, Circle)\n\n\n";

#[test]
fn typ_31_a_guard_narrows_its_argument_where_it_is_tested() {
    let accepted = [
        format!(
            "{GUARDS}def f(value: int | str) -> str:\n    if is_text(value):\n        \
             return value.upper()\n    return str(value + 1)\n\n\nprint(f(1))\n"
        ),
        format!(
            "{GUARDS}def f(shape: Shape) -> str:\n    if is_circle(shape):\n        \
             return shape.label()\n    return 'shape'\n\n\nprint(f(Circle()))\n"
        ),
        // A guard of `T | None`, and a call whose argument is no variable, which is a `bool`.
        "from typing import TypeIs\n\n\ndef is_small(value: object) -> TypeIs[int | None]:\n    \
         return value is None or isinstance(value, int)\n\n\n\
         def f(value: int | str | None) -> str:\n    if is_small(value):\n        \
         return str(value)\n    return value.upper()\n\n\nprint(f('a'), is_small(3 + 1))\n"
            .to_owned(),
    ];
    for source in accepted {
        assert_eq!(at_3_13(&source), Vec::new(), "{source}");
    }
}

#[test]
fn typ_31_a_type_guard_narrows_nothing_where_it_fails() {
    let found = at_3_13(&format!(
        "{GUARDS}def f(shape: Shape) -> str:\n    if not is_circle(shape):\n        \
         return shape.label()\n    return 'circle'\n"
    ));
    assert!(found.iter().any(|(code, ..)| code == "E0414"), "{found:?}");
}

#[test]
fn typ_31_forms_a_later_release_gives_are_planned() {
    let cases = [
        "from typing import TypeGuard\n\n\ndef f(value: object) -> TypeGuard[list[int]]:\n    \
         return True\n",
        "from typing import TypeIs\n\n\nclass C:\n    def f(self, value: object) -> TypeIs[int]:\n        \
         return True\n",
        // A guard of a protocol or of a copy of a generic class, which no test of a class
        // tells apart.
        "from typing import Protocol, TypeGuard\n\n\nclass P(Protocol):\n    def f(self) -> int: ...\n\n\n\
         def g(value: object) -> TypeGuard[P]:\n    return True\n",
        "from typing import TypeGuard\n\n\nclass Box[T]:\n    def __init__(self, item: T) -> None:\n        \
         self.item = item\n\n\ndef g(value: object) -> TypeGuard[Box[int]]:\n    return True\n",
    ];
    for source in cases {
        let found = at_3_13(source);
        assert!(
            found
                .iter()
                .any(|(code, _, rule)| code == "E0201" && rule == "TYP-37"),
            "{source}\n{found:?}"
        );
    }
}

#[test]
fn typ_31_a_guard_is_written_as_a_result() {
    let cases = [
        "from typing import TypeIs\n\n\ndef f(value: TypeIs[int]) -> bool:\n    return True\n",
        "from typing import TypeIs\n\n\ndef f(value: object) -> TypeIs:\n    return True\n",
    ];
    for source in cases {
        let found = at_3_13(source);
        assert!(
            found
                .iter()
                .any(|(code, _, rule)| code == "E0408" && rule == "TYP-31"),
            "{source}\n{found:?}"
        );
    }
}

#[test]
fn typ_31_type_is_comes_with_the_python_floor_3_13() {
    let found = messages("from typing import TypeIs\n");
    assert_eq!(
        found,
        [(
            "E0425".to_owned(),
            "module `typing` has no name `TypeIs` before Python 3.13".to_owned(),
            "TYP-31".to_owned()
        )]
    );
    assert_eq!(messages("from typing import TypeGuard\n"), Vec::new());
}

#[test]
fn typ_31_a_guard_call_the_checker_refuses_or_decides_reports_alone() {
    // Arguments the guard does not take give their error alone.
    let found = at_3_13(&format!("{GUARDS}print(is_text(1, 2))\n"));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, "E0404", "{found:?}");
    // An argument of the guard's own type needs no test when the program runs.
    let found = at_3_13(&format!(
        "{GUARDS}def f(value: str) -> str:\n    if is_text(value):\n        \
         return value\n    return ''\n\n\nprint(f('a'))\n"
    ));
    assert_eq!(found, Vec::new());
}
