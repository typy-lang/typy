# typy-test: TYP-08
# typy-expect: rejected E0414 at 10:1
from typing import Final


class Box:
    SIZE: Final[int] = 3


Box.SIZE = 4
