# typy-test: FUN-07
# typy-expect: accepted
from collections.abc import Iterator


def scaled(factor: float) -> Iterator[float]:
    yield factor * 2


print(list(scaled(1.5)))


def repeat(nothing: None) -> Iterator[bool]:
    yield nothing is None


print(list(repeat(None)))
