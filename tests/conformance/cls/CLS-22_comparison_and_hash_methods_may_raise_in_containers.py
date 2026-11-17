# typy-test: CLS-22
# typy-expect: accepted
class Key:
    def __init__(self, n: int) -> None:
        self.n = n

    def __eq__(self, other: "Key") -> bool:
        print("eq", self.n, other.n)
        if self.n < 0 or other.n < 0:
            raise ValueError(f"bad key {self.n} {other.n}")
        return self.n == other.n

    def __hash__(self) -> int:
        if self.n == 99:
            raise KeyError("no hash")
        return self.n % 3

    def __lt__(self, other: "Key") -> bool:
        print("lt", self.n, other.n)
        if other.n == 0:
            raise ArithmeticError("zero")
        return self.n < other.n

    def __bool__(self) -> bool:
        if self.n == 7:
            raise RuntimeError("seven")
        return self.n > 0


keys = [Key(1), Key(-2), Key(3)]
print(Key(3) in keys[2:])
for attempt in ["in", "index", "count", "remove", "set", "dict", "hash", "sort", "min", "any", "lt"]:
    try:
        if attempt == "in":
            print(Key(3) in keys)
        elif attempt == "index":
            print(keys.index(Key(3)))
        elif attempt == "count":
            print(keys.count(Key(1)))
        elif attempt == "remove":
            keys.remove(Key(3))
        elif attempt == "set":
            found = {Key(1), Key(4), Key(-1)}
            print(len(found))
        elif attempt == "dict":
            table = {Key(1): "a"}
            table[Key(-4)] = "b"
            print(len(table))
        elif attempt == "hash":
            print(len({Key(5), Key(99)}))
        elif attempt == "sort":
            print([key.n for key in sorted([Key(0), Key(3)])])
        elif attempt == "min":
            print(min([Key(2), Key(0)]).n)
        elif attempt == "any":
            print(any([Key(0), Key(7), Key(1)]))
        else:
            print(Key(1) < Key(0))
    except (ValueError, KeyError, ArithmeticError, RuntimeError) as error:
        print(attempt, type(error).__name__, error)
print([key.n for key in keys])
table = {Key(1): "a"}
try:
    table[Key(-4)] = "b"
except ValueError:
    pass
print(len(table), sorted([Key(2), Key(1)])[0].n)

