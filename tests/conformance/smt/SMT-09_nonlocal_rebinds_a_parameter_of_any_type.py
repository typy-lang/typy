# typy-test: SMT-09
# typy-expect: accepted
from collections.abc import Iterator


def counter(start: int) -> int:
    def bump() -> None:
        nonlocal start
        start += 1

    bump()
    return start


print(counter(1))


def scale(factor: float) -> float:
    def double() -> None:
        nonlocal factor
        factor *= 2

    double()
    return factor


print(scale(1.5))


def counting(start: int) -> Iterator[int]:
    def bump() -> None:
        nonlocal start
        start += 1

    bump()
    yield start


print(list(counting(1)))
