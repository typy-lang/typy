//! Tests of checking unions of objects through the public API (TYP-14, TYP-07): what a union
//! accepts, how `isinstance` narrows it, and what is refused.

mod common;

use common::app;

const SHAPES: &str = concat!(
    "class Shape:\n",
    "    def __init__(self, name: str) -> None:\n",
    "        self.name = name\n",
    "class Circle(Shape):\n",
    "    pass\n",
    "class Square(Shape):\n",
    "    pass\n",
    "class Dot(Circle):\n",
    "    pass\n",
    "class Other:\n",
    "    pass\n",
);

fn with_shapes(body: &str) -> Vec<String> {
    app(&format!("{SHAPES}{body}"))
}

#[test]
fn isinstance_narrows_each_kind_of_union() {
    let cases = [
        // A tested subclass of a member narrows to that subclass.
        "def f(s: Circle | Square) -> str:\n    if isinstance(s, Dot):\n        return s.name\n    return ''\n",
        // An unrelated class leaves no member, and a class every member is leaves the union.
        "def f(s: Circle | Square) -> bool:\n    return isinstance(s, Other) or isinstance(s, Shape)\n",
        // The other branch keeps the optional members no class takes.
        "def f(s: Circle | Square | None) -> str:\n    if isinstance(s, Circle):\n        return s.name\n    if s is None:\n        return ''\n    return s.name\n",
        // A union of exceptions narrows to the tested class.
        "def f(e: ValueError | KeyError) -> str:\n    if isinstance(e, KeyError):\n        return str(e)\n    raise e\n",
        // A union of objects tested against a built-in type.
        "def f(v: str | list[int]) -> int:\n    if isinstance(v, (str, list)):\n        return 1\n    return 0\n",
        // An instance passed where a wider union, or the members' base, is expected.
        "def g(s: Shape) -> None:\n    pass\ndef f(s: Circle | Square, d: Dot) -> None:\n    g(s)\n    t: Circle | Square | Other = s\n    u: Circle | Square = d\n    print(t is u, type(t), type(s))\n",
        "def f(v: str | list[int] | None) -> None:\n    print(type(v), 1 if v else 2)\n",
        "def f(e: ValueError | KeyError) -> None:\n    raise e from e\n",
    ];
    for body in cases {
        assert_eq!(with_shapes(body), Vec::<String>::new(), "{body}");
    }
}

#[test]
fn operations_not_every_member_has_are_refused() {
    let cases: &[(&str, &str)] = &[
        ("x: int | str = 1.5\n", "E0402"),
        (
            "def f(v: str | list[int]) -> bool:\n    return v is v\n",
            "E0301",
        ),
        (
            "def f(v: str | bytes) -> bytes:\n    return v.encode()\n",
            "E0412",
        ),
        // Each member is checked, so the error names the member without the attribute.
        (
            "def f(s: Circle | Other) -> str:\n    return s.name\n",
            "E0414",
        ),
        ("x: str | bytes = [1]\n", "E0402"),
    ];
    for (body, code) in cases {
        assert_eq!(with_shapes(body), vec![code.to_string()], "{body}");
    }
}

#[test]
fn a_truth_test_of_a_union_asks_each_object_s_class() {
    let classes = concat!(
        "class Base:\n    pass\n",
        "class Full(Base):\n    def __len__(self) -> int:\n        return 1\n",
        "class Flag(Base):\n    def __bool__(self) -> bool:\n        return True\n",
        "class Plain(Base):\n    pass\n",
        "class Alone:\n    pass\n",
        "class Sized(Alone):\n    def __len__(self) -> int:\n        return 0\n",
    );
    // Each object's class decides, through its type object, also by `__len__` (CLS-24).
    let accepted = [
        "def f(v: Full | Flag) -> bool:\n    return not v\n",
        "def f(v: Sized | str) -> bool:\n    return bool(v)\n",
        "def f(v: Plain | Flag | None) -> bool:\n    return bool(v)\n",
        "def f(v: Alone | str) -> bool:\n    return bool(v)\n",
        "def f(v: list[Full | str]) -> bool:\n    return any(v)\n",
        "def f(v: Flag | str) -> bool:\n    return bool(v)\n",
        "def f(v: list[Plain | str]) -> bool:\n    return any(v)\n",
        "class Tested:\n    def __bool__(self) -> bool:\n        return False\nclass Left(Tested):\n    pass\nclass Right(Tested):\n    pass\ndef f(v: Left | Right) -> bool:\n    return bool(v)\n",
        "class Measured:\n    def __len__(self) -> int:\n        return 0\nclass Left(Measured):\n    pass\nclass Right(Measured):\n    pass\ndef f(v: Left | Right) -> bool:\n    return bool(v)\n",
        "def f(v: Plain | Full) -> bool:\n    return isinstance(v, Base)\n",
    ];
    for body in accepted {
        assert_eq!(
            app(&format!("{classes}{body}")),
            Vec::<String>::new(),
            "{body}"
        );
    }
}

#[test]
fn a_display_where_a_union_is_expected_takes_its_member() {
    let source = concat!(
        "def f(v: str | list[int] | dict[str, int] | set[str]) -> None:\n",
        "    print(v)\n",
        "f([])\n",
        "f({})\n",
        "f(set())\n",
        "f([n for n in range(2)])\n",
        "f('a')\n",
    );
    assert_eq!(app(source), Vec::<String>::new());
}

#[test]
fn an_operation_on_a_union_variable_is_checked_for_each_member() {
    let accepted = [
        // Two union operands split one inside the other.
        "def f(a: str | bytes, b: list[int] | set[int]) -> int:\n    return len(a) + len(b)\n",
        // A method every member has, whose result is a union of objects.
        "def f(v: str | bytes) -> None:\n    w = v.upper()\n    print(w)\n",
        // A union of classes that share a base reads the base's attribute without a split.
        "def f(s: Circle | Square) -> str:\n    return s.name\n",
        // A class no member is gives false, and the test narrows nothing.
        "def f(v: str | bytes) -> bool:\n    return isinstance(v, list)\n",
        // Code after a `return` is checked too.
        "def f(v: str | bytes) -> int:\n    return 0\n    return len(v)\n",
        // A variable already narrowed to a smaller union is split on what is left.
        "def f(v: str | bytes | list[int]) -> int:\n    if isinstance(v, list):\n        return 0\n    return len(v)\n",
    ];
    for body in accepted {
        assert_eq!(with_shapes(body), Vec::<String>::new(), "{body}");
    }
    let refused: &[(&str, &str)] = &[
        // An iterator has no class to test the object against.
        (
            "from collections.abc import Iterator\ndef f(v: Iterator[int] | str) -> int:\n    return len(v)\n",
            "E0201",
        ),
        // A variable a function declares `global` is not narrowed, so it is not split.
        (
            "g: str | bytes = 'a'\ndef f() -> None:\n    global g\n    g = b'b'\ndef h() -> int:\n    return len(g)\n",
            "E0404",
        ),
        // A union of exceptions is used through their common class, not split.
        (
            "def f(e: ValueError | KeyError) -> None:\n    print(e + e)\n",
            "E0401",
        ),
        // A union computed in place is not split.
        (
            "def f(v: str | bytes) -> int:\n    return len(v * 2)\n",
            "E0404",
        ),
        // A union computed in place is not compared.
        (
            "def f(v: str | bytes) -> bool:\n    return v * 2 == v * 2\n",
            "E0407",
        ),
        (
            "def f(v: str | bytes) -> bool:\n    return v * 2 < v * 2\n",
            "E0407",
        ),
    ];
    for (body, code) in refused {
        assert_eq!(with_shapes(body), vec![code.to_string()], "{body}");
    }
}
