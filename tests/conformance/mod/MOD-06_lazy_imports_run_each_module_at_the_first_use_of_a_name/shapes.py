print("shapes runs")
UNIT = 1


class Point:
    def __init__(self, x: int) -> None:
        self.x = x


def area(p: Point) -> int:
    return p.x * p.x
