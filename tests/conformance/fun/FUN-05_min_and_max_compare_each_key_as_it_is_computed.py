# typy-test: FUN-05
# typy-expect: accepted
class Rank:
    def __init__(self, value: int) -> None:
        self.value = value

    def __lt__(self, other: "Rank") -> bool:
        print("compare", self.value, other.value)
        return self.value < other.value

    def __gt__(self, other: "Rank") -> bool:
        print("greater", self.value, other.value)
        return self.value > other.value


def rank(value: int) -> Rank:
    print("key", value)
    return Rank(value % 4)


values = [5, 2, 7, 4, 6]
print(min(values, key=rank))
print(max(values, key=rank))
# Which comparisons a sort makes changed in CPython 3.13, so only `min` and `max` show theirs.
print(max(1, 6, 3, key=rank), min([4, 8], key=rank))
