# typy-test: GEN-17
# typy-expect: rejected E0201 at 16:3
from typing import Protocol


class Named(Protocol):
    name: str


class Thing:
    def __init__(self) -> None:
        self.name = "t"


x: Named = Thing()
x.name = "u"
