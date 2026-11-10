# typy-test: CLS-02
# typy-expect: accepted
class Counter:
    total: int

    def __init__(self) -> None:
        self.total = 0
        self.label = "count"


counter = Counter()
counter.total = 5
counter.total += 2
counter.label = counter.label.upper()
print(counter.total, counter.label)
