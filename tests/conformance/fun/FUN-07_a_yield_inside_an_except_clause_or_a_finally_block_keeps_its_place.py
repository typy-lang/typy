# typy-test: FUN-07, SMT-11, EXC-04
# typy-expect: accepted
from collections.abc import Generator, Iterator


def inside_clause() -> Iterator[str]:
    try:
        raise ValueError("inner")
    except ValueError as error:
        yield "in clause " + str(error)
        try:
            raise KeyError("nested")
        except KeyError:
            yield "nested clause"
        yield "back in outer clause"
    yield "after clause"


def unnamed() -> Iterator[int]:
    try:
        raise IndexError("x")
    except IndexError:
        yield 1
        yield 2
    yield 3


def finally_yields() -> Generator[int, None, None]:
    try:
        yield 1
        raise ValueError("pending")
    finally:
        yield 2
        print("finally done")


def raises_in_clause() -> Iterator[int]:
    try:
        raise ValueError("first")
    except ValueError:
        yield 1
        raise KeyError("second")


def bare_raise() -> Iterator[int]:
    try:
        raise ValueError("again")
    except ValueError:
        yield 1
        raise


def thrown_in_clause() -> Generator[int, None, None]:
    try:
        raise ValueError("handled")
    except ValueError:
        yield 1


def main() -> None:
    for text in inside_clause():
        print(text)
    print(list(unnamed()))
    numbers = finally_yields()
    print(next(numbers))
    print(next(numbers))
    try:
        next(numbers)
    except ValueError as error:
        print("pending raised", error)
    clause = raises_in_clause()
    print(next(clause))
    try:
        next(clause)
    except KeyError as error:
        print("second", error)
    again = bare_raise()
    print(next(again))
    try:
        next(again)
    except ValueError as error:
        print("re-raised", error)
    thrown = thrown_in_clause()
    print(next(thrown))
    try:
        thrown.throw(KeyError("thrown"))
    except KeyError as error:
        print("thrown", error)


main()
