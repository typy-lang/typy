# typy-test: TYP-29
# typy-expect: rejected E0432 at 14:7
from typing import overload


@overload
def double(value: int) -> int: ...
@overload
def double(value: str) -> str: ...
def double(value: int | str) -> int | str:
    return value


print(double(1.5))
