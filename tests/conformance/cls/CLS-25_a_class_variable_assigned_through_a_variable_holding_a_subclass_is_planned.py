# typy-test: CLS-25
# typy-expect: rejected E0201 at 16:5
from typing import ClassVar


class Counter:
    total: ClassVar[int] = 0


class Tally(Counter):
    pass


def reset() -> None:
    kind = Tally
    kind.total = 5


reset()
print(Counter.total, Tally.total)
