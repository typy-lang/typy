# typy-test: FUN-04
# typy-expect: accepted
from typing import Callable


def make_adder(step: int) -> Callable[[int], int]:
    def add(value: int) -> int:
        return value + step
    return add


def make_greeter(greeting: str, mark: str = "!") -> Callable[[str], str]:
    prefix = greeting + ", "

    def greet(name: str) -> str:
        return prefix + name + mark
    return greet


def countdown(start: int) -> list[int]:
    def walk(value: int) -> list[int]:
        if value == 0:
            return [0]
        return [value] + walk(value - 1)
    return walk(start)


def outer(base: int) -> int:
    def middle(x: int) -> int:
        def inner(y: int) -> int:
            return base + x + y
        return inner(10)
    return middle(100)


class Box:
    def __init__(self, size: int) -> None:
        self.size = size

    def scaled(self) -> Callable[[int], int]:
        def scale(factor: int) -> int:
            return self.size * factor
        return scale


add3 = make_adder(3)
add5 = make_adder(5)
print(add3(1), add5(1), add3 is add5, add3 == add3, make_adder(1) == make_adder(1))
print(make_greeter("Hello")("Ada"), make_greeter("Hi", "?")("Alan"))
print(countdown(4), outer(1), Box(7).scaled()(3))
print(repr(add3).startswith("<function make_adder.<locals>.add at 0x"))
adders = [make_adder(n) for n in range(4)]
print([each(10) for each in adders])
