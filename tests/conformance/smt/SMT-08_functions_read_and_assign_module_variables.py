# typy-test: SMT-08, MOD-03
# typy-expect: accepted
count = 0
names: list[str] = []
limit: int
label: str | None = None


def bump(step: int) -> int:
    global count
    count += step
    return count


def show() -> None:
    print("count", count, "names", names, "limit", limit)


def remember(name: str) -> None:
    names.append(name)


def relabel(text: str) -> None:
    global label
    label = text


def early() -> None:
    print(late)


try:
    early()
except NameError as error:
    print("NameError", error)
try:
    show()
except NameError as error:
    print("NameError", error)
limit = 3
print(bump(2), bump(5), count)
remember("ada")
remember("alan")
show()
relabel("x")
print(label)
current = label
if current is not None:
    print(current.upper())
for count in range(3):
    pass
show()
late = "now"
early()
pairs = [(1, "a")]
for number, letter in pairs:
    print(number, letter)
first, second = 10, 20
print(first + second)
