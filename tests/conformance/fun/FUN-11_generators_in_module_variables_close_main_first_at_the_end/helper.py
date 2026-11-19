from collections.abc import Iterator


def g(name: str) -> Iterator[int]:
    try:
        yield 1
    finally:
        print("closing", name)


in_helper = g("helper global")
next(in_helper)
