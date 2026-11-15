# typy-test: CLS-16
# typy-expect: rejected E0413 at 12:7
from dataclasses import dataclass, field


@dataclass(frozen=True)
class Box:
    items: list[int]


print(Box([1]))
print({Box([1])})
