# typy-test: GEN-03
# typy-expect: accepted
def count[T](items: list[T]) -> int:
    if not items:
        return 0
    return 1 + count(items[1:])


def same[T](left: T, right: T) -> bool:
    return left == right


def last[T](items: list[T]) -> T:
    return items[count(items) - 1]


print(count([1, 2, 3]), count(["a"]), last([1.5, 2.5]), last(["x", "y"]))
print(same(1, 2), same("a", "a"), same(1, "a"))
