# typy-test: FUN-05
# typy-expect: accepted
from typing import Callable


class Word:
    def __init__(self, text: str) -> None:
        self.text = text

    def size(self) -> int:
        return len(self.text)

    def __repr__(self) -> str:
        return "Word(" + self.text + ")"


def length(text: str) -> int:
    return len(text)


def last(pair: tuple[str, int]) -> int:
    return pair[1]


def by_size(word: Word) -> int:
    return word.size()


def negative(value: int) -> int:
    return -value


def loud(value: int) -> int:
    print("key", value)
    return value % 3


words = ["pear", "fig", "banana", "kiwi", "apple"]
print(sorted(words, key=length), sorted(words, key=length, reverse=True))
print(min(words, key=length), max(words, key=length), sorted(words, key=None))
pairs = [("b", 2), ("a", 2), ("c", 1)]
print(sorted(pairs, key=last), max(pairs, key=last), min(pairs, key=last))
objects = [Word("ccc"), Word("a"), Word("bb")]
objects.sort(key=by_size)
print(objects, min(objects, key=by_size), max(objects, key=by_size))
objects.sort(key=by_size, reverse=True)
print(objects)
numbers = [3, 1, 4, 1, 5, 9, 2, 6]
numbers.sort(key=negative)
print(numbers, sorted(numbers, reverse=True, key=loud))
empty: list[str] = []
print(min(empty, key=length, default="none"), max(words, key=length, default="x"))
print(min(5, 3, 8, key=negative), max("b", "aa", key=length))
keyed: Callable[[str], int] = length
print(sorted(words, key=keyed)[0], sorted(set(words), key=length)[-1])
try:
    min(empty, key=length)
except ValueError as error:
    print("ValueError:", error)


def fails(text: str) -> int:
    if text == "kiwi":
        raise KeyError(text)
    return len(text)


try:
    words.sort(key=fails)
except KeyError as error:
    print("KeyError", error, words)
