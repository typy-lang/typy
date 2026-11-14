# typy-test: GEN-09
# typy-expect: rejected E0201 at 12:9
from typing import Protocol


class Measured(Protocol):
    def size(self) -> int: ...


class Box:
    @classmethod
    def total(cls, item: Measured) -> int:
        return item.size()
