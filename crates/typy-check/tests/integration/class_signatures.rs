//! Overrides, protocol members and callables whose types are classes.

use crate::common::{app, messages};

#[test]
fn signatures_that_fit_their_bases_and_protocols_are_accepted() {
    let accepted = [
        "from collections.abc import Callable\nfrom typing import Protocol\n\n\n\
        class Shape(Protocol):\n    def area(self) -> float: ...\n\n\nclass Square:\n    \
        def area(self) -> float:\n        return 4.0\n\n\ndef make() -> Shape:\n    \
        return Square()\n\n\nmaker: Callable[[], Shape] = make\nprint(maker().area())\n",
        "from typing import Protocol\n\n\nclass Sized(Protocol):\n    \
        def size(self) -> int: ...\n\n\nclass Box:\n    def size(self) -> int:\n        \
        return 3\n\n\ndef total(pair: tuple[Sized, Sized]) -> int:\n    \
        return pair[0].size() + pair[1].size()\n\n\nprint(total((Box(), Box())))\n",
        "from dataclasses import dataclass\n\ntype Coordinate = int\n\n\n\
        @dataclass(frozen=True)\nclass Point:\n    at: tuple[Coordinate, Coordinate]\n\n\n\
        print(len({Point((1, 2)), Point((1, 2))}))\n",
    ];
    for source in accepted {
        assert_eq!(app(source), Vec::<String>::new(), "{source}");
    }
}

#[test]
fn a_signature_that_does_not_fit_is_refused_with_the_reason() {
    let refused = [
        (
            "class Base[T]:\n    def pick[U](self, item: U) -> U:\n        return item\n\n\n\
            class Derived(Base[int]):\n    def pick[U](self, item: U) -> U:\n        \
            return item\n\n\nprint(Derived().pick(\"a\"))\n",
            "E0201",
            "a generic method that overrides or is overridden by one with other type \
            parameters, or in a generic class, is planned for TyPy 0.13 (GEN-16)",
        ),
        (
            "from collections.abc import Callable\nfrom typing import Protocol\n\n\n\
            class Shape(Protocol):\n    def area(self) -> float: ...\n\n\nclass Square:\n    \
            def area(self) -> float:\n        return 4.0\n\n\ndef make() -> Square:\n    \
            return Square()\n\n\nmaker: Callable[[], Shape] = make\nprint(maker().area())\n",
            "E0402",
            "`maker` was declared as Callable[[], Shape] and is assigned a Callable[[], \
            Square]",
        ),
        (
            "class Base:\n    def total(self, *values: int) -> int:\n        \
            return len(values)\n\n\nclass Derived(Base):\n    \
            def total(self, values: int) -> int:\n        return values\n",
            "E0404",
            "`Derived.total` takes `values` where `Base.total`, which it overrides, takes \
            `*values`",
        ),
        (
            "from typing import Protocol\n\n\nclass Sized(Protocol):\n    \
            def size(self) -> int: ...\n\n\nclass Shape(Protocol):\n    \
            def bounds(self) -> Sized: ...\n",
            "E0201",
            "the protocol `Sized` in the signature of a protocol's method is planned for \
            TyPy 0.13 (GEN-17)",
        ),
        (
            "class Counter:\n    count: int\n\n\nCounter.count = 1\n",
            "E0412",
            "a type[Counter] has no attributes in TyPy",
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
