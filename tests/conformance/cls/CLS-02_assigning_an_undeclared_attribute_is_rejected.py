# typy-test: CLS-02
# typy-expect: rejected E0414 at 7:7
class Point:
    def __init__(self) -> None:
        self.x = 0
point = Point()
point.z = 1
