# typy-test: MEM-03
# typy-expect: accepted
from dataclasses import dataclass
from typing import Protocol


class Shape(Protocol):
    def area(self) -> int: ...


class Square:
    def __init__(self, side: int) -> None:
        self.side = side

    def area(self) -> int:
        return self.side * self.side


class Manager:
    def __init__(self, name: str) -> None:
        self.name = name
        self.entered = False

    def __enter__(self) -> "Manager":
        self.entered = True
        return self

    def __exit__(self, kind, error, trace) -> None:
        self.entered = False


class Strict:
    def __init__(self, value: int) -> None:
        self.value = value
        if value < 0:
            raise ValueError("negative " + str(value))


@dataclass
class Pair:
    left: int
    right: str


class Box[T]:
    def __init__(self, item: T) -> None:
        self.item = item

    def get(self) -> T:
        return self.item


registry: list[Square] = []


class Registered:
    def __init__(self, side: int) -> None:
        self.square = Square(side)

    def register(self) -> None:
        registry.append(self.square)


def through_protocol(count: int) -> int:
    total = 0
    for index in range(count):
        shape: Shape = Square(index)
        total += shape.area()
    return total


def with_manager() -> str:
    with Manager("m") as held:
        return held.name + str(held.entered)


def strict(values: list[int]) -> int:
    total = 0
    for value in values:
        try:
            total += Strict(value).value
        except ValueError as error:
            print("refused", error)
    return total


def pairs(count: int) -> int:
    total = 0
    for index in range(count):
        pair = Pair(index, "x" * index)
        other = Pair(index, "x" * index)
        if pair == other:
            total += pair.left + len(pair.right)
    return total


def boxes(count: int) -> int:
    total = 0
    for index in range(count):
        box = Box(index)
        named = Box("n" + str(index))
        total += box.get() + len(named.get())
    return total


def unions(flag: bool) -> str:
    value: Square | None = Square(3) if flag else None
    if value is not None:
        return str(value.area())
    return "none"


def registered(side: int) -> int:
    holder = Registered(side)
    holder.register()
    return holder.square.side


def chain(count: int) -> int:
    first = Square(1)
    second = first
    for index in range(count):
        second = Square(index)
        if index % 2 == 0:
            first = second
    return first.side + second.side


class Point:
    def __init__(self, x: int, y: int, label: str) -> None:
        self.x = x
        self.y = y
        self.label = label

    def norm(self) -> int:
        return self.x * self.x + self.y * self.y


kept: list[Point] = []


def total(count: int) -> int:
    result = 0
    previous = Point(0, 0, "start")
    for index in range(count):
        current = Point(index, index + 1, "p" + str(index))
        result += current.norm() + previous.x
        previous = current
    return result


def make(x: int) -> Point:
    return Point(x, x, "made")


def keep(x: int) -> None:
    kept.append(Point(x, x, "kept"))


def recurse(depth: int) -> int:
    here = Point(depth, depth, "level " + str(depth))
    if depth == 0:
        return here.x
    return recurse(depth - 1) + here.norm()


def raising(flag: bool) -> int:
    local = Point(1, 2, "raising")
    if flag:
        raise ValueError(local.label)
    return local.y


print(through_protocol(5))
print(with_manager())
print(strict([1, -2, 3]))
print(pairs(4))
print(boxes(3))
print(unions(True), unions(False))
print(registered(7), registry[0].side)
print(chain(5))
print(total(10))
print(make(3).norm())
keep(4)
print(kept[0].label)
print(recurse(5))
try:
    raising(True)
except ValueError as error:
    print("caught", error)
print(raising(False))
