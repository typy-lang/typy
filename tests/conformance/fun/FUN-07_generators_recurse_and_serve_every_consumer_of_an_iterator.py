# typy-test: FUN-07, SMT-11
# typy-expect: accepted
from collections.abc import Callable, Iterator


def tree(depth: int) -> Iterator[int]:
    yield depth
    if depth > 0:
        for value in tree(depth - 1):
            yield value * 10


class Box:
    def __init__(self, items: list[str]) -> None:
        self.items = items

    def each(self) -> Iterator[str]:
        for item in self.items:
            yield item + "!"


def outer(step: int) -> Callable[[int], Iterator[int]]:
    def inner(n: int) -> Iterator[int]:
        total = 0
        for i in range(n):
            total += step
            yield total
    return inner


def text_lines() -> Iterator[str]:
    yield "a"
    yield "bb"


def optional_items() -> Iterator[int | None]:
    yield 1
    yield None


print(list(tree(3)), list(Box(["x", "y"]).each()), list(outer(5)(3)))
print(sorted(text_lines()), max(text_lines()), "-".join(text_lines()))
print(list(optional_items()), [x for x in tree(2)], dict(enumerate(text_lines())))
gen = tree(1)
print(next(gen), next(gen), next(gen, -1))
