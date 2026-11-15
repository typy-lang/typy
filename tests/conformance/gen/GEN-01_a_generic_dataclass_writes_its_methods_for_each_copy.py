# typy-test: GEN-01, CLS-05
# typy-expect: accepted
from dataclasses import dataclass


@dataclass
class Box[T]:
    v: T
    n: int


b = Box("a", 2)
print(b, b == Box("a", 2), b.v, isinstance(b, Box))
c = Box(1.5, 3)
print(c)
