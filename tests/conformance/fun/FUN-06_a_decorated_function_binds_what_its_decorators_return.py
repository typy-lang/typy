# typy-test: FUN-06
# typy-expect: accepted
from collections.abc import Callable


def label(text: str) -> Callable[[Callable[[int], int]], Callable[[int], int]]:
    print("label", text, "evaluated")

    def apply(function: Callable[[int], int]) -> Callable[[int], int]:
        print("label", text, "applied")
        return function

    return apply


def trace(function: Callable[[int], int]) -> Callable[[int], int]:
    def wrapper(value: int) -> int:
        print("calling with", value)
        return function(value)

    return wrapper


def twice[T](function: Callable[[T], T]) -> Callable[[T], T]:
    def wrapper(value: T) -> T:
        return function(function(value))

    return wrapper


registry: list[Callable[[int], int]] = []


def register(function: Callable[[int], int]) -> Callable[[int], int]:
    registry.append(function)
    return function


@label("outer")
@label("inner")
def plain(value: int) -> int:
    return value


@trace
def increment(value: int) -> int:
    return value + 1


@twice
def shout(text: str) -> str:
    return text + "!"


@register
def triple(value: int) -> int:
    return value * 3


@trace
def countdown(value: int) -> int:
    if value == 0:
        return 0
    return countdown(value - 1)


print(plain(5), increment(2), shout("hi"), registry[0] is triple, triple(2))
print(countdown(2), repr(plain).startswith("<function plain at 0x"), repr(increment).startswith("<function trace.<locals>.wrapper at 0x"))
