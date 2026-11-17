# typy-test: CLS-33
# typy-expect: rejected E0201 at 7:9
class Cell:
    def __init__(self, value: int) -> None:
        self.value = value

    def __add__(self, other: "Cell") -> "Cell | None":
        if other.value < 0:
            return NotImplemented
        if other.value == 0:
            return None
        return Cell(self.value + other.value)


result = Cell(1) + Cell(2)
print(result is None, Cell(1) + Cell(0))
try:
    print(Cell(1) + Cell(-1))
except TypeError as error:
    print("TypeError:", error)
