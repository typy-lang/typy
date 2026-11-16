# typy-test: TYP-19
# typy-expect: rejected E0201 at 7:16
from collections.abc import Iterator


def first(source: Iterator[int] | Iterator[str]) -> str:
    return str(next(source))


print(first(iter([1])))
