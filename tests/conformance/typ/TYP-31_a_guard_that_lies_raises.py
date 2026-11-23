# typy-test: TYP-31
# typy-expect: accepted
# A guard whose result its value does not bear out raises TypeError, so a narrowed read
# never takes a value of another type.
from typing import TypeGuard


def says_int(value: object) -> TypeGuard[int]:
    return True


def thing() -> object:
    return "text"


def check(value: object) -> int:
    if says_int(value):
        return value + 1
    return 0


v = thing()
try:
    print(check(v))
except TypeError as error:
    print("raised TypeError")
