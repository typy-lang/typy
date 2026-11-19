# typy-test: SMT-11
# typy-expect: rejected E0411 at 7:16
from collections.abc import Iterator


def numbers() -> Iterator[int]:
    yield from 5
