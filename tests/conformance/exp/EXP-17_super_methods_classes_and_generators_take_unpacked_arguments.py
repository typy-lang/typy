# typy-test: EXP-17
# typy-expect: accepted
from collections.abc import Iterator


class Base:
    def __init__(self, *parts: str, **marks: int) -> None:
        self.parts = parts
        self.marks = marks

    def scale(self, value: int, factor: int = 2) -> int:
        return value * factor


class Derived(Base):
    def __init__(self, *parts: str, **marks: int) -> None:
        super().__init__(*parts, **marks)

    def scale(self, value: int, factor: int = 3) -> int:
        return value * factor + 1


def numbers(*values: int) -> Iterator[int]:
    for value in values:
        yield value * 2


item: Base = Derived("a", "b", x=1)
print(item.parts, item.marks)
print(item.scale(*[5]), item.scale(**{"value": 5}), Base.scale(item, *[5, 1]))
print(list(numbers(*[1, 2, 3])))
args = ("p", "q")
print(Derived(*args, **{"y": 2}).marks)
