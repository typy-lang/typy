# typy-test: FUN-08
# typy-expect: accepted
from collections.abc import Callable


def total(values: list[int]) -> int:
    return sum(values)


measure: Callable[[str], int] = len
count: Callable[[list[int]], int] = len
add: Callable[[list[int]], int] = total
names = {count: "len", add: "total"}
print(measure is count, measure == count, count is not measure, measure != count)
print(names[count], names[total], names[len], count is add, add is total, add == total)
