# typy-test: FUN-07, EXC-04
# typy-expect: accepted
from collections.abc import Iterator


def suspended() -> Iterator[int]:
    try:
        raise ValueError("inside the generator")
    except ValueError:
        yield 1


def failing() -> Iterator[int]:
    yield 2
    raise KeyError("in generator")


def main() -> None:
    held = suspended()
    numbers = failing()
    print(next(numbers))
    try:
        raise OSError("outside")
    except OSError:
        print(next(held))
        print(next(numbers))


main()
