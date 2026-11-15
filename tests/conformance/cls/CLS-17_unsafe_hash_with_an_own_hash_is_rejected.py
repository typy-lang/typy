# typy-test: CLS-17
# typy-expect: rejected E0404 at 6:2
from dataclasses import dataclass


@dataclass(unsafe_hash=True)
class K:
    n: int

    def __hash__(self) -> int:
        return 1
