# typy-test: TYP-08
# typy-expect: rejected E0414 at 11:9
from typing import Final


class Box:
    def __init__(self, width: int) -> None:
        self.width: Final = width

    def grow(self) -> None:
        self.width = self.width + 1
