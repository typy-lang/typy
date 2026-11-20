# typy-test: FUN-11, EXC-04
# typy-expect: accepted
from collections.abc import Iterator


def counting(name: str) -> Iterator[int]:
    try:
        yield 1
        yield 2
    finally:
        print("closing", name)


def consume(numbers: Iterator[int], divisor: int) -> int:
    return next(numbers) // divisor


def holds_one() -> None:
    held = counting("local")
    next(held)
    raise ValueError("from holds_one")


def nested() -> None:
    outer = counting("outer local")
    next(outer)
    holds_one()


def kept_by_name() -> ValueError | None:
    try:
        holds_one()
    except ValueError as error:
        print("caught", error)
        return error
    return None


try:
    consume(counting("argument"), 0)
except ZeroDivisionError:
    print("caught division")
try:
    holds_one()
except ValueError:
    print("caught value error")
try:
    nested()
except ValueError:
    print("caught nested")
saved = kept_by_name()
print("saved", saved)
saved = None
print("released saved")
for number in counting("loop"):
    try:
        raise KeyError("in loop")
    except KeyError:
        print("loop caught")
    break
print("after loop")
holds_one()
