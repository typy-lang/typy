# typy-test: FUN-11, FUN-07
# typy-expect: accepted
from collections.abc import Iterator


def g(name: str) -> Iterator[int]:
    try:
        yield 1
    finally:
        print("closing", name)


def started(name: str) -> Iterator[int]:
    made = g(name)
    next(made)
    return made


def dropped_list() -> None:
    items = [started("list 1"), started("list 2"), started("list 3")]
    print("list made", len(items))


def cleared_list() -> None:
    items = [started("clear 1"), started("clear 2")]
    items.clear()
    print("after clear", items)


def dropped_dict() -> None:
    table = {"a": started("dict a"), "b": started("dict b")}
    print("dict made", len(table))


def dropped_tuple() -> None:
    pair = (started("tuple 1"), started("tuple 2"))
    print("tuple made", len(pair))


class Holder:
    def __init__(self) -> None:
        self.first = started("attribute first")
        self.second = started("attribute second")


def dropped_instance() -> None:
    holder = Holder()
    print("holder made", holder is not None)


def replaced() -> None:
    items = [started("item 0")]
    items[0] = started("item 0 again")
    print("replaced")
    popped = items.pop()
    print("popped")
    popped = started("replacement")
    print("rebound")


dropped_list()
cleared_list()
dropped_dict()
dropped_tuple()
dropped_instance()
replaced()
for value in started("loop"):
    print("loop", value)
    break
print("after loop")
