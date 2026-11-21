//! Annotations and the names a module defines, which the checker reads before any body.

use crate::common::{app, messages};

#[test]
fn declarations_cpython_reads_alike_are_accepted() {
    let accepted = [
        "class Key:\n    NAME = \"name\"\n\n\ndef find(record: dict[str, int]) -> int:\n    \
        match record:\n        case {Key.NAME: value}:\n            return value\n    \
        return 0\n\n\nprint(find({\"name\": 1}))\n",
    ];
    for source in accepted {
        assert_eq!(app(source), Vec::<String>::new(), "{source}");
    }
}

#[test]
fn a_declaration_typy_cannot_read_as_cpython_does_is_refused() {
    let refused = [
        (
            "from collections.abc import Generator\n\n\n\
            def numbers() -> Generator[int, None]:\n    yield 1\n",
            "E0408",
            "a `Generator` annotation names what it yields, what it is sent and what it \
            returns, such as `Generator[int, None, None]`",
        ),
        (
            "import collections.abc\n\n\n\
            def total(items: collections.abc.Sized) -> int:\n    return 0\n\n\n\
            print(total({1, 2}))\n",
            "E0205",
            "this annotation does not name a type",
        ),
        (
            "from math import floor\n\n\nclass floor:\n    pass\n\n\n\
            box: floor[int] = floor()\n",
            "E0202",
            "`floor` is bound by an import and defined by the module; TyPy keeps one \
            definition per name",
        ),
        (
            "def show() -> None:\n    import typing\n    \
            items: typing.Sequence[int] = [1, 2]\n    print(items)\n\n\nshow()\n",
            "E0202",
            "generic annotations other than `list`, `dict`, `set`, `tuple`, `type`, \
            `Iterator`, `Iterable` and their `typing` names, such as `frozenset[int]`, \
            cannot be compiled by TyPy with CPython's meaning",
        ),
        (
            "from collections.abc import Callable\n\n\n\
            def apply(action: Callable[[int], int, str]) -> int:\n    return 0\n",
            "E0408",
            "a `Callable` annotation names its parameter types and its result, such as \
            `Callable[[int, str], bool]`",
        ),
        (
            "from collections.abc import Callable\nfrom typing import Protocol\n\n\n\
            def apply(action: Callable[[int], Protocol]) -> int:\n    return 0\n",
            "E0408",
            "`Protocol` is not a type; a protocol class derives from it",
        ),
        (
            "class Box[T]:\n    def __init__(self, item: T) -> None:\n        \
            self.item = item\n\n\ndef first[U](box: Box, item: U) -> U:\n    return item\n",
            "E0408",
            "the generic class `Box` is named with its type arguments, such as `Box[int]`",
        ),
        (
            "from math import floor\n\nfloor = 2\nprint(floor)\n",
            "E0202",
            "`floor` is a function and is also assigned as a variable",
        ),
        (
            "def parse(text: str) -> int:\n    try:\n        return int(text)\n    \
            except ValueError as error:\n        raise ValueError(error.args)\n\n\n\
            print(parse(\"1\") + \"a\")\n",
            "E0401",
            "`+` is not defined for int and str",
        ),
    ];
    for (source, code, message) in refused {
        let found = messages(source);
        assert!(
            found
                .iter()
                .any(|(each, text, _)| each == code && text == message),
            "{source}\n{found:?}"
        );
    }
}
