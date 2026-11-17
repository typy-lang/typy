# typy-test: CLS-21
# typy-expect: accepted
from base import Base


class Child(Base):
    def __eq__(self, other: Base) -> bool:
        if other.value < 0:
            return NotImplemented
        return self.value == other.value


print(Child(1) == Child(1), Child(1) == Base(-1), Base(1) == Child(1), Child(2) in [Base(2)])
