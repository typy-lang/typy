//! `*args` and `**kwargs` parameters, and the binding of the arguments left over to them
//! (EXP-17, FUN-13).

use crate::common::{app, dump, messages};

#[test]
fn exp_17_star_args_and_star_star_kwargs_take_the_arguments_left_over() {
    for source in [
        "def total(*numbers: int) -> int:\n    return sum(numbers)\n\n\n\
         print(total(), total(1, 2))\n",
        "def tagged(name: str, *tags: str, sep: str = \",\", **sizes: int) -> str:\n    \
         return name + sep.join(tags) + str(len(sizes))\n\n\n\
         print(tagged(\"a\", \"b\", \"c\", sep=\"/\", width=2), tagged(\"x\"))\n",
        "def rest(a: int, /, **others: int) -> dict[str, int]:\n    return others\n\n\n\
         print(rest(1, a=2))\n",
        "def both(*args: int, **kwargs: int) -> str:\n    return f\"{args} {kwargs}\"\n\n\n\
         print(both(args=1, kwargs=2))\n",
        "def maybe(*values: int | None) -> int:\n    return len(values)\n\n\n\
         print(maybe(1, None))\n",
        "def first[T](*items: T) -> T:\n    return items[0]\n\n\n\
         def pick[T](**options: T) -> list[T]:\n    return list(options.values())\n\n\n\
         print(first(1, 2) + 1, pick(a=\"x\")[0].upper())\n",
        "class Shape:\n    def __init__(self, *sides: float) -> None:\n        \
         self.sides = sides\n\n\nprint(Shape(1.0, 2.0).sides)\n",
        "def outer() -> int:\n    def inner(*values: int) -> int:\n        \
         return len(values)\n    return inner(1, 2)\n\n\nprint(outer())\n",
    ] {
        assert_eq!(app(source), Vec::<String>::new(), "{source}");
    }
    let found = dump("def total(*numbers: int, **named: str) -> int:\n    return len(numbers)\n");
    assert!(found.contains("tuple[int, ...]"), "{found}");
}

#[test]
fn exp_17_an_argument_left_over_is_refused_where_cpython_or_typing_refuses_it() {
    let refusals = [
        (
            "def f(*words: str) -> None:\n    pass\n\n\nf(\"a\", 1)\n",
            "an argument for `*words` must be a str, and this is an int",
        ),
        (
            "def f(**sizes: int) -> None:\n    pass\n\n\nf(width=\"x\")\n",
            "argument `width` must be an int, and this is a str",
        ),
        (
            "def f(a: int, *rest: int) -> None:\n    pass\n\n\nf(1, a=2)\n",
            "argument `a` is given more than once",
        ),
        (
            "def f(*args: int) -> None:\n    pass\n\n\nf(args=1)\n",
            "`f` has no parameter named `args`",
        ),
        (
            "def f(a: int, **rest: int) -> None:\n    pass\n\n\nf(1, 2)\n",
            "`f` takes 1 argument, and more are given",
        ),
        (
            "def f(*, key: int, **rest: int) -> None:\n    pass\n\n\nf(1, key=2)\n",
            "`f` takes 0 positional arguments, and more are given; the rest are keyword-only",
        ),
        (
            "def f(a: int, /) -> None:\n    pass\n\n\nf(a=1)\n",
            "`f` takes `a` by position only, not by keyword",
        ),
        (
            "def f(*items: int) -> None:\n    pass\n\n\ng = f\n",
            "`f` takes `*args` or `**kwargs`, so as a value it needs the `Callable` type its \
             context expects",
        ),
        (
            "def outer() -> None:\n    def inner(**options: int) -> None:\n        pass\n    \
             keep = inner\n",
            "`inner` takes `**options`, and a function with `*args` or `**kwargs` as a value is \
             planned for TyPy 0.11 (FUN-19)",
        ),
        (
            "class Box:\n    def put(self, *items: int) -> None:\n        pass\n\n\n\
             method = Box().put\n",
            "`Box.put` takes `*items`, and a function with `*args` or `**kwargs` as a value is \
             planned for TyPy 0.11 (FUN-19)",
        ),
        (
            "from typing import Callable\n\ncount: Callable[[], int] = lambda *values: 0\n",
            "`*args` and `**kwargs` in a `lambda` are planned for TyPy 0.11 (FUN-19)",
        ),
        (
            "class Base:\n    def run(self, steps: int) -> None:\n        pass\n\n\n\
             class Derived(Base):\n    def run(self, *steps: int) -> None:\n        pass\n",
            "`Derived.run` takes `*steps` where `Base.run`, which it overrides, takes `steps`",
        ),
        (
            "class Base:\n    def run(self, **steps: int) -> None:\n        pass\n\n\n\
             class Derived(Base):\n    def run(self, *steps: int) -> None:\n        pass\n",
            "`Derived.run` takes `*steps` where `Base.run`, which it overrides, takes `**steps`",
        ),
    ];
    for (source, message) in refusals {
        let found = messages(source);
        assert!(
            found.iter().any(|(_, text, _)| text == message),
            "{source}: {found:?}"
        );
    }
}
