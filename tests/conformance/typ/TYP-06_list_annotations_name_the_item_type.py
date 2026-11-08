# typy-test: TYP-06
# typy-expect: accepted
def total(numbers: list[int]) -> int:
    result = 0
    for n in numbers:
        result += n
    return result


def words(count: int) -> list[str]:
    found: list[str] = []
    for i in range(count):
        found.append("w" + str(i))
    return found


print(total([1, 2, 3]), words(3), words(0))
