# typy-test: LIB-20
# typy-expect: rejected E0408 at 6:19
from collections.abc import Sequence


def total(values: Sequence) -> int:
    return len(values)


print(total([1, 2]))
