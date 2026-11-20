# typy-test: FUN-11, STD-05
# typy-expect: accepted
from collections.abc import Iterator


def g(name: str) -> Iterator[int]:
    try:
        yield 1
    finally:
        print("closing", name)


def f() -> None:
    local = g("local")
    next(local)
    raise ValueError("boom")


kept = g("global")
next(kept)
try:
    f()
except ValueError:
    print("caught")
f()
