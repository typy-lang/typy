# typy-test: EXP-17
# typy-expect: accepted
# typy-python: 3.15
from typing import Callable


def pair(a: int, b: int) -> int:
    return a * 10 + b


def keyed(a: int, *, k: int = 0) -> int:
    return a + k


def only(a: int, b: int, /) -> int:
    return a - b


def opt(a: int, b: int = 2) -> int:
    return a + b


def sink(*args: int, **kwargs: int) -> str:
    return f"{args} {kwargs}"


def forward(*args: int, **kwargs: int) -> str:
    return sink(*args, **kwargs)


def named(value: int, count: int = 0) -> int:
    return value + count


class Box:
    def __init__(self, a: int, b: int = 5) -> None:
        self.total = a + b

    def put(self, a: int, b: int) -> int:
        return self.total + a + b


def outer(values: list[int]) -> int:
    def inner(a: int) -> int:
        return a
    return inner(*values)


def show(label: str) -> int:
    print("evaluating", label)
    return 1


def items(label: str) -> list[int]:
    print("evaluating", label)
    return [1]


def run(case: Callable[[], int]) -> None:
    try:
        print(case())
    except TypeError as error:
        print("TypeError:", error)


def run_text(case: Callable[[], str]) -> None:
    try:
        print(case())
    except TypeError as error:
        print("TypeError:", error)


nothing: list[int] = []


point = (3, 4)
print(pair(*point), pair(*[1, 2]), pair(1, *[2]), pair(**{"a": 1, "b": 2}))
print(forward(1, 2, x=3), forward(), keyed(*[1], **{"k": 2}), opt(*[1]), only(*[9, 4]))
print(Box(*[1]).total, Box(**{"a": 1, "b": 1}).total, Box(1).put(*[2, 3]))
print(sink(show("a"), *items("b"), show("c"), k=show("d"), **{"m": show("e")}, n=show("f")))
print(sink(k=show("g"), *items("h")))
print(outer([7]))
run(lambda: pair(*[1, 2, 3]))
run(lambda: pair(*[1]))
run(lambda: pair(*nothing))
run(lambda: pair(1, *[2], **{"b": 3}))
run(lambda: pair(**{"a": 1, "c": 2}))
run(lambda: pair(**{"a": 1}))
run(lambda: keyed(**{"a": 1, "k": 2, "z": 3}))
run(lambda: only(*[1], **{"a": 1}))
run(lambda: Box(1).put(*[1, 2, 3]))
run(lambda: Box(*[1, 2, 3]).total)
run(lambda: opt(*[1, 2, 3]))
run(lambda: named(1, **{"cont": 2}))
run_text(lambda: sink(**{"k": 1}, k=2))
run_text(lambda: sink(**{"k": 1}, **{"k": 2}))
run(lambda: outer([1, 2]))
