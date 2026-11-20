# typy-test: FUN-12
# typy-expect: rejected E0201 at 7:12
from collections.abc import Iterator


def echo() -> Iterator[int]:
    print((yield 1))
