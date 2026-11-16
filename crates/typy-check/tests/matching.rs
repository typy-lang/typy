//! Tests of checking `match` statements through the public API (CST-06): what a pattern
//! narrows, which cases always match, and the one error each mistake gives.

mod common;

use camino::Utf8PathBuf;
use common::app;
use typy_check::TypeChecker;
use typy_source::{Project, ProjectSettings};

/// Returns each warning of the accepted one-module program `source`, as its code and message.
fn warnings(source: &str) -> Vec<String> {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let base = Utf8PathBuf::from_path_buf(std::env::temp_dir()).unwrap();
    let root = base.join(format!(
        "typy-check-warnings-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("app.py"), source).unwrap();
    let project = Project::load(&root.join("app.py"), ProjectSettings::new(root.clone())).unwrap();
    let checked = TypeChecker::check(&project);
    let _ = std::fs::remove_dir_all(&root);
    checked
        .expect("the program is accepted")
        .warnings
        .iter()
        .map(|warning| format!("{}: {}", warning.code(), warning.message()))
        .collect()
}

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
        // A generic dataclass gets them too.
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

const SHAPES: &str = "from dataclasses import dataclass\n@dataclass\nclass Circle:\n    radius: int\n@dataclass\nclass Square:\n    side: int\n";

#[test]
fn cases_that_cover_every_member_need_no_wildcard() {
    let cases = [
        "def f(s: Circle | Square) -> int:\n    match s:\n        case Circle(r):\n            return r\n        case Square(side=n):\n            return n\n",
        "def f(b: bool) -> int:\n    match b:\n        case True:\n            return 1\n        case False:\n            return 0\n",
        "def f(v: int | None) -> int:\n    match v:\n        case None:\n            return 0\n        case int(n) | n:\n            return n\n",
        "def f(v: tuple[int, str] | int) -> int:\n    match v:\n        case (n, _):\n            return n\n        case int(n):\n            return n\n",
        "def f(v: list[int] | None) -> int:\n    match v:\n        case None:\n            return 0\n        case [*rest]:\n            return len(rest)\n",
        "def f(v: dict[str, int] | bool) -> int:\n    match v:\n        case {}:\n            return 0\n        case int():\n            return 1\n",
    ];
    for source in cases {
        let program = format!("{SHAPES}{source}");
        assert_eq!(app(&program), Vec::<String>::new(), "{source}");
        assert_eq!(warnings(&program), Vec::<String>::new(), "{source}");
    }
}

#[test]
fn a_match_that_leaves_members_unmatched_warns_and_names_them() {
    let cases = [
        (
            "def f(s: Circle | Square | None) -> int:\n    match s:\n        case Circle():\n            return 1\n        case Square(side=2):\n            return 2\n    return 0\n",
            "W0401: this `match` has no case for None, Square",
        ),
        // A case with a guard covers nothing, as the guard may be false.
        (
            "def f(b: bool) -> int:\n    match b:\n        case True:\n            return 1\n        case False if b is False:\n            return 0\n    return 2\n",
            "W0401: this `match` has no case for False",
        ),
        (
            "def f(v: list[int] | str) -> int:\n    match v:\n        case [a, *_]:\n            return a\n        case str():\n            return 0\n    return 1\n",
            "W0401: this `match` has no case for list[int]",
        ),
    ];
    for (source, expected) in cases {
        assert_eq!(
            warnings(&format!("{SHAPES}{source}")),
            [expected],
            "{source}"
        );
    }
}

#[test]
fn a_match_on_a_type_of_one_member_never_warns() {
    let source = "def f(n: int) -> int:\n    match n:\n        case 1:\n            return 1\n    return 0\n";
    assert_eq!(warnings(source), Vec::<String>::new());
}
