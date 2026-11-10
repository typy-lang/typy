# typy-test: TYP-13
# typy-expect: accepted
def first_even(values: list[int]) -> int | None:
    for value in values:
        if value % 2 == 0:
            return value
    return None


def double(value: int | None) -> int:
    if value is None:
        return -1
    return value * 2


def pick(a: int | None, b: int | None) -> int:
    if a is not None and b is not None:
        return a + b
    if a is None or b is None:
        return 0
    return 99


print(double(first_even([1, 3, 4])), double(first_even([1])), pick(1, 2), pick(None, 2))
n: int | None = None
count = 0
while n is None or n < 3:
    if n is None:
        n = 0
    else:
        n = n + 1
    count += 1
print(n, count)
m: int | None = 5
m = m + 1
print(m)
w: int | None = 1
for i in range(3):
    if w is not None:
        print(w + i)
    w = None
values: list[int | None] = [1, None, 3]
print([v * 2 for v in values if v is not None], sum([v for v in values if v]))
scores = {"a": 3}
s = scores.get("b", 0) + scores.get("a", 0)
print(s, {k: None for k in "ab"})
h: set[int | None] = {1, None, 2}
print(sorted([v for v in h if v is not None]), None in h, len(h))
d: dict[str, int | None] = {"x": None, "y": 2}
print(d, d["x"] is None, d.get("z", 7))
opt = d["y"]
if opt:
    print(opt + 1)
print(not opt, (opt is None) or opt > 1)
