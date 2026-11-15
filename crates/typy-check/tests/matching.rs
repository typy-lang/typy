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
