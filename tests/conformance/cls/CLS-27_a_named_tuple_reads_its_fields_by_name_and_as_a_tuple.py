# typy-test: CLS-27
# typy-expect: accepted
from typing import NamedTuple


class Point(NamedTuple):
    x: int
    y: int = 0
    label = "pt"

    def total(self) -> int:
        return self.x + self.y


p = Point(3)
q = Point(x=1, y=5)
print(p, q, p.total(), p.label, Point._fields, p.x, q.y)
x, y = p
print(x, y, len(p), 3 in p, p[1], p[-2], p[0:1])
for item in p:
    print(item)
print(p == (3, 0), p != (3, 0), (3, 0) == p, p < (4, 0), p == q, p < q)
print(sorted([Point(2, 1), Point(1, 5)]))
print({p: 1}, {p, Point(3)}, isinstance(p, tuple), type(p).__name__, type(p))
print(p + (9,), list(p), tuple(p), max(p), sum(p), sorted(p))
print(str(p), f"{p}", repr([p]))
