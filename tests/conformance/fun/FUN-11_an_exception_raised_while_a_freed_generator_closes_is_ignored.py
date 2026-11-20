# typy-test: FUN-11, FUN-07
# typy-expect: accepted
from collections.abc import Iterator


def raising() -> Iterator[int]:
    try:
        yield 1
    finally:
        raise ValueError("boom")


def stubborn() -> Iterator[int]:
    try:
        yield 1
    except GeneratorExit:
        yield 2


def chained() -> Iterator[int]:
    try:
        yield 1
    finally:
        try:
            raise KeyError("first")
        except KeyError:
            raise OSError("second")


def drop(made: Iterator[int]) -> None:
    next(made)


drop(raising())
print("after raising")
drop(stubborn())
print("after stubborn")
drop(chained())
print("after chained")
