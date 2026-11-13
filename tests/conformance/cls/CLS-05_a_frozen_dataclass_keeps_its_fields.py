# typy-test: CLS-05
# typy-expect: rejected E0414 at 12:1
from dataclasses import dataclass


@dataclass(frozen=True)
class Token:
    kind: str


token = Token("name")
token.kind = "number"
