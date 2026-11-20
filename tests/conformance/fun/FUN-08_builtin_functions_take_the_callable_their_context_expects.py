# typy-test: FUN-08
# typy-expect: accepted
from collections.abc import Callable

words = ["pear", "fig", "banana", "Kiwi"]
print(sorted(words, key=len), max(words, key=len), min([3, -5, 2], key=abs))
measure: Callable[[str], int] = len
letter = chr
print(measure("abc"), letter(65), ord("a"), measure, letter, type(measure))
print(measure is len, measure == len, len is measure)


def apply(function: Callable[[int], str], value: int) -> str:
    return function(value)


print(apply(chr, 97), apply(repr, 5))
