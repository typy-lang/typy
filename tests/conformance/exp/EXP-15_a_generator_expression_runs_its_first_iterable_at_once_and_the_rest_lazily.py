# typy-test: EXP-15, FUN-07
# typy-expect: accepted
from collections.abc import Generator, Iterator


def first_source() -> list[int]:
    print("first iterable runs now")
    return [1, 2, 3]


def second_source(limit: int) -> list[int]:
    print("second iterable runs for", limit)
    return list(range(limit))


def main() -> None:
    squares = (number * number for number in first_source())
    print("made")
    print(sum(squares))
    pairs = ((a, b) for a in first_source() if a > 1 for b in second_source(a) if b != a)
    print("pairs made")
    for pair in pairs:
        print(pair)
    scale = 10
    scaled = list(scale * value for value in range(3))
    print(scaled)
    print(any(value > 2 for value in [1, 2, 3]), all(value > 0 for value in range(0)))
    print(",".join(str(value) for value in range(4)))
    print(max(len(word) for word in ["a", "abc", "ab"]))
    words = (word.upper() for word in "a b".split())
    print(next(words), next(words), next(words, "end"))
    lazy = (1 // value for value in [1, 0])
    print(next(lazy))
    try:
        next(lazy)
    except ZeroDivisionError:
        print("raised as it ran")
    typed: Iterator[int | None] = (value for value in [1, 2])
    print(list(typed))
    gen: Generator[str, None, None] = (str(value) for value in range(2))
    print(list(gen))
    print(repr(squares).split(" at ")[0], type(squares).__name__)
    nested = list(list(c for c in word) for word in ["ab", "c"])
    print(nested)


print(sum(value for value in range(5)))
module_level = (value + 1 for value in range(2))
print(repr(module_level).split(" at ")[0])
main()
