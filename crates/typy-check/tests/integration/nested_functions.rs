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
            "def f() -> None:\n    def g() -> int:\n        return x\n    x = 3\n",
            "E0201",
            "`g` captures `x`, which a loop binds or which is assigned again or after the \
             `def`, and capturing such a variable is planned for later in TyPy 0.8 (FUN-04)",
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
