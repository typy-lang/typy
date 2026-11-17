//! Tests of checking protocols through the public API (GEN-04, GEN-05, GEN-10): which
//! classes satisfy a protocol, and where a protocol may be named.

mod common;

use common::app;

/// A protocol and two classes that satisfy it, each test's program starts with.
const SHAPES: &str = concat!(
    "from typing import Protocol\n",
    "import typing\n",
    "class Shape(Protocol):\n",
    "    sides: int\n",
    "    def area(self) -> float: ...\n",
    "    def scaled(self, factor: float) -> float:\n",
    "        ...\n",
    "class Named(typing.Protocol):\n",
    "    def name(self) -> str: ...\n",
    "class Base:\n",
    "    def __init__(self) -> None:\n",
    "        self.sides = 4\n",
    "    def area(self) -> float:\n",
    "        return 1.0\n",
    "class Square(Base):\n",
    "    def scaled(self, factor: float, extra: float = 1.0) -> float:\n",
    "        return factor * extra\n",
    "    def name(self) -> str:\n",
    "        return 'square'\n",
);

#[test]
fn a_class_with_the_members_of_a_protocol_satisfies_it() {
    let program = concat!(
        "def describe(shape: Shape, named: Named) -> str:\n",
        "    return named.name() + str(shape.area() + shape.sides)\n",
        "def total[S: Shape](shapes: list[S]) -> float:\n",
        "    return sum([shape.scaled(2.0) for shape in shapes], 0.0)\n",
        "def first(shapes: list[Shape]) -> float:\n",
        "    return shapes[0].area()\n",
        "class Report:\n",
        "    def line(self, shape: Shape) -> str:\n",
        "        return describe(shape, Square())\n",
        "    @staticmethod\n",
        "    def area_of(shape: Shape) -> float:\n",
        "        return shape.area()\n",
        "...\n",
        "print(describe(Square(), Square()), total([Square()]), first([Square()]))\n",
        "print(Report().line(Square()), Report.area_of(Square()))\n",
    );
    assert_eq!(app(&format!("{SHAPES}{program}")), Vec::<String>::new());
}

#[test]
fn a_class_without_a_member_of_the_protocol_is_refused() {
    let cases: &[(&str, &str)] = &[
        // No method, a static method, another parameter name, another type, and a
        // parameter the protocol's method does not take, without a default.
        ("class C:\n    sides = 1\n", "E0404"),
        (
            "class C(Base):\n    @staticmethod\n    def scaled(factor: float) -> float:\n        return factor\n",
            "E0404",
        ),
        (
            "class C(Base):\n    def scaled(self, size: float) -> float:\n        return size\n",
            "E0404",
        ),
        (
            "class C(Base):\n    def scaled(self, factor: int) -> float:\n        return 1.0\n",
            "E0404",
        ),
        (
            "class C(Base):\n    def scaled(self, factor: float, extra: float) -> float:\n        return factor\n",
            "E0404",
        ),
        (
            "class C(Base):\n    def scaled(self) -> float:\n        return 1.0\n",
            "E0404",
        ),
        (
            "class C(Base):\n    def scaled(self, factor: float) -> str:\n        return ''\n",
            "E0404",
        ),
        // No attribute, and an attribute of another type.
        (
            "class C:\n    def area(self) -> float:\n        return 1.0\n    def scaled(self, factor: float) -> float:\n        return 1.0\n",
            "E0404",
        ),
        (
            "class C:\n    def __init__(self) -> None:\n        self.sides = 'four'\n    def area(self) -> float:\n        return 1.0\n    def scaled(self, factor: float) -> float:\n        return 1.0\n",
            "E0404",
        ),
    ];
    for (class, code) in cases {
        let program = format!("{SHAPES}{class}def f(shape: Shape) -> None:\n    pass\nf(C())\n");
        assert_eq!(app(&program), vec![code.to_string()], "{class}");
    }
}

#[test]
fn a_value_that_is_not_an_instance_does_not_satisfy_a_protocol() {
    let cases = [
        "def f(shape: Shape) -> None:\n    pass\nf(1)\n",
        // A copy checks what its type argument gives.
        "def g(shape: Shape) -> None:\n    pass\ndef f[T](value: T) -> None:\n    g(value)\nf(1)\n",
    ];
    for program in cases {
        assert_eq!(
            app(&format!("{SHAPES}{program}")),
            vec!["E0404".to_string()],
            "{program}"
        );
    }
}

#[test]
fn a_protocol_is_refused_where_typy_does_not_hold_one() {
    let cases: &[(&str, &str)] = &[
        // GEN-05 and calling the protocol.
        ("print(isinstance(Square(), Shape))\n", "E0401"),
        ("Shape()\n", "E0401"),
        ("x: Protocol = Square()\n", "E0408"),
        // GEN-10: a value of a class that does not satisfy the protocol.
        ("def f() -> Shape:\n    return Base()\n", "E0405"),
        ("x: list[Shape] = [Square(), Base()]\n", "E0409"),
        ("x: Shape = Square()\nx.sides = 3\n", "E0201"),
        // GEN-17: a base, members, and the protocol in its own signatures.
        ("class C(Shape):\n    pass\n", "E0201"),
        (
            "class P(Protocol):\n    @property\n    def size(self) -> int: ...\n",
            "E0201",
        ),
        ("class P(Protocol):\n    size: int = 0\n", "E0201"),
        ("class P(Protocol):\n    size = 0\n", "E0201"),
        (
            "class P(Protocol):\n    def merge(self, other: Shape) -> None: ...\n",
            "E0201",
        ),
        // GEN-13: a generic protocol.
        (
            "class P[T](Protocol):\n    def get(self) -> T: ...\n",
            "E0201",
        ),
    ];
    for (program, code) in cases {
        let found = app(&format!("{SHAPES}{program}"));
        assert_eq!(
            found.first().map(String::as_str),
            Some(*code),
            "{program}\n{found:?}"
        );
    }
}

#[test]
fn a_protocol_types_values_of_the_classes_that_satisfy_it() {
    let cases = [
        "def f() -> Shape:\n    return Square()\nprint(f().area() + f().scaled(2.0))\n",
        "x: list[Shape] = [Square(), Square()]\nprint(x[0].sides + len(x))\n",
        "class C:\n    def __init__(self, shape: Shape) -> None:\n        self.shape = shape\nprint(C(Square()).shape.area())\n",
        "x: dict[str, Shape] = {'a': Square()}\ny: Shape | None = None\nprint(x['a'] == x['a'], y is None)\n",
        "x: Shape = Square()\nif isinstance(x, Square):\n    print(x.name())\n",
    ];
    for program in cases {
        assert_eq!(
            app(&format!("{SHAPES}{program}")),
            Vec::<String>::new(),
            "{program}"
        );
    }
}

#[test]
fn a_protocol_value_meets_what_it_does_not_hold() {
    let cases: &[(&str, &[&str])] = &[
        // An exception class is never a protocol value.
        (
            "class Failure(Exception):\n    pass\nx: Shape = Square()\n",
            &[],
        ),
        ("x: Shape = Square()\nprint(x.missing)\n", &["E0414"]),
        (
            "def f(s: Square | None) -> Shape:\n    return s\n",
            &["E0405"],
        ),
        ("def f(s: Named) -> Shape:\n    return s\n", &["E0405"]),
        ("x: Shape = 'text'\n", &["E0402"]),
    ];
    for (program, expected) in cases {
        assert_eq!(app(&format!("{SHAPES}{program}")), *expected, "{program}");
    }
}

#[test]
fn a_parameter_with_a_default_in_the_protocol_needs_one_in_the_class() {
    let program = concat!(
        "from typing import Protocol\n",
        "class Sized(Protocol):\n",
        "    def size(self, scale: int = 1) -> int: ...\n",
        "class Kept:\n",
        "    def size(self, scale: int = 2) -> int:\n",
        "        return scale\n",
        "class Lacking:\n",
        "    def size(self, scale: int) -> int:\n",
        "        return scale\n",
        "def measure[S: Sized](item: S) -> int:\n",
        "    return item.size()\n",
        "def through(item: Sized) -> int:\n",
        "    return measure(item)\n",
        "print(measure(Kept()), through(Kept()))\n",
    );
    assert_eq!(app(program), Vec::<String>::new());
    let refused = format!("{program}print(measure(Lacking()))\n");
    assert_eq!(app(&refused), ["E0404"]);
}
