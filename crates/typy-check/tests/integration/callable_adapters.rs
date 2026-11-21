//! Nested functions, methods and `lambda`s with defaults, `*args` or `**kwargs` as values of
//! the `Callable` their context expects, through an adapter (FUN-19, ADR-0080).

use crate::common::{app, dump, messages};

const APPLY: &str = "from collections.abc import Callable\n\n\n\
                     def once(function: Callable[[int], int], value: int) -> int:\n    \
                     return function(value)\n\n\n";

#[test]
fn fun_19_each_function_with_defaults_or_star_args_fits_a_callable() {
    for body in [
        "def outer() -> int:\n    def add(value: int, more: int = 1) -> int:\n        \
         return value + more\n    return once(add, 2)\n",
        "def outer(base: int) -> int:\n    def total(*values: int, **named: int) -> int:\n        \
         return sum(values) + base\n    def run() -> int:\n        return once(total, 1)\n    \
         return run()\n",
        "class Meter:\n    def scale(self, value: int, extra: int = 0) -> int:\n        \
         return value + extra\n\n\nprint(once(Meter().scale, 1))\n",
        "class Meter:\n    def step(self, value: int, *, by: int = 2) -> int:\n        \
         return value + by\n\n\nthrough: Callable[[Meter, int], int] = Meter.step\n",
        "class Meter:\n    @classmethod\n    def make(cls, value: int, more: int = 1) -> int:\n        \
         return value + more\n\n\nprint(once(Meter.make, 1), once(Meter().make, 1))\n",
        "class Meter:\n    @staticmethod\n    def less(value: int, *rest: int) -> int:\n        \
         return value - sum(rest)\n\n\nprint(once(Meter.less, 1), once(Meter().less, 1))\n",
        "print(once(lambda value, by=2: value * by, 3))\n",
        "print(once(lambda value, *, step=1: value + step, 3))\n",
        "pair: Callable[[int, int], int] = lambda *values: sum(values)\n",
        "from typing import Protocol\n\n\nclass Weighed(Protocol):\n    \
         def weigh(self, value: int, extra: int = 0) -> int: ...\n\n\n\
         class Box:\n    def weigh(self, value: int, extra: int = 0) -> int:\n        \
         return value + extra\n\n\nclass Lonely(Protocol):\n    \
         def heft(self, value: int, *more: int) -> int: ...\n\n\n\
         def use(item: Weighed) -> int:\n    return once(item.weigh, 1)\n\n\n\
         def alone(other: Lonely) -> int:\n    return once(other.heft, 1)\n\n\n\
         print(use(Box()))\n",
        "class Meter:\n    @staticmethod\n    def less(value: int, *rest: int) -> int:\n        \
         return value\n\n\nclass Wide(Meter):\n    \
         def less(self, value: int, *rest: int) -> int:\n        return value\n\n\n\
         meter: Meter = Wide()\nprint(once(meter.less, 1))\n",
    ] {
        let source = format!("{APPLY}{body}");
        assert_eq!(app(&source), Vec::<String>::new(), "{source}");
    }
}

#[test]
fn fun_19_an_adapter_captures_the_function_and_a_method_is_bound_to_its_adapter() {
    let found = dump(&format!(
        "{APPLY}def outer() -> int:\n    def add(value: int, more: int = 1) -> int:\n        \
         return value + more\n    return once(add, 2)\n\n\n\
         class Meter:\n    def scale(self, value: int, extra: int = 0) -> int:\n        \
         return value + extra\n\n\nprint(once(Meter().scale, 1))\n"
    ));
    assert!(found.contains("Lambda"), "{found}");
    assert!(found.contains("CallNested"), "{found}");
    assert!(found.contains("BoundMethod"), "{found}");
}

#[test]
fn fun_19_a_value_no_adapter_fits_is_refused_with_the_reason() {
    let refusals = [
        (
            "def outer() -> None:\n    def add(value: int, more: int = 1) -> int:\n        \
             return value + more\n    text: Callable[[int], str] = add\n",
            "E0404",
            "`add` called with an int gives an int, and this `Callable` returns str",
        ),
        (
            "def outer() -> None:\n    def keyed(value: int, *, step: int) -> int:\n        \
             return value + step\n    once(keyed, 1)\n",
            "E0404",
            "`keyed` is missing the argument `step`",
        ),
        (
            "def outer() -> None:\n    def add(value: int, more: int = 1) -> int:\n        \
             return value + more\n    pair: Callable[[int, int, int], int] = add\n",
            "E0404",
            "`add` takes 2 arguments, and more are given",
        ),
        (
            "class Meter:\n    @classmethod\n    def gather(cls, *values: int) -> int:\n        \
             return sum(values)\n\n\nloose = Meter.gather\n",
            "E0203",
            "`Meter.gather` takes `*values`, so as a value it needs the `Callable` type its \
             context expects",
        ),
        (
            "class Meter:\n    def pair(self, value: int, other: int, extra: int = 0) -> int:\n        \
             return value + other\n\n\nonce(Meter().pair, 1)\n",
            "E0404",
            "`Meter.pair` is missing the argument `other`",
        ),
        (
            "class Meter:\n    def pair(self, value: int, extra: int = 0) -> int:\n        \
             return value\n\n\nonce(Meter.pair, 1)\n",
            "E0404",
            "argument `self` must be a Meter, and this is an int",
        ),
        (
            "class Meter:\n    @staticmethod\n    def less(value: int, *rest: int) -> int:\n        \
             return value\n\n\nclass Wide(Meter):\n    \
             def less(self, value: int, *rest: int) -> int:\n        return value\n\n\n\
             text: Callable[[int], str] = Wide().less\n",
            "E0404",
            "`Wide.less` called with an int gives an int, and this `Callable` returns str",
        ),
        (
            "from typing import Protocol\n\n\nclass Weighed(Protocol):\n    \
             def weigh(self, value: int, other: int, extra: int = 0) -> int: ...\n\n\n\
             def use(item: Weighed) -> int:\n    return once(item.weigh, 1)\n",
            "E0404",
            "`Weighed.weigh` is missing the argument `other`",
        ),
        (
            "named = lambda **options: 0\n",
            "E0203",
            "`lambda` parameter `**options` has no type: a `Callable` passes no keywords",
        ),
        (
            "once(lambda value, *rest: value, 1)\n",
            "E0203",
            "`lambda` parameter `*rest` has no type: the `Callable` expected here passes no \
             argument past the others",
        ),
        (
            "mixed: Callable[[int, str], int] = lambda *values: 0\n",
            "E0409",
            "the items of `*values` have one type; the `Callable` expected here passes an int \
             and a str to it",
        ),
        (
            "once(lambda value, *, step: value, 1)\n",
            "E0202",
            "the `lambda` takes `step` by keyword only, and a `Callable` passes every argument \
             by position",
        ),
    ];
    for (body, code, message) in refusals {
        let source = format!("{APPLY}{body}");
        let found = messages(&source);
        assert!(
            found
                .iter()
                .any(|(each, text, _)| each == code && text == message),
            "{source}: {found:?}"
        );
    }
}
