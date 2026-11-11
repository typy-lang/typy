print("shapes starts", __name__)
NAME = "shapes"


class Point:
    def __init__(self, x: int, y: int) -> None:
        self.x = x
        self.y = y


def area(p: Point) -> int:
    return p.x * p.y


ORIGIN = Point(0, 0)
