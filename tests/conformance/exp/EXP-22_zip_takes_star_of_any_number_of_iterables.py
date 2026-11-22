# typy-test: EXP-22
# typy-expect: accepted
rows = [[1, 2, 3], [4, 5, 6]]
print(list(zip(*rows)))
for column in zip(*rows):
    print(column, sum(column))
pairs = [(1, 2), (3, 4), (5, 6)]
print(list(zip(*pairs)))
print(list(zip(*["ab", "cd"])), list(zip(*[[1, 2], [3]])))
nothing: list[list[int]] = []
print(list(zip(*nothing)))
for shape in [[[1, 2], [3]], [[1], [2, 3]], [[1, 2], [3, 4], [5]], [[1], [2], [3, 4]], [[1, 2], [3, 4]]]:
    try:
        print(list(zip(*shape, strict=True)))
    except ValueError as error:
        print(error)
columns = zip(*rows)
print(next(columns), type(columns).__name__)
from collections.abc import Iterator


class Broken:
    def __iter__(self) -> Iterator[int]:
        raise ValueError("no items")


def counted(limit: int) -> Iterator[int]:
    for index in range(limit):
        yield index
    raise KeyError("done")


try:
    print(list(zip(*[Broken()])))
except ValueError as error:
    print(error)
try:
    print(list(zip(*[counted(2), counted(3)])))
except KeyError as error:
    print("key", error)
words = {"a": 1, "b": 2}
print(list(zip(*[words, words])), list(zip(*[words.items()])))
