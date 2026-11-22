# typy-test: LIB-29
# typy-expect: rejected E0201 at 3:1
from __future__ import annotations


def double(value: int) -> int:
    return value * 2


print(double(4))
