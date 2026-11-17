# typy-test: GEN-14
# typy-expect: rejected E0201 at 11:2
from dataclasses import dataclass


@dataclass
class Point:
    x: int


@dataclass
class Tagged[T](Point):
    tag: T


print(Tagged(1, "a") == Tagged(1, "a"))
