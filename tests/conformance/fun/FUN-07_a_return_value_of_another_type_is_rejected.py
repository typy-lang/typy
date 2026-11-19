# typy-test: FUN-07
# typy-expect: rejected E0405 at 8:12
from collections.abc import Generator


def count() -> Generator[int, None, str]:
    yield 1
    return 5
