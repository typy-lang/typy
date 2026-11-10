# typy-test: STD-05
# typy-expect: accepted
def pick(items: list[int], index: int) -> int:
    return items[index]


print(pick([1, 2, 3], 1))
print(pick([1, 2, 3], 5))
