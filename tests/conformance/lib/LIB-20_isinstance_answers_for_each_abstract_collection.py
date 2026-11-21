# typy-test: LIB-20
# typy-expect: accepted
from collections.abc import Mapping, Sequence


def describe(value: Sequence[int] | int) -> str:
    if isinstance(value, Sequence):
        return f"sequence of {len(value)}"
    return "number"


def head(values: Sequence[int]) -> Sequence[int]:
    return values[:2]


print(describe([1, 2]), describe(3), describe(range(4)), head([5, 6, 7]), head((8, 9, 10)), head(range(5)))
print(isinstance({"a": 1}, Mapping), isinstance([1], Mapping))
from collections.abc import Collection, Container, Iterator, MutableSequence, Reversible


def kinds(value: list[int] | set[int] | dict[str, int] | str) -> str:
    return " ".join(
        [
            str(isinstance(value, Sequence)),
            str(isinstance(value, MutableSequence)),
            str(isinstance(value, Collection)),
            str(isinstance(value, Reversible)),
            str(isinstance(value, Container)),
            str(isinstance(value, Mapping)),
        ]
    )


def numbers() -> Iterator[int]:
    yield 1


print(kinds([1]), "|", kinds({1}), "|", kinds({"a": 1}), "|", kinds("s"))
print(isinstance(numbers(), Collection), isinstance(iter([1]), Container))
