# typy-test: TYP-24
# typy-expect: accepted
from typing import cast


class Shape:
    def area(self) -> float:
        return 0.0


class Square(Shape):
    def __init__(self, side: float) -> None:
        self.side = side

    def area(self) -> float:
        return self.side * self.side


record: dict[str, object] = {"name": "ada", "age": 36, "ratio": 0.5}
name = cast(str, record["name"])
age = cast(int, record["age"])
ratio = cast(float, record["ratio"])
print(name.upper(), age + 1, ratio * 2)
shape: Shape = Square(3.0)
square = cast(Square, shape)
print(square.side, cast(Shape, square).area())
maybe: str | None = "x"
print(cast(str, maybe).upper())
mixed: int | str = 4
print(cast(int, mixed) + 1)
