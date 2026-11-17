# typy-test: SMT-12
# typy-expect: accepted
import shapes
from shapes import Number, Pair, Path
from shapes import Keyed as Index


def describe(value: Number) -> str:
    if isinstance(value, int):
        return f"int {value}"
    return f"float {value}"


def swap(pair: Pair[int]) -> shapes.Pair[int]:
    return (pair[1], pair[0])


def length(path: Path) -> int:
    total = 0
    for point in path:
        total += abs(point.x) + abs(point.y)
    return total


def group(words: list[str]) -> Index[int, str]:
    table: shapes.Keyed[int, str] = {}
    for word in words:
        table.setdefault(len(word), []).append(word)
    return table


count: shapes.Number = 3
print(describe(1), describe(2.5), swap((1, 2)), length([shapes.Point(1, -2), shapes.Point(3, 4)]))
print(group(["a", "bb", "cc"]), shapes.double(count))
