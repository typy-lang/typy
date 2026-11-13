# typy-test: CLS-09
# typy-expect: rejected E0201 at 8:27
class Shape:
    pass


kind = Shape
print(isinstance(Shape(), kind))
