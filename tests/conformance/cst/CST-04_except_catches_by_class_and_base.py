# typy-test: CST-04, EXC-01
# typy-expect: accepted
def lookup(table: dict[str, int], key: str) -> int:
    try:
        return table[key]
    except LookupError:
        return -1


def first(items: list[int]) -> int:
    try:
        return items[0]
    except KeyError:
        return -2
    except IndexError as error:
        print("caught", repr(error), type(error).__name__)
    return -3


table = {"a": 1}
print(lookup(table, "a"), lookup(table, "b"))
print(first([7]), first([]))
