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
        ("x: int | str = 1\n", "E0201"),
        ("x: tuple[int, int] | str = 'a'\n", "E0201"),
        (
            "def f(v: str | list[int]) -> bool:\n    return v is v\n",
            "E0301",
        ),
        (
            "def f(v: str | bytes) -> bytes:\n    return v.encode()\n",
            "E0412",
        ),
        (
            "def f(s: Circle | Other) -> str:\n    return s.name\n",
            "E0412",
        ),
        ("x: str | bytes = [1]\n", "E0402"),
    ];
    for (body, code) in cases {
        assert_eq!(with_shapes(body), vec![code.to_string()], "{body}");
    }
}

#[test]
fn a_truth_test_the_object_types_cannot_answer_is_planned() {
    let classes = concat!(
        "class Base:\n    pass\n",
        "class Full(Base):\n    def __len__(self) -> int:\n        return 1\n",
        "class Flag(Base):\n    def __bool__(self) -> bool:\n        return True\n",
        "class Plain(Base):\n    pass\n",
        "class Alone:\n    pass\n",
        "class Sized(Alone):\n    def __len__(self) -> int:\n        return 0\n",
    );
    let planned = [
        "def f(v: Full | Flag) -> bool:\n    return not v\n",
        "def f(v: Plain | Flag | None) -> bool:\n    return bool(v)\n",
        "def f(v: Alone | str) -> bool:\n    return bool(v)\n",
        "def f(v: list[Full | str]) -> bool:\n    return any(v)\n",
    ];
    for body in planned {
        assert_eq!(
            app(&format!("{classes}{body}")),
            vec!["E0201".to_string()],
            "{body}"
        );
    }
    let accepted = [
        "def f(v: Flag | str) -> bool:\n    return bool(v)\n",
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
