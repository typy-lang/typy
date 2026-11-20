# typy-test: SMT-11
# typy-expect: rejected E0402 at 7:21
from collections.abc import Generator


def total() -> Generator[int, int, None]:
    received: int = yield 0
    print(received)
