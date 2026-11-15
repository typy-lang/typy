# typy-test: TYP-14
# typy-expect: accepted
items: set[str | bytes] = {"a", b"a"}
print(len(items))
values: list[str | bytes] = ["a", b"a"]
print(values.count("a"), values.index(b"a"))
table: dict[str | bytes, int] = {"k": 1, b"k": 2}
print(len(table), table)


class Feet:
    def __init__(self, value: int) -> None:
        self.value = value


class Meters:
    def __init__(self, value: int) -> None:
        self.value = value

    def __eq__(self, other: "Meters") -> bool:
        return other.value == self.value


feet = Feet(3)
lengths: list[Meters | Feet] = [Meters(3), feet]
print(lengths.index(Meters(3)), Meters(4) in lengths, feet in lengths)
