//! Tests of checking `match` statements through the public API (CST-06): what a pattern
//! narrows, which cases always match, and the one error each mistake gives.

mod common;

use common::app;

#[test]
fn patterns_narrow_their_subject_and_parts() {
    let cases = [
        // `case None:` leaves the later cases an `int`.
        "def f(v: int | None) -> int:\n    match v:\n        case None:\n            return 0\n        case n:\n            return n + 1\n",
        // A sequence pattern on an optional list tests for `None` first.
        "def f(v: list[int] | None) -> int:\n    match v:\n        case [a, b]:\n            return a + b\n        case _:\n            return 0\n",
        // A union narrows to the member a sequence or mapping pattern takes.
        "def f(v: int | list[int] | dict[str, int]) -> int:\n    match v:\n        case [a]:\n            return a\n        case {'k': b}:\n            return b\n        case _:\n            return 0\n",
        // An optional item is stored and narrowed by the nested pattern.
        "def f(v: list[int | None]) -> int:\n    match v:\n        case [None, b]:\n            return 0\n        case [a, _] if a is not None:\n            return a\n        case _:\n            return 1\n",
        // `[*rest]` matches every list, so the function cannot reach its end.
        "def f(v: list[int]) -> int:\n    match v:\n        case [*rest]:\n            return len(rest)\n",
    ];
    for source in cases {
        assert_eq!(app(source), Vec::<String>::new(), "{source}");
    }
}

#[test]
fn each_mistake_in_a_pattern_gives_one_error() {
    let cases: &[(&str, &[&str])] = &[
        // The subject's own error is the only one; the captured names are not reported.
        (
            "match missing:\n    case [a, *b]:\n        print(a, b)\n",
            &["E0205"],
        ),
        (
            "match 1:\n    case {'k': a, **b}:\n        print(a, b)\n",
            &["E0407"],
        ),
        (
            "match (1, 2):\n    case (a, b, c):\n        print(a, b, c)\n",
            &["E0407"],
        ),
        (
            "def f(v: list[int] | tuple[int, int]) -> None:\n    match v:\n        case [a, b]:\n            print(a, b)\n",
            &["E0201"],
        ),
    ];
    for (source, expected) in cases {
        assert_eq!(app(source), *expected, "{source}");
    }
}

const POINT: &str = "class Point:\n    __match_args__ = ('x', 'y')\n    def __init__(self, x: int, y: int) -> None:\n        self.x = x\n        self.y = y\n";

#[test]
fn class_patterns_take_match_args_from_the_class_or_its_base() {
    let cases = [
        // A subclass without `__match_args__` takes its base's.
        "class Point3(Point):\n    def __init__(self, x: int, y: int) -> None:\n        super().__init__(x, y)\ndef f(p: Point3) -> int:\n    match p:\n        case Point3(a, b):\n            return a + b\n    return 0\n",
        // A dataclass gets its fields as `__match_args__`.
        "from dataclasses import dataclass\n@dataclass\nclass Pair:\n    left: int\n    right: str\ndef f(p: Pair) -> str:\n    match p:\n        case Pair(n, text):\n            return text * n\n    return ''\n",
        // A generic dataclass gets no class variable before GEN-08, but its fields still
        // name the positional sub-patterns.
        "from dataclasses import dataclass\n@dataclass\nclass Box[T]:\n    item: T\ndef f(b: Box[int]) -> int:\n    match b:\n        case Box(n):\n            return n\n    return 0\n",
        // A dataclass may write its own.
        "from dataclasses import dataclass\n@dataclass\nclass Pair:\n    left: int\n    right: str\n    __match_args__ = ('right',)\ndef f(p: Pair) -> str:\n    match p:\n        case Pair(text):\n            return text\n    return ''\n",
    ];
    for source in cases {
        assert_eq!(
            app(&format!("{POINT}{source}")),
            Vec::<String>::new(),
            "{source}"
        );
    }
}

#[test]
fn class_patterns_that_cannot_be_checked_give_one_error() {
    let cases: &[(&str, &[&str])] = &[
        // CPython raises `TypeError` for a positional sub-pattern without `__match_args__`.
        (
            "from dataclasses import dataclass\n@dataclass(match_args=False)\nclass Pair:\n    left: int\nmatch Pair(1):\n    case Pair(n):\n        print(n)\n",
            &["E0401"],
        ),
        (
            "class Odd:\n    __match_args__ = ['a']\n    def __init__(self) -> None:\n        self.a = 1\nmatch Odd():\n    case Odd(n):\n        print(n)\n",
            &["E0201"],
        ),
        (
            "match 1:\n    case Missing(n):\n        print(n)\n",
            &["E0205"],
        ),
        (
            "match Point(1, 2):\n    case Point(n, x=m):\n        print(n, m)\n",
            &["E0401"],
        ),
    ];
    for (source, expected) in cases {
        assert_eq!(app(&format!("{POINT}{source}")), *expected, "{source}");
    }
}
