# typy-test: CLS-24
# typy-expect: accepted
class Shape:
    pass


class Empty(Shape):
    def __len__(self) -> int:
        print("len Empty")
        return 0


def describe(shape: Shape) -> str:
    if shape:
        return "true"
    return "false"


print(describe(Shape()), describe(Empty()))
