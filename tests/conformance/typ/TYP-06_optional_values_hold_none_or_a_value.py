# typy-test: TYP-06, EXP-10
# typy-expect: accepted
def find(words: list[str], wanted: str) -> int | None:
    for index, word in enumerate(words):
        if word == wanted:
            return index
    return None


def describe(value: int | None = None) -> str:
    if value is None:
        return "nothing"
    return "got " + str(value + 1)


words = ["a", "b", "c"]
print(find(words, "b"), find(words, "z"), describe(), describe(4))
x: int | None = find(words, "c")
if x is not None:
    print(x * 10)
y = find(words, "q")
print(y is None, y is not None, y, repr(y), str(y))
counts = {"a": 1}
got = counts.get("b", 0)
total: int | None = None
for n in [3, 4]:
    if total is None:
        total = n
    else:
        total = total + n
print(total)
names: list[str | None] = ["x", None, "z"]
print(names, [n for n in names if n is not None], len([n for n in names if n]))
f: float | None = 1.5
b: bool | None = None
t: tuple[int, str] | None = (1, "a")
print(f, b, t, None if f is None else f * 2)
if t is not None:
    print(t[1])
flags: list[bool | None] = [True, None, False]
pairs: list[tuple[int, int] | None] = [(1, 2), None]
print(flags, pairs)
s: str | None = ""
print(bool(s), bool(None), "yes" if s else "no", s is not None and len(s) == 0)
z: int | None = 2 ** 70
print(z, z is not None and z > 5)
