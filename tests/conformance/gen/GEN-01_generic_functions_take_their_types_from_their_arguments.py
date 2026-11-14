# typy-test: GEN-01, GEN-02, GEN-03
# typy-expect: accepted
class Shape:
    def __init__(self, name: str) -> None:
        self.name = name

    def area(self) -> float:
        return 1.0


class Square(Shape):
    def __init__(self, side: float) -> None:
        super().__init__("square")
        self.side = side

    def area(self) -> float:
        return self.side * self.side


def first[T](items: list[T]) -> T:
    return items[0]


def pair[K, V](key: K, value: V) -> tuple[K, V]:
    return (key, value)


def largest[S: Shape](shapes: list[S]) -> S:
    best = shapes[0]
    for shape in shapes:
        if shape.area() > best.area():
            best = shape
    return best


def doubled[N: (int, float)](value: N) -> N:
    return value + value


def keep[T](value: T, fallback: T | None) -> T:
    if fallback is None:
        return value
    return fallback


def repeat[T](value: T, times: int) -> list[T]:
    result: list[T] = []
    for _ in range(times):
        result.append(value)
    return result


print(first([1, 2]), first(["a"]), first([[1.5]]))
print(pair("a", 1), pair(2.5, [True]))
print(largest([Square(1.0), Square(3.0)]).side, largest([Shape("s")]).name)
print(doubled(3), doubled(1.25))
print(keep(1, None), keep("a", "b"))
print(repeat("x", 3), repeat(first([7]), 2))
