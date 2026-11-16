# typy-test: CLS-21
# typy-expect: accepted
class Meters:
    def __init__(self, value: float) -> None:
        self.value = value

    def __add__(self, other: "Meters | Feet | int") -> "Meters":
        if isinstance(other, Meters):
            return Meters(self.value + other.value)
        return NotImplemented

    def __repr__(self) -> str:
        return f"Meters({self.value})"


class Feet:
    def __init__(self, value: float) -> None:
        self.value = value

    def __radd__(self, other: "Meters | int") -> Meters:
        if isinstance(other, Meters):
            return Meters(other.value + self.value * 0.5)
        return NotImplemented


class Only:
    def __radd__(self, other: "Only") -> int:
        return 1


print(Meters(1.0) + Meters(2.0))
print(Meters(1.0) + Feet(10.0))
try:
    print(Meters(1.0) + 5)
except TypeError as error:
    print("TypeError:", error)
try:
    print(5 + Feet(1.0))
except TypeError as error:
    print("TypeError:", error)
try:
    print(Only() + Only())
except TypeError as error:
    print("TypeError:", error)
