# typy-test: LIB-23
# typy-expect: accepted
from collections.abc import Iterator


def total(values: Iterator[int]) -> int:
    return sum(values)


def lengths(words: list[str]) -> Iterator[int]:
    return map(len, words)


kept: Iterator[str] = filter(None, ["a", ""])
print(total(map(int, ["1", "2"])), list(lengths(["ab", "c"])), list(kept))
print(list(map(int, ["1", "x"])))
