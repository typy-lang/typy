# typy-test: GEN-01, GEN-02, GEN-03
# typy-expect: accepted
class Shape:
    def area(self) -> float:
        return 2.0


class Store:
    def __init__(self, name: str) -> None:
        self.name = name

    def pair[T](self, value: T) -> tuple[str, T]:
        return (self.name, value)

    def first[T](self, items: list[T]) -> T:
        return items[0]

    def total[S: Shape](self, shapes: list[S]) -> float:
        return sum([shape.area() for shape in shapes], 0.0)

    def twice[N: (int, str)](self, value: N) -> N:
        return value + value

    @staticmethod
    def wrap[T](value: T) -> list[T]:
        return [value]

    def again[T](self, value: T) -> list[T]:
        return Store.wrap(self.first([value]))


class Shop(Store):
    def describe(self) -> str:
        return str(super().pair(3))


store = Store("s")
print(store.pair(1), store.pair("a"), store.first([1.5, 2.5]))
print(store.total([Shape(), Shape()]), store.twice(4), store.twice("ab"))
print(Store.wrap(7), store.again("x"), Shop("t").describe(), Shop("u").pair(None))
