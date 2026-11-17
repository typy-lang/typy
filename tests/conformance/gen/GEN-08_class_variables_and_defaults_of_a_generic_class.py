# typy-test: GEN-08, CLS-11, CLS-05
# typy-expect: accepted
from dataclasses import dataclass
from typing import ClassVar


class Counter[T]:
    made = 0
    kind: ClassVar[str] = "counter"
    size: int = 3
    label: str = "none"

    def __init__(self, value: T) -> None:
        self.value = value
        Counter.made += 1
        if Counter.made > 1:
            self.size = Counter.made

    def describe(self) -> str:
        return self.kind + " " + str(self.size) + " " + self.label

    @staticmethod
    def count() -> int:
        return Counter.made


@dataclass
class Entry[K]:
    key: K
    hits: int = 0
    note: str = "-"


a = Counter(1)
b = Counter("x")
c: Counter[float] = Counter(2.5)
print(Counter.made, a.describe(), b.describe(), c.describe())
print(a.made, b.kind, Counter.count(), a.value, b.value)
b.label = "b"
print(a.label, b.label, Counter.label)
Counter.kind = "renamed"
print(a.kind, c.kind)
e = Entry("k")
f = Entry(1, 5)
print(e, f, e.hits + f.hits, Entry.note)
