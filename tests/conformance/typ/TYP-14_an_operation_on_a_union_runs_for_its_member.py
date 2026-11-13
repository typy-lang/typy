# typy-test: TYP-14
# typy-expect: accepted
class Dog:
    def speak(self) -> str:
        return "woof"

    def __len__(self) -> int:
        return 3


class Robot:
    def speak(self) -> str:
        return "beep"

    def __len__(self) -> int:
        return 7


def describe(value: str | bytes) -> str:
    shouted = value.upper()
    return str(len(value)) + " " + str(shouted) + " " + str(value[:2])


def total(values: list[int] | set[int]) -> int:
    return len(values) + sum(values) + (1 if 2 in values else 0)


def talk(friend: Dog | Robot) -> str:
    return friend.speak() + str(len(friend))


def grow(items: list[str] | dict[str, int]) -> None:
    if isinstance(items, list):
        items.append("x")
    print(len(items), items)


print(describe("abc"), describe(b"xyz"))
print(total([1, 2, 3]), total({4, 5}))
print(talk(Dog()), talk(Robot()))
grow(["a"])
grow({"k": 1})


def wipe(items: list[str] | dict[str, int], other: str | bytes) -> None:
    items.clear()
    doubled = other * 2
    print(items, other + other, doubled)


wipe(["a"], "s")
wipe({"k": 1}, b"t")
