# typy-test: CLS-27
# typy-expect: accepted
from typing import NamedTuple


class P(NamedTuple):
    x: int
    y: str = "a"
    z: int | None = None


class Q(P):
    def twice(self) -> int:
        return 2 * self.x


class E(NamedTuple):
    pass


class Pair[T](NamedTuple):
    first: T
    second: T


def bump(value: P) -> P:
    return value._replace(x=value.x + 1)


p = P(1)
print(p._replace(y="b"), p._replace(), p._replace(x=5, y="c", z=None), p._replace(z=3))
print(Q(2)._replace(x=3), bump(Q(4)), bump(p), type(bump(Q(4))).__name__)
print(p._asdict(), Q(4)._asdict(), E()._asdict(), E()._replace())
print(Pair(1, 2)._replace(second=5), Pair("a", "b")._asdict())
