//! `map` and `filter`: objects of the classes a module is given for each form it calls,
//! typed from the items of the iterables and the function's result, and the calls CPython
//! refuses (LIB-23, LIB-25, ADR-0089).

use crate::common::{app, messages};

/// Returns the one error of `source`: its code, message and rule.
fn only_error(source: &str) -> (String, String, String) {
    let found = messages(source);
    assert_eq!(found.len(), 1, "{found:?}");
    found.into_iter().next().unwrap()
}

#[test]
fn lib_23_map_and_filter_take_functions_lambdas_and_built_ins() {
    let source = "\
from collections.abc import Iterator


def double(value: int) -> int:
    return 2 * value


def lengths(words: list[str]) -> Iterator[int]:
    return map(len, words)


words = ['1', '22']
print(list(map(double, [1])), list(map(int, words)), list(map(str.upper, words)))
print(list(map(lambda a, b: a * b, [1, 2], [3, 4])), sum(lengths(words)))
print(list(filter(None, [0, 1])), list(filter(lambda w: len(w) > 1, words)))
print(list(map(str, map(double, filter(None, [0, 3])))))
";
    assert_eq!(app(source), Vec::<String>::new());
}

#[test]
fn lib_23_a_module_that_binds_the_name_calls_its_own() {
    let source = "\
def map(function: int, values: list[int]) -> int:
    return function + len(values)


print(map(1, [2]))
";
    assert_eq!(app(source), Vec::<String>::new());
}

#[test]
fn lib_23_each_refused_call_has_cpythons_text() {
    let cases = [
        (
            "print(map(str))\n",
            "map() must have at least two arguments, as CPython raises `TypeError`",
        ),
        (
            "print(filter(str, [1], [2]))\n",
            "filter expected 2 arguments, got 3, as CPython raises `TypeError`",
        ),
        (
            "print(list(map(str, [1], strict=True)))\n",
            "map() takes no keyword arguments, as CPython raises `TypeError`",
        ),
        (
            "print(list(filter(None, [1], strict=True)))\n",
            "filter() takes no keyword arguments, as CPython raises `TypeError`",
        ),
    ];
    for (source, message) in cases {
        let (code, text, rule) = only_error(source);
        assert_eq!(
            (code.as_str(), text.as_str(), rule.as_str()),
            ("E0404", message, "LIB-23"),
            "{source}"
        );
    }
}

#[test]
fn lib_23_a_function_that_takes_other_items_is_refused() {
    let source = "def f(x: int) -> int:\n    return x\n\n\nprint(list(map(f, ['a'])))\n";
    let (code, text, _) = only_error(source);
    assert_eq!(code, "E0404");
    assert_eq!(
        text,
        "the function of `map` is called with an item of each iterable, of type str, and \
         this is a Callable[[int], int]"
    );
}

#[test]
fn lib_23_a_map_object_has_no_attributes_of_its_own() {
    let source = "m = map(str, [1])\nprint(m.function)\n";
    let (code, text, _) = only_error(source);
    assert_eq!(
        (code.as_str(), text.as_str()),
        ("E0414", "`map` has no attribute `function`")
    );
}

#[test]
fn lib_25_map_as_a_value_and_unpacking_into_it_are_planned() {
    let cases = [
        ("f = map\n", "`map` is planned for TyPy 1.0 (LIB-25)"),
        (
            "lists = [[1], [2]]\nprint(list(map(max, *lists)))\n",
            "`*` in a call of `map` is planned for TyPy 1.0 (LIB-25)",
        ),
    ];
    for (source, message) in cases {
        let (code, text, rule) = only_error(source);
        assert_eq!(
            (code.as_str(), text.as_str(), rule.as_str()),
            ("E0201", message, "LIB-25"),
            "{source}"
        );
    }
}
