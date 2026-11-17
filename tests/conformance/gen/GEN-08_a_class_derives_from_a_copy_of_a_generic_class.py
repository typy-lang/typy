# typy-test: GEN-08, GEN-03, CLS-04
# typy-expect: accepted
class Base:
    def __init__(self, name: str) -> None:
        self.name = name

    def label(self) -> str:
        return "base " + self.name


class Box[T](Base):
    def __init__(self, item: T) -> None:
        super().__init__("box")
        self.item = item

    def get(self) -> T:
        return self.item

    def show(self) -> str:
        return self.label() + " " + str(self.get())


class IntBox(Box[int]):
    def __init__(self, item: int) -> None:
        super().__init__(item * 2)
        self.extra = 1

    def get(self) -> int:
        return self.item + self.extra


class Pair[T](Box[T]):
    def __init__(self, item: T, other: T) -> None:
        super().__init__(item)
        self.other = other

    def both(self) -> tuple[T, T]:
        return (self.get(), self.other)


class Labeled[T](Box[list[T]]):
    def __init__(self, items: list[T]) -> None:
        super().__init__(items)

    def count(self) -> int:
        return len(self.get())


def read(box: Box[int]) -> int:
    return box.get()


i = IntBox(3)
p = Pair("a", "b")
q = Pair(1, 2)
lab = Labeled([1.5, 2.5])
print(i.get(), i.show(), read(i), read(q), read(Box(9)))
print(p.both(), q.both(), p.show(), lab.count(), lab.get())
print(isinstance(i, Box), isinstance(i, Base), isinstance(p, Box), type(p) is Pair, type(i) is IntBox)
things: list[Base] = [i, p, q, lab, Base("b")]
print([each.label() for each in things])
boxes: list[Box[int]] = [i, q, Box(4)]
print([each.get() for each in boxes])
