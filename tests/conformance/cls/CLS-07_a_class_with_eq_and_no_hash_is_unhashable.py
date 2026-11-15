# typy-test: CLS-07
# typy-expect: rejected E0413 at 11:8
class Point:
    def __init__(self, x: int) -> None:
        self.x = x

    def __eq__(self, other: "Point") -> bool:
        return self.x == other.x


seen = {Point(1)}
