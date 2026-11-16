# typy-test: GEN-09, GEN-04, CLS-25
# typy-expect: accepted
from typing import Protocol


class Sized(Protocol):
    def size(self) -> int: ...


class Item:
    def __init__(self, n: int) -> None:
        self.n = n

    def size(self) -> int:
        return self.n


class Registry:
    def __init__(self, name: str) -> None:
        self.name = name

    @classmethod
    def make[U](cls, name: str, extra: U) -> "tuple[Registry, U]":
        return (cls(name), extra)

    @classmethod
    def measure(cls, thing: Sized) -> int:
        return thing.size() * 2


class Special(Registry):
    pass


class Box[T]:
    def __init__(self, item: T) -> None:
        self.item = item

    @classmethod
    def pair[U](cls, item: T, other: U) -> "tuple[Box[T], U]":
        return (cls(item), other)


r, e = Registry.make("r", 1.5)
s, f = Special.make("s", "x")
print(r.name, e, s.name, f, type(s).__name__, Registry.measure(Item(3)))
b, o = Box.pair(1, "o")
c, p = Box[str].pair("c", 2)
print(b.item, o, c.item, p)
