# typy-test: EXP-13
# typy-expect: accepted
def first_big(values: list[int]) -> int | None:
    if (found := [v for v in values if v > 2]):
        return found[0]
    return None


def lookup(table: dict[str, int], key: str) -> int:
    if (found := table.get(key)) is not None:
        return found + 1
    return -1


def scope() -> int:
    inner = -1
    [(inner := k) for k in range(3)]
    return inner


def rebind(count: int) -> int:
    (count := count * 2)
    return count


print(first_big([1, 5, 7]), first_big([1]))
print(lookup({"a": 1}, "a"), lookup({"a": 1}, "b"))
total = 0
data = [3, 1, 4]
n = 0
while (n := len(data)) > 0:
    total += data.pop() * n
print(total, n)
y = 0
squares = [y for x in range(5) if (y := x * x) % 2 == 0]
print(squares, y)
last = ""
print([last := v for v in "abc"], last)
print(scope(), rebind(4))
value: int | None = None
print((value := 10) + 1, value)
words = ["one", "two", "three"]
size = 0
print([(size := len(word), size * 2) for word in words], size)
lines = iter(["a", "b", ""])
collected: list[str] = []
while (line := next(lines)):
    collected.append(line)
print(collected, repr(line))
print(f"{(z := 3)} {z}")
