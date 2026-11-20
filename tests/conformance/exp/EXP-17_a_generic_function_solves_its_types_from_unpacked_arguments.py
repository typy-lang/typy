# typy-test: EXP-17
# typy-expect: accepted
def first[T](*items: T) -> T:
    return items[0]


def pick[T](a: T, b: T) -> T:
    return b


def value[T](**options: T) -> list[T]:
    return list(options.values())


numbers = [3, 4]
print(first(*numbers) + 1, pick(*numbers) + 1, pick(*("a", "b")).upper())
print(value(**{"x": 1.5}), pick(**{"a": 1, "b": 2}) + 1)
