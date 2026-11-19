# typy-test: FUN-07
# typy-expect: rejected E0404 at 11:14
from collections.abc import Generator


def total() -> Generator[int, int, None]:
    yield 0


counter = total()
counter.send(None)
