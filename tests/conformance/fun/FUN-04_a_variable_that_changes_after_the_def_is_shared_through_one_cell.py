# typy-test: FUN-04
# typy-expect: accepted
from typing import Callable


def loop_lambdas() -> list[int]:
    functions: list[Callable[[], int]] = []
    for i in range(3):
        def get() -> int:
            return i
        functions.append(get)
    return [each() for each in functions]


def later() -> int:
    def read() -> int:
        return total * 2
    total = 21
    return read()


def counter() -> Callable[[], int]:
    count = 0

    def peek() -> int:
        return count
    count = 5
    count += 1
    return peek


def unbound() -> str:
    def read() -> int:
        return value
    try:
        read()
    except NameError as error:
        return str(error)
    value = 1
    return "bound"


def sibling() -> int:
    def first() -> int:
        return second() + 1

    def second() -> int:
        return 10
    return first()


def handler() -> str:
    try:
        raise ValueError("oops")
    except ValueError as error:
        def describe() -> str:
            return str(error)
        return describe()


def parameter(step: int) -> int:
    def add(value: int) -> int:
        return value + step
    step = step * 10
    return add(1)


def nested_cells() -> int:
    base = 1

    def middle() -> int:
        def inner() -> int:
            return base
        return inner()
    base = 7
    return middle()


print(loop_lambdas(), later(), counter()(), unbound())
print(sibling(), handler(), parameter(2), nested_cells())
