# typy-test: EXP-15, FUN-04
# typy-expect: accepted
from collections.abc import Iterator


def late() -> None:
    factor = 1
    scaled = (value * factor for value in range(3))
    factor = 10
    print(list(scaled))


def per_iteration() -> None:
    made = [(index for _ in range(1)) for index in range(3)]
    print([next(each) for each in made])


def unbound_iterable() -> None:
    values = [1, 2]
    sums = (value + offset for value in values for offset in [100])
    values.append(3)
    print(list(sums))


def raises_in_element() -> Iterator[int]:
    return (10 // value for value in [1, 0])


late()
per_iteration()
unbound_iterable()
failing = raises_in_element()
print(next(failing))
try:
    next(failing)
except ZeroDivisionError:
    print("the element raised as the generator ran")
