# typy-test: CLS-15
# typy-expect: rejected E0404 at 8:20
from dataclasses import dataclass, field


@dataclass
class F:
    n: list[int] = field(default=[], default_factory=list)
