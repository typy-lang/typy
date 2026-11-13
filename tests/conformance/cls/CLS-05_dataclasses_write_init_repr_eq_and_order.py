# typy-test: CLS-05, LIB-06
# typy-expect: accepted
from dataclasses import dataclass, field
import dataclasses


@dataclass
class Point:
    x: int
    y: int = 0


@dataclass(order=True)
class Version:
    major: int
    minor: int = field(default=0)
    label: str = "release"


@dataclasses.dataclass(frozen=True)
class Token:
    kind: str
    text: str


@dataclass
class Point3(Point):
    z: int = 5


p = Point(1)
q = Point(1, 0)
print(p, q, p == q, p == Point(2), Point.y)
print(sorted([Version(2), Version(1, 5), Version(1, 2, "beta")]))
print(Version(1) < Version(1, 1), Version(3) >= Version(2, 9))
t = Token("name", "x")
print(t, t == Token("name", "x"), t.kind)
p3 = Point3(1, 2)
print(p3, p3 == Point3(1, 2, 5), p3.z)
p.x = 10
print(p)
