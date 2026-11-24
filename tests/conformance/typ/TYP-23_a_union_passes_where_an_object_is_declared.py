# typy-test: TYP-23
# typy-expect: accepted
# A value of a union is held as an object of its member's type where `object` is declared.
def f(v: object) -> bool:
    return isinstance(v, str)


def g(value: int | str) -> bool:
    return f(value)


print(g(1), g("a"))
