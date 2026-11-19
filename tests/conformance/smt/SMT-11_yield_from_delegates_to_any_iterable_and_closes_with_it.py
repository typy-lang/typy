# typy-test: SMT-11, FUN-11
# typy-expect: accepted
from collections.abc import Generator, Iterator


def leaf(name: str) -> Generator[int, None, str]:
    try:
        yield 1
        yield 2
    finally:
        print("leaf finally", name)
    return name.upper()


def middle(name: str) -> Generator[int | None, None, int]:
    try:
        result = yield from leaf(name)
        print("middle got", result)
        yield None
    finally:
        print("middle finally", name)
    return len(result)


def tree(depth: int) -> Iterator[str]:
    yield "node " + str(depth)
    if depth > 0:
        yield from tree(depth - 1)
        yield from tree(depth - 1)


def mixed() -> Iterator[str]:
    yield from "ab"
    yield from {"k": 1}
    yield from [str(number) for number in range(2)]


def numbers() -> Iterator[int]:
    yield from range(3)


def returns_in_finally() -> Generator[int, None, int]:
    try:
        yield 1
        return 1
    finally:
        print("finally runs before StopIteration")


def outer_result() -> Generator[int, None, None]:
    value = yield from returns_in_finally()
    print("outer saw", value)


print(list(middle("full")))
early = middle("early")
print(next(early))
early.close()
print("after close")
dropped = middle("dropped")
print(next(dropped))
dropped = middle("replacement")
print("after drop")
print(list(tree(2)))
print(list(mixed()))
print(sum(numbers()))
print(list(outer_result()))
