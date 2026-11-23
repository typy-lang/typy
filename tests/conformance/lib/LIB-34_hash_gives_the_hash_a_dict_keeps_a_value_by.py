# typy-test: LIB-34
# typy-expect: accepted
# hash gives CPython's hash of numbers, None and tuples of them, what __hash__ returns for an
# instance, and in a copy of a generic class raises TypeError for a value without a hash.
from dataclasses import dataclass


class Point:
    def __init__(self, x: int, y: int) -> None:
        self.x = x
        self.y = y

    def __eq__(self, other: "Point") -> bool:
        return (self.x, self.y) == (other.x, other.y)

    def __hash__(self) -> int:
        return hash((self.x, self.y))


@dataclass(frozen=True)
class Pair:
    left: int
    right: str


class Box[T]:
    def __init__(self, item: T) -> None:
        self.item = item

    def key(self) -> int:
        return hash(self.item)


print(hash(1), hash(-1), hash(2**61), hash(-(2**70)), hash(1.5), hash(-0.0), hash(True))
print(hash(None), hash((1, 2.0, None)), hash(()), hash(((1, 2), (3,))))
print(hash(Point(1, 2)) == hash((1, 2)), len({Point(1, 2), Point(1, 2)}))
print(hash("text") == hash("text"), hash(b"text") == hash(b"text"), hash(Pair(1, "a")) == hash(Pair(1, "a")))
print(Box(3).key(), Box((1, 2)).key() == hash((1, 2)))
for box in (Box([1]), Box([2])):
    try:
        box.key()
    except TypeError as error:
        print("TypeError", error)
