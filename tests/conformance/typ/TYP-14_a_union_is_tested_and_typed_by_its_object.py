# typy-test: TYP-14
# typy-expect: accepted
class A:
    pass


class B:
    def __bool__(self) -> bool:
        return False

    def __len__(self) -> int:
        return 0


def show(v: str | list[int] | None) -> None:
    if v:
        print("true", type(v), v)
    else:
        print("false", type(v), v)


def same(x: A | B, y: A | B) -> bool:
    return x is y


show("ab")
show("")
show([])
show([1])
show(None)
a = A()
print(same(a, a), same(a, B()))
w: A | B = A()
print(bool(w), type(w).__name__, not w)
