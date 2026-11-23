# typy-test: TYP-34
# typy-expect: accepted
# Code that cannot run is neither checked nor compiled, as CPython runs it.
def label(value: int) -> str:
    if isinstance(value, str):
        return value.upper()
    return str(value)


def first(values: list[int]) -> int:
    for value in values:
        return value
        print(value.upper())
    return -1


def total(n: int) -> int:
    result = 0
    for i in range(n):
        if i % 2:
            continue
            result = result.upper()
        result += i
    return result


def checked(x: int) -> int:
    assert not isinstance(x, int), "always an int"
    return x.upper()


if True:
    ready = "yes"
print(label(3), first([4]), first([]), total(6), ready)
try:
    checked(1)
except AssertionError as error:
    print("raised:", error)
