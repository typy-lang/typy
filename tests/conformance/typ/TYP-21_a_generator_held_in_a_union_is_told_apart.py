# typy-test: TYP-21, EXP-17
# typy-expect: accepted
from collections.abc import Generator, Iterable, Iterator


def sink(*args: int, **kwargs: int) -> str:
    return f"{args} {kwargs}"


def numbers() -> Generator[int, None, None]:
    yield 1
    yield 2


def take(value: Iterable[int] | int) -> str:
    if isinstance(value, Generator):
        return "generator"
    return "other"


def step(value: Iterator[int] | None) -> int:
    return -1 if value is None else next(value)


values = [1, 2]
print(sink(*values), take(numbers()), take(4), step(numbers()), step(None))
