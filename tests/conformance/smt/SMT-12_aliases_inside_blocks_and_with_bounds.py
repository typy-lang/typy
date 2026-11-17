# typy-test: SMT-12
# typy-expect: accepted
type Number = int | float


def scale(factor: int) -> list[int]:
    type Row = list[int]
    row: Row = [factor, factor * 2]
    if factor > 1:
        type Grid = list[Row]
        grid: Grid = [row, row]
        return grid[1]
    return row


class Table:
    type Cell = str | None

    def __init__(self) -> None:
        self.cells: list[str | None] = ["a", None]

    def first(self) -> str | None:
        type Pair[T] = tuple[T, T]
        pair: Pair[str | None] = (self.cells[0], self.cells[1])
        return pair[0]


type Bounded[T: (int, str)] = list[T]
type Ordered[T: float] = tuple[T, T]
names: Bounded[str] = ["a"]
span: Ordered[float] = (1.0, 2.5)
for step in range(2):
    type Total = Number
    total: Total = step * 1.5
    print(total)
print(scale(1), scale(3), Table().first(), names, span)
