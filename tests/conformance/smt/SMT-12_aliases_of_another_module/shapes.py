class Point:
    def __init__(self, x: int, y: int) -> None:
        self.x = x
        self.y = y


type Number = int | float
type Pair[T] = tuple[T, T]
type Path = list[Point]
type Keyed[K, V] = dict[K, list[V]]


def double(value: Number) -> Number:
    return value * 2
