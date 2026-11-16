# typy-test: GEN-10, CLS-24
# typy-expect: accepted
from typing import Protocol


class Sized(Protocol):
    def __len__(self) -> int: ...

    def label(self) -> str: ...


class Bag:
    def __init__(self, n: int) -> None:
        self.n = n

    def __len__(self) -> int:
        return self.n

    def label(self) -> str:
        return "bag"

    def __repr__(self) -> str:
        return "Bag(" + str(self.n) + ")"


s: Sized = Bag(0)
t: Sized = Bag(3)
print(len(s), len(t), bool(s), bool(t), str(t), repr(t), [s, t])
if t:
    print("true", t.label())
print(len({s, t}))
