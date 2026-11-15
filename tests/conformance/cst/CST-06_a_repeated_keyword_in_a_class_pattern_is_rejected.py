# typy-test: CST-06
# typy-expect: rejected E0103 at 11:21
class Point:
    def __init__(self, x: int, y: int) -> None:
        self.x = x
        self.y = y


point = Point(1, 2)
match point:
    case Point(x=1, x=2):
        print("twice")
