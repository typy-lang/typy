# typy-test: CLS-19
# typy-expect: rejected E0401 at 7:18
class Shape:
    pass


print(issubclass(Shape(), Shape))
