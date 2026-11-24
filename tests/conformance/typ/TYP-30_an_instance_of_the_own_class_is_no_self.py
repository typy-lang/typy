# typy-test: TYP-30
# typy-expect: rejected E0433 at 8:16
from typing import Self


class Shape:
    def copy(self) -> Self:
        return Shape()


print(Shape().copy())
