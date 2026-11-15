# typy-test: LIB-06
# typy-expect: accepted
import dataclasses
from dataclasses import field


@dataclasses.dataclass
class Item:
    name: str
    count: int = field(default=1)


print(Item("pen"), Item("ink", 3) == Item("ink", 3))
