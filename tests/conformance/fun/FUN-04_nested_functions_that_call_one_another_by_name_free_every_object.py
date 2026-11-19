# typy-test: FUN-04
# typy-expect: accepted
from typing import Callable


def parity(n: int) -> str:
    def even(k: int) -> bool:
        return True if k == 0 else odd(k - 1)

    def odd(k: int) -> bool:
        return False if k == 0 else even(k - 1)
    return "even" if even(n) else "odd"


def labels(words: list[str], limit: int | None) -> list[str]:
    assert limit is not None
    seen: list[str] = []

    def first(i: int) -> list[str]:
        if i >= len(words) or i >= limit:
            return seen
        seen.append(words[i].upper())
        return second(i + 1)

    def step() -> int:
        return 1

    def second(i: int) -> list[str]:
        if i >= len(words):
            return seen
        seen.append(words[i] + suffix)
        return first(i + step())
    suffix = "!"
    return first(0)


def kept() -> Callable[[int], str]:
    def ping(k: int) -> str:
        return "ping" if k == 0 else pong(k - 1)

    def pong(k: int) -> str:
        return "pong" if k == 0 else ping(k - 1)
    return pong


def in_a_loop() -> list[int]:
    out: list[int] = []
    for i in range(3):
        def down(k: int) -> int:
            return k if k <= 0 else across(k - 1)

        def across(k: int) -> int:
            return i if k <= 0 else down(k - 1)
        out.append(down(i + 1))
    return out


def three(n: int) -> list[str]:
    trail: list[str] = []

    def a(k: int) -> None:
        trail.append("a")
        if k > 0:
            b(k - 1)

    def b(k: int) -> None:
        trail.append("b")
        if k > 0:
            c(k - 1)

    def c(k: int) -> None:
        trail.append("c")
        if k > 0:
            a(k - 1)
    a(n)
    return trail


def fails(n: int) -> int:
    def down(k: int) -> int:
        if k == 0:
            raise ValueError("bottom")
        return up(k - 1)

    def up(k: int) -> int:
        return down(k)
    return down(n)


print(parity(7), parity(10))
print(labels(["a", "b", "c", "d"], 3))
answer = kept()
print(answer(3), answer(4))
print(in_a_loop())
print(three(7))
try:
    fails(5)
except ValueError as error:
    print("ValueError:", error)
try:
    print(parity(10 ** 8))
except RecursionError:
    print("RecursionError")
