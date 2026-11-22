//! Values of type `object` (TYP-23, ADR-0090): any value passes as one, and a test narrows it
//! before any operation `object` does not define.

use crate::common::{app, messages};

#[test]
fn typ_23_every_value_passes_as_an_object() {
    let source = concat!(
        "class Box:\n",
        "    def __init__(self, value: object) -> None:\n",
        "        self.value: object = value\n",
        "def make(flag: bool) -> object:\n",
        "    if flag:\n",
        "        return 1\n",
        "    return 'one'\n",
        "items: list[object] = [1, 'a', 2.5, None, True, (1, 'b'), [1], Box(3)]\n",
        "items.append(make(True))\n",
        "record: dict[str, object] = {'name': 'ada', 'age': 36}\n",
        "pair: tuple[int, object] = (1, None)\n",
        "maybe: object | None = None\n",
        "print(items, record, pair, maybe, Box(2.5).value)\n",
    );
    assert_eq!(app(source), Vec::<String>::new());
}

#[test]
fn typ_23_an_object_compares_writes_and_tests_its_truth_without_narrowing() {
    let source = concat!(
        "x: object = 3\n",
        "y: object = 'a'\n",
        "print(x == 3, x != y, x is y, x is None, str(x), repr(y), f'{x}', bool(x))\n",
        "print(type(x), 3 in [x], isinstance(x, int))\n",
        "if x:\n",
        "    print('true')\n",
    );
    assert_eq!(app(source), Vec::<String>::new());
}

#[test]
fn typ_23_isinstance_a_class_pattern_and_type_is_narrow_an_object() {
    let source = concat!(
        "class Point:\n",
        "    def __init__(self, x: int) -> None:\n",
        "        self.x = x\n",
        "    def __eq__(self, other: object) -> bool:\n",
        "        return isinstance(other, Point) and self.x == other.x\n",
        "def describe(value: object) -> str:\n",
        "    if isinstance(value, str):\n",
        "        return value.upper()\n",
        "    if type(value) is int:\n",
        "        return str(value + 1)\n",
        "    if isinstance(value, float):\n",
        "        return str(value * 2.0)\n",
        "    match value:\n",
        "        case Point():\n",
        "            return str(value.x)\n",
        "        case None:\n",
        "            return 'none'\n",
        "        case _:\n",
        "            return 'other'\n",
        "print(describe('a'), describe(1), describe(1.5), describe(Point(2)), describe(None))\n",
        "print(Point(1) == Point(1), Point(1) == 3)\n",
    );
    assert_eq!(app(source), Vec::<String>::new());
}

#[test]
fn typ_23_isinstance_int_keeps_a_bool_a_bool() {
    let found = messages(concat!(
        "def double(value: object) -> object:\n",
        "    if isinstance(value, int):\n",
        "        return value\n",
        "    return 0\n",
        "x: object = True\n",
        "y: int | bool = 0\n",
        "if isinstance(x, int):\n",
        "    y = x\n",
        "print(double(x), y)\n",
    ));
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn typ_23_operations_an_object_does_not_define_are_refused_until_it_is_narrowed() {
    for (statement, code) in [
        ("print(x + 1)", "E0401"),
        ("print(x < 4)", "E0407"),
        ("print(x.upper())", "E0412"),
        ("for c in x:\n    print(c)", "E0411"),
        ("y: int = x", "E0402"),
        ("print(sorted([x, x]))", "E0407"),
    ] {
        let found = messages(&format!("x: object = 3\n{statement}\n"));
        assert_eq!(found.len(), 1, "{statement}: {found:?}");
        assert_eq!(found[0].0, code, "{statement}");
    }
}

#[test]
fn typ_23_narrowing_an_object_to_a_collection_waits_for_any() {
    let found =
        messages("def f(x: object) -> None:\n    if isinstance(x, list):\n        print(x)\n");
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, "E0201");
    assert_eq!(found[0].2, "TYP-25");
    assert!(
        found[0].1.contains("narrowing an `object` to `list`"),
        "{found:?}"
    );
}

#[test]
fn typ_23_isinstance_narrows_an_object_to_each_kind_of_class() {
    let source = concat!(
        "class Box[T]:\n",
        "    def __init__(self, item: T) -> None:\n",
        "        self.item = item\n",
        "def describe(value: object) -> str:\n",
        "    if isinstance(value, float):\n",
        "        return str(value + 0.5)\n",
        "    if isinstance(value, bool):\n",
        "        return str(not value)\n",
        "    if isinstance(value, bytes):\n",
        "        return str(len(value))\n",
        "    if isinstance(value, type(None)):\n",
        "        return 'none'\n",
        "    if isinstance(value, type):\n",
        "        return value.__name__\n",
        "    if isinstance(value, range):\n",
        "        return str(len(value))\n",
        "    if isinstance(value, slice):\n",
        "        return str(value.start)\n",
        "    if isinstance(value, ValueError):\n",
        "        return str(value)\n",
        "    if isinstance(value, Box):\n",
        "        return 'box'\n",
        "    if isinstance(value, object):\n",
        "        return 'object'\n",
        "    return 'other'\n",
        "print(describe(1.5), describe(True), describe(b'ab'), describe(None), describe(int))\n",
        "print(describe(range(3)), describe(slice(1, 2)), describe(ValueError('x')), describe(Box(1)))\n",
    );
    assert_eq!(app(source), Vec::<String>::new());
}

#[test]
fn typ_23_type_is_narrows_an_object_to_each_kind_of_class() {
    let source = concat!(
        "def describe(value: object) -> str:\n",
        "    if type(value) is str:\n",
        "        return value.upper()\n",
        "    if type(value) is not float:\n",
        "        return 'not a float'\n",
        "    return str(value)\n",
        "print(describe('a'), describe(1), describe(2.5))\n",
    );
    assert_eq!(app(source), Vec::<String>::new());
    let found = messages(
        "def f(value: object) -> None:\n    if type(value) is dict:\n        print(value)\n",
    );
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].2, "TYP-25");
}
