# typy-test: CLS-15, EXP-17
# typy-expect: accepted
from dataclasses import dataclass, field


@dataclass
class Bag:
    first: list[int]
    second: list[int] = field(default_factory=list)


values = [[1]]
bag = Bag(*values)
print(bag)
