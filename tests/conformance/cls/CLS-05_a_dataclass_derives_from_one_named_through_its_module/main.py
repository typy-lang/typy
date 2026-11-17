# typy-test: CLS-05, MOD-10
# typy-expect: accepted
from dataclasses import dataclass

import shapes


@dataclass
class Labeled(shapes.Point):
    label: str = "p"


a = Labeled(1, 2, "a")
print(a, Labeled(3), a == Labeled(1, 2, "a"), a == Labeled(1), a.x + a.y)
