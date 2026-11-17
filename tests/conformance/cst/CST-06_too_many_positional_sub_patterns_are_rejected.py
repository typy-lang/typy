# typy-test: CST-06
# typy-expect: rejected E0401 at 12:10
class Point:
    __match_args__ = ("x", "y")

    def __init__(self, x: int, y: int) -> None:
        self.x = x
        self.y = y


match Point(1, 2):
    case Point(1, 2, 3):
        print("three")
