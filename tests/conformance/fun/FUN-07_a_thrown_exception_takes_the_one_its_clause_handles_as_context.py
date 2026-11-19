# typy-test: FUN-07, EXC-04
# typy-expect: accepted
from collections.abc import Generator


def thrown_in_clause() -> Generator[int, None, None]:
    try:
        raise ValueError("handled")
    except ValueError:
        yield 1


def main() -> None:
    numbers = thrown_in_clause()
    print(next(numbers))
    numbers.throw(KeyError("thrown"))


main()
