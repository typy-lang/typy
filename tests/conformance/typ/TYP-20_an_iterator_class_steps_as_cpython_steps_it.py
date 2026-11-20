# typy-test: TYP-20
# typy-expect: accepted
from collections.abc import Iterable, Iterator


class Done(StopIteration):
    pass


class Steps:
    def __init__(self, count: int, ending: str = "plain") -> None:
        self.count = count
        self.ending = ending

    def __iter__(self) -> "Steps":
        print("iter called")
        return self

    def __next__(self) -> int:
        if self.count == 0:
            if self.ending == "value":
                raise StopIteration(42)
            if self.ending == "derived":
                raise Done()
            if self.ending == "error":
                raise ValueError("broken")
            raise StopIteration
        self.count -= 1
        return self.count


class Pairs:
    def __init__(self) -> None:
        self.at = 0

    def __iter__(self) -> "Pairs":
        return self

    def __next__(self) -> tuple[int, str]:
        self.at += 1
        if self.at > 2:
            raise StopIteration
        return (self.at, "x" * self.at)


class Maybe:
    def __init__(self) -> None:
        self.values: list[int | None] = [1, None, 3]

    def __iter__(self) -> "Maybe":
        return self

    def __next__(self) -> int | None:
        if not self.values:
            raise StopIteration
        return self.values.pop(0)


class Repeat[T]:
    def __init__(self, value: T, times: int) -> None:
        self.value = value
        self.times = times

    def __iter__(self) -> "Repeat[T]":
        return self

    def __next__(self) -> T:
        if self.times == 0:
            raise StopIteration
        self.times -= 1
        return self.value


class Loud(Steps):
    pass


def walk(values: Iterator[int]) -> list[int]:
    found: list[int] = []
    for value in values:
        found.append(value)
    return found


def pairs(values: Iterable[tuple[int, str]]) -> dict[int, str]:
    return dict(values)


def words(values: Iterator[str]) -> str:
    return "-".join(values)


it: Iterator[int] = Steps(3)
for value in it:
    print("got", value)
    break
print(next(it), walk(it))
try:
    next(Steps(0, "value"))
except StopIteration as error:
    print("stop", error.args)
print(walk(Steps(2, "derived")))
try:
    walk(Steps(1, "error"))
except ValueError as error:
    print("error", error)
print(pairs(Pairs()), list(Maybe()), words(Repeat("ab", 3)), walk(Loud(2)))
maybe: Iterable[int | None] = Maybe()
print([value for value in maybe], next(Steps(1), -1), next(Steps(0), -1))
