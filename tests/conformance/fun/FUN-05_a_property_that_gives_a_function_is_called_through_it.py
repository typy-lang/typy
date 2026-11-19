# typy-test: FUN-05, CLS-06
# typy-expect: accepted
from typing import Callable


class Scaler:
    def __init__(self, factor: int) -> None:
        self.factor = factor

    @property
    def apply(self) -> Callable[[int], int]:
        return lambda value: value * self.factor


class Shifter(Scaler):
    @property
    def apply(self) -> Callable[[int], int]:
        return lambda value: value + self.factor


items: list[Scaler] = [Scaler(3), Shifter(3)]
print([each.apply(5) for each in items])
