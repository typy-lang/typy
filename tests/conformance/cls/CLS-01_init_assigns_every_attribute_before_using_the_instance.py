# typy-test: CLS-01
# typy-expect: rejected E0309 at 6:9
class Point:
    def __init__(self, x: int) -> None:
        self.x = x
        self.show()
        self.y = 0

    def show(self) -> None:
        print(self.x, self.y)
