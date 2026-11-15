# typy-test: TYP-14
# typy-expect: accepted
class Box:
    def __init__(self, size: int) -> None:
        self.size = size

    def grow(self) -> int:
        return self.size + 1


def doubled(value: int | str) -> int | str:
    return value * 2


def length(value: str | list[int] | tuple[int, int]) -> int:
    return len(value)


def bigger(value: int | float) -> bool:
    return value > 2


def grown(value: Box | int) -> int:
    if isinstance(value, Box):
        return value.grow()
    return value + 1


def show(values: list[int | str]) -> None:
    for value in values:
        print(value * 3, value == 1, value != "a")


print(doubled(4), doubled("ab"))
print(length("abc"), length([1, 2]), length((1, 2)))
print(bigger(3), bigger(1.5), bigger(2))
print(grown(Box(2)), grown(5))
show([1, "a", 2])
total: int | float = 1
total = total + 2.5
print(total)
