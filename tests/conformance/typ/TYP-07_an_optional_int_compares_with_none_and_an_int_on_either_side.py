# typy-test: TYP-07
# typy-expect: accepted
def describe(value: int | None) -> bool:
    return value == None


print(describe(None), describe(3))


def matches(value: int | None) -> bool:
    return 3 == value


print(matches(None), matches(3), matches(4))
