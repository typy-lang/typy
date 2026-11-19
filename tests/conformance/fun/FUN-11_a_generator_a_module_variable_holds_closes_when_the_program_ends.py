# typy-test: FUN-11
# typy-expect: accepted
from collections.abc import Iterator


def g(name: str) -> Iterator[int]:
    try:
        yield 1
    finally:
        print("closing", name)


first = g("first")
next(first)
second = g("second")
next(second)
held = [g("in list")]
next(held[0])


def main() -> None:
    local = g("local")
    next(local)
    print("main ends")


main()
print("module ends")
