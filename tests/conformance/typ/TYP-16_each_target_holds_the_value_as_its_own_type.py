# typy-test: TYP-16
# typy-expect: accepted
class Cat:
    def __init__(self, name: str) -> None:
        self.name = name


class Dog:
    def __init__(self, name: str) -> None:
        self.name = name


class Home:
    def __init__(self) -> None:
        self.pet: Cat | Dog | None = None
        self.size: int | float = 0


home = Home()
pets: list[Cat | Dog] = [Dog("rex")]
named: dict[str, Cat | Dog | None] = {}
tom = home.pet = pets[0] = named["tom"] = Cat("tom")
print(tom.name, type(home.pet).__name__, type(pets[0]).__name__, named["tom"] is tom)
count: int | str
label: int | str | None
(count, home.size), (label, named["x"]) = (1, 2), (3, None)
print(count, home.size, label, sorted(named), named["x"])
last = 0.0
for slot in range(2):
    home.size = last = slot * 1.5
print(home.size, last)
scores: dict[str, int | float] = {}
total = scores["a"] = scores["b"] = 10
scores["b"] = total / 4
print(scores)
