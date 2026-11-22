# typy-test: CLS-12
# typy-expect: accepted
def pick(kind: int) -> None | int | tuple[int, ...]:
    if kind == 0:
        return None
    if kind == 1:
        return 7
    return tuple([4, 5])


def named(kind: int) -> str:
    value = pick(kind)
    return type(value).__name__


def shown(kind: int) -> None | str | list[int]:
    if kind == 0:
        return None
    if kind == 1:
        return "text"
    return [1]


for kind in range(3):
    value = shown(kind)
    print(named(kind), type(value).__name__, type(value) is list)
