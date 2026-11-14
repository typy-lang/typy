//! Tests of checking generic functions through the public API (GEN-01 to GEN-03, GEN-07):
//! how calls solve type parameters, and what is refused.

mod common;

use common::{app, codes};

#[test]
fn calls_solve_type_parameters_from_every_kind_of_argument() {
    let source = concat!(
        "from collections.abc import Iterable\n",
        "def keys[K, V](table: dict[K, V]) -> list[K]:\n",
        "    return list(table)\n",
        "def swap[A, B](pair: tuple[A, B]) -> tuple[B, A]:\n",
        "    return (pair[1], pair[0])\n",
        "def head[T](items: tuple[T, ...]) -> T:\n",
        "    return items[0]\n",
        "def total[T](items: Iterable[T]) -> list[T]:\n",
        "    return [item for item in items]\n",
        "def chosen[T](value: T | None, fallback: T) -> T:\n",
        "    return fallback if value is None else value\n",
        "def labelled[T](value: T | str) -> str:\n",
        "    return str(value)\n",
        "def outer[T](value: T) -> list[T]:\n",
        "    held: list[T] = [value]\n",
        "    return held + keep(held)\n",
        "def keep[T](items: list[T]) -> list[T]:\n",
        "    return items\n",
        "def mixed[N: (int, float), T](number: N, other: T) -> T:\n",
        "    print(number + number)\n",
        "    return other\n",
        "class Shape:\n",
        "    def area(self) -> float:\n",
        "        return 1.0\n",
        "def measure(shape: Shape) -> float:\n",
        "    return shape.area()\n",
        "def bounded[S: Shape](shape: S) -> float:\n",
        "    return measure(shape)\n",
        "print(keys({'a': 1}), swap((1, 'x')), head((1, 2)), total({3}), total(range(2)))\n",
        "print(chosen(None, 4), chosen(fallback='b', value='a'), labelled(1))\n",
        "print(outer(1), outer(2), mixed(1.5, 'z'), bounded(Shape()))\n",
    );
    assert_eq!(app(source), Vec::<String>::new());
}

#[test]
fn calls_that_give_no_fitting_type_are_refused() {
    let cases: &[(&str, &str)] = &[
        (
            "def f[T](x: T) -> T:\n    return x\nprint(f(missing))\n",
            "E0205",
        ),
        ("def f[T](x: T) -> T:\n    return x\nprint(f())\n", "E0404"),
        (
            "def f[T](x: T) -> T:\n    return x\nprint(f(1, 2))\n",
            "E0404",
        ),
        (
            "def f[T](x: list[T]) -> T:\n    return x[0]\nprint(f(1))\n",
            "E0404",
        ),
        (
            "def f[T](x: T) -> T:\n    return x\ny: str = f(1)\n",
            "E0402",
        ),
        ("def f[T](x: T) -> T:\n    return x < x\n", "E0407"),
        // A tuple of another length, a value that is not iterable, a union with two open
        // members, and a member that takes the argument itself solve nothing.
        (
            "def f[A, B](x: tuple[A, B]) -> A:\n    return x[0]\nprint(f((1, 2, 3)))\n",
            "E0404",
        ),
        (
            "from collections.abc import Iterable\ndef f[T](x: Iterable[T]) -> int:\n    return 0\nprint(f(1))\n",
            "E0404",
        ),
        (
            "def f[A, B](x: A | B) -> int:\n    return 0\nprint(f(1))\n",
            "E0404",
        ),
        (
            "def f[T](x: T | str) -> int:\n    return 0\nprint(f('s'))\n",
            "E0404",
        ),
        // Type parameters of methods and of aliases come with generic classes.
        (
            "class C:\n    def m[T](self, x: T) -> T:\n        return x\n",
            "E0201",
        ),
        ("type Pair[T] = tuple[T, T]\n", "E0201"),
    ];
    for (source, code) in cases {
        let found = app(source);
        assert_eq!(
            found.first().map(String::as_str),
            Some(*code),
            "{source}\n{found:?}"
        );
    }
}

#[test]
fn a_generic_of_another_module_is_planned() {
    let found = codes(&[
        ("app.py", "from tools import first\nprint(first([1]))\n"),
        (
            "tools.py",
            "def first[T](items: list[T]) -> T:\n    return items[0]\n",
        ),
    ]);
    assert_eq!(found, vec!["E0201".to_string()]);
}
