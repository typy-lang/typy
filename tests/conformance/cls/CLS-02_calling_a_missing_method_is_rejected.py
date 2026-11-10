# typy-test: CLS-02
# typy-expect: rejected E0414 at 6:7
class Point:
    pass
point = Point()
point.move()
