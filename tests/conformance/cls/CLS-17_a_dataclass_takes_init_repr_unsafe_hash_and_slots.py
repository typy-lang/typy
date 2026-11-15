# typy-test: CLS-17
# typy-expect: accepted
from dataclasses import dataclass


@dataclass(init=False, repr=False, unsafe_hash=True, slots=True)
class Counter:
    count: int = 0

    def __init__(self, start: int) -> None:
        self.count = start


counter = Counter(3)
print(counter.count, len({counter, Counter(3)}), repr(counter)[:8])
