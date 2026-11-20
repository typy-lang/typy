# typy-test: TYP-20
# typy-expect: rejected E0404 at 18:13
from collections.abc import Iterator


class Count:
    def __iter__(self) -> "Count":
        return self

    def __next__(self) -> int:
        raise StopIteration


def words(values: Iterator[str]) -> list[str]:
    return list(values)


print(words(Count()))
