# typy-test: MEM-11
# typy-expect: accepted
from dataclasses import dataclass


@dataclass(order=True, frozen=True)
class Mixed:
    flag: bool
    count: int
    ratio: float
    done: bool
    name: str


a = Mixed(True, 3, 0.5, False, "a")
b = Mixed(True, 3, 0.25, True, "b")
print(a, a == b, a < b, b < a, {a: 1}[Mixed(True, 3, 0.5, False, "a")], a in {b, a})
match a:
    case Mixed(flag, count, ratio, done, name):
        print(flag, count, ratio, done, name)

pairs = [(True, 2, False, 1.5), (False, 1, True, 2.5)]
print(sorted(pairs), pairs[0][2], max(pairs), pairs[1] in pairs)
nested: tuple[bool, tuple[bool, int], float | None] = (False, (True, 7), None)
print(nested, nested[1][1], nested[2] is None, {nested: "key"}[nested])
counts = {(False, 1): "x", (True, 0): "y"}
for key, value in sorted(counts.items()):
    print(key, value)
