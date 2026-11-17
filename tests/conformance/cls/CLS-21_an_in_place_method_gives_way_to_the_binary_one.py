# typy-test: CLS-21
# typy-expect: accepted
class Counter:
    def __init__(self, count: int) -> None:
        self.count = count

    def __iadd__(self, other: "Counter | int | str") -> "Counter":
        print("iadd")
        if isinstance(other, int):
            self.count += other
            return self
        return NotImplemented

    def __add__(self, other: "Counter | int | str") -> "Counter":
        print("add")
        if isinstance(other, Counter):
            return Counter(self.count + other.count)
        return NotImplemented

    def __pow__(self, other: int) -> "Counter":
        return NotImplemented

    def __ipow__(self, other: int) -> "Counter":
        return NotImplemented


c = Counter(1)
c += 2
print(c.count)
c += Counter(5)
print(c.count)
try:
    c += "x"
except TypeError as error:
    print("TypeError:", error)
try:
    c **= 2
except TypeError as error:
    print("TypeError:", error)
try:
    print(c ** 2)
except TypeError as error:
    print("TypeError:", error)
print(c.count)
