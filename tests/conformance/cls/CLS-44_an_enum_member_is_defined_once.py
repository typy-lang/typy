# typy-test: CLS-44
# typy-expect: rejected E0404 at 8:5
from enum import Enum


class Color(Enum):
    RED = 1
    RED = 2


print(Color.RED)
