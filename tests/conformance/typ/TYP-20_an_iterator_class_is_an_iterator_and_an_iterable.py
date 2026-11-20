# typy-test: TYP-20
# typy-expect: accepted
from collections.abc import Iterable, Iterator


class Countdown:
    def __init__(self, start: int) -> None:
        self.current = start

    def __iter__(self) -> "Countdown":
        return self

    def __next__(self) -> int:
        if self.current == 0:
            raise StopIteration
        self.current -= 1
        return self.current + 1


class Letters:
    def __init__(self, text: str) -> None:
        self.text = text

    def __iter__(self) -> Iterator[str]:
        return iter(self.text)


class Pairs:
    def __iter__(self) -> Iterator[tuple[int, str]]:
        yield (1, "a")
        yield (2, "b")


def total(values: Iterable[int]) -> int:
    result = 0
    for value in values:
        result += value
    return result


def first(values: Iterator[int]) -> int:
    return next(values)


def drain(values: Iterator[int]) -> list[int]:
    return list(values)


countdown = Countdown(3)
print(total(Countdown(4)), first(countdown), drain(countdown), next(countdown, -1))
print(sum(Countdown(3)), sorted(Countdown(3)), list(Letters("ab")), list(Pairs()))
it: Iterator[int] = Countdown(2)
print(iter(it) is it, [value * 10 for value in it], max(Countdown(5)))
source: Iterable[str] = Letters("xyz")
print(", ".join(source), list(enumerate(Letters("pq"))))
a, b = Countdown(2)
print(a, b, *Countdown(3))
