# typy-test: FUN-07
# typy-expect: rejected E0412 at 10:9
from collections.abc import Iterator


def count() -> Iterator[int]:
    yield 1


count().close()
