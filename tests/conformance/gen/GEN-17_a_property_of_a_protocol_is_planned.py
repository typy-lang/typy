# typy-test: GEN-17
# typy-expect: rejected E0201 at 8:9
from typing import Protocol


class Sized(Protocol):
    @property
    def size(self) -> int: ...
