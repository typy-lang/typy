# typy-test: EXP-10
# typy-expect: accepted
def count(items: list[int]) -> int:
    if not items:
        return 0
    return 1 + count(items[1:])


def keys(table: dict[int, int]) -> int:
    if not table:
        return 0
    return len(table) + keys({})


def members(values: set[int]) -> int:
    if not values:
        return 0
    return 1 + members(set())


def size(data: bytes) -> int:
    if not data:
        return 0
    return 1 + size(data[1:])


print(count([1, 2, 3]), keys({1: 1}), members({4}), size(b"ab"))
