# typy-test: GEN-13
# typy-expect: rejected E0201 at 6:7
from typing import Protocol


class Getter[T](Protocol):
    def get(self) -> T: ...
