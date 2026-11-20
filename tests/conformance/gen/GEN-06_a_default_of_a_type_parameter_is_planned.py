# typy-test: GEN-06
# typy-expect: rejected E0201 at 4:11
# typy-python: 3.13
def first[T = int](items: list[T]) -> T:
    return items[0]


print(first([1]))
