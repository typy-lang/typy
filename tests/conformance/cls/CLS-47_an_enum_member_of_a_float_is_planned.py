# typy-test: CLS-47
# typy-expect: rejected E0201 at 7:12
from enum import Enum


class Ratio(Enum):
    HALF = 0.5


print(Ratio.HALF)
