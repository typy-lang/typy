# typy-test: GEN-01
# typy-expect: accepted
from collections.abc import Callable, Generator


def apply[T](value: T, change: Callable[[T], T]) -> T:
    return change(value)


def first_of[T](changes: list[Callable[[T], T]], value: T) -> T:
    return changes[0](value)


def call_with_one[R](make: Callable[[int], R]) -> R:
    return make(1)


def repeat[T](value: T, times: int) -> Generator[T, None, int]:
    for _ in range(times):
        yield value
    return times


def increment(number: int) -> int:
    return number + 1


def shout(text: str) -> str:
    return text.upper() + "!"


def describe(number: int) -> str:
    return "number " + str(number)


print(apply(1, increment), apply("hi", shout))
print(first_of([increment], 41), call_with_one(describe), call_with_one(increment))
print(list(repeat("a", 3)), [n * 2 for n in repeat(4, 2)])
