# typy-test: TYP-14
# typy-expect: accepted
from collections.abc import Iterator


def total(source: Iterator[int] | list[int]) -> int:
    return sum(list(source))


def first(source: Iterator[str] | str) -> str:
    return next(iter(source))


print(total(iter([1, 2])), total([3, 4]), first("abc"), first(iter(["x"])))
