//! `typing.overload`: a call is checked against the first overload that takes its
//! arguments, which gives its result, and the body against every overload (TYP-29, ADR-0112).

use crate::common::{app, messages};

const DOUBLE: &str = "from typing import overload\n\n\n@overload\ndef double(value: int) -> int: ...\n\
                      @overload\ndef double(value: str) -> str: ...\n\
                      def double(value: int | str) -> int | str:\n    if isinstance(value, int):\n        \
                      return value * 2\n    return value + value\n\n\n";

#[test]
fn typ_29_a_call_takes_the_result_of_the_overload_it_matches() {
    let accepted = [
        format!("{DOUBLE}n: int = double(2)\ns: str = double('a')\nprint(n + 1, s.upper())\n"),
        // A parameter an overload leaves out takes the implementation's default.
        "from typing import overload\n\n\n@overload\ndef get(key: str) -> str | None: ...\n\
         @overload\ndef get(key: str, default: str) -> str: ...\n\
         def get(key: str, default: str | None = None) -> str | None:\n    return default\n\n\n\
         found: str = get('a', 'b')\nprint(found.upper(), get('a'))\n"
            .to_owned(),
        // A method's overloads, with the instance first.
        "from typing import overload\n\n\nclass Shelf:\n    \
         def __init__(self, items: list[int]) -> None:\n        self.items = items\n\n    \
         @overload\n    def get(self, index: int) -> int: ...\n    \
         @overload\n    def get(self, index: slice) -> list[int]: ...\n    \
         def get(self, index: int | slice) -> int | list[int]:\n        \
         if isinstance(index, int):\n            return self.items[index]\n        \
         return self.items[index]\n\n\nshelf = Shelf([1, 2])\nfirst: int = shelf.get(0)\n\
         rest: list[int] = shelf.get(slice(1, 2))\nprint(first, rest)\n"
            .to_owned(),
        // Overloads of literal types, whose body a test of the literal decides for each.
        "from typing import Literal, overload\n\n\n@overload\n\
         def read(mode: Literal['r']) -> str: ...\n@overload\n\
         def read(mode: Literal['rb']) -> bytes: ...\n\
         def read(mode: Literal['r', 'rb']) -> str | bytes:\n    if mode == 'r':\n        \
         return 'text'\n    return b'data'\n\n\ntext: str = read('r')\ndata: bytes = read('rb')\n\
         print(text, data)\n"
            .to_owned(),
        // `typing.overload` through the module, and an implementation whose result is
        // inferred.
        "import typing\n\n\n@typing.overload\ndef same(x: int) -> int: ...\n@typing.overload\n\
         def same(x: str) -> str: ...\ndef same(x: int | str):\n    return x\n\n\n\
         n: int = same(1)\nprint(n)\n"
            .to_owned(),
    ];
    for source in accepted {
        assert_eq!(app(&source), Vec::<String>::new(), "{source}");
    }
}

#[test]
fn typ_29_a_call_no_overload_takes_is_e0432() {
    let found = messages(&format!("{DOUBLE}print(double(1.5))\n"));
    assert!(
        found.iter().any(|(code, text, rule)| code == "E0432"
            && text == "no overload of `double` takes these arguments"
            && rule == "TYP-29"),
        "{found:?}"
    );
}

#[test]
fn typ_29_the_body_is_checked_against_every_overload() {
    let found = messages(
        "from typing import overload\n\n\n@overload\ndef half(value: int) -> int: ...\n\
         @overload\ndef half(value: float) -> float: ...\n\
         def half(value: int | float) -> int | float:\n    return value / 2\n",
    );
    let codes: Vec<&str> = found.iter().map(|(code, ..)| code.as_str()).collect();
    assert_eq!(codes, ["E0405"], "{found:?}");
}

#[test]
fn typ_29_an_overload_its_implementation_does_not_take_is_e0432() {
    let cases = [
        (
            "from typing import overload\n\n\n@overload\ndef f(a: int) -> int: ...\n\
             @overload\ndef f(a: int, b: int) -> int: ...\ndef f(a: int) -> int:\n    return a\n",
            "overload 2 of `f` does not fit its implementation: the implementation has no \
             parameter `b` in that place",
        ),
        (
            "from typing import overload\n\n\n@overload\ndef f(a: int) -> int: ...\n\
             @overload\ndef f(a: str) -> int: ...\ndef f(a: int, b: int) -> int:\n    return a\n",
            "overload 1 of `f` does not fit its implementation: it leaves out `b`, which has no \
             default in the implementation",
        ),
        (
            "from typing import overload\n\n\n@overload\ndef f(a: str) -> int: ...\n\
             @overload\ndef f(a: int) -> int: ...\ndef f(a: int) -> int:\n    return a\n",
            "overload 1 of `f` does not fit its implementation: its `a: str` is not held by the \
             implementation's `a: int`",
        ),
        (
            "from typing import overload\n\n\n@overload\ndef f(a: int) -> str: ...\n\
             @overload\ndef f(a: int, b: int = ...) -> int: ...\n\
             def f(a: int, b: int = 0) -> int:\n    return a\n",
            "overload 1 of `f` does not fit its implementation: its result, str, is not one the \
             implementation's int holds",
        ),
        (
            "from typing import overload\n\n\n@overload\ndef f(a: int, b: int = ...) -> int: ...\n\
             @overload\ndef f(a: str) -> int: ...\ndef f(a: int | str, b: int) -> int:\n    \
             return b\n",
            "overload 1 of `f` does not fit its implementation: a call may leave out `b`, which \
             has no default in the implementation",
        ),
        (
            "from typing import overload\n\n\n@overload\ndef f(a: int, /) -> int: ...\n\
             @overload\ndef f(a: str) -> int: ...\ndef f(a: int | str) -> int:\n    return 1\n",
            "overload 1 of `f` does not fit its implementation: it passes `a` otherwise than \
             the implementation does",
        ),
        (
            "from typing import overload\n\n\n@overload\ndef f(a: int) -> list[int]: ...\n\
             @overload\ndef f(a: str) -> list[str]: ...\n\
             def f(a: int | str) -> list[int] | list[str]:\n    return []\n",
            "overload 1 of `f` does not fit its implementation: no test tells its result, \
             list[int], from the implementation's others when the program runs",
        ),
    ];
    for (source, message) in cases {
        let found = messages(source);
        assert!(
            found
                .iter()
                .any(|(code, text, rule)| code == "E0432" && text == message && rule == "TYP-29"),
            "{source}\n{found:?}"
        );
    }
}

#[test]
fn typ_29_overloads_without_an_implementation_are_e0432() {
    let cases = [
        "from typing import overload\n\n\n@overload\ndef f(a: int) -> int: ...\n",
        "from typing import overload\n\n\n@overload\ndef f(a: int) -> int: ...\nx = 1\n",
        "from typing import overload\n\n\n@overload\ndef f(a: int) -> int: ...\n@overload\n\
         def g(a: int) -> int: ...\ndef g(a: int) -> int:\n    return a\n",
    ];
    for source in cases {
        let found = messages(source);
        assert!(
            found.iter().any(|(code, text, _)| code == "E0432"
                && text == "the overloads of `f` have no implementation"),
            "{source}\n{found:?}"
        );
    }
}

#[test]
fn typ_29_forms_a_later_release_gives_are_planned() {
    let cases = [
        "from typing import overload\n\n\nclass C:\n    @overload\n    @staticmethod\n    \
         def f(a: int) -> int: ...\n    @staticmethod\n    def f(a: int) -> int:\n        \
         return a\n",
        "from typing import overload\n\n\nclass C:\n    @overload\n    \
         def __add__(self, a: int) -> int: ...\n    def __add__(self, a: int) -> int:\n        \
         return a\n",
        "from typing import overload\n\n\ndef outer() -> None:\n    @overload\n    \
         def f(a: int) -> int: ...\n    def f(a: int) -> int:\n        return a\n",
        "from typing import overload\n\n\n@overload\ndef f[T](a: T) -> T: ...\n\
         def f[T](a: T) -> T:\n    return a\n",
        "from typing import overload\n\n\nclass Box[T]:\n    def __init__(self, item: T) -> None:\n        \
         self.item = item\n\n    @overload\n    def get(self, a: int) -> T: ...\n    \
         def get(self, a: int) -> T:\n        return self.item\n",
        "from typing import overload\n\n\nclass A:\n    @overload\n    \
         def get(self, a: int) -> int: ...\n    def get(self, a: int) -> int:\n        \
         return a\n\n\nclass B(A):\n    def get(self, a: int) -> int:\n        return a\n",
        "from typing import overload\n\n\n@overload\ndef f(a: int) -> int: ...\n\
         def f(a: int) -> int:\n    return a\n\n\nargs = (1,)\nprint(f(*args))\n",
    ];
    for source in cases {
        let found = messages(source);
        assert!(
            found
                .iter()
                .any(|(code, _, rule)| code == "E0201" && rule == "TYP-35"),
            "{source}\n{found:?}"
        );
    }
}

#[test]
fn typ_29_overload_is_no_value_and_no_type() {
    let found = messages("from typing import overload\n\nx = overload\n");
    assert_eq!(found[0].0, "E0202", "{found:?}");
    let found = messages("from typing import overload\n\nx: overload = 1\n");
    assert_eq!(found[0].0, "E0408", "{found:?}");
}
