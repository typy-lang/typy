# typy-test: FUN-16
# typy-expect: rejected E0201 at 11:30
from collections.abc import Callable


class Box:
    def __init__(self, size: int) -> None:
        self.size = size


make: Callable[[int], Box] = Box
print(make(3).size)
