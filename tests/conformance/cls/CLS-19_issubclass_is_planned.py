# typy-test: CLS-19
# typy-expect: rejected E0201 at 7:7
class Shape:
    pass


print(issubclass(Shape, object))
