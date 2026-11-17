# typy-test: GEN-15
# typy-expect: rejected E0201 at 12:2
from dataclasses import dataclass


@dataclass
class Pair[K, V]:
    key: K
    value: V


@dataclass
class Named(Pair[str, int]):
    note: str
