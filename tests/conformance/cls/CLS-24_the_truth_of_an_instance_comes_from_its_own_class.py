# typy-test: CLS-24
# typy-expect: accepted
class Shape:
    pass


class Empty(Shape):
    def __len__(self) -> int:
        print("len Empty")
        return 0


class Full(Shape):
    def __bool__(self) -> bool:
        print("bool Full")
        return True


class Sized:
    def __init__(self, n: int) -> None:
        self.n = n

    def __len__(self) -> int:
        return self.n


class Never(Sized):
    def __bool__(self) -> bool:
        return False


def test(shape: Shape) -> str:
    return "yes" if shape else "no"


print(test(Shape()), test(Empty()), test(Full()))
print(any([Sized(0), Sized(2)]), all([Sized(1), Sized(0)]), any([Sized(0)]))
s: Sized = Never(5)
print(bool(s), not s, bool(Sized(3)), bool(Sized(0)))
mixed: Sized | Full = Full()
print(bool(mixed))
items: list[Sized | Full] = [Sized(0), Full(), Sized(1)]
print([bool(item) for item in items], any(items), all(items))
try:
    print(bool(Sized(-1)))
except ValueError as error:
    print("ValueError:", error)
try:
    print(any([Sized(0), Sized(-2)]))
except ValueError as error:
    print("ValueError:", error)
