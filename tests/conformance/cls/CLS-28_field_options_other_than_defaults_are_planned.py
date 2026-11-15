# typy-test: CLS-28
# typy-expect: rejected E0201 at 8:20
from dataclasses import dataclass, field


@dataclass
class Item:
    m: int = field(repr=False, default=1)
