# typy-test: CLS-11
# typy-expect: accepted
from typing import ClassVar

PREFIX = "item"


class Counter:
    created = 0
    label: ClassVar[str] = PREFIX + "-counter"
    step: int = 1
    tags: list[str] = []

    def __init__(self, start: int) -> None:
        self.value = start
        Counter.created += 1

    def advance(self) -> int:
        self.value += self.step
        return self.value


class Fast(Counter):
    step: int = 5

    def __init__(self) -> None:
        super().__init__(10)
        self.step = 7


a = Counter(0)
b = Counter(3)
f = Fast()
a.tags.append("shared")
print(Counter.created, a.created, Fast.created, Counter.label, f.label)
print(a.advance(), b.advance(), f.advance(), a.step, Counter.step, Fast.step)
print(b.tags, f.tags, Counter.tags)


def bump() -> None:
    Counter.created = Counter.created + 100


bump()
print(Counter.created, f.created)


class Slow(Counter):
    step: int = 2


slow = Slow(1)
print(slow.advance(), slow.advance(), Slow.step, Counter.created)
