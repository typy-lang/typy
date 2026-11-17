# typy-test: CLS-23
# typy-expect: accepted
class Bag:
    def __init__(self, items: list[int]) -> None:
        self.items = items
        self.at = 0

    def __iter__(self) -> "Bag":
        self.at = 0
        return self

    def __next__(self) -> int:
        if self.at >= len(self.items):
            raise StopIteration
        self.at += 1
        print("next", self.at)
        return self.items[self.at - 1]


class Point:
    def __init__(self, x: int) -> None:
        self.x = x

    def __eq__(self, other: "Point") -> bool:
        print("eq", self.x, other.x)
        return self.x == other.x


class Points:
    def __init__(self, points: list[Point]) -> None:
        self.points = points

    def __iter__(self) -> "PointWalk":
        return PointWalk(self.points)


class PointWalk:
    def __init__(self, points: list[Point]) -> None:
        self.points = points
        self.at = 0

    def __next__(self) -> Point:
        if self.at == len(self.points):
            raise StopIteration
        self.at += 1
        return self.points[self.at - 1]


bag = Bag([1, 2, 3])
print(2 in bag, 5 in bag, 2 not in bag, 3 not in Bag([]))
p = Point(2)
group = Points([Point(1), p, Point(3)])
print(p in group, Point(3) in group, Point(9) not in group)


def has(value: int, where: Bag) -> bool:
    return value in where


print(has(1, bag), [x in bag for x in [3, 4]])
