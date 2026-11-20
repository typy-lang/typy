# typy-test: FUN-07
# typy-expect: accepted
from collections.abc import Generator


def numbers() -> Generator[int, None, int]:
    try:
        yield 1
        return 5
    finally:
        raise ValueError("stopped")


try:
    print(list(numbers()))
except ValueError as error:
    print(error)
