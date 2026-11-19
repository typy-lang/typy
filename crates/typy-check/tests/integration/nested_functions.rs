//! Nested functions and the variables they capture (FUN-04, ADR-0061).

use crate::common::{app, messages};

#[test]
fn fun_04_a_nested_function_reads_its_captures_its_own_name_and_its_siblings() {
    for source in [
        "def outer(step: int) -> int:\n    def add(value: int) -> int:\n        return value + step\n    \
         return add(1)\n",
        "def outer() -> int:\n    def down(n: int) -> int:\n        return 0 if n == 0 else down(n - 1)\n    \
         return down(3)\n",
        "def outer() -> int:\n    def base() -> int:\n        return 1\n    def twice() -> int:\n        \
         return base() * 2\n    return twice()\n",
        "def outer() -> int:\n    def scale(value: int, *, by: int = 2) -> int:\n        \
         return value * by\n    return scale(1, by=3)\n",
        "class Box:\n    def __init__(self) -> None:\n        self.size = 2\n\n    \
         def area(self) -> int:\n        def side() -> int:\n            return self.size\n        \
         return side() * side()\n",
        "def outer(base: int) -> int:\n    def middle() -> int:\n        def inner() -> int:\n            \
         return base\n        return inner()\n    return middle()\n",
    ] {
        assert_eq!(app(source), Vec::<String>::new(), "{source}");
    }
}

#[test]
fn fun_04_the_typed_tree_shows_the_def_its_captures_and_the_direct_calls() {
    let dump = crate::common::dump(
        "def outer(step: int) -> int:\n    def add(value: int) -> int:\n        \
         return value + step + add(0) * 0\n    return add(1)\n",
    );
    for line in [
        "FunctionDef outer.<locals>.add as add",
        "Local step : int",
        "Capture 0 : int",
        "CallNested outer.<locals>.add : int",
        "RunningFunction : Callable[[int], int]",
    ] {
        assert!(dump.contains(line), "{line} missing from\n{dump}");
    }
}

#[test]
fn fun_04_a_variable_that_may_change_after_the_def_is_shared_through_its_cell() {
    let dump = crate::common::dump(
        "def outer() -> int:\n    total = 1\n    def read() -> int:\n        def deeper() -> int:\n            \
         return total\n        return total + deeper()\n    total = 2\n    return read() + total\n",
    );
    for line in [
        "FunctionDef outer.<locals>.read as read",
        "CellObject total : int",
        "CellRead total : int",
        "CapturedCell 0 : int",
        "CapturedCellObject 0 : int",
    ] {
        assert!(dump.contains(line), "{line} missing from\n{dump}");
    }
    for source in [
        "def f() -> list[int]:\n    out: list[int] = []\n    for i in range(3):\n        def g() -> int:\n            \
         return i\n        out.append(g())\n    return out\n",
        "def f() -> str:\n    try:\n        raise ValueError(\"x\")\n    except ValueError as e:\n        \
         def g() -> str:\n            return str(e)\n        return g()\n",
        "def f(step: int) -> int:\n    def g() -> int:\n        return step\n    step += 1\n    return g()\n",
        "def f() -> int:\n    def a() -> int:\n        return b()\n    def b() -> int:\n        return 1\n    \
         return a()\n",
        // A `def` no path reaches is checked as any other.
        "def f() -> int:\n    return 1\n    def g() -> int:\n        return 2\n",
    ] {
        assert_eq!(app(source), Vec::<String>::new(), "{source}");
    }
}

#[test]
fn fun_04_a_nested_function_without_one_binding_or_one_type_is_refused() {
    let cases = [
        (
            "def f() -> None:\n    def g() -> int:\n        return 1\n    g = 3\n",
            "E0202",
            "`g` is a function and is also assigned as a variable",
        ),
        (
            "def f() -> int:\n    x = g()\n    def g() -> int:\n        return 1\n    return x\n",
            "E0306",
            "`g` may be used before it is assigned",
        ),
        (
            "def f() -> None:\n    def g(*, k: int) -> int:\n        return k\n    h = g\n",
            "E0202",
            "`g` takes `k` by keyword only, and a `Callable` passes every argument by \
             position, so `g` has no `Callable` type",
        ),
        (
            "def f[T](value: T) -> T:\n    def g() -> T:\n        return value\n    return g()\n",
            "E0201",
            "a nested function with type parameters, or inside a generic function, is planned \
             for TyPy 0.9 (FUN-10)",
        ),
        (
            "def f() -> None:\n    def g[T](value: T) -> T:\n        return value\n",
            "E0201",
            "a nested function with type parameters, or inside a generic function, is planned \
             for TyPy 0.9 (FUN-10)",
        ),
        (
            "def f() -> None:\n    g = 1\n    def g() -> int:\n        return 1\n",
            "E0202",
            "`g` is a function and is also assigned as a variable",
        ),
        (
            "def f() -> int:\n    return 1\n    def g[T](value: T) -> T:\n        return value\n",
            "E0201",
            "a nested function with type parameters, or inside a generic function, is planned \
             for TyPy 0.9 (FUN-10)",
        ),
        (
            "from typing import Callable\n\n\ndef f(g: Callable[[int, int], int]) -> int:\n    \
             return g(1)\n",
            "E0404",
            "a value of type Callable[[int, int], int] takes 2 arguments, and 1 is given",
        ),
        (
            "def two(a: str, b: str) -> int:\n    return 0\n\n\nprint(sorted([\"a\"], key=two))\n",
            "E0404",
            "the `key` of `sorted` is called with each item, of type str, and this is a \
             Callable[[str, str], int]",
        ),
        (
            "def f() -> None:\n    count = 1\n    def g() -> None:\n        count()\n",
            "E0401",
            "`count` is an int and cannot be called",
        ),
    ];
    for (source, code, message) in cases {
        let found = messages(source);
        assert!(
            found
                .iter()
                .any(|(each, text, _)| each == code && text == message),
            "{source}: {found:?}"
        );
    }
}

#[test]
fn fun_04_functions_that_call_one_another_by_name_share_the_first_ones_captures() {
    let dump = crate::common::dump(
        "def outer(limit: int | None) -> bool:\n    assert limit is not None\n    \
         def even(k: int) -> bool:\n        return k == 0 or odd(k - 1)\n    \
         def odd(k: int) -> bool:\n        return k < limit and even(k - 1)\n    \
         return even(4)\n",
    );
    for line in [
        "FunctionDef outer.<locals>.even as even",
        "FunctionDef outer.<locals>.odd as odd",
        // The later function copies the first one's capture, of its narrowed type.
        "CaptureOf outer.<locals>.even 0 : int",
        "CallNested outer.<locals>.odd : bool",
        "CallNested outer.<locals>.even : bool",
        "RunningFunction : Callable[[int], bool]",
    ] {
        assert!(dump.contains(line), "{line} missing from\n{dump}");
    }
    assert!(
        !dump.contains("CellObject"),
        "no function lives in a cell\n{dump}"
    );
}

#[test]
fn fun_04_a_refused_function_of_a_group_gives_one_error() {
    let found = messages(
        "def f(n: int) -> int:\n    def a(k: int) -> int:\n        return 0 if k == 0 else b(k - 1)\n    \
         def b[T](k: T) -> T:\n        return a(k)\n    return a(n)\n",
    );
    let codes: Vec<&str> = found.iter().map(|(code, _, _)| code.as_str()).collect();
    assert_eq!(codes, ["E0201"], "{found:?}");
}
