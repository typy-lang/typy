# typy-test: CLS-09
# typy-expect: rejected E0401 at 7:27
class Shape:
    pass


print(isinstance(Shape(), 3))
