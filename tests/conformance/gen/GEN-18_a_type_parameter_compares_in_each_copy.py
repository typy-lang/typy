# typy-test: GEN-18
# typy-expect: accepted
# A generic function compares values of its type parameter, and each copy compares them as
# its types do: numbers, text, tuples item by item, lists and sets.
def first_over[T](items: list[T], limit: T) -> int:
    for index, item in enumerate(items):
        if limit < item:
            return index
    return len(items)


def smallest[T](items: list[T]) -> T:
    return min(items)


print(first_over([1, 5, 9], 4), first_over(["a", "c"], "b"), smallest([3.5, 1.5]))
print(first_over([(1, "x"), (2, "a")], (1, "z")), smallest([[2], [1, 9]]))
print(first_over([{1}, {1, 2}], {1}), sorted([(2, "b"), (1, "c")]) == [(1, "c"), (2, "b")])
