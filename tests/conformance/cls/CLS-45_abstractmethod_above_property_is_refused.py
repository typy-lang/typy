# typy-test: CLS-45
# typy-expect: rejected E0434 at 7:6
from abc import ABC, abstractmethod


class Sized(ABC):
    @abstractmethod
    @property
    def size(self) -> int: ...


print(Sized)
